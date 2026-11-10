//! VarInt and VarLong: little-endian base-128 integers.
//!
//! Semantics follow the vanilla implementation exactly: at most 5 (resp. 10) bytes, and bits that
//! overflow the integer width in the final byte are discarded rather than rejected.

use crate::error::{ProtocolError, Result};

pub const MAX_VARINT_LEN: usize = 5;
pub const MAX_VARLONG_LEN: usize = 10;

/// Outcome of decoding from a buffer that may not hold the whole value yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Peek<T> {
    /// A complete value and the number of bytes it occupied.
    Done(T, usize),
    /// More bytes are needed.
    Incomplete,
}

/// Number of bytes `value` occupies as a VarInt.
#[inline]
pub const fn len_i32(value: i32) -> usize {
    // 9/64 approximates 1/7 per leading-zero step; exact for every 32-bit input.
    ((31 - (value as u32 | 1).leading_zeros()) * 9 + 73) as usize >> 6
}

/// Number of bytes `value` occupies as a VarLong.
#[inline]
pub const fn len_i64(value: i64) -> usize {
    ((63 - (value as u64 | 1).leading_zeros()) * 9 + 73) as usize >> 6
}

/// Encodes `value` branch-free: the 7-bit groups are spread into byte lanes of a `u64` and the
/// continuation bits are set on every byte but the last. Returns the little-endian lanes (only
/// the first `len` bytes are meaningful) and `len`.
#[inline]
pub const fn encode_i32_wide(value: i32) -> (u64, usize) {
    let v = value as u32 as u64;
    let spread = (v & 0x7F)
        | ((v & 0x3F80) << 1)
        | ((v & 0x1F_C000) << 2)
        | ((v & 0xFE0_0000) << 3)
        | ((v & 0xF000_0000) << 4);
    let len = len_i32(value);
    let continuation = 0x80_8080_8080u64 & ((1u64 << (8 * (len - 1))) - 1);
    (spread | continuation, len)
}

/// Encodes `value`, returning the buffer and the number of bytes used.
#[inline]
pub fn encode_i32(value: i32) -> ([u8; MAX_VARINT_LEN], usize) {
    let (wide, len) = encode_i32_wide(value);
    let b = wide.to_le_bytes();
    ([b[0], b[1], b[2], b[3], b[4]], len)
}

/// Appends `value` as a VarInt with a single unaligned 8-byte store when the buffer has room,
/// instead of a variable-length copy.
#[inline]
pub fn put_i32<B: bytes::BufMut>(buf: &mut B, value: i32) {
    let (wide, len) = encode_i32_wide(value);
    let chunk = buf.chunk_mut();
    if chunk.len() >= 8 {
        // SAFETY: `chunk` has at least 8 writable bytes, so the 8-byte store is in bounds, and
        // `len <= 5 <= 8` of those bytes are initialised before they are marked as written.
        unsafe {
            chunk.as_mut_ptr().cast::<u64>().write_unaligned(wide.to_le());
            buf.advance_mut(len);
        }
    } else {
        buf.put_slice(&wide.to_le_bytes()[..len]);
    }
}

/// Encodes `value`, returning the buffer and the number of bytes used.
#[inline]
pub fn encode_i64(value: i64) -> ([u8; MAX_VARLONG_LEN], usize) {
    let mut v = value as u64;
    let mut out = [0u8; MAX_VARLONG_LEN];
    let mut i = 0;
    while v >= 0x80 {
        out[i] = (v as u8) | 0x80;
        v >>= 7;
        i += 1;
    }
    out[i] = v as u8;
    (out, i + 1)
}

/// Decodes a VarInt from the front of `buf` without consuming it.
#[inline]
pub fn peek_i32(buf: &[u8]) -> Result<Peek<i32>> {
    let mut value = 0u32;
    for i in 0..MAX_VARINT_LEN {
        let Some(&byte) = buf.get(i) else { return Ok(Peek::Incomplete) };
        value |= u32::from(byte & 0x7F) << (7 * i);
        if byte & 0x80 == 0 {
            return Ok(Peek::Done(value as i32, i + 1));
        }
    }
    Err(ProtocolError::VarIntTooLong)
}

/// Decodes a VarLong from the front of `buf` without consuming it.
#[inline]
pub fn peek_i64(buf: &[u8]) -> Result<Peek<i64>> {
    let mut value = 0u64;
    for i in 0..MAX_VARLONG_LEN {
        let Some(&byte) = buf.get(i) else { return Ok(Peek::Incomplete) };
        value |= u64::from(byte & 0x7F) << (7 * i);
        if byte & 0x80 == 0 {
            return Ok(Peek::Done(value as i64, i + 1));
        }
    }
    Err(ProtocolError::VarLongTooLong)
}

/// Decodes a VarInt that must be fully present, advancing `buf`.
#[inline]
pub fn read_i32(buf: &mut &[u8]) -> Result<i32> {
    match peek_i32(buf)? {
        Peek::Done(v, n) => {
            *buf = &buf[n..];
            Ok(v)
        }
        Peek::Incomplete => Err(ProtocolError::Eof),
    }
}

/// Decodes a VarLong that must be fully present, advancing `buf`.
#[inline]
pub fn read_i64(buf: &mut &[u8]) -> Result<i64> {
    match peek_i64(buf)? {
        Peek::Done(v, n) => {
            *buf = &buf[n..];
            Ok(v)
        }
        Peek::Incomplete => Err(ProtocolError::Eof),
    }
}
