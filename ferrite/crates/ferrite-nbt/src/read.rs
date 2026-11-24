use crate::error::{NbtError, Result};
use crate::tag::{Compound, List, Tag};
use crate::{id, mutf8};

/// Vanilla's per-read limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Approximate deserialised size in bytes (see the accounting table in the NBT spec).
    pub quota: usize,
    /// Maximum nesting of lists and compounds.
    pub depth: usize,
}

impl Default for Limits {
    /// Vanilla's defaults for network NBT: 2 MiB quota and depth 512.
    fn default() -> Self {
        Self { quota: 2 * 1024 * 1024, depth: 512 }
    }
}

impl Limits {
    pub const fn new(quota: usize, depth: usize) -> Self {
        Self { quota, depth }
    }
    /// No effective limit; for trusted local files only.
    pub const UNLIMITED: Limits = Limits { quota: usize::MAX, depth: 512 };
}

struct Reader<'a> {
    buf: &'a [u8],
    quota_left: usize,
    quota: usize,
    max_depth: usize,
}

/// Reads a network-format root tag. `Ok(None)` means the root was `TAG_End` ("no tag").
///
/// Advances `buf` past the tag. Trailing bytes are left in `buf` for the caller.
pub fn read_network(buf: &mut &[u8], limits: Limits) -> Result<Option<Tag>> {
    let mut r =
        Reader { buf, quota_left: limits.quota, quota: limits.quota, max_depth: limits.depth };
    let ty = r.u8()?;
    if ty == id::END {
        *buf = r.buf;
        return Ok(None);
    }
    let tag = r.payload(ty, 0)?;
    *buf = r.buf;
    Ok(Some(tag))
}

