//! Crafting: which recipe a grid of items makes.
//!
//! The recipes are the game's own (embedded by `ferrite-datagen`). This crate matches shaped,
//! shapeless, transmute and dye recipes the way the game does: the grid is trimmed to the box
//! around its items, shaped recipes are tried in both orientations, shapeless ones need a
//! one-to-one matching of ingredients to items.

use std::collections::HashMap;
use std::sync::OnceLock;

use ferrite_data::configuration::TAG_REGISTRIES;
use serde_json::Value;

/// An item stack as the crafting grid sees it: item id and count (0 = empty).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stack {
    pub item: i32,
    pub count: i32,
}

impl Stack {
    pub fn is_empty(&self) -> bool {
        self.count <= 0
    }
}

/// What something that matches an ingredient may be: one of these item ids.
#[derive(Clone, Debug, Default)]
struct Ingredient(Vec<i32>);

impl Ingredient {
    fn matches(&self, item: i32) -> bool {
        self.0.contains(&item)
    }
}

#[derive(Clone, Debug)]
enum Kind {
    Shaped {
        width: usize,
        height: usize,
        cells: Vec<Option<Ingredient>>,
        mirrored_too: bool,
    },
    Shapeless {
        ingredients: Vec<Ingredient>,
    },
    /// One `input` plus `material_count` of the `material`; the result copies the input's stack
    /// (with the item replaced).
    Transmute {
        input: Ingredient,
        material: Ingredient,
        material_count: usize,
    },
    /// A dye applied to any item of the `target` set.
    Dye {
        dye: Ingredient,
        target: Ingredient,
    },
}

#[derive(Clone, Debug)]
pub struct Recipe {
    pub name: &'static str,
    kind: Kind,
    pub result: Stack,
}

fn item_tags() -> &'static HashMap<&'static str, &'static [u16]> {
    static TAGS: OnceLock<HashMap<&'static str, &'static [u16]>> = OnceLock::new();
    TAGS.get_or_init(|| {
        TAG_REGISTRIES
            .iter()
            .find(|r| r.registry == "minecraft:item")
            .map(|r| r.tags.iter().map(|t| (t.name, t.entries)).collect())
            .unwrap_or_default()
    })
}

fn item_id(name: &str) -> Option<i32> {
    ferrite_data::registries::item::by_name.get(name).map(|id| *id as i32)
}

fn ingredient(v: &Value) -> Option<Ingredient> {
    let mut ids = Vec::new();
    let mut add = |s: &str| -> Option<()> {
        if let Some(tag) = s.strip_prefix('#') {
            ids.extend(item_tags().get(tag)?.iter().map(|i| i32::from(*i)));
        } else {
            ids.push(item_id(s)?);
        }
        Some(())
    };
    match v {
        Value::String(s) => add(s)?,
        Value::Array(a) => {
            for x in a {
                add(x.as_str()?)?;
            }
        }
        _ => return None,
    }
    Some(Ingredient(ids))
}

fn result(v: &Value) -> Option<Stack> {
    let item = item_id(v["id"].as_str()?)?;
    Some(Stack { item, count: v.get("count").and_then(Value::as_i64).unwrap_or(1) as i32 })
}

fn parse(name: &'static str, json: &str) -> Option<Recipe> {
    let v: Value = serde_json::from_str(json).ok()?;
    let kind = match v["type"].as_str()? {
        "minecraft:crafting_shaped" => {
            let pattern: Vec<&str> =
                v["pattern"].as_array()?.iter().filter_map(Value::as_str).collect();
            let height = pattern.len();
            let width = pattern.first()?.chars().count();
            let key = v["key"].as_object()?;
            let mut cells = Vec::with_capacity(width * height);
            for row in &pattern {
                for ch in row.chars() {
                    cells.push(if ch == ' ' {
                        None
                    } else {
                        Some(ingredient(key.get(&ch.to_string())?)?)
                    });
                }
            }
            Kind::Shaped { width, height, cells, mirrored_too: true }
        }
        "minecraft:crafting_shapeless" => Kind::Shapeless {
            ingredients: v["ingredients"]
                .as_array()?
                .iter()
                .map(ingredient)
                .collect::<Option<_>>()?,
        },
        "minecraft:crafting_transmute" => Kind::Transmute {
            input: ingredient(&v["input"])?,
            material: ingredient(&v["material"])?,
            material_count: v.get("material_count").and_then(Value::as_u64).unwrap_or(1) as usize,
        },
        "minecraft:crafting_dye" => {
            Kind::Dye { dye: ingredient(&v["dye"])?, target: ingredient(&v["target"])? }
        }
        _ => return None,
    };
    let result = match &kind {
        Kind::Dye { .. } | Kind::Transmute { .. } => result(&v["result"]).unwrap_or_default(),
        _ => result(&v["result"])?,
    };
    Some(Recipe { name, kind, result })
}

