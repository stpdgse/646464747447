use ferrite_world::codec::{CodecError, Limits, decode, decode_raw, encode, encode_raw};
use ferrite_world::{Chunk, Dimension, Kind, PalettedContainer, Repr, Section};
use proptest::prelude::*;

fn limits() -> Limits {
    Limits::vanilla()
}

/// A short description of how a container is currently stored.
fn shape(c: &PalettedContainer) -> String {
    match c.repr() {
        Repr::Single(_) => "single".into(),
        Repr::Indirect { bits, .. } => format!("indirect{bits}"),
        Repr::Direct { .. } => "direct".into(),
    }
}

#[test]
fn vanilla_limits_come_from_the_generated_data() {
    let l = Limits::vanilla();
    assert_eq!((l.max_block_state, l.biome_count), (35722, 67));
}

// ---------------------------------------------------------------------------------------------
// Packing and the paletted container
// ---------------------------------------------------------------------------------------------

#[test]
fn storage_sizes_follow_vanillas_no_straddling_rule() {
    // A long holds floor(64 / bits) whole entries, so a 4096-entry section needs
    // ceil(4096 / floor(64 / bits)) longs: 256, 342, 410, 456 and 512 at 4 to 8 bits.
    let mut c = PalettedContainer::filled(Kind::Blocks, 0);
    let longs = |c: &PalettedContainer| match c.repr() {
        Repr::Indirect { data, .. } | Repr::Direct { data } => data.len(),
        Repr::Single(_) => 0,
    };
    let mut seen = Vec::new();
    for v in 1..=256u16 {
        c.set(usize::from(v), v);
        let entry = (shape(&c), longs(&c));
        if seen.last() != Some(&entry) {
            seen.push(entry);
        }
    }
    assert_eq!(
        seen,
        [
            ("indirect4".to_string(), 256),
            ("indirect5".to_string(), 342),
            ("indirect6".to_string(), 410),
            ("indirect7".to_string(), 456),
            ("indirect8".to_string(), 512),
            ("direct".to_string(), 1024),
        ]
    );
}

#[test]
fn a_container_grows_through_every_width_without_losing_data() {
    let mut c = PalettedContainer::filled(Kind::Blocks, 0);
    assert_eq!(shape(&c), "single");
    for n in 1..=300usize {
        c.set(n - 1, n as u16);
        // The palette holds the filler plus `n` distinct values.
        let distinct = n + 1;
        let expected = match distinct {
            2..=16 => "indirect4",
            17..=32 => "indirect5",
            33..=64 => "indirect6",
            65..=128 => "indirect7",
            129..=256 => "indirect8",
            _ => "direct",
        };
        assert_eq!(shape(&c), expected, "{distinct} distinct values");
        // Nothing already stored may change when the representation does.
        for k in 0..n {
            assert_eq!(c.get(k), (k + 1) as u16, "entry {k} after {n} sets");
        }
        assert_eq!(c.get(4095), 0);
    }
}

#[test]
fn biomes_use_their_own_width_rules() {
    let mut c = PalettedContainer::filled(Kind::Biomes, 0);
    for n in 1..=12usize {
        c.set(n, n as u16);
        let distinct = n + 1;
        let expected = match distinct {
            2 => "indirect1",
            3..=4 => "indirect2",
            5..=8 => "indirect3",
            _ => "direct",
        };
        assert_eq!(shape(&c), expected, "{distinct} biomes");
    }
    assert_eq!(c.get(12), 12);
}

#[test]
#[should_panic(expected = "does not fit")]
fn ids_wider_than_the_container_are_refused() {
    PalettedContainer::filled(Kind::Biomes, 0).set(0, 128);
}

#[test]
fn equality_is_by_contents_not_by_representation() {
    let mut a = PalettedContainer::filled(Kind::Blocks, 5);
    let mut b = PalettedContainer::filled(Kind::Blocks, 9);
    for i in 0..4096 {
        a.set(i, (i % 7) as u16);
        b.set(i, (i % 7) as u16);
    }
    assert_eq!(a, b);
    // One has a sprawling palette from its history, the other is minimal; still equal.
    let mut wide = PalettedContainer::filled(Kind::Blocks, 0);
    for v in 0..200u16 {
        wide.set(usize::from(v), v + 1);
    }
    for i in 0..4096 {
        wide.set(i, (i % 7) as u16);
    }
    assert_eq!(wide, a);
    assert_ne!(wide.heap_size(), a.canonical().heap_size());
    assert_eq!(wide.canonical().heap_size(), a.canonical().heap_size());
    b.set(100, 4000);
    assert_ne!(a, b);
}

