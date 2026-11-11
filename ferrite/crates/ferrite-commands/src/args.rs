//! Argument types: what each parser reads from a command line and the value it makes.
//!
//! Syntax only: whether an entity or a block exists is the executing command's business, as in
//! the official server.

use crate::coordinates::{self, Coord, Coordinates};
use crate::identifier::{read_identifier, registry_contains, unknown_resource};
use crate::range::{self, Range};
use crate::reader::{Arg, Message, Reader, Result, SyntaxError};
use crate::selector::{self, Selector};
use crate::tree::{Parser, ResourceKind, StringKind, id};

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
    /// A block position or a position with decimals.
    Position(Coordinates),
    /// Two coordinates (`vec2`, `column_pos`, `rotation`).
    Pair([Coord; 2]),
    /// An angle in degrees, and whether it is relative.
    Angle {
        relative: bool,
        degrees: f32,
    },
    /// One of a fixed set of words (an enum), by its name.
    Word(String),
    /// Which of the axes `x`, `y`, `z` were named (bits 0 to 2).
    Axes(u8),
    IntRange(Range<i32>),
    FloatRange(Range<f64>),
    /// A resource of a registry, by its full id.
    Resource(String),
    /// An entity selector, player name or UUID.
    Selector(Selector),
    /// A `message`: the text, with the selectors found in it and where (UTF-16 offsets).
    Message {
        text: String,
        selectors: Vec<(usize, usize, Selector)>,
    },
    /// A score holder: a name (`*` for everyone that has a score) or a selector.
    ScoreHolder(ScoreHolder),
    Uuid(u128),
}

/// Who a score belongs to.
#[derive(Clone, Debug, PartialEq)]
pub enum ScoreHolder {
    Name(String),
    Wildcard,
    Selector(Box<Selector>),
}

/// Who runs the command (what parsing may depend on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
    /// Permission level 0 to 4.
    pub permission_level: u8,
}

impl Source {
    /// Whether the source may use `@a`-style selectors (game masters and up).
    pub fn allow_selectors(self) -> bool {
        self.permission_level >= 2
    }
}

const GAME_MODES: [&str; 4] = ["survival", "creative", "adventure", "spectator"];
const COLORS: [&str; 17] = [
    "black",
    "dark_blue",
    "dark_green",
    "dark_aqua",
    "dark_red",
    "dark_purple",
    "gold",
    "gray",
    "dark_gray",
    "blue",
    "green",
    "aqua",
    "red",
    "light_purple",
    "yellow",
    "white",
    "reset",
];
const OPERATIONS: [&str; 9] = ["=", "+=", "-=", "*=", "/=", "%=", "<", ">", "><"];

fn read_word(reader: &mut Reader) -> String {
    reader.read_unquoted_string()
}

fn invalid(reader: &Reader, key: &str, word: &str) -> SyntaxError {
    reader.error(Message::tr_with(key, vec![Arg::Str(word.to_string())]))
}

