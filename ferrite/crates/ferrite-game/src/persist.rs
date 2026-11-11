//! Saving the world: chunks, container contents, players and the clock.
//!
//! ```text
//! <world>/chunks/       the chunk store (append-only log, see ferrite-world)
//! <world>/containers/   chest and barrel contents, one record per chunk
//! <world>/players/      one file per player (by uuid), written atomically
//! <world>/level.dat     game time
//! ```
//!
//! Items are stored by registry name, not numeric id, so a save survives registry changes.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use ferrite_blocks::BlockPos;
use ferrite_protocol::packets::play::ItemStack;
use ferrite_world::{Chunk, ChunkPos, ChunkStore, Limits, Options, Store};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum PersistError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Chunks(#[from] ferrite_world::ChunkStoreError),
    #[error(transparent)]
    Store(#[from] ferrite_world::StoreError),
    #[error("corrupt {what}: {why}")]
    Corrupt { what: &'static str, why: &'static str },
}

/// What is saved about a player.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSave {
    pub pos: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub game_mode: u8,
    pub held_slot: u8,
    /// `(slot, stack)` of every non-empty slot except the crafting grid.
    pub slots: Vec<(u8, ItemStack)>,
}

#[derive(Debug)]
pub struct Persistence {
    dir: PathBuf,
    chunks: ChunkStore,
    containers: Store,
}

fn item_name(item: i32) -> Option<&'static str> {
    ferrite_data::registries::item::names.get(usize::try_from(item).ok()?).copied()
}

fn put_stack(out: &mut Vec<u8>, s: ItemStack) {
    let name = item_name(s.item).unwrap_or("minecraft:air");
    out.push(name.len() as u8);
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(&s.count.to_le_bytes());
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], PersistError> {
        if self.0.len() < n {
            return Err(PersistError::Corrupt { what: "record", why: "truncated" });
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }

    fn u8(&mut self) -> Result<u8, PersistError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, PersistError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("2")))
    }

    fn i32(&mut self) -> Result<i32, PersistError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().expect("4")))
    }

    fn i64(&mut self) -> Result<i64, PersistError> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().expect("8")))
    }

    fn f64(&mut self) -> Result<f64, PersistError> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().expect("8")))
    }

    fn f32(&mut self) -> Result<f32, PersistError> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().expect("4")))
    }

    /// A stack; items this version does not know are skipped (`None`).
    fn stack(&mut self) -> Result<Option<ItemStack>, PersistError> {
        let n = usize::from(self.u8()?);
        let name = std::str::from_utf8(self.take(n)?)
            .map_err(|_| PersistError::Corrupt { what: "record", why: "bad item name" })?
            .to_string();
        let count = self.i32()?;
        Ok(ferrite_data::registries::item::by_name
            .get(name.as_str())
            .filter(|_| count > 0)
            .map(|id| ItemStack { count, item: *id as i32 }))
    }
}

