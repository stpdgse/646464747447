//! Differential test: Ferrite's SNBT parser against the official one (`java/SnbtOracle.java`).
//!
//! Run with `scripts/vanilla-snbt.sh <unpacked bundler dir> [cases]`. Every generated line is
//! parsed by both; values must be identical (compared type-exactly, floats bit for bit) and so
//! must failures (cursor, message key and arguments), for the whole-string and the
//! prefix-of-a-command form alike.

mod common;

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;

use common::{expected_lines, unescape};

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
    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

const NUMBERS: &[&str] = &[
    "0",
    "1",
    "-1",
    "+1",
    "7",
    "127",
    "128",
    "255",
    "256",
    "-128",
    "-129",
    "32767",
    "32768",
    "65535",
    "65536",
    "2147483647",
    "2147483648",
    "4294967295",
    "4294967296",
    "-2147483648",
    "9223372036854775807",
    "9223372036854775808",
    "18446744073709551615",
    "18446744073709551616",
    "0x0",
    "0xF",
    "0xff",
    "0x7F",
    "0x80",
    "0xFFFF",
    "0x10000",
    "0xFFFFFFFF",
    "0x100000000",
    "0xFFFFFFFFFFFFFFFF",
    "0x10000000000000000",
    "-0x1",
    "0b0",
    "0b1",
    "0b101",
    "0b11111111",
    "0b100000000",
    "1_000",
    "1__0",
    "0_1",
    "1_",
    "0x1_F",
    "0b1_0",
    "00",
    "01",
    "1.5",
    "1.",
    ".5",
    "-.5",
    "1e3",
    "1E-3",
    "1.5e+2",
    "1e",
    "1.5e",
    "1_0.5",
    "1._5",
    "0.1",
    "3.4028235e38",
    "3.4028236e38",
    "1e308",
    "1e309",
    "1e-45",
    "1e-50",
    "-0.0",
    "0.0",
];
const SUFFIXES: &[&str] = &[
    "", "", "", "b", "B", "s", "S", "i", "I", "l", "L", "f", "F", "d", "D", "u", "U", "ub", "us",
    "ui", "ul", "sb", "ss", "si", "sl", "uB", "Sl",
];
const WORDS: &[&str] = &[
    "a", "abc", "true", "false", "True", "FALSE", "tru", "truex", "_", "_a", "a.b", "a-b", "a+b",
    "a_b", "a1", "x", "B", "I", "L", "b", "NaN", "Infinity", "bool", "uuid", "nope", "é", "a é",
    "-a", "+a", ".a", "1a", "1_a",
];
const WS: &[&str] = &["", "", "", "", " ", " ", "  ", "\\n", "\\t"];

fn number(rng: &mut Rng) -> String {
    format!("{}{}", rng.pick(NUMBERS), rng.pick(SUFFIXES))
}

fn string(rng: &mut Rng) -> String {
    let q = if rng.chance(70) { '"' } else { '\'' };
    let mut s = String::new();
    s.push(q);
    for _ in 0..rng.below(5) {
        match rng.below(14) {
            0 => s.push_str("\\n"),
            1 => s.push_str("\\\\\\\\"),
            2 => s.push_str("\\\\\""),
            3 => s.push_str("\\\\'"),
            4 => s.push_str("\\\\x41"),
            5 => s.push_str("\\\\u00e9"),
            6 => s.push_str("\\\\ud83d\\\\ude00"),
            7 => s.push_str("\\\\U0001F600"),
            8 => s.push_str("\\\\N{LATIN SMALL LETTER A}"),
            9 => s.push_str(rng.pick(&[
                "\\\\q",
                "\\\\x4",
                "\\\\u12",
                "\\\\N{",
                "\\\\N{nope}",
                "\\\\0",
            ])),
            10 => s.push('"'),
            11 => s.push('\''),
            _ => s.push_str(rng.pick(&["a", "b c", "é", "😀", "{}", "[1]"])),
        }
    }
    if rng.chance(95) {
        s.push(q);
    }
    s
}

fn value(rng: &mut Rng, depth: usize) -> String {
    let ws = |rng: &mut Rng| rng.pick(WS).to_string();
    match rng.below(if depth > 3 { 5 } else { 11 }) {
        0..=2 => number(rng),
        3 => string(rng),
        4 => rng.pick(WORDS).to_string(),
        5 | 6 => {
            let mut out = String::from("{");
            for i in 0..rng.below(4) {
                if i > 0 {
                    out.push(',');
                }
                let key = match rng.below(5) {
                    0 => string(rng),
                    1 => rng.pick(&["a", "b", "1", "x.y", "é", "", "a b"]).to_string(),
                    _ => rng.pick(&["a", "b", "key", "k1", "A"]).to_string(),
                };
                let _ =
                    write!(out, "{}{key}{}:{}{}", ws(rng), ws(rng), ws(rng), value(rng, depth + 1));
            }
            if rng.chance(15) {
                out.push(',');
            }
            if rng.chance(95) {
                out.push_str(&ws(rng));
                out.push('}');
            }
            out
        }
        7 | 8 => {
            let mut out = String::from("[");
            match rng.below(6) {
                0 => out.push_str("B;"),
                1 => out.push_str("I;"),
                2 => out.push_str("L;"),
                _ => {}
            }
            let typed = out.len() > 1;
            for i in 0..rng.below(5) {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&ws(rng));
                if typed && rng.chance(85) {
                    let _ = write!(out, "{}", number(rng));
                } else {
                    out.push_str(&value(rng, depth + 1));
                }
            }
            if rng.chance(10) {
                out.push(',');
            }
            if rng.chance(95) {
                out.push_str(&ws(rng));
                out.push(']');
            }
            out
        }
        9 => {
            let name = rng.pick(&["bool", "bool", "uuid", "x", "BOOL", "a.b"]);
            let mut out = format!("{name}{}(", ws(rng));
            let n = if rng.chance(70) { 1 } else { rng.below(3) };
            for i in 0..n {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&ws(rng));
                if name == "uuid" && rng.chance(60) {
                    out.push_str(rng.pick(&[
                        "\"00000000-0000-0000-0000-000000000001\"",
                        "\"0-0-0-0-1\"",
                        "\"ffffffff-ffff-ffff-ffff-ffffffffffff\"",
                        "\"a\"",
                        "\"+1-2-3-4-5\"",
                        "'1-2-3-4-5'",
                    ]));
                } else {
                    out.push_str(&value(rng, depth + 1));
                }
            }
            if rng.chance(92) {
                out.push_str(&ws(rng));
                out.push(')');
            }
            out
        }
        _ => format!("{}{}", rng.pick(&["-", "+", ".", "0x", "0b", ""]), rng.pick(NUMBERS)),
    }
}