/// A word that must be one of `allowed`, or `key` is raised with the word.
fn one_of(reader: &mut Reader, allowed: &[&str], key: &str) -> Result<Value> {
    let word = read_word(reader);
    if allowed.contains(&word.as_str()) {
        Ok(Value::Word(word))
    } else {
        Err(invalid(reader, key, &word))
    }
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
    pub fn parse(&self, reader: &mut Reader, source: Source) -> Result<Value> {
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
            Parser::Simple(id::BLOCK_POS) => {
                coordinates::parse_block_pos(reader).map(Value::Position)
            }
            Parser::Simple(id::VEC3) => coordinates::parse_vec3(reader, true).map(Value::Position),
            Parser::Simple(id::COLUMN_POS) => {
                coordinates::parse_pair(reader, true, false, "argument.pos2d.incomplete")
                    .map(Value::Pair)
            }
            Parser::Simple(id::VEC2) => {
                coordinates::parse_pair(reader, false, true, "argument.pos2d.incomplete")
                    .map(Value::Pair)
            }
            Parser::Simple(id::ROTATION) => {
                coordinates::parse_pair(reader, false, false, "argument.rotation.incomplete")
                    .map(Value::Pair)
            }
            Parser::Simple(id::ANGLE) => coordinates::parse_angle(reader)
                .map(|(relative, degrees)| Value::Angle { relative, degrees }),
            Parser::Entity { single, players } => {
                selector::parse_entity(reader, *single, *players, source.allow_selectors())
                    .map(Value::Selector)
            }
            Parser::ScoreHolder { multiple } => {
                if reader.can_read() && reader.peek() == '@' {
                    let sel = selector::parse_selector(reader, true)?;
                    if !*multiple && sel.max_results > 1 {
                        return Err(reader.error(Message::tr("argument.entity.toomany")));
                    }
                    return Ok(Value::ScoreHolder(ScoreHolder::Selector(Box::new(sel))));
                }
                let from = reader.cursor();
                while reader.can_read() && reader.peek() != ' ' {
                    reader.skip();
                }
                let name = reader.substring(from, reader.cursor());
                Ok(Value::ScoreHolder(if name == "*" {
                    ScoreHolder::Wildcard
                } else {
                    ScoreHolder::Name(name)
                }))
            }
            Parser::Simple(id::OBJECTIVE | id::TEAM) => {
                Ok(Value::String(reader.read_unquoted_string()))
            }
            Parser::Simple(id::GAME_PROFILE) => {
                if reader.can_read() && reader.peek() == '@' {
                    let sel = selector::parse_selector(reader, true)?;
                    if sel.includes_entities {
                        return Err(reader.error(Message::tr("argument.player.entities")));
                    }
                    return Ok(Value::Selector(sel));
                }
                let from = reader.cursor();
                while reader.can_read() && reader.peek() != ' ' {
                    reader.skip();
                }
                Ok(Value::String(reader.substring(from, reader.cursor())))
            }
            Parser::Simple(id::UUID) => {
                let rest = reader.rest();
                let len = rest.chars().take_while(|c| c.is_ascii_hexdigit() || *c == '-').count();
                match selector::parse_uuid(&rest[..len]) {
                    Some(u) if len > 0 => {
                        reader.set_cursor(reader.cursor() + len);
                        Ok(Value::Uuid(u))
                    }
                    _ => Err(reader.error(Message::tr("argument.uuid.invalid"))),
                }
            }
            Parser::Simple(id::MESSAGE) => parse_message(reader, source.allow_selectors()),
            Parser::Simple(id::GAMEMODE) => {
                let word = read_word(reader);
                match GAME_MODES.iter().position(|m| *m == word) {
                    Some(i) => Ok(Value::GameMode(i as u8)),
                    None => Err(invalid(reader, "argument.gamemode.invalid", &word)),
                }
            }
            Parser::Time { min } => {
                let f = reader.read_float()?;
                let unit = read_word(reader);
                let per = match unit.as_str() {
                    "d" => 24000.0f32,
                    "s" => 20.0,
                    "t" | "" => 1.0,
                    _ => return Err(reader.error(Message::tr("argument.time.invalid_unit"))),
                };
                let ticks = (f * per + 0.5).floor() as i32;
                if ticks < *min {
                    return Err(reader.error(Message::tr_with(
                        "argument.time.tick_count_too_low",
                        vec![Arg::Int(i64::from(*min)), Arg::Int(i64::from(ticks))],
                    )));
                }
                Ok(Value::Time(ticks))
            }
            Parser::Simple(id::RESOURCE_LOCATION) | Parser::Simple(id::DIMENSION) => {
                read_identifier(reader).map(Value::Identifier)
            }
            Parser::Simple(id::ENTITY_ANCHOR) => {
                one_of(reader, &["feet", "eyes"], "argument.anchor.invalid")
            }
            Parser::Simple(id::TEAM_COLOR) => one_of(reader, &COLORS, "argument.color.invalid"),
            Parser::Simple(id::OPERATION) => {
                if !reader.can_read() {
                    return Err(
                        reader.error_without_position(Message::tr("arguments.operation.invalid"))
                    );
                }
                let from = reader.cursor();
                while reader.can_read() && reader.peek() != ' ' {
                    reader.skip();
                }
                let word = reader.substring(from, reader.cursor());
                if OPERATIONS.contains(&word.as_str()) {
                    Ok(Value::Word(word))
                } else {
                    Err(reader.error_without_position(Message::tr("arguments.operation.invalid")))
                }
            }
            Parser::Simple(id::SWIZZLE) => {
                let mut axes = 0u8;
                while reader.can_read() && reader.peek() != ' ' {
                    let bit = match reader.read() {
                        'x' => 1,
                        'y' => 2,
                        'z' => 4,
                        _ => return Err(reader.error(Message::tr("arguments.swizzle.invalid"))),
                    };
                    if axes & bit != 0 {
                        return Err(reader.error(Message::tr("arguments.swizzle.invalid")));
                    }
                    axes |= bit;
                }
                Ok(Value::Axes(axes))
            }
            Parser::Simple(id::HEIGHTMAP) => one_of(
                reader,
                &["world_surface", "ocean_floor", "motion_blocking", "motion_blocking_no_leaves"],
                "argument.enum.invalid",
            ),
            Parser::Simple(id::TEMPLATE_MIRROR) => {
                one_of(reader, &["none", "left_right", "front_back"], "argument.enum.invalid")
            }
            Parser::Simple(id::TEMPLATE_ROTATION) => one_of(
                reader,
                &["none", "clockwise_90", "180", "counterclockwise_90"],
                "argument.enum.invalid",
            ),
            Parser::Simple(id::SCOREBOARD_SLOT) => {
                let word = read_word(reader);
                let team_color =
                    word.strip_prefix("sidebar.team.").is_some_and(|c| COLORS[..16].contains(&c));
                if matches!(word.as_str(), "list" | "sidebar" | "below_name") || team_color {
                    Ok(Value::Word(word))
                } else {
                    Err(invalid(reader, "argument.scoreboardDisplaySlot.invalid", &word))
                }
            }
            Parser::Simple(id::INT_RANGE) => range::parse_int_range(reader).map(Value::IntRange),
            Parser::Simple(id::FLOAT_RANGE) => {
                range::parse_float_range(reader).map(Value::FloatRange)
            }
            Parser::Resource { kind: ResourceKind::Resource, registry } => {
                let id = read_identifier(reader)?;
                match registry_contains(registry, &id) {
                    Some(true) => Ok(Value::Resource(id)),
                    _ => Err(unknown_resource(reader, &id, registry)),
                }
            }
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
                | Parser::Time { .. }
                | Parser::Entity { .. }
                | Parser::ScoreHolder { .. }
                | Parser::Resource { kind: ResourceKind::Resource, .. }
                | Parser::Simple(
                    id::BLOCK_POS | id::VEC3 | id::COLUMN_POS | id::VEC2 | id::ROTATION | id::ANGLE
                )
        )
    }
}

