//! Argument types: what each parser reads from a command line and the value it makes.
//!
//! Syntax only: whether an entity or a block exists is the executing command's business, as in
//! the official server.

use crate::reader::{Arg, Message, Reader, Result, SyntaxError};
use crate::tree::{Parser, StringKind};

/// A parsed argument value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    /// A game mode by id (0 survival, 1 creative, 2 adventure, 3 spectator).
    GameMode(u8),
    /// A duration in ticks.
    Time(i32),
    /// A `namespace:path` identifier (the namespace defaults to `minecraft`).
    Identifier(String),
}

fn sax(reader: &Reader, message: Message) -> SyntaxError {
    reader.error(message)
}

/// Brigadier's bounds errors, as the official server words them: `argument.integer.low` with
/// the bound and what was found.
fn bound_error(
    reader: &mut Reader,
    start: usize,
    key: &str,
    bound: Arg,
    found: Arg,
) -> SyntaxError {
    reader.set_cursor(start);
    sax(reader, Message::tr_with(key, vec![bound, found]))
}

/// Java's `Float.toString` / `Double.toString` as Brigadier prints bounds and found values.
pub fn java_float(v: f32) -> String {
    java_number(f64::from(v), true)
}

pub fn java_double(v: f64) -> String {
    java_number(v, false)
}

fn java_number(v: f64, single: bool) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0.0" } else { "0.0" }.into();
    }
    let abs = v.abs();
    // Java switches to scientific notation below 1e-3 and from 1e7.
    if (1e-3..1e7).contains(&abs) {
        let s = if single { format!("{}", v as f32) } else { format!("{v}") };
        if s.contains('.') { s } else { format!("{s}.0") }
    } else {
        let s = if single { format!("{:e}", v as f32) } else { format!("{v:e}") };
        let (mantissa, exp) = s.split_once('e').expect("exponent");
        let mantissa =
            if mantissa.contains('.') { mantissa.to_string() } else { format!("{mantissa}.0") };
        format!("{mantissa}E{exp}")
    }
}

impl Parser {
    /// Reads a value of this type from `reader`.
    pub fn parse(&self, reader: &mut Reader) -> Result<Value> {
        let start = reader.cursor();
        match self {
            Parser::Bool => reader.read_boolean().map(Value::Bool),
            Parser::Integer { min, max } => {
                let v = reader.read_int()?;
                if let Some(m) = min
                    && v < *m
                {
                    return Err(bound_error(
                        reader,
                        start,
                        "argument.integer.low",
                        Arg::Int(i64::from(*m)),
                        Arg::Int(i64::from(v)),
                    ));
                }
                if let Some(m) = max
                    && v > *m
                {
                    return Err(bound_error(
                        reader,
                        start,
                        "argument.integer.big",
                        Arg::Int(i64::from(*m)),
                        Arg::Int(i64::from(v)),
                    ));
                }
                Ok(Value::Int(v))
            }
            Parser::Long { min, max } => {
                let v = reader.read_long()?;
                if let Some(m) = min
                    && v < *m
                {
                    return Err(bound_error(
                        reader,
                        start,
                        "argument.long.low",
                        Arg::Int(*m),
                        Arg::Int(v),
                    ));
                }
                if let Some(m) = max
                    && v > *m
                {
                    return Err(bound_error(
                        reader,
                        start,
                        "argument.long.big",
                        Arg::Int(*m),
                        Arg::Int(v),
                    ));
                }
                Ok(Value::Long(v))
            }
            Parser::Float { min, max } => {
                let v = reader.read_float()?;
                if let Some(m) = min
                    && v < *m
                {
                    return Err(bound_error(
                        reader,
                        start,
                        "argument.float.low",
                        Arg::Float(*m),
                        Arg::Float(v),
                    ));
                }
                if let Some(m) = max
                    && v > *m
                {
                    return Err(bound_error(
                        reader,
                        start,
                        "argument.float.big",
                        Arg::Float(*m),
                        Arg::Float(v),
                    ));
                }
                Ok(Value::Float(v))
            }
            Parser::Double { min, max } => {
                let v = reader.read_double()?;
                if let Some(m) = min
                    && v < *m
                {
                    return Err(bound_error(
                        reader,
                        start,
                        "argument.double.low",
                        Arg::Double(*m),
                        Arg::Double(v),
                    ));
                }
                if let Some(m) = max
                    && v > *m
                {
                    return Err(bound_error(
                        reader,
                        start,
                        "argument.double.big",
                        Arg::Double(*m),
                        Arg::Double(v),
                    ));
                }
                Ok(Value::Double(v))
            }
            Parser::String(kind) => match kind {
                StringKind::Greedy => {
                    let text = reader.rest();
                    reader.set_cursor(reader.total_length());
                    Ok(Value::String(text))
                }
                StringKind::Word => Ok(Value::String(reader.read_unquoted_string())),
                StringKind::Quotable => reader.read_string().map(Value::String),
            },
            Parser::Simple(id) => {
                Err(sax(reader, Message::lit(format!("argument type {id} is not supported yet"))))
            }
            other => Err(sax(
                reader,
                Message::lit(format!("argument type {other:?} is not supported yet")),
            )),
        }
    }

    /// Whether [`Parser::parse`] handles this type.
    pub fn is_supported(&self) -> bool {
        matches!(
            self,
            Parser::Bool
                | Parser::Integer { .. }
                | Parser::Long { .. }
                | Parser::Float { .. }
                | Parser::Double { .. }
                | Parser::String(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_number_formatting() {
        assert_eq!(java_double(0.5), "0.5");
        assert_eq!(java_double(3.0), "3.0");
        assert_eq!(java_double(1e7), "1.0E7");
        assert_eq!(java_double(0.0001), "1.0E-4");
        assert_eq!(java_float(2.5), "2.5");
        assert_eq!(java_float(0.1), "0.1");
        assert_eq!(java_double(-3.0e10), "-3.0E10");
    }

    #[test]
    fn integers_enforce_their_bounds() {
        let p = Parser::Integer { min: Some(1), max: Some(10) };
        let mut r = Reader::new("11");
        let e = p.parse(&mut r).unwrap_err();
        assert_eq!(e.message.to_json(), r#"{"translate":"argument.integer.big","with":[10,11]}"#);
        assert_eq!(e.cursor, 0);
        assert_eq!(p.parse(&mut Reader::new("5")).unwrap(), Value::Int(5));
    }
}
