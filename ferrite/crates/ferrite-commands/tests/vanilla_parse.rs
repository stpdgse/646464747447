//! Differential test: Ferrite's parser against the official dispatcher (`java/ParseOracle.java`).
//!
//! Random command lines are walked through the vanilla tree (valid words, garbage, boundary
//! values, truncations); both parsers must agree on the matched nodes or the error and where it
//! is. Needs a JDK and the server jar: `scripts/vanilla-commands.sh <unpacked bundler dir>`.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;

use ferrite_commands::args::Source;
use ferrite_commands::dispatch::{parse, parse_error};
use ferrite_commands::tree::{Node, NodeKind, Parser, ResourceKind, StringKind, Tree, id};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

fn numbers(min: Option<f64>, max: Option<f64>, int: bool, rng: &mut Rng) -> String {
    let mut pool: Vec<String> = ["0", "1", "-1", "5", "42", "100", "abc", "", "-", "1-2", "007"]
        .iter()
        .map(ToString::to_string)
        .collect();
    if int {
        pool.extend(
            ["99999999999", "1.5", "2147483647", "-2147483648", "2147483648"].map(String::from),
        );
    } else {
        pool.extend(
            [
                ".5",
                "5.",
                ".",
                "1.5",
                "-2.25",
                "1e5",
                "1.2.3",
                "3.4028235E38",
                "100000000000",
                "0.0001",
            ]
            .map(String::from),
        );
    }
    if let Some(m) = min {
        pool.push(format!("{}", m as i64));
        pool.push(format!("{}", m as i64 - 1));
    }
    if let Some(m) = max {
        pool.push(format!("{}", m as i64));
        pool.push(format!("{}", m as i64 + 1));
    }
    rng.pick(&pool).clone()
}

/// A word for an argument; `None` for argument types the parser does not handle yet.
fn selector_sample(rng: &mut Rng) -> String {
    if rng.chance(30) {
        return pick_str(
            rng,
            &[
                "Steve",
                "steve",
                "Steve_1",
                "\"Steve\"",
                "12345678-1234-1234-1234-123456789abc",
                "1-2-3-4-5",
                "1234",
                "",
                "Steve123456789012345678901",
                "a b",
                "@",
                "@x",
                "@A",
            ],
        );
    }
    let mut out = String::from("@");
    out.push(*rng.pick(&['a', 'e', 'p', 'r', 's', 'n', 'a', 'e']));
    if rng.chance(70) {
        let options = [
            "nbt={}",
            "nbt={a:1}",
            "nbt=!{a:1b,b:[1,2]}",
            "nbt=1",
            "nbt={a}",
            "nbt=",
            "nbt={a:",
            "nbt= {a:'x'}",
            "nbt=!1",
            "nbt=[]",
            "nbt={a:1L}x",
            "limit=1",
            "limit=0",
            "limit=x",
            "limit=3",
            "type=cow",
            "type=!cow",
            "type=pig",
            "type=player",
            "type=nope",
            "type=#minecraft:skeletons",
            "type=#foo",
            "type=!pig",
            "name=Steve",
            "name=!Steve",
            "name=\"a b\"",
            "name=",
            "distance=..5",
            "distance=-1..5",
            "distance=1.5",
            "distance=5..1",
            "level=1..5",
            "level=1.5",
            "level=..3",
            "x=1",
            "x=1.5",
            "x=a",
            "y=64",
            "z=-3",
            "dx=1",
            "dy=2",
            "dz=a",
            "gamemode=creative",
            "gamemode=!survival",
            "gamemode=foo",
            "gamemode=",
            "team=red",
            "team=!red",
            "team=",
            "tag=foo",
            "tag=!foo",
            "tag=",
            "sort=nearest",
            "sort=random",
            "sort=foo",
            "sort=furthest",
            "x_rotation=0..90",
            "y_rotation=..90",
            "x_rotation=a",
            "scores={a=1}",
            "scores={a=1..5,b=..3}",
            "scores={a=}",
            "scores={}",
            "scores=1",
            "scores={a=1",
            "advancements={a=true}",
            "advancements={a={b=true}}",
            "advancements={}",
            "advancements={a=maybe}",
            "predicate=foo",
            "predicate=!foo",
            "foo=bar",
            "foo",
            "limit",
            "",
            " limit=1 ",
        ];
        let n = 1 + rng.below(3);
        let list: Vec<String> = (0..n).map(|_| rng.pick(&options).to_string()).collect();
        out.push('[');
        out.push_str(&list.join(","));
        if !rng.chance(8) {
            out.push(']');
        }
    }
    out
}

