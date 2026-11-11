//! Loot tables: what a broken block drops.
//!
//! The tables are the game's own (embedded by `ferrite-datagen`); this crate interprets them.
//! It supports every table construct the block tables use: item, alternatives and dynamic
//! entries; the `set_count`, `apply_bonus`, `explosion_decay`, `limit_count`, `copy_components`
//! and `copy_state` functions; and the conditions `all_of`, `any_of`, `inverted`,
//! `survives_explosion`, `match_block`, `match_tool`, `table_bonus`, `random_chance`,
//! `location_check` and `entity_properties`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::Value;

/// A source of randomness; the game seeds one per loot table, here the caller chooses.
pub trait LootRandom {
    /// A float in `0.0..1.0`.
    fn next_f32(&mut self) -> f32;
    /// An integer in `0..bound` (`bound > 0`).
    fn next_below(&mut self, bound: u32) -> u32;
}

/// A small, fast generator (splitmix64) for callers without their own.
#[derive(Clone, Debug)]
pub struct SplitMix(pub u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

impl LootRandom for SplitMix {
    fn next_f32(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn next_below(&mut self, bound: u32) -> u32 {
        ((self.next() >> 32) as u32) % bound.max(1)
    }
}

/// The tool the block is broken with.
#[derive(Clone, Copy, Debug, Default)]
pub struct Tool<'a> {
    /// Item name, e.g. `minecraft:shears`; empty for a bare hand.
    pub item: &'a str,
    /// Enchantments as `(name, level)`.
    pub enchantments: &'a [(&'a str, u8)],
}

impl Tool<'_> {
    fn level(&self, enchantment: &str) -> u8 {
        self.enchantments.iter().find(|(n, _)| *n == enchantment).map_or(0, |(_, l)| *l)
    }
}

/// What the tables may ask about the world and the break.
pub trait LootWorld: std::fmt::Debug {
    /// The block `offset_y` blocks above the broken one, as `(name, properties)`.
    fn block_above(&self, offset_y: i32) -> Option<(String, Vec<(String, String)>)>;
}

/// Everything a block loot table is evaluated against.
#[derive(Debug)]
pub struct LootContext<'a> {
    /// The broken block, `minecraft:oak_slab`.
    pub block: &'a str,
    /// Its properties.
    pub properties: &'a [(&'a str, &'a str)],
    pub tool: Tool<'a>,
    /// Whether something (a player) broke it, as opposed to e.g. a piston or fluid.
    pub has_this_entity: bool,
    /// Explosion radius if the block was destroyed by an explosion.
    pub explosion_radius: Option<f32>,
    pub world: &'a dyn LootWorld,
    /// Set when evaluating an entity's table.
    pub entity: Option<EntityLoot>,
}

/// What an entity's loot table may ask about the kill.
#[derive(Clone, Copy, Debug, Default)]
pub struct EntityLoot {
    pub on_fire: bool,
    pub killed_by_player: bool,
    /// Looting level of the killer's weapon.
    pub looting: u8,
}

/// One stack the table produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drop {
    pub item: String,
    pub count: i32,
}

struct Eval<'a, 'b> {
    ctx: &'a LootContext<'a>,
    rng: &'b mut dyn LootRandom,
}

