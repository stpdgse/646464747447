//! Container windows: crafting tables, chests and barrels.

use ferrite_data::shapes::{facts, flag};

use super::*;
use crate::inventory::{Input, Layout, Menu};

/// What an open window is showing.
#[derive(Clone, Debug)]
pub(super) enum Source {
    Crafting,
    /// One chest, or the two halves of a double chest (first half's slots first).
    Chest(Vec<BlockPos>),
    Barrel(BlockPos),
}

#[derive(Clone, Debug)]
pub(super) struct Window {
    pub id: i32,
    pub menu: Menu,
    pub source: Source,
    pub state: i32,
}

const MENU_GENERIC_9X3: i32 = 2;
const MENU_GENERIC_9X6: i32 = 5;
const MENU_CRAFTING: i32 = 12;

fn title(key: &str) -> Text {
    let mut c = ferrite_nbt::Compound::new();
    c.0.insert("translate".into(), ferrite_nbt::Tag::String(key.into()));
    Text(ferrite_nbt::Tag::Compound(c))
}

impl Game {
    /// The block name and a property of the state at `pos`.
    fn name_and_prop(
        &self,
        pos: BlockPos,
        prop: &str,
    ) -> Option<(&'static str, Option<&'static str>)> {
        let state = self.world.block(pos)?;
        let name =
            ferrite_data::blocks::block_of_state(state)?.name.trim_start_matches("minecraft:");
        Some((name, ferrite_blocks::Kinds::get().property(state, prop)))
    }

    /// The other half of a double chest, if `pos` is part of one.
    fn chest_partner(&self, pos: BlockPos) -> Option<BlockPos> {
        let state = self.world.block(pos)?;
        let kinds = ferrite_blocks::Kinds::get();
        let facing =
            kinds.property(state, "facing").and_then(ferrite_blocks::Direction::from_name)?;
        let side = match kinds.property(state, "type")? {
            "left" => facing.clockwise(),
            "right" => facing.counter_clockwise(),
            _ => return None,
        };
        let other = pos.offset(side);
        let (a, b) = (self.world.block(pos)?, self.world.block(other)?);
        (ferrite_blocks::kinds_block_id(a) == ferrite_blocks::kinds_block_id(b)).then_some(other)
    }

    /// Opens the window of the container at `pos` for the player, if it is one and not blocked.
    /// Returns whether the block was a container (the click is then used up either way).
    pub(super) fn try_open_container(&mut self, id: PlayerId, pos: BlockPos) -> bool {
        let Some((name, ty)) = self.name_and_prop(pos, "type") else { return false };
        match name {
            "crafting_table" => {
                self.open_window(
                    id,
                    Source::Crafting,
                    Layout::Crafting,
                    MENU_CRAFTING,
                    "container.crafting",
                );
                true
            }
            "chest" | "trapped_chest" => {
                let blocked = |g: &Game, p: BlockPos| {
                    g.world.block(p.above()).is_some_and(|s| facts(s).has(flag::REDSTONE_CONDUCTOR))
                };
                let mut halves = vec![pos];
                if ty.is_some_and(|t| t != "single")
                    && let Some(other) = self.chest_partner(pos)
                {
                    // The "right" half's slots come first.
                    if ty == Some("right") { halves.push(other) } else { halves.insert(0, other) }
                }
                if halves.iter().any(|p| blocked(self, *p)) {
                    return true;
                }
                let (layout, menu, key) = if halves.len() == 2 {
                    (Layout::Generic(6), MENU_GENERIC_9X6, "container.chestDouble")
                } else {
                    (Layout::Generic(3), MENU_GENERIC_9X3, "container.chest")
                };
                self.open_window(id, Source::Chest(halves), layout, menu, key);
                true
            }
            "barrel" => {
                self.open_window(
                    id,
                    Source::Barrel(pos),
                    Layout::Generic(3),
                    MENU_GENERIC_9X3,
                    "container.barrel",
                );
                true
            }
            _ => false,
        }
    }

    fn container_positions(source: &Source) -> Vec<BlockPos> {
        match source {
            Source::Crafting => Vec::new(),
            Source::Chest(p) => p.clone(),
            Source::Barrel(p) => vec![*p],
        }
    }

