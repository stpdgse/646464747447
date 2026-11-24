//! Chunk codec and store performance, on chunks that look like real terrain.

use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use ferrite_data::blocks::block_by_name;
use ferrite_world::codec::{decode, encode, encode_raw};
use ferrite_world::store::{Options, Store};
use ferrite_world::{Chunk, ChunkPos, Dimension, Kind, Limits, PalettedContainer};
use tempfile::TempDir;

fn state(name: &str) -> u16 {
    block_by_name(name).unwrap_or_else(|| panic!("no block {name}")).default_state
}

/// Deterministic value noise in [0, 1): cheap, smooth enough to make hills and caves.
fn noise(x: f64, y: f64, z: f64, seed: f64) -> f64 {
    let v = (x * 0.31 + seed).sin() * (z * 0.27 - seed).cos()
        + (y * 0.21 + x * 0.11 + seed * 2.0).sin() * 0.7
        + ((x + z) * 0.13).sin() * 0.5;
    ((v / 2.2) + 1.0) * 0.5
}

/// A chunk with hills, bedrock, stone, dirt, grass, sea, caves and ores, like a freshly generated
/// overworld column. `cx`, `cz` vary the terrain between chunks.
fn terrain(cx: i32, cz: i32) -> Chunk {
    let (air, stone, dirt, grass, water, bedrock, deepslate, gravel) = (
        state("air"),
        state("stone"),
        state("dirt"),
        state("grass_block"),
        state("water"),
        state("bedrock"),
        state("deepslate"),
        state("gravel"),
    );
    let ores = [state("coal_ore"), state("iron_ore"), state("gold_ore"), state("diamond_ore")];
    let mut chunk = Chunk::filled(Dimension::OVERWORLD, air, 4);
    for x in 0..16 {
        for z in 0..16 {
            let (wx, wz) = (f64::from(cx * 16 + x), f64::from(cz * 16 + z));
            let height = 64 + (noise(wx, 0.0, wz, 1.0) * 40.0 - 20.0) as i32;
            for y in -64..=height.max(62) {
                let (fy, depth) = (f64::from(y), height - y);
                let s = if y == -64 {
                    bedrock
                } else if y > height {
                    water
                } else if y == height {
                    if height < 62 { gravel } else { grass }
                } else if depth <= 3 {
                    dirt
                } else {
                    let base = if y < 0 { deepslate } else { stone };
                    let cave = noise(wx, fy, wz, 7.0) > 0.82;
                    let hash =
                        ((x * 73856093) ^ (y * 19349663) ^ (z * 83492791) ^ (cx * 31 + cz * 17))
                            as u32;
                    if cave {
                        air
                    } else if hash % 211 == 0 {
                        ores[(hash >> 8) as usize % 4]
                    } else {
                        base
                    }
                };
                chunk.set_block(x, y, z, s);
            }
            for y in (-64..height.max(62)).step_by(4) {
                let biome =
                    ((noise(wx, 0.0, wz, 3.0) * 6.0) as u16).min(5) + if y < 0 { 6 } else { 0 };
                chunk.set_biome(x, y, z, biome);
            }
        }
    }
    chunk
}

fn flat() -> Chunk {
    let mut c = Chunk::filled(Dimension::OVERWORLD, state("air"), 1);
    for x in 0..16 {
        for z in 0..16 {
            c.set_block(x, -64, z, state("bedrock"));
            for y in -63..-61 {
                c.set_block(x, y, z, state("dirt"));
            }
            c.set_block(x, -61, z, state("grass_block"));
        }
    }
    c
}

/// Every block a different state: the direct representation at its fullest.
fn worst_case() -> Chunk {
    let mut c = Chunk::filled(Dimension::OVERWORLD, 0, 0);
    for (i, s) in c.sections_mut().iter_mut().enumerate() {
        for k in 0..4096usize {
            s.blocks.set(k, ((k * 7 + i * 131) % 35000) as u16 + 1);
        }
        for k in 0..64usize {
            s.biomes.set(k, (k % 60) as u16);
        }
    }
    c
}

fn bench_container(c: &mut Criterion) {
    let mut g = c.benchmark_group("paletted container (4096 entries)");
    g.throughput(Throughput::Elements(4096));
    let mut mixed = PalettedContainer::filled(Kind::Blocks, 1);
    for i in 0..4096 {
        mixed.set(i, (i % 12) as u16);
    }
    g.bench_function("get, 12-value palette (4-bit)", |b| {
        b.iter(|| (0..4096).fold(0u32, |a, i| a.wrapping_add(u32::from(mixed.get(black_box(i))))))
    });
    g.bench_function("set, 12-value palette (4-bit)", |b| {
        let mut c = mixed.clone();
        b.iter(|| {
            for i in 0..4096 {
                c.set(black_box(i), (i % 12) as u16);
            }
        })
    });
    let uniform = PalettedContainer::filled(Kind::Blocks, 9);
    g.bench_function("get, uniform", |b| {
        b.iter(|| (0..4096).fold(0u32, |a, i| a.wrapping_add(u32::from(uniform.get(black_box(i))))))
    });
    g.finish();
}

