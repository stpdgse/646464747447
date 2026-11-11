//! Mobs in the game: spawning, ticking, telling players about them, and being attacked.

use ferrite_data::entities::entity_by_name;
use ferrite_data::items::item_info;
use ferrite_loot::{EntityLoot, LootRandom, LootWorld, Tool};

use super::*;
use crate::mobs::{Mob, angle_byte};

#[derive(Debug)]
struct NoWorld;

impl LootWorld for NoWorld {
    fn block_above(&self, _: i32) -> Option<(String, Vec<(String, String)>)> {
        None
    }
}

struct Dice<'a>(&'a mut u64);

impl LootRandom for Dice<'_> {
    fn next_f32(&mut self) -> f32 {
        let mut s = ferrite_loot::SplitMix(*self.0);
        let v = s.next_f32();
        *self.0 = s.0;
        v
    }

    fn next_below(&mut self, bound: u32) -> u32 {
        let mut s = ferrite_loot::SplitMix(*self.0);
        let v = s.next_below(bound);
        *self.0 = s.0;
        v
    }
}

/// Passive animals that spawn with new chunks: `(name, weight, min group, max group)`.
const ANIMALS: [(&str, u32, u32, u32); 4] = [
    ("minecraft:sheep", 12, 2, 4),
    ("minecraft:pig", 10, 2, 4),
    ("minecraft:chicken", 10, 4, 4),
    ("minecraft:cow", 8, 2, 4),
];

/// The damage left after armor (`CombatRules.getDamageAfterAbsorb`).
pub fn damage_after_armor(damage: f32, armor: f32, toughness: f32) -> f32 {
    let f = 2.0 + toughness / 4.0;
    let g = (armor - damage / f).clamp(armor * 0.2, 20.0);
    damage * (1.0 - g / 25.0)
}

impl Game {
    fn mob_dice(&mut self) -> Rng {
        let mut s = ferrite_loot::SplitMix(self.rng);
        let v = s.next_below(u32::MAX);
        self.rng = s.0;
        Rng(u64::from(v) << 16 ^ self.rng)
    }

    /// Where the ground is at `(x, z)`: the first free cell above the topmost solid block.
    fn surface_at(&self, x: i32, z: i32) -> Option<i32> {
        let dim = self.cfg.generator.dimension;
        (dim.min_y..dim.max_y()).rev().find_map(|y| {
            let s = self.world.block(BlockPos::new(x, y, z))?;
            let f = ferrite_data::shapes::facts(s);
            (!f.collision_shape().is_empty() && !f.has(ferrite_data::shapes::flag::LIQUID))
                .then_some(y + 1)
        })
    }

    pub(super) fn spawn_mob(&mut self, name: &str, pos: [f64; 3], yaw: f32) -> Option<i32> {
        let (kind, info) = entity_by_name(name)?;
        let id = self.next_entity;
        self.next_entity += 1;
        let uuid = Uuid::from_u128(
            (u128::from(self.mob_dice().next_u64()) << 64) | u128::from(self.mob_dice().next_u64()),
        );
        let mob = Mob::new(id, uuid, kind, info, pos, yaw);
        self.mobs.insert(id, mob);
        Some(id)
    }

    /// Animals that come with a freshly generated chunk (a tenth of chunks get a herd).
    pub(super) fn spawn_chunk_animals(&mut self, cx: i32, cz: i32) {
        let mut rng = self.mob_dice();
        if rng.unit() >= self.cfg.animal_chance {
            return;
        }
        let total: u32 = ANIMALS.iter().map(|a| a.1).sum();
        let mut roll = rng.below(total);
        let (name, _, lo, hi) = *ANIMALS
            .iter()
            .find(|a| {
                if roll < a.1 {
                    true
                } else {
                    roll -= a.1;
                    false
                }
            })
            .expect("a weighted pick");
        let count = lo + rng.below(hi - lo + 1);
        let (hx, hz) = (cx * 16 + rng.below(16) as i32, cz * 16 + rng.below(16) as i32);
        for _ in 0..count {
            let x = hx + rng.below(7) as i32 - 3;
            let z = hz + rng.below(7) as i32 - 3;
            if x >> 4 != cx || z >> 4 != cz {
                continue;
            }
            let Some(y) = self.surface_at(x, z) else { continue };
            let yaw = rng.between(0.0, 360.0) as f32;
            self.spawn_mob(name, [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5], yaw);
        }
    }

    pub(super) fn tick_mobs(&mut self) {
        if self.mobs.is_empty() {
            return;
        }
        let players: Vec<[f64; 3]> = self
            .order
            .iter()
            .filter_map(|id| self.players.get(id))
            .map(|p| [p.x, p.y, p.z])
            .collect();
        let ids: Vec<i32> = self.mobs.keys().copied().collect();
        let mut gone = Vec::new();
        for id in ids {
            let world = &self.world;
            let Some(m) = self.mobs.get_mut(&id) else { continue };
            // Only mobs where a player can see them move; the rest stand still.
            m.tick(world, &players);
            if m.death_time.is_some_and(|t| t >= 20)
                || m.pos[1] < f64::from(self.cfg.generator.dimension.min_y) - 64.0
            {
                gone.push(id);
            }
        }
        for id in &gone {
            self.mobs.remove(id);
        }
        self.sync_mobs(&gone);
    }

