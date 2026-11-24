use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum NbtError {
    #[error("unexpected end of input")]
    Eof,
    #[error("unknown tag type id {0}")]
    UnknownTag(u8),
    #[error("tag type {0} is not allowed here")]
    UnexpectedTag(u8),
    #[error("negative array length {0}")]
    NegativeLength(i32),
    #[error("list of {len} elements declares the element type TAG_End")]
    EndList { len: i32 },
    #[error("string is not valid modified UTF-8")]
    InvalidString,
    #[error("string of {0} bytes does not fit a 16-bit length prefix")]
    StringTooLong(usize),
    #[error("tag exceeds the size quota of {quota} bytes")]
    QuotaExceeded { quota: usize },
    #[error("tag is nested deeper than {depth} levels")]
    TooDeep { depth: usize },
    #[error("{0} trailing bytes after the root tag")]
    Trailing(usize),
    #[error("list or array length {0} does not fit the format")]
    LengthOverflow(usize),
}

pub type Result<T, E = NbtError> = core::result::Result<T, E>;
