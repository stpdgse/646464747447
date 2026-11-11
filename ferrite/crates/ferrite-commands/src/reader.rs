//! Brigadier's `StringReader`: a cursor over a command line, with the ways it reads values and
//! the errors it raises (worded exactly as the library words them).

use std::fmt;

/// An argument of a translatable message.
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Str(String),
    Int(i64),
    Float(f32),
    Double(f64),
    Msg(Box<Message>),
}

/// An error message: a translation key with arguments, as the official server words its
/// command errors (it replaces Brigadier's English texts with keys), or plain text.
#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Literal(String),
    Translatable { key: String, args: Vec<Arg> },
}

impl Message {
    pub fn lit(s: impl Into<String>) -> Message {
        Message::Literal(s.into())
    }

    pub fn tr(key: &str) -> Message {
        Message::Translatable { key: key.to_string(), args: Vec::new() }
    }

    pub fn tr_with(key: &str, args: Vec<Arg>) -> Message {
        Message::Translatable { key: key.to_string(), args }
    }

    /// The message as compact JSON, the way the official server's tooling prints a component.
    pub fn to_json(&self) -> String {
        fn string(s: &str, out: &mut String) {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        match self {
            Message::Literal(s) => {
                let mut out = String::from("lit:");
                out.push_str(s);
                out
            }
            Message::Translatable { key, args } => {
                let mut out = String::from("{\"translate\":");
                string(key, &mut out);
                if !args.is_empty() {
                    out.push_str(",\"with\":[");
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            out.push(',');
                        }
                        match a {
                            Arg::Str(s) => string(s, &mut out),
                            Arg::Int(v) => out.push_str(&v.to_string()),
                            Arg::Float(v) => out.push_str(&crate::args::java_float(*v)),
                            Arg::Double(v) => out.push_str(&crate::args::java_double(*v)),
                            Arg::Msg(m) => out.push_str(&m.to_json()),
                        }
                    }
                    out.push(']');
                }
                out.push('}');
                out
            }
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_json())
    }
}

/// A command that does not parse: what is wrong and where.
#[derive(Clone, Debug, PartialEq)]
pub struct SyntaxError {
    pub message: Message,
    /// Offset in UTF-16 units from the start of the input; -1 for errors raised without a position.
    pub cursor: i64,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at position {}", self.message, self.cursor)
    }
}

impl std::error::Error for SyntaxError {}

pub type Result<T> = std::result::Result<T, SyntaxError>;

const SYNTAX_ESCAPE: char = '\\';
const SYNTAX_DOUBLE_QUOTE: char = '"';
const SYNTAX_SINGLE_QUOTE: char = '\'';

/// Whether `c` may appear in an unquoted string.
pub fn is_allowed_in_unquoted_string(c: char) -> bool {
    c.is_ascii_digit()
        || c.is_ascii_uppercase()
        || c.is_ascii_lowercase()
        || c == '_'
        || c == '-'
        || c == '.'
        || c == '+'
}

pub fn is_quoted_string_start(c: char) -> bool {
    c == SYNTAX_DOUBLE_QUOTE || c == SYNTAX_SINGLE_QUOTE
}

fn is_allowed_number(c: char) -> bool {
    c.is_ascii_digit() || c == '.' || c == '-'
}

/// A cursor over a command line. Positions are in UTF-16 code units, like the Java strings
/// the official server parses, so errors point to the same place.
#[derive(Clone, Debug)]
pub struct Reader {
    units: Vec<u16>,
    cursor: usize,
}