fn report_sizes() {
    let limits = Limits::vanilla();
    eprintln!("\nchunk sizes (raw = before zstd, then at each zstd level):");
    eprintln!(
        "{:<14}{:>9}{:>9}{:>9}{:>9}{:>9}{:>9}",
        "chunk", "raw", "zstd1", "zstd3", "zstd6", "zstd9", "zstd15"
    );
    for (name, chunk) in [
        ("empty", Chunk::filled(Dimension::OVERWORLD, 0, 0)),
        ("flat", flat()),
        ("terrain", terrain(3, -2)),
        ("worst case", worst_case()),
    ] {
        let raw = encode_raw(&chunk).len();
        let sizes: Vec<usize> =
            [1, 3, 6, 9, 15].iter().map(|&l| encode(&chunk, l).unwrap().len()).collect();
        eprintln!(
            "{name:<14}{raw:>9}{:>9}{:>9}{:>9}{:>9}{:>9}",
            sizes[0], sizes[1], sizes[2], sizes[3], sizes[4]
        );
        assert_eq!(decode(&encode(&chunk, 3).unwrap(), &limits).unwrap(), chunk);
    }
    eprintln!();
}

fn bench_codec(c: &mut Criterion) {
    report_sizes();
    let limits = Limits::vanilla();
    let chunks = [("flat", flat()), ("terrain", terrain(3, -2)), ("worst case", worst_case())];

    for (name, chunk) in &chunks {
        let mut g = c.benchmark_group(format!("codec: {name}"));
        g.throughput(Throughput::Elements(1));
        g.bench_function("serialise (no compression)", |b| b.iter(|| black_box(encode_raw(chunk))));
        for level in [1, 3, 6, 9] {
            g.bench_function(format!("encode, zstd {level}"), |b| {
                b.iter(|| black_box(encode(chunk, level).unwrap()))
            });
        }
        let packed = encode(chunk, 3).unwrap();
        g.bench_function("decode (zstd 3 data)", |b| {
            b.iter(|| black_box(decode(&packed, &limits).unwrap()))
        });
        g.finish();
    }
}

fn bench_store(c: &mut Criterion) {
    let limits = Limits::vanilla();
    let payloads: Vec<Vec<u8>> = (0..64).map(|i| encode(&terrain(i, i / 8), 3).unwrap()).collect();
    let avg = payloads.iter().map(Vec::len).sum::<usize>() / payloads.len();
    eprintln!("average terrain chunk payload: {avg} bytes");

    let mut g = c.benchmark_group("store");
    g.throughput(Throughput::Bytes(avg as u64));
    g.bench_function("put (no fsync)", |b| {
        let dir = TempDir::new().unwrap();
        let store = Store::open(dir.path(), Options::default()).unwrap();
        let mut i = 0i32;
        b.iter(|| {
            store.put(ChunkPos::new(i % 5000, i / 5000), &payloads[(i as usize) % 64]).unwrap();
            i += 1;
        })
    });
    g.bench_function("put + fsync every write", |b| {
        let dir = TempDir::new().unwrap();
        let store =
            Store::open(dir.path(), Options { sync_on_write: true, ..Options::default() }).unwrap();
        let mut i = 0i32;
        b.iter(|| {
            store.put(ChunkPos::new(i, 0), &payloads[(i as usize) % 64]).unwrap();
            i += 1;
        })
    });
    {
        let dir = TempDir::new().unwrap();
        let store = Store::open(dir.path(), Options::default()).unwrap();
        for i in 0..4096 {
            store.put(ChunkPos::new(i % 64, i / 64), &payloads[(i as usize) % 64]).unwrap();
        }
        g.bench_function("get (verifies checksum)", |b| {
            let mut i = 0;
            b.iter(|| {
                i = (i + 17) % 4096;
                black_box(store.get(ChunkPos::new(i % 64, i / 64)).unwrap())
            })
        });
        g.bench_function("get + decode chunk", |b| {
            let mut i = 0;
            b.iter(|| {
                i = (i + 17) % 4096;
                let bytes = store.get(ChunkPos::new(i % 64, i / 64)).unwrap().unwrap();
                black_box(decode(&bytes, &limits).unwrap())
            })
        });
    }
    g.finish();

    let mut g = c.benchmark_group("store: open and compaction (20,000 chunks)");
    g.sample_size(10);
    g.bench_function("open (rebuild the index)", |b| {
        let dir = TempDir::new().unwrap();
        {
            let store = Store::open(
                dir.path(),
                Options { max_segment_bytes: 64 << 20, ..Options::default() },
            )
            .unwrap();
            for i in 0..20_000 {
                store.put(ChunkPos::new(i % 200, i / 200), &payloads[(i as usize) % 64]).unwrap();
            }
            store.sync().unwrap();
        }
        b.iter(|| {
            black_box(
                Store::open(
                    dir.path(),
                    Options { max_segment_bytes: 64 << 20, ..Options::default() },
                )
                .unwrap(),
            )
        })
    });
    g.bench_function("compact (50% dead)", |b| {
        b.iter_batched(
            || {
                let dir = TempDir::new().unwrap();
                let store = Store::open(
                    dir.path(),
                    Options { max_segment_bytes: 8 << 20, ..Options::default() },
                )
                .unwrap();
                for round in 0..2 {
                    for i in 0..10_000 {
                        store
                            .put(
                                ChunkPos::new(i % 100, i / 100),
                                &payloads[(i as usize + round) % 64],
                            )
                            .unwrap();
                    }
                }
                (dir, store)
            },
            |(dir, store)| {
                black_box(store.compact().unwrap());
                (dir, store)
            },
            BatchSize::PerIteration,
        )
    });
    g.finish();
}

criterion_group!(benches, bench_container, bench_codec, bench_store);
criterion_main!(benches);
