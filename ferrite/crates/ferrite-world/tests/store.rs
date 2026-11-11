use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use ferrite_world::store::{MAX_PAYLOAD, Options, Store, StoreError};
use ferrite_world::{Chunk, ChunkPos, ChunkStore, ChunkStoreError, Dimension, Limits};
use proptest::prelude::*;
use tempfile::TempDir;

fn pos(i: i32) -> ChunkPos {
    ChunkPos::new(i, -i * 3)
}

/// Deterministic, self-describing payload: key, version, then a pattern derived from both.
fn payload(key: i32, version: u32, len: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(len + 8);
    v.extend_from_slice(&key.to_le_bytes());
    v.extend_from_slice(&version.to_le_bytes());
    let mut x = (key as u32).wrapping_mul(2654435761).wrapping_add(version);
    for _ in 0..len {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        v.push((x >> 24) as u8);
    }
    v
}

fn small() -> Options {
    Options { max_segment_bytes: 300, sync_on_write: false }
}

type Model = BTreeMap<ChunkPos, Vec<u8>>;

fn assert_state(store: &Store, model: &Model, what: &str) {
    assert_eq!(store.keys(), model.keys().copied().collect::<Vec<_>>(), "{what}: keys");
    assert_eq!(store.len(), model.len(), "{what}: len");
    for (p, v) in model {
        assert_eq!(store.get(*p).unwrap().as_deref(), Some(v.as_slice()), "{what}: value of {p:?}");
    }
}

fn segment_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "log"))
        .collect();
    v.sort();
    v
}

/// One record of a segment, parsed independently of the implementation under test.
#[derive(Debug, Clone)]
struct Rec {
    delete: bool,
    pos: ChunkPos,
    start: usize,
    end: usize,
}