impl Reader {
    pub fn new(s: &str) -> Reader {
        Reader { units: s.encode_utf16().collect(), cursor: 0 }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_cursor(&mut self, c: usize) {
        self.cursor = c;
    }

    pub fn total_length(&self) -> usize {
        self.units.len()
    }

    pub fn remaining_length(&self) -> usize {
        self.units.len() - self.cursor
    }

    pub fn can_read_n(&self, n: usize) -> bool {
        self.cursor + n <= self.units.len()
    }

    pub fn can_read(&self) -> bool {
        self.can_read_n(1)
    }

    fn unit(&self, i: usize) -> u16 {
        self.units[i]
    }

    /// The character at the cursor (a lone surrogate half is returned as U+FFFD).
    pub fn peek(&self) -> char {
        self.peek_at(0)
    }

    pub fn peek_at(&self, offset: usize) -> char {
        char::decode_utf16([self.unit(self.cursor + offset)])
            .next()
            .and_then(|r| r.ok())
            .unwrap_or('\u{FFFD}')
    }

    pub fn read(&mut self) -> char {
        let c = self.peek();
        self.cursor += 1;
        c
    }

    pub fn skip(&mut self) {
        self.cursor += 1;
    }

    pub fn skip_whitespace(&mut self) {
        while self.can_read() && self.peek().is_whitespace() {
            self.skip();
        }
    }

    pub fn substring(&self, from: usize, to: usize) -> String {
        String::from_utf16_lossy(&self.units[from..to])
    }

    pub fn string(&self) -> String {
        self.substring(0, self.units.len())
    }

    pub fn read_so_far(&self) -> String {
        self.substring(0, self.cursor)
    }

    pub fn rest(&self) -> String {
        self.substring(self.cursor, self.units.len())
    }

    /// An error that carries no position (Brigadier's `create()`).
    pub fn error_without_position(&self, message: Message) -> SyntaxError {
        SyntaxError { message, cursor: -1 }
    }

    pub fn error(&self, message: Message) -> SyntaxError {
        SyntaxError { message, cursor: self.cursor as i64 }
    }

    pub fn read_int(&mut self) -> Result<i32> {
        let start = self.cursor;
        while self.can_read() && is_allowed_number(self.peek()) {
            self.skip();
        }
        let number = self.substring(start, self.cursor);
        if number.is_empty() {
            return Err(self.error(Message::tr("parsing.int.expected")));
        }
        number.parse::<i32>().map_err(|_| {
            self.cursor = start;
            self.error(Message::tr_with("parsing.int.invalid", vec![Arg::Str(number.clone())]))
        })
    }

    pub fn read_long(&mut self) -> Result<i64> {
        let start = self.cursor;
        while self.can_read() && is_allowed_number(self.peek()) {
            self.skip();
        }
        let number = self.substring(start, self.cursor);
        if number.is_empty() {
            return Err(self.error(Message::tr("parsing.long.expected")));
        }
        number.parse::<i64>().map_err(|_| {
            self.cursor = start;
            self.error(Message::tr_with("parsing.long.invalid", vec![Arg::Str(number.clone())]))
        })
    }

    pub fn read_double(&mut self) -> Result<f64> {
        let start = self.cursor;
        while self.can_read() && is_allowed_number(self.peek()) {
            self.skip();
        }
        let number = self.substring(start, self.cursor);
        if number.is_empty() {
            return Err(self.error(Message::tr("parsing.double.expected")));
        }
        parse_java_double(&number).ok_or_else(|| {
            self.cursor = start;
            self.error(Message::tr_with("parsing.double.invalid", vec![Arg::Str(number.clone())]))
        })
    }

    pub fn read_float(&mut self) -> Result<f32> {
        let start = self.cursor;
        while self.can_read() && is_allowed_number(self.peek()) {
            self.skip();
        }
        let number = self.substring(start, self.cursor);
        if number.is_empty() {
            return Err(self.error(Message::tr("parsing.float.expected")));
        }
        parse_java_float(&number).ok_or_else(|| {
            self.cursor = start;
            self.error(Message::tr_with("parsing.float.invalid", vec![Arg::Str(number.clone())]))
        })
    }

    pub fn read_unquoted_string(&mut self) -> String {
        let start = self.cursor;
        while self.can_read() && is_allowed_in_unquoted_string(self.peek()) {
            self.skip();
        }
        self.substring(start, self.cursor)
    }

    pub fn read_quoted_string(&mut self) -> Result<String> {
        if !self.can_read() {
            return Ok(String::new());
        }
        let next = self.peek();
        if !is_quoted_string_start(next) {
            return Err(self.error(Message::tr("parsing.quote.expected.start")));
        }
        self.skip();
        self.read_string_until(next)
    }

    pub fn read_string_until(&mut self, terminator: char) -> Result<String> {
        let mut result = String::new();
        let mut escaped = false;
        while self.can_read() {
            let c = self.read();
            if escaped {
                if c == terminator || c == SYNTAX_ESCAPE {
                    result.push(c);
                    escaped = false;
                } else {
                    self.cursor -= 1;
                    return Err(self.error(Message::tr_with(
                        "parsing.quote.escape",
                        vec![Arg::Str(c.to_string())],
                    )));
                }
            } else if c == SYNTAX_ESCAPE {
                escaped = true;
            } else if c == terminator {
                return Ok(result);
            } else {
                result.push(c);
            }
        }
        Err(self.error(Message::tr("parsing.quote.expected.end")))
    }

    pub fn read_string(&mut self) -> Result<String> {
        if !self.can_read() {
            return Ok(String::new());
        }
        let next = self.peek();
        if is_quoted_string_start(next) {
            self.skip();
            return self.read_string_until(next);
        }
        Ok(self.read_unquoted_string())
    }

    pub fn read_boolean(&mut self) -> Result<bool> {
        let start = self.cursor;
        let value = self.read_string()?;
        if value.is_empty() {
            return Err(self.error(Message::tr("parsing.bool.expected")));
        }
        match value.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => {
                self.cursor = start;
                Err(self
                    .error(Message::tr_with("parsing.bool.invalid", vec![Arg::Str(value.clone())])))
            }
        }
    }

