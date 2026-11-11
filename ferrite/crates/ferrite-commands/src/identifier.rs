//! Resource identifiers (`minecraft:stone`) and the registries they name.

use crate::reader::{Arg, Message, Reader, Result};

pub fn is_allowed_in_identifier(c: char) -> bool {
    c.is_ascii_digit()
        || c.is_ascii_lowercase()
        || c == '_'
        || c == ':'
        || c == '/'
        || c == '.'
        || c == '-'
}

fn valid_namespace(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase() || c == '_' || c == '-' || c == '.')
}

fn valid_path(s: &str) -> bool {
    s.chars().all(|c| {
        c.is_ascii_digit() || c.is_ascii_lowercase() || c == '_' || c == '-' || c == '.' || c == '/'
    })
}

/// Splits and checks `namespace:path`; the namespace defaults to `minecraft`.
pub fn parse_identifier(s: &str) -> Option<(String, String)> {
    let (ns, path) = match s.find(':') {
        Some(0) => ("minecraft", &s[1..]),
        Some(i) => (&s[..i], &s[i + 1..]),
        None => ("minecraft", s),
    };
    (valid_namespace(ns) && valid_path(path)).then(|| (ns.to_string(), path.to_string()))
}

/// `Identifier.read`: the longest run of identifier characters.
pub fn read_identifier(reader: &mut Reader) -> Result<String> {
    let start = reader.cursor();
    while reader.can_read() && is_allowed_in_identifier(reader.peek()) {
        reader.skip();
    }
    let text = reader.substring(start, reader.cursor());
    match parse_identifier(&text) {
        Some((ns, path)) => Ok(format!("{ns}:{path}")),
        None => {
            reader.set_cursor(start);
            Err(reader.error(Message::tr("argument.id.invalid")))
        }
    }
}

/// Whether `registry` (e.g. `minecraft:entity_type`) holds `id`; `None` when Ferrite has no
/// data for that registry.
pub fn registry_contains(registry: &str, id: &str) -> Option<bool> {
    if let Some(r) = ferrite_data::registries::Registry::find(registry) {
        return Some(r.id_of(id).is_some());
    }
    ferrite_data::configuration::SYNCHRONIZED_REGISTRIES
        .iter()
        .find(|r| r.id == registry)
        .map(|r| r.entries.contains(&id))
}

pub fn unknown_resource(reader: &Reader, id: &str, registry: &str) -> crate::reader::SyntaxError {
    reader.error(Message::tr_with(
        "argument.resource.not_found",
        vec![Arg::Str(id.to_string()), Arg::Str(registry.to_string())],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_default_to_the_minecraft_namespace() {
        assert_eq!(parse_identifier("stone"), Some(("minecraft".into(), "stone".into())));
        assert_eq!(parse_identifier(":stone"), Some(("minecraft".into(), "stone".into())));
        assert_eq!(parse_identifier("a:b/c"), Some(("a".into(), "b/c".into())));
        assert_eq!(parse_identifier("a:b:c"), None);
        assert_eq!(parse_identifier("Stone"), None);
        assert_eq!(parse_identifier(""), Some(("minecraft".into(), String::new())));
    }

    #[test]
    fn registries_are_found_by_name() {
        assert_eq!(registry_contains("minecraft:entity_type", "minecraft:cow"), Some(true));
        assert_eq!(registry_contains("minecraft:entity_type", "minecraft:nope"), Some(false));
        assert_eq!(registry_contains("minecraft:worldgen/biome", "minecraft:plains"), Some(true));
    }
}
