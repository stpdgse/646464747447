//! Furnaces in the world: ticking them, keeping open windows in step, and the experience they
//! pay out when their output is taken.

use ferrite_crafting::Cooker;

use super::containers::Source;
use super::*;
use crate::furnace::Furnace;
use crate::persist::{FurnaceSave, LoadedContainers};

/// Which kind of cooker a block name is.
pub(super) fn cooker_of(name: &str) -> Option<Cooker> {
    match name {
        "minecraft:furnace" => Some(Cooker::Furnace),
        "minecraft:blast_furnace" => Some(Cooker::BlastFurnace),
        "minecraft:smoker" => Some(Cooker::Smoker),
        _ => None,
    }
}

pub(super) fn furnace_to_save(f: &Furnace) -> FurnaceSave {
    FurnaceSave {
        slots: f.slots,
        lit: f.lit,
        lit_total: f.lit_total,
        spent: f.spent,
        total: f.total,
        speed: f.speed,
        used: f.used.iter().map(|(k, v)| (k.clone(), *v)).collect(),
    }
}

fn furnace_from_save(cooker: Cooker, s: FurnaceSave) -> Furnace {
    let mut f = Furnace::new(cooker);
    f.slots = s.slots;
    f.lit = s.lit;
    f.lit_total = s.lit_total;
    f.spent = s.spent;
    f.total = s.total;
    f.speed = s.speed;
    f.used = s.used.into_iter().collect();
    f
}

/// The experience needed to go from `level` to the next one.
pub fn xp_needed_for_next_level(level: i32) -> i32 {
    if level >= 30 {
        112 + (level - 30) * 9
    } else if level >= 15 {
        37 + (level - 15) * 5
    } else {
        7 + level * 2
    }
}

impl Game {
    /// The kind of cooker the block at `pos` is (`None` if it is not one or is not loaded).
    pub(super) fn cooker_at(&self, pos: BlockPos) -> Option<Cooker> {
        let state = self.world.block(pos)?;
        cooker_of(ferrite_data::blocks::block_of_state(state)?.name)
    }

    /// Takes in what a chunk's record held.
    pub(super) fn restore_containers(&mut self, found: LoadedContainers) {
        self.containers.extend(found.chests);
        for (pos, save) in found.furnaces {
            if let Some(cooker) = self.cooker_at(pos) {
                self.furnaces.insert(pos, furnace_from_save(cooker, save));
            }
        }
    }

    /// Changes what a furnace holds the way the game's `setItem` does: a different ingredient
    /// restarts cooking.
    pub(super) fn set_furnace_slots(&mut self, pos: BlockPos, new: [ItemStack; 3]) {
        let Some(f) = self.furnaces.get_mut(&pos) else { return };
        let old = f.slots[crate::furnace::INGREDIENT];
        let now = new[crate::furnace::INGREDIENT];
        let same = now.count > 0 && old.count > 0 && old.item == now.item;
        f.slots = new;
        if !same {
            f.total = f.cook_time();
            f.spent = 0;
        }
        self.dirty.insert((pos.x >> 4, pos.z >> 4));
    }

    pub(super) fn tick_furnaces(&mut self) {
        let positions: Vec<BlockPos> = self.furnaces.keys().copied().collect();
        for pos in positions {
            // A furnace whose chunk is not loaded waits; one whose block is gone is forgotten.
            let Some(state) = self.world.block(pos) else { continue };
            let Some(cooker) = self.cooker_at(pos) else {
                self.furnaces.remove(&pos);
                continue;
            };
            let Some(f) = self.furnaces.get_mut(&pos) else { continue };
            if f.cooker != cooker {
                f.cooker = cooker;
            }
            let flipped = f.tick();
            let (lit, busy) = (f.is_lit(), f.is_lit() || f.spent > 0);
            if flipped {
                let value = if lit { "true" } else { "false" };
                if let Some(new) = ferrite_blocks::Kinds::get().with_property(state, "lit", value) {
                    self.world.set_block(pos, new, ferrite_blocks::update::ALL);
                }
            }
            if busy || flipped {
                self.dirty.insert((pos.x >> 4, pos.z >> 4));
            }
        }
        self.sync_furnace_windows();
    }

    /// Tells everyone looking at a furnace what changed in it since the last tick.
    fn sync_furnace_windows(&mut self) {
        let ids = self.order.clone();
        for id in ids {
            let Some(p) = self.players.get_mut(&id) else { continue };
            let Some(w) = p.window.as_mut() else { continue };
            let Source::Furnace(pos) = w.source else { continue };
            let Some(f) = self.furnaces.get(&pos) else { continue };
            for i in 0..3 {
                if w.menu.slots[i] != f.slots[i] {
                    w.menu.slots[i] = f.slots[i];
                    w.state = (w.state + 1) & 0x7FFF;
                    send(
                        &p.sink,
                        &ContainerSetSlot {
                            window_id: VarInt(w.id),
                            state_id: VarInt(w.state),
                            slot: i as i16,
                            item: f.slots[i],
                        },
                    );
                }
            }
            let data = furnace_data(f);
            for (property, value) in data.into_iter().enumerate() {
                if w.sent_data[property] != value {
                    w.sent_data[property] = value;
                    send(
                        &p.sink,
                        &ContainerSetData {
                            window_id: VarInt(w.id),
                            property: property as i16,
                            value,
                        },
                    );
                }
            }
        }
    }

    /// The experience stored in a furnace goes to the player who took its output.
    pub(super) fn award_furnace_experience(&mut self, id: PlayerId, pos: BlockPos) {
        let mut dice = self.mob_dice();
        let Some(f) = self.furnaces.get_mut(&pos) else { return };
        let points = f.take_experience(|| dice.unit() as f32);
        self.dirty.insert((pos.x >> 4, pos.z >> 4));
        if points > 0 {
            self.give_experience(id, points as i32);
        }
    }

    /// `ServerPlayer.giveExperiencePoints`.
    pub(super) fn give_experience(&mut self, id: PlayerId, amount: i32) {
        let Some(p) = self.players.get_mut(&id) else { return };
        p.xp_progress += amount as f32 / xp_needed_for_next_level(p.xp_level) as f32;
        p.xp_total = p.xp_total.saturating_add(amount).max(0);
        while p.xp_progress >= 1.0 {
            p.xp_progress = (p.xp_progress - 1.0) * xp_needed_for_next_level(p.xp_level) as f32;
            p.xp_level += 1;
            p.xp_progress /= xp_needed_for_next_level(p.xp_level) as f32;
        }
        self.send_experience(id);
    }

    pub(super) fn send_experience(&self, id: PlayerId) {
        if let Some(p) = self.players.get(&id) {
            send(
                &p.sink,
                &SetExperience {
                    progress: p.xp_progress,
                    total: VarInt(p.xp_total),
                    level: VarInt(p.xp_level),
                },
            );
        }
    }
}

/// The four numbers a furnace window shows: burn time left, burn time in all, progress, total.
pub(super) fn furnace_data(f: &Furnace) -> [i16; 4] {
    [f.lit as i16, f.lit_total as i16, f.spent as i16, f.total as i16]
}
