//! Positions and angles: `1 2 3`, `~ ~1 ~-2`, `^ ^ ^5`.

use crate::reader::{Message, Reader, Result, SyntaxError};

/// One coordinate: absolute, or relative to the executor (`~`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coord {
    pub relative: bool,
    pub value: f64,
}

/// A position: three world coordinates, or `^left ^up ^forward` relative to where the executor
/// looks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Coordinates {
    World([Coord; 3]),
    Local { left: f64, up: f64, forward: f64 },
}

fn mixed(reader: &Reader) -> SyntaxError {
    reader.error(Message::tr("argument.pos.mixed"))
}

fn is_relative(reader: &mut Reader) -> bool {
    if reader.peek() == '~' {
        reader.skip();
        true
    } else {
        false
    }
}

/// `WorldCoordinate.parseInt`: a whole number, or `~` with an offset that may have decimals.
fn parse_int_coord(reader: &mut Reader) -> Result<Coord> {
    if reader.can_read() && reader.peek() == '^' {
        return Err(mixed(reader));
    }
    if !reader.can_read() {
        return Err(reader.error(Message::tr("argument.pos.missing.int")));
    }
    let relative = is_relative(reader);
    let value = if reader.can_read() && reader.peek() != ' ' {
        if relative { reader.read_double()? } else { f64::from(reader.read_int()?) }
    } else {
        0.0
    };
    Ok(Coord { relative, value })
}

/// `WorldCoordinate.parseDouble`: `center_correct` moves a whole number to the middle of its
/// block (`tp 1 64 1` goes to `1.5 64 1.5`).
fn parse_double_coord(reader: &mut Reader, center_correct: bool) -> Result<Coord> {
    if reader.can_read() && reader.peek() == '^' {
        return Err(mixed(reader));
    }
    if !reader.can_read() {
        return Err(reader.error(Message::tr("argument.pos.missing.double")));
    }
    let relative = is_relative(reader);
    let start = reader.cursor();
    let mut value =
        if reader.can_read() && reader.peek() != ' ' { reader.read_double()? } else { 0.0 };
    let text = reader.substring(start, reader.cursor());
    if relative && text.is_empty() {
        return Ok(Coord { relative: true, value: 0.0 });
    }
    if !text.contains('.') && !relative && center_correct {
        value += 0.5;
    }
    Ok(Coord { relative, value })
}

fn parse_local(reader: &mut Reader) -> Result<Coordinates> {
    let start = reader.cursor();
    let next = |reader: &mut Reader| -> Result<f64> {
        if !reader.can_read() || reader.peek() != '^' {
            reader.set_cursor(start);
            return Err(mixed(reader));
        }
        reader.skip();
        Ok(if reader.can_read() && reader.peek() != ' ' { reader.read_double()? } else { 0.0 })
    };
    let left = next(reader)?;
    if reader.can_read() && reader.peek() == ' ' {
        reader.skip();
        let up = next(reader)?;
        if reader.can_read() && reader.peek() == ' ' {
            reader.skip();
            let forward = next(reader)?;
            return Ok(Coordinates::Local { left, up, forward });
        }
    }
    reader.set_cursor(start);
    Err(reader.error(Message::tr("argument.pos3d.incomplete")))
}

/// A block position (`block_pos`).
pub fn parse_block_pos(reader: &mut Reader) -> Result<Coordinates> {
    if reader.can_read() && reader.peek() == '^' {
        return parse_local(reader);
    }
    let start = reader.cursor();
    let x = parse_int_coord(reader)?;
    if reader.can_read() && reader.peek() == ' ' {
        reader.skip();
        let y = parse_int_coord(reader)?;
        if reader.can_read() && reader.peek() == ' ' {
            reader.skip();
            let z = parse_int_coord(reader)?;
            return Ok(Coordinates::World([x, y, z]));
        }
    }
    reader.set_cursor(start);
    Err(reader.error(Message::tr("argument.pos3d.incomplete")))
}

/// A position with decimals (`vec3`).
pub fn parse_vec3(reader: &mut Reader, center_correct: bool) -> Result<Coordinates> {
    if reader.can_read() && reader.peek() == '^' {
        return parse_local(reader);
    }
    let start = reader.cursor();
    let x = parse_double_coord(reader, center_correct)?;
    if reader.can_read() && reader.peek() == ' ' {
        reader.skip();
        let y = parse_double_coord(reader, false)?;
        if reader.can_read() && reader.peek() == ' ' {
            reader.skip();
            let z = parse_double_coord(reader, center_correct)?;
            return Ok(Coordinates::World([x, y, z]));
        }
    }
    reader.set_cursor(start);
    Err(reader.error(Message::tr("argument.pos3d.incomplete")))
}

/// Two coordinates (`vec2`, `column_pos`, `rotation`).
pub fn parse_pair(
    reader: &mut Reader,
    ints: bool,
    center_correct: bool,
    incomplete: &str,
) -> Result<[Coord; 2]> {
    let start = reader.cursor();
    if !reader.can_read() {
        return Err(reader.error(Message::tr(incomplete)));
    }
    let one = |reader: &mut Reader, center: bool| {
        if ints { parse_int_coord(reader) } else { parse_double_coord(reader, center) }
    };
    let x = one(reader, center_correct)?;
    if reader.can_read() && reader.peek() == ' ' {
        reader.skip();
        let z = one(reader, center_correct)?;
        return Ok([x, z]);
    }
    reader.set_cursor(start);
    Err(reader.error(Message::tr(incomplete)))
}

/// An angle (`angle`): degrees, optionally relative.
pub fn parse_angle(reader: &mut Reader) -> Result<(bool, f32)> {
    if !reader.can_read() {
        return Err(reader.error(Message::tr("argument.angle.incomplete")));
    }
    let relative = is_relative(reader);
    let value = if reader.can_read() && reader.peek() != ' ' { reader.read_float()? } else { 0.0 };
    Ok((relative, value))
}
