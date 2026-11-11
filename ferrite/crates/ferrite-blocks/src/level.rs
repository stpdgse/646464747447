//! Block access for the behaviour code.

use std::collections::HashMap;

use ferrite_world::{Chunk, ChunkPos, Dimension};

use crate::pos::BlockPos;

/// Read/write access to block states.
///
/// `block` returns `None` for positions that are not loaded or are outside the build height;
/// behaviour code treats those as not existing (it neither reads through nor writes to them),
/// which is what keeps simulation inside loaded chunks.
pub trait BlockAccess {
    fn block(&self, pos: BlockPos) -> Option<u16>;
    /// Stores a state without any side effects. Returns the previous state, or `None` (having
    /// changed nothing) if the position is not loaded.
    fn set_block_raw(&mut self, pos: BlockPos, state: u16) -> Option<u16>;
    fn dimension(&self) -> Dimension;
}

/// A set of loaded chunks.
#[derive(Clone, Debug)]
pub struct Chunks {
    dimension: Dimension,
    chunks: HashMap<ChunkPos, Chunk>,
}

impl Chunks {
    pub fn new(dimension: Dimension) -> Self {
        Self { dimension, chunks: HashMap::new() }
    }

    pub fn insert(&mut self, pos: ChunkPos, chunk: Chunk) -> Option<Chunk> {
        assert_eq!(chunk.dimension(), self.dimension, "chunk belongs to another dimension");
        self.chunks.insert(pos, chunk)
    }

    pub fn remove(&mut self, pos: ChunkPos) -> Option<Chunk> {
        self.chunks.remove(&pos)
    }

    pub fn get(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos)
    }

    pub fn is_loaded(&self, pos: ChunkPos) -> bool {
        self.chunks.contains_key(&pos)
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }
}

impl BlockAccess for Chunks {
    #[inline]
    fn block(&self, pos: BlockPos) -> Option<u16> {
        self.chunks.get(&pos.chunk())?.block(pos.x & 15, pos.y, pos.z & 15)
    }

    #[inline]
    fn set_block_raw(&mut self, pos: BlockPos, state: u16) -> Option<u16> {
        let chunk = self.chunks.get_mut(&pos.chunk())?;
        let old = chunk.block(pos.x & 15, pos.y, pos.z & 15)?;
        if old != state {
            chunk.set_block(pos.x & 15, pos.y, pos.z & 15, state);
        }
        Some(old)
    }

    fn dimension(&self) -> Dimension {
        self.dimension
    }
}
