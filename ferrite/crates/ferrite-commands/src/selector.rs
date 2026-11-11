//! Entity selectors: `@a`, `@e[type=cow,limit=3]`, player names and UUIDs.
//!
//! This is the syntax of the official server's `EntitySelectorParser` and
//! `EntitySelectorOptions`: the same options, the same errors at the same positions. What a
//! selector picks is for the game to work out from the [`Selector`].

use crate::identifier::{read_identifier, registry_contains};
use crate::range::{self, Range};
use crate::reader::{Arg, Message, Reader, Result, SyntaxError};

/// How results are ordered before `limit` is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Arbitrary,
    Nearest,
    Furthest,
    Random,
}

/// What a selector names.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// `@a`, `@e`, `@p`, `@r`, `@s`, `@n`.
    Selector(char),
    /// A player by name.
    Name(String),
    /// An entity or player by UUID.
    Uuid(u128),
}

/// An entity type or tag a `type=` option names.
#[derive(Clone, Debug, PartialEq)]
pub enum TypeFilter {
    Type(String),
    Tag(String),
}

/// A parsed selector.
#[derive(Clone, Debug, PartialEq)]
pub struct Selector {
    pub target: Target,
    pub max_results: i32,
    /// Whether it can select things that are not players.
    pub includes_entities: bool,
    /// `@s`.
    pub current_entity: bool,
    pub order: Order,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub dx: Option<f64>,
    pub dy: Option<f64>,
    pub dz: Option<f64>,
    pub distance: Option<Range<f64>>,
    pub level: Option<Range<i32>>,
    pub x_rotation: Option<Range<f64>>,
    pub y_rotation: Option<Range<f64>>,
    /// The one type the selector is limited to (`@a` is limited to players).
    pub only_type: Option<String>,
    /// `type=`, `type=!`: `(negated, what)`.
    pub types: Vec<(bool, TypeFilter)>,
    pub names: Vec<(bool, String)>,
    pub tags: Vec<(bool, String)>,
    pub teams: Vec<(bool, String)>,
    pub game_modes: Vec<(bool, u8)>,
    pub scores: Vec<(String, Range<i32>)>,
    /// `advancements={id=true}` or `{id={criterion=true}}`.
    pub advancements: Vec<(String, AdvancementTest)>,
    pub predicates: Vec<(bool, String)>,
}

/// What a selector asks of one advancement.
#[derive(Clone, Debug, PartialEq)]
pub enum AdvancementTest {
    Done(bool),
    Criteria(Vec<(String, bool)>),
}

impl Selector {
    fn new(target: Target) -> Selector {
        Selector {
            target,
            max_results: 0,
            includes_entities: false,
            current_entity: false,
            order: Order::Arbitrary,
            x: None,
            y: None,
            z: None,
            dx: None,
            dy: None,
            dz: None,
            distance: None,
            level: None,
            x_rotation: None,
            y_rotation: None,
            only_type: None,
            types: Vec::new(),
            names: Vec::new(),
            tags: Vec::new(),
            teams: Vec::new(),
            game_modes: Vec::new(),
            scores: Vec::new(),
            advancements: Vec::new(),
            predicates: Vec::new(),
        }
    }
}

/// Parser state that decides which options are still allowed.
#[derive(Default)]
struct State {
    limited: bool,
    sorted: bool,
    has_name_equals: bool,
    has_name_not_equals: bool,
    has_game_mode_equals: bool,
    has_game_mode_not_equals: bool,
    has_team_equals: bool,
    has_team_not_equals: bool,
    type_limited: bool,
    type_limited_inversely: bool,
    /// Entity type tags already named by `type=#...`.
    type_tags: Vec<String>,
    has_scores: bool,
    has_advancements: bool,
    /// `nbt=` was used and Ferrite cannot parse NBT yet.
    unsupported: bool,
}

fn err(reader: &Reader, key: &str) -> SyntaxError {
    reader.error(Message::tr(key))
}

fn err_with(reader: &Reader, key: &str, arg: &str) -> SyntaxError {
    reader.error(Message::tr_with(key, vec![Arg::Str(arg.to_string())]))
}

