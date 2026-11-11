//! Differential test: Ferrite's parser against the official dispatcher (`java/ParseOracle.java`).
//!
//! Random command lines are walked through the vanilla tree (valid words, garbage, boundary
//! values, truncations); both parsers must agree on the matched nodes or the error and where it
//! is. Needs a JDK and the server jar: `scripts/vanilla-commands.sh <unpacked bundler dir>`.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;

use ferrite_commands::dispatch::{parse, parse_error};
use ferrite_commands::tree::{Node, NodeKind, Parser, StringKind, Tree};

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
fn sample(node: &Node, rng: &mut Rng) -> Option<String> {
    let parser = node.parser.as_ref()?;
    if !parser.is_supported() {
        return None;
    }
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

fn ours(tree: &Tree, line: &str) -> Option<String> {
    let results = parse(tree, line);
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
        std::env::var("FERRITE_PARSE_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(4000);
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
        let Some(got) = ours(&trees[*level], line) else {
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
