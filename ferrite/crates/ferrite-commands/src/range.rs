//! Number ranges: `5`, `1..5`, `..5`, `1..`.

use crate::reader::{Arg, Message, Reader, Result, SyntaxError};

/// A range with optional ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

fn is_allowed_number(reader: &Reader) -> bool {
    let c = reader.peek();
    if c.is_ascii_digit() || c == '-' {
        return true;
    }
    // A dot belongs to the number unless it starts the `..` of a range.
    c == '.' && !(reader.can_read_n(2) && reader.peek_at(1) == '.')
}

fn read_text(reader: &mut Reader) -> String {
    let start = reader.cursor();
    while reader.can_read() && is_allowed_number(reader) {
        reader.skip();
    }
    reader.substring(start, reader.cursor())
}

fn parse_range<T: Copy + PartialOrd>(
    reader: &mut Reader,
    convert: impl Fn(&str) -> Option<T>,
    invalid: &str,
) -> Result<Range<T>> {
    let start = reader.cursor();
    // Errors point to the start of the range, whatever went wrong inside it.
    let at_start = |e: SyntaxError| SyntaxError { cursor: start as i64, ..e };
    if !reader.can_read() {
        return Err(at_start(reader.error(Message::tr("argument.range.empty"))));
    }
    let number = |reader: &mut Reader| -> Result<Option<T>> {
        let text = read_text(reader);
        if text.is_empty() {
            return Ok(None);
        }
        convert(&text)
            .map(Some)
            .ok_or_else(|| reader.error(Message::tr_with(invalid, vec![Arg::Str(text.clone())])))
    };
    let result = (|| {
        let min = number(reader)?;
        let max = if reader.can_read_n(2) && reader.peek() == '.' && reader.peek_at(1) == '.' {
            reader.skip();
            reader.skip();
            let max = number(reader)?;
            if min.is_none() && max.is_none() {
                return Err(reader.error(Message::tr("argument.range.empty")));
            }
            max
        } else {
            min
        };
        if min.is_none() && max.is_none() {
            return Err(reader.error(Message::tr("argument.range.empty")));
        }
        if let (Some(a), Some(b)) = (min, max)
            && a > b
        {
            return Err(reader.error(Message::tr("argument.range.swapped")));
        }
        Ok(Range { min, max })
    })();
    result.map_err(at_start)
}

pub fn parse_int_range(reader: &mut Reader) -> Result<Range<i32>> {
    parse_range(reader, |s| s.parse::<i32>().ok(), "parsing.int.invalid")
}

pub fn parse_float_range(reader: &mut Reader) -> Result<Range<f64>> {
    parse_range(
        reader,
        |s| {
            let body = s.strip_prefix('-').unwrap_or(s);
            let ok = !body.is_empty()
                && body.chars().filter(|c| *c == '.').count() <= 1
                && body.chars().all(|c| c.is_ascii_digit() || c == '.')
                && body.chars().any(|c| c.is_ascii_digit());
            if ok { s.parse::<f64>().ok() } else { None }
        },
        "parsing.float.invalid",
    )
}

/// Degrees ranges (`x_rotation=`, `y_rotation=`) read floats, which is why their errors say
/// `float`.
pub fn parse_degrees(reader: &mut Reader) -> Result<Range<f64>> {
    parse_float_range(reader)
}