fn pick_str(rng: &mut Rng, pool: &[&str]) -> String {
    rng.pick(pool).to_string()
}

fn sample(node: &Node, rng: &mut Rng) -> Option<String> {
    let parser = node.parser.as_ref()?;
    Some(match parser {
        Parser::Bool => rng.pick(&["true", "false", "maybe", "TRUE", "", "tru"]).to_string(),
        Parser::Integer { min, max } => numbers(min.map(f64::from), max.map(f64::from), true, rng),
        Parser::Long { min, max } => {
            numbers(min.map(|v| v as f64), max.map(|v| v as f64), true, rng)
        }
        Parser::Float { min, max } => numbers(min.map(f64::from), max.map(f64::from), false, rng),
        Parser::Double { min, max } => numbers(*min, *max, false, rng),
        Parser::String(StringKind::Word) => {
            rng.pick(&["abc", "a_b-c.d+e", "Steve", "x", "a:b", "\"q\""]).to_string()
        }
        Parser::String(StringKind::Quotable) => rng
            .pick(&[
                "abc",
                "\"hello world\"",
                "'single quoted'",
                "\"unterminated",
                "\"bad\\escape\"",
                "\"esc\\\"aped\"",
                "a b",
            ])
            .to_string(),
        Parser::String(StringKind::Greedy) => {
            rng.pick(&["hello world", "x", "  spaced  out ", "\"quoted\" text"]).to_string()
        }
        Parser::Simple(id::BLOCK_POS | id::VEC3 | id::COLUMN_POS | id::VEC2 | id::ROTATION) => {
            let one = |rng: &mut Rng| -> String {
                rng.pick(&[
                    "0", "1", "-5", "~", "~1", "~-2.5", "1.5", "^", "^2", "x", "~x", "", ".", "3.",
                    "0.5",
                ])
                .to_string()
            };
            let n = if matches!(parser, Parser::Simple(id::BLOCK_POS | id::VEC3)) { 3 } else { 2 };
            let count = if rng.chance(15) { rng.below(4) } else { n };
            (0..count).map(|_| one(rng)).collect::<Vec<_>>().join(" ")
        }
        Parser::Simple(id::GAMEMODE) => pick_str(
            rng,
            &["survival", "creative", "adventure", "spectator", "Creative", "foo", "", "s"],
        ),
        Parser::Time { .. } => pick_str(
            rng,
            &["5", "5d", "5s", "5t", "1.5s", "-3", "0", "5q", "abc", "", "1h", "0.5d", "100000d"],
        ),
        Parser::Simple(id::RESOURCE_LOCATION | id::DIMENSION) => pick_str(
            rng,
            &[
                "minecraft:overworld",
                "overworld",
                "foo:bar",
                "a:b:c",
                "Foo",
                "",
                ":x",
                "x/y",
                "a/b:c",
                "the_nether",
            ],
        ),
        Parser::Simple(id::ENTITY_ANCHOR) => pick_str(rng, &["feet", "eyes", "foot", "Eyes", ""]),
        Parser::Simple(id::TEAM_COLOR) => pick_str(
            rng,
            &["red", "dark_red", "DARK_RED", "reset", "bold", "gold", "", "white", "nope"],
        ),
        Parser::Simple(id::OPERATION) => {
            pick_str(rng, &["=", "+=", "-=", "*=", "/=", "%=", "<", ">", "><", "?=", "", "=="])
        }
        Parser::Simple(id::SWIZZLE) => {
            pick_str(rng, &["x", "y", "z", "xy", "xyz", "xx", "w", "", "xzy", "yz"])
        }
        Parser::Simple(id::HEIGHTMAP) => pick_str(
            rng,
            &[
                "world_surface",
                "ocean_floor",
                "motion_blocking",
                "motion_blocking_no_leaves",
                "foo",
                "world_surface_wg",
            ],
        ),
        Parser::Simple(id::TEMPLATE_MIRROR) => {
            pick_str(rng, &["none", "left_right", "front_back", "left", "x"])
        }
        Parser::Simple(id::TEMPLATE_ROTATION) => {
            pick_str(rng, &["none", "clockwise_90", "180", "counterclockwise_90", "90", "flip"])
        }
        Parser::Simple(id::SCOREBOARD_SLOT) => pick_str(
            rng,
            &[
                "list",
                "sidebar",
                "below_name",
                "sidebar.team.red",
                "sidebar.team.reset",
                "sidebar.team.bold",
                "foo",
                "SIDEBAR",
            ],
        ),
        Parser::Simple(id::INT_RANGE | id::FLOAT_RANGE) => pick_str(
            rng,
            &[
                "1..5", "..5", "1..", "5", "-3..-1", "1.5", "a", "5..1", "..", "1...", "..-1", "",
                "--1", "1.5..2.5", ".5..", "1e3", "3..3",
            ],
        ),
        Parser::Resource { kind: ResourceKind::Resource, registry } => {
            let mut pool: Vec<String> =
                vec!["foo".into(), "minecraft:foo".into(), "".into(), "Cow".into(), "a:b".into()];
            if let Some(r) = ferrite_data::registries::Registry::find(registry) {
                for _ in 0..4 {
                    let e = r.entries[rng.below(r.entries.len())];
                    pool.push(e.to_string());
                    pool.push(e.trim_start_matches("minecraft:").to_string());
                }
            } else if let Some(r) = ferrite_data::configuration::SYNCHRONIZED_REGISTRIES
                .iter()
                .find(|r| r.id == registry)
            {
                for _ in 0..4 {
                    pool.push(r.entries[rng.below(r.entries.len())].to_string());
                }
            }
            rng.pick(&pool).clone()
        }
        Parser::Entity { .. } => Some(selector_sample(rng))?,
        Parser::ScoreHolder { .. } => {
            if rng.chance(40) {
                selector_sample(rng)
            } else {
                pick_str(
                    rng,
                    &[
                        "Steve",
                        "*",
                        "",
                        "a b",
                        "\"a b\"",
                        "#fake",
                        "12345678-1234-1234-1234-123456789abc",
                        "Steve123456789012345678901",
                    ],
                )
            }
        }
        Parser::Simple(id::OBJECTIVE | id::TEAM) => pick_str(
            rng,
            &[
                "abc",
                "a b",
                "\"a b\"",
                "",
                "a.b",
                "a:b",
                "a-b_c+d",
                "verylongobjectivenamethatisreallyverylongindeed",
            ],
        ),
        Parser::Simple(id::GAME_PROFILE) => {
            if rng.chance(50) {
                selector_sample(rng)
            } else {
                pick_str(
                    rng,
                    &[
                        "Steve",
                        "",
                        "Steve123456789012345678901",
                        "12345678-1234-1234-1234-123456789abc",
                        "a b",
                    ],
                )
            }
        }
        Parser::Simple(id::UUID) => pick_str(
            rng,
            &[
                "12345678-1234-1234-1234-123456789abc",
                "x",
                "",
                "12345678123412341234123456789abc",
                "1-2-3-4-5",
                "12345678-1234-1234-1234-123456789abcdef",
                "-----",
            ],
        ),
        Parser::Simple(id::MESSAGE) => pick_str(
            rng,
            &[
                "hello",
                "@a",
                "@x",
                "@",
                "hello @a world",
                "@e[type=cow",
                "hi @s[",
                "@p[limit=2]",
                "@a[limit=0]",
                "x @e[type=nope] y",
                "@@a",
                "a@b",
                "@a@e",
            ],
        ),
        Parser::Simple(id::ITEM_STACK) => {
            let items = ferrite_data::registries::Registry::find("minecraft:item")
                .expect("item registry")
                .entries;
            let mut out = if rng.chance(6) {
                pick_str(rng, &["nope", "minecraft:", "stone ", "#minecraft:logs", "Stone", "a:b"])
            } else {
                items[rng.below(items.len())].to_string()
            };
            if rng.chance(55) {
                out.push('[');
                let validated = ferrite_commands::components::VALIDATED;
                let all = ferrite_data::registries::Registry::find("minecraft:data_component_type")
                    .expect("component registry");
                let mut parts: Vec<String> = Vec::new();
                for _ in 0..rng.below(4) {
                    let name = if rng.chance(75) {
                        validated[rng.below(validated.len())].to_string()
                    } else if rng.chance(70) {
                        let n = rng.below(all.entries.len());
                        all.entries[n].to_string()
                    } else {
                        pick_str(rng, &["nope", "minecraft:nope", "", "Damage", "x:y"])
                    };
                    let part = if rng.chance(15) {
                        format!("!{name}")
                    } else if rng.chance(8) {
                        name
                    } else if validated.contains(&name.as_str()) {
                        let v = pick_str(
                            rng,
                            &[
                                "1",
                                "0",
                                "-1",
                                "99",
                                "100",
                                "1.5",
                                "2b",
                                "true",
                                "\"x\"",
                                "[1]",
                                "{}",
                                "{a:1}",
                                "common",
                                "epic",
                                "rare",
                                "nope",
                                "'{a:1}'",
                                "'x'",
                                "",
                                "5L",
                                "1e3",
                                "abc",
                                "100000000000",
                            ],
                        );
                        format!("{name}={v}")
                    } else if rng.chance(50) {
                        // The value of a component that is not checked yet is only read.
                        name.clone()
                    } else {
                        format!("{name}=")
                    };
                    parts.push(part);
                }
                out.push_str(&parts.join(if rng.chance(90) { "," } else { " , " }));
                if rng.chance(92) {
                    out.push(']');
                }
            }
            out
        }
        Parser::Simple(id::NBT_COMPOUND_TAG | id::NBT_TAG) => pick_str(
            rng,
            &[
                "{}",
                "{a:1}",
                "{a:1b,b:[1,2],c:{d:\"x\"}}",
                "{ a : 1 }",
                "1",
                "[1]",
                "\"x\"",
                "x",
                "{a}",
                "{a:",
                "{a:1}x",
                "{a:1} x",
                "",
                " {a:1}",
                "{a:0x1F}",
                "{a:1_0}",
                "{a:1.5e3f}",
                "{\"k y\":[B;1,2]}",
                "[I;1]",
                "bool(1)",
                "1b",
                "{a:uuid(\"0-0-0-0-1\")}",
                "-",
                "{a:1,}",
            ],
        ),
        Parser::Simple(id::BLOCK_STATE | id::BLOCK_PREDICATE) => {
            let blocks = ferrite_data::blocks::BLOCKS;
            let b = &blocks[rng.below(blocks.len())];
            let mut out = if rng.chance(8) {
                "#minecraft:logs".to_string()
            } else if rng.chance(5) {
                "nope".to_string()
            } else {
                b.name.to_string()
            };
            if rng.chance(60) {
                out.push('[');
                let n = rng.below(3);
                let mut parts = Vec::new();
                for _ in 0..n {
                    if b.properties.is_empty() || rng.chance(15) {
                        parts.push(pick_str(
                            rng,
                            &["x=1", "facing=north", "facing", "=1", "", "waterlogged=maybe"],
                        ));
                    } else {
                        let p = &b.properties[rng.below(b.properties.len())];
                        let v = if rng.chance(15) {
                            "zzz"
                        } else {
                            p.values[rng.below(p.values.len())]
                        };
                        parts.push(format!("{}={}", p.name, v));
                    }
                }
                out.push_str(&parts.join(if rng.chance(90) { "," } else { " " }));
                if rng.chance(90) {
                    out.push(']');
                }
            }
            if rng.chance(12) {
                out.push_str(rng.pick(&["{}", "{a:1}", "{a", "{a:1,b:[1]}", "{ x : 'y' }", "{1}"]));
            }
            out
        }
        Parser::Simple(id::ANGLE) if false => unreachable!(),
        Parser::Simple(id::ANGLE) => {
            rng.pick(&["0", "90", "~", "~10", "-45.5", "x", "", "180f", "1e3"]).to_string()
        }
        _ => return None,
    })
}

