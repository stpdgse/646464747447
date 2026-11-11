//! Shared by the SNBT tests: the oracle's canonical value form and error text.
#![allow(dead_code)]

use std::fmt::Write as _;

use ferrite_nbt::parse::{ParseError, parse, parse_prefix};
use ferrite_nbt::{Compound, List, Tag};

pub fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for unit in s.encode_utf16() {
        if (0x20..0x7f).contains(&unit) && unit != u16::from(b'"') && unit != u16::from(b'\\') {
            out.push(char::from(unit as u8));
        } else {
            let _ = write!(out, "\\u{unit:04x}");
        }
    }
    out.push('"');
    out
}

/// The oracle's canonical form (`SnbtOracle.canon`).
pub fn canon(t: &Tag) -> String {
    match t {
        Tag::Byte(v) => format!("B{v}"),
        Tag::Short(v) => format!("S{v}"),
        Tag::Int(v) => format!("I{v}"),
        Tag::Long(v) => format!("L{v}"),
        Tag::Float(v) => format!("F{:x}", v.to_bits()),
        Tag::Double(v) => format!("D{:x}", v.to_bits()),
        Tag::String(s) => quote(s),
        Tag::ByteArray(v) => format!("[B;{}]", join(v)),
        Tag::IntArray(v) => format!("[I;{}]", join(v)),
        Tag::LongArray(v) => format!("[L;{}]", join(v)),
        Tag::List(l) => format!("[{}]", list_items(l).join(",")),
        Tag::Compound(c) => compound(c),
    }
}

pub fn join<T: ToString>(v: &[T]) -> String {
    v.iter().map(T::to_string).collect::<Vec<_>>().join(",")
}

pub fn compound(c: &Compound) -> String {
    let mut keys: Vec<&String> = c.0.keys().collect();
    // Java sorts by UTF-16 code units.
    keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    let parts: Vec<String> =
        keys.iter().map(|k| format!("{}:{}", quote(k), canon(&c.0[*k]))).collect();
    format!("{{{}}}", parts.join(","))
}

pub fn list_items(l: &List) -> Vec<String> {
    match l {
        List::Empty => vec![],
        List::Byte(v) => v.iter().map(|x| canon(&Tag::Byte(*x))).collect(),
        List::Short(v) => v.iter().map(|x| canon(&Tag::Short(*x))).collect(),
        List::Int(v) => v.iter().map(|x| canon(&Tag::Int(*x))).collect(),
        List::Long(v) => v.iter().map(|x| canon(&Tag::Long(*x))).collect(),
        List::Float(v) => v.iter().map(|x| canon(&Tag::Float(*x))).collect(),
        List::Double(v) => v.iter().map(|x| canon(&Tag::Double(*x))).collect(),
        List::ByteArray(v) => v.iter().map(|x| canon(&Tag::ByteArray(x.clone()))).collect(),
        List::String(v) => v.iter().map(|x| quote(x)).collect(),
        List::List(v) => v.iter().map(|x| canon(&Tag::List(x.clone()))).collect(),
        List::Compound(v) => v.iter().map(compound).collect(),
        List::IntArray(v) => v.iter().map(|x| canon(&Tag::IntArray(x.clone()))).collect(),
        List::LongArray(v) => v.iter().map(|x| canon(&Tag::LongArray(x.clone()))).collect(),
        List::Mixed(v) => v.iter().map(canon).collect(),
    }
}

pub fn error_text(e: &ParseError) -> String {
    let mut s = format!("ERR {} {}", e.cursor, e.key);
    for a in &e.args {
        s.push('\u{1}');
        s.push_str(&a.replace('\n', "\\n"));
    }
    s
}

pub fn expected_lines(input: &str) -> [String; 2] {
    let whole = match parse(input) {
        Ok(t) => format!("OK {}", canon(&t)),
        Err(e) => error_text(&e),
    };
    let prefix = match parse_prefix(input) {
        Ok((t, end)) => format!("ARG {end} {}", canon(&t)),
        Err(e) => error_text(&e),
    };
    [whole, prefix]
}

pub fn unescape(s: &str) -> String {
    // What the oracle will see (`\n` -> newline, `\t` -> tab, `\x` -> `x`).
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(o) => out.push(o),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}