#[test]
fn canonical_form_depends_only_on_contents() {
    // Same contents reached by different histories must serialise to identical bytes.
    let build = |order: &[usize]| {
        let mut c = PalettedContainer::filled(Kind::Blocks, 0);
        for &i in order {
            c.set(i, (i % 9) as u16 + 1);
        }
        for i in 0..4096 {
            if !order.contains(&i) {
                c.set(i, 0);
            }
        }
        c
    };
    let forward: Vec<usize> = (0..400).collect();
    let backward: Vec<usize> = (0..400).rev().collect();
    assert_eq!(build(&forward), build(&backward));
    let to_chunk = |c: PalettedContainer| {
        let mut s = vec![Section::filled(0, 0); 1];
        s[0].blocks = c;
        Chunk::from_sections(Dimension { min_y: 0, height: 16 }, s).unwrap()
    };
    assert_eq!(encode_raw(&to_chunk(build(&forward))), encode_raw(&to_chunk(build(&backward))));
    assert_eq!(shape(&build(&forward).canonical()), "indirect4");
}

#[test]
fn uniform_containers_collapse_to_a_single_value() {
    let mut c = PalettedContainer::filled(Kind::Blocks, 1);
    c.set(0, 2);
    c.set(0, 1);
    assert_eq!(c.uniform(), Some(1));
    assert_eq!(shape(&c), "indirect4", "it keeps its palette until canonicalised");
    assert_eq!(shape(&c.canonical()), "single");
}

// ---------------------------------------------------------------------------------------------
// Chunks
// ---------------------------------------------------------------------------------------------

#[test]
fn chunk_addressing_covers_the_whole_height_including_negative_y() {
    let mut c = Chunk::filled(Dimension::OVERWORLD, 0, 0);
    assert_eq!(c.sections().len(), 24);
    for (x, y, z, v) in
        [(0, -64, 0, 1), (15, 319, 15, 2), (7, 0, 8, 3), (3, -1, 12, 4), (3, 63, 12, 5)]
    {
        assert!(c.set_block(x, y, z, v));
        assert_eq!(c.block(x, y, z), Some(v), "({x}, {y}, {z})");
    }
    // Neighbours are untouched.
    assert_eq!(c.block(0, -63, 0), Some(0));
    assert_eq!(c.block(1, -64, 0), Some(0));
    // Anything outside the chunk is rejected without changing it.
    let before = c.clone();
    for (x, y, z) in [(-1, 0, 0), (16, 0, 0), (0, 0, 16), (0, -65, 0), (0, 320, 0)] {
        assert_eq!(c.block(x, y, z), None);
        assert!(!c.set_block(x, y, z, 9));
    }
    assert_eq!(c, before);
}

#[test]
fn biomes_are_stored_per_4x4x4_cell() {
    let mut c = Chunk::filled(Dimension::NETHER, 0, 0);
    assert!(c.set_biome(5, 70, 9, 12));
    // The whole cell containing (5, 70, 9) reads back 12: x 4..8, y 68..72, z 8..12.
    for x in 4..8 {
        for y in 68..72 {
            for z in 8..12 {
                assert_eq!(c.biome(x, y, z), Some(12), "({x}, {y}, {z})");
            }
        }
    }
    for (x, y, z) in [(3, 70, 9), (8, 70, 9), (5, 67, 9), (5, 72, 9), (5, 70, 7), (5, 70, 12)] {
        assert_eq!(c.biome(x, y, z), Some(0), "({x}, {y}, {z})");
    }
    assert_eq!(c.biome(5, 256, 9), None);
}

#[test]
fn invalid_dimensions_are_refused() {
    assert!(!Dimension { min_y: 5, height: 256 }.is_valid());
    assert!(!Dimension { min_y: 0, height: 250 }.is_valid());
    assert!(!Dimension { min_y: 0, height: 0 }.is_valid());
    assert!(!Dimension { min_y: 0, height: 4112 }.is_valid());
    assert!(
        Chunk::from_sections(Dimension::NETHER, vec![Section::filled(0, 0)]).is_none(),
        "wrong section count"
    );
}

// ---------------------------------------------------------------------------------------------
// The codec
// ---------------------------------------------------------------------------------------------

/// A varied chunk: a floor, caves, a few hundred distinct states in one section, many biomes.
fn terrain() -> Chunk {
    let mut c = Chunk::filled(Dimension { min_y: -32, height: 96 }, 0, 3);
    for x in 0..16 {
        for z in 0..16 {
            for y in -32i32..20 {
                let state = if y < -28 {
                    1
                } else if (x * z + y) % 11 == 0 {
                    0
                } else {
                    9 + (y.rem_euclid(5i32) as u16)
                };
                c.set_block(x, y, z, state);
            }
            c.set_biome(x, 0, z, ((x / 4 + z / 4) % 6) as u16);
        }
    }
    // One section with far more than 256 distinct states forces the direct representation.
    for i in 0..300i32 {
        c.set_block(i % 16, 48 + (i / 256), (i / 16) % 16, 100 + i as u16);
    }
    c.set_block(0, 63, 0, 35722);
    c
}

