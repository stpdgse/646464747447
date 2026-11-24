//! Chunk <-> bytes.
//!
//! ```text
//! zstd( "FCK1" | version u8 | reserved u8 | min_y i32 | height i32 | section * N )
//! section   = blocks container | biomes container
//! container = 0 u8, value u16                                   (every entry equal)
//!           | 1 u8, bits u8, palette_len u16, palette u16*, words u64*   (indirect)
//!           | 2 u8, words u64*                                  (direct, fixed width)
//! ```
//!
//! All integers are little-endian. Containers are written in *canonical form* (see
//! [`PalettedContainer::canonical`]) so equal chunks always encode to equal bytes.
//!
//! Decoding treats its input as hostile: the decompressed size is capped before anything is
//! allocated, every length and index is checked, and ids must be valid for the world.

use thiserror::Error;

use crate::chunk::{Chunk, Dimension, Section};
use crate::paletted::{Kind, OwnedRepr, PalettedContainer, Repr};

const MAGIC: &[u8; 4] = b"FCK1";
const VERSION: u8 = 1;
/// Larger than any legitimate chunk (256 sections of fully direct data is about 2.1 MiB).
const MAX_RAW: usize = 3 * 1024 * 1024;

/// What counts as a valid id in the world being loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Highest valid block-state id.
    pub max_block_state: u16,
    /// Number of biomes; valid ids are `0..biome_count`.
    pub biome_count: u16,
}