    fn open_window(
        &mut self,
        id: PlayerId,
        source: Source,
        layout: Layout,
        menu_type: i32,
        key: &str,
    ) {
        if self.players.get(&id).is_some_and(|p| p.window.is_some()) {
            self.close_window(id);
        }
        let positions = Self::container_positions(&source);
        let mut contents: Vec<ItemStack> = Vec::new();
        for pos in &positions {
            let stored =
                self.containers.entry(*pos).or_insert_with(|| vec![ItemStack::default(); 27]);
            contents.extend(stored.iter().copied());
        }
        let Some(p) = self.players.get_mut(&id) else { return };
        p.next_window = if p.next_window >= 100 { 1 } else { p.next_window + 1 };
        let wid = i32::from(p.next_window);
        let mut menu = Menu::opened_over(layout, &p.menu);
        for (i, item) in contents.into_iter().enumerate() {
            menu.slots[i] = item;
        }
        menu.refresh_result();
        send(
            &p.sink,
            &OpenScreen {
                container_id: VarInt(wid),
                menu_type: VarInt(menu_type),
                title: title(key),
            },
        );
        send(
            &p.sink,
            &ContainerSetContent {
                window_id: VarInt(wid),
                state_id: VarInt(1),
                items: LenVec(menu.slots.clone()),
                carried: ItemStack::default(),
            },
        );
        p.window = Some(Window { id: wid, menu, source: source.clone(), state: 1 });
        for pos in positions {
            self.adjust_viewers(pos, 1);
        }
    }

    /// Chest lids open while anyone looks inside.
    fn adjust_viewers(&mut self, pos: BlockPos, delta: i16) {
        let Some(state) = self.world.block(pos) else { return };
        if !matches!(
            ferrite_data::blocks::block_of_state(state).map(|b| b.name),
            Some("minecraft:chest" | "minecraft:trapped_chest")
        ) {
            return;
        }
        let n = self.viewers.entry(pos).or_insert(0);
        *n = (i16::from(*n) + delta).max(0) as u8;
        let count = *n;
        if count == 0 {
            self.viewers.remove(&pos);
        }
        let packet = BlockEvent {
            pos: PackedPos::new(pos.x, pos.y, pos.z),
            kind: 1,
            data: count,
            block: VarInt(i32::from(ferrite_blocks::kinds_block_id(state))),
        };
        for p in self.players.values() {
            send(&p.sink, &packet);
        }
    }

    /// Copies the window's container slots back into storage.
    fn store_window(&mut self, id: PlayerId) {
        let Some(w) = self.players.get(&id).and_then(|p| p.window.as_ref()) else { return };
        let positions = Self::container_positions(&w.source);
        for (i, pos) in positions.iter().enumerate() {
            if let Some(stored) = self.containers.get_mut(pos) {
                stored.copy_from_slice(&w.menu.slots[i * 27..(i + 1) * 27]);
            }
            self.dirty.insert((pos.x >> 4, pos.z >> 4));
        }
    }

    pub(super) fn window_click(&mut self, id: PlayerId, c: &ContainerClick) {
        let Some(p) = self.players.get_mut(&id) else { return };
        let Some(w) = p.window.as_mut().filter(|w| w.id == c.window_id.0) else { return };
        let Some(input) = Input::from_id(c.click_type.0) else { return };
        if c.state_id.0 != w.state {
            return self.resync_window(id);
        }
        let creative = p.game_mode == 1;
        let drops = w.menu.click(i32::from(c.slot), i32::from(c.button), input, creative);
        let agrees = c.changed.0.iter().all(|ch| {
            usize::try_from(ch.slot)
                .ok()
                .and_then(|s| w.menu.slots.get(s))
                .is_some_and(|ours| HashedStack::plain(*ours) == ch.item)
        }) && HashedStack::plain(w.menu.carried) == c.carried;
        w.menu.write_back(&mut p.menu);
        p.menu.refresh_result();
        self.store_window(id);
        for stack in drops {
            self.throw_item(id, stack, false);
        }
        if !agrees {
            self.resync_window(id);
        }
    }

    pub(super) fn resync_window(&mut self, id: PlayerId) {
        let Some(p) = self.players.get_mut(&id) else { return };
        let Some(w) = p.window.as_mut() else { return };
        w.state = (w.state + 1) & 0x7FFF;
        send(
            &p.sink,
            &ContainerSetContent {
                window_id: VarInt(w.id),
                state_id: VarInt(w.state),
                items: LenVec(w.menu.slots.clone()),
                carried: w.menu.carried,
            },
        );
    }

