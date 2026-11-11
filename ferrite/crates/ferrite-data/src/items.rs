//! Per-item facts: stack size, durability and tool behaviour, generated from the official item
//! component reports.

pub use crate::generated::items::ITEMS;

/// One entry of a tool's `rules`: which blocks it affects and how.
#[derive(Clone, Copy, Debug)]
pub struct ToolRule {
    /// A block name (`minecraft:cobweb`) or a block tag (`#minecraft:mineable/pickaxe`).
    pub blocks: &'static str,
    /// Mining speed on these blocks, if the rule sets one.
    pub speed: Option<f32>,
    /// Whether the tool counts as the correct one for drops, if the rule says.
    pub correct_for_drops: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub struct ToolInfo {
    pub rules: &'static [ToolRule],
    pub damage_per_block: u8,
    pub can_destroy_blocks_in_creative: bool,
    pub default_mining_speed: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ItemInfo {
    pub max_stack: u8,
    /// 0 for items without durability.
    pub max_damage: u16,
    pub tool: Option<ToolInfo>,
}

/// The facts about the item with this protocol id.
pub fn item_info(id: i32) -> Option<&'static ItemInfo> {
    ITEMS.get(usize::try_from(id).ok()?)
}