#[test]
fn chunks_round_trip_exactly() {
    for chunk in [
        Chunk::filled(Dimension::OVERWORLD, 0, 0),
        Chunk::filled(Dimension::NETHER, 35722, 66),
        terrain(),
    ] {
        let bytes = encode(&chunk, 3).unwrap();
        assert_eq!(decode(&bytes, &limits()).unwrap(), chunk);
        assert_eq!(decode_raw(&encode_raw(&chunk), &limits()).unwrap(), chunk);
    }
}

#[test]
fn an_empty_chunk_is_tiny_on_disk() {
    let bytes = encode(&Chunk::filled(Dimension::OVERWORLD, 0, 0), 3).unwrap();
    assert!(bytes.len() < 100, "an all-air overworld chunk took {} bytes", bytes.len());
}

#[test]
fn encoding_is_deterministic() {
    assert_eq!(encode(&terrain(), 3).unwrap(), encode(&terrain(), 3).unwrap());
}

#[test]
fn malformed_headers_are_rejected() {
    let raw = encode_raw(&Chunk::filled(Dimension { min_y: 0, height: 16 }, 0, 0));
    let with = |f: &dyn Fn(&mut Vec<u8>)| {
        let mut r = raw.clone();
        f(&mut r);
        decode_raw(&r, &limits())
    };
    assert_eq!(with(&|r| r[0] = b'X'), Err(CodecError::BadMagic));
    assert_eq!(with(&|r| r[4] = 9), Err(CodecError::UnsupportedVersion(9)));
    assert!(matches!(
        with(&|r| r[10..14].copy_from_slice(&15i32.to_le_bytes())),
        Err(CodecError::BadDimension { .. })
    ));
    assert!(matches!(
        with(&|r| r[6..10].copy_from_slice(&5i32.to_le_bytes())),
        Err(CodecError::BadDimension { .. })
    ));
    assert_eq!(with(&|r| r.push(0)), Err(CodecError::Trailing(1)));
    assert!(
        matches!(with(&|r| r[14] = 7), Err(CodecError::BadSection { .. })),
        "unknown container kind"
    );
}

#[test]
fn every_truncation_of_a_valid_chunk_is_an_error_not_a_panic() {
    let raw = encode_raw(&terrain());
    for cut in 0..raw.len() {
        assert!(decode_raw(&raw[..cut], &limits()).is_err(), "cut at {cut}");
    }
    assert!(decode_raw(&raw, &limits()).is_ok());
}

#[test]
fn ids_outside_the_world_are_rejected() {
    let mut c = Chunk::filled(Dimension { min_y: 0, height: 16 }, 0, 0);
    c.set_block(1, 1, 1, 30000);
    let raw = encode_raw(&c);
    // Valid for a world that has that many states...
    assert!(decode_raw(&raw, &Limits { max_block_state: 30000, biome_count: 67 }).is_ok());
    // ...but not one that does not.
    assert!(matches!(
        decode_raw(&raw, &Limits { max_block_state: 29999, biome_count: 67 }),
        Err(CodecError::IdOutOfRange { section: 0, id: 30000 })
    ));
    let mut c = Chunk::filled(Dimension { min_y: 0, height: 16 }, 0, 0);
    c.set_biome(0, 0, 0, 66);
    let raw = encode_raw(&c);
    assert!(decode_raw(&raw, &Limits { max_block_state: 100, biome_count: 67 }).is_ok());
    assert!(matches!(
        decode_raw(&raw, &Limits { max_block_state: 100, biome_count: 66 }),
        Err(CodecError::IdOutOfRange { id: 66, .. })
    ));
}

