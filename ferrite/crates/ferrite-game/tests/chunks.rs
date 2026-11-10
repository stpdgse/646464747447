//! Chunk encoding against the chunks the official server sent (`ferrite-capture`).

use ferrite_data::blocks::block_by_name;
use ferrite_game::chunkenc::{decode_sections, encode_sections, full_bright_light, heightmaps};
use ferrite_game::flat::FlatGenerator;
use ferrite_protocol::codec::decode_exact;
use ferrite_protocol::packets::play::LevelChunkWithLight;
use ferrite_world::Dimension;

fn vanilla_chunks() -> Vec<LevelChunkWithLight> {
    include_str!("../../ferrite-protocol/tests/captures/join.txt")
        .lines()
        .filter(|l| l.contains(" level_chunk_with_light "))
        .map(|l| {
            let hex = l.splitn(3, ' ').nth(2).unwrap();
            let bytes: Vec<u8> = (0..hex.len() / 2)
                .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
                .collect();
            decode_exact(&bytes).expect("a captured chunk decodes")
        })
        .collect()
}

#[test]
fn vanilla_chunk_data_decodes_and_re_encodes_to_the_same_chunk() {
    let chunks = vanilla_chunks();
    assert!(!chunks.is_empty());
    for c in chunks {
        let decoded =
            decode_sections(&c.buffer.0, Dimension::OVERWORLD).expect("vanilla data decodes");
        // Our encoder must produce something that decodes to the same chunk.
        let again = decode_sections(&encode_sections(&decoded), Dimension::OVERWORLD).unwrap();
        assert_eq!(decoded, again, "chunk ({}, {})", c.x, c.z);
    }
}

#[test]
fn our_flat_world_matches_what_vanilla_generates_for_its_flat_preset() {
    // Chunk (-1, 0) of the capture is pristine superflat terrain far from the test arena.
    let vanilla =
        vanilla_chunks().into_iter().find(|c| c.x == -1 && c.z == 0).expect("chunk (-1, 0)");
    let theirs = decode_sections(&vanilla.buffer.0, Dimension::OVERWORLD).unwrap();
    let ours = FlatGenerator::overworld().generate();
    for y in -64..320 {
        for z in 0..16 {
            for x in 0..16 {
                // The vanilla world has the same state ids because both come from the same tables.
                assert_eq!(theirs.block(x, y, z), ours.block(x, y, z), "block ({x}, {y}, {z})");
            }
        }
    }
    // Biomes: same cells (plains).
    assert_eq!(theirs.biome(0, 0, 0), ours.biome(0, 0, 0));
}

#[test]
fn heightmaps_match_vanilla_for_the_same_terrain() {
    let vanilla = vanilla_chunks().into_iter().find(|c| c.x == -1 && c.z == 0).unwrap();
    let ours = heightmaps(&FlatGenerator::overworld().generate());
    for h in &ours {
        let theirs =
            vanilla.heightmaps.0.iter().find(|t| t.kind == h.kind).expect("same heightmap kinds");
        assert_eq!(theirs.data, h.data, "heightmap {:?}", h.kind);
    }
    assert_eq!(ours.len(), vanilla.heightmaps.0.len());
}

#[test]
fn light_masks_follow_the_section_count() {
    let l = full_bright_light(24);
    assert!(l.sky_mask.contains(0) && l.sky_mask.contains(25) && !l.sky_mask.contains(26));
    assert_eq!(l.sky.0.len(), 26);
    assert!(l.empty_block_mask.contains(25));
}

#[test]
fn air_counts_are_exact() {
    let c = FlatGenerator::overworld().generate();
    let air = ferrite_game::flat::air_states();
    let bottom = &c.sections()[0];
    // Section 0 spans y -64..-48: bedrock, 2 dirt, 1 grass = 4 layers of 256 blocks.
    assert_eq!(ferrite_game::chunkenc::non_air_count(bottom, &air), 4 * 256);
    assert_eq!(ferrite_game::chunkenc::non_air_count(&c.sections()[5], &air), 0);
    let _ = block_by_name("stone");
}

#[test]
fn the_biome_id_we_use_is_the_one_vanilla_chunks_carry() {
    let vanilla = vanilla_chunks().into_iter().find(|c| c.x == -1 && c.z == 0).unwrap();
    let theirs = decode_sections(&vanilla.buffer.0, Dimension::OVERWORLD).unwrap();
    let id = ferrite_game::synchronized_id("minecraft:worldgen/biome", "minecraft:plains").unwrap();
    assert_eq!(u32::from(theirs.biome(3, -50, 7).unwrap()), id);
    eprintln!("plains has id {id}");
}
