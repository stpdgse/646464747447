//! How long breaking a block takes: hardness, the tool in hand and the player's condition.

use std::collections::HashMap;
use std::sync::OnceLock;

use ferrite_data::configuration::TAG_REGISTRIES;
use ferrite_data::items::{ItemInfo, ToolRule};
use ferrite_data::shapes::destroy_info;

use crate::kinds::block_id_of;

/// Everything about the digger that changes the speed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Digger {
    /// The held item's tool component, if it has one.
    pub tool: Option<&'static ItemInfo>,
    /// Efficiency level on the held item (0 = none).
    pub efficiency: u8,
    /// Haste amplifier plus one (0 = no haste).
    pub haste: u8,
    /// Mining fatigue amplifier plus one (0 = none).
    pub fatigue: u8,
    /// The head is under water.
    pub eyes_in_water: bool,
    /// Aqua affinity is worn.
    pub aqua_affinity: bool,
    pub on_ground: bool,
}

fn block_tags() -> &'static HashMap<&'static str, &'static [u16]> {
    static TAGS: OnceLock<HashMap<&'static str, &'static [u16]>> = OnceLock::new();
    TAGS.get_or_init(|| {
        TAG_REGISTRIES
            .iter()
            .find(|r| r.registry == "minecraft:block")
            .map(|r| r.tags.iter().map(|t| (t.name, t.entries)).collect())
            .unwrap_or_default()
    })
}

/// Whether the block state is selected by a tool rule's `blocks` (a tag or a block name).
fn selects(rule: &ToolRule, state: u16) -> bool {
    if let Some(tag) = rule.blocks.strip_prefix('#') {
        block_tags().get(tag).is_some_and(|entries| entries.contains(&block_id_of(state)))
    } else {
        ferrite_data::blocks::block_by_name(rule.blocks.trim_start_matches("minecraft:"))
            .is_some_and(|b| b.id == block_id_of(state))
    }
}

impl Digger {
    /// The tool's speed on this block (vanilla's `Tool.getMiningSpeed`).
    fn tool_speed(&self, state: u16) -> f32 {
        let Some(tool) = self.tool.and_then(|i| i.tool) else { return 1.0 };
        tool.rules
            .iter()
            .find(|r| r.speed.is_some() && selects(r, state))
            .and_then(|r| r.speed)
            .unwrap_or(tool.default_mining_speed)
    }

    /// Whether what is held is the right tool for this block's drops.
    pub fn is_correct_tool(&self, state: u16) -> bool {
        let (_, needs_tool) = destroy_info(state);
        if !needs_tool {
            return true;
        }
        let Some(tool) = self.tool.and_then(|i| i.tool) else { return false };
        tool.rules
            .iter()
            .find(|r| r.correct_for_drops.is_some() && selects(r, state))
            .and_then(|r| r.correct_for_drops)
            .unwrap_or(false)
    }

    /// The player's mining speed against `state`.
    pub fn speed(&self, state: u16) -> f32 {
        let mut speed = self.tool_speed(state);
        if speed > 1.0 && self.efficiency > 0 {
            let level = f32::from(self.efficiency);
            speed += level * level + 1.0;
        }
        if self.haste > 0 {
            speed *= 1.0 + f32::from(self.haste) * 0.2;
        }
        if self.fatigue > 0 {
            speed *= match self.fatigue {
                1 => 0.3,
                2 => 0.09,
                3 => 0.027,
                _ => 0.00081,
            };
        }
        if self.eyes_in_water && !self.aqua_affinity {
            speed *= 0.2;
        }
        if !self.on_ground {
            speed /= 5.0;
        }
        speed
    }

    /// How much of the block one tick of digging removes (`BlockState.getDestroyProgress`);
    /// the block breaks when the total reaches 1. Zero for unbreakable blocks.
    pub fn progress_per_tick(&self, state: u16) -> f32 {
        let (hardness, _) = destroy_info(state);
        if hardness == -1.0 {
            return 0.0;
        }
        let divisor = if self.is_correct_tool(state) { 30.0 } else { 100.0 };
        self.speed(state) / hardness / divisor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrite_data::blocks::block_by_name;
    use ferrite_data::items::item_info;
    use ferrite_data::registries::item;

    fn ticks_to_break(block: &str, tool: &str, efficiency: u8) -> u32 {
        let state = block_by_name(block).expect("block").default_state;
        let info = item::by_name
            .get(format!("minecraft:{tool}").as_str())
            .and_then(|id| item_info(*id as i32));
        let digger = Digger { tool: info, efficiency, on_ground: true, ..Digger::default() };
        let p = digger.progress_per_tick(state);
        if p <= 0.0 {
            return u32::MAX;
        }
        (1.0 / p).ceil() as u32
    }

    // Times from the game's wiki table of breaking speeds, in seconds * 20.
    #[test]
    fn matches_the_known_breaking_times() {
        assert_eq!(ticks_to_break("dirt", "air", 0), 15); // 0.75 s by hand
        assert_eq!(ticks_to_break("stone", "air", 0), 150); // 7.5 s: no drops, so the slow divisor
        assert_eq!(ticks_to_break("stone", "iron_pickaxe", 0), 8); // 0.4 s (7.5 ticks, rounded up)
        assert_eq!(ticks_to_break("stone", "wooden_pickaxe", 0), 23); // 1.15 s
        assert_eq!(ticks_to_break("obsidian", "diamond_pickaxe", 0), 188); // 9.4 s
        assert_eq!(ticks_to_break("oak_log", "iron_axe", 0), 10); // 0.5 s
    }

    #[test]
    fn efficiency_and_unbreakable_blocks() {
        assert!(
            ticks_to_break("stone", "iron_pickaxe", 5) < ticks_to_break("stone", "iron_pickaxe", 0)
        );
        assert_eq!(ticks_to_break("bedrock", "diamond_pickaxe", 0), u32::MAX);
        // Obsidian needs a diamond pickaxe for drops: with an iron one it is 5x slower.
        assert!(
            ticks_to_break("obsidian", "iron_pickaxe", 0)
                > 4 * ticks_to_break("obsidian", "diamond_pickaxe", 0)
        );
    }
}