    fn mob_range(&self, kind: u32, view: i32) -> f64 {
        let tracking = ferrite_data::entities::entity_info(kind).map_or(5, |e| e.tracking_range);
        f64::from(tracking.min(view)) * 16.0
    }

    fn add_mob_packet(m: &Mob) -> AddEntity {
        AddEntity {
            id: VarInt(m.id),
            uuid: m.uuid,
            kind: VarInt(m.kind as i32),
            x: m.pos[0],
            y: m.pos[1],
            z: m.pos[2],
            velocity: LpVec3 { x: m.vel[0], y: m.vel[1], z: m.vel[2] },
            x_rot: angle_byte(m.pitch),
            y_rot: angle_byte(m.yaw),
            y_head_rot: angle_byte(m.head_yaw),
            data: VarInt(0),
        }
    }

    /// Spawns, moves, turns and removes mobs for each player by distance.
    fn sync_mobs(&mut self, gone: &[i32]) {
        let ids = self.order.clone();
        for pid in ids {
            let (px, pz, view) = {
                let p = &self.players[&pid];
                (p.x, p.z, p.view_distance.min(i32::from(self.cfg.simulation_distance)))
            };
            let mut spawn = Vec::new();
            let mut remove: Vec<VarInt> = Vec::new();
            {
                let p = &self.players[&pid];
                for m in self.mobs.values() {
                    let range = self.mob_range(m.kind, view);
                    let near = (m.pos[0] - px).abs() <= range && (m.pos[2] - pz).abs() <= range;
                    let tracked = p.tracked_mobs.contains(&m.id);
                    if near && !tracked && !gone.contains(&m.id) {
                        spawn.push(m.id);
                    } else if tracked && (!near || gone.contains(&m.id)) {
                        remove.push(VarInt(m.id));
                    }
                }
                for id in &p.tracked_mobs {
                    if !self.mobs.contains_key(id) && !remove.iter().any(|v| v.0 == *id) {
                        remove.push(VarInt(*id));
                    }
                }
            }
            let p = self.players.get_mut(&pid).expect("exists");
            for id in &remove {
                p.tracked_mobs.remove(&id.0);
            }
            if !remove.is_empty() {
                send(&p.sink, &RemoveEntities { ids: LenVec(remove) });
            }
            for id in spawn {
                send(&p.sink, &Self::add_mob_packet(&self.mobs[&id]));
                p.tracked_mobs.insert(id);
            }
        }
        // Movement, once per mob for everyone tracking it.
        let ids: Vec<i32> = self.mobs.keys().copied().collect();
        for id in ids {
            let Some(m) = self.mobs.get_mut(&id) else { continue };
            let now = m.pos.map(|v| (v * 4096.0).floor() as i64);
            let d = [now[0] - m.sent_pos[0], now[1] - m.sent_pos[1], now[2] - m.sent_pos[2]];
            let rot = (angle_byte(m.yaw), angle_byte(m.pitch));
            let head = angle_byte(m.head_yaw);
            let moved = d != [0, 0, 0];
            let turned = rot != m.sent_rot;
            let head_turned = head != m.sent_head;
            if !moved && !turned && !head_turned {
                continue;
            }
            let (pos, on_ground, eid) = (m.pos, m.on_ground, VarInt(m.id));
            let small = d.iter().all(|v| v.abs() <= 32767);
            if moved {
                m.sent_pos = now;
            }
            m.sent_rot = rot;
            m.sent_head = head;
            for p in self.players.values() {
                if !p.tracked_mobs.contains(&id) {
                    continue;
                }
                if moved && !small {
                    send(
                        &p.sink,
                        &TeleportEntity {
                            id: eid,
                            x: pos[0],
                            y: pos[1],
                            z: pos[2],
                            dx: 0.0,
                            dy: 0.0,
                            dz: 0.0,
                            y_rot: m.yaw,
                            x_rot: m.pitch,
                            relative: 0,
                            on_ground,
                        },
                    );
                } else if moved && turned {
                    send(
                        &p.sink,
                        &MoveEntityPosRot {
                            id: eid,
                            on_ground: OnGround(on_ground),
                            dx: d[0] as i16,
                            dy: d[1] as i16,
                            dz: d[2] as i16,
                            y_rot: rot.0,
                            x_rot: rot.1,
                        },
                    );
                } else if moved {
                    send(
                        &p.sink,
                        &MoveEntityPos {
                            id: eid,
                            on_ground: OnGround(on_ground),
                            dx: d[0] as i16,
                            dy: d[1] as i16,
                            dz: d[2] as i16,
                        },
                    );
                } else if turned {
                    send(
                        &p.sink,
                        &MoveEntityRot { id: eid, y_rot: rot.0, x_rot: rot.1, on_ground },
                    );
                }
                if head_turned {
                    send(&p.sink, &RotateHead { id: eid, y_head_rot: head });
                }
            }
        }
    }