/// `Java UUID.fromString`, which is more lenient than the canonical form.
pub fn parse_uuid(s: &str) -> Option<u128> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return None;
    }
    let hex = |p: &str, max_digits: usize| -> Option<u128> {
        if p.is_empty() || p.len() > max_digits || !p.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        u128::from_str_radix(p, 16).ok()
    };
    let a = hex(parts[0], 8)?;
    let b = hex(parts[1], 4)?;
    let c = hex(parts[2], 4)?;
    let d = hex(parts[3], 4)?;
    let e = hex(parts[4], 12)?;
    Some((a << 96) | (b << 80) | (c << 64) | (d << 48) | e)
}

/// `shouldInvertValue`: a `!` before the value.
fn should_invert(reader: &mut Reader) -> bool {
    reader.skip_whitespace();
    if reader.can_read() && reader.peek() == '!' {
        reader.skip();
        reader.skip_whitespace();
        return true;
    }
    false
}

const GAME_MODES: [&str; 4] = ["survival", "creative", "adventure", "spectator"];

fn handle_option(
    reader: &mut Reader,
    sel: &mut Selector,
    st: &mut State,
    name: &str,
    option_start: usize,
) -> Result<()> {
    // Which options may still be used.
    let usable = match name {
        "name" => !st.has_name_equals,
        "distance" => sel.distance.is_none(),
        "level" => sel.level.is_none(),
        "x" => sel.x.is_none(),
        "y" => sel.y.is_none(),
        "z" => sel.z.is_none(),
        "dx" => sel.dx.is_none(),
        "dy" => sel.dy.is_none(),
        "dz" => sel.dz.is_none(),
        "x_rotation" => sel.x_rotation.is_none(),
        "y_rotation" => sel.y_rotation.is_none(),
        "limit" => !sel.current_entity && !st.limited,
        "sort" => !sel.current_entity && !st.sorted,
        "gamemode" => !st.has_game_mode_equals,
        "team" => !st.has_team_equals,
        "type" => !st.type_limited,
        "tag" | "nbt" | "predicate" => true,
        "scores" => !st.has_scores,
        "advancements" => !st.has_advancements,
        _ => {
            reader.set_cursor(option_start);
            return Err(err_with(reader, "argument.entity.options.unknown", name));
        }
    };
    if !usable {
        reader.set_cursor(option_start);
        return Err(err_with(reader, "argument.entity.options.inapplicable", name));
    }
    let value_start = reader.cursor();
    let inapplicable = |reader: &mut Reader| {
        reader.set_cursor(value_start);
        err_with(reader, "argument.entity.options.inapplicable", name)
    };
    match name {
        "name" => {
            let negated = should_invert(reader);
            if st.has_name_not_equals && !negated {
                return Err(inapplicable(reader));
            }
            let s = reader.read_string()?;
            if negated {
                st.has_name_not_equals = true;
            } else {
                st.has_name_equals = true;
            }
            sel.names.push((negated, s));
        }
        "distance" => {
            let r = range::parse_float_range(reader)?;
            if r.min.is_some_and(|v| v < 0.0) || r.max.is_some_and(|v| v < 0.0) {
                reader.set_cursor(value_start);
                return Err(err(reader, "argument.entity.options.distance.negative"));
            }
            sel.distance = Some(r);
        }
        "level" => {
            sel.level = Some(range::parse_int_range(reader)?);
            sel.includes_entities = false;
        }
        "x" => sel.x = Some(reader.read_double()?),
        "y" => sel.y = Some(reader.read_double()?),
        "z" => sel.z = Some(reader.read_double()?),
        "dx" => sel.dx = Some(reader.read_double()?),
        "dy" => sel.dy = Some(reader.read_double()?),
        "dz" => sel.dz = Some(reader.read_double()?),
        "x_rotation" => sel.x_rotation = Some(range::parse_degrees(reader)?),
        "y_rotation" => sel.y_rotation = Some(range::parse_degrees(reader)?),
        "limit" => {
            let n = reader.read_int()?;
            if n < 1 {
                reader.set_cursor(value_start);
                return Err(err(reader, "argument.entity.options.limit.toosmall"));
            }
            sel.max_results = n;
            st.limited = true;
        }
        "sort" => {
            let s = reader.read_unquoted_string();
            sel.order = match s.as_str() {
                "nearest" => Order::Nearest,
                "furthest" => Order::Furthest,
                "random" => Order::Random,
                "arbitrary" => Order::Arbitrary,
                _ => {
                    reader.set_cursor(value_start);
                    return Err(err_with(reader, "argument.entity.options.sort.irreversible", &s));
                }
            };
            st.sorted = true;
        }
        "gamemode" => {
            let negated = should_invert(reader);
            if st.has_game_mode_not_equals && !negated {
                return Err(inapplicable(reader));
            }
            let s = reader.read_unquoted_string();
            let Some(mode) = GAME_MODES.iter().position(|m| *m == s) else {
                reader.set_cursor(value_start);
                return Err(err_with(reader, "argument.entity.options.mode.invalid", &s));
            };
            if negated {
                st.has_game_mode_not_equals = true;
            } else {
                st.has_game_mode_equals = true;
            }
            sel.includes_entities = false;
            sel.game_modes.push((negated, mode as u8));
        }
        "team" => {
            let negated = should_invert(reader);
            if st.has_team_not_equals && !negated {
                return Err(inapplicable(reader));
            }
            let s = reader.read_unquoted_string();
            if negated {
                st.has_team_not_equals = true;
            } else {
                st.has_team_equals = true;
            }
            sel.teams.push((negated, s));
        }
        "type" => {
            let negated = should_invert(reader);
            if reader.can_read() && reader.peek() == '#' {
                // A tag limits the type only by a test, like an inverted type: a single type
                // can no longer be asked for afterwards.
                st.type_limited_inversely = true;
                reader.skip();
                let tag = read_identifier(reader)?;
                if st.type_tags.contains(&tag) {
                    return Err(inapplicable(reader));
                }
                st.type_tags.push(tag.clone());
                sel.types.push((negated, TypeFilter::Tag(tag)));
            } else {
                if st.type_limited_inversely && !negated {
                    return Err(inapplicable(reader));
                }
                if negated {
                    st.type_limited_inversely = true;
                }
                let id = read_identifier(reader)?;
                if registry_contains("minecraft:entity_type", &id) != Some(true) {
                    reader.set_cursor(value_start);
                    return Err(err_with(reader, "argument.entity.options.type.invalid", &id));
                }
                if id == "minecraft:player" && !negated {
                    sel.includes_entities = false;
                }
                if !negated {
                    st.type_limited = true;
                    sel.only_type = Some(id.clone());
                }
                sel.types.push((negated, TypeFilter::Type(id)));
            }
        }
        "tag" => {
            let negated = should_invert(reader);
            let s = reader.read_unquoted_string();
            sel.tags.push((negated, s));
        }
        "predicate" => {
            let negated = should_invert(reader);
            let id = read_identifier(reader)?;
            sel.predicates.push((negated, id));
        }
        "nbt" => {
            // NBT needs the SNBT parser, which Ferrite does not have yet.
            st.unsupported = true;
            return Err(err(reader, "ferrite.unsupported"));
        }
        "scores" => {
            reader.expect('{')?;
            reader.skip_whitespace();
            while reader.can_read() && reader.peek() != '}' {
                reader.skip_whitespace();
                let objective = reader.read_unquoted_string();
                reader.skip_whitespace();
                reader.expect('=')?;
                reader.skip_whitespace();
                let r = range::parse_int_range(reader)?;
                sel.scores.push((objective, r));
                reader.skip_whitespace();
                if reader.can_read() && reader.peek() == ',' {
                    reader.skip();
                }
            }
            reader.expect('}')?;
            st.has_scores = true;
        }
        "advancements" => {
            reader.expect('{')?;
            reader.skip_whitespace();
            while reader.can_read() && reader.peek() != '}' {
                reader.skip_whitespace();
                let id = read_identifier(reader)?;
                reader.skip_whitespace();
                reader.expect('=')?;
                reader.skip_whitespace();
                if reader.can_read() && reader.peek() == '{' {
                    let mut criteria = Vec::new();
                    reader.skip_whitespace();
                    reader.expect('{')?;
                    reader.skip_whitespace();
                    while reader.can_read() && reader.peek() != '}' {
                        reader.skip_whitespace();
                        let criterion = reader.read_unquoted_string();
                        reader.skip_whitespace();
                        reader.expect('=')?;
                        reader.skip_whitespace();
                        let done = reader.read_boolean()?;
                        criteria.push((criterion, done));
                        reader.skip_whitespace();
                        if reader.can_read() && reader.peek() == ',' {
                            reader.skip();
                        }
                    }
                    reader.expect('}')?;
                    reader.skip_whitespace();
                    sel.advancements.push((id, AdvancementTest::Criteria(criteria)));
                } else {
                    let done = reader.read_boolean()?;
                    sel.advancements.push((id, AdvancementTest::Done(done)));
                }
                reader.skip_whitespace();
                if reader.can_read() && reader.peek() == ',' {
                    reader.skip();
                }
            }
            reader.expect('}')?;
            st.has_advancements = true;
            // Advancements only exist for players.
            if !sel.advancements.is_empty() {
                sel.includes_entities = false;
            }
        }
        _ => unreachable!("checked above"),
    }
    Ok(())
}

