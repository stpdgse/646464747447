//! Furnaces, blast furnaces and smokers: fuel, cooking progress, and the output they build up.
//!
//! The rules are those of the official server's `AbstractFurnaceBlockEntity`, checked tick by tick
//! against recordings of it (`tests/furnace.rs`). A blast furnace and a smoker (the "fast
//! cooking" blocks) burn their fuel for half as long and cook twice as fast.

use std::collections::BTreeMap;

use ferrite_crafting::{Cooker, Cooking, cook};
use ferrite_data::items::{craft_remainder, item_info};
use ferrite_protocol::packets::play::ItemStack;

pub const INGREDIENT: usize = 0;
pub const FUEL: usize = 1;
pub const RESULT: usize = 2;

/// A furnace's state. Its three slots are the ingredient, the fuel and the result.
#[derive(Clone, Debug, PartialEq)]
pub struct Furnace {
    pub cooker: Cooker,
    pub slots: [ItemStack; 3],
    /// Ticks the current fuel will still burn.
    pub lit: u32,
    /// How long that fuel burns in all.
    pub lit_total: u32,
    /// Ticks spent cooking the current item.
    pub spent: u32,
    /// Ticks the current item takes.
    pub total: u32,
    /// How fast the current fuel cooks (a blast furnace's or smoker's is 2, the rest 1).
    pub speed: f32,
    /// Recipes that made output nobody has collected yet, with how often (for experience).
    pub used: BTreeMap<String, u32>,
}

fn is_empty(s: &ItemStack) -> bool {
    s.count <= 0
}

fn max_stack(item: i32) -> i32 {
    item_info(item).map_or(64, |i| i32::from(i.max_stack))
}

impl Furnace {
    pub fn new(cooker: Cooker) -> Furnace {
        Furnace {
            cooker,
            slots: [ItemStack::default(); 3],
            lit: 0,
            lit_total: 0,
            spent: 0,
            total: 0,
            speed: 1.0,
            used: BTreeMap::new(),
        }
    }

    pub fn is_lit(&self) -> bool {
        self.lit > 0
    }

    /// How long `item` burns in this kind of furnace (0 if it is no fuel).
    pub fn burn_time(&self, item: i32) -> u32 {
        let base = item_info(item).map_or(0, |i| i.fuel);
        if self.cooker.is_fast() { base / 2 } else { base }
    }