/// Reads a root tag with a name (the on-disk format). Returns the name and the tag.
pub fn read_named(buf: &mut &[u8], limits: Limits) -> Result<(String, Tag)> {
    let mut r =
        Reader { buf, quota_left: limits.quota, quota: limits.quota, max_depth: limits.depth };
    let ty = r.u8()?;
    if ty == id::END {
        return Err(NbtError::UnexpectedTag(ty));
    }
    let name = r.string()?;
    let tag = r.payload(ty, 0)?;
    *buf = r.buf;
    Ok((name, tag))
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let (head, tail) = self.buf.split_at_checked(n).ok_or(NbtError::Eof)?;
        self.buf = tail;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn account(&mut self, bytes: usize) -> Result<()> {
        self.quota_left = self
            .quota_left
            .checked_sub(bytes)
            .ok_or(NbtError::QuotaExceeded { quota: self.quota })?;
        Ok(())
    }

    fn string(&mut self) -> Result<String> {
        let len = u16::from_be_bytes(self.take(2)?.try_into().unwrap()) as usize;
        mutf8::decode(self.take(len)?)
    }

    fn length(&mut self) -> Result<usize> {
        let len = i32::from_be_bytes(self.take(4)?.try_into().unwrap());
        usize::try_from(len).map_err(|_| NbtError::NegativeLength(len))
    }

    /// A primitive array: account first, then bound by the bytes actually present.
    fn array<T>(
        &mut self,
        width: usize,
        quota_base: usize,
        f: impl Fn(&[u8]) -> T,
    ) -> Result<Vec<T>> {
        let len = self.length()?;
        self.account(quota_base + width.saturating_mul(len))?;
        let bytes = self.take(len.checked_mul(width).ok_or(NbtError::Eof)?)?;
        Ok(bytes.chunks_exact(width).map(f).collect())
    }

    fn payload(&mut self, ty: u8, depth: usize) -> Result<Tag> {
        Ok(match ty {
            id::BYTE => {
                self.account(9)?;
                Tag::Byte(self.u8()? as i8)
            }
            id::SHORT => {
                self.account(10)?;
                Tag::Short(i16::from_be_bytes(self.take(2)?.try_into().unwrap()))
            }
            id::INT => {
                self.account(12)?;
                Tag::Int(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
            }
            id::LONG => {
                self.account(16)?;
                Tag::Long(i64::from_be_bytes(self.take(8)?.try_into().unwrap()))
            }
            id::FLOAT => {
                self.account(12)?;
                Tag::Float(f32::from_be_bytes(self.take(4)?.try_into().unwrap()))
            }
            id::DOUBLE => {
                self.account(16)?;
                Tag::Double(f64::from_be_bytes(self.take(8)?.try_into().unwrap()))
            }
            id::BYTE_ARRAY => Tag::ByteArray(self.array(1, 24, |b| b[0] as i8)?),
            id::STRING => {
                let s = self.string()?;
                self.account(36 + 2 * s.encode_utf16().count())?;
                Tag::String(s)
            }
            id::LIST => Tag::List(self.list(depth + 1)?),
            id::COMPOUND => Tag::Compound(self.compound(depth + 1)?),
            id::INT_ARRAY => {
                Tag::IntArray(self.array(4, 24, |b| i32::from_be_bytes(b.try_into().unwrap()))?)
            }
            id::LONG_ARRAY => {
                Tag::LongArray(self.array(8, 24, |b| i64::from_be_bytes(b.try_into().unwrap()))?)
            }
            other => return Err(NbtError::UnknownTag(other)),
        })
    }

    fn enter(&self, depth: usize) -> Result<()> {
        if depth > self.max_depth {
            return Err(NbtError::TooDeep { depth: self.max_depth });
        }
        Ok(())
    }

    fn compound(&mut self, depth: usize) -> Result<Compound> {
        self.enter(depth)?;
        self.account(48)?;
        let mut map = indexmap::IndexMap::new();
        loop {
            let ty = self.u8()?;
            if ty == id::END {
                return Ok(Compound(map));
            }
            let key = self.string()?;
            self.account(64 + 2 * key.encode_utf16().count())?;
            let value = self.payload(ty, depth)?;
            map.insert(key, value);
        }
    }

    fn list(&mut self, depth: usize) -> Result<List> {
        self.enter(depth)?;
        let elem = self.u8()?;
        let len_raw = i32::from_be_bytes(self.take(4)?.try_into().unwrap());
        let len = usize::try_from(len_raw).unwrap_or(0);
        self.account(36)?;
        if len == 0 {
            // Any element type is accepted for an empty (or negative-length) list.
            return Ok(List::Empty);
        }
        if elem == id::END {
            return Err(NbtError::EndList { len: len_raw });
        }
        self.account(4usize.saturating_mul(len))?;

        macro_rules! fixed {
            ($variant:ident, $width:expr, $ty:ty, $conv:expr) => {{
                let bytes = self.take(len.checked_mul($width).ok_or(NbtError::Eof)?)?;
                List::$variant(bytes.chunks_exact($width).map(|b| -> $ty { $conv(b) }).collect())
            }};
        }
        // The per-element quota of fixed-size types is accounted in one step, before reading.
        Ok(match elem {
            id::BYTE => {
                self.account(len.saturating_mul(9))?;
                fixed!(Byte, 1, i8, |b: &[u8]| b[0] as i8)
            }
            id::SHORT => {
                self.account(len.saturating_mul(10))?;
                fixed!(Short, 2, i16, |b: &[u8]| i16::from_be_bytes(b.try_into().unwrap()))
            }
            id::INT => {
                self.account(len.saturating_mul(12))?;
                fixed!(Int, 4, i32, |b: &[u8]| i32::from_be_bytes(b.try_into().unwrap()))
            }
            id::LONG => {
                self.account(len.saturating_mul(16))?;
                fixed!(Long, 8, i64, |b: &[u8]| i64::from_be_bytes(b.try_into().unwrap()))
            }
            id::FLOAT => {
                self.account(len.saturating_mul(12))?;
                fixed!(Float, 4, f32, |b: &[u8]| f32::from_be_bytes(b.try_into().unwrap()))
            }
            id::DOUBLE => {
                self.account(len.saturating_mul(16))?;
                fixed!(Double, 8, f64, |b: &[u8]| f64::from_be_bytes(b.try_into().unwrap()))
            }
            id::BYTE_ARRAY => List::ByteArray(self.items(len, |r| r.array(1, 24, |b| b[0] as i8))?),
            id::STRING => List::String(self.items(len, |r| {
                let s = r.string()?;
                r.account(36 + 2 * s.encode_utf16().count())?;
                Ok(s)
            })?),
            id::LIST => List::List(self.items(len, |r| r.list(depth + 1))?),
            id::COMPOUND => List::Compound(self.items(len, |r| r.compound(depth + 1))?),
            id::INT_ARRAY => {
                List::IntArray(self.items(len, |r| {
                    r.array(4, 24, |b| i32::from_be_bytes(b.try_into().unwrap()))
                })?)
            }
            id::LONG_ARRAY => {
                List::LongArray(self.items(len, |r| {
                    r.array(8, 24, |b| i64::from_be_bytes(b.try_into().unwrap()))
                })?)
            }
            other => return Err(NbtError::UnknownTag(other)),
        })
    }

    /// Reads `len` items, growing the vector as data actually arrives rather than trusting `len`.
    fn items<T>(
        &mut self,
        len: usize,
        mut f: impl FnMut(&mut Self) -> Result<T>,
    ) -> Result<Vec<T>> {
        let mut out = Vec::with_capacity(len.min(self.buf.len()));
        for _ in 0..len {
            out.push(f(self)?);
        }
        Ok(out)
    }
}