fn parse_options(reader: &mut Reader, sel: &mut Selector, st: &mut State) -> Result<()> {
    reader.skip_whitespace();
    while reader.can_read() && reader.peek() != ']' {
        reader.skip_whitespace();
        let start = reader.cursor();
        let name = reader.read_string()?;
        // An option must exist and be allowed before we look for its `=`.
        let known = matches!(
            name.as_str(),
            "name"
                | "distance"
                | "level"
                | "x"
                | "y"
                | "z"
                | "dx"
                | "dy"
                | "dz"
                | "x_rotation"
                | "y_rotation"
                | "limit"
                | "sort"
                | "gamemode"
                | "team"
                | "type"
                | "tag"
                | "nbt"
                | "predicate"
                | "scores"
                | "advancements"
        );
        if !known {
            reader.set_cursor(start);
            return Err(err_with(reader, "argument.entity.options.unknown", &name));
        }
        // `get` checks that the option may still be used before the `=` is read.
        let usable = match name.as_str() {
            "name" => !st.has_name_equals,
            "distance" => sel.distance.is_none(),
            "level" => sel.level.is_none(),
            "x" => sel.x.is_none(),
            "y" => sel.y.is_none(),
            "z" => sel.z.is_none(),
            "dx" => sel.dx.is_none(),
            "dy" => sel.dy.is_none(),
            "dz" => sel.dz.is_none(),
            "x_rotation" => sel.x_rotation.is_none(),
            "y_rotation" => sel.y_rotation.is_none(),
            "limit" => !sel.current_entity && !st.limited,
            "sort" => !sel.current_entity && !st.sorted,
            "gamemode" => !st.has_game_mode_equals,
            "team" => !st.has_team_equals,
            "type" => !st.type_limited,
            "scores" => !st.has_scores,
            "advancements" => !st.has_advancements,
            _ => true,
        };
        if !usable {
            reader.set_cursor(start);
            return Err(err_with(reader, "argument.entity.options.inapplicable", &name));
        }
        reader.skip_whitespace();
        if !reader.can_read() || reader.peek() != '=' {
            reader.set_cursor(start);
            return Err(err_with(reader, "argument.entity.options.valueless", &name));
        }
        reader.skip();
        reader.skip_whitespace();
        handle_option(reader, sel, st, &name, start)?;
        reader.skip_whitespace();
        if !reader.can_read() {
            continue;
        }
        if reader.peek() == ',' {
            reader.skip();
            continue;
        }
        if reader.peek() != ']' {
            return Err(err(reader, "argument.entity.options.unterminated"));
        }
        break;
    }
    if !reader.can_read() {
        return Err(err(reader, "argument.entity.options.unterminated"));
    }
    reader.skip();
    Ok(())
}