    fn recipe(&self) -> Option<&'static Cooking> {
        let ing = &self.slots[INGREDIENT];
        if is_empty(ing) { None } else { cook(self.cooker, ing.item) }
    }

    /// The ticks the item in the ingredient slot takes to cook at `speed`.
    pub fn cook_time_at(&self, speed: f32) -> u32 {
        (self.recipe().map_or(200, |r| r.time) as f32 / speed) as u32
    }

    /// The ticks the item in the ingredient slot takes to cook at the current speed.
    pub fn cook_time(&self) -> u32 {
        self.cook_time_at(self.speed)
    }

    /// Whether the recipe's result fits in the output slot.
    fn can_burn(&self, recipe: Option<&'static Cooking>) -> bool {
        let Some(recipe) = recipe else { return false };
        if recipe.result.is_empty() {
            return false;
        }
        let out = &self.slots[RESULT];
        if is_empty(out) {
            return true;
        }
        out.item == recipe.result.item && out.count < max_stack(out.item).min(64)
    }

    /// Cooks one item: the ingredient becomes output.
    fn burn(&mut self, recipe: &'static Cooking) {
        let out = &mut self.slots[RESULT];
        if is_empty(out) {
            *out = ItemStack { count: recipe.result.count, item: recipe.result.item };
        } else {
            out.count += recipe.result.count;
        }
        // A wet sponge dries out, and the bucket in the fuel slot comes back full.
        let id = |name: &str| ferrite_data::registries::item::by_name.get(name).map(|i| *i as i32);
        if Some(self.slots[INGREDIENT].item) == id("minecraft:wet_sponge")
            && !is_empty(&self.slots[FUEL])
            && Some(self.slots[FUEL].item) == id("minecraft:bucket")
            && let Some(water) = id("minecraft:water_bucket")
        {
            self.slots[FUEL] = ItemStack { count: 1, item: water };
        }
        self.slots[INGREDIENT].count -= 1;
        if is_empty(&self.slots[INGREDIENT]) {
            self.slots[INGREDIENT] = ItemStack::default();
        }
        let name = recipe.name.strip_prefix("minecraft:").unwrap_or(recipe.name);
        *self.used.entry(format!("minecraft:{name}")).or_insert(0) += 1;
    }

    /// Replaces the three slots the way the game's `setItem` does: when the ingredient is
    /// replaced by a different item (or removed), cooking starts over.
    pub fn set_slots(&mut self, new: [ItemStack; 3]) {
        let old = self.slots[INGREDIENT];
        let now = new[INGREDIENT];
        let untouched = (is_empty(&old) && is_empty(&now)) || old == now;
        let same = !is_empty(&now) && !is_empty(&old) && old.item == now.item;
        self.slots = new;
        if !untouched && !same {
            self.total = self.cook_time();
            self.spent = 0;
        }
    }

    /// One game tick. Returns whether the block's `lit` state flipped.
    pub fn tick(&mut self) -> bool {
        let was_lit = self.is_lit();
        if self.lit > 0 {
            self.lit -= 1;
        }
        let has_fuel = !is_empty(&self.slots[FUEL]);
        let has_ingredient = !is_empty(&self.slots[INGREDIENT]);
        if self.is_lit() || (has_fuel && has_ingredient) {
            let recipe = self.recipe();
            if !self.is_lit() && self.can_burn(recipe) {
                self.lit = self.burn_time(self.slots[FUEL].item);
                self.lit_total = self.lit;
                // The fuel decides how fast the item cooks (anything else counts as 1). When
                // that changes the item's total time, the progress made so far is kept as a
                // fraction of it.
                self.speed = if self.is_lit() { self.cooker.speed() as f32 } else { 1.0 };
                let new_total = self.cook_time_at(self.speed);
                if self.total != 0 {
                    self.spent =
                        (self.spent as f32 / self.total as f32 * new_total as f32).ceil() as u32;
                    self.total = new_total;
                }
                if self.is_lit() {
                    let item = self.slots[FUEL].item;
                    self.slots[FUEL].count -= 1;
                    if is_empty(&self.slots[FUEL]) {
                        self.slots[FUEL] = match craft_remainder(item) {
                            Some(rem) => ItemStack { count: 1, item: rem },
                            None => ItemStack::default(),
                        };
                    }
                }
            }
            if self.is_lit() && self.can_burn(recipe) {
                self.spent += 1;
                if self.spent >= self.total {
                    self.spent = 0;
                    self.total = self.cook_time();
                    if let Some(recipe) = recipe {
                        self.burn(recipe);
                    }
                }
            } else {
                self.spent = 0;
            }
        } else if !self.is_lit() && self.spent > 0 {
            self.spent = self.spent.saturating_sub(2).min(self.total);
        }
        was_lit != self.is_lit()
    }

    /// Experience stored for the output collected so far: each use of a recipe is worth its
    /// experience, with the fraction rounded up with that probability (`roll` in `0..1`).
    pub fn take_experience(&mut self, mut roll: impl FnMut() -> f32) -> u32 {
        let mut points = 0;
        for (name, count) in std::mem::take(&mut self.used) {
            let short = name.strip_prefix("minecraft:").unwrap_or(&name);
            let Some(xp) = ferrite_crafting::cooking_experience(short) else { continue };
            let total = count as f32 * xp;
            points += total.floor() as u32;
            let frac = total - total.floor();
            if frac != 0.0 && roll() < frac {
                points += 1;
            }
        }
        points
    }
}
