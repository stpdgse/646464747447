//! A flat world: bedrock, dirt and a grass surface, all of it plains.

use ferrite_data::blocks::block_by_name;
use ferrite_world::{Chunk, Dimension, Kind, PalettedContainer, Section};

/// States of air, cave air and void air.
pub fn air_states() -> Vec<u16> {
    ["air", "cave_air", "void_air"]
        .iter()
        .map(|n| block_by_name(n).expect("air block").default_state)
        .collect()
}

/// One layer of the flat world.
#[derive(Clone, Copy, Debug)]
pub struct Layer {
    pub block: &'static str,
    pub height: i32,
}

/// The default superflat preset from the bottom of the world up.
pub const DEFAULT_LAYERS: [Layer; 3] = [
    Layer { block: "bedrock", height: 1 },
    Layer { block: "dirt", height: 2 },
    Layer { block: "grass_block", height: 1 },
];

#[derive(Clone, Debug)]
pub struct FlatGenerator {
    pub dimension: Dimension,
    pub layers: Vec<Layer>,
    /// Id of the biome in the synchronized `worldgen/biome` registry.
    pub biome: u16,
}

impl FlatGenerator {
    pub fn new(dimension: Dimension, layers: &[Layer], biome: u16) -> Self {
        Self { dimension, layers: layers.to_vec(), biome }
    }

    /// The default flat world in the overworld, with `plains` as its biome.
    pub fn overworld() -> Self {
        let biome = crate::synchronized_id("minecraft:worldgen/biome", "minecraft:plains")
            .expect("plains is a biome") as u16;
        Self::new(Dimension::OVERWORLD, &DEFAULT_LAYERS, biome)
    }

    /// Height of the top block, i.e. the y of the surface.
    pub fn surface_y(&self) -> i32 {
        self.dimension.min_y + self.layers.iter().map(|l| l.height).sum::<i32>() - 1
    }

    /// Generates a chunk. Flat worlds are identical everywhere.
    pub fn generate(&self) -> Chunk {
        let air = block_by_name("air").expect("air").default_state;
        let mut sections = Vec::with_capacity(self.dimension.section_count());
        for _ in 0..self.dimension.section_count() {
            sections.push(Section {
                blocks: PalettedContainer::filled(Kind::Blocks, air),
                biomes: PalettedContainer::filled(Kind::Biomes, self.biome),
            });
        }
        let mut chunk = Chunk::from_sections(self.dimension, sections).expect("section count");
        let mut y = self.dimension.min_y;
        for layer in &self.layers {
            let state = block_by_name(layer.block).expect("layer block").default_state;
            for _ in 0..layer.height {
                for z in 0..16 {
                    for x in 0..16 {
                        chunk.set_block(x, y, z, state);
                    }
                }
                y += 1;
            }
        }
        chunk
    }
}