fn mutate(rng: &mut Rng, s: &str) -> String {
    let mut chars: Vec<char> = s.chars().collect();
    for _ in 0..rng.below(3) {
        if chars.is_empty() {
            break;
        }
        let at = rng.below(chars.len());
        match rng.below(4) {
            0 => {
                chars.remove(at);
            }
            1 => chars.insert(
                at,
                rng.pick(&[
                    ",", ":", "{", "}", "[", "]", "(", ")", "\"", "'", "-", ".", " ", "_", "x",
                    "b", "e", ";", "\\\\",
                ])
                .chars()
                .next()
                .unwrap(),
            ),
            2 => chars.truncate(at),
            _ => {
                let c = rng.pick(&["0", "1", "a", " ", "é"]).chars().next().unwrap();
                chars[at] = c;
            }
        }
    }
    chars.into_iter().collect()
}

/// Escapes a line the way `SnbtOracle.unescape` reads it: `\\` is a backslash, `\n` a newline.
/// The generators above already emit pairs for those, so only real newlines remain to escape.
fn generate(rng: &mut Rng) -> String {
    let mut line = value(rng, 0);
    if rng.chance(40) {
        line = mutate(rng, &line);
    }
    if rng.chance(15) {
        line.push_str(rng.pick(&["x", " x", ",", " ", "}", "(1)", "b"]));
    }
    if rng.chance(20) {
        line = format!("{}{line}", rng.pick(WS));
    }
    line
}

fn run_oracle(input: &str) -> Vec<String> {
    let env = |k: &str| std::env::var(k).ok();
    let classpath = env("FERRITE_VANILLA_CLASSPATH").expect(
        "set FERRITE_VANILLA_CLASSPATH to the server jar and its libraries (scripts/vanilla-snbt.sh)",
    );
    let java = env("FERRITE_JAVA").unwrap_or_else(|| "java".into());
    let javac = env("FERRITE_JAVAC").unwrap_or_else(|| "javac".into());
    let dir = std::env::temp_dir().join(format!("ferrite-snbt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (input_path, output_path) = (dir.join("in.txt"), dir.join("out.txt"));
    std::fs::write(&input_path, input).unwrap();
    let source: PathBuf = [env!("CARGO_MANIFEST_DIR"), "java", "SnbtOracle.java"].iter().collect();
    let status = Command::new(&javac)
        .args(["-nowarn", "-cp", &classpath, "-d"])
        .arg(&dir)
        .arg(&source)
        .status()
        .expect("javac");
    assert!(status.success(), "javac failed");
    let out = Command::new(&java)
        .args(["-cp", &format!("{}:{classpath}", dir.display()), "SnbtOracle"])
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
#[ignore = "needs the official server jar (scripts/vanilla-snbt.sh)"]
fn snbt_agrees_with_the_official_parser() {
    let count: usize =
        std::env::var("FERRITE_SNBT_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(20000);
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    let mut lines: Vec<String> = Vec::new();
    while lines.len() < count {
        let l = generate(&mut rng);
        // The oracle reads line by line; real newlines would split a case.
        if !l.contains('\n') && !l.contains('\r') && !l.is_empty() {
            lines.push(l);
        }
    }
    let input = lines.join("\n") + "\n";
    let got = run_oracle(&input);
    assert_eq!(got.len(), lines.len() * 2, "the oracle answered a different number of lines");
    let mut bad = 0;
    for (i, line) in lines.iter().enumerate() {
        let actual = unescape(line);
        let want = expected_lines(&actual);
        for k in 0..2 {
            if want[k] != got[2 * i + k] {
                bad += 1;
                if bad <= 40 {
                    println!(
                        "{} {:?}\n    vanilla {}\n    ferrite {}",
                        if k == 0 { "whole " } else { "prefix" },
                        actual,
                        got[2 * i + k].replace('\u{1}', "|"),
                        want[k].replace('\u{1}', "|"),
                    );
                }
            }
        }
    }
    println!("{} lines compared ({} answers)", lines.len(), lines.len() * 2);
    if let Ok(path) = std::env::var("FERRITE_SNBT_RECORD") {
        // Keep the first few thousand cases so CI can replay them without the server jar.
        let mut out = String::new();
        for (i, line) in lines.iter().enumerate().take(4000) {
            let _ = writeln!(out, "{line}\n{}\n{}", got[2 * i], got[2 * i + 1]);
        }
        std::fs::write(path, out).unwrap();
    }
    assert_eq!(bad, 0, "{bad} answers differ");
}
