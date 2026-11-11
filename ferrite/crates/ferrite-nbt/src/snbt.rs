//! A plain textual form of tags (the string notation commands and `data get` use).
//!
//! This writer is deterministic rather than pretty: strings are always double-quoted, keys are
//! quoted when they are not simple words, and compounds can be written with sorted keys so two
//! tags that differ only in key order print the same.

use std::fmt::Write;

use crate::tag::{Compound, List, Tag};

fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn key(s: &str, out: &mut String) {
    if !s.is_empty()
        && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+'))
    {
        out.push_str(s);
    } else {
        quote(s, out);
    }
}

fn float(v: f64, suffix: char, out: &mut String) {
    let _ = write!(out, "{v:?}{suffix}");
}

fn compound(c: &Compound, sorted: bool, out: &mut String) {
    out.push('{');
    let mut entries: Vec<(&String, &Tag)> = c.0.iter().collect();
    if sorted {
        entries.sort_by(|a, b| a.0.cmp(b.0));
    }
    for (i, (k, v)) in entries.into_iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        key(k, out);
        out.push(':');
        write_tag(v, sorted, out);
    }
    out.push('}');
}

fn list(l: &List, sorted: bool, out: &mut String) {
    fn items<T>(v: &[T], out: &mut String, mut f: impl FnMut(&T, &mut String)) {
        out.push('[');
        for (i, x) in v.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            f(x, out);
        }
        out.push(']');
    }
    match l {
        List::Empty => out.push_str("[]"),
        List::Byte(v) => items(v, out, |x, o| {
            let _ = write!(o, "{x}b");
        }),
        List::Short(v) => items(v, out, |x, o| {
            let _ = write!(o, "{x}s");
        }),
        List::Int(v) => items(v, out, |x, o| {
            let _ = write!(o, "{x}");
        }),
        List::Long(v) => items(v, out, |x, o| {
            let _ = write!(o, "{x}L");
        }),
        List::Float(v) => items(v, out, |x, o| float(f64::from(*x), 'f', o)),
        List::Double(v) => items(v, out, |x, o| float(*x, 'd', o)),
        List::ByteArray(v) => {
            items(v, out, |x, o| write_tag(&Tag::ByteArray(x.clone()), sorted, o))
        }
        List::String(v) => items(v, out, |x, o| quote(x, o)),
        List::List(v) => items(v, out, |x, o| list(x, sorted, o)),
        List::Compound(v) => items(v, out, |x, o| compound(x, sorted, o)),
        List::IntArray(v) => items(v, out, |x, o| write_tag(&Tag::IntArray(x.clone()), sorted, o)),
        List::LongArray(v) => {
            items(v, out, |x, o| write_tag(&Tag::LongArray(x.clone()), sorted, o))
        }
    }
}

fn write_tag(t: &Tag, sorted: bool, out: &mut String) {
    match t {
        Tag::Byte(v) => {
            let _ = write!(out, "{v}b");
        }
        Tag::Short(v) => {
            let _ = write!(out, "{v}s");
        }
        Tag::Int(v) => {
            let _ = write!(out, "{v}");
        }
        Tag::Long(v) => {
            let _ = write!(out, "{v}L");
        }
        Tag::Float(v) => float(f64::from(*v), 'f', out),
        Tag::Double(v) => float(*v, 'd', out),
        Tag::ByteArray(v) => {
            out.push_str("[B;");
            for (i, x) in v.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{x}b");
            }
            out.push(']');
        }
        Tag::String(s) => quote(s, out),
        Tag::List(l) => list(l, sorted, out),
        Tag::Compound(c) => compound(c, sorted, out),
        Tag::IntArray(v) => {
            out.push_str("[I;");
            for (i, x) in v.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{x}");
            }
            out.push(']');
        }
        Tag::LongArray(v) => {
            out.push_str("[L;");
            for (i, x) in v.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                let _ = write!(out, "{x}L");
            }
            out.push(']');
        }
    }
}

/// The tag as text, compounds in the order they hold their keys.
pub fn to_snbt(t: &Tag) -> String {
    let mut out = String::new();
    write_tag(t, false, &mut out);
    out
}

/// The tag as text with every compound's keys sorted (for comparing tags that may differ only
/// in key order).
pub fn to_sorted_snbt(t: &Tag) -> String {
    let mut out = String::new();
    write_tag(t, true, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_every_kind_of_tag() {
        let mut c = Compound::default();
        c.0.insert("translate".into(), Tag::String("commands.x".into()));
        c.0.insert("with".into(), Tag::List(List::Int(vec![1, 2])));
        c.0.insert("b".into(), Tag::Byte(1));
        c.0.insert("f".into(), Tag::Float(1.5));
        c.0.insert("a b".into(), Tag::Long(7));
        let t = Tag::Compound(c);
        assert_eq!(to_snbt(&t), r#"{translate:"commands.x",with:[1,2],b:1b,f:1.5f,"a b":7L}"#);
        assert_eq!(
            to_sorted_snbt(&t),
            r#"{"a b":7L,b:1b,f:1.5f,translate:"commands.x",with:[1,2]}"#
        );
    }
}
