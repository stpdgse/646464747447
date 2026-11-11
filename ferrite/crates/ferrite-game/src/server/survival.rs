//! Survival mining and dropped items: digging progress, loot, item entities and pickup.

use ferrite_blocks::dig::Digger;
use ferrite_data::items::item_info;
use ferrite_loot::{LootContext, LootRandom, LootWorld, Tool};

use super::*;
use crate::items::{ItemEntity, add_to_inventory, max_stack};

/// A block a player is in the middle of breaking.
#[derive(Clone, Copy, Debug)]
pub(super) struct Dig {
    pos: BlockPos,
    /// The tick digging began.
    start: u64,
    /// The last progress stage (0..=9) shown to other players.
    stage: i32,
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

struct WorldView<'a>(&'a World<Chunks>, BlockPos);

impl std::fmt::Debug for WorldView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WorldView")
    }
}

/// Block name and property values of a state.
fn describe(state: u16) -> Option<(&'static str, Vec<(&'static str, &'static str)>)> {
    let block = ferrite_data::blocks::block_of_state(state)?;
    let mut values = [0u8; 16];
    let n = block.value_indices(state, &mut values)?;
    let props = block
        .properties
        .iter()
        .zip(&values[..n])
        .map(|(p, v)| (p.name, p.values[usize::from(*v)]))
        .collect();
    Some((block.name, props))
}

impl LootWorld for WorldView<'_> {
    fn block_above(&self, offset_y: i32) -> Option<(String, Vec<(String, String)>)> {
        let state = self.0.block(BlockPos::new(self.1.x, self.1.y + offset_y, self.1.z))?;
        let (name, props) = describe(state)?;
        Some((
            name.to_string(),
            props.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
        ))
    }
}

impl Game {
    fn next_random(&mut self) -> f64 {
        let mut s = ferrite_loot::SplitMix(self.rng);
        let v = f64::from(s.next_f32());
        self.rng = s.0;
        v
    }

    fn random_between(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.next_random() * (hi - lo)
    }

    /// The digger's tool and condition, as the speed rules see them.
    fn digger_for(&self, id: PlayerId) -> Digger {
        let Some(p) = self.players.get(&id) else { return Digger::default() };
        let held = p.menu.slots[36 + usize::from(p.held_slot)];
        let tool = if held.count > 0 { item_info(held.item) } else { None };
        Digger { tool, on_ground: p.on_ground, ..Digger::default() }
    }

    /// Starts digging at `pos` (survival). Breaks at once if one tick is enough.
    pub(super) fn start_dig(&mut self, id: PlayerId, pos: BlockPos) {
        let Some(state) = self.world.block(pos) else { return };
        if ferrite_data::shapes::facts(state).has(ferrite_data::shapes::flag::AIR) {
            self.resend_block(id, pos);
            return;
        }
        let progress = self.digger_for(id).progress_per_tick(state);
        if progress >= 1.0 {
            self.break_survival(id, pos);
            return;
        }
        let tick = self.tick;
        if let Some(p) = self.players.get_mut(&id) {
            p.dig = Some(Dig { pos, start: tick, stage: -1 });
        }
        self.show_dig_stage(id, pos, 0);
    }

    /// The client says it has finished digging `pos`. Accepted if enough time has passed (with
    /// the game's tolerance, since the client and server tick counts differ by a tick or so).
    pub(super) fn stop_dig(&mut self, id: PlayerId, pos: BlockPos) {
        let Some(dig) = self.players.get(&id).and_then(|p| p.dig) else { return };
        if dig.pos != pos {
            return;
        }
        let Some(state) = self.world.block(pos) else { return };
        let elapsed = self.tick - dig.start;
        let f = self.digger_for(id).progress_per_tick(state) * (elapsed + 1) as f32;
        if f >= 0.7 {
            self.break_survival(id, pos);
        } else {
            self.resend_block(id, pos);
        }
    }

    pub(super) fn abort_dig(&mut self, id: PlayerId) {
        if let Some(dig) = self.players.get_mut(&id).and_then(|p| p.dig.take()) {
            self.show_dig_stage(id, dig.pos, 255);
        }
    }