#[test]
fn inconsistent_containers_are_rejected() {
    // Hand-build one section whose block container is an indirect palette of two entries but whose
    // packed data points at index 5.
    let mut raw = Vec::new();
    raw.extend_from_slice(b"FCK1\x01\x00");
    raw.extend_from_slice(&0i32.to_le_bytes());
    raw.extend_from_slice(&16i32.to_le_bytes());
    raw.extend_from_slice(&[1, 4]); // indirect, 4 bits
    raw.extend_from_slice(&2u16.to_le_bytes());
    raw.extend_from_slice(&[0, 0, 1, 0]); // palette [0, 1]
    for w in 0..256u64 {
        raw.extend_from_slice(&(if w == 3 { 0x5 } else { 0u64 }).to_le_bytes());
    }
    raw.extend_from_slice(&[0, 0, 0]); // biomes: single 0
    assert!(matches!(
        decode_raw(&raw, &limits()),
        Err(CodecError::BadSection { reason: "inconsistent container", .. })
    ));

    // A palette larger than its width allows.
    let mut raw2 = raw[..14].to_vec();
    raw2.extend_from_slice(&[1, 4]);
    raw2.extend_from_slice(&17u16.to_le_bytes());
    assert!(matches!(
        decode_raw(&raw2, &limits()),
        Err(CodecError::BadSection { reason: "palette size does not fit its width", .. })
    ));
    // A width outside the legal range for blocks.
    let mut raw3 = raw[..14].to_vec();
    raw3.extend_from_slice(&[1, 9]);
    raw3.extend_from_slice(&2u16.to_le_bytes());
    assert!(matches!(
        decode_raw(&raw3, &limits()),
        Err(CodecError::BadSection { reason: "palette width out of range", .. })
    ));
}

#[test]
fn a_decompression_bomb_is_refused_before_it_is_expanded() {
    // 64 MiB of zeroes compresses to a few KiB; decoding must refuse it, not allocate it.
    let bomb = zstd_compress(&vec![0u8; 64 * 1024 * 1024]);
    assert!(bomb.len() < 10_000, "the bomb is small on disk ({} bytes)", bomb.len());
    assert_eq!(decode(&bomb, &limits()), Err(CodecError::TooLarge));
    assert!(matches!(decode(b"definitely not zstd", &limits()), Err(CodecError::Compression(_))));
    assert!(decode(&[], &limits()).is_err());
}

fn zstd_compress(data: &[u8]) -> Vec<u8> {
    // A tiny helper so the test needs no direct zstd dependency: a valid chunk is compressed by
    // the codec, and a bomb is produced by compressing a large run through the same entry point.
    ferrite_world::codec::compress_for_tests(data)
}

#[test]
fn flipping_any_single_bit_never_panics() {
    let raw = encode_raw(&Chunk::filled(Dimension { min_y: 0, height: 32 }, 7, 2));
    for byte in 0..raw.len() {
        for bit in 0..8 {
            let mut r = raw.clone();
            r[byte] ^= 1 << bit;
            let _ = decode_raw(&r, &limits());
        }
    }
    let mut busy = Chunk::filled(Dimension { min_y: 0, height: 16 }, 0, 0);
    for i in 0..40 {
        busy.set_block(i % 16, i / 16, 0, i as u16);
    }
    let raw = encode_raw(&busy);
    for byte in 0..raw.len().min(600) {
        for bit in 0..8 {
            let mut r = raw.clone();
            r[byte] ^= 1 << bit;
            let _ = decode_raw(&r, &limits());
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 300, ..ProptestConfig::default() })]

    #[test]
    fn random_edits_round_trip_and_match_a_flat_model(
        edits in prop::collection::vec(((0i32..16), (-32i32..64), (0i32..16), 0u16..=35722, any::<bool>()), 0..400)
    ) {
        let dim = Dimension { min_y: -32, height: 96 };
        let mut chunk = Chunk::filled(dim, 0, 0);
        let mut model = std::collections::HashMap::new();
        for &(x, y, z, v, biome) in &edits {
            if biome {
                chunk.set_biome(x, y, z, v % 67);
            } else {
                chunk.set_block(x, y, z, v);
                model.insert((x, y, z), v);
            }
        }
        for (&(x, y, z), &v) in &model {
            prop_assert_eq!(chunk.block(x, y, z), Some(v));
        }
        let decoded = decode(&encode(&chunk, 1).unwrap(), &limits()).unwrap();
        prop_assert_eq!(&decoded, &chunk);
        for (&(x, y, z), &v) in &model {
            prop_assert_eq!(decoded.block(x, y, z), Some(v));
        }
    }

    #[test]
    fn arbitrary_bytes_never_panic_the_decoder(bytes in prop::collection::vec(any::<u8>(), 0..2000)) {
        let _ = decode(&bytes, &limits());
        let _ = decode_raw(&bytes, &limits());
        // And with a plausible header in front, so the parser gets past the first checks.
        let mut framed = b"FCK1\x01\x00".to_vec();
        framed.extend_from_slice(&0i32.to_le_bytes());
        framed.extend_from_slice(&32i32.to_le_bytes());
        framed.extend_from_slice(&bytes);
        let _ = decode_raw(&framed, &limits());
    }
}
