//! SNBT, the text form of NBT, parsed the way the vanilla server does.
//!
//! The grammar and its error reporting were established by probing the official parser (see
//! `java/SnbtOracle.java` and `tests/vanilla_snbt.rs`): whitespace is allowed between every pair of
//! tokens (even `1 .5` or `- 1`), numbers take `_` separators, `0x`/`0b` radixes, `u`/`s`
//! signedness markers and `b`/`s`/`i`/`l`/`f`/`d` type suffixes, strings take `\x`, `\u`, `\U` and
//! `\N{NAME}` escapes, lists may mix types, and `bool(..)`/`uuid(..)` are built-in operations.
//!
//! On failure the error is the one that got **furthest** into the text, the first one recorded when
//! several share that position - which is how the packrat parser vanilla uses reports it.

use crate::{Compound, List, Tag};

/// Why parsing failed, as a translatable message plus the position (in UTF-16 units, like
/// Brigadier's string reader) it points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub cursor: usize,
    pub key: &'static str,
    pub args: Vec<String>,
}

/// Parses `input` as one SNBT value with nothing but whitespace after it (`parseFully`).
pub fn parse(input: &str) -> Result<Tag, ParseError> {
    let (tag, _, trailing) = parse_prefix_inner(input)?;
    match trailing {
        Some(at) => Err(ParseError { cursor: at, key: "argument.nbt.trailing", args: vec![] }),
        None => Ok(tag),
    }
}

/// Parses one SNBT value at the start of `input` and returns it with how many UTF-16 units it
/// used (`parseAsArgument`).
pub fn parse_prefix(input: &str) -> Result<(Tag, usize), ParseError> {
    let (tag, end, _) = parse_prefix_inner(input)?;
    Ok((tag, end))
}

fn parse_prefix_inner(input: &str) -> Result<(Tag, usize, Option<usize>), ParseError> {
    let mut p = Parser::new(input);
    match p.value(0) {
        Some((tag, end)) => {
            let after = p.ws(end);
            let trailing = (after < p.chars.len()).then(|| p.utf16(after));
            Ok((tag, p.utf16(end), trailing))
        }
        None => Err(p.take_error()),
    }
}

const NAME_EXTRA: &[char] = &['_', '-', '.', '+'];

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || NAME_EXTRA.contains(&c)
}