/// Every crafting recipe, in registry order.
pub fn recipes() -> &'static [Recipe] {
    static ALL: OnceLock<Vec<Recipe>> = OnceLock::new();
    ALL.get_or_init(|| {
        ferrite_data::loot::RECIPES
            .iter()
            .filter_map(|(name, json)| {
                if !json.contains("\"minecraft:crafting_") {
                    return None;
                }
                parse(name, json)
            })
            .collect()
    })
}

/// The grid cropped to the smallest box containing every item.
fn trimmed(grid: &[Stack], width: usize, height: usize) -> (Vec<Stack>, usize, usize) {
    let (mut x0, mut y0, mut x1, mut y1) = (width, height, 0, 0);
    for y in 0..height {
        for x in 0..width {
            if !grid[y * width + x].is_empty() {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    if x1 == 0 {
        return (Vec::new(), 0, 0);
    }
    let mut out = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            out.push(grid[y * width + x]);
        }
    }
    (out, x1 - x0, y1 - y0)
}

impl Recipe {
    fn shaped_matches(
        cells: &[Option<Ingredient>],
        w: usize,
        h: usize,
        items: &[Stack],
        mirror: bool,
    ) -> bool {
        for y in 0..h {
            for x in 0..w {
                let cell = &cells[y * w + if mirror { w - 1 - x } else { x }];
                let item = items[y * w + x];
                match cell {
                    None if !item.is_empty() => return false,
                    Some(ing) if item.is_empty() || !ing.matches(item.item) => return false,
                    _ => {}
                }
            }
        }
        true
    }

    /// The stack this recipe makes from the grid, if it matches.
    pub fn matches(&self, grid: &[Stack], width: usize, height: usize) -> Option<Stack> {
        let (items, w, h) = trimmed(grid, width, height);
        if items.is_empty() {
            return None;
        }
        match &self.kind {
            Kind::Shaped { width: pw, height: ph, cells, mirrored_too } => {
                if w != *pw || h != *ph {
                    return None;
                }
                (Self::shaped_matches(cells, w, h, &items, false)
                    || (*mirrored_too && Self::shaped_matches(cells, w, h, &items, true)))
                .then_some(self.result)
            }
            Kind::Shapeless { ingredients } => {
                let present: Vec<i32> =
                    items.iter().filter(|s| !s.is_empty()).map(|s| s.item).collect();
                (present.len() == ingredients.len()
                    && assign(ingredients, &present, 0, &mut vec![false; present.len()]))
                .then_some(self.result)
            }
            Kind::Transmute { input, material, material_count } => {
                let present: Vec<Stack> = items.iter().filter(|s| !s.is_empty()).copied().collect();
                if present.len() != 1 + material_count {
                    return None;
                }
                for (i, base) in present.iter().enumerate() {
                    if input.matches(base.item)
                        && present
                            .iter()
                            .enumerate()
                            .all(|(j, s)| j == i || material.matches(s.item))
                    {
                        // The result takes the input's place: same count, new item.
                        return Some(Stack {
                            item: self.result.item,
                            count: self.result.count.max(1),
                        });
                    }
                }
                None
            }
            Kind::Dye { dye, target } => {
                let present: Vec<i32> =
                    items.iter().filter(|s| !s.is_empty()).map(|s| s.item).collect();
                let dyes = present.iter().filter(|i| dye.matches(**i)).count();
                let targets = present.iter().filter(|i| target.matches(**i)).count();
                (present.len() >= 2 && dyes >= 1 && targets == 1 && dyes + targets == present.len())
                    .then_some(self.result)
            }
        }
    }
}

/// Whether each ingredient can be given its own distinct item (bipartite matching).
fn assign(ingredients: &[Ingredient], items: &[i32], next: usize, used: &mut Vec<bool>) -> bool {
    let Some(ing) = ingredients.get(next) else { return true };
    for (i, item) in items.iter().enumerate() {
        if !used[i] && ing.matches(*item) {
            used[i] = true;
            if assign(ingredients, items, next + 1, used) {
                return true;
            }
            used[i] = false;
        }
    }
    false
}

/// The first recipe (in registry order) the grid makes, and its result.
pub fn craft(grid: &[Stack], width: usize, height: usize) -> Option<(&'static Recipe, Stack)> {
    recipes().iter().find_map(|r| r.matches(grid, width, height).map(|s| (r, s)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(name: &str) -> Stack {
        Stack { item: item_id(name).unwrap_or_else(|| panic!("{name}")), count: 1 }
    }

    const E: Stack = Stack { item: 0, count: 0 };

    #[test]
    fn planks_make_sticks_in_either_column_of_a_2x2() {
        let p = s("minecraft:oak_planks");
        for grid in [[p, E, p, E], [E, p, E, p]] {
            let (_, out) = craft(&grid, 2, 2).expect("sticks");
            assert_eq!((out.item, out.count), (item_id("minecraft:stick").unwrap(), 4));
        }
    }

    #[test]
    fn a_log_makes_four_planks_of_its_own_wood() {
        let (_, out) = craft(&[s("minecraft:birch_log"), E, E, E], 2, 2).unwrap();
        assert_eq!((out.item, out.count), (item_id("minecraft:birch_planks").unwrap(), 4));
    }

    #[test]
    fn four_planks_make_a_crafting_table_only_when_all_are_planks() {
        let p = s("minecraft:oak_planks");
        let (_, out) = craft(&[p, p, p, p], 2, 2).unwrap();
        assert_eq!(out.item, item_id("minecraft:crafting_table").unwrap());
        assert!(craft(&[p, p, p, s("minecraft:stone")], 2, 2).is_none());
    }

    #[test]
    fn shaped_recipes_work_mirrored_and_in_a_bigger_grid() {
        // A sword: two stone over a stick, anywhere in the 3x3, even shifted.
        let (st, k) = (s("minecraft:cobblestone"), s("minecraft:stick"));
        let grid = [E, st, E, E, st, E, E, k, E];
        let (_, out) = craft(&grid, 3, 3).unwrap();
        assert_eq!(out.item, item_id("minecraft:stone_sword").unwrap());
        let shifted = [st, E, E, st, E, E, k, E, E];
        assert!(craft(&shifted, 3, 3).is_some());
    }

    #[test]
    fn shapeless_recipes_match_in_any_arrangement() {
        let (a, b) = (s("minecraft:flint"), s("minecraft:iron_ingot"));
        let one = craft(&[a, b, E, E], 2, 2).map(|(_, o)| o.item);
        let two = craft(&[E, b, a, E], 2, 2).map(|(_, o)| o.item);
        assert_eq!(one, two);
        assert_eq!(one, Some(item_id("minecraft:flint_and_steel").unwrap()));
    }

    #[test]
    fn every_recipe_parses() {
        let n = recipes().len();
        assert!(n > 1000, "only {n} crafting recipes parsed");
    }
}

/// The blocks that cook one item at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cooker {
    Furnace,
    BlastFurnace,
    Smoker,
    Campfire,
}

impl Cooker {
    fn recipe_type(self) -> &'static str {
        match self {
            Cooker::Furnace => "minecraft:smelting",
            Cooker::BlastFurnace => "minecraft:blasting",
            Cooker::Smoker => "minecraft:smoking",
            Cooker::Campfire => "minecraft:campfire_cooking",
        }
    }

    /// Blast furnaces and smokers cook twice as fast and burn their fuel in half the time
    /// (`minecraft:block/fast_cooking`).
    pub fn is_fast(self) -> bool {
        matches!(self, Cooker::BlastFurnace | Cooker::Smoker)
    }

    /// How much cooking progress one tick adds.
    pub fn speed(self) -> u32 {
        if self.is_fast() { 2 } else { 1 }
    }
}

/// A smelting, blasting, smoking or campfire recipe.
#[derive(Clone, Debug)]
pub struct Cooking {
    pub name: &'static str,
    kind: Cooker,
    ingredient: Ingredient,
    pub result: Stack,
    /// Experience points for the recipe's result.
    pub experience: f32,
    /// Ticks of cooking in an ordinary furnace.
    pub time: u32,
}

fn cooking_recipes() -> &'static [Cooking] {
    static ALL: OnceLock<Vec<Cooking>> = OnceLock::new();
    ALL.get_or_init(|| {
        ferrite_data::loot::RECIPES
            .iter()
            .filter_map(|(name, json)| {
                let v: Value = serde_json::from_str(json).ok()?;
                let ty = v["type"].as_str()?;
                let kind =
                    [Cooker::Furnace, Cooker::BlastFurnace, Cooker::Smoker, Cooker::Campfire]
                        .into_iter()
                        .find(|k| k.recipe_type() == ty)?;
                let default_time = match kind {
                    Cooker::Furnace => 200,
                    Cooker::BlastFurnace | Cooker::Smoker => 100,
                    Cooker::Campfire => 600,
                };
                Some(Cooking {
                    name,
                    kind,
                    ingredient: ingredient(&v["ingredient"])?,
                    result: result(&v["result"])?,
                    experience: v.get("experience").and_then(Value::as_f64).unwrap_or(0.0) as f32,
                    time: v.get("cookingtime").and_then(Value::as_u64).unwrap_or(default_time)
                        as u32,
                })
            })
            .collect()
    })
}

