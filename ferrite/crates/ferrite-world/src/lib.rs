//! World storage.
//!
//! * [`PalettedContainer`]: a cube of ids stored as a palette plus bit-packed indices, using
//!   vanilla's packing rule (entries never straddle a 64-bit word), so the same buffers can later
//!   go straight to the network.
//! * [`Chunk`]: a column of 16x16x16 sections of block states and 4x4x4 biome cells.
//! * [`codec`]: chunk <-> bytes, versioned, zstd-compressed, strict about untrusted input.
//! * [`store`]: an append-only, CRC-checked, crash-safe log of chunk records with an in-memory index
//!   and compaction.

#![forbid(unsafe_code)]

pub mod chunk;
mod chunkstore;
pub mod codec;
mod packed;
mod paletted;
mod platform;
pub mod store;

pub use chunk::{Chunk, ChunkPos, Dimension, Section};
pub use chunkstore::{ChunkStore, ChunkStoreError};
pub use codec::{CodecError, Limits};
pub use paletted::{Kind, OwnedRepr, PalettedContainer, Repr};
pub use store::{Options, Store, StoreError};