/// `Character.isWhitespace`.
fn is_ws(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r' | '\u{1c}'..='\u{1f}' | ' ')
        || (c as u32 > 127
            && !matches!(c, '\u{a0}' | '\u{2007}' | '\u{202f}')
            && matches!(
                c,
                '\u{1680}' | '\u{2000}'
                    ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{205f}' | '\u{3000}'
            ))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ty {
    Byte,
    Short,
    Int,
    Long,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Marker {
    None,
    Signed,
    Unsigned,
}

struct Parser {
    chars: Vec<char>,
    /// `utf16_at[i]` is the UTF-16 offset of char `i` (with one extra entry for the end).
    utf16_at: Vec<usize>,
    best_pos: isize,
    best: Option<ParseError>,
}

type Res<T> = Option<T>;

impl Parser {
    fn new(input: &str) -> Parser {
        let chars: Vec<char> = input.chars().collect();
        let mut utf16_at = Vec::with_capacity(chars.len() + 1);
        let mut at = 0;
        for c in &chars {
            utf16_at.push(at);
            at += c.len_utf16();
        }
        utf16_at.push(at);
        Parser { chars, utf16_at, best_pos: -1, best: None }
    }

    fn utf16(&self, i: usize) -> usize {
        self.utf16_at[i.min(self.chars.len())]
    }

    fn at(&self, i: usize) -> Option<char> {
        self.chars.get(i).copied()
    }

    fn ws(&self, mut i: usize) -> usize {
        while self.at(i).is_some_and(is_ws) {
            i += 1;
        }
        i
    }

    fn take_error(&mut self) -> ParseError {
        self.best.take().unwrap_or(ParseError {
            cursor: 0,
            key: "snbt.parser.expected_unquoted_string",
            args: vec![],
        })
    }

    /// Records a failure at char index `at`; only the furthest (first of equals) survives.
    fn note(&mut self, at: usize, key: &'static str, args: Vec<String>) {
        if at as isize > self.best_pos {
            self.best_pos = at as isize;
            self.best = Some(ParseError { cursor: self.utf16(at), key, args });
        }
    }

    fn note_lit(&mut self, at: usize, lit: &str) {
        self.note(at, "argument.literal.incorrect", vec![lit.to_string()]);
    }

    /// A single-character token after optional whitespace; records `display` when absent.
    fn lit(&mut self, i: usize, c: char, display: &str) -> Res<usize> {
        let j = self.ws(i);
        if self.at(j) == Some(c) {
            Some(j + 1)
        } else {
            self.note_lit(j, display);
            None
        }
    }

    // ---- values ----------------------------------------------------------------------------

    fn value(&mut self, i: usize) -> Res<(Tag, usize)> {
        let p = self.ws(i);
        let mut number_failed_hard = false;
        if self.at(p).is_some_and(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.')) {
            match self.number(p) {
                NumberResult::Ok(t, e) => return Some((t, e)),
                NumberResult::Soft => {}
                NumberResult::Hard => number_failed_hard = true,
                NumberResult::Stop => return None,
            }
        }
        match self.call(p) {
            Call::Done(t, e) => return Some((t, e)),
            Call::Failed => return None,
            Call::Plain => {}
        }
        if number_failed_hard {
            return None;
        }
        match self.at(p) {
            Some(q @ ('"' | '\'')) => {
                let (s, e) = self.quoted(p, q)?;
                Some((Tag::String(s), e))
            }
            Some('{') => self.compound(p),
            Some('[') => self.list_or_array(p),
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                let mut e = p;
                while self.at(e).is_some_and(is_name_char) {
                    e += 1;
                }
                let word: String = self.chars[p..e].iter().collect();
                let tag = match word.to_ascii_lowercase().as_str() {
                    "true" => Tag::Byte(1),
                    "false" => Tag::Byte(0),
                    _ => Tag::String(word),
                };
                Some((tag, e))
            }
            _ => None,
        }
    }

    // ---- function calls: bool(..), uuid(..) --------------------------------------------------

    fn call(&mut self, p: usize) -> Call {
        let mut e = p;
        while self.at(e).is_some_and(is_name_char) {
            e += 1;
        }
        if e == p {
            self.note(p, "snbt.parser.expected_unquoted_string", vec![]);
            return Call::Plain;
        }
        let q = self.ws(e);
        if self.at(q) != Some('(') {
            self.note_lit(q, "(");
            return Call::Plain;
        }
        let name: String = self.chars[p..e].iter().collect();
        // From here on the call is committed: a failure inside it is the value's failure.
        let mut args: Vec<Tag> = Vec::new();
        let mut j = q + 1;
        let mut closed = None;
        loop {
            let k = self.ws(j);
            if self.at(k) == Some(')') {
                closed = Some(k + 1);
                break;
            }
            let Some((t, end)) = self.value(k) else {
                return self.partial_builtin(&name, &args, e);
            };
            args.push(t);
            let after = self.ws(end);
            if self.at(after) == Some(',') {
                j = after + 1;
                continue;
            }
            if self.at(after) == Some(')') {
                closed = Some(after + 1);
                break;
            }
            self.note_lit(after, ",");
            self.note_lit(after, ")");
            break;
        }
        let Some(end) = closed else {
            return self.partial_builtin(&name, &args, e);
        };
        if !name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            self.note(end, "snbt.parser.invalid_unquoted_start", vec![]);
            return Call::Failed;
        }
        match builtin(&name, &args) {
            Ok(t) => Call::Done(t, end),
            Err((key, a)) => {
                self.note(end, key, a);
                Call::Failed
            }
        }
    }

    /// A built-in call that went wrong after its first, acceptable argument still yields that
    /// value, positioned just after the name (vanilla's parser does exactly this).
    fn partial_builtin(&self, name: &str, args: &[Tag], name_end: usize) -> Call {
        if args.len() == 1
            && matches!(name, "bool" | "uuid")
            && let Ok(t) = builtin(name, args)
        {
            return Call::Done(t, name_end);
        }
        Call::Failed
    }

    // ---- strings -----------------------------------------------------------------------------

    fn quoted(&mut self, p: usize, quote: char) -> Res<(String, usize)> {
        let mut units: Vec<u16> = Vec::new();
        let mut i = p + 1;
        loop {
            let Some(c) = self.at(i) else {
                self.note(i, "snbt.parser.invalid_string_contents", vec![]);
                return None;
            };
            if c == quote {
                return Some((String::from_utf16_lossy(&units), i + 1));
            }
            if c != '\\' {
                let mut buf = [0u16; 2];
                units.extend_from_slice(c.encode_utf16(&mut buf));
                i += 1;
                continue;
            }
            i = self.ws(i + 1);
            let Some(e) = self.at(i) else {
                self.note_lit(i, "b");
                return None;
            };
            let simple = match e {
                'b' => Some('\u{8}'),
                's' => Some(' '),
                't' => Some('\t'),
                'n' => Some('\n'),
                'f' => Some('\u{c}'),
                'r' => Some('\r'),
                '\\' => Some('\\'),
                '\'' => Some('\''),
                '"' => Some('"'),
                _ => None,
            };
            if let Some(ch) = simple {
                units.push(ch as u16);
                i += 1;
                continue;
            }
            match e {
                'x' | 'u' | 'U' => {
                    let n = match e {
                        'x' => 2,
                        'u' => 4,
                        _ => 8,
                    };
                    let start = i + 1;
                    let digits = &self.chars[start.min(self.chars.len())..];
                    let ok = digits.len() >= n && digits[..n].iter().all(char::is_ascii_hexdigit);
                    if !ok {
                        self.note(start, "snbt.parser.expected_hex_escape", vec![n.to_string()]);
                        return None;
                    }
                    let text: String = digits[..n].iter().collect();
                    let code = u32::from_str_radix(&text, 16).unwrap_or(u32::MAX);
                    if code > 0x10_ffff {
                        self.note(
                            start + n,
                            "snbt.parser.invalid_codepoint",
                            vec![format!("U+{code:08X}")],
                        );
                        return None;
                    }
                    if code <= 0xffff {
                        units.push(code as u16);
                    } else {
                        let c = code - 0x1_0000;
                        units.push(0xd800 + (c >> 10) as u16);
                        units.push(0xdc00 + (c & 0x3ff) as u16);
                    }
                    i = start + n;
                }
                'N' => {
                    let open = self.ws(i + 1);
                    if self.at(open) != Some('{') {
                        self.note_lit(open, "{");
                        return None;
                    }
                    let mut j = open + 1;
                    while self
                        .at(j)
                        .is_some_and(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-')
                    {
                        j += 1;
                    }
                    let name: String = self.chars[open + 1..j].iter().collect();
                    if j == open + 1 {
                        self.note(j, "snbt.parser.invalid_character_name", vec![]);
                        return None;
                    }
                    let j = {
                        let close = self.ws(j);
                        if self.at(close) != Some('}') {
                            self.note_lit(close, "}");
                            return None;
                        }
                        close
                    };
                    match character_by_name(&name) {
                        Some(ch) => {
                            let mut buf = [0u16; 2];
                            units.extend_from_slice(ch.encode_utf16(&mut buf));
                            i = j + 1;
                        }
                        None => {
                            self.note(j + 1, "snbt.parser.invalid_character_name", vec![]);
                            return None;
                        }
                    }
                }
                _ => {
                    self.note_lit(i, "b");
                    return None;
                }
            }
        }
    }

    // ---- compounds ---------------------------------------------------------------------------

    fn key(&mut self, i: usize) -> Res<(String, usize)> {
        let p = self.ws(i);
        match self.at(p) {
            Some(q @ ('"' | '\'')) => self.quoted(p, q),
            _ => {
                self.note_lit(p, "\"");
                let mut e = p;
                while self.at(e).is_some_and(is_name_char) {
                    e += 1;
                }
                if e == p {
                    self.note(p, "snbt.parser.expected_unquoted_string", vec![]);
                    return None;
                }
                Some((self.chars[p..e].iter().collect(), e))
            }
        }
    }

    fn compound(&mut self, p: usize) -> Res<(Tag, usize)> {
        let mut map = Compound::new();
        let mut i = p + 1;
        loop {
            let j = self.ws(i);
            if self.at(j) == Some('}') {
                return Some((Tag::Compound(map), j + 1));
            }
            let (key, after_key) = self.key(j)?;
            let colon = self.lit(after_key, ':', ":")?;
            let (value, end) = self.value(colon)?;
            if key.is_empty() {
                self.note(end, "snbt.parser.empty_key", vec![]);
                return None;
            }
            map.insert(key, value);
            let k = self.ws(end);
            match self.at(k) {
                Some(',') => i = k + 1,
                Some('}') => return Some((Tag::Compound(map), k + 1)),
                _ => {
                    self.note_lit(k, ",");
                    self.note_lit(k, "}");
                    return None;
                }
            }
        }
    }

    // ---- lists and arrays ----------------------------------------------------------------------

    fn list_or_array(&mut self, p: usize) -> Res<(Tag, usize)> {
        let start = self.ws(p + 1);
        if self.at(start) == Some(']') {
            return Some((Tag::List(List::Empty), start + 1));
        }
        for (letter, kind) in [('B', Ty::Byte), ('I', Ty::Int), ('L', Ty::Long)] {
            if self.at(start) == Some(letter) {
                let semi = self.ws(start + 1);
                if self.at(semi) == Some(';') {
                    return self.array(semi + 1, kind);
                }
                self.note_lit(semi, ";");
            } else {
                self.note_lit(start, &letter.to_string());
            }
        }
        let mut items: Vec<Tag> = Vec::new();
        let mut i = start;
        loop {
            let j = self.ws(i);
            if self.at(j) == Some(']') {
                return Some((Tag::List(build_list(items)), j + 1));
            }
            let (v, end) = self.value(j)?;
            items.push(v);
            let k = self.ws(end);
            match self.at(k) {
                Some(',') => i = k + 1,
                Some(']') => return Some((Tag::List(build_list(items)), k + 1)),
                _ => {
                    self.note_lit(k, ",");
                    self.note_lit(k, "]");
                    return None;
                }
            }
        }
    }

    fn array(&mut self, mut i: usize, kind: Ty) -> Res<(Tag, usize)> {
        let mut raws: Vec<RawInt> = Vec::new();
        loop {
            let j = self.ws(i);
            if self.at(j) == Some(']') {
                let end = j + 1;
                return self.finish_array(kind, raws, end);
            }
            let raw = match self.int_syntax(j, true) {
                Syntax::Ok(raw) => raw,
                _ => return None,
            };
            let k = self.ws(raw.end);
            raws.push(raw);
            match self.at(k) {
                Some(',') => i = k + 1,
                Some(']') => i = k,
                _ => {
                    self.note_lit(k, ",");
                    self.note_lit(k, "]");
                    return None;
                }
            }
        }
    }

    /// Typed arrays check and convert their elements only once the closing bracket is read, so
    /// every problem is reported just after it.
    fn finish_array(&mut self, kind: Ty, raws: Vec<RawInt>, end: usize) -> Res<(Tag, usize)> {
        let mut values: Vec<i64> = Vec::new();
        for raw in &raws {
            let allowed = match (kind, raw.ty) {
                (_, None) => true,
                (Ty::Byte, Some(t)) => t == Ty::Byte,
                (Ty::Int, Some(t)) => t != Ty::Long,
                (_, Some(_)) => true,
            };
            if !allowed {
                self.note(end, "snbt.parser.invalid_array_element_type", vec![]);
                return None;
            }
            let target = raw.ty.unwrap_or(kind);
            match convert(raw.neg, &raw.digits, raw.radix, raw.marker, target) {
                Ok(t) => values.push(t.as_i64().unwrap_or(0)),
                Err(Convert::NonNegative) => {
                    self.note(end, "snbt.parser.expected_non_negative_number", vec![]);
                    return None;
                }
                Err(Convert::Failure(msg)) => {
                    self.note(end, "snbt.parser.number_parse_failure", vec![msg]);
                    return None;
                }
            }
        }
        let tag = match kind {
            Ty::Byte => Tag::ByteArray(values.iter().map(|v| *v as i8).collect()),
            Ty::Int => Tag::IntArray(values.iter().map(|v| *v as i32).collect()),
            _ => Tag::LongArray(values),
        };
        Some((tag, end))
    }

    // ---- numbers -----------------------------------------------------------------------------

    /// Scans `[0-9_]+` (or the hex/binary equivalents) starting exactly at `i`; returns the raw
    /// text, the end and whether it is well formed (no leading or trailing `_`).
    fn digits(&self, i: usize, radix: u32) -> Option<(String, usize, bool)> {
        let ok = |c: char| c == '_' || c.is_digit(radix);
        let mut e = i;
        while self.at(e).is_some_and(ok) {
            e += 1;
        }
        if e == i {
            return None;
        }
        let raw: String = self.chars[i..e].iter().collect();
        let valid = !raw.starts_with('_') && !raw.ends_with('_');
        Some((raw, e, valid))
    }

    /// Digits were expected at `at` and are absent or malformed.
    fn digits_missing(&mut self, at: usize) {
        self.numeral_missing(at, 10);
    }

    fn numeral_missing(&mut self, at: usize, radix: u32) {
        match self.digits(at, radix) {
            Some((_, _, false)) => self.note(at, "snbt.parser.underscore_not_allowed", vec![]),
            _ => {
                let key = match radix {
                    16 => "snbt.parser.expected_hex_numeral",
                    2 => "snbt.parser.expected_binary_numeral",
                    _ => "snbt.parser.expected_decimal_numeral",
                };
                self.note(at, key, vec![]);
            }
        }
    }

    /// A number at `p`: a float if it reads as one, else an integer.
    fn number(&mut self, p: usize) -> NumberResult {
        if let Some((t, e)) = self.float(p) {
            return NumberResult::Ok(t, e);
        }
        match self.int_syntax(p, false) {
            Syntax::Ok(raw) => {
                let end = raw.end;
                match convert(
                    raw.neg,
                    &raw.digits,
                    raw.radix,
                    raw.marker,
                    raw.ty.unwrap_or(Ty::Int),
                ) {
                    Ok(t) => NumberResult::Ok(t, end),
                    Err(Convert::NonNegative) => {
                        self.note(end, "snbt.parser.expected_non_negative_number", vec![]);
                        NumberResult::Stop
                    }
                    Err(Convert::Failure(msg)) => {
                        if raw.ty.is_some() {
                            self.note(end, "snbt.parser.number_parse_failure", vec![msg]);
                            NumberResult::Stop
                        } else {
                            // An untyped integer that does not fit: no error of its own, but
                            // nothing else may be read here either.
                            NumberResult::Stop
                        }
                    }
                }
            }
            Syntax::Soft => NumberResult::Soft,
            Syntax::Hard => NumberResult::Hard,
            Syntax::Stop => NumberResult::Stop,
        }
    }

    /// `sign? (int . frac? exp? | . frac exp? | int exp) [fFdD]?` with whitespace between parts.
    fn float(&mut self, p: usize) -> Res<(Tag, usize)> {
        let mut i = p;
        let mut text = String::new();
        match self.at(i) {
            Some('-') => {
                text.push('-');
                i += 1;
            }
            Some('+') => i += 1,
            _ => {}
        }
        let signed = i > p;
        let int_at = self.ws(i);
        let int = self.digits(int_at, 10);
        let mut has_frac_or_exp = false;
        let mut int_text = String::new();
        let mut frac_text = String::new();
        let mut exp_text = String::new();
        let mut end;
        match int {
            Some((raw, e, true)) => {
                int_text = raw.replace('_', "");
                end = e;
            }
            Some(_) => {
                self.note(int_at, "snbt.parser.underscore_not_allowed", vec![]);
                return None;
            }
            None => {
                if signed {
                    self.note(int_at, "snbt.parser.expected_decimal_numeral", vec![]);
                }
                end = int_at;
                // Must be `.frac`.
                let dot = self.ws(end);
                if self.at(dot) != Some('.') {
                    if !signed {
                        self.note_lit(dot, ".");
                    }
                    return None;
                }
                let f_at = self.ws(dot + 1);
                match self.digits(f_at, 10) {
                    Some((raw, e, true)) => {
                        frac_text = raw.replace('_', "");
                        end = e;
                        has_frac_or_exp = true;
                    }
                    _ => {
                        self.digits_missing(f_at);
                        return None;
                    }
                }
            }
        }
        if !has_frac_or_exp {
            let dot = self.ws(end);
            if self.at(dot) == Some('.') {
                has_frac_or_exp = true;
                end = dot + 1;
                let f_at = self.ws(end);
                if let Some((raw, e, true)) = self.digits(f_at, 10) {
                    frac_text = raw.replace('_', "");
                    end = e;
                } else {
                    self.digits_missing(f_at);
                }
            } else {
                self.note_lit(dot, ".");
            }
        }
        // Exponent.
        let e_at = self.ws(end);
        if matches!(self.at(e_at), Some('e' | 'E')) {
            let mut k = self.ws(e_at + 1);
            let mut sign = String::new();
            if matches!(self.at(k), Some('+' | '-')) {
                sign.push(self.chars[k]);
                k = self.ws(k + 1);
            } else {
                self.note_lit(k, "+");
            }
            if let Some((raw, e, true)) = self.digits(k, 10) {
                exp_text = format!("{sign}{}", raw.replace('_', ""));
                end = e;
                has_frac_or_exp = true;
            } else {
                self.digits_missing(k);
            }
        } else {
            self.note_lit(e_at, "e|E");
        }
        // Suffix.
        let s_at = self.ws(end);
        let mut is_float = false;
        let mut has_suffix = false;
        match self.at(s_at) {
            Some('f' | 'F') => {
                is_float = true;
                has_suffix = true;
                end = s_at + 1;
            }
            Some('d' | 'D') => {
                has_suffix = true;
                end = s_at + 1;
            }
            _ => {
                self.note_lit(s_at, "f|F");
                self.note_lit(s_at, "d|D");
            }
        }
        if !has_frac_or_exp && !has_suffix {
            return None;
        }
        let int_part = if int_text.is_empty() { "0" } else { &int_text };
        let frac_part = if frac_text.is_empty() { "0" } else { &frac_text };
        let mut literal = format!("{text}{int_part}.{frac_part}");
        if !exp_text.is_empty() {
            literal.push('e');
            literal.push_str(&exp_text);
        }
        if is_float {
            let v: f32 = literal.parse().ok()?;
            if v.is_infinite() {
                self.note(end, "snbt.parser.infinity_not_allowed", vec![]);
                return None;
            }
            Some((Tag::Float(if v == 0.0 { 0.0 } else { v }), end))
        } else {
            let v: f64 = literal.parse().ok()?;
            if v.is_infinite() {
                self.note(end, "snbt.parser.infinity_not_allowed", vec![]);
                return None;
            }
            Some((Tag::Double(if v == 0.0 { 0.0 } else { v }), end))
        }
    }

    /// The syntax of an integer literal: sign, numeral, then the first of the readings
    /// `marker+type`, `type`, `nothing` that fits.
    fn int_syntax(&mut self, p: usize, array: bool) -> Syntax {
        let mut i = p;
        let mut neg = false;
        match self.at(i) {
            Some('-') => {
                neg = true;
                i += 1;
            }
            Some('+') => i += 1,
            _ => {
                if array {
                    self.note_lit(i, "+");
                }
            }
        }
        let d_at = self.ws(i);
        let mut radix = 10;
        let mut digits_at = d_at;
        if self.at(d_at) == Some('0') {
            let x = self.ws(d_at + 1);
            match self.at(x) {
                Some('x' | 'X') => {
                    let h = self.ws(x + 1);
                    match self.digits(h, 16) {
                        Some((_, _, true)) => {
                            radix = 16;
                            digits_at = h;
                        }
                        _ => {
                            self.numeral_missing(h, 16);
                            return Syntax::Hard;
                        }
                    }
                }
                Some('b' | 'B') => {
                    let h = self.ws(x + 1);
                    if let Some((_, _, true)) = self.digits(h, 2) {
                        radix = 2;
                        digits_at = h;
                    } else {
                        self.numeral_missing(h, 2);
                    }
                }
                _ => self.note_lit(x, "x|X"),
            }
        }
        if radix == 10 && self.at(d_at) != Some('0') {
            self.note_lit(d_at, "0");
        }
        let (digits, end) = if radix == 10 {
            match self.at(d_at) {
                Some('0') => {
                    let after = self.ws(d_at + 1);
                    if self.at(after).is_some_and(|c| c.is_ascii_digit())
                        && let Some((_, end, true)) = self.digits(after, 10)
                    {
                        self.note(end, "snbt.parser.leading_zero_not_allowed", vec![]);
                        return Syntax::Soft;
                    }
                    ("0".to_string(), d_at + 1)
                }
                Some(c) if c.is_ascii_digit() => match self.digits(d_at, 10) {
                    Some((raw, e, true)) => (raw.replace('_', ""), e),
                    _ => return Syntax::Soft,
                },
                _ => {
                    self.note_lit(d_at, "0");
                    return Syntax::Soft;
                }
            }
        } else {
            let (raw, e, _) = self.digits(digits_at, radix).expect("checked above");
            (raw.replace('_', ""), e)
        };
        let m_at = self.ws(end);
        let marker = match self.at(m_at) {
            Some('u' | 'U') => Some(Marker::Unsigned),
            Some('s' | 'S') => Some(Marker::Signed),
            _ => {
                self.note_lit(m_at, "u|U");
                None
            }
        };
        let mut options: Vec<(Marker, Option<Ty>, usize)> = Vec::new();
        if let Some(m) = marker {
            let t_at = self.ws(m_at + 1);
            if let Some(t) = self.type_suffix(t_at) {
                options.push((m, Some(t), t_at + 1));
            } else {
                self.note_lit(t_at, "b|B");
            }
        }
        let t_at = self.ws(end);
        if let Some(t) = self.type_suffix(t_at) {
            options.push((Marker::None, Some(t), t_at + 1));
        } else if marker.is_none() {
            self.note_lit(t_at, "b|B");
        }
        options.push((Marker::None, None, end));
        if array {
            let (marker, ty, e) = options[0];
            return Syntax::Ok(RawInt { neg, digits, radix, marker, ty, end: e });
        }
        // A negative hex or binary number without `s` is silently not a number; so is a
        // negative unsigned one, but only after its type was read (that is a loud error).
        let (marker, ty, e) = options[0];
        if radix != 10 && neg && marker == Marker::None && ty.is_none() {
            return Syntax::Stop;
        }
        Syntax::Ok(RawInt { neg, digits, radix, marker, ty, end: e })
    }

    fn type_suffix(&self, i: usize) -> Option<Ty> {
        match self.at(i)? {
            'b' | 'B' => Some(Ty::Byte),
            's' | 'S' => Some(Ty::Short),
            'i' | 'I' => Some(Ty::Int),
            'l' | 'L' => Some(Ty::Long),
            _ => None,
        }
    }
}

enum NumberResult {
    Ok(Tag, usize),
    /// Not a number here; other readings (a function call, a plain string) may still apply.
    Soft,
    /// A `0x` without hex digits: not a number, and no other number reading is allowed, but a
    /// function call still gets its chance to report how far it got.
    Hard,
    /// A number that cannot be converted: the whole value fails right here.
    Stop,
}

struct RawInt {
    neg: bool,
    digits: String,
    radix: u32,
    marker: Marker,
    ty: Option<Ty>,
    end: usize,
}

enum Syntax {
    Ok(RawInt),
    Soft,
    Hard,
    Stop,
}

enum Call {
    Done(Tag, usize),
    Failed,
    Plain,
}

enum Convert {
    NonNegative,
    Failure(String),
}

fn build_list(items: Vec<Tag>) -> List {
    let Some(first) = items.first() else {
        return List::Empty;
    };
    let id = first.type_id();
    if items.iter().any(|t| t.type_id() != id) {
        return List::Mixed(items);
    }
    macro_rules! gather {
        ($variant:ident, $pat:path) => {
            List::$variant(
                items
                    .into_iter()
                    .map(|t| match t {
                        $pat(v) => v,
                        _ => unreachable!(),
                    })
                    .collect(),
            )
        };
    }
    match first {
        Tag::Byte(_) => gather!(Byte, Tag::Byte),
        Tag::Short(_) => gather!(Short, Tag::Short),
        Tag::Int(_) => gather!(Int, Tag::Int),
        Tag::Long(_) => gather!(Long, Tag::Long),
        Tag::Float(_) => gather!(Float, Tag::Float),
        Tag::Double(_) => gather!(Double, Tag::Double),
        Tag::ByteArray(_) => gather!(ByteArray, Tag::ByteArray),
        Tag::String(_) => gather!(String, Tag::String),
        Tag::IntArray(_) => gather!(IntArray, Tag::IntArray),
        Tag::LongArray(_) => gather!(LongArray, Tag::LongArray),
        Tag::Compound(_) => gather!(Compound, Tag::Compound),
        Tag::List(_) => gather!(List, Tag::List),
    }
}

/// Java's integer-parse failure messages for the four widths.
fn convert(neg: bool, digits: &str, radix: u32, marker: Marker, ty: Ty) -> Result<Tag, Convert> {
    let unsigned = match marker {
        Marker::Unsigned => true,
        Marker::Signed => false,
        Marker::None => radix != 10,
    };
    if unsigned && neg {
        return Err(Convert::NonNegative);
    }
    let value: Option<u128> = u128::from_str_radix(digits, radix).ok();
    if unsigned {
        return match ty {
            Ty::Byte | Ty::Short => {
                // `Integer.parseInt`, then a range check.
                let v = parse_int(digits, radix).map_err(Convert::Failure)?;
                let limit = if ty == Ty::Byte { 0xff } else { 0xffff };
                if v > limit {
                    return Err(Convert::Failure(format!("out of range: {v}")));
                }
                Ok(if ty == Ty::Byte {
                    Tag::Byte(v as u8 as i8)
                } else {
                    Tag::Short(v as u16 as i16)
                })
            }
            Ty::Int => parse_unsigned_int(digits, radix)
                .map(|v| Tag::Int(v as i32))
                .map_err(Convert::Failure),
            Ty::Long => parse_unsigned_long(digits, radix)
                .map(|v| Tag::Long(v as i64))
                .map_err(Convert::Failure),
        };
    }
    let text = if neg { format!("-{digits}") } else { digits.to_string() };
    let signed: Option<i128> = value
        .filter(|v| *v <= i128::MAX as u128)
        .map(|v| if neg { -(v as i128) } else { v as i128 });
    let fail = |ty: Ty| -> Convert {
        Convert::Failure(match ty {
            Ty::Byte | Ty::Short => {
                format!("Value out of range. Value:\"{text}\" Radix:{radix}")
            }
            Ty::Int | Ty::Long => {
                if radix == 10 {
                    format!("For input string: \"{text}\"")
                } else {
                    format!("For input string: \"{text}\" under radix {radix}")
                }
            }
        })
    };
    let Some(v) = signed else {
        return Err(fail(Ty::Int));
    };
    // `Byte.parseByte` and `Short.parseShort` read an int first.
    if matches!(ty, Ty::Byte | Ty::Short) && i32::try_from(v).is_err() {
        return Err(fail(Ty::Int));
    }
    match ty {
        Ty::Byte => i8::try_from(v).map(Tag::Byte).map_err(|_| fail(ty)),
        Ty::Short => i16::try_from(v).map(Tag::Short).map_err(|_| fail(ty)),
        Ty::Int => i32::try_from(v).map(Tag::Int).map_err(|_| fail(ty)),
        Ty::Long => i64::try_from(v).map(Tag::Long).map_err(|_| fail(ty)),
    }
}

type OpError = (&'static str, Vec<String>);

/// The built-in operations: `bool(x)` and `uuid("...")`.
fn builtin(name: &str, args: &[Tag]) -> Result<Tag, OpError> {
    match (name, args) {
        ("bool", [arg]) => {
            let truth = match arg {
                Tag::Byte(v) => *v != 0,
                Tag::Short(v) => *v != 0,
                Tag::Int(v) => *v != 0,
                Tag::Long(v) => *v != 0,
                Tag::Float(v) => *v != 0.0,
                Tag::Double(v) => *v != 0.0,
                _ => return Err(("snbt.parser.expected_number_or_boolean", vec![])),
            };
            Ok(Tag::Byte(i8::from(truth)))
        }
        ("uuid", [arg]) => {
            let Tag::String(s) = arg else {
                return Err(("snbt.parser.expected_string_uuid", vec![]));
            };
            match parse_uuid(s) {
                Some(u) => Ok(Tag::IntArray(vec![
                    (u >> 96) as i32,
                    (u >> 64) as i32,
                    (u >> 32) as i32,
                    u as i32,
                ])),
                None => Err(("snbt.parser.expected_string_uuid", vec![])),
            }
        }
        _ => Err(("snbt.parser.no_such_operation", vec![format!("{name}/{}", args.len())])),
    }
}

/// `UUID.fromString`: five dash-separated hex fields, each taken modulo its width.
fn parse_uuid(s: &str) -> Option<u128> {
    if s.chars().count() > 36 {
        return None;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return None;
    }
    let field = |text: &str| -> Option<u64> {
        let t = text.strip_prefix('+').unwrap_or(text);
        if t.is_empty() || !t.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        // Java's Long.parseLong rejects anything beyond 63 bits of magnitude.
        let v = u128::from_str_radix(t, 16).ok()?;
        (v <= i64::MAX as u128).then_some(v as u64)
    };
    let a = field(parts[0])? & 0xffff_ffff;
    let b = field(parts[1])? & 0xffff;
    let c = field(parts[2])? & 0xffff;
    let d = field(parts[3])? & 0xffff;
    let e = field(parts[4])? & 0xffff_ffff_ffff;
    let most = (a << 32) | (b << 16) | c;
    let least = (d << 48) | e;
    Some((u128::from(most) << 64) | u128::from(least))
}

fn for_input(text: &str, radix: u32) -> String {
    if radix == 10 {
        format!("For input string: \"{text}\"")
    } else {
        format!("For input string: \"{text}\" under radix {radix}")
    }
}

/// `Integer.parseInt` for non-negative digit strings: the value, or Java's message.
fn parse_int(digits: &str, radix: u32) -> Result<i64, String> {
    match u128::from_str_radix(digits, radix) {
        Ok(v) if v <= i32::MAX as u128 => Ok(v as i64),
        _ => Err(for_input(digits, radix)),
    }
}

/// `Integer.parseUnsignedInt`: vanilla reports every overflow the same way.
fn parse_unsigned_int(digits: &str, radix: u32) -> Result<u32, String> {
    match u128::from_str_radix(digits, radix) {
        Ok(v) if v <= u128::from(u32::MAX) => Ok(v as u32),
        _ => Err(format!("String value {digits} exceeds range of unsigned int.")),
    }
}

/// `Long.parseUnsignedLong`.
fn parse_unsigned_long(digits: &str, radix: u32) -> Result<u64, String> {
    match u128::from_str_radix(digits, radix) {
        Ok(v) if v <= u128::from(u64::MAX) => Ok(v as u64),
        _ => Err(format!("String value {digits} exceeds range of unsigned long.")),
    }
}

fn character_by_name(name: &str) -> Option<char> {
    let ch = unicode_names2::character(name)?;
    // The lookup is forgiving about spacing; Java's is not.
    let exact = unicode_names2::name(ch)?.to_string();
    (exact.eq_ignore_ascii_case(name.trim())).then_some(ch)
}
