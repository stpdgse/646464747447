//! Item facts the survival rules need: stack sizes, durability and tool rules, read from the
//! official data generator's item component reports.

use std::fmt::Write;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::GenContext;
use crate::registries::Registry;

pub fn generate(cx: &GenContext, regs: &[Registry]) -> Result<String> {
    let items =
        regs.iter().find(|r| r.key == "minecraft:item").context("the item registry is missing")?;
    let dir = cx.reports.join("minecraft/components/item");
    let mut out = String::new();
    out.push_str(&cx.header);
    out.push_str("\nuse crate::items::{EquipSlot, FoodInfo, ItemInfo, ToolInfo, ToolRule};\n\n");
    let _ = writeln!(out, "pub static ITEMS: [ItemInfo; {}] = [", items.names.len());
    for name in &items.names {
        let short = name.strip_prefix("minecraft:").unwrap_or(name);
        let path = dir.join(format!("{short}.json"));
        let json: Value = serde_json::from_slice(
            &std::fs::read(&path).with_context(|| format!("reading {path:?}"))?,
        )?;
        let c = json.get("components").context("no components")?;
        let max_stack = c
            .get("minecraft:max_stack_size")
            .and_then(Value::as_u64)
            .with_context(|| format!("{name}: no max_stack_size"))?;
        if !(1..=99).contains(&max_stack) {
            bail!("{name}: stack size {max_stack} out of range");
        }
        let max_damage = c.get("minecraft:max_damage").and_then(Value::as_u64).unwrap_or(0);
        let tool = match c.get("minecraft:tool") {
            None => "None".to_string(),
            Some(t) => {
                let mut rules = String::new();
                for r in t["rules"].as_array().context("tool rules")? {
                    let blocks = r["blocks"].as_str().context("tool rule blocks")?;
                    let speed = match r.get("speed").and_then(Value::as_f64) {
                        Some(s) => format!("Some({:?}f32)", s as f32),
                        None => "None".to_string(),
                    };
                    let correct = match r.get("correct_for_drops").and_then(Value::as_bool) {
                        Some(b) => format!("Some({b})"),
                        None => "None".to_string(),
                    };
                    let _ = write!(
                        rules,
                        "ToolRule {{ blocks: {blocks:?}, speed: {speed}, correct_for_drops: {correct} }}, "
                    );
                }
                let per_block = t.get("damage_per_block").and_then(Value::as_u64).unwrap_or(1);
                let creative = t
                    .get("can_destroy_blocks_in_creative")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let default_speed =
                    t.get("default_mining_speed").and_then(Value::as_f64).unwrap_or(1.0) as f32;
                format!(
                    "Some(ToolInfo {{ rules: &[{rules}], damage_per_block: {per_block}, can_destroy_blocks_in_creative: {creative}, default_mining_speed: {default_speed:?}f32 }})"
                )
            }
        };
        let equip = match c.get("minecraft:equippable").and_then(|e| e["slot"].as_str()) {
            None => "None".to_string(),
            Some(slot) => {
                let variant = match slot {
                    "head" => "Head",
                    "chest" => "Chest",
                    "legs" => "Legs",
                    "feet" => "Feet",
                    "body" => "Body",
                    "saddle" => "Saddle",
                    "offhand" => "Offhand",
                    "mainhand" => "Mainhand",
                    other => bail!("{name}: unknown equipment slot {other}"),
                };
                format!("Some(EquipSlot::{variant})")
            }
        };
        let food = match c.get("minecraft:food") {
            None => "None".to_string(),
            Some(f) => {
                let nutrition = f["nutrition"].as_i64().context("food nutrition")?;
                let saturation = f["saturation"].as_f64().context("food saturation")? as f32;
                let always = f.get("can_always_eat").and_then(Value::as_bool).unwrap_or(false);
                let seconds = c
                    .get("minecraft:consumable")
                    .and_then(|k| k.get("consume_seconds"))
                    .and_then(Value::as_f64)
                    .unwrap_or(1.6);
                let ticks = (seconds * 20.0) as u32;
                format!(
                    "Some(FoodInfo {{ nutrition: {nutrition}, saturation: {saturation:?}f32, can_always_eat: {always}, consume_ticks: {ticks} }})"
                )
            }
        };
        let _ = writeln!(
            out,
            "    ItemInfo {{ max_stack: {max_stack}, max_damage: {max_damage}, tool: {tool}, equip: {equip}, food: {food} }}, // {name}"
        );
    }
    out.push_str("];\n");
    Ok(out)
}
