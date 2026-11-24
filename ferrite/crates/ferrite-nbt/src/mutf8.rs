//! Java's "modified UTF-8": NUL is `C0 80`, and supplementary characters are written as two
//! 3-byte surrogates (CESU-8) instead of one 4-byte sequence.

use crate::error::{NbtError, Result};

/// Decodes modified UTF-8. Unpaired surrogates cannot be represented in a Rust `String` and are
/// rejected rather than silently altered.
pub fn decode(bytes: &[u8]) -> Result<String> {
    // Fast path: plain UTF-8 without NUL is valid modified UTF-8 and needs no conversion.
    if bytes.is_ascii() {
        // ASCII has no C0 80 and no multi-byte sequences; a literal 0x00 is also accepted by Java.
        return Ok(String::from_utf8_lossy(bytes).into_owned());
    }
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    let mut pending_high: Option<u16> = None;
    while i < bytes.len() {
        let b = bytes[i];
        let (unit, len): (u32, usize) = match b {
            0x00..=0x7F => (u32::from(b), 1),
            0xC0..=0xDF => {
                let c = *bytes.get(i + 1).ok_or(NbtError::InvalidString)?;
                if c & 0xC0 != 0x80 {
                    return Err(NbtError::InvalidString);
                }
                let v = (u32::from(b & 0x1F) << 6) | u32::from(c & 0x3F);
                // Only C0 80 may encode below U+0080 (overlong otherwise).
                if v < 0x80 && v != 0 {
                    return Err(NbtError::InvalidString);
                }
                (v, 2)
            }
            0xE0..=0xEF => {
                let (c1, c2) = (
                    *bytes.get(i + 1).ok_or(NbtError::InvalidString)?,
                    *bytes.get(i + 2).ok_or(NbtError::InvalidString)?,
                );
                if c1 & 0xC0 != 0x80 || c2 & 0xC0 != 0x80 {
                    return Err(NbtError::InvalidString);
                }
                let v = (u32::from(b & 0x0F) << 12)
                    | (u32::from(c1 & 0x3F) << 6)
                    | u32::from(c2 & 0x3F);
                if v < 0x800 {
                    return Err(NbtError::InvalidString);
                }
                (v, 3)
            }
            // 4-byte sequences and stray continuation bytes do not exist in modified UTF-8.
            _ => return Err(NbtError::InvalidString),
        };
        i += len;

        match (pending_high.take(), unit) {
            (Some(high), 0xDC00..=0xDFFF) => {
                let cp = 0x10000 + ((u32::from(high) - 0xD800) << 10) + (unit - 0xDC00);
                out.push(char::from_u32(cp).ok_or(NbtError::InvalidString)?);
            }
            (Some(_), _) => return Err(NbtError::InvalidString),
            (None, 0xD800..=0xDBFF) => pending_high = Some(unit as u16),
            (None, 0xDC00..=0xDFFF) => return Err(NbtError::InvalidString),
            (None, u) => out.push(char::from_u32(u).ok_or(NbtError::InvalidString)?),
        }
    }
    if pending_high.is_some() {
        return Err(NbtError::InvalidString);
    }
    Ok(out)
}

/// Encoded length in bytes, without allocating.
pub fn encoded_len(s: &str) -> usize {
    s.chars()
        .map(|c| match c as u32 {
            0 => 2,
            1..=0x7F => 1,
            0x80..=0x7FF => 2,
            0x800..=0xFFFF => 3,
            _ => 6,
        })
        .sum()
}

pub fn encode(s: &str, out: &mut Vec<u8>) {
    if s.bytes().all(|b| b != 0 && b < 0x80) {
        out.extend_from_slice(s.as_bytes());
        return;
    }
    for c in s.chars() {
        let cp = c as u32;
        match cp {
            0 => out.extend_from_slice(&[0xC0, 0x80]),
            1..=0x7F => out.push(cp as u8),
            0x80..=0x7FF => {
                out.extend_from_slice(&[0xC0 | (cp >> 6) as u8, 0x80 | (cp & 0x3F) as u8])
            }
            0x800..=0xFFFF => push3(out, cp),
            _ => {
                let v = cp - 0x10000;
                push3(out, 0xD800 + (v >> 10));
                push3(out, 0xDC00 + (v & 0x3FF));
            }
        }
    }
}

fn push3(out: &mut Vec<u8>, cp: u32) {
    out.extend_from_slice(&[
        0xE0 | (cp >> 12) as u8,
        0x80 | ((cp >> 6) & 0x3F) as u8,
        0x80 | (cp & 0x3F) as u8,
    ]);
}