    /// Tells the other players how far along the digging is.
    fn show_dig_stage(&self, digger: PlayerId, pos: BlockPos, stage: u8) {
        let Some(me) = self.players.get(&digger) else { return };
        for (other_id, other) in &self.players {
            if *other_id != digger {
                send(
                    &other.sink,
                    &BlockDestruction {
                        id: VarInt(me.entity_id),
                        pos: PackedPos::new(pos.x, pos.y, pos.z),
                        progress: stage,
                    },
                );
            }
        }
    }

    /// Every tick: digging that has accumulated a full block's worth breaks it, and the stage
    /// other players see advances.
    pub(super) fn tick_digging(&mut self) {
        let ids: Vec<PlayerId> = self.order.clone();
        for id in ids {
            let Some(dig) = self.players.get(&id).and_then(|p| p.dig) else { continue };
            let Some(state) = self.world.block(dig.pos) else { continue };
            if ferrite_data::shapes::facts(state).has(ferrite_data::shapes::flag::AIR) {
                if let Some(p) = self.players.get_mut(&id) {
                    p.dig = None;
                }
                continue;
            }
            let f =
                self.digger_for(id).progress_per_tick(state) * (self.tick - dig.start + 1) as f32;
            if f >= 1.0 {
                self.break_survival(id, dig.pos);
                continue;
            }
            let stage = ((f * 10.0) as i32).clamp(0, 9);
            if stage != dig.stage {
                if let Some(p) = self.players.get_mut(&id)
                    && let Some(d) = p.dig.as_mut()
                {
                    d.stage = stage;
                }
                self.show_dig_stage(id, dig.pos, stage as u8);
            }
        }
    }

    /// Breaks the block a survival player has dug through, dropping what its loot table says if
    /// the tool allows.
    fn break_survival(&mut self, id: PlayerId, pos: BlockPos) {
        if let Some(p) = self.players.get_mut(&id) {
            p.dig = None;
        }
        self.show_dig_stage(id, pos, 255);
        let Some(state) = self.world.block(pos) else { return };
        let digger = self.digger_for(id);
        let mut drops = Vec::new();
        if digger.is_correct_tool(state)
            && let Some((name, props)) = describe(state)
        {
            let held_name = self
                .players
                .get(&id)
                .map(|p| p.menu.slots[36 + usize::from(p.held_slot)])
                .filter(|s| s.count > 0)
                .and_then(|s| ferrite_data::registries::item::names.get(s.item as usize).copied())
                .unwrap_or("");
            let view = WorldView(&self.world, pos);
            let ctx = LootContext {
                block: name,
                properties: &props,
                tool: Tool { item: held_name, enchantments: &[] },
                has_this_entity: true,
                explosion_radius: None,
                world: &view,
            };
            drops = ferrite_loot::block_drops(&ctx, &mut Dice(&mut self.rng));
        }
        self.world.destroy_block(pos);
        for drop in drops {
            let Some(item) =
                ferrite_data::registries::item::by_name.get(drop.item.as_str()).copied()
            else {
                continue;
            };
            let mut left = drop.count;
            while left > 0 {
                let n = left.min(max_stack(item as i32));
                self.drop_item(pos, ItemStack { count: n, item: item as i32 });
                left -= n;
            }
        }
    }

    /// Spawns a dropped stack where a block was (the game's `popResource`).
    pub(super) fn drop_item(&mut self, pos: BlockPos, stack: ItemStack) {
        let at = [
            f64::from(pos.x) + 0.5 + self.random_between(-0.25, 0.25),
            f64::from(pos.y) + 0.5 + self.random_between(-0.25, 0.25) - 0.125,
            f64::from(pos.z) + 0.5 + self.random_between(-0.25, 0.25),
        ];
        let vel = [self.random_between(-0.1, 0.1), 0.2, self.random_between(-0.1, 0.1)];
        let id = self.next_entity;
        self.next_entity += 1;
        let uuid = Uuid::from_u128(
            (u128::from(self.next_random().to_bits()) << 64)
                | u128::from(self.next_random().to_bits()),
        );
        self.items.insert(id, ItemEntity::new(id, uuid, at, vel, stack));
    }

