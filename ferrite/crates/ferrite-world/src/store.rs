//! A log-structured chunk store.
//!
//! The store is a directory of append-only **segment** files plus an in-memory **index** from chunk
//! position to the record that holds its latest value. Writes only ever append, so a crash can
//! damage at most the tail of the newest segment, and recovery just discards that tail.
//!
//! ```text
//! record = "FRC1" | kind u8 | x i32 | z i32 | len u32 | payload | crc32 u32
//!                            (crc covers kind .. payload; all integers little-endian)
//! kind   = 1 put | 2 delete (len = 0)
//! ```
//!
//! Overwritten and deleted records stay in their segment as dead space until [`Store::compact`]
//! rewrites the live ones. Compaction is crash-safe: see its documentation.
//!
//! Opening scans every segment's record *headers* to rebuild the index. The newest segment is read
//! and checksummed in full, since that is where a torn write can be; older segments are trusted
//! structurally and every [`Store::get`] re-verifies the checksum of what it reads, so silent disk
//! corruption surfaces as an error for that one chunk instead of bad data.

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use thiserror::Error;

use crate::chunk::ChunkPos;
use crate::platform::read_exact_at;

const MAGIC: [u8; 4] = *b"FRC1";
const HEADER: usize = 4 + 1 + 4 + 4 + 4;
const TRAILER: usize = 4;
const KIND_PUT: u8 = 1;
const KIND_DELETE: u8 = 2;
/// Largest payload a record may carry (a chunk is a few KiB; this is a sanity bound).
pub const MAX_PAYLOAD: usize = 64 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
    #[error("segment {segment} is corrupt at offset {offset}: {reason}")]
    Corrupt { segment: u32, offset: u64, reason: &'static str },
    #[error("the record for chunk ({}, {}) failed verification: {reason}", pos.x, pos.z)]
    BadRecord { pos: ChunkPos, reason: &'static str },
    #[error("another process or store handle already has this directory open")]
    AlreadyOpen,
    #[error("payload of {0} bytes exceeds the {MAX_PAYLOAD} byte limit")]
    PayloadTooLarge(usize),
}

pub type Result<T, E = StoreError> = std::result::Result<T, E>;

#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// A segment is sealed once it reaches this size and a new one is started.
    pub max_segment_bytes: u64,
    /// `fsync` after every put/delete. Off by default: call [`Store::sync`] at checkpoints.
    pub sync_on_write: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { max_segment_bytes: 256 * 1024 * 1024, sync_on_write: false }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub chunks: usize,
    pub segments: usize,
    pub total_bytes: u64,
    pub live_bytes: u64,
}

impl Stats {
    /// Fraction of the log that is dead space (0.0 for an empty store).
    pub fn dead_ratio(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            1.0 - self.live_bytes as f64 / self.total_bytes as f64
        }
    }
}

/// What opening the store had to repair.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recovery {
    /// Bytes discarded from the end of the newest segment (a torn write). They were saved next to
    /// the segment as `*.discarded` for inspection.
    pub truncated_bytes: u64,
    /// Leftover files from an interrupted compaction that were removed.
    pub removed_temp_files: usize,
}

struct Segment {
    id: u32,
    path: PathBuf,
    file: File,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Loc {
    segment: u32,
    /// Offset of the record's first byte.
    offset: u64,
    payload_len: u32,
}

impl Loc {
    fn record_len(&self) -> u64 {
        (HEADER + self.payload_len as usize + TRAILER) as u64
    }
}

struct Inner {
    segments: BTreeMap<u32, Arc<Segment>>,
    active: Arc<Segment>,
    active_len: u64,
    index: HashMap<ChunkPos, Loc>,
    next_id: u32,
}

pub struct Store {
    dir: PathBuf,
    opts: Options,
    inner: Mutex<Inner>,
    compacting: Mutex<()>,
    recovery: Recovery,
    /// Held for the life of the store so a second handle (or process) cannot open the directory.
    _lock: File,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store").field("dir", &self.dir).finish_non_exhaustive()
    }
}

fn segment_path(dir: &Path, id: u32) -> PathBuf {
    dir.join(format!("seg-{id:08}.log"))
}

