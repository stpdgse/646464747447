//! Chunks in the network format.
//!
//! A chunk packet carries the sections concatenated: for each one a count of non-air blocks, a
//! count of fluid blocks, the block states and the biomes, each as a paletted container. [`PalettedContainer`] already stores
//! its data the way the wire wants it, so encoding is mostly copying.

use ferrite_data::blocks::block_by_name;
use ferrite_data::shapes::{facts, flag};
use ferrite_protocol::codec::VarInt;
use ferrite_protocol::packets::play::{BitSetBytes, Heightmap, LightData};
use ferrite_protocol::types::{ByteVec, LenVec};
use ferrite_world::{Chunk, Dimension, Kind, OwnedRepr, PalettedContainer, Repr, Section};

use crate::flat::air_states;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ChunkDecodeError {
    #[error("the chunk data ends early")]
    Truncated,
    #[error("a paletted container is malformed")]
    BadContainer,
    #[error("the chunk data has {0} bytes left over")]
    Trailing(usize),
}

fn put_varint(out: &mut Vec<u8>, mut v: u32) {
    while v >= 0x80 {
        out.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn write_container(out: &mut Vec<u8>, c: &PalettedContainer) {
    match c.repr() {
        Repr::Single(v) => {
            out.push(0);
            put_varint(out, u32::from(v));
        }
        Repr::Indirect { palette, data, bits } => {
            out.push(bits);
            put_varint(out, palette.len() as u32);
            for &p in palette {
                put_varint(out, u32::from(p));
            }
            // No length prefix: the word count follows from the width.
            for w in data {
                out.extend_from_slice(&w.to_be_bytes());
            }
        }
        Repr::Direct { data } => {
            out.push(c.kind().direct_bits());
            for w in data {
                out.extend_from_slice(&w.to_be_bytes());
            }
        }
    }
}

/// Number of blocks in a section that hold a fluid (water, lava, waterlogged blocks).
pub fn fluid_count(section: &Section) -> u16 {
    let mut count = 0u16;
    match section.blocks.repr() {
        Repr::Single(v) => {
            if facts(v).fluid_amount() > 0 {
                count = 4096;
            }
        }
        _ => {
            for i in 0..4096 {
                if facts(section.blocks.get(i)).fluid_amount() > 0 {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Number of blocks in a section that are not air (the client uses it to skip empty sections).
pub fn non_air_count(section: &Section, air: &[u16]) -> u16 {
    let mut count = 0u16;
    match section.blocks.repr() {
        Repr::Single(v) => {
            if !air.contains(&v) {
                count = 4096;
            }
        }
        _ => {
            for i in 0..4096 {
                if !air.contains(&section.blocks.get(i)) {
                    count += 1;
                }
            }
        }
    }
    count
}

/// The concatenated sections of a chunk, ready for the `buffer` field of a chunk packet.
pub fn encode_sections(chunk: &Chunk) -> Vec<u8> {
    let air = air_states();
    let mut out = Vec::with_capacity(chunk.sections().len() * 16);
    for section in chunk.sections() {
        out.extend_from_slice(&non_air_count(section, &air).to_be_bytes());
        out.extend_from_slice(&fluid_count(section).to_be_bytes());
        write_container(&mut out, &section.blocks);
        write_container(&mut out, &section.biomes);
    }
    out
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, ChunkDecodeError> {
        let (&b, rest) = self.0.split_first().ok_or(ChunkDecodeError::Truncated)?;
        self.0 = rest;
        Ok(b)
    }
    fn varint(&mut self) -> Result<u32, ChunkDecodeError> {
        let (mut v, mut shift) = (0u32, 0);
        loop {
            let b = self.byte()?;
            v |= u32::from(b & 0x7F) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
            shift += 7;
            if shift > 28 {
                return Err(ChunkDecodeError::BadContainer);
            }
        }
    }
    fn long(&mut self) -> Result<u64, ChunkDecodeError> {
        if self.0.len() < 8 {
            return Err(ChunkDecodeError::Truncated);
        }
        let (a, rest) = self.0.split_at(8);
        self.0 = rest;
        Ok(u64::from_be_bytes(a.try_into().expect("8 bytes")))
    }
}

fn read_container(r: &mut Reader<'_>, kind: Kind) -> Result<PalettedContainer, ChunkDecodeError> {
    let bits = r.byte()?;
    let words_for = |bits: u8| kind.len().div_ceil(64 / usize::from(bits));
    let repr = if bits == 0 {
        let v = r.varint()?;
        OwnedRepr::Single(u16::try_from(v).map_err(|_| ChunkDecodeError::BadContainer)?)
    } else if bits <= kind.max_indirect_bits() {
        let palette_len = r.varint()? as usize;
        if palette_len == 0 || palette_len > 1usize << bits {
            return Err(ChunkDecodeError::BadContainer);
        }
        let mut palette = Vec::with_capacity(palette_len);
        for _ in 0..palette_len {
            palette.push(u16::try_from(r.varint()?).map_err(|_| ChunkDecodeError::BadContainer)?);
        }
        let mut data = Vec::with_capacity(words_for(bits));
        for _ in 0..words_for(bits) {
            data.push(r.long()?);
        }
        OwnedRepr::Indirect { palette, data, bits }
    } else if bits == kind.direct_bits() {
        let mut data = Vec::with_capacity(words_for(bits));
        for _ in 0..words_for(bits) {
            data.push(r.long()?);
        }
        OwnedRepr::Direct { data }
    } else {
        return Err(ChunkDecodeError::BadContainer);
    };
    PalettedContainer::from_repr(kind, repr).ok_or(ChunkDecodeError::BadContainer)
}

/// Decodes the `buffer` of a chunk packet back into a chunk (used by tests and bots).
pub fn decode_sections(buffer: &[u8], dimension: Dimension) -> Result<Chunk, ChunkDecodeError> {
    let mut r = Reader(buffer);
    let mut sections = Vec::with_capacity(dimension.section_count());
    for _ in 0..dimension.section_count() {
        let _non_air = (u16::from(r.byte()?) << 8) | u16::from(r.byte()?);
        let _fluid = (u16::from(r.byte()?) << 8) | u16::from(r.byte()?);
        let blocks = read_container(&mut r, Kind::Blocks)?;
        let biomes = read_container(&mut r, Kind::Biomes)?;
        sections.push(Section { blocks, biomes });
    }
    if !r.0.is_empty() {
        return Err(ChunkDecodeError::Trailing(r.0.len()));
    }
    Chunk::from_sections(dimension, sections).ok_or(ChunkDecodeError::BadContainer)
}

/// Heightmap type ids.
pub mod heightmap {
    pub const WORLD_SURFACE: i32 = 1;
    pub const MOTION_BLOCKING: i32 = 4;
    pub const MOTION_BLOCKING_NO_LEAVES: i32 = 5;
}

fn blocks_motion(state: u16) -> bool {
    let f = facts(state);
    let cobweb = block_by_name("cobweb").map(|b| b.default_state);
    let sapling = block_by_name("bamboo_sapling").map(|b| b.id);
    let is_cobweb = cobweb.is_some_and(|c| {
        ferrite_data::blocks::block_of_state(state).map(|b| b.id)
            == ferrite_data::blocks::block_of_state(c).map(|b| b.id)
    });
    let is_sapling = sapling
        .is_some_and(|s| ferrite_data::blocks::block_of_state(state).map(|b| b.id) == Some(s));
    f.has(flag::SOLID) && !is_cobweb && !is_sapling
}

fn is_leaves(state: u16) -> bool {
    ferrite_data::blocks::block_of_state(state).is_some_and(|b| b.name.ends_with("_leaves"))
}

/// Packs 256 column heights (each `0..=height`) into the 9-bit-per-entry long array the client
/// expects (entries never straddle a long).
fn pack_heights(heights: &[u16; 256], height: i32) -> Vec<i64> {
    let bits = (usize::BITS - (height as usize).leading_zeros()) as usize; // ceil(log2(height + 1))
    let per_long = 64 / bits;
    let mut out = vec![0i64; 256usize.div_ceil(per_long)];
    for (i, h) in heights.iter().enumerate() {
        out[i / per_long] |= i64::from(*h) << ((i % per_long) * bits);
    }
    out
}

/// The three heightmaps the client uses, computed from the blocks.
pub fn heightmaps(chunk: &Chunk) -> Vec<Heightmap> {
    let dim = chunk.dimension();
    let air = air_states();
    let mut world_surface = [0u16; 256];
    let mut motion = [0u16; 256];
    let mut motion_no_leaves = [0u16; 256];
    for z in 0..16 {
        for x in 0..16 {
            let col = (z * 16 + x) as usize;
            let (mut ws, mut mb, mut mbn) = (false, false, false);
            let mut y = dim.max_y() - 1;
            while y >= dim.min_y && !(ws && mb && mbn) {
                let s = chunk.block(x, y, z).expect("inside the chunk");
                let h = (y - dim.min_y + 1) as u16;
                if !ws && !air.contains(&s) {
                    world_surface[col] = h;
                    ws = true;
                }
                let solid_or_fluid = blocks_motion(s) || facts(s).fluid_amount() > 0;
                if !mb && solid_or_fluid {
                    motion[col] = h;
                    mb = true;
                }
                if !mbn && solid_or_fluid && !is_leaves(s) {
                    motion_no_leaves[col] = h;
                    mbn = true;
                }
                y -= 1;
            }
        }
    }
    let mk = |kind: i32, hs: &[u16; 256]| Heightmap {
        kind: VarInt(kind),
        data: LenVec(pack_heights(hs, dim.height)),
    };
    vec![
        mk(heightmap::MOTION_BLOCKING_NO_LEAVES, &motion_no_leaves),
        mk(heightmap::WORLD_SURFACE, &world_surface),
        mk(heightmap::MOTION_BLOCKING, &motion),
    ]
}

/// Light data that makes every section fully sky-lit and block-light free.
///
/// Ferrite has no lighting engine yet; this keeps worlds readable (instead of black) at the cost
/// of showing caves and shade as bright. Bit `i` of a mask is the section `i - 1`, so a world of
/// `n` sections uses bits `0..n + 2`.
pub fn full_bright_light(section_count: usize) -> LightData {
    let bits = 0..section_count + 2;
    LightData {
        sky_mask: BitSetBytes::from_bits(bits.clone()),
        block_mask: BitSetBytes::default(),
        empty_sky_mask: BitSetBytes::default(),
        empty_block_mask: BitSetBytes::from_bits(bits.clone()),
        sky: LenVec(bits.map(|_| ByteVec(vec![0xFF; 2048])).collect()),
        block: LenVec(Vec::new()),
    }
}