fn parse(bytes: &[u8]) -> Vec<Rec> {
    let mut out = Vec::new();
    let mut at = 0;
    while at + 17 <= bytes.len() {
        assert_eq!(&bytes[at..at + 4], b"FRC1");
        let len = u32::from_le_bytes(bytes[at + 13..at + 17].try_into().unwrap()) as usize;
        let end = at + 17 + len + 4;
        if end > bytes.len() {
            break;
        }
        out.push(Rec {
            delete: bytes[at + 4] == 2,
            pos: ChunkPos::new(
                i32::from_le_bytes(bytes[at + 5..at + 9].try_into().unwrap()),
                i32::from_le_bytes(bytes[at + 9..at + 13].try_into().unwrap()),
            ),
            start: at,
            end,
        });
        at = end;
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Basics
// ---------------------------------------------------------------------------------------------

#[test]
fn put_get_overwrite_delete_and_reopen() {
    let dir = TempDir::new().unwrap();
    let mut model = Model::new();
    {
        let s = Store::open(dir.path(), Options::default()).unwrap();
        assert!(s.is_empty());
        assert_eq!(s.get(pos(1)).unwrap(), None);
        for i in 0..20 {
            s.put(pos(i), &payload(i, 1, 50)).unwrap();
            model.insert(pos(i), payload(i, 1, 50));
        }
        s.put(pos(3), &payload(3, 2, 10)).unwrap();
        model.insert(pos(3), payload(3, 2, 10));
        assert!(s.delete(pos(4)).unwrap());
        assert!(!s.delete(pos(4)).unwrap(), "deleting twice reports that nothing was there");
        assert!(!s.delete(pos(99)).unwrap());
        model.remove(&pos(4));
        s.put(pos(5), &[]).unwrap();
        model.insert(pos(5), vec![]);
        assert!(s.contains(pos(5)) && !s.contains(pos(4)));
        assert_state(&s, &model, "before reopen");
        s.sync().unwrap();
    }
    let s = Store::open(dir.path(), Options::default()).unwrap();
    assert_eq!(*s.recovery(), Default::default());
    assert_state(&s, &model, "after reopen");
}

#[test]
fn a_directory_can_only_be_open_once() {
    let dir = TempDir::new().unwrap();
    let first = Store::open(dir.path(), Options::default()).unwrap();
    assert!(matches!(Store::open(dir.path(), Options::default()), Err(StoreError::AlreadyOpen)));
    drop(first);
    Store::open(dir.path(), Options::default()).unwrap();
}

#[test]
fn oversized_payloads_are_refused() {
    let dir = TempDir::new().unwrap();
    let s = Store::open(dir.path(), Options::default()).unwrap();
    let big = vec![0u8; MAX_PAYLOAD + 1];
    assert!(
        matches!(s.put(pos(0), &big), Err(StoreError::PayloadTooLarge(n)) if n == MAX_PAYLOAD + 1)
    );
    assert!(s.is_empty());
}

#[test]
fn segments_rotate_and_everything_stays_readable() {
    let dir = TempDir::new().unwrap();
    let mut model = Model::new();
    {
        let s = Store::open(dir.path(), small()).unwrap();
        for i in 0..60 {
            s.put(pos(i), &payload(i, 1, 40)).unwrap();
            model.insert(pos(i), payload(i, 1, 40));
        }
        assert!(s.stats().segments > 5, "{:?}", s.stats());
        assert_state(&s, &model, "rotating");
    }
    let s = Store::open(dir.path(), small()).unwrap();
    assert_state(&s, &model, "reopened");
}

#[test]
fn a_chunk_larger_than_a_segment_still_fits() {
    let dir = TempDir::new().unwrap();
    let s = Store::open(dir.path(), small()).unwrap();
    s.put(pos(1), &payload(1, 1, 5000)).unwrap();
    s.put(pos(2), &payload(2, 1, 5000)).unwrap();
    assert_eq!(s.get(pos(2)).unwrap().unwrap(), payload(2, 1, 5000));
}

// ---------------------------------------------------------------------------------------------
// Crash recovery
// ---------------------------------------------------------------------------------------------

/// Cut the log at *every* byte offset: recovery must always yield exactly the records that were
/// fully written before the cut, and the store must stay writable afterwards.
#[test]
fn a_torn_tail_is_recovered_at_every_possible_cut() {
    let dir = TempDir::new().unwrap();
    let ops: Vec<(bool, i32, u32)> = vec![
        (false, 1, 1),
        (false, 2, 1),
        (false, 1, 2),
        (false, 3, 1),
        (true, 2, 0),
        (false, 4, 1),
        (false, 2, 2),
        (true, 3, 0),
        (false, 5, 1),
        (false, 4, 2),
    ];
    {
        let s = Store::open(dir.path(), Options::default()).unwrap();
        for &(delete, key, version) in &ops {
            if delete {
                s.delete(pos(key)).unwrap();
            } else {
                s.put(pos(key), &payload(key, version, 7 + key as usize * 3)).unwrap();
            }
        }
    }
    let files = segment_files(dir.path());
    assert_eq!(files.len(), 1);
    let log = fs::read(&files[0]).unwrap();
    let records = parse(&log);
    assert_eq!(records.len(), ops.len());

    for cut in 0..=log.len() {
        let work = TempDir::new().unwrap();
        fs::write(work.path().join("seg-00000001.log"), &log[..cut]).unwrap();

        // What the first `n` complete records leave behind.
        let complete: Vec<&Rec> = records.iter().take_while(|r| r.end <= cut).collect();
        let mut model = Model::new();
        for (r, &(_, key, version)) in complete.iter().zip(&ops) {
            if r.delete {
                model.remove(&r.pos);
            } else {
                model.insert(r.pos, payload(key, version, 7 + key as usize * 3));
            }
        }
        let valid_end = complete.last().map_or(0, |r| r.end);

        let s = Store::open(work.path(), Options::default())
            .unwrap_or_else(|e| panic!("cut {cut}: {e}"));
        assert_state(&s, &model, &format!("cut {cut}"));
        assert_eq!(s.recovery().truncated_bytes, (cut - valid_end) as u64, "cut {cut}");
        let discarded = work.path().join("seg-00000001.log.discarded");
        if cut > valid_end {
            assert_eq!(
                fs::read(&discarded).unwrap(),
                &log[valid_end..cut],
                "cut {cut}: the torn tail is saved"
            );
        } else {
            assert!(!discarded.exists());
        }

        // The repaired store is fully usable, and its next write survives a restart.
        s.put(pos(77), &payload(77, 1, 5)).unwrap();
        s.sync().unwrap();
        drop(s);
        let s = Store::open(work.path(), Options::default()).unwrap();
        assert_eq!(s.get(pos(77)).unwrap().unwrap(), payload(77, 1, 5), "cut {cut}");
        assert_eq!(s.len(), model.len() + 1);
    }
}

#[test]
fn a_corrupt_record_in_the_newest_segment_truncates_from_there_and_keeps_the_rest_aside() {
    let dir = TempDir::new().unwrap();
    {
        let s = Store::open(dir.path(), Options::default()).unwrap();
        for i in 0..6 {
            s.put(pos(i), &payload(i, 1, 30)).unwrap();
        }
    }
    let path = &segment_files(dir.path())[0];
    let mut log = fs::read(path).unwrap();
    let recs = parse(&log);
    log[recs[3].start + 20] ^= 0xff; // damage the payload of the fourth record
    fs::write(path, &log).unwrap();

    let s = Store::open(dir.path(), Options::default()).unwrap();
    assert_eq!(s.len(), 3, "records 0..3 survive; the damaged one and everything after is dropped");
    for i in 0..3 {
        assert!(s.contains(pos(i)));
    }
    assert_eq!(s.recovery().truncated_bytes as usize, log.len() - recs[3].start);
    let saved = fs::read(path.with_extension("log.discarded")).unwrap();
    assert_eq!(saved, &log[recs[3].start..], "nothing is destroyed, only moved aside");
}

#[test]
fn corruption_in_a_sealed_segment_fails_only_that_chunk() {
    let dir = TempDir::new().unwrap();
    {
        let s = Store::open(dir.path(), small()).unwrap();
        for i in 0..30 {
            s.put(pos(i), &payload(i, 1, 40)).unwrap();
        }
    }
    let files = segment_files(dir.path());
    assert!(files.len() > 3);
    let sealed = &files[0];
    let mut log = fs::read(sealed).unwrap();
    let recs = parse(&log);
    let victim = recs[0].pos;
    log[recs[0].start + 25] ^= 0x01;
    fs::write(sealed, &log).unwrap();

    let s = Store::open(dir.path(), small()).unwrap(); // headers are intact, so it opens
    assert!(matches!(
        s.get(victim),
        Err(StoreError::BadRecord { reason: "checksum mismatch", .. })
    ));
    // Every other chunk is fine, and the bad one does not poison them.
    for i in 0..30 {
        if pos(i) != victim {
            assert_eq!(s.get(pos(i)).unwrap().unwrap(), payload(i, 1, 40));
        }
    }
    // Overwriting the damaged chunk repairs it.
    s.put(victim, b"fresh").unwrap();
    assert_eq!(s.get(victim).unwrap().unwrap(), b"fresh");
}

#[test]
fn structural_damage_in_a_sealed_segment_is_reported_not_guessed_at() {
    let dir = TempDir::new().unwrap();
    {
        let s = Store::open(dir.path(), small()).unwrap();
        for i in 0..30 {
            s.put(pos(i), &payload(i, 1, 40)).unwrap();
        }
    }
    let sealed = &segment_files(dir.path())[0];
    let mut log = fs::read(sealed).unwrap();
    let recs = parse(&log);
    log[recs[1].start] = b'X'; // smash a record header in the middle of a sealed segment
    fs::write(sealed, &log).unwrap();
    match Store::open(dir.path(), small()) {
        Err(StoreError::Corrupt { segment: 1, offset, reason }) => {
            assert_eq!(offset as usize, recs[1].start);
            assert_eq!(reason, "invalid record header");
        }
        other => panic!("expected a corruption error, got {other:?}"),
    }
}

#[test]
fn leftovers_of_an_interrupted_compaction_are_ignored_and_removed() {
    let dir = TempDir::new().unwrap();
    let mut model = Model::new();
    {
        let s = Store::open(dir.path(), small()).unwrap();
        for i in 0..20 {
            s.put(pos(i), &payload(i, 1, 30)).unwrap();
            model.insert(pos(i), payload(i, 1, 30));
        }
    }
    // A crash before the rename leaves a half-written temporary output behind.
    fs::write(dir.path().join("seg-00000099.log.tmp"), b"partial garbage").unwrap();
    let s = Store::open(dir.path(), small()).unwrap();
    assert_eq!(s.recovery().removed_temp_files, 1);
    assert!(!dir.path().join("seg-00000099.log.tmp").exists());
    assert_state(&s, &model, "after removing the temp file");
}

// ---------------------------------------------------------------------------------------------
// Compaction
// ---------------------------------------------------------------------------------------------

fn churn(store: &Store, model: &mut Model, rounds: u32) {
    for round in 1..=rounds {
        for i in 0..12 {
            store.put(pos(i), &payload(i, round, 25)).unwrap();
            model.insert(pos(i), payload(i, round, 25));
        }
    }
}

#[test]
fn compaction_reclaims_dead_space_without_changing_anything_visible() {
    let dir = TempDir::new().unwrap();
    let mut model = Model::new();
    let s = Store::open(dir.path(), small()).unwrap();
    churn(&s, &mut model, 12);
    s.delete(pos(2)).unwrap();
    model.remove(&pos(2));

    let before = s.stats();
    assert!(before.dead_ratio() > 0.8, "{before:?}");
    let reclaimed = s.compact().unwrap();
    let after = s.stats();
    assert!(reclaimed > 0 && after.total_bytes < before.total_bytes / 3, "{before:?} -> {after:?}");
    assert!(after.dead_ratio() < 0.5, "{after:?}");
    assert!(after.segments < before.segments);
    assert_state(&s, &model, "after compaction");

    // Writes after compaction work, and so does a restart.
    churn(&s, &mut model, 2);
    drop(s);
    let s = Store::open(dir.path(), small()).unwrap();
    assert_state(&s, &model, "after compaction and reopen");
}

#[test]
fn compacting_a_store_with_nothing_dead_does_nothing() {
    let dir = TempDir::new().unwrap();
    let s = Store::open(dir.path(), small()).unwrap();
    assert_eq!(s.compact().unwrap(), 0, "empty store");
    for i in 0..10 {
        s.put(pos(i), &payload(i, 1, 30)).unwrap();
    }
    let segments = s.stats().segments;
    assert_eq!(s.compact().unwrap(), 0);
    assert_eq!(s.stats().segments, segments, "no segments churned");
    // And repeating it after a real compaction is also a no-op, not an endless rewrite.
    for i in 0..10 {
        s.put(pos(i), &payload(i, 2, 30)).unwrap();
    }
    assert!(s.compact().unwrap() > 0);
    let after = s.stats();
    assert_eq!(s.compact().unwrap(), 0);
    assert_eq!(s.stats(), after);
}

#[test]
fn the_threshold_variant_only_compacts_when_worth_it() {
    let dir = TempDir::new().unwrap();
    let s = Store::open(dir.path(), small()).unwrap();
    let mut model = Model::new();
    churn(&s, &mut model, 1);
    assert_eq!(s.compact_if_dead_ratio_over(0.5).unwrap(), 0);
    churn(&s, &mut model, 10);
    assert!(s.compact_if_dead_ratio_over(0.5).unwrap() > 0);
    assert_state(&s, &model, "thresholded");
}

/// Reconstructs every state the disk can be in if the process dies while compaction is removing
/// the old segments, and checks that opening any of them yields exactly the right data.
fn crash_states(delete_oldest_first: bool) -> Vec<Result<(), String>> {
    let dir = TempDir::new().unwrap();
    let mut model = Model::new();
    {
        let s = Store::open(dir.path(), small()).unwrap();
        // Chunk 1 is written early, then lots of filler pushes the log through many segments, and
        // only then is chunk 1 deleted: the put and its cancelling tombstone are in different
        // segments, which is the case that could resurrect it.
        s.put(pos(1), &payload(1, 1, 30)).unwrap();
        model.insert(pos(1), payload(1, 1, 30));
        for i in 2..40 {
            s.put(pos(i), &payload(i, 1, 30)).unwrap();
            model.insert(pos(i), payload(i, 1, 30));
        }
        s.delete(pos(1)).unwrap();
        model.remove(&pos(1));
        for i in 2..6 {
            s.put(pos(i), &payload(i, 2, 30)).unwrap();
            model.insert(pos(i), payload(i, 2, 30));
        }
        s.delete(pos(7)).unwrap();
        model.remove(&pos(7));
    }
    let sources: Vec<(PathBuf, Vec<u8>)> =
        segment_files(dir.path()).into_iter().map(|p| (p.clone(), fs::read(&p).unwrap())).collect();
    assert!(sources.len() >= 5, "need several segments, got {}", sources.len());

    // Run the real compaction, then keep only what it *added*: the output and the new active file.
    {
        let s = Store::open(dir.path(), small()).unwrap();
        assert!(s.compact().unwrap() > 0);
    }
    let after: Vec<(String, Vec<u8>)> = segment_files(dir.path())
        .into_iter()
        .map(|p| (p.file_name().unwrap().to_string_lossy().into_owned(), fs::read(&p).unwrap()))
        .collect();
    let source_names: Vec<String> = sources
        .iter()
        .map(|(p, _)| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    let added: Vec<&(String, Vec<u8>)> =
        after.iter().filter(|(n, _)| !source_names.contains(n)).collect();
    assert!(!added.is_empty());

    let k = sources.len();
    let mut results = Vec::new();
    for removed in 0..=k {
        // A crash after `removed` of the old segments were deleted.
        let kept: Vec<&(PathBuf, Vec<u8>)> = if delete_oldest_first {
            sources.iter().skip(removed).collect()
        } else {
            sources.iter().take(k - removed).collect()
        };
        let work = TempDir::new().unwrap();
        for (path, bytes) in &kept {
            fs::write(work.path().join(path.file_name().unwrap()), bytes).unwrap();
        }
        for (name, bytes) in &added {
            fs::write(work.path().join(name), bytes).unwrap();
        }
        let outcome = (|| {
            let s = Store::open(work.path(), small())
                .map_err(|e| format!("{removed} removed: open failed: {e}"))?;
            let keys: Vec<_> = model.keys().copied().collect();
            if s.keys() != keys {
                return Err(format!(
                    "{removed} removed: keys differ (extra: {:?})",
                    s.keys().into_iter().filter(|k| !model.contains_key(k)).collect::<Vec<_>>()
                ));
            }
            for (p, v) in &model {
                if s.get(*p).map_err(|e| e.to_string())?.as_deref() != Some(v.as_slice()) {
                    return Err(format!("{removed} removed: wrong value for {p:?}"));
                }
            }
            Ok(())
        })();
        results.push(outcome);
    }
    results
}

#[test]
fn a_crash_at_any_point_while_old_segments_are_removed_loses_and_resurrects_nothing() {
    let results = crash_states(true);
    assert!(results.len() >= 6);
    for r in &results {
        r.as_ref().unwrap();
    }
}

/// Negative control: removing the old segments *newest first* would let a deleted chunk come back,
/// which is exactly why compaction removes them oldest first. If this stopped failing, the test
/// above would be proving nothing.
#[test]
fn removing_old_segments_in_the_wrong_order_would_resurrect_a_deleted_chunk() {
    let results = crash_states(false);
    let failures: Vec<_> = results.iter().filter_map(|r| r.as_ref().err()).collect();
    assert!(
        failures.iter().any(|f| f.contains("extra: [ChunkPos { x: 1")),
        "wrong-order deletion should resurrect chunk 1; got {failures:?}"
    );
}

#[test]
fn compaction_while_writing_loses_no_update() {
    let dir = TempDir::new().unwrap();
    let store = Arc::new(
        Store::open(dir.path(), Options { max_segment_bytes: 2000, sync_on_write: false }).unwrap(),
    );
    let stop = Arc::new(AtomicBool::new(false));
    let compactor = {
        let (store, stop) = (store.clone(), stop.clone());
        std::thread::spawn(move || {
            let mut runs = 0;
            while !stop.load(Ordering::Relaxed) {
                store.compact().unwrap();
                runs += 1;
            }
            runs
        })
    };
    let mut model = Model::new();
    for version in 1..=60u32 {
        for key in 0..40 {
            store.put(pos(key), &payload(key, version, 20 + (version as usize % 7))).unwrap();
            model.insert(pos(key), payload(key, version, 20 + (version as usize % 7)));
        }
        if version % 9 == 0 {
            store.delete(pos(version as i32 % 40)).unwrap();
            model.remove(&pos(version as i32 % 40));
        }
    }
    stop.store(true, Ordering::Relaxed);
    let runs = compactor.join().unwrap();
    assert!(runs > 0);
    assert_state(&store, &model, "after concurrent compaction");
    let store = Arc::try_unwrap(store).unwrap();
    drop(store);
    assert_state(&Store::open(dir.path(), Options::default()).unwrap(), &model, "after reopening");
}

#[test]
fn compaction_refuses_to_drop_a_corrupt_live_record() {
    let dir = TempDir::new().unwrap();
    {
        let s = Store::open(dir.path(), small()).unwrap();
        for round in 1..=4 {
            for i in 0..8 {
                s.put(pos(i), &payload(i, round, 30)).unwrap();
            }
        }
    }
    // Damage a record that is still the live version of its chunk (in a sealed segment).
    let files = segment_files(dir.path());
    let victim_file = &files[files.len() - 2];
    let mut log = fs::read(victim_file).unwrap();
    let recs = parse(&log);
    log[recs[0].start + 30] ^= 0xff;
    fs::write(victim_file, &log).unwrap();

    let s = Store::open(dir.path(), small()).unwrap();
    let before: Vec<(PathBuf, Vec<u8>)> =
        segment_files(dir.path()).into_iter().map(|p| (p.clone(), fs::read(&p).unwrap())).collect();
    assert!(
        matches!(s.compact(), Err(StoreError::BadRecord { .. })),
        "must not silently lose data"
    );
    // Every original segment is still there, byte for byte. (A fresh empty active segment may have
    // been started before the failure was found, which is harmless.)
    for (path, bytes) in &before {
        assert_eq!(&fs::read(path).unwrap(), bytes, "{path:?} must be untouched");
    }
    assert!(
        !fs::read_dir(dir.path()).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp"))
    );
    // The store keeps working.
    s.put(pos(100), b"still alive").unwrap();
    assert_eq!(s.get(pos(100)).unwrap().unwrap(), b"still alive");
}

// ---------------------------------------------------------------------------------------------
// Concurrency
// ---------------------------------------------------------------------------------------------

#[test]
fn readers_never_see_torn_or_time_travelling_values() {
    let dir = TempDir::new().unwrap();
    let store = Arc::new(
        Store::open(dir.path(), Options { max_segment_bytes: 4096, sync_on_write: false }).unwrap(),
    );
    for key in 0..16 {
        store.put(pos(key), &payload(key, 0, 64)).unwrap();
    }
    let stop = Arc::new(AtomicBool::new(false));

    let readers: Vec<_> = (0..4)
        .map(|r| {
            let (store, stop) = (store.clone(), stop.clone());
            std::thread::spawn(move || {
                let mut last: HashMap<i32, u32> = HashMap::new();
                let mut reads = 0u64;
                while !stop.load(Ordering::Relaxed) {
                    for key in 0..16 {
                        let bytes = store
                            .get(pos(key))
                            .unwrap()
                            .expect("a chunk that is never deleted must always be readable");
                        let (k, version) = (
                            i32::from_le_bytes(bytes[..4].try_into().unwrap()),
                            u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
                        );
                        assert_eq!(k, key, "reader {r}: a value for another key");
                        assert_eq!(
                            bytes,
                            payload(key, version, 64),
                            "reader {r}: a torn or mixed-up value"
                        );
                        let seen = last.entry(key).or_insert(0);
                        assert!(
                            version >= *seen,
                            "reader {r}: version went backwards ({seen} -> {version})"
                        );
                        *seen = version;
                        reads += 1;
                    }
                }
                reads
            })
        })
        .collect();
    let compactor = {
        let (store, stop) = (store.clone(), stop.clone());
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                store.compact().unwrap();
            }
        })
    };
    for version in 1..=300u32 {
        for key in 0..16 {
            store.put(pos(key), &payload(key, version, 64)).unwrap();
        }
    }
    stop.store(true, Ordering::Relaxed);
    compactor.join().unwrap();
    let reads: u64 = readers.into_iter().map(|r| r.join().unwrap()).sum();
    assert!(reads > 1000, "the readers actually ran ({reads} reads)");
    for key in 0..16 {
        assert_eq!(store.get(pos(key)).unwrap().unwrap(), payload(key, 300, 64));
    }
}