fn parse_segment_name(name: &str) -> Option<u32> {
    name.strip_prefix("seg-")?.strip_suffix(".log")?.parse().ok()
}

fn encode_record(kind: u8, pos: ChunkPos, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER + payload.len() + TRAILER);
    out.extend_from_slice(&MAGIC);
    out.push(kind);
    out.extend_from_slice(&pos.x.to_le_bytes());
    out.extend_from_slice(&pos.z.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    let crc = crc32fast::hash(&out[4..]);
    out.extend_from_slice(&crc.to_le_bytes());
    out
}

struct Header {
    kind: u8,
    pos: ChunkPos,
    len: u32,
}

fn parse_header(h: &[u8; HEADER]) -> Option<Header> {
    if h[..4] != MAGIC {
        return None;
    }
    let kind = h[4];
    let len = u32::from_le_bytes(h[13..17].try_into().unwrap());
    let valid = match kind {
        KIND_PUT => len as usize <= MAX_PAYLOAD,
        KIND_DELETE => len == 0,
        _ => false,
    };
    valid.then(|| Header {
        kind,
        pos: ChunkPos::new(
            i32::from_le_bytes(h[5..9].try_into().unwrap()),
            i32::from_le_bytes(h[9..13].try_into().unwrap()),
        ),
        len,
    })
}

struct Scanned {
    kind: u8,
    pos: ChunkPos,
    offset: u64,
    len: u32,
}

