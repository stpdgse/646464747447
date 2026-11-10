//! Replays recordings of the official server (see `scripts/record-physics.py`) and requires
//! Ferrite to produce exactly the same blocks and the same pending fluid ticks, in the same
//! order, at every recorded moment.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use ferrite_blocks::{BlockAccess, BlockPos, Chunks, Rules, World};
use ferrite_data::blocks::{block_by_name, block_of_state};
use ferrite_world::{Chunk, ChunkPos, Dimension};

const Y0: i32 = 96;
const Y1: i32 = 111;

/// `name[key=value,...]` -> state id; unnamed properties keep the block's default.
fn parse_state(text: &str) -> u16 {
    let (name, props) = match text.split_once('[') {
        Some((n, rest)) => {
            (n, rest.strip_suffix(']').unwrap_or_else(|| panic!("bad state {text}")))
        }
        None => (text, ""),
    };
    let block = block_by_name(name).unwrap_or_else(|| panic!("unknown block {name}"));
    let mut values = [0u8; 16];
    let n = block.value_indices(block.default_state, &mut values).unwrap();
    for kv in props.split(',').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').unwrap_or_else(|| panic!("bad property {kv} in {text}"));
        let (i, prop) = block.property(k).unwrap_or_else(|| panic!("{name} has no property {k}"));
        values[i] =
            prop.values.iter().position(|x| *x == v).unwrap_or_else(|| panic!("{text}: bad value"))
                as u8;
    }
    block.state_id(&values[..n]).unwrap()
}

fn state_name(state: u16) -> String {
    let block = block_of_state(state).unwrap();
    let mut values = [0u8; 16];
    let n = block.value_indices(state, &mut values).unwrap();
    let mut s = block.name.trim_start_matches("minecraft:").to_string();
    if n > 0 {
        let parts: Vec<String> = block
            .properties
            .iter()
            .zip(&values[..n])
            .map(|(p, v)| format!("{}={}", p.name, p.values[usize::from(*v)]))
            .collect();
        let _ = write!(s, "[{}]", parts.join(","));
    }
    s
}

fn fluid_name(id: u8) -> &'static str {
    ferrite_data::generated::registries::fluid::names[usize::from(id)]
        .trim_start_matches("minecraft:")
}

fn nums<const N: usize>(parts: &[&str]) -> [i32; N] {
    std::array::from_fn(|i| parts[i].parse().unwrap_or_else(|_| panic!("bad number in {parts:?}")))
}

type Blocks = BTreeMap<(i32, i32, i32), u16>;

fn region(world: &World<Chunks>) -> Blocks {
    let air = parse_state("air");
    let mut out = Blocks::new();
    for y in Y0..=Y1 {
        for z in 0..16 {
            for x in 0..16 {
                let s = world.block(BlockPos::new(x, y, z)).unwrap();
                if s != air {
                    out.insert((x, y, z), s);
                }
            }
        }
    }
    out
}