// ---------------------------------------------------------------------------------------------
// The chunk layer
// ---------------------------------------------------------------------------------------------

#[test]
fn chunks_survive_the_store() {
    let dir = TempDir::new().unwrap();
    let mut chunk = Chunk::filled(Dimension::OVERWORLD, 0, 1);
    for i in 0..200 {
        chunk.set_block(i % 16, -64 + i, (i / 16) % 16, 1 + (i as u16 % 40));
    }
    {
        let cs = ChunkStore::open(dir.path(), small(), Limits::vanilla()).unwrap();
        assert!(cs.load(ChunkPos::new(3, 4)).unwrap().is_none());
        cs.save(ChunkPos::new(3, 4), &chunk).unwrap();
        cs.save(ChunkPos::new(-1, 0), &Chunk::filled(Dimension::NETHER, 0, 0)).unwrap();
        assert_eq!(cs.load(ChunkPos::new(3, 4)).unwrap().unwrap(), chunk);
        assert!(cs.delete(ChunkPos::new(-1, 0)).unwrap());
    }
    let cs = ChunkStore::open(dir.path(), small(), Limits::vanilla()).unwrap().with_level(1);
    assert_eq!(
        cs.load(ChunkPos::new(3, 4)).unwrap().unwrap(),
        chunk,
        "data written at one level reads at any level"
    );
    assert!(cs.load(ChunkPos::new(-1, 0)).unwrap().is_none());
}

