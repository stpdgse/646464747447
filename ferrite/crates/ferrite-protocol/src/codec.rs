//! Zero-copy encoding and decoding of protocol values.
//!
//! [`Decode`] borrows from the input slice, so strings and byte arrays are never copied while a
//! packet is being parsed. [`Encode`] writes into any [`bytes::BufMut`].
//!
//! Anything whose length is chosen by the peer ([`read_str`], [`read_byte_array`],
//! [`read_list`]) requires an explicit upper bound so a packet definition cannot forget one.

use bytes::BufMut;
use uuid::Uuid;

use crate::error::{ProtocolError, Result};
use crate::varint;

/// Writes a value in wire format.
pub trait Encode {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()>;
}

/// Reads a value in wire format, borrowing from the input where possible.
pub trait Decode<'a>: Sized {
    fn decode(buf: &mut &'a [u8]) -> Result<Self>;
}

/// Decodes a `T` from `buf`, requiring that the whole slice is consumed.
pub fn decode_exact<'a, T: Decode<'a>>(mut buf: &'a [u8]) -> Result<T> {
    let value = T::decode(&mut buf)?;
    if buf.is_empty() { Ok(value) } else { Err(ProtocolError::TrailingBytes(buf.len())) }
}

#[inline]
fn take<'a, const N: usize>(buf: &mut &'a [u8]) -> Result<&'a [u8; N]> {
    let (head, tail) = buf.split_first_chunk::<N>().ok_or(ProtocolError::Eof)?;
    *buf = tail;
    Ok(head)
}

#[inline]
fn take_slice<'a>(buf: &mut &'a [u8], n: usize) -> Result<&'a [u8]> {
    let (head, tail) = buf.split_at_checked(n).ok_or(ProtocolError::Eof)?;
    *buf = tail;
    Ok(head)
}

macro_rules! impl_be {
    ($($t:ty),*) => {$(
        impl Encode for $t {
            #[inline]
            fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
                buf.put_slice(&self.to_be_bytes());
                Ok(())
            }
        }
        impl<'a> Decode<'a> for $t {
            #[inline]
            fn decode(buf: &mut &'a [u8]) -> Result<Self> {
                Ok(<$t>::from_be_bytes(*take(buf)?))
            }
        }
    )*};
}
impl_be!(u8, i8, u16, i16, u32, i32, u64, i64, u128, f32, f64);

impl Encode for bool {
    #[inline]
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        buf.put_u8(u8::from(*self));
        Ok(())
    }
}
impl<'a> Decode<'a> for bool {
    /// Like vanilla, any non-zero byte is `true`.
    #[inline]
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(u8::decode(buf)? != 0)
    }
}

/// A variable-length 32-bit integer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarInt(pub i32);

impl Encode for VarInt {
    #[inline]
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        varint::put_i32(buf, self.0);
        Ok(())
    }
}
impl<'a> Decode<'a> for VarInt {
    #[inline]
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        varint::read_i32(buf).map(VarInt)
    }
}

/// A variable-length 64-bit integer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarLong(pub i64);

impl Encode for VarLong {
    #[inline]
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        let (bytes, n) = varint::encode_i64(self.0);
        buf.put_slice(&bytes[..n]);
        Ok(())
    }
}
impl<'a> Decode<'a> for VarLong {
    #[inline]
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        varint::read_i64(buf).map(VarLong)
    }
}

impl Encode for Uuid {
    #[inline]
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        buf.put_slice(self.as_bytes());
        Ok(())
    }
}
impl<'a> Decode<'a> for Uuid {
    #[inline]
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(Uuid::from_bytes(*take(buf)?))
    }
}

impl<const N: usize> Encode for [u8; N] {
    #[inline]
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        buf.put_slice(self);
        Ok(())
    }
}
impl<'a, const N: usize> Decode<'a> for [u8; N] {
    #[inline]
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        Ok(*take(buf)?)
    }
}

impl<T: Encode> Encode for Option<T> {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        match self {
            Some(v) => {
                true.encode(buf)?;
                v.encode(buf)
            }
            None => false.encode(buf),
        }
    }
}
impl<'a, T: Decode<'a>> Decode<'a> for Option<T> {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        if bool::decode(buf)? { T::decode(buf).map(Some) } else { Ok(None) }
    }
}

/// Maximum string length, in UTF-16 code units, unless a packet says otherwise.
pub const DEFAULT_MAX_STRING: usize = 32767;

/// Length of `s` in UTF-16 code units (what Java's `String.length()` reports).
fn utf16_len(s: &str) -> usize {
    if s.is_ascii() { s.len() } else { s.chars().map(char::len_utf16).sum() }
}

