//! Health, hunger, fall damage, eating, death and respawning.

use ferrite_data::items::item_info;

use super::*;

/// What a player is eating.
#[derive(Clone, Copy, Debug)]
pub(super) struct Eating {
    slot: usize,
    start: u64,
}

pub(super) const MAX_HEALTH: f32 = 20.0;

/// Bit-exact vanilla fall damage: how many hearts' worth of hit-points a fall costs.
pub fn fall_damage(fall_distance: f64, multiplier: f64) -> i32 {
    ((fall_distance as f32 - 3.0) as f64 * multiplier).ceil() as i32
}

impl Game {
    fn damage_type_id(name: &str) -> VarInt {
        VarInt(synchronized_id("minecraft:damage_type", &format!("minecraft:{name}")).unwrap_or(0)
            as i32)
    }

    fn invulnerable(&self, id: PlayerId) -> bool {
        self.players.get(&id).is_none_or(|p| p.dead || p.game_mode == 1 || p.game_mode == 3)
    }

    /// Hurts a player: `source` names a damage type (`fall`, `starve`, ...). Follows vanilla's
    /// invulnerability frames (after a hit, only a stronger hit within 10 ticks lands, for the
    /// difference).
    pub(super) fn hurt(&mut self, id: PlayerId, amount: f32, source: &str, bypasses_frames: bool) {
        if amount <= 0.0 || self.invulnerable(id) {
            return;
        }
        let ty = Self::damage_type_id(source);
        let Some(p) = self.players.get_mut(&id) else { return };
        let mut dealt = amount;
        if !bypasses_frames && p.invulnerable_ticks > 10 {
            if amount <= p.last_hurt {
                return;
            }
            dealt = amount - p.last_hurt;
            p.last_hurt = amount;
        } else {
            p.last_hurt = amount;
            p.invulnerable_ticks = 20;
        }
        p.health = (p.health - dealt).max(0.0);
        let (eid, died) = (p.entity_id, p.health <= 0.0);
        self.send_vitals(id);
        let event = DamageEvent {
            entity_id: VarInt(eid),
            source_type: ty,
            source_cause: VarInt(0),
            source_direct: VarInt(0),
            source_position: None,
        };
        for other in self.players.values() {
            send(&other.sink, &event);
            send(&other.sink, &EntityEvent { entity_id: eid, event: 2 });
        }
        if died {
            self.kill(id, source);
        }
    }

    pub(super) fn send_vitals(&self, id: PlayerId) {
        if let Some(p) = self.players.get(&id) {
            send(
                &p.sink,
                &SetHealth { health: p.health, food: VarInt(p.food), saturation: p.saturation },
            );
        }
    }

    pub(super) fn heal(&mut self, id: PlayerId, amount: f32) {
        if let Some(p) = self.players.get_mut(&id)
            && !p.dead
            && p.health > 0.0
        {
            p.health = (p.health + amount).min(MAX_HEALTH);
            self.send_vitals(id);
        }
    }

    pub(super) fn add_exhaustion(&mut self, id: PlayerId, amount: f32) {
        if let Some(p) = self.players.get_mut(&id)
            && p.game_mode != 1
            && p.game_mode != 3
        {
            p.exhaustion = (p.exhaustion + amount).min(40.0);
        }
    }

    /// Death: the screen, the dropped inventory and a death animation for everyone.
    fn kill(&mut self, id: PlayerId, source: &str) {
        let Some(p) = self.players.get_mut(&id) else { return };
        p.dead = true;
        p.dig = None;
        p.eating = None;
        let (eid, at) = (
            p.entity_id,
            BlockPos::new(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32),
        );
        let name = p.name.clone();
        // Survival players drop everything where they died.
        let mut drops: Vec<ItemStack> = Vec::new();
        if p.game_mode == 0 {
            for slot in p.menu.slots.iter_mut().skip(1) {
                drops.push(std::mem::take(slot));
            }
            drops.push(std::mem::take(&mut p.menu.carried));
        }
        p.menu.refresh_result();
        let message = Text::plain(format!("{name} died ({source})"));
        send(&p.sink, &PlayerCombatKill { player_id: VarInt(eid), message });
        for stack in drops.into_iter().filter(|s| s.count > 0) {
            self.drop_item(at, stack);
        }
        for other in self.players.values() {
            send(&other.sink, &EntityEvent { entity_id: eid, event: 3 });
        }
        self.resync_inventory(id);
        tracing::info!("{name} died ({source})");
    }