/// Whether parsing stopped because of something Ferrite cannot parse yet.
pub fn is_unsupported(e: &SyntaxError) -> bool {
    matches!(&e.message, Message::Translatable { key, .. } if key == "ferrite.unsupported")
}

fn parse_selector_type(reader: &mut Reader) -> Result<Selector> {
    if !reader.can_read() {
        return Err(err(reader, "argument.entity.selector.missing"));
    }
    let at = reader.cursor();
    let c = reader.read();
    let mut sel = Selector::new(Target::Selector(c));
    match c {
        'a' => {
            sel.max_results = i32::MAX;
            sel.includes_entities = false;
            sel.only_type = Some("minecraft:player".into());
        }
        'e' => {
            sel.max_results = i32::MAX;
            sel.includes_entities = true;
        }
        'n' => {
            sel.max_results = 1;
            sel.includes_entities = true;
            sel.order = Order::Nearest;
        }
        'p' => {
            sel.max_results = 1;
            sel.includes_entities = false;
            sel.order = Order::Nearest;
            sel.only_type = Some("minecraft:player".into());
        }
        'r' => {
            sel.max_results = 1;
            sel.includes_entities = false;
            sel.order = Order::Random;
            sel.only_type = Some("minecraft:player".into());
        }
        's' => {
            sel.max_results = 1;
            sel.includes_entities = true;
            sel.current_entity = true;
        }
        other => {
            reader.set_cursor(at);
            return Err(err_with(reader, "argument.entity.selector.unknown", &format!("@{other}")));
        }
    }
    Ok(sel)
}

