use anyhow::{Result, bail};
use heck::ToShoutySnakeCase;

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];

/// Strips the `minecraft:` namespace; other namespaces are kept as a prefix.
pub fn strip_namespace(key: &str) -> String {
    match key.strip_prefix("minecraft:") {
        Some(rest) => rest.to_string(),
        None => key.replace(':', "_"),
    }
}

/// `minecraft:worldgen/biome_source` -> `worldgen_biome_source`.
pub fn module_name(key: &str) -> Result<String> {
    let name: String = strip_namespace(key)
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        bail!("cannot derive a module name from {key:?}");
    }
    if KEYWORDS.contains(&name.as_str()) {
        return Ok(format!("{name}_"));
    }
    Ok(name)
}

/// `minecraft:acacia_boat` -> `ACACIA_BOAT`.
pub fn const_name(key: &str) -> String {
    let stripped = strip_namespace(key);
    let mut s = stripped.replace(['/', '.', '-'], "_").to_shouty_snake_case();
    if s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert(0, '_');
    }
    s
}

/// Asserts `ids` is a permutation of `0..n` and returns the position of each id.
pub fn order_by_id<T: Clone>(what: &str, items: &[(i64, T)]) -> Result<Vec<T>> {
    let mut out: Vec<Option<T>> = vec![None; items.len()];
    for (id, item) in items {
        let Some(slot) = usize::try_from(*id).ok().and_then(|i| out.get_mut(i)) else {
            bail!("{what}: id {id} is outside 0..{} (ids are not contiguous)", items.len());
        };
        if slot.replace(item.clone()).is_some() {
            bail!("{what}: duplicate id {id}");
        }
    }
    Ok(out.into_iter().map(|o| o.expect("permutation checked above")).collect())
}