/// What scanning a segment found.
struct ScanResult {
    records: Vec<Scanned>,
    /// Offset just past the last good record.
    valid_end: u64,
    /// Where and why scanning stopped early, if it did.
    problem: Option<(u64, &'static str)>,
}

/// Reads every record of a segment.
fn scan(file: &File, verify: bool) -> io::Result<ScanResult> {
    let file_len = file.metadata()?.len();
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    reader.seek(SeekFrom::Start(0))?;
    let mut records = Vec::new();
    let mut offset = 0u64;
    loop {
        if offset == file_len {
            return Ok(ScanResult { records, valid_end: offset, problem: None });
        }
        let mut head = [0u8; HEADER];
        if file_len - offset < HEADER as u64 {
            return Ok(ScanResult {
                records,
                valid_end: offset,
                problem: Some((offset, "truncated record header")),
            });
        }
        reader.read_exact(&mut head)?;
        let Some(h) = parse_header(&head) else {
            return Ok(ScanResult {
                records,
                valid_end: offset,
                problem: Some((offset, "invalid record header")),
            });
        };
        let end = offset + (HEADER + h.len as usize + TRAILER) as u64;
        if end > file_len {
            return Ok(ScanResult {
                records,
                valid_end: offset,
                problem: Some((offset, "truncated record")),
            });
        }
        if verify {
            let mut body = vec![0u8; h.len as usize + TRAILER];
            reader.read_exact(&mut body)?;
            let (payload, crc) = body.split_at(h.len as usize);
            let mut hasher = crc32fast::Hasher::new();
            hasher.update(&head[4..]);
            hasher.update(payload);
            if hasher.finalize().to_le_bytes() != crc {
                return Ok(ScanResult {
                    records,
                    valid_end: offset,
                    problem: Some((offset, "checksum mismatch")),
                });
            }
        } else {
            reader.seek_relative((h.len as usize + TRAILER) as i64)?;
        }
        records.push(Scanned { kind: h.kind, pos: h.pos, offset, len: h.len });
        offset = end;
    }
}

#[cfg(unix)]
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_dir(_: &Path) -> io::Result<()> {
    // Directories cannot be opened for syncing on Windows; NTFS journals the rename.
    Ok(())
}

fn open_segment(dir: &Path, id: u32, create: bool) -> io::Result<Arc<Segment>> {
    let path = segment_path(dir, id);
    let mut o = OpenOptions::new();
    o.read(true).append(true);
    if create {
        o.create_new(true);
    }
    let file = o.open(&path)?;
    Ok(Arc::new(Segment { id, path, file }))
}

impl Store {
    /// Opens (creating if needed) the store in `dir`, repairing a torn tail if the last run
    /// crashed.
    pub fn open(dir: impl AsRef<Path>, opts: Options) -> Result<Store> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join("LOCK"))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(fs::TryLockError::WouldBlock) => return Err(StoreError::AlreadyOpen),
            Err(fs::TryLockError::Error(e)) => return Err(e.into()),
        }

        let mut recovery = Recovery::default();
        let mut ids = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            if name.ends_with(".tmp") {
                fs::remove_file(dir.join(&name))?; // an interrupted compaction; nothing depended on it
                recovery.removed_temp_files += 1;
            } else if let Some(id) = parse_segment_name(&name) {
                ids.push(id);
            }
        }
        ids.sort_unstable();

        let mut segments = BTreeMap::new();
        let mut index: HashMap<ChunkPos, Loc> = HashMap::new();
        for (n, &id) in ids.iter().enumerate() {
            let segment = open_segment(&dir, id, false)?;
            let is_last = n + 1 == ids.len();
            let ScanResult { records, valid_end, problem } = scan(&segment.file, is_last)?;
            if let Some((offset, reason)) = problem {
                if !is_last {
                    return Err(StoreError::Corrupt { segment: id, offset, reason });
                }
                // A torn write: keep the good prefix, but save what is being thrown away.
                let total = segment.file.metadata()?.len();
                let mut tail = vec![0u8; (total - valid_end) as usize];
                read_exact_at(&segment.file, &mut tail, valid_end)?;
                fs::write(segment.path.with_extension("log.discarded"), &tail)?;
                // Truncate through a separate write handle: the segment handle is append-only,
                // and on Windows an append-only handle may not change a file's length.
                let truncator = OpenOptions::new().write(true).open(&segment.path)?;
                truncator.set_len(valid_end)?;
                truncator.sync_all()?;
                drop(truncator);
                recovery.truncated_bytes = total - valid_end;
            }
            for r in records {
                match r.kind {
                    KIND_PUT => {
                        index.insert(
                            r.pos,
                            Loc { segment: id, offset: r.offset, payload_len: r.len },
                        );
                    }
                    _ => {
                        index.remove(&r.pos);
                    }
                }
            }
            segments.insert(id, segment);
        }

        let (active, next_id) = match segments.values().next_back() {
            Some(last) => (last.clone(), last.id + 1),
            None => {
                let first = open_segment(&dir, 1, true)?;
                segments.insert(1, first.clone());
                sync_dir(&dir)?;
                (first, 2)
            }
        };
        let active_len = active.file.metadata()?.len();
        Ok(Store {
            dir,
            opts,
            inner: Mutex::new(Inner { segments, active, active_len, index, next_id }),
            compacting: Mutex::new(()),
            recovery,
            _lock: lock,
        })
    }

    /// What opening this store had to repair.
    pub fn recovery(&self) -> &Recovery {
        &self.recovery
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().expect("store lock poisoned")
    }

    pub fn contains(&self, pos: ChunkPos) -> bool {
        self.lock().index.contains_key(&pos)
    }

    pub fn len(&self) -> usize {
        self.lock().index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// All stored chunk positions, sorted.
    pub fn keys(&self) -> Vec<ChunkPos> {
        let mut keys: Vec<_> = self.lock().index.keys().copied().collect();
        keys.sort_unstable();
        keys
    }

    /// Reads a chunk's payload, verifying its checksum.
    pub fn get(&self, pos: ChunkPos) -> Result<Option<Vec<u8>>> {
        let (loc, segment) = {
            let inner = self.lock();
            let Some(&loc) = inner.index.get(&pos) else { return Ok(None) };
            (loc, inner.segments[&loc.segment].clone())
        };
        // The segment handle keeps the file alive even if compaction replaces it meanwhile.
        let mut record = vec![0u8; loc.record_len() as usize];
        read_exact_at(&segment.file, &mut record, loc.offset)?;
        let bad = |reason| StoreError::BadRecord { pos, reason };
        let head: &[u8; HEADER] = record[..HEADER].try_into().unwrap();
        let h = parse_header(head).ok_or_else(|| bad("invalid record header"))?;
        if h.kind != KIND_PUT || h.pos != pos || h.len != loc.payload_len {
            return Err(bad("record does not match the index"));
        }
        let (body, crc) = record.split_at(HEADER + h.len as usize);
        if crc32fast::hash(&body[4..]).to_le_bytes() != *crc {
            return Err(bad("checksum mismatch"));
        }
        Ok(Some(record[HEADER..HEADER + h.len as usize].to_vec()))
    }

    fn append(&self, inner: &mut Inner, record: &[u8]) -> Result<Loc> {
        if inner.active_len > 0
            && inner.active_len + record.len() as u64 > self.opts.max_segment_bytes
        {
            self.rotate(inner)?;
        }
        let offset = inner.active_len;
        (&inner.active.file).write_all(record)?;
        if self.opts.sync_on_write {
            inner.active.file.sync_data()?;
        }
        inner.active_len += record.len() as u64;
        Ok(Loc {
            segment: inner.active.id,
            offset,
            payload_len: (record.len() - HEADER - TRAILER) as u32,
        })
    }

    /// Seals the active segment and starts a new one.
    fn rotate(&self, inner: &mut Inner) -> Result<()> {
        inner.active.file.sync_all()?;
        let id = inner.next_id;
        let next = open_segment(&self.dir, id, true)?;
        sync_dir(&self.dir)?;
        inner.next_id += 1;
        inner.segments.insert(id, next.clone());
        inner.active = next;
        inner.active_len = 0;
        Ok(())
    }

    /// Stores `payload` as the chunk's value, replacing any previous one.
    pub fn put(&self, pos: ChunkPos, payload: &[u8]) -> Result<()> {
        if payload.len() > MAX_PAYLOAD {
            return Err(StoreError::PayloadTooLarge(payload.len()));
        }
        let record = encode_record(KIND_PUT, pos, payload);
        let mut inner = self.lock();
        let loc = self.append(&mut inner, &record)?;
        inner.index.insert(pos, loc);
        Ok(())
    }

    /// Removes a chunk. Returns whether it existed.
    pub fn delete(&self, pos: ChunkPos) -> Result<bool> {
        let mut inner = self.lock();
        if !inner.index.contains_key(&pos) {
            return Ok(false);
        }
        let record = encode_record(KIND_DELETE, pos, &[]);
        self.append(&mut inner, &record)?;
        inner.index.remove(&pos);
        Ok(true)
    }

    /// Flushes everything written so far to stable storage.
    pub fn sync(&self) -> Result<()> {
        let inner = self.lock();
        inner.active.file.sync_all()?;
        Ok(())
    }

    pub fn stats(&self) -> Stats {
        let inner = self.lock();
        let mut total = 0;
        for seg in inner.segments.values() {
            total += seg.file.metadata().map(|m| m.len()).unwrap_or(0);
        }
        Stats {
            chunks: inner.index.len(),
            segments: inner.segments.len(),
            total_bytes: total,
            live_bytes: inner.index.values().map(Loc::record_len).sum(),
        }
    }

    /// Rewrites the live records of every sealed segment into one new segment and discards the
    /// old ones, reclaiming dead space. Writers and readers keep working throughout.
    ///
    /// Crash safety. The active segment is sealed first and a new one opened *two ids later*, so
    /// the output can take the id in between: sources < output < new writes, which is exactly the
    /// order recovery replays them in. The output is written under a `.tmp` name, fsynced and only
    /// then renamed, so a crash before the rename changes nothing. After the rename the sources are
    /// removed **oldest first**. A put always precedes the tombstone that cancels it, so whatever
    /// prefix of the sources a crash leaves deleted, no cancelled chunk can come back.
    ///
    /// Returns the number of bytes reclaimed.
    pub fn compact(&self) -> Result<u64> {
        let _only_one = self.compacting.lock().expect("compaction lock poisoned");

        // Phase 1: seal the log and decide what to copy.
        let (sources, output_id, snapshot, before) = {
            let mut inner = self.lock();
            let total: u64 = inner
                .segments
                .values()
                .map(|s| s.file.metadata().map(|m| m.len()).unwrap_or(0))
                .sum();
            let live: u64 = inner.index.values().map(Loc::record_len).sum();
            if total == live {
                return Ok(0); // nothing is dead, so there is nothing to reclaim
            }
            self.rotate_for_compaction(&mut inner)?;
            let sealed_max = inner.active.id - 2;
            let sources: Vec<Arc<Segment>> =
                inner.segments.range(..=sealed_max).map(|(_, s)| s.clone()).collect();
            let snapshot: Vec<(ChunkPos, Loc)> = {
                let mut v: Vec<_> = inner
                    .index
                    .iter()
                    .filter(|(_, l)| l.segment <= sealed_max)
                    .map(|(p, l)| (*p, *l))
                    .collect();
                v.sort_unstable_by_key(|(p, l)| (l.segment, l.offset, *p));
                v
            };
            let before: u64 =
                sources.iter().map(|s| s.file.metadata().map(|m| m.len()).unwrap_or(0)).sum();
            (sources, sealed_max + 1, snapshot, before)
        };

        // Phase 2: copy live records into the temporary output, outside the lock.
        let tmp_path = segment_path(&self.dir, output_id).with_extension("log.tmp");
        let final_path = segment_path(&self.dir, output_id);
        let mut out =
            OpenOptions::new().read(true).append(true).create_new(true).open(&tmp_path)?;
        let by_id: HashMap<u32, &Arc<Segment>> = sources.iter().map(|s| (s.id, s)).collect();
        let mut moved: Vec<(ChunkPos, Loc, Loc)> = Vec::with_capacity(snapshot.len());
        let mut written = 0u64;
        let mut buffer = Vec::new();
        for (pos, old) in &snapshot {
            let mut record = vec![0u8; old.record_len() as usize];
            read_exact_at(&by_id[&old.segment].file, &mut record, old.offset)?;
            let head: &[u8; HEADER] = record[..HEADER].try_into().unwrap();
            let valid = parse_header(head).is_some_and(|h| h.kind == KIND_PUT && h.pos == *pos)
                && crc32fast::hash(&record[4..record.len() - TRAILER]).to_le_bytes()
                    == record[record.len() - TRAILER..];
            if !valid {
                // Never silently drop live data: leave the sources untouched and report it.
                let _ = fs::remove_file(&tmp_path);
                return Err(StoreError::BadRecord {
                    pos: *pos,
                    reason: "corrupt record found during compaction",
                });
            }
            moved.push((
                *pos,
                *old,
                Loc { segment: output_id, offset: written, payload_len: old.payload_len },
            ));
            written += record.len() as u64;
            buffer.extend_from_slice(&record);
            if buffer.len() >= 1 << 20 {
                out.write_all(&buffer)?;
                buffer.clear();
            }
        }
        out.write_all(&buffer)?;
        out.sync_all()?;
        drop(out);

        // Phase 3: publish. Anything rewritten while we copied keeps its newer location.
        fs::rename(&tmp_path, &final_path)?;
        sync_dir(&self.dir)?;
        let output = open_segment(&self.dir, output_id, false)?;
        {
            let mut inner = self.lock();
            inner.segments.insert(output_id, output);
            for (pos, old, new) in moved {
                if inner.index.get(&pos) == Some(&old) {
                    inner.index.insert(pos, new);
                }
            }
            // Remove the sources oldest first and stop at the first failure (Windows refuses to
            // delete a file a reader still has open). Anything left over only holds records the
            // output already supersedes, and the next compaction picks it up.
            for seg in &sources {
                if fs::remove_file(&seg.path).is_err() {
                    break;
                }
                inner.segments.remove(&seg.id);
            }
        }
        sync_dir(&self.dir)?;
        drop(by_id);
        drop(sources);
        Ok(before.saturating_sub(written))
    }

    /// Like [`rotate`](Self::rotate), but reserves the id after the sealed segment for the
    /// compaction output.
    fn rotate_for_compaction(&self, inner: &mut Inner) -> Result<()> {
        inner.next_id += 1; // skip one id: the output will take it
        self.rotate(inner)
    }

    /// Compacts only if at least `threshold` (0.0..=1.0) of the log is dead space.
    pub fn compact_if_dead_ratio_over(&self, threshold: f64) -> Result<u64> {
        if self.stats().dead_ratio() >= threshold { self.compact() } else { Ok(0) }
    }
}