fn generate(tree: &Tree, rng: &mut Rng) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut node = tree.root;
    for _ in 0..14 {
        let children = &tree.nodes[node].children;
        if children.is_empty() || (tree.nodes[node].executable && rng.chance(25)) {
            break;
        }
        let child = *rng.pick(children);
        let n = &tree.nodes[child];
        let word = match n.kind {
            NodeKind::Literal => {
                let mut w = n.name.clone();
                match rng.below(20) {
                    0 => w.truncate(w.len().saturating_sub(1)),
                    1 => w.push('x'),
                    2 => w = w.to_uppercase(),
                    _ => {}
                }
                w
            }
            NodeKind::Argument => match sample(n, rng) {
                Some(w) => w,
                None => break,
            },
            NodeKind::Root => break,
        };
        words.push(word);
        node = n.redirect.unwrap_or(child);
    }
    let mut line = words.join(" ");
    match rng.below(12) {
        0 => line.push(' '),
        1 => line.push_str(" extra"),
        2 => line = line.trim_end().to_string(),
        _ => {}
    }
    line
}

fn ours(tree: &Tree, level: usize, line: &str) -> Option<String> {
    let results = parse(tree, line, Source { permission_level: level as u8 });
    if results.unsupported {
        return None;
    }
    Some(match parse_error(&results) {
        Some(e) => format!("ERR {} {}", e.cursor, e.message.to_json()),
        None => {
            let mut nodes = String::new();
            let mut ctx = Some(&results.context);
            let mut first = true;
            while let Some(c) = ctx {
                if !first {
                    nodes.push('>');
                }
                first = false;
                for (i, (n, _)) in c.nodes.iter().enumerate() {
                    if i > 0 {
                        nodes.push('/');
                    }
                    nodes.push_str(&tree.nodes[*n].name);
                }
                ctx = c.child.as_deref();
            }
            format!("OK {nodes} 1")
        }
    })
}

