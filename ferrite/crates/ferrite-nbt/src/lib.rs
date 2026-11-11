//! Named Binary Tag (NBT).
//!
//! Two root encodings exist and both are supported:
//!
//! * **Network** (since protocol 764): the root tag has *no name*, and any tag type may be the root
//!   (text components are sent as a bare string tag). A root of `TAG_End` means "absent".
//! * **Named** (world files, `level.dat`): the root is a compound with a name.
//!
//! Reading enforces vanilla's *quota* (an estimate of the deserialised memory) and *depth* limits
//! **before** allocating, so a few hostile bytes cannot demand gigabytes.

mod error;
mod mutf8;
pub mod parse;
mod read;
pub mod snbt;
mod tag;
mod write;

pub use error::{NbtError, Result};
pub use read::{Limits, read_named, read_network};
pub use tag::{Compound, List, Tag};
pub use write::{write_named, write_network};

/// Tag type ids as they appear on the wire.
pub mod id {
    pub const END: u8 = 0;
    pub const BYTE: u8 = 1;
    pub const SHORT: u8 = 2;
    pub const INT: u8 = 3;
    pub const LONG: u8 = 4;
    pub const FLOAT: u8 = 5;
    pub const DOUBLE: u8 = 6;
    pub const BYTE_ARRAY: u8 = 7;
    pub const STRING: u8 = 8;
    pub const LIST: u8 = 9;
    pub const COMPOUND: u8 = 10;
    pub const INT_ARRAY: u8 = 11;
    pub const LONG_ARRAY: u8 = 12;
}