/// Parses a selector, a player name or a UUID. `allow_selectors`: the source may use `@x`.
pub fn parse_selector(reader: &mut Reader, allow_selectors: bool) -> Result<Selector> {
    let mut st = State::default();
    if reader.can_read() && reader.peek() == '@' {
        if !allow_selectors {
            return Err(err(reader, "argument.entity.selector.not_allowed"));
        }
        reader.skip();
        let mut sel = parse_selector_type(reader)?;
        // `@a[...]`: the type's own limit (one for @p) stays unless `limit` changes it.
        if reader.can_read() && reader.peek() == '[' {
            reader.skip();
            parse_options(reader, &mut sel, &mut st)?;
        }
        return Ok(sel);
    }
    let start = reader.cursor();
    let s = reader.read_string()?;
    let mut sel;
    if let Some(uuid) = parse_uuid(&s) {
        sel = Selector::new(Target::Uuid(uuid));
        sel.includes_entities = true;
    } else {
        if s.is_empty() || s.encode_utf16().count() > 16 {
            reader.set_cursor(start);
            return Err(err(reader, "argument.entity.invalid"));
        }
        sel = Selector::new(Target::Name(s));
        sel.includes_entities = false;
    }
    sel.max_results = 1;
    Ok(sel)
}

/// `EntityArgument.parse`: a selector with the argument's own restrictions.
pub fn parse_entity(
    reader: &mut Reader,
    single: bool,
    players_only: bool,
    allow_selectors: bool,
) -> Result<Selector> {
    let sel = parse_selector(reader, allow_selectors)?;
    if sel.max_results > 1 && single {
        reader.set_cursor(0);
        return Err(err(
            reader,
            if players_only { "argument.player.toomany" } else { "argument.entity.toomany" },
        ));
    }
    if sel.includes_entities && players_only && !sel.current_entity {
        reader.set_cursor(0);
        return Err(err(reader, "argument.player.entities"));
    }
    Ok(sel)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<Selector> {
        parse_selector(&mut Reader::new(s), true)
    }

    #[test]
    fn the_selector_types_set_their_defaults() {
        let a = parse("@a").unwrap();
        assert_eq!((a.max_results, a.includes_entities), (i32::MAX, false));
        let p = parse("@p").unwrap();
        assert_eq!((p.max_results, p.order), (1, Order::Nearest));
        assert!(parse("@s").unwrap().current_entity);
        assert_eq!(parse("Steve").unwrap().target, Target::Name("Steve".into()));
    }

    #[test]
    fn options_are_collected() {
        let s = parse("@e[type=cow,limit=3,name=!Bob,tag=x,distance=..5,scores={a=1..2}]").unwrap();
        assert_eq!(s.max_results, 3);
        assert_eq!(s.only_type.as_deref(), Some("minecraft:cow"));
        assert_eq!(s.names, vec![(true, "Bob".to_string())]);
        assert_eq!(s.scores.len(), 1);
    }

    #[test]
    fn uuids_follow_javas_leniency() {
        assert_eq!(parse_uuid("12345678-1234-1234-1234-123456789abc").is_some(), true);
        assert!(parse_uuid("1-2-3-4-5").is_some());
        assert!(parse_uuid("1234").is_none());
    }
}