    /// A player hits an entity: damage from the weapon and the swing's charge, armor, knockback,
    /// then death and loot.
    pub(super) fn attack_entity(&mut self, id: PlayerId, target: i32) {
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.dead || p.game_mode == 3 {
            return;
        }
        let Some(mob) = self.mobs.get(&target) else { return };
        // Reach: three blocks from the eyes to the hitbox.
        let eye = [p.x, p.y + 1.62, p.z];
        let b = mob.bbox();
        let nearest =
            [eye[0].clamp(b[0], b[3]), eye[1].clamp(b[1], b[4]), eye[2].clamp(b[2], b[5])];
        let d = [nearest[0] - eye[0], nearest[1] - eye[1], nearest[2] - eye[2]];
        if d[0] * d[0] + d[1] * d[1] + d[2] * d[2] > 9.0 {
            return;
        }
        let held = p.menu.slots[36 + usize::from(p.held_slot)];
        let info = (held.count > 0).then(|| item_info(held.item)).flatten();
        let (bonus, speed) =
            info.map_or((0.0, 4.0), |i| (i.attack_damage, 4.0 + i.attack_speed_modifier));
        // The swing's charge: a full-strength hit needs the cooldown to have run.
        let cooldown = 20.0 / speed;
        let charge = ((p.attack_ticker as f32 + 0.5) / cooldown).clamp(0.0, 1.0);
        p.attack_ticker = 0;
        let mut damage = (1.0 + bonus) * (0.2 + charge * charge * 0.8);
        // Critical hit: falling, with a nearly full swing.
        if p.fall_distance > 0.0 && !p.on_ground && charge > 0.9 {
            damage *= 1.5;
        }
        let armor = mob.info.attributes.and_then(|a| a.armor).unwrap_or(0.0) as f32;
        let damage = damage_after_armor(damage, armor, 0.0);
        let from = [p.x, p.y, p.z];
        let held_name = if held.count > 0 {
            ferrite_data::registries::item::names.get(held.item as usize).copied().unwrap_or("")
        } else {
            ""
        };
        self.add_exhaustion(id, 0.1);
        let Some(mob) = self.mobs.get_mut(&target) else { return };
        let was_dead = mob.is_dead();
        if !mob.hurt(damage, from) {
            return;
        }
        let (eid, died, name) = (mob.id, mob.is_dead() && !was_dead, mob.info.name);
        let (pos, vel) = (mob.pos, mob.vel);
        let ty = Self::damage_type_for_player_attack();
        for pl in self.players.values() {
            if pl.tracked_mobs.contains(&target) {
                send(
                    &pl.sink,
                    &DamageEvent {
                        entity_id: VarInt(eid),
                        source_type: ty,
                        source_cause: VarInt(0),
                        source_direct: VarInt(0),
                        source_position: None,
                    },
                );
                send(&pl.sink, &EntityEvent { entity_id: eid, event: if died { 3 } else { 2 } });
                send(
                    &pl.sink,
                    &SetEntityMotion {
                        id: VarInt(eid),
                        velocity: LpVec3 { x: vel[0], y: vel[1], z: vel[2] },
                    },
                );
            }
        }
        if died {
            let state = EntityLoot { on_fire: false, killed_by_player: true, looting: 0 };
            let drops = ferrite_loot::entity_drops(
                name,
                state,
                Tool { item: held_name, enchantments: &[] },
                &NoWorld,
                &mut Dice(&mut self.rng),
            );
            let at =
                BlockPos::new(pos[0].floor() as i32, pos[1].floor() as i32, pos[2].floor() as i32);
            for drop in drops {
                let Some(item) =
                    ferrite_data::registries::item::by_name.get(drop.item.as_str()).copied()
                else {
                    continue;
                };
                self.drop_item(at, ItemStack { count: drop.count, item: item as i32 });
            }
        }
    }

    fn damage_type_for_player_attack() -> VarInt {
        VarInt(
            synchronized_id("minecraft:damage_type", "minecraft:player_attack").unwrap_or(0) as i32
        )
    }

    /// Per-player attack cooldown counter.
    pub(super) fn tick_attack_cooldowns(&mut self) {
        for p in self.players.values_mut() {
            p.attack_ticker = p.attack_ticker.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::damage_after_armor;

    #[test]
    fn armor_takes_a_share_of_the_damage() {
        assert!((damage_after_armor(10.0, 0.0, 0.0) - 10.0).abs() < 1e-6);
        // 20 armor points: protection = 20 - 5/2 = 17.5 of 25, so 30% gets through.
        assert!((damage_after_armor(5.0, 20.0, 0.0) - 1.5).abs() < 1e-5);
        // Armor always blocks at least a fifth of its points: 2 points shave 1.6% off 7 damage.
        let d = damage_after_armor(7.0, 2.0, 0.0);
        assert!((d - 6.888).abs() < 1e-4, "{d}");
    }
}