/// `MessageArgument.parse` for a source that may use selectors: the rest of the line, with the
/// selectors in it parsed (a lone `@` or an unknown type is plain text).
fn parse_message(reader: &mut Reader, allow_selectors: bool) -> Result<Value> {
    let start = reader.cursor();
    if !allow_selectors {
        let text = reader.rest();
        reader.set_cursor(reader.total_length());
        return Ok(Value::Message { text, selectors: Vec::new() });
    }
    let mut selectors = Vec::new();
    loop {
        if !reader.can_read() {
            return Ok(Value::Message {
                text: reader.substring(start, reader.cursor()),
                selectors,
            });
        }
        if reader.peek() == '@' {
            let at = reader.cursor();
            match selector::parse_selector(reader, true) {
                Ok(sel) => selectors.push((at - start, reader.cursor() - start, sel)),
                Err(e) => {
                    let plain = matches!(
                        &e.message,
                        Message::Translatable { key, .. }
                            if key == "argument.entity.selector.missing"
                                || key == "argument.entity.selector.unknown"
                    );
                    if !plain {
                        return Err(e);
                    }
                    reader.set_cursor(at + 1);
                }
            }
        } else {
            reader.skip();
        }
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
        let e = p.parse(&mut r, Source { permission_level: 4 }).unwrap_err();
        assert_eq!(e.message.to_json(), r#"{"translate":"argument.integer.big","with":[10,11]}"#);
        assert_eq!(e.cursor, 0);
        assert_eq!(
            p.parse(&mut Reader::new("5"), Source { permission_level: 4 }).unwrap(),
            Value::Int(5)
        );
    }
}