impl Persistence {
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, PersistError> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(dir.join("players"))?;
        Ok(Persistence {
            chunks: ChunkStore::open(dir.join("chunks"), Options::default(), Limits::vanilla())?,
            containers: Store::open(dir.join("containers"), Options::default())?,
            dir,
        })
    }

    pub fn load_chunk(&self, pos: ChunkPos) -> Result<Option<Chunk>, PersistError> {
        Ok(self.chunks.load(pos)?)
    }

    pub fn save_chunk(&self, pos: ChunkPos, chunk: &Chunk) -> Result<(), PersistError> {
        Ok(self.chunks.save(pos, chunk)?)
    }

    /// Makes everything written so far durable.
    pub fn sync(&self) -> Result<(), PersistError> {
        self.chunks.store().sync()?;
        self.containers.sync()?;
        Ok(())
    }

    // ---- containers ----

    /// Saves the containers inside one chunk (an empty list deletes the record).
    pub fn save_containers(
        &self,
        chunk: ChunkPos,
        containers: &[(BlockPos, &[ItemStack])],
    ) -> Result<(), PersistError> {
        if containers.iter().all(|(_, s)| s.iter().all(|i| i.count <= 0)) {
            self.containers.delete(chunk)?;
            return Ok(());
        }
        let mut out = vec![1u8];
        out.extend_from_slice(&(containers.len() as u16).to_le_bytes());
        for (pos, slots) in containers {
            for v in [pos.x, pos.y, pos.z] {
                out.extend_from_slice(&v.to_le_bytes());
            }
            let filled: Vec<(usize, &ItemStack)> =
                slots.iter().enumerate().filter(|(_, s)| s.count > 0).collect();
            out.extend_from_slice(&(slots.len() as u16).to_le_bytes());
            out.extend_from_slice(&(filled.len() as u16).to_le_bytes());
            for (i, s) in filled {
                out.extend_from_slice(&(i as u16).to_le_bytes());
                put_stack(&mut out, *s);
            }
        }
        self.containers.put(chunk, &out)?;
        Ok(())
    }

    pub fn load_containers(
        &self,
        chunk: ChunkPos,
    ) -> Result<HashMap<BlockPos, Vec<ItemStack>>, PersistError> {
        let mut out = HashMap::new();
        let Some(bytes) = self.containers.get(chunk)? else { return Ok(out) };
        let mut r = Reader(&bytes);
        if r.u8()? != 1 {
            return Err(PersistError::Corrupt { what: "containers", why: "unknown version" });
        }
        for _ in 0..r.u16()? {
            let pos = BlockPos::new(r.i32()?, r.i32()?, r.i32()?);
            let size = usize::from(r.u16()?);
            let mut slots = vec![ItemStack::default(); size];
            for _ in 0..r.u16()? {
                let i = usize::from(r.u16()?);
                let stack = r.stack()?;
                if let (Some(stack), Some(slot)) = (stack, slots.get_mut(i)) {
                    *slot = stack;
                }
            }
            out.insert(pos, slots);
        }
        Ok(out)
    }

    // ---- players ----

    fn player_path(&self, uuid: Uuid) -> PathBuf {
        self.dir.join("players").join(format!("{uuid}.dat"))
    }

    pub fn save_player(&self, uuid: Uuid, p: &PlayerSave) -> Result<(), PersistError> {
        let mut out = vec![1u8];
        for v in p.pos {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&p.yaw.to_le_bytes());
        out.extend_from_slice(&p.pitch.to_le_bytes());
        out.push(p.game_mode);
        out.push(p.held_slot);
        out.extend_from_slice(&(p.slots.len() as u16).to_le_bytes());
        for (slot, stack) in &p.slots {
            out.push(*slot);
            put_stack(&mut out, *stack);
        }
        write_atomic(&self.player_path(uuid), &out)
    }

    pub fn load_player(&self, uuid: Uuid) -> Result<Option<PlayerSave>, PersistError> {
        let mut bytes = Vec::new();
        match std::fs::File::open(self.player_path(uuid)) {
            Ok(mut f) => f.read_to_end(&mut bytes).map(|_| ())?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        let mut r = Reader(&bytes);
        if r.u8()? != 1 {
            return Err(PersistError::Corrupt { what: "player", why: "unknown version" });
        }
        let pos = [r.f64()?, r.f64()?, r.f64()?];
        let (yaw, pitch) = (r.f32()?, r.f32()?);
        let (game_mode, held_slot) = (r.u8()?, r.u8()?);
        let mut slots = Vec::new();
        for _ in 0..r.u16()? {
            let slot = r.u8()?;
            if let Some(stack) = r.stack()? {
                slots.push((slot, stack));
            }
        }
        Ok(Some(PlayerSave { pos, yaw, pitch, game_mode, held_slot, slots }))
    }

    // ---- the clock ----

    pub fn save_time(&self, game_time: i64) -> Result<(), PersistError> {
        let mut out = vec![1u8];
        out.extend_from_slice(&game_time.to_le_bytes());
        write_atomic(&self.dir.join("level.dat"), &out)
    }

    pub fn load_time(&self) -> Result<Option<i64>, PersistError> {
        let bytes = match std::fs::read(self.dir.join("level.dat")) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let mut r = Reader(&bytes);
        if r.u8()? != 1 {
            return Err(PersistError::Corrupt { what: "level", why: "unknown version" });
        }
        Ok(Some(r.i64()?))
    }
}

/// Writes through a temporary file and renames, so a crash leaves the old file or the new one.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), PersistError> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(name: &str, count: i32) -> ItemStack {
        ItemStack { count, item: ferrite_data::registries::item::by_name[name] as i32 }
    }

    #[test]
    fn containers_players_and_time_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let p = Persistence::open(dir.path()).unwrap();
        let mut chest = vec![ItemStack::default(); 27];
        chest[3] = stack("minecraft:stone", 12);
        chest[26] = stack("minecraft:diamond", 1);
        let at = BlockPos::new(5, 70, -9);
        p.save_containers(ChunkPos::new(0, -1), &[(at, &chest)]).unwrap();
        let back = p.load_containers(ChunkPos::new(0, -1)).unwrap();
        assert_eq!(back[&at], chest);
        assert!(p.load_containers(ChunkPos::new(7, 7)).unwrap().is_empty());
        // An emptied chunk's record goes away.
        p.save_containers(ChunkPos::new(0, -1), &[(at, &vec![ItemStack::default(); 27])]).unwrap();
        assert!(p.load_containers(ChunkPos::new(0, -1)).unwrap().is_empty());

        let uuid = Uuid::from_u128(42);
        let save = PlayerSave {
            pos: [1.5, 64.0, -3.25],
            yaw: 90.0,
            pitch: -10.0,
            game_mode: 0,
            held_slot: 4,
            slots: vec![(36, stack("minecraft:oak_log", 64)), (45, stack("minecraft:shield", 1))],
        };
        p.save_player(uuid, &save).unwrap();
        assert_eq!(p.load_player(uuid).unwrap(), Some(save));
        assert_eq!(p.load_player(Uuid::from_u128(1)).unwrap(), None);

        p.save_time(123_456).unwrap();
        assert_eq!(p.load_time().unwrap(), Some(123_456));
    }

    #[test]
    fn a_truncated_player_file_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let p = Persistence::open(dir.path()).unwrap();
        let uuid = Uuid::from_u128(9);
        std::fs::write(p.player_path(uuid), [1u8, 0, 0]).unwrap();
        assert!(p.load_player(uuid).is_err());
    }
}