    /// The respawn button: back to spawn with full health, a fresh inventory view and the world
    /// sent again.
    pub(super) fn respawn(&mut self, id: PlayerId) {
        let (x, y, z) = self.spawn_point();
        let cfg = &self.cfg;
        let dim = cfg.generator.dimension;
        let Some(p) = self.players.get_mut(&id) else { return };
        if !p.dead {
            return;
        }
        p.dead = false;
        p.health = MAX_HEALTH;
        p.food = 20;
        p.saturation = 5.0;
        p.exhaustion = 0.0;
        p.fall_distance = 0.0;
        p.invulnerable_ticks = 0;
        p.x = x;
        p.y = y;
        p.z = z;
        p.on_ground = true;
        p.loaded.clear();
        p.queue.clear();
        p.center = (x.floor() as i32 >> 4, z.floor() as i32 >> 4);
        p.sent.base = [quantize(x), quantize(y), quantize(z)];
        let tp = p.next_teleport + 1;
        p.next_teleport = tp;
        p.pending_teleport = Some(tp);
        let spawn = SpawnInfo {
            dimension_type: VarInt(
                synchronized_id("minecraft:dimension_type", "minecraft:overworld").unwrap_or(0)
                    as i32,
            ),
            dimension: Identifier::new("minecraft:overworld").expect("valid identifier"),
            seed: cfg.seed,
            game_mode: VarInt(i32::from(p.game_mode)),
            previous_game_mode: None,
            is_debug: false,
            is_flat: true,
            last_death: None,
            portal_cooldown: VarInt(0),
            sea_level: VarInt(dim.min_y + 1),
        };
        send(&p.sink, &Respawn { spawn, data_to_keep: 0 });
        send(
            &p.sink,
            &PlayerPosition {
                teleport_id: VarInt(tp),
                x,
                y,
                z,
                dx: 0.0,
                dy: 0.0,
                dz: 0.0,
                y_rot: 0.0,
                x_rot: 0.0,
                relative: 0,
            },
        );
        self.send_vitals(id);
        self.resync_inventory(id);
        self.plan_chunks(id);
        // Others see the player at the spawn again.
        let ids = self.order.clone();
        let my_entity = self.players[&id].entity_id;
        for other in ids {
            if other != id
                && let Some(o) = self.players.get_mut(&other)
                && o.tracked.remove(&id)
            {
                send(&o.sink, &RemoveEntities { ids: LenVec(vec![VarInt(my_entity)]) });
            }
        }
    }

    /// Called for every movement: tracks falling and applies landing damage.
    pub(super) fn track_fall(&mut self, id: PlayerId, dy: f64, on_ground: bool) {
        let Some(p) = self.players.get(&id) else { return };
        let (bx, by, bz) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let feet = self.world.block(BlockPos::new(bx, by, bz));
        let in_fluid = feet.is_some_and(|s| {
            ferrite_data::shapes::facts(s).has(ferrite_data::shapes::flag::LIQUID)
        });
        let climbing = feet.is_some_and(|s| {
            ferrite_data::blocks::block_of_state(s).is_some_and(|b| {
                matches!(
                    b.name,
                    "minecraft:ladder"
                        | "minecraft:vine"
                        | "minecraft:scaffolding"
                        | "minecraft:cobweb"
                        | "minecraft:sweet_berry_bush"
                        | "minecraft:cave_vines"
                        | "minecraft:cave_vines_plant"
                        | "minecraft:twisting_vines"
                        | "minecraft:weeping_vines"
                )
            })
        });
        let landed_on = self.world.block(BlockPos::new(bx, (p.y - 0.2).floor() as i32, bz));
        let sneaking = p.sneaking;
        let Some(p) = self.players.get_mut(&id) else { return };
        if in_fluid || climbing {
            p.fall_distance = 0.0;
            return;
        }
        if dy < 0.0 {
            p.fall_distance -= dy;
        }
        if on_ground {
            let dist = std::mem::take(&mut p.fall_distance);
            if dist > 0.0 {
                let mult = match landed_on
                    .and_then(ferrite_data::blocks::block_of_state)
                    .map(|b| b.name)
                {
                    Some("minecraft:hay_block" | "minecraft:honey_block") => 0.2,
                    Some("minecraft:slime_block") if !sneaking => 0.0,
                    Some(n) if n.ends_with("_bed") => 0.5,
                    _ => 1.0,
                };
                let amount = fall_damage(dist, mult);
                if amount > 0 {
                    self.hurt(id, amount as f32, "fall", false);
                }
            }
        }
    }

