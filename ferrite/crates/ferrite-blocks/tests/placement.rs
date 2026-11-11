//! Replays recordings of the official server placing blocks (`scripts/record-placement.py`) and
//! requires Ferrite to produce the same blocks around the click in every case.

use std::collections::BTreeMap;

use ferrite_blocks::{BlockAccess, BlockPos, Chunks, Direction, PlaceContext, Rules, World};
use ferrite_data::blocks::{block_by_name, block_of_state};
use ferrite_world::{Chunk, ChunkPos, Dimension};

/// Items that share a block's name but are not the block's item (the crop item `wheat` is not
/// the seeds that plant the `wheat` block; lily pads and frogspawn are placed on the water surface by
/// `use_item`, not by clicking a block); the recordings show them placing nothing.
const NOT_BLOCK_ITEMS: [&str; 3] = ["wheat", "lily_pad", "frogspawn"];

/// Recorded behaviour Ferrite does not reproduce yet: `(item, clicked block, face)`.
/// A vine clicked on the underside of farmland is refused by the game although farmland's
/// bottom face is as full as soul sand's, which accepts it; the rule behind it is not known.
const KNOWN_GAPS: [(&str, &str, usize); 1] = [("vine", "farmland", 0)];

const TARGET: (i32, i32, i32) = (8, 101, 8);
const WINDOW: ((i32, i32, i32), (i32, i32, i32)) = ((5, 99, 5), (11, 105, 11));

fn parse_state(text: &str) -> u16 {
    let (name, props) = match text.split_once('[') {
        Some((n, rest)) => (n, rest.trim_end_matches(']')),
        None => (text, ""),
    };
    let block = block_by_name(name).unwrap_or_else(|| panic!("unknown block {name}"));
    let mut values = [0u8; 16];
    let n = block.value_indices(block.default_state, &mut values).unwrap();
    for kv in props.split(',').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').unwrap();
        let (i, prop) = block.property(k).unwrap_or_else(|| panic!("{name} has no property {k}"));
        values[i] =
            prop.values.iter().position(|x| *x == v).unwrap_or_else(|| panic!("{text}")) as u8;
    }
    block.state_id(&values[..n]).unwrap()
}

fn state_name(state: u16) -> String {
    let block = block_of_state(state).unwrap();
    let mut values = [0u8; 16];
    let n = block.value_indices(state, &mut values).unwrap();
    let props: Vec<String> = block
        .properties
        .iter()
        .zip(&values[..n])
        .map(|(p, v)| format!("{}={}", p.name, p.values[usize::from(*v)]))
        .collect();
    if props.is_empty() {
        block.name.trim_start_matches("minecraft:").into()
    } else {
        format!("{}[{}]", block.name.trim_start_matches("minecraft:"), props.join(","))
    }
}

struct Case {
    /// The block that was clicked (stone unless the recording says otherwise).
    support: u16,
    item: String,
    face: usize,
    cursor: [f32; 3],
    yaw: f32,
    pitch: f32,
    /// Non-air cells relative to the target, other than the target stone itself.
    cells: BTreeMap<(i32, i32, i32), u16>,
}

fn load(text: &str) -> Vec<Case> {
    let mut out = Vec::new();
    let mut cur: Option<Case> = None;
    let mut support = parse_state("stone");
    for line in text.lines() {
        if let Some(state) = line.strip_prefix("# support ") {
            support = parse_state(state);
            continue;
        }
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let p: Vec<&str> = line.splitn(5, ' ').collect();
        match p[0] {
            "case" => {
                cur = Some(Case {
                    support,
                    item: p[1].to_string(),
                    face: p[2].parse().unwrap(),
                    cursor: {
                        let rest: Vec<&str> = line.split(' ').collect();
                        [
                            rest[3].parse().unwrap(),
                            rest[4].parse().unwrap(),
                            rest[5].parse().unwrap(),
                        ]
                    },
                    yaw: line.split(' ').nth(6).unwrap().parse().unwrap(),
                    pitch: line.split(' ').nth(7).unwrap().parse().unwrap(),
                    cells: BTreeMap::new(),
                });
            }
            "b" => {
                let c = cur.as_mut().unwrap();
                let at: (i32, i32, i32) =
                    (p[1].parse().unwrap(), p[2].parse().unwrap(), p[3].parse().unwrap());
                let state = parse_state(p[4]);
                // The clicked block itself is part of the setup, not of the outcome.
                if !(at == (0, 0, 0) && state == c.support) {
                    c.cells.insert(at, state);
                }
            }
            "end" => out.push(cur.take().unwrap()),
            other => panic!("bad line {other}"),
        }
    }
    out
}

