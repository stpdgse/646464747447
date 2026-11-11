//! Loot tables as generated JSON text (see `ferrite-loot` for evaluation).

pub use crate::generated::loot::{BLOCK_LOOT, PREDICATES, RECIPES};

/// The JSON of a block's loot table, by block name (`minecraft:stone`).
pub fn block_loot(block: &str) -> Option<&'static str> {
    let key = format!("minecraft:blocks/{}", block.trim_start_matches("minecraft:"));
    BLOCK_LOOT.binary_search_by(|(n, _)| (*n).cmp(key.as_str())).ok().map(|i| BLOCK_LOOT[i].1)
}

/// The JSON of a named predicate (`minecraft:tool/can_shear`).
pub fn predicate(name: &str) -> Option<&'static str> {
    PREDICATES.binary_search_by(|(n, _)| (*n).cmp(name)).ok().map(|i| PREDICATES[i].1)
}
