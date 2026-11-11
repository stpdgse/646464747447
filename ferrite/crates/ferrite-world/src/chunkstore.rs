use std::path::Path;

use thiserror::Error;

use crate::chunk::{Chunk, ChunkPos};
use crate::codec::{self, CodecError, Limits};
use crate::store::{Options, Store, StoreError};

#[derive(Debug, Error)]
pub enum ChunkStoreError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("chunk ({}, {}) could not be decoded: {source}", pos.x, pos.z)]
    Codec { pos: ChunkPos, source: CodecError },
    #[error(transparent)]
    Encode(CodecError),
}

/// Chunks on disk: the [codec](crate::codec) on top of the [`Store`].
#[derive(Debug)]
pub struct ChunkStore {
    store: Store,
    limits: Limits,
    level: i32,
}

impl ChunkStore {
    /// zstd level 3 is zstd's default and, measured on this crate's benchmark, the best trade of
    /// size against speed for chunk data.
    pub const DEFAULT_LEVEL: i32 = 3;

    pub fn open(
        dir: impl AsRef<Path>,
        opts: Options,
        limits: Limits,
    ) -> Result<Self, ChunkStoreError> {
        Ok(Self { store: Store::open(dir, opts)?, limits, level: Self::DEFAULT_LEVEL })
    }

    pub fn with_level(mut self, level: i32) -> Self {
        self.level = level;
        self
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn save(&self, pos: ChunkPos, chunk: &Chunk) -> Result<(), ChunkStoreError> {
        let bytes = codec::encode(chunk, self.level).map_err(ChunkStoreError::Encode)?;
        Ok(self.store.put(pos, &bytes)?)
    }

    pub fn load(&self, pos: ChunkPos) -> Result<Option<Chunk>, ChunkStoreError> {
        let Some(bytes) = self.store.get(pos)? else { return Ok(None) };
        codec::decode(&bytes, &self.limits)
            .map(Some)
            .map_err(|source| ChunkStoreError::Codec { pos, source })
    }

    pub fn delete(&self, pos: ChunkPos) -> Result<bool, ChunkStoreError> {
        Ok(self.store.delete(pos)?)
    }
}