    /// Per-tick upkeep: invulnerability, hunger, regeneration, starvation, the void, eating.
    pub(super) fn tick_vitals(&mut self) {
        let min_y = f64::from(self.cfg.generator.dimension.min_y);
        let ids: Vec<PlayerId> = self.order.clone();
        for id in ids {
            let Some(p) = self.players.get_mut(&id) else { continue };
            if p.dead {
                continue;
            }
            p.invulnerable_ticks = p.invulnerable_ticks.saturating_sub(1);
            if p.invulnerable_ticks == 0 {
                p.last_hurt = 0.0;
            }
            let (y, mode) = (p.y, p.game_mode);
            if mode == 1 || mode == 3 {
                continue;
            }
            // Food: exhaustion turns into lost saturation, then lost hunger.
            if p.exhaustion > 4.0 {
                p.exhaustion -= 4.0;
                if p.saturation > 0.0 {
                    p.saturation = (p.saturation - 1.0).max(0.0);
                } else {
                    p.food = (p.food - 1).max(0);
                }
                self.send_vitals(id);
            }
            let p = self.players.get_mut(&id).expect("exists");
            let hurt = p.health < MAX_HEALTH;
            let (food, sat) = (p.food, p.saturation);
            if sat > 0.0 && hurt && food >= 20 {
                p.food_timer += 1;
                if p.food_timer >= 10 {
                    let f = sat.min(6.0);
                    p.food_timer = 0;
                    self.heal(id, f / 6.0);
                    self.add_exhaustion(id, f);
                }
            } else if food >= 18 && hurt {
                p.food_timer += 1;
                if p.food_timer >= 80 {
                    p.food_timer = 0;
                    self.heal(id, 1.0);
                    self.add_exhaustion(id, 6.0);
                }
            } else if food <= 0 {
                p.food_timer += 1;
                if p.food_timer >= 80 {
                    p.food_timer = 0;
                    // Easy difficulty: starvation stops at 10 health.
                    if p.health > 10.0 {
                        self.hurt(id, 1.0, "starve", true);
                    }
                }
            } else {
                p.food_timer = 0;
            }
            if y < min_y - 64.0 {
                self.hurt(id, 4.0, "out_of_world", true);
            }
            self.tick_eating(id);
        }
    }

    // ---- eating ----

    /// The player uses the item in hand: foods start being eaten.
    pub(super) fn use_item(&mut self, id: PlayerId) {
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.dead || p.eating.is_some() {
            return;
        }
        let slot = 36 + usize::from(p.held_slot);
        let held = p.menu.slots[slot];
        let Some(food) =
            (held.count > 0).then(|| item_info(held.item)).flatten().and_then(|i| i.food)
        else {
            return;
        };
        if p.food >= 20 && !food.can_always_eat || p.game_mode == 3 {
            return;
        }
        p.eating = Some(Eating { slot, start: self.tick });
    }

    pub(super) fn stop_eating(&mut self, id: PlayerId) {
        if let Some(p) = self.players.get_mut(&id) {
            p.eating = None;
        }
    }

    fn tick_eating(&mut self, id: PlayerId) {
        let tick = self.tick;
        let Some(p) = self.players.get_mut(&id) else { return };
        let Some(e) = p.eating else { return };
        let held = p.menu.slots[e.slot];
        let food = (held.count > 0).then(|| item_info(held.item)).flatten().and_then(|i| i.food);
        // Switching slots or losing the item stops the meal.
        let Some(food) = food.filter(|_| e.slot == 36 + usize::from(p.held_slot)) else {
            p.eating = None;
            return;
        };
        if tick - e.start < u64::from(food.consume_ticks) {
            return;
        }
        p.eating = None;
        p.food = (p.food + food.nutrition).min(20);
        p.saturation = (p.saturation + food.saturation).min(p.food as f32);
        if p.game_mode != 1 {
            p.menu.slots[e.slot].count -= 1;
            if p.menu.slots[e.slot].count <= 0 {
                p.menu.slots[e.slot] = ItemStack::default();
            }
        }
        p.inv_state = (p.inv_state + 1) & 0x7FFF;
        send(
            &p.sink,
            &ContainerSetSlot {
                window_id: VarInt(0),
                state_id: VarInt(p.inv_state),
                slot: e.slot as i16,
                item: p.menu.slots[e.slot],
            },
        );
        self.send_vitals(id);
    }
}

#[cfg(test)]
mod tests {
    use super::fall_damage;

    #[test]
    fn falls_cost_a_point_per_block_beyond_three() {
        assert_eq!(fall_damage(3.0, 1.0), 0);
        assert_eq!(fall_damage(3.5, 1.0), 1);
        assert_eq!(fall_damage(4.0, 1.0), 1);
        assert_eq!(fall_damage(10.0, 1.0), 7);
        // Hay bales take 80% off, rounding up.
        assert_eq!(fall_damage(10.0, 0.2), 2);
        assert_eq!(fall_damage(100.0, 0.0), 0);
    }
}
