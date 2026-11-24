use crate::paletted::{Kind, PalettedContainer};

/// A chunk column's position, in chunks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// The chunk containing block column `(block_x, block_z)`.
    pub const fn containing(block_x: i32, block_z: i32) -> Self {
        Self { x: block_x >> 4, z: block_z >> 4 }
    }
}

/// The vertical extent of a dimension. Both values are in blocks and multiples of 16.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dimension {
    pub min_y: i32,
    pub height: i32,
}

impl Dimension {
    pub const OVERWORLD: Dimension = Dimension { min_y: -64, height: 384 };
    pub const NETHER: Dimension = Dimension { min_y: 0, height: 256 };
    pub const END: Dimension = Dimension { min_y: 0, height: 256 };

    pub fn is_valid(&self) -> bool {
        self.min_y % 16 == 0 && self.height % 16 == 0 && (16..=4096).contains(&self.height)
    }

    pub fn section_count(&self) -> usize {
        (self.height / 16) as usize
    }

    pub fn max_y(&self) -> i32 {
        self.min_y + self.height
    }
}

/// A 16x16x16 cube of blocks plus its 4x4x4 biome cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub blocks: PalettedContainer,
    pub biomes: PalettedContainer,
}

impl Section {
    pub fn filled(block: u16, biome: u16) -> Self {
        Self {
            blocks: PalettedContainer::filled(Kind::Blocks, block),
            biomes: PalettedContainer::filled(Kind::Biomes, biome),
        }
    }
}

/// `(y * 16 + z) * 16 + x` within a section, vanilla's block order.
#[inline]
fn block_index(x: i32, y: i32, z: i32) -> usize {
    (((y & 15) << 8) | (z << 4) | x) as usize
}

/// `(y4 * 4 + z4) * 4 + x4` for the biome cell containing the block.
#[inline]
fn biome_index(x: i32, y: i32, z: i32) -> usize {
    ((((y & 15) >> 2) << 4) | ((z >> 2) << 2) | (x >> 2)) as usize
}

/// A column of sections covering a dimension's full height.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk {
    dimension: Dimension,
    sections: Vec<Section>,
}

impl Chunk {
    /// A chunk where every block is `block` and every biome cell is `biome`.
    pub fn filled(dimension: Dimension, block: u16, biome: u16) -> Self {
        assert!(dimension.is_valid(), "invalid dimension {dimension:?}");
        Self { dimension, sections: vec![Section::filled(block, biome); dimension.section_count()] }
    }

    /// Builds a chunk from sections; the count must match the dimension.
    pub fn from_sections(dimension: Dimension, sections: Vec<Section>) -> Option<Self> {
        (dimension.is_valid() && sections.len() == dimension.section_count())
            .then_some(Self { dimension, sections })
    }

    pub fn dimension(&self) -> Dimension {
        self.dimension
    }

    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    pub fn sections_mut(&mut self) -> &mut [Section] {
        &mut self.sections
    }

    fn locate(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        let inside = (0..16).contains(&x)
            && (0..16).contains(&z)
            && y >= self.dimension.min_y
            && y < self.dimension.max_y();
        inside.then(|| ((y - self.dimension.min_y) >> 4) as usize)
    }

    /// The block state at chunk-local `x`, `z` (0..16) and absolute `y`, or `None` outside.
    pub fn block(&self, x: i32, y: i32, z: i32) -> Option<u16> {
        let s = self.locate(x, y, z)?;
        Some(self.sections[s].blocks.get(block_index(x, y, z)))
    }

    /// Sets a block; returns `false` (changing nothing) if the position is outside the chunk.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, state: u16) -> bool {
        let Some(s) = self.locate(x, y, z) else { return false };
        self.sections[s].blocks.set(block_index(x, y, z), state);
        true
    }

    pub fn biome(&self, x: i32, y: i32, z: i32) -> Option<u16> {
        let s = self.locate(x, y, z)?;
        Some(self.sections[s].biomes.get(biome_index(x, y, z)))
    }

    pub fn set_biome(&mut self, x: i32, y: i32, z: i32, biome: u16) -> bool {
        let Some(s) = self.locate(x, y, z) else { return false };
        self.sections[s].biomes.set(biome_index(x, y, z), biome);
        true
    }

    /// Approximate heap size in bytes.
    pub fn heap_size(&self) -> usize {
        self.sections.iter().map(|s| s.blocks.heap_size() + s.biomes.heap_size()).sum()
    }
}