/// The recipe `cooker` follows for `item`, if there is one (the first in registry order).
pub fn cook(cooker: Cooker, item: i32) -> Option<&'static Cooking> {
    cooking_recipes().iter().find(|r| r.kind == cooker && r.ingredient.matches(item))
}

/// The experience of the cooking recipe called `name` (without the namespace).
pub fn cooking_experience(name: &str) -> Option<f32> {
    cooking_recipes()
        .iter()
        .find(|r| r.name == name || r.name.strip_prefix("minecraft:") == Some(name))
        .map(|r| r.experience)
}

#[cfg(test)]
mod cooking_tests {
    use super::*;

    fn id(name: &str) -> i32 {
        item_id(name).expect("an item")
    }

    #[test]
    fn furnaces_follow_their_own_recipe_types() {
        let smelt = cook(Cooker::Furnace, id("minecraft:raw_iron")).expect("smelts");
        assert_eq!(smelt.result, Stack { item: id("minecraft:iron_ingot"), count: 1 });
        assert_eq!((smelt.time, smelt.experience), (200, 0.7));
        let blast = cook(Cooker::BlastFurnace, id("minecraft:raw_iron")).expect("blasts");
        // The recipe says 200 ticks too; the blast furnace's speed multiplier of 2 halves it.
        assert_eq!(blast.time, 200);
        // Food is for the smoker and the campfire, not the blast furnace.
        assert!(cook(Cooker::BlastFurnace, id("minecraft:beef")).is_none());
        let steak = cook(Cooker::Smoker, id("minecraft:beef")).expect("smokes");
        assert_eq!(steak.result.item, id("minecraft:cooked_beef"));
        assert_eq!(cook(Cooker::Campfire, id("minecraft:beef")).expect("campfire").time, 600);
        assert!(cook(Cooker::Furnace, id("minecraft:stick")).is_none());
    }

    #[test]
    fn tags_work_as_ingredients() {
        // Any log makes charcoal.
        for log in ["minecraft:oak_log", "minecraft:birch_log"] {
            assert_eq!(
                cook(Cooker::Furnace, id(log)).expect("a log").result.item,
                id("minecraft:charcoal")
            );
        }
    }
}