fn run_oracle(input: &str) -> Vec<String> {
    let env = |k: &str| std::env::var(k).ok();
    let classpath = env("FERRITE_VANILLA_CLASSPATH").expect(
        "set FERRITE_VANILLA_CLASSPATH to the server jar and its libraries (scripts/vanilla-commands.sh)",
    );
    let java = env("FERRITE_JAVA").unwrap_or_else(|| "java".into());
    let javac = env("FERRITE_JAVAC").unwrap_or_else(|| "javac".into());
    let dir = std::env::temp_dir().join(format!("ferrite-parse-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (input_path, output_path) = (dir.join("in.txt"), dir.join("out.txt"));
    std::fs::write(&input_path, input).unwrap();
    let source: PathBuf = [env!("CARGO_MANIFEST_DIR"), "java", "ParseOracle.java"].iter().collect();
    let status = Command::new(&javac)
        .args(["-nowarn", "-cp", &classpath, "-d"])
        .arg(&dir)
        .arg(&source)
        .status()
        .expect("javac");
    assert!(status.success(), "javac failed");
    let out = Command::new(&java)
        .args(["-cp", &format!("{}:{classpath}", dir.display()), "ParseOracle"])
        .arg(&input_path)
        .arg(&output_path)
        .output()
        .expect("java");
    let text = std::fs::read_to_string(&output_path).unwrap_or_else(|_| {
        panic!("the oracle wrote nothing:\n{}", String::from_utf8_lossy(&out.stderr))
    });
    let _ = std::fs::remove_dir_all(&dir);
    text.lines().map(str::to_string).collect()
}

#[test]
#[ignore = "needs the official server jar (scripts/vanilla-commands.sh)"]
fn parsing_agrees_with_the_official_dispatcher() {
    let trees: Vec<Tree> = (0..5).map(Tree::for_level).collect();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let count: usize =
        std::env::var("FERRITE_PARSE_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(20000);
    let mut cases: Vec<(usize, String)> = Vec::new();
    while cases.len() < count {
        let level = match rng.below(10) {
            0 => 0,
            1 => 2,
            2 => 3,
            _ => 4,
        };
        let line = generate(&trees[level], &mut rng);
        if line.contains('\n') {
            continue;
        }
        cases.push((level, line));
    }
    let mut input = String::new();
    for (level, line) in &cases {
        let _ = writeln!(input, "{level} {line}");
    }
    let theirs = run_oracle(&input);
    assert_eq!(
        theirs.len(),
        cases.len(),
        "the oracle answered {} of {}",
        theirs.len(),
        cases.len()
    );
    let (mut compared, mut skipped, mut bad) = (0, 0, Vec::new());
    for ((level, line), want) in cases.iter().zip(&theirs) {
        let Some(got) = ours(&trees[*level], *level, line) else {
            skipped += 1;
            continue;
        };
        compared += 1;
        if &got != want {
            bad.push(format!("level {level}: {line:?}\n    vanilla {want}\n    ferrite {got}"));
        }
    }
    eprintln!("{compared} lines compared, {skipped} skipped (argument types not supported yet)");
    let mut histogram = std::collections::BTreeMap::new();
    for want in &theirs {
        let key = match want.split_once(' ') {
            Some(("OK", _)) => "OK".to_string(),
            Some(("ERR", rest)) => {
                let json = rest.split_once(' ').map_or("", |x| x.1);
                json.split('"').nth(3).unwrap_or(json).to_string()
            }
            _ => want.clone(),
        };
        *histogram.entry(key).or_insert(0) += 1;
    }
    eprintln!("outcomes: {histogram:?}");
    assert!(compared > 100);
    assert!(
        bad.is_empty(),
        "{} of {compared} lines differ:\n{}",
        bad.len(),
        bad.iter().take(30).cloned().collect::<Vec<_>>().join("\n")
    );
}