/// Reads a length-prefixed UTF-8 string of at most `max_chars` UTF-16 code units.
pub fn read_str<'a>(buf: &mut &'a [u8], max_chars: usize) -> Result<&'a str> {
    let len = varint::read_i32(buf)?;
    let len = usize::try_from(len).map_err(|_| ProtocolError::NegativeLength(len))?;
    // A UTF-16 unit takes at most 3 UTF-8 bytes; reject before touching the payload.
    if len > max_chars.saturating_mul(3) {
        return Err(ProtocolError::StringTooLong { max: max_chars });
    }
    let s = std::str::from_utf8(take_slice(buf, len)?).map_err(|_| ProtocolError::InvalidUtf8)?;
    if len > max_chars && utf16_len(s) > max_chars {
        return Err(ProtocolError::StringTooLong { max: max_chars });
    }
    Ok(s)
}

/// Writes a length-prefixed UTF-8 string, enforcing the same bound as [`read_str`].
pub fn write_str<B: BufMut>(buf: &mut B, s: &str, max_chars: usize) -> Result<()> {
    if s.len() > max_chars.saturating_mul(3) || (s.len() > max_chars && utf16_len(s) > max_chars) {
        return Err(ProtocolError::StringTooLong { max: max_chars });
    }
    VarInt(s.len() as i32).encode(buf)?;
    buf.put_slice(s.as_bytes());
    Ok(())
}

/// A string with a compile-time length bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BoundedStr<'a, const MAX: usize = DEFAULT_MAX_STRING>(pub &'a str);

impl<const MAX: usize> Encode for BoundedStr<'_, MAX> {
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        write_str(buf, self.0, MAX)
    }
}
impl<'a, const MAX: usize> Decode<'a> for BoundedStr<'a, MAX> {
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        read_str(buf, MAX).map(BoundedStr)
    }
}

/// Reads a VarInt-prefixed byte array of at most `max` bytes.
pub fn read_byte_array<'a>(buf: &mut &'a [u8], max: usize) -> Result<&'a [u8]> {
    let len = varint::read_i32(buf)?;
    let len = usize::try_from(len).map_err(|_| ProtocolError::NegativeLength(len))?;
    if len > max {
        return Err(ProtocolError::LengthLimit { len, max });
    }
    take_slice(buf, len)
}

/// Writes a VarInt-prefixed byte array of at most `max` bytes.
pub fn write_byte_array<B: BufMut>(buf: &mut B, bytes: &[u8], max: usize) -> Result<()> {
    if bytes.len() > max {
        return Err(ProtocolError::LengthLimit { len: bytes.len(), max });
    }
    VarInt(bytes.len() as i32).encode(buf)?;
    buf.put_slice(bytes);
    Ok(())
}

/// Reads a VarInt-prefixed list of at most `max` items.
///
/// Every item must occupy at least one byte on the wire, which bounds the work an attacker can
/// force with a large prefix by the size of the packet itself.
pub fn read_list<'a, T: Decode<'a>>(buf: &mut &'a [u8], max: usize) -> Result<Vec<T>> {
    let len = varint::read_i32(buf)?;
    let len = usize::try_from(len).map_err(|_| ProtocolError::NegativeLength(len))?;
    if len > max {
        return Err(ProtocolError::LengthLimit { len, max });
    }
    let mut out = Vec::with_capacity(len.min(buf.len()));
    for _ in 0..len {
        let before = buf.len();
        out.push(T::decode(buf)?);
        if buf.len() == before {
            // A zero-width item would let a tiny packet demand `max` iterations of nothing.
            return Err(ProtocolError::LengthLimit { len, max: 0 });
        }
    }
    Ok(out)
}

/// Writes a VarInt-prefixed list of at most `max` items.
pub fn write_list<B: BufMut, T: Encode>(buf: &mut B, items: &[T], max: usize) -> Result<()> {
    if items.len() > max {
        return Err(ProtocolError::LengthLimit { len: items.len(), max });
    }
    VarInt(items.len() as i32).encode(buf)?;
    items.iter().try_for_each(|item| item.encode(buf))
}

/// A block position, packed into a single `i64` on the wire (x: 26 bits, z: 26 bits, y: 12 bits).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Packs the position. Coordinates outside the representable range wrap, as in vanilla.
    #[inline]
    pub const fn pack(self) -> i64 {
        ((self.x as i64 & 0x3FF_FFFF) << 38)
            | ((self.z as i64 & 0x3FF_FFFF) << 12)
            | (self.y as i64 & 0xFFF)
    }

    #[inline]
    pub const fn unpack(v: i64) -> Self {
        Self { x: (v >> 38) as i32, y: ((v << 52) >> 52) as i32, z: ((v << 26) >> 38) as i32 }
    }
}

impl Encode for BlockPos {
    #[inline]
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<()> {
        self.pack().encode(buf)
    }
}
impl<'a> Decode<'a> for BlockPos {
    #[inline]
    fn decode(buf: &mut &'a [u8]) -> Result<Self> {
        i64::decode(buf).map(BlockPos::unpack)
    }
}