    // ---- item entities ------------------------------------------------------------------

    fn item_kind() -> VarInt {
        VarInt(ferrite_data::registries::entity_type::by_name["minecraft:item"] as i32)
    }

    fn item_data_packet(item: &ItemEntity) -> SetEntityData {
        let mut data = vec![8u8, 7u8];
        let _ = item.stack.encode(&mut data);
        data.push(255);
        SetEntityData { id: VarInt(item.id), data: ferrite_protocol::types::Remaining(data) }
    }

    /// Advances every dropped item, lets players pick them up, and tells players about them.
    pub(super) fn tick_items(&mut self) {
        let ids: Vec<i32> = self.items.keys().copied().collect();
        let mut gone: Vec<i32> = Vec::new();
        for id in ids {
            let alive = {
                let world = &self.world;
                self.items.get_mut(&id).is_some_and(|it| it.tick(world))
            };
            if !alive {
                gone.push(id);
            }
        }
        // Pickup.
        let players: Vec<PlayerId> = self.order.clone();
        for pid in players {
            let Some(p) = self.players.get(&pid) else { continue };
            if p.game_mode == 3 {
                continue;
            }
            let reach = [
                p.x - 0.3 - 1.0,
                p.y - 0.5,
                p.z - 0.3 - 1.0,
                p.x + 0.3 + 1.0,
                p.y + 1.8 + 0.5,
                p.z + 0.3 + 1.0,
            ];
            let near: Vec<i32> = self
                .items
                .values()
                .filter(|it| it.pickup_delay == 0 && !gone.contains(&it.id))
                .filter(|it| {
                    let b = it.bbox();
                    b[0] < reach[3]
                        && b[3] > reach[0]
                        && b[1] < reach[4]
                        && b[4] > reach[1]
                        && b[2] < reach[5]
                        && b[5] > reach[2]
                })
                .map(|it| it.id)
                .collect();
            for item_id in near {
                self.pick_up(pid, item_id, &mut gone);
            }
        }
        for id in &gone {
            self.items.remove(id);
        }
        self.sync_items(&gone);
    }

    fn pick_up(&mut self, pid: PlayerId, item_id: i32, gone: &mut Vec<i32>) {
        let Some(item) = self.items.get(&item_id) else { return };
        let mut stack = item.stack;
        let before = stack.count;
        let Some(p) = self.players.get_mut(&pid) else { return };
        let held = usize::from(p.held_slot);
        let changed = add_to_inventory(&mut p.menu.slots, held, &mut stack);
        let taken = before - stack.count;
        if taken == 0 {
            return;
        }
        for slot in changed {
            p.inv_state = (p.inv_state + 1) & 0x7FFF;
            send(
                &p.sink,
                &ContainerSetSlot {
                    window_id: VarInt(0),
                    state_id: VarInt(p.inv_state),
                    slot: slot as i16,
                    item: p.menu.slots[slot],
                },
            );
        }
        let collector = p.entity_id;
        // Everyone who can see the item sees it fly to the collector.
        for other in self.players.values() {
            if other.tracked_items.contains(&item_id) || other.entity_id == collector {
                send(
                    &other.sink,
                    &TakeItemEntity {
                        collected: VarInt(item_id),
                        collector: VarInt(collector),
                        amount: VarInt(taken),
                    },
                );
            }
        }
        if let Some(it) = self.items.get_mut(&item_id) {
            it.stack.count = stack.count;
            if stack.count > 0 {
                let packet = Self::item_data_packet(it);
                for other in self.players.values() {
                    if other.tracked_items.contains(&item_id) {
                        send(&other.sink, &packet);
                    }
                }
            } else {
                gone.push(item_id);
            }
        }
    }

