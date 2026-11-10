use thiserror::Error;

/// Everything that can go wrong while reading or writing the wire protocol.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ProtocolError {
    /// The input ended before the value was complete.
    #[error("unexpected end of input")]
    Eof,
    #[error("VarInt is longer than 5 bytes")]
    VarIntTooLong,
    #[error("VarLong is longer than 10 bytes")]
    VarLongTooLong,
    #[error("string is longer than {max} characters")]
    StringTooLong { max: usize },
    #[error("string is not valid UTF-8")]
    InvalidUtf8,
    #[error("negative length {0}")]
    NegativeLength(i32),
    #[error("length {len} exceeds the limit of {max}")]
    LengthLimit { len: usize, max: usize },
    #[error("{0} trailing bytes after the packet body")]
    TrailingBytes(usize),
    #[error("frame length prefix is wider than {max_bytes} bytes")]
    LengthPrefixTooWide { max_bytes: usize },
    #[error("frame of {len} bytes exceeds the limit of {max}")]
    FrameTooLarge { len: usize, max: usize },
    #[error("compressed packet of {len} bytes is below the compression threshold {threshold}")]
    BelowThreshold { len: usize, threshold: usize },
    #[error("packet decompresses to {len} bytes, above the limit of {max}")]
    DecompressedTooLarge { len: usize, max: usize },
    #[error("zlib stream is corrupt or does not match its declared size")]
    Decompress,
    #[error("compression failed")]
    Compress,
}

pub type Result<T, E = ProtocolError> = core::result::Result<T, E>;