#[test]
fn a_record_that_is_not_a_chunk_is_a_decode_error_not_a_panic() {
    let dir = TempDir::new().unwrap();
    let cs = ChunkStore::open(dir.path(), small(), Limits::vanilla()).unwrap();
    cs.store().put(ChunkPos::new(0, 0), b"this is not a zstd frame").unwrap();
    assert!(matches!(cs.load(ChunkPos::new(0, 0)), Err(ChunkStoreError::Codec { .. })));
}

// ---------------------------------------------------------------------------------------------
// Model-based test: any sequence of operations, restarts and compactions must match a HashMap.
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Op {
    Put(i32, usize, u32),
    Delete(i32),
    Get(i32),
    Reopen,
    Compact,
    Sync,
}

fn arb_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        6 => (0i32..8, 0usize..200, any::<u32>()).prop_map(|(k, l, v)| Op::Put(k, l, v)),
        2 => (0i32..8).prop_map(Op::Delete),
        3 => (0i32..8).prop_map(Op::Get),
        1 => Just(Op::Reopen),
        1 => Just(Op::Compact),
        1 => Just(Op::Sync),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 60, ..ProptestConfig::default() })]

    #[test]
    fn the_store_agrees_with_a_map_through_restarts_and_compactions(ops in prop::collection::vec(arb_op(), 1..90)) {
        let dir = TempDir::new().unwrap();
        let mut store = Some(Store::open(dir.path(), small()).unwrap());
        let mut model = Model::new();
        for op in ops {
            let s = store.as_ref().unwrap();
            match op {
                Op::Put(k, len, v) => {
                    s.put(pos(k), &payload(k, v, len)).unwrap();
                    model.insert(pos(k), payload(k, v, len));
                }
                Op::Delete(k) => prop_assert_eq!(s.delete(pos(k)).unwrap(), model.remove(&pos(k)).is_some()),
                Op::Get(k) => prop_assert_eq!(s.get(pos(k)).unwrap(), model.get(&pos(k)).cloned()),
                Op::Sync => s.sync().unwrap(),
                Op::Compact => { s.compact().unwrap(); }
                Op::Reopen => {
                    // Dropping the old handle releases the directory lock before reopening.
                    drop(store.take());
                    store = Some(Store::open(dir.path(), small()).unwrap());
                }
            }
            let s = store.as_ref().unwrap();
            prop_assert_eq!(s.len(), model.len());
        }
        let s = store.as_ref().unwrap();
        prop_assert_eq!(s.keys(), model.keys().copied().collect::<Vec<_>>());
        for (p, v) in &model {
            let got = s.get(*p).unwrap();
            prop_assert_eq!(got.as_ref(), Some(v));
        }
        // A final restart must agree too.
        drop(store.take());
        let s = Store::open(dir.path(), small()).unwrap();
        prop_assert_eq!(s.keys(), model.keys().copied().collect::<Vec<_>>());
    }
}