/// Replays one fixture; returns a description of the first mismatch, if any.
fn replay(text: &str) -> Result<(), String> {
    let air = parse_state("air");
    let mut chunks = Chunks::new(Dimension::OVERWORLD);
    // The arena chunk plus a ring around it, as on the recording server.
    for cx in -1..=1 {
        for cz in -1..=1 {
            chunks.insert(ChunkPos::new(cx, cz), Chunk::filled(Dimension::OVERWORLD, air, 0));
        }
    }
    let mut world = World::new(chunks, Rules::default());
    let mut initial = Blocks::new();
    let mut lines = text.lines().enumerate().filter(|(_, l)| !l.starts_with('#'));
    let mut step_no = 0;

    while let Some((line_no, line)) = lines.next() {
        let parts: Vec<&str> = line.split(' ').collect();
        match parts[0] {
            "init" => {
                for (_, l) in lines.by_ref() {
                    if l == "end" {
                        break;
                    }
                    let p: Vec<&str> = l.splitn(6, ' ').collect();
                    assert_eq!(p[0], "r", "bad init line {l}");
                    let [y, z, x0, x1] = nums(&p[1..5]);
                    let state = parse_state(p[5]);
                    for x in x0..=x1 {
                        world.blocks.set_block_raw(BlockPos::new(x, y, z), state);
                    }
                }
                initial = region(&world);
            }
            "set" => {
                let [x, y, z] = nums(&parts[1..4]);
                world.command_set_block(BlockPos::new(x, y, z), parse_state(parts[4]));
            }
            "step" => {
                for _ in 0..nums::<1>(&parts[1..2])[0] {
                    world.tick();
                }
                step_no += 1;
            }
            "expect" => {
                let mut want = initial.clone();
                let mut want_ticks = Vec::new();
                for (_, l) in lines.by_ref() {
                    if l == "end" {
                        break;
                    }
                    let p: Vec<&str> = l.splitn(5, ' ').collect();
                    let [x, y, z] = nums(&p[1..4]);
                    match p[0] {
                        "b" => {
                            let s = parse_state(p[4]);
                            if s == air {
                                want.remove(&(x, y, z));
                            } else {
                                want.insert((x, y, z), s);
                            }
                        }
                        "f" => want_ticks.push(format!("{x} {y} {z} {}", p[4])),
                        other => panic!("bad expect line kind {other}"),
                    }
                }
                let got = region(&world);
                if got != want {
                    let mut msg =
                        format!("blocks differ at snapshot {step_no} (line {}):\n", line_no + 1);
                    let keys: std::collections::BTreeSet<_> =
                        got.keys().chain(want.keys()).collect();
                    let mut shown = 0;
                    for k in keys {
                        let (g, w) = (got.get(k), want.get(k));
                        if g != w {
                            shown += 1;
                            if shown <= 12 {
                                let name = |s: Option<&u16>| {
                                    s.map_or("air".to_string(), |s| state_name(*s))
                                };
                                let _ = writeln!(
                                    msg,
                                    "  {k:?}: ferrite {} / vanilla {}",
                                    name(g),
                                    name(w)
                                );
                            }
                        }
                    }
                    let _ = writeln!(msg, "  ({shown} positions differ)");
                    return Err(msg);
                }
                let got_ticks: Vec<String> = world
                    .pending_fluid_ticks()
                    .iter()
                    .filter(|t| {
                        (Y0..=Y1).contains(&t.pos.y)
                            && (0..16).contains(&t.pos.x)
                            && (0..16).contains(&t.pos.z)
                    })
                    .map(|t| {
                        format!(
                            "{} {} {} {} {} {}",
                            t.pos.x,
                            t.pos.y,
                            t.pos.z,
                            fluid_name(t.kind),
                            t.delay,
                            t.priority
                        )
                    })
                    .collect();
                if got_ticks != want_ticks {
                    let first = got_ticks
                        .iter()
                        .zip(&want_ticks)
                        .position(|(a, b)| a != b)
                        .unwrap_or(got_ticks.len().min(want_ticks.len()));
                    return Err(format!(
                        "fluid ticks differ at snapshot {step_no} (line {}): {} vs vanilla {} entries; first difference at #{first}: ferrite {:?} / vanilla {:?}\n",
                        line_no + 1,
                        got_ticks.len(),
                        want_ticks.len(),
                        got_ticks.get(first),
                        want_ticks.get(first)
                    ));
                }
            }
            other => panic!("unknown fixture line {other:?}"),
        }
    }
    Ok(())
}

fn fixtures() -> Vec<(String, String)> {
    // FERRITE_FIXTURE_DIR points the test at a larger, uncommitted sweep of recordings.
    let dir = std::env::var_os("FERRITE_FIXTURE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"));
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .map(|p| {
            (
                p.file_stem().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

#[test]
fn ferrite_matches_every_vanilla_recording() {
    let all = fixtures();
    assert!(!all.is_empty(), "no fixtures found");
    let only = std::env::var("FERRITE_FIXTURE").unwrap_or_default();
    let mut failures = Vec::new();
    let mut ran = 0;
    for (name, text) in &all {
        if !name.starts_with(&only) {
            continue;
        }
        ran += 1;
        if let Err(e) = replay(text) {
            failures.push(format!("{name}: {e}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {ran} recordings differ from vanilla:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn state_names_round_trip() {
    for text in [
        "stone",
        "water[level=3]",
        "oak_stairs[facing=north,half=top,shape=straight,waterlogged=false]",
    ] {
        assert_eq!(state_name(parse_state(text)), text);
    }
    // Unnamed properties take the block's default.
    assert_eq!(
        parse_state("oak_stairs[facing=east]"),
        parse_state("oak_stairs[facing=east,half=bottom,shape=straight,waterlogged=false]")
    );
}