fn tables() -> &'static Mutex<HashMap<String, Option<&'static Value>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<&'static Value>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Parsed JSON, leaked once per name (tables live for the whole process).
fn cached(key: &str, source: impl FnOnce() -> Option<&'static str>) -> Option<&'static Value> {
    let mut cache = tables().lock().ok()?;
    if let Some(v) = cache.get(key) {
        return *v;
    }
    let parsed = source()
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .map(|v| &*Box::leak(Box::new(v)));
    cache.insert(key.to_string(), parsed);
    parsed
}

/// What breaking the block described by `ctx` drops (empty if it has no table).
pub fn block_drops(ctx: &LootContext<'_>, rng: &mut dyn LootRandom) -> Vec<Drop> {
    let name = ctx.block.trim_start_matches("minecraft:");
    let key = format!("block:{name}");
    let Some(table) = cached(&key, || ferrite_data::loot::block_loot(name)) else {
        return Vec::new();
    };
    let mut ev = Eval { ctx, rng };
    ev.table(table)
}

/// What killing the entity called `entity` (e.g. `minecraft:cow`) drops.
pub fn entity_drops(
    entity: &str,
    state: EntityLoot,
    tool: Tool<'_>,
    world: &dyn LootWorld,
    rng: &mut dyn LootRandom,
) -> Vec<Drop> {
    let name = entity.trim_start_matches("minecraft:");
    let key = format!("entity:{name}");
    let Some(table) = cached(&key, || ferrite_data::loot::entity_loot(name)) else {
        return Vec::new();
    };
    let ctx = LootContext {
        block: "",
        properties: &[],
        tool,
        has_this_entity: true,
        explosion_radius: None,
        world,
        entity: Some(state),
    };
    let mut ev = Eval { ctx: &ctx, rng };
    ev.table(table)
}

fn number(v: &Value) -> Option<f64> {
    v.as_f64()
}

impl Eval<'_, '_> {
    fn table(&mut self, table: &Value) -> Vec<Drop> {
        let mut out = Vec::new();
        for pool in table["pools"].as_array().map(Vec::as_slice).unwrap_or_default() {
            if !self.conditions(pool.get("condition")) {
                continue;
            }
            let rolls = self.int(pool.get("rolls"), 1);
            for _ in 0..rolls.max(0) {
                let mut choices = Vec::new();
                for entry in pool["entries"].as_array().map(Vec::as_slice).unwrap_or_default() {
                    self.expand(entry, &mut choices);
                }
                if let Some(entry) = self.pick(&choices) {
                    out.extend(self.run_entry(entry));
                }
            }
        }
        if let Some(modifier) = table.get("modifier") {
            out = out.into_iter().filter_map(|d| self.apply_all(modifier, d)).collect();
        }
        out.retain(|d| d.count > 0);
        out
    }

    /// Collects the entries that could be chosen from `entry` (alternatives yield their first
    /// passing child).
    fn expand<'t>(&mut self, entry: &'t Value, out: &mut Vec<&'t Value>) -> bool {
        if !self.conditions(entry.get("condition")) {
            return false;
        }
        match entry["type"].as_str().unwrap_or("") {
            "minecraft:alternatives" => {
                for child in entry["children"].as_array().map(Vec::as_slice).unwrap_or_default() {
                    if self.expand(child, out) {
                        return true;
                    }
                }
                false
            }
            "minecraft:empty" => {
                out.push(entry);
                true
            }
            _ => {
                out.push(entry);
                true
            }
        }
    }

    fn pick<'t>(&mut self, choices: &[&'t Value]) -> Option<&'t Value> {
        let weights: Vec<u32> = choices
            .iter()
            .map(|e| e.get("weight").and_then(Value::as_u64).unwrap_or(1) as u32)
            .collect();
        let total: u32 = weights.iter().sum();
        if total == 0 {
            return None;
        }
        let mut roll = self.rng.next_below(total);
        for (e, w) in choices.iter().zip(weights) {
            if roll < w {
                return Some(e);
            }
            roll -= w;
        }
        None
    }

    fn run_entry(&mut self, entry: &Value) -> Vec<Drop> {
        match entry["type"].as_str().unwrap_or("") {
            "minecraft:item" => {
                let Some(name) = entry["name"].as_str() else { return Vec::new() };
                let drop = Drop { item: name.to_string(), count: 1 };
                match entry.get("modifier") {
                    Some(m) => self.apply_all(m, drop).into_iter().collect(),
                    None => vec![drop],
                }
            }
            "minecraft:loot_table" => {
                // Another table, by name (sheep wool, for instance).
                let Some(id) = entry["value"].as_str() else { return Vec::new() };
                let short = id.trim_start_matches("minecraft:");
                let key = format!("table:{short}");
                let found = cached(&key, || {
                    ferrite_data::loot::ENTITY_LOOT
                        .binary_search_by(|(n, _)| (*n).cmp(format!("minecraft:{short}").as_str()))
                        .ok()
                        .map(|i| ferrite_data::loot::ENTITY_LOOT[i].1)
                });
                let mut out = found.map(|t| self.table(t)).unwrap_or_default();
                if let Some(m) = entry.get("modifier") {
                    out = out.into_iter().filter_map(|d| self.apply_all(m, d)).collect();
                }
                out
            }
            // Contents of containers are copied from the block entity, which has none here.
            _ => Vec::new(),
        }
    }

    // ---- functions ----

    fn apply_all(&mut self, modifier: &Value, mut drop: Drop) -> Option<Drop> {
        let list: Vec<&Value> = match modifier {
            Value::Array(a) => a.iter().collect(),
            other => vec![other],
        };
        for f in list {
            if !self.conditions(f.get("condition")) {
                continue;
            }
            match f["type"].as_str().unwrap_or("") {
                "minecraft:set_count" => {
                    let n = self.int(f.get("count"), 1);
                    if f.get("add").and_then(Value::as_bool).unwrap_or(false) {
                        drop.count += n;
                    } else {
                        drop.count = n;
                    }
                }
                "minecraft:explosion_decay" => {
                    if let Some(radius) = self.ctx.explosion_radius {
                        let chance = 1.0 / radius;
                        let kept =
                            (0..drop.count).filter(|_| self.rng.next_f32() <= chance).count();
                        drop.count = kept as i32;
                    }
                }
                "minecraft:apply_bonus" => {
                    let level =
                        i32::from(self.ctx.tool.level(f["enchantment"].as_str().unwrap_or("")));
                    drop.count = self.bonus(f, level, drop.count);
                }
                "minecraft:limit_count" => {
                    let lim = &f["limit"];
                    let (lo, hi) = match lim {
                        Value::Number(_) => {
                            (number(lim).map(|x| x as i32), number(lim).map(|x| x as i32))
                        }
                        _ => (
                            lim.get("min").and_then(number).map(|x| x as i32),
                            lim.get("max").and_then(number).map(|x| x as i32),
                        ),
                    };
                    if let Some(hi) = hi {
                        drop.count = drop.count.min(hi);
                    }
                    if let Some(lo) = lo {
                        drop.count = drop.count.max(lo);
                    }
                }
                "minecraft:enchanted_count_increase" => {
                    let level = i32::from(self.ctx.entity.map_or(0, |e| e.looting));
                    if level > 0 {
                        let mut extra = 0;
                        for _ in 0..level {
                            extra += self.int(f.get("count"), 0);
                        }
                        if let Some(limit) = f.get("limit").and_then(Value::as_i64) {
                            drop.count = (drop.count + extra).min(limit as i32).max(drop.count);
                        } else {
                            drop.count += extra;
                        }
                    }
                }
                "minecraft:furnace_smelt" => {
                    if let Some(cooked) = smelts_to(&drop.item) {
                        drop.item = cooked;
                    }
                }
                // Item components are not modelled yet: stacks carry only item and count.
                "minecraft:copy_components" | "minecraft:copy_state" => {}
                _ => {}
            }
        }
        (drop.count > 0).then_some(drop)
    }

    fn bonus(&mut self, f: &Value, level: i32, count: i32) -> i32 {
        match f["formula"].as_str().unwrap_or("") {
            "minecraft:ore_drops" => {
                if level > 0 {
                    let j = (self.rng.next_below(level as u32 + 2) as i32 - 1).max(0);
                    count * (j + 1)
                } else {
                    count
                }
            }
            "minecraft:uniform_bonus_count" => {
                let mult = f["parameters"]["bonusMultiplier"].as_i64().unwrap_or(1) as i32;
                count + self.rng.next_below((mult * level + 1) as u32) as i32
            }
            "minecraft:binomial_with_bonus_count" => {
                let extra = f["parameters"]["extra"].as_i64().unwrap_or(0) as i32;
                let p = f["parameters"]["probability"].as_f64().unwrap_or(0.5) as f32;
                let mut c = count;
                for _ in 0..(level + extra).max(0) {
                    if self.rng.next_f32() < p {
                        c += 1;
                    }
                }
                c
            }
            _ => count,
        }
    }

    // ---- number providers ----

    fn int(&mut self, v: Option<&Value>, default: i32) -> i32 {
        let Some(v) = v else { return default };
        if let Some(n) = number(v) {
            return n as i32;
        }
        match v["type"].as_str().unwrap_or("") {
            "minecraft:uniform" | "uniform" => {
                let lo = self.int(v.get("min"), 0);
                let hi = self.int(v.get("max"), 0);
                if hi < lo { lo } else { lo + self.rng.next_below((hi - lo + 1) as u32) as i32 }
            }
            "minecraft:binomial" | "binomial" => {
                let n = self.int(v.get("n"), 0);
                let p = self.float(v.get("p"), 0.5);
                (0..n.max(0)).filter(|_| self.rng.next_f32() < p).count() as i32
            }
            "minecraft:constant" | "constant" => self.int(v.get("value"), default),
            _ => default,
        }
    }

    fn float(&mut self, v: Option<&Value>, default: f32) -> f32 {
        let Some(v) = v else { return default };
        if let Some(n) = number(v) {
            return n as f32;
        }
        match v["type"].as_str().unwrap_or("") {
            "minecraft:uniform" | "uniform" => {
                let lo = self.float(v.get("min"), 0.0);
                let hi = self.float(v.get("max"), 0.0);
                lo + self.rng.next_f32() * (hi - lo)
            }
            "minecraft:constant" | "constant" => self.float(v.get("value"), default),
            _ => default,
        }
    }

    // ---- conditions ----

    /// `None` is "no condition"; a string names a predicate; an object is an inline one.
    fn conditions(&mut self, c: Option<&Value>) -> bool {
        match c {
            None | Some(Value::Null) => true,
            Some(Value::Array(a)) => a.iter().all(|x| self.condition(x)),
            Some(other) => self.condition(other),
        }
    }

    fn condition(&mut self, c: &Value) -> bool {
        if let Some(name) = c.as_str() {
            return match cached(&format!("pred:{name}"), || ferrite_data::loot::predicate(name)) {
                Some(p) => self.condition(p),
                None => false,
            };
        }
        match c["type"].as_str().unwrap_or("") {
            "minecraft:all_of" => c["terms"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .all(|t| self.condition(t)),
            "minecraft:any_of" => c["terms"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .any(|t| self.condition(t)),
            "minecraft:inverted" => !self.condition(&c["term"]),
            "minecraft:survives_explosion" => match self.ctx.explosion_radius {
                Some(r) => self.rng.next_f32() <= 1.0 / r,
                None => true,
            },
            "minecraft:random_chance" => {
                let chance = self.float(c.get("chance"), 0.0);
                self.rng.next_f32() < chance
            }
            "minecraft:table_bonus" => {
                let level =
                    usize::from(self.ctx.tool.level(c["enchantment"].as_str().unwrap_or("")));
                let chances: Vec<f32> = c["chances"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect())
                    .unwrap_or_default();
                let Some(&chance) = chances.get(level.min(chances.len().saturating_sub(1))) else {
                    return false;
                };
                self.rng.next_f32() < chance
            }
            "minecraft:match_block" => {
                self.block_matches(&c["blocks"], &c["state"], self.ctx.block, &|k| {
                    self.ctx.properties.iter().find(|(n, _)| *n == k).map(|(_, v)| (*v).to_string())
                })
            }
            "minecraft:match_tool" => self.tool_matches(&c["predicate"]),
            "minecraft:location_check" => {
                let offset = c["offsetY"].as_i64().unwrap_or(0) as i32;
                let Some((name, props)) = self.ctx.world.block_above(offset) else { return false };
                let block = &c["predicate"]["block"];
                self.block_matches(&block["blocks"], &block["state"], &name, &|k| {
                    props.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone())
                })
            }
            "minecraft:entity_properties" => {
                let Some(e) = self.ctx.entity else { return self.ctx.has_this_entity };
                // The only things the tables ask: is it burning, what does the attacker hold.
                if c["entity"].as_str() == Some("this") {
                    return match c["predicate"]["minecraft:flags"]["is_on_fire"].as_bool() {
                        Some(want) => e.on_fire == want,
                        None => c["predicate"].as_object().is_none_or(|o| o.is_empty()),
                    };
                }
                // The killer's weapon: no components modelled, so nothing enchanted.
                c["predicate"].as_object().is_none_or(|o| o.is_empty())
            }
            "minecraft:killed_by_player" => self.ctx.entity.is_some_and(|e| e.killed_by_player),
            "minecraft:random_chance_with_enchanted_bonus" => {
                let level = f64::from(self.ctx.entity.map_or(0, |e| e.looting));
                let chance = if level > 0.0 {
                    let ec = &c["enchanted_chance"];
                    ec["base"].as_f64().unwrap_or(0.0)
                        + ec["per_level_above_first"].as_f64().unwrap_or(0.0) * (level - 1.0)
                } else {
                    self.float(c.get("unenchanted_chance"), 0.0).into()
                };
                f64::from(self.rng.next_f32()) < chance
            }
            "minecraft:damage_source_properties" => false,
            _ => false,
        }
    }

    fn block_matches(
        &self,
        blocks: &Value,
        state: &Value,
        actual: &str,
        prop: &dyn Fn(&str) -> Option<String>,
    ) -> bool {
        let names: Vec<&str> = match blocks {
            Value::String(s) => vec![s.as_str()],
            Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        if !names.is_empty() && !names.contains(&actual) {
            return false;
        }
        let Some(obj) = state.as_object() else { return true };
        obj.iter().all(|(k, want)| {
            let Some(have) = prop(k) else { return false };
            match want {
                Value::String(s) => *s == have,
                Value::Bool(b) => have == b.to_string(),
                Value::Object(range) => {
                    let n: i64 = have.parse().unwrap_or(i64::MIN);
                    range.get("min").and_then(Value::as_i64).is_none_or(|m| n >= m)
                        && range.get("max").and_then(Value::as_i64).is_none_or(|m| n <= m)
                }
                _ => false,
            }
        })
    }

    fn tool_matches(&self, predicate: &Value) -> bool {
        if let Some(items) = predicate.get("items") {
            let ok = match items {
                Value::String(s) => s == self.ctx.tool.item,
                Value::Array(a) => a.iter().any(|i| i.as_str() == Some(self.ctx.tool.item)),
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        let Some(list) = predicate["predicates"]["minecraft:enchantments"].as_array() else {
            return true;
        };
        list.iter().all(|e| {
            let name = e["enchantments"].as_str().unwrap_or("");
            let level = i64::from(self.ctx.tool.level(name));
            e["levels"]["min"].as_i64().is_none_or(|m| level >= m)
                && e["levels"]["max"].as_i64().is_none_or(|m| level <= m)
        })
    }
}

/// What an item smelts into (the first smelting recipe that takes it).
fn smelts_to(item: &str) -> Option<String> {
    for (_, json) in ferrite_data::loot::RECIPES.iter() {
        if !json.contains("\"minecraft:smelting\"") || !json.contains(item) {
            continue;
        }
        let v: Value = serde_json::from_str(json).ok()?;
        let ing = &v["ingredient"];
        let hit = match ing {
            Value::String(s) => s == item,
            Value::Array(a) => a.iter().any(|x| x.as_str() == Some(item)),
            _ => false,
        };
        if hit {
            return v["result"]["id"].as_str().map(str::to_string);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct NoWorld;
    impl LootWorld for NoWorld {
        fn block_above(&self, _: i32) -> Option<(String, Vec<(String, String)>)> {
            None
        }
    }

    fn drops(block: &str, props: &[(&str, &str)], tool: Tool<'_>, seed: u64) -> Vec<Drop> {
        let ctx = LootContext {
            block,
            properties: props,
            tool,
            has_this_entity: true,
            explosion_radius: None,
            world: &NoWorld,
            entity: None,
        };
        block_drops(&ctx, &mut SplitMix(seed))
    }

    fn one(item: &str, count: i32) -> Vec<Drop> {
        vec![Drop { item: item.to_string(), count }]
    }

    #[test]
    fn stone_drops_cobblestone_or_itself_with_silk_touch() {
        assert_eq!(
            drops("minecraft:stone", &[], Tool::default(), 1),
            one("minecraft:cobblestone", 1)
        );
        let silk = Tool {
            item: "minecraft:diamond_pickaxe",
            enchantments: &[("minecraft:silk_touch", 1)],
        };
        assert_eq!(drops("minecraft:stone", &[], silk, 1), one("minecraft:stone", 1));
    }

    #[test]
    fn double_slabs_drop_two() {
        let single = drops("minecraft:oak_slab", &[("type", "bottom")], Tool::default(), 1);
        let double = drops("minecraft:oak_slab", &[("type", "double")], Tool::default(), 1);
        assert_eq!(single, one("minecraft:oak_slab", 1));
        assert_eq!(double, one("minecraft:oak_slab", 2));
    }

    #[test]
    fn crops_drop_seeds_until_ripe() {
        assert_eq!(
            drops("minecraft:wheat", &[("age", "3")], Tool::default(), 1),
            one("minecraft:wheat_seeds", 1)
        );
        let ripe = drops("minecraft:wheat", &[("age", "7")], Tool::default(), 1);
        assert!(ripe.iter().any(|d| d.item == "minecraft:wheat" && d.count == 1));
        assert!(ripe.iter().any(|d| d.item == "minecraft:wheat_seeds"));
    }

    #[test]
    fn fortune_raises_ore_drops() {
        let mut total = [0i64; 2];
        for seed in 0..4000 {
            for (i, level) in [0u8, 3].into_iter().enumerate() {
                let tool = Tool {
                    item: "minecraft:iron_pickaxe",
                    enchantments: &[("minecraft:fortune", level)],
                };
                total[i] += i64::from(drops("minecraft:diamond_ore", &[], tool, seed)[0].count);
            }
        }
        assert_eq!(total[0], 4000);
        // Fortune III averages (1 + 2/5 * ... ) = 2.2 per ore in the long run.
        let mean = total[1] as f64 / 4000.0;
        assert!((mean - 2.2).abs() < 0.1, "mean {mean}");
    }

    #[test]
    fn leaves_need_shears_or_silk_touch_to_drop_themselves() {
        let shears = Tool { item: "minecraft:shears", enchantments: &[] };
        assert_eq!(drops("minecraft:oak_leaves", &[], shears, 1), one("minecraft:oak_leaves", 1));
        let by_hand = drops("minecraft:oak_leaves", &[], Tool::default(), 1);
        assert!(by_hand.iter().all(|d| d.item != "minecraft:oak_leaves"));
    }

    #[test]
    fn sapling_chance_matches_the_table() {
        let n = 20_000;
        let hits = (0..n)
            .filter(|s| {
                drops("minecraft:oak_leaves", &[], Tool::default(), *s)
                    .iter()
                    .any(|d| d.item == "minecraft:oak_sapling")
            })
            .count();
        let p = hits as f64 / f64::from(n as i32);
        assert!((p - 0.05).abs() < 0.01, "sapling rate {p}");
    }

    #[test]
    fn explosions_destroy_some_drops() {
        let ctx = LootContext {
            block: "minecraft:stone",
            properties: &[],
            tool: Tool::default(),
            has_this_entity: false,
            explosion_radius: Some(4.0),
            world: &NoWorld,
            entity: None,
        };
        let kept = (0..4000).filter(|s| !block_drops(&ctx, &mut SplitMix(*s)).is_empty()).count();
        let p = kept as f64 / 4000.0;
        assert!((p - 0.25).abs() < 0.03, "survival rate {p}");
    }

    #[test]
    fn mobs_drop_what_their_tables_say() {
        let state = EntityLoot { on_fire: false, killed_by_player: true, looting: 0 };
        // A cow leaves 0-2 leather and 1-3 raw beef (cooked when burning).
        let mut beef = 0;
        for seed in 0..500 {
            let drops = entity_drops(
                "minecraft:cow",
                state,
                Tool::default(),
                &NoWorld,
                &mut SplitMix(seed),
            );
            assert!(
                drops.iter().all(|d| d.item == "minecraft:leather" || d.item == "minecraft:beef"),
                "{drops:?}"
            );
            beef +=
                drops.iter().filter(|d| d.item == "minecraft:beef").map(|d| d.count).sum::<i32>();
        }
        assert!((500..=1500).contains(&beef), "beef {beef}");
        let burning = EntityLoot { on_fire: true, ..state };
        let cooked =
            entity_drops("minecraft:cow", burning, Tool::default(), &NoWorld, &mut SplitMix(3));
        assert!(cooked.iter().any(|d| d.item == "minecraft:cooked_beef"), "{cooked:?}");
        // Zombie rare drops need the player to have killed it.
        let by_player = (0..4000).any(|s| {
            entity_drops("minecraft:zombie", state, Tool::default(), &NoWorld, &mut SplitMix(s))
                .iter()
                .any(|d| d.item == "minecraft:iron_ingot")
        });
        let by_nature = (0..4000).any(|s| {
            entity_drops(
                "minecraft:zombie",
                EntityLoot::default(),
                Tool::default(),
                &NoWorld,
                &mut SplitMix(s),
            )
            .iter()
            .any(|d| d.item == "minecraft:iron_ingot")
        });
        assert!(by_player && !by_nature);
    }

    #[test]
    fn every_block_table_evaluates() {
        for (name, _) in ferrite_data::loot::BLOCK_LOOT.iter() {
            let block = name.replace("blocks/", "");
            let _ = drops(&block, &[], Tool::default(), 7);
        }
    }
}