    /// Spawns, moves and removes item entities for each player by distance.
    fn sync_items(&mut self, gone: &[i32]) {
        let ids = self.order.clone();
        let limit = f64::from(self.cfg.view_distance.min(self.cfg.simulation_distance)) * 16.0;
        for pid in ids {
            let (px, pz, range) = {
                let p = &self.players[&pid];
                (
                    p.x,
                    p.z,
                    f64::from(p.view_distance.min(i32::from(self.cfg.simulation_distance)) * 16)
                        .min(limit.max(16.0)),
                )
            };
            let mut to_remove: Vec<VarInt> = Vec::new();
            let mut packets: Vec<Vec<u8>> = Vec::new();
            let mut spawn: Vec<i32> = Vec::new();
            {
                let p = &self.players[&pid];
                for item in self.items.values() {
                    let near =
                        (item.pos[0] - px).abs() <= range && (item.pos[2] - pz).abs() <= range;
                    let tracked = p.tracked_items.contains(&item.id);
                    if near && !tracked && !gone.contains(&item.id) {
                        spawn.push(item.id);
                    } else if tracked && (!near || gone.contains(&item.id)) {
                        to_remove.push(VarInt(item.id));
                    }
                }
                for id in &p.tracked_items {
                    if !self.items.contains_key(id) && !to_remove.iter().any(|v| v.0 == *id) {
                        to_remove.push(VarInt(*id));
                    }
                }
            }
            let _ = &mut packets;
            let p = self.players.get_mut(&pid).expect("exists");
            for id in &to_remove {
                p.tracked_items.remove(&id.0);
            }
            if !to_remove.is_empty() {
                send(&p.sink, &RemoveEntities { ids: LenVec(to_remove) });
            }
            for id in spawn {
                let item = &self.items[&id];
                send(
                    &p.sink,
                    &AddEntity {
                        id: VarInt(id),
                        uuid: item.uuid,
                        kind: Self::item_kind(),
                        x: item.pos[0],
                        y: item.pos[1],
                        z: item.pos[2],
                        velocity: LpVec3 { x: item.vel[0], y: item.vel[1], z: item.vel[2] },
                        x_rot: 0,
                        y_rot: 0,
                        y_head_rot: 0,
                        data: VarInt(0),
                    },
                );
                send(&p.sink, &Self::item_data_packet(item));
                p.tracked_items.insert(id);
            }
        }
        // Movement, once per item for everyone tracking it.
        let ids: Vec<i32> = self.items.keys().copied().collect();
        for id in ids {
            let (now, d) = {
                let it = &self.items[&id];
                let now = it.pos.map(quantize);
                (now, [now[0] - it.sent[0], now[1] - it.sent[1], now[2] - it.sent[2]])
            };
            if d == [0, 0, 0] {
                continue;
            }
            let it = self.items.get_mut(&id).expect("exists");
            it.sent = now;
            let on_ground = it.on_ground;
            let small = d.iter().all(|v| v.abs() <= 32767);
            let (pos, vel) = (it.pos, it.vel);
            for p in self.players.values() {
                if !p.tracked_items.contains(&id) {
                    continue;
                }
                if small {
                    send(
                        &p.sink,
                        &MoveEntityPos {
                            id: VarInt(id),
                            on_ground: OnGround(on_ground),
                            dx: d[0] as i16,
                            dy: d[1] as i16,
                            dz: d[2] as i16,
                        },
                    );
                } else {
                    send(
                        &p.sink,
                        &TeleportEntity {
                            id: VarInt(id),
                            x: pos[0],
                            y: pos[1],
                            z: pos[2],
                            dx: vel[0],
                            dy: vel[1],
                            dz: vel[2],
                            y_rot: 0.0,
                            x_rot: 0.0,
                            relative: 0,
                            on_ground,
                        },
                    );
                }
            }
        }
    }
}

// ---- inventory --------------------------------------------------------------------------

impl Game {
    /// Sends the whole inventory window and the cursor stack, as after any disagreement.
    pub(super) fn resync_inventory(&mut self, id: PlayerId) {
        let Some(p) = self.players.get_mut(&id) else { return };
        p.inv_state = (p.inv_state + 1) & 0x7FFF;
        send(
            &p.sink,
            &ContainerSetContent {
                window_id: VarInt(0),
                state_id: VarInt(p.inv_state),
                items: LenVec(p.menu.slots.to_vec()),
                carried: p.menu.carried,
            },
        );
    }

