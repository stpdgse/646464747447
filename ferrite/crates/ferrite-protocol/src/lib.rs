//! The Minecraft Java Edition wire protocol, from bytes up to packets.
//!
//! * [`varint`], [`codec`]: value encoding, zero-copy decoding with mandatory length limits.
//! * [`frame`]: length prefixes, zlib compression (libdeflate) and packet ids.
//! * [`cipher`]: AES-128-CFB8 with a pipelined decryptor.
//!
//! Everything here is synchronous and allocation-conscious; it knows nothing about sockets or
//! runtimes, so it can be driven from tokio, io_uring or a test.

pub mod cipher;
pub mod codec;
pub mod error;
pub mod frame;
pub mod varint;

pub use codec::{BlockPos, BoundedStr, Decode, Encode, VarInt, VarLong};
pub use error::{ProtocolError, Result};
pub use frame::{Compression, Frame, FrameDecoder, FrameEncoder};

pub use ferrite_data::{Direction, PROTOCOL_VERSION, State};