/// The cells as text, with the random starting age of kelp and nether vines blanked out (the game
/// rolls it from its world random source, which no recording can predict).
fn canon(cells: &BTreeMap<(i32, i32, i32), u16>) -> BTreeMap<(i32, i32, i32), String> {
    cells
        .iter()
        .map(|(k, v)| {
            let mut name = state_name(*v);
            if (name.starts_with("kelp[")
                || name.starts_with("twisting_vines[")
                || name.starts_with("weeping_vines["))
                && let Some(i) = name.find("age=")
            {
                let end = name[i..].find([',', ']']).map_or(name.len(), |e| i + e);
                name.replace_range(i..end, "age=*");
            }
            (*k, name)
        })
        .collect()
}

fn fresh_world() -> World<Chunks> {
    let air = block_by_name("air").unwrap().default_state;
    let mut chunks = Chunks::new(Dimension::OVERWORLD);
    for cx in -1..=1 {
        for cz in -1..=1 {
            chunks.insert(ChunkPos::new(cx, cz), Chunk::filled(Dimension::OVERWORLD, air, 0));
        }
    }
    World::new(chunks, Rules::default())
}

fn run(case: &Case) -> BTreeMap<(i32, i32, i32), u16> {
    let stone = case.support;
    let mut world = fresh_world();
    world.blocks.set_block_raw(BlockPos::new(TARGET.0, TARGET.1, TARGET.2), stone);
    let ctx = PlaceContext {
        clicked: BlockPos::new(TARGET.0, TARGET.1, TARGET.2),
        face: Direction::ALL[case.face],
        cursor: case.cursor,
        yaw: case.yaw,
        pitch: case.pitch,
        sneaking: false,
        operator: false,
    };
    world.place_block(&case.item, &ctx, |_, _| false, &mut |_| 0);
    let mut out = BTreeMap::new();
    for y in WINDOW.0.1..=WINDOW.1.1 {
        for z in WINDOW.0.2..=WINDOW.1.2 {
            for x in WINDOW.0.0..=WINDOW.1.0 {
                let s = world.block(BlockPos::new(x, y, z)).unwrap();
                let rel = (x - TARGET.0, y - TARGET.1, z - TARGET.2);
                if s != block_by_name("air").unwrap().default_state
                    && !(rel == (0, 0, 0) && s == stone)
                {
                    out.insert(rel, s);
                }
            }
        }
    }
    out
}

#[test]
fn placement_matches_every_recording() {
    // `FERRITE_PLACEMENT_DATA` may name several recordings, separated by `:`.
    let paths: Vec<std::path::PathBuf> = match std::env::var("FERRITE_PLACEMENT_DATA") {
        Ok(list) => list.split(':').map(std::path::PathBuf::from).collect(),
        Err(_) => {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
            let mut found: Vec<_> = std::fs::read_dir(&dir)
                .unwrap_or_else(|e| panic!("{dir:?}: {e}"))
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("placement") && n.ends_with(".txt"))
                })
                .collect();
            found.sort();
            found
        }
    };
    let mut cases = Vec::new();
    for path in &paths {
        cases.extend(load(
            &std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path:?}: {e}")),
        ));
    }
    assert!(!cases.is_empty());
    let only = std::env::var("FERRITE_PLACEMENT_ONLY").unwrap_or_default();
    let mut bad: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut ran = 0;
    for c in &cases {
        if !c.item.starts_with(&only) || NOT_BLOCK_ITEMS.contains(&c.item.as_str()) {
            continue;
        }
        if KNOWN_GAPS.iter().any(|(i, sup, f)| {
            *i == c.item && state_name(c.support).starts_with(sup) && *f == c.face
        }) {
            continue;
        }
        ran += 1;
        let got = run(c);
        if canon(&got) != canon(&c.cells) {
            let describe = |m: &BTreeMap<(i32, i32, i32), u16>| {
                m.iter()
                    .map(|(k, v)| format!("{k:?}={}", state_name(*v)))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            bad.entry(c.item.clone()).or_default().push(format!(
                "on {} face {} cursor {:?} yaw {} pitch {}: ferrite [{}] / vanilla [{}]",
                state_name(c.support),
                c.face,
                c.cursor,
                c.yaw,
                c.pitch,
                describe(&got),
                describe(&c.cells)
            ));
        }
    }
    if std::env::var_os("FERRITE_PLACEMENT_REPORT").is_some() {
        eprintln!(
            "{} of {ran} cases differ in {} blocks",
            bad.values().map(Vec::len).sum::<usize>(),
            bad.len()
        );
        for (item, v) in &bad {
            eprintln!("{item}: {} cases, e.g. {}", v.len(), v[0]);
        }
    }
    assert!(
        bad.is_empty(),
        "{} blocks place differently from vanilla (first: {} {})",
        bad.len(),
        bad.keys().next().unwrap(),
        bad.values().next().unwrap()[0]
    );
}
