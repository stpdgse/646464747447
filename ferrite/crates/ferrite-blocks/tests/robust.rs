//! Properties that must hold for any world, not just the recorded ones.

use ferrite_blocks::{BlockAccess, BlockPos, Chunks, Rules, World};
use ferrite_data::blocks::{STATE_COUNT, block_by_name};
use ferrite_world::{Chunk, ChunkPos, Dimension};
use proptest::prelude::*;

fn state(name: &str) -> u16 {
    block_by_name(name).unwrap().default_state
}

fn empty_world(radius: i32) -> World<Chunks> {
    let mut chunks = Chunks::new(Dimension::OVERWORLD);
    for x in -radius..=radius {
        for z in -radius..=radius {
            chunks
                .insert(ChunkPos::new(x, z), Chunk::filled(Dimension::OVERWORLD, state("air"), 0));
        }
    }
    World::new(chunks, Rules::default())
}

fn snapshot(world: &World<Chunks>) -> Vec<u16> {
    let mut out = Vec::new();
    for y in 60..80 {
        for z in -16..32 {
            for x in -16..32 {
                out.push(world.block(BlockPos::new(x, y, z)).unwrap());
            }
        }
    }
    out
}

/// Builds a world out of *arbitrary* block states (every one of the 35,723 can appear), pours
/// fluids into it and runs it.
fn run(
    blocks: &[(u8, u8, u8, u16)],
    fluids: &[(u8, u8, u8, bool)],
    ticks: usize,
) -> (Vec<u16>, usize) {
    let mut world = empty_world(1);
    for x in -16..32 {
        for z in -16..32 {
            world.blocks.set_block_raw(BlockPos::new(x, 63, z), state("stone"));
        }
    }
    for &(x, y, z, s) in blocks {
        world.blocks.set_block_raw(
            BlockPos::new(i32::from(x % 16), 64 + i32::from(y % 6), i32::from(z % 16)),
            s,
        );
    }
    for &(x, y, z, lava) in fluids {
        let pos = BlockPos::new(i32::from(x % 16), 64 + i32::from(y % 8), i32::from(z % 16));
        world.command_set_block(pos, state(if lava { "lava" } else { "water" }));
    }
    for _ in 0..ticks {
        world.tick();
    }
    (snapshot(&world), world.pending_fluid_ticks().len())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// No combination of block states makes the simulation panic, and the same input always
    /// gives the same world.
    #[test]
    fn any_world_runs_and_is_deterministic(
        blocks in prop::collection::vec((any::<u8>(), any::<u8>(), any::<u8>(), 0..STATE_COUNT as u16), 0..300),
        fluids in prop::collection::vec((any::<u8>(), any::<u8>(), any::<u8>(), any::<bool>()), 1..5),
    ) {
        let a = run(&blocks, &fluids, 120);
        let b = run(&blocks, &fluids, 120);
        prop_assert_eq!(a, b);
    }
}

#[test]
fn nothing_happens_outside_loaded_chunks() {
    // One loaded chunk; water poured at its edge must not create blocks or ticks beyond it.
    let mut chunks = Chunks::new(Dimension::OVERWORLD);
    chunks.insert(ChunkPos::new(0, 0), Chunk::filled(Dimension::OVERWORLD, state("air"), 0));
    let mut world = World::new(chunks, Rules::default());
    for x in 0..16 {
        for z in 0..16 {
            world.blocks.set_block_raw(BlockPos::new(x, 63, z), state("stone"));
        }
    }
    assert!(world.command_set_block(BlockPos::new(0, 64, 0), state("water")));
    assert!(!world.command_set_block(BlockPos::new(-1, 64, 0), state("water")), "unloaded");
    for _ in 0..200 {
        world.tick();
    }
    for t in world.pending_fluid_ticks() {
        assert!((0..16).contains(&t.pos.x) && (0..16).contains(&t.pos.z), "tick escaped: {t:?}");
    }
    assert_eq!(world.block(BlockPos::new(-1, 64, 0)), None);
    assert_ne!(world.block(BlockPos::new(1, 64, 0)), Some(state("air")), "water did spread inside");
}

#[test]
fn a_drained_world_comes_to_rest() {
    // Remove the source: every flowing cell must disappear and no tick may remain.
    let mut world = empty_world(1);
    for x in -16..32 {
        for z in -16..32 {
            world.blocks.set_block_raw(BlockPos::new(x, 63, z), state("stone"));
        }
    }
    let source = BlockPos::new(8, 64, 8);
    world.command_set_block(source, state("water"));
    for _ in 0..100 {
        world.tick();
    }
    world.command_set_block(source, state("air"));
    for _ in 0..400 {
        world.tick();
    }
    let air = state("air");
    let stone = state("stone");
    assert!(snapshot(&world).iter().all(|s| *s == air || *s == stone));
    assert!(world.pending_fluid_ticks().is_empty());
}

#[test]
fn an_infinite_pool_refills() {
    // A 2x2 pool of sources: take one out and it comes back.
    let mut world = empty_world(0);
    for x in 0..16 {
        for z in 0..16 {
            for y in 62..=64 {
                world.blocks.set_block_raw(BlockPos::new(x, y, z), state("stone"));
            }
        }
    }
    let cells = [(7, 7), (8, 7), (7, 8), (8, 8)];
    for (x, z) in cells {
        world.blocks.set_block_raw(BlockPos::new(x, 64, z), state("air"));
    }
    for (x, z) in cells {
        world.command_set_block(BlockPos::new(x, 64, z), state("water"));
    }
    world.command_set_block(BlockPos::new(7, 64, 7), state("air"));
    for _ in 0..20 {
        world.tick();
    }
    assert_eq!(world.block(BlockPos::new(7, 64, 7)), Some(state("water")));
}