impl Limits {
    /// The limits of the game version in `ferrite-data`.
    pub fn vanilla() -> Self {
        let biomes = ferrite_data::configuration::SYNCHRONIZED_REGISTRIES
            .iter()
            .find(|r| r.id == "minecraft:worldgen/biome")
            .map_or(0, |r| r.entries.len());
        Self {
            max_block_state: (ferrite_data::blocks::STATE_COUNT - 1) as u16,
            biome_count: biomes as u16,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CodecError {
    #[error("chunk data is truncated")]
    Truncated,
    #[error("not a chunk (bad magic)")]
    BadMagic,
    #[error("unsupported chunk format version {0}")]
    UnsupportedVersion(u8),
    #[error("invalid dimension: min_y {min_y}, height {height}")]
    BadDimension { min_y: i32, height: i32 },
    #[error("section {section}: {reason}")]
    BadSection { section: usize, reason: &'static str },
    #[error("section {section}: id {id} is not valid in this world")]
    IdOutOfRange { section: usize, id: u16 },
    #[error("chunk decompresses to more than {MAX_RAW} bytes")]
    TooLarge,
    #[error("{0} unexpected bytes after the last section")]
    Trailing(usize),
    #[error("decompression failed: {0}")]
    Compression(String),
}

/// Serialises `chunk` without compression.
pub fn encode_raw(chunk: &Chunk) -> Vec<u8> {
    let dim = chunk.dimension();
    let mut out = Vec::with_capacity(64 + chunk.sections().len() * 64);
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(0);
    out.extend_from_slice(&dim.min_y.to_le_bytes());
    out.extend_from_slice(&dim.height.to_le_bytes());
    for section in chunk.sections() {
        write_container(&mut out, &section.blocks.canonical());
        write_container(&mut out, &section.biomes.canonical());
    }
    out
}

fn write_container(out: &mut Vec<u8>, c: &PalettedContainer) {
    match c.repr() {
        Repr::Single(v) => {
            out.push(0);
            out.extend_from_slice(&v.to_le_bytes());
        }
        Repr::Indirect { palette, data, bits } => {
            out.push(1);
            out.push(bits);
            out.extend_from_slice(&(palette.len() as u16).to_le_bytes());
            palette.iter().for_each(|p| out.extend_from_slice(&p.to_le_bytes()));
            data.iter().for_each(|w| out.extend_from_slice(&w.to_le_bytes()));
        }
        Repr::Direct { data } => {
            out.push(2);
            data.iter().for_each(|w| out.extend_from_slice(&w.to_le_bytes()));
        }
    }
}

/// Serialises and compresses `chunk` with zstd at `level`.
pub fn encode(chunk: &Chunk, level: i32) -> Result<Vec<u8>, CodecError> {
    zstd::bulk::compress(&encode_raw(chunk), level)
        .map_err(|e| CodecError::Compression(e.to_string()))
}

/// Decompresses and parses a chunk.
pub fn decode(bytes: &[u8], limits: &Limits) -> Result<Chunk, CodecError> {
    // The frame header declares the decompressed size, so allocate exactly that, and refuse an
    // oversized declaration before allocating anything. A frame that lies about its size and
    // produces more output than it declared is stopped by the capacity, too.
    let capacity = match zstd::zstd_safe::get_frame_content_size(bytes) {
        Ok(Some(n)) if n as usize <= MAX_RAW => n as usize,
        Ok(Some(_)) => return Err(CodecError::TooLarge),
        // Size not recorded in the frame: fall back to the global cap.
        Ok(None) => MAX_RAW,
        Err(_) => return Err(CodecError::Compression("not a zstd frame".into())),
    };
    let raw = zstd::bulk::decompress(bytes, capacity).map_err(|e| {
        // zstd reports "Destination buffer is too small" when a frame produces more than it said.
        if e.to_string().to_ascii_lowercase().contains("too small") {
            CodecError::TooLarge
        } else {
            CodecError::Compression(e.to_string())
        }
    })?;
    decode_raw(&raw, limits)
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], CodecError> {
        let (head, tail) = self.0.split_at_checked(n).ok_or(CodecError::Truncated)?;
        self.0 = tail;
        Ok(head)
    }
    fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, CodecError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, CodecError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn words(&mut self, n: usize) -> Result<Vec<u64>, CodecError> {
        let bytes = self.take(n.checked_mul(8).ok_or(CodecError::Truncated)?)?;
        Ok(bytes.as_chunks::<8>().0.iter().map(|c| u64::from_le_bytes(*c)).collect())
    }
}

/// Parses an uncompressed chunk.
pub fn decode_raw(raw: &[u8], limits: &Limits) -> Result<Chunk, CodecError> {
    let mut r = Reader(raw);
    if r.take(4)? != MAGIC {
        return Err(CodecError::BadMagic);
    }
    let version = r.u8()?;
    if version != VERSION {
        return Err(CodecError::UnsupportedVersion(version));
    }
    r.u8()?; // reserved
    let (min_y, height) = (r.i32()?, r.i32()?);
    let dimension = Dimension { min_y, height };
    if !dimension.is_valid() {
        return Err(CodecError::BadDimension { min_y, height });
    }

    let mut sections = Vec::with_capacity(dimension.section_count());
    for section in 0..dimension.section_count() {
        let blocks = read_container(&mut r, Kind::Blocks, section)?;
        let biomes = read_container(&mut r, Kind::Biomes, section)?;
        if !blocks.all_ids(|v| v <= limits.max_block_state) {
            let id = blocks
                .iter()
                .find(|&v| v > limits.max_block_state)
                .unwrap_or(limits.max_block_state + 1);
            return Err(CodecError::IdOutOfRange { section, id });
        }
        if !biomes.all_ids(|v| v < limits.biome_count) {
            let id = biomes.iter().find(|&v| v >= limits.biome_count).unwrap_or(limits.biome_count);
            return Err(CodecError::IdOutOfRange { section, id });
        }
        sections.push(Section { blocks, biomes });
    }
    if !r.0.is_empty() {
        return Err(CodecError::Trailing(r.0.len()));
    }
    Chunk::from_sections(dimension, sections).ok_or(CodecError::BadDimension { min_y, height })
}

fn read_container(
    r: &mut Reader<'_>,
    kind: Kind,
    section: usize,
) -> Result<PalettedContainer, CodecError> {
    let bad = |reason| CodecError::BadSection { section, reason };
    let repr = match r.u8()? {
        0 => OwnedRepr::Single(r.u16()?),
        1 => {
            let bits = r.u8()?;
            let palette_len = usize::from(r.u16()?);
            if !(kind.min_bits()..=kind.max_indirect_bits()).contains(&bits) {
                return Err(bad("palette width out of range"));
            }
            if palette_len == 0 || palette_len > 1usize << bits {
                return Err(bad("palette size does not fit its width"));
            }
            let palette = (0..palette_len).map(|_| r.u16()).collect::<Result<Vec<_>, _>>()?;
            let words = crate::packed::PackedArray::words_for(bits, kind.len());
            OwnedRepr::Indirect { palette, data: r.words(words)?, bits }
        }
        2 => OwnedRepr::Direct {
            data: r.words(crate::packed::PackedArray::words_for(kind.direct_bits(), kind.len()))?,
        },
        _ => return Err(bad("unknown container kind")),
    };
    PalettedContainer::from_repr(kind, repr).ok_or(bad("inconsistent container"))
}

/// Compresses arbitrary bytes the way chunk payloads are. Only for tests that need to build
/// hostile frames (such as a decompression bomb) without depending on zstd directly.
#[doc(hidden)]
pub fn compress_for_tests(data: &[u8]) -> Vec<u8> {
    zstd::bulk::compress(data, 3).expect("compressing test data")
}