    pub(super) fn on_container_click(&mut self, id: PlayerId, c: &ContainerClick) {
        use crate::inventory::Input;
        if c.window_id.0 != 0 {
            return;
        }
        let Some(p) = self.players.get_mut(&id) else { return };
        let (Some(input), true) = (Input::from_id(c.click_type.0), p.game_mode != 3) else {
            return self.resync_inventory(id);
        };
        if c.state_id.0 != p.inv_state {
            // The client acted on a stale view: ignore the click and show the truth.
            return self.resync_inventory(id);
        }
        let creative = p.game_mode == 1;
        let drops = p.menu.click(i32::from(c.slot), i32::from(c.button), input, creative);
        // The client says what it believes happened; any difference means a resync.
        let agrees = c.changed.0.iter().all(|ch| {
            usize::try_from(ch.slot)
                .ok()
                .and_then(|s| p.menu.slots.get(s))
                .is_some_and(|ours| HashedStack::plain(*ours) == ch.item)
        }) && HashedStack::plain(p.menu.carried) == c.carried;
        for stack in drops {
            self.throw_item(id, stack, false);
        }
        if !agrees {
            self.resync_inventory(id);
        }
    }

    /// The player closes the inventory: the cursor stack and crafting grid fall to the floor.
    pub(super) fn close_inventory(&mut self, id: PlayerId) {
        let Some(p) = self.players.get_mut(&id) else { return };
        let mut drops = vec![std::mem::take(&mut p.menu.carried)];
        for slot in 1..=4 {
            drops.push(std::mem::take(&mut p.menu.slots[slot]));
        }
        for stack in drops.into_iter().filter(|s| s.count > 0) {
            self.throw_item(id, stack, false);
        }
    }

    /// The drop key: one item, or the whole stack with ctrl.
    pub(super) fn drop_held(&mut self, id: PlayerId, whole: bool) {
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.game_mode == 3 {
            return;
        }
        let slot = 36 + usize::from(p.held_slot);
        let held = p.menu.slots[slot];
        if held.count <= 0 {
            return;
        }
        let n = if whole { held.count } else { 1 };
        p.menu.slots[slot].count -= n;
        if p.menu.slots[slot].count <= 0 {
            p.menu.slots[slot] = ItemStack::default();
        }
        p.inv_state = (p.inv_state + 1) & 0x7FFF;
        send(
            &p.sink,
            &ContainerSetSlot {
                window_id: VarInt(0),
                state_id: VarInt(p.inv_state),
                slot: slot as i16,
                item: p.menu.slots[slot],
            },
        );
        self.throw_item(id, ItemStack { count: n, item: held.item }, true);
    }

    /// Drops a stack from the player's eyes the way the game does (`Player.drop`): thrown along
    /// the look direction, or just dropped in front when `forward` is false.
    pub(super) fn throw_item(&mut self, id: PlayerId, stack: ItemStack, _key: bool) {
        let Some(p) = self.players.get(&id) else { return };
        let (yaw, pitch) = (f64::from(p.yaw).to_radians(), f64::from(p.pitch).to_radians());
        let at = [p.x, p.y + 1.62 - 0.3, p.z];
        let speed = 0.3;
        let mut vel = [
            -yaw.sin() * pitch.cos() * speed,
            -pitch.sin() * speed + 0.1,
            yaw.cos() * pitch.cos() * speed,
        ];
        let angle = self.next_random() * std::f64::consts::TAU;
        let spread = 0.02 * self.next_random();
        vel[0] += angle.cos() * spread;
        vel[1] += (self.next_random() - self.next_random()) * 0.1;
        vel[2] += angle.sin() * spread;
        let eid = self.next_entity;
        self.next_entity += 1;
        let uuid = Uuid::from_u128(
            (u128::from(self.next_random().to_bits()) << 64)
                | u128::from(self.next_random().to_bits()),
        );
        let mut item = ItemEntity::new(eid, uuid, at, vel, stack);
        item.pickup_delay = 40;
        self.items.insert(eid, item);
    }
}