    /// Closes the player's open window: the cursor stack (and a crafting table's grid) go back
    /// to the inventory or onto the floor.
    pub(super) fn close_window(&mut self, id: PlayerId) {
        let Some(p) = self.players.get_mut(&id) else { return };
        let Some(mut w) = p.window.take() else { return };
        let mut loose = vec![std::mem::take(&mut w.menu.carried)];
        if w.menu.layout == Layout::Crafting {
            for cell in &mut w.menu.slots[1..=9] {
                loose.push(std::mem::take(cell));
            }
        }
        w.menu.write_back(&mut p.menu);
        p.menu.refresh_result();
        let held = usize::from(p.held_slot);
        let mut floor = Vec::new();
        for mut stack in loose.into_iter().filter(|s| s.count > 0) {
            crate::items::add_to_inventory(&mut p.menu.slots, held, &mut stack);
            if stack.count > 0 {
                floor.push(stack);
            }
        }
        send(&p.sink, &CloseContainer { container_id: VarInt(w.id) });
        // The player's inventory may have changed under the window; show it.
        let positions = Self::container_positions(&w.source);
        self.resync_inventory(id);
        for stack in floor {
            self.throw_item(id, stack, false);
        }
        for pos in positions {
            self.adjust_viewers(pos, -1);
        }
    }

    /// Saves everything that changed: dirty chunks with their containers, every player, the clock.
    pub(super) fn save_all(&mut self) {
        let Some(persist) = &self.persist else { return };
        let dirty: Vec<(i32, i32)> = self.dirty.drain().collect();
        for (cx, cz) in dirty {
            let pos = ferrite_world::ChunkPos::new(cx, cz);
            if let Some(chunk) = self.world.blocks.get(pos)
                && let Err(e) = persist.save_chunk(pos, chunk)
            {
                tracing::error!("cannot save chunk {cx},{cz}: {e}");
            }
            let inside: Vec<(BlockPos, &[ItemStack])> = self
                .containers
                .iter()
                .filter(|(p, _)| (p.x >> 4, p.z >> 4) == (cx, cz))
                .map(|(p, v)| (*p, v.as_slice()))
                .collect();
            if let Err(e) = persist.save_containers(pos, &inside) {
                tracing::error!("cannot save the containers of chunk {cx},{cz}: {e}");
            }
        }
        let items: Vec<crate::persist::ItemSave> = self
            .items
            .values()
            .map(|i| crate::persist::ItemSave {
                pos: i.pos,
                vel: i.vel,
                age: i.age,
                pickup_delay: i.pickup_delay,
                stack: i.stack,
            })
            .collect();
        if let Err(e) = persist.save_items(&items) {
            tracing::error!("cannot save dropped items: {e}");
        }
        if let Err(e) = persist.save_time(self.world.time()) {
            tracing::error!("cannot save the clock: {e}");
        }
        let ids: Vec<PlayerId> = self.order.clone();
        for id in ids {
            self.save_player(id);
        }
        if let Some(persist) = &self.persist
            && let Err(e) = persist.sync()
        {
            tracing::error!("cannot sync the world: {e}");
        }
    }

    pub(super) fn save_player(&self, id: PlayerId) {
        let (Some(persist), Some(p)) = (&self.persist, self.players.get(&id)) else { return };
        let slots = p
            .menu
            .slots
            .iter()
            .enumerate()
            .filter(|(i, s)| *i >= 5 && s.count > 0)
            .map(|(i, s)| (i as u8, *s))
            .collect();
        let save = crate::persist::PlayerSave {
            pos: [p.x, p.y, p.z],
            yaw: p.yaw,
            pitch: p.pitch,
            game_mode: p.game_mode,
            held_slot: p.held_slot,
            slots,
            health: p.health,
            food: p.food,
            saturation: p.saturation,
        };
        if let Err(e) = persist.save_player(p.uuid, &save) {
            tracing::error!("cannot save player {}: {e}", p.name);
        }
    }

    /// What was inside a container that is gone: it spills out as items.
    pub(super) fn spill_container(&mut self, pos: BlockPos) {
        let Some(items) = self.containers.remove(&pos) else { return };
        self.dirty.insert((pos.x >> 4, pos.z >> 4));
        for stack in items.into_iter().filter(|s| s.count > 0) {
            self.drop_item(pos, stack);
        }
    }
}