    pub fn expect(&mut self, c: char) -> Result<()> {
        if !self.can_read() || self.peek() != c {
            return Err(
                self.error(Message::tr_with("parsing.expected", vec![Arg::Str(c.to_string())]))
            );
        }
        self.skip();
        Ok(())
    }
}

/// `Double.parseDouble` for the characters a command number can have (digits, `.`, `-`).
fn parse_java_double(s: &str) -> Option<f64> {
    if !java_decimal(s) {
        return None;
    }
    s.parse::<f64>().ok()
}

fn parse_java_float(s: &str) -> Option<f32> {
    if !java_decimal(s) {
        return None;
    }
    s.parse::<f32>().ok()
}

/// Java accepts `1.`, `.5` and `-.5` but not `-`, `.`, `1-2` or `1.2.3`.
fn java_decimal(s: &str) -> bool {
    let body = s.strip_prefix('-').unwrap_or(s);
    let mut dots = 0;
    let mut digits = 0;
    for c in body.chars() {
        match c {
            '.' => dots += 1,
            '0'..='9' => digits += 1,
            _ => return false,
        }
    }
    dots <= 1 && digits > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_like_brigadier() {
        let mut r = Reader::new("12 -3.5 abc");
        assert_eq!(r.read_int().unwrap(), 12);
        r.skip();
        assert_eq!(r.read_double().unwrap(), -3.5);
        r.skip();
        let e = r.read_int().unwrap_err();
        assert_eq!((e.message, e.cursor), (Message::tr("parsing.int.expected"), 8));
    }

    #[test]
    fn invalid_numbers_rewind_to_their_start() {
        let mut r = Reader::new("1-2");
        let e = r.read_int().unwrap_err();
        assert_eq!(
            (e.message.to_json(), e.cursor),
            (r#"{"translate":"parsing.int.invalid","with":["1-2"]}"#.to_string(), 0)
        );
        let mut r = Reader::new("99999999999");
        assert!(r.read_int().is_err());
    }

    #[test]
    fn strings_quoted_and_not() {
        let mut r = Reader::new(r#"hello "wor\"ld" 'x'"#);
        assert_eq!(r.read_string().unwrap(), "hello");
        r.skip();
        assert_eq!(r.read_string().unwrap(), "wor\"ld");
        r.skip();
        assert_eq!(r.read_string().unwrap(), "x");
        let mut r = Reader::new("\"open");
        assert_eq!(r.read_string().unwrap_err().message, Message::tr("parsing.quote.expected.end"));
    }

    #[test]
    fn booleans() {
        assert!(Reader::new("true").read_boolean().unwrap());
        assert_eq!(
            Reader::new("maybe").read_boolean().unwrap_err().message,
            Message::tr_with("parsing.bool.invalid", vec![Arg::Str("maybe".into())])
        );
        assert_eq!(
            Reader::new("").read_boolean().unwrap_err().message,
            Message::tr("parsing.bool.expected")
        );
    }
}
