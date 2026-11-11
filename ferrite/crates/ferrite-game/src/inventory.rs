//! The player inventory window and the clicks that move stacks around in it.
//!
//! The layout is vanilla's window 0: slot 0 the crafting result, 1-4 the crafting grid, 5-8 armor
//! (head to feet), 9-35 the main inventory, 36-44 the hotbar, 45 the off hand. The server applies
//! every click itself and tells the client when its own result differed.

use ferrite_data::items::{EquipSlot, item_info};
use ferrite_protocol::packets::play::ItemStack;

pub const SLOTS: usize = 46;

/// The click types of the `container_click` packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Pickup,
    QuickMove,
    Swap,
    Clone,
    Throw,
    QuickCraft,
    PickupAll,
}

impl Input {
    pub fn from_id(id: i32) -> Option<Input> {
        Some(match id {
            0 => Input::Pickup,
            1 => Input::QuickMove,
            2 => Input::Swap,
            3 => Input::Clone,
            4 => Input::Throw,
            5 => Input::QuickCraft,
            6 => Input::PickupAll,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Default)]
struct Drag {
    kind: u8,
    slots: Vec<usize>,
}

/// Which window a [`Menu`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// The player's own inventory (window 0).
    Player,
    /// A crafting table.
    Crafting,
    /// A chest-like container with this many rows of nine slots.
    Generic(usize),
}

/// Where the parts of a window sit.
#[derive(Clone, Debug)]
struct Geometry {
    result: Option<usize>,
    grid: std::ops::Range<usize>,
    grid_width: usize,
    armor: std::ops::Range<usize>,
    main: std::ops::Range<usize>,
    hotbar: std::ops::Range<usize>,
    offhand: Option<usize>,
}

impl Layout {
    fn geometry(self) -> Geometry {
        match self {
            Layout::Player => Geometry {
                result: Some(0),
                grid: 1..5,
                grid_width: 2,
                armor: 5..9,
                main: 9..36,
                hotbar: 36..45,
                offhand: Some(45),
            },
            Layout::Crafting => Geometry {
                result: Some(0),
                grid: 1..10,
                grid_width: 3,
                armor: 0..0,
                main: 10..37,
                hotbar: 37..46,
                offhand: None,
            },
            Layout::Generic(rows) => Geometry {
                result: None,
                grid: 0..0,
                grid_width: 0,
                armor: 0..0,
                main: 9 * rows..9 * rows + 27,
                hotbar: 9 * rows + 27..9 * rows + 36,
                offhand: None,
            },
        }
    }

    pub fn slot_count(self) -> usize {
        match self {
            Layout::Player | Layout::Crafting => 46,
            Layout::Generic(rows) => 9 * rows + 36,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Menu {
    pub layout: Layout,
    pub slots: Vec<ItemStack>,
    pub carried: ItemStack,
    /// The player's off-hand item, which number-key swaps can reach even when the window has no
    /// off-hand slot.
    pub offhand: ItemStack,
    drag: Option<Drag>,
}

impl Default for Menu {
    fn default() -> Self {
        Menu::new(Layout::Player)
    }
}

impl Menu {
    pub fn new(layout: Layout) -> Self {
        Menu {
            layout,
            slots: vec![ItemStack::default(); layout.slot_count()],
            carried: ItemStack::default(),
            offhand: ItemStack::default(),
            drag: None,
        }
    }

    fn geo(&self) -> Geometry {
        self.layout.geometry()
    }

    /// Opens `layout` over the player's inventory: its main and hotbar slots are copied in.
    pub fn opened_over(layout: Layout, player: &Menu) -> Menu {
        let mut m = Menu::new(layout);
        let (g, pg) = (m.geo(), player.geo());
        for (to, from) in g.main.clone().chain(g.hotbar.clone()).zip(pg.main.chain(pg.hotbar)) {
            m.slots[to] = player.slots[from];
        }
        m.offhand = player.slots[45];
        m
    }

    /// Writes the player's own slots back after clicks in an opened window.
    pub fn write_back(&self, player: &mut Menu) {
        let (g, pg) = (self.geo(), player.geo());
        for (from, to) in g.main.clone().chain(g.hotbar.clone()).zip(pg.main.chain(pg.hotbar)) {
            player.slots[to] = self.slots[from];
        }
        player.slots[45] = self.offhand;
    }
}

fn empty(s: ItemStack) -> bool {
    s.count <= 0
}

fn max_stack(item: i32) -> i32 {
    crate::items::max_stack(item)
}

fn same(a: ItemStack, b: ItemStack) -> bool {
    !empty(a) && !empty(b) && a.item == b.item
}

fn split(stack: &mut ItemStack, n: i32) -> ItemStack {
    let n = n.min(stack.count).max(0);
    let out = ItemStack { count: n, item: stack.item };
    stack.count -= n;
    if stack.count <= 0 {
        *stack = ItemStack::default();
    }
    if n == 0 { ItemStack::default() } else { out }
}

/// Which armor slot (5-8) an item belongs in.
fn armor_slot(item: i32) -> Option<usize> {
    match item_info(item)?.equip? {
        EquipSlot::Head => Some(5),
        EquipSlot::Chest => Some(6),
        EquipSlot::Legs => Some(7),
        EquipSlot::Feet => Some(8),
        _ => None,
    }
}

impl Menu {
    fn may_place(&self, slot: usize, stack: ItemStack) -> bool {
        let g = self.geo();
        if g.result == Some(slot) {
            return false;
        }
        if g.armor.contains(&slot) {
            return armor_slot(stack.item) == Some(slot);
        }
        true
    }

    fn may_pickup(&self, _slot: usize) -> bool {
        true
    }

    fn slot_max(&self, slot: usize, item: i32) -> i32 {
        if self.geo().armor.contains(&slot) { 1 } else { max_stack(item) }
    }

    /// Recomputes the crafting result from the 2x2 grid.
    pub fn refresh_result(&mut self) {
        let g = self.geo();
        let Some(result) = g.result else { return };
        let grid: Vec<ferrite_crafting::Stack> = self.slots[g.grid.clone()]
            .iter()
            .map(|s| ferrite_crafting::Stack { item: s.item, count: s.count })
            .collect();
        self.slots[result] = match ferrite_crafting::craft(&grid, g.grid_width, g.grid_width) {
            Some((_, out)) => ItemStack { count: out.count, item: out.item },
            None => ItemStack::default(),
        };
    }

    /// Takes one craft's worth out of the grid (the game's `ResultSlot.onTake`).
    fn consume_ingredients(&mut self) {
        let grid = self.geo().grid;
        for cell in &mut self.slots[grid] {
            if !empty(*cell) {
                cell.count -= 1;
                if cell.count <= 0 {
                    *cell = ItemStack::default();
                }
            }
        }
    }

    /// Applies one click. Returns the stacks the player drops as a result.
    pub fn click(
        &mut self,
        slot: i32,
        button: i32,
        input: Input,
        creative: bool,
    ) -> Vec<ItemStack> {
        let drops = self.click_inner(slot, button, input, creative);
        self.refresh_result();
        drops
    }

    fn click_inner(
        &mut self,
        slot: i32,
        button: i32,
        input: Input,
        creative: bool,
    ) -> Vec<ItemStack> {
        let mut drops = Vec::new();
        if input == Input::QuickCraft {
            self.quick_craft(slot, button, &mut drops);
            return drops;
        }
        self.drag = None;
        match input {
            Input::Pickup => self.pickup(slot, button, &mut drops),
            Input::QuickMove => {
                if let Ok(s) = usize::try_from(slot)
                    && s < self.slots.len()
                {
                    let item = self.slots[s].item;
                    let mut moved = self.quick_move(s);
                    while !empty(moved) && moved.item == item {
                        moved = self.quick_move(s);
                    }
                }
            }
            Input::Swap => {
                if let Ok(s) = usize::try_from(slot)
                    && s < self.slots.len()
                    && (0..9).contains(&button) | (button == 40)
                {
                    self.swap(s, button, &mut drops);
                }
            }
            Input::Clone => {
                if creative
                    && empty(self.carried)
                    && let Ok(s) = usize::try_from(slot)
                    && s < self.slots.len()
                    && !empty(self.slots[s])
                {
                    let item = self.slots[s].item;
                    self.carried = ItemStack { count: max_stack(item), item };
                }
            }
            Input::Throw => {
                if empty(self.carried)
                    && let Ok(s) = usize::try_from(slot)
                    && s < self.slots.len()
                    && !empty(self.slots[s])
                    && self.may_pickup(s)
                {
                    let n = if button == 0 { 1 } else { self.slots[s].count };
                    drops.push(split(&mut self.slots[s], n));
                }
            }
            Input::PickupAll => self.pickup_all(slot, button),
            Input::QuickCraft => {}
        }
        drops
    }

    fn pickup(&mut self, slot: i32, button: i32, drops: &mut Vec<ItemStack>) {
        if button != 0 && button != 1 {
            return;
        }
        if slot == -999 {
            if !empty(self.carried) {
                let n = if button == 0 { self.carried.count } else { 1 };
                drops.push(split(&mut self.carried, n));
            }
            return;
        }
        let Ok(s) = usize::try_from(slot) else { return };
        if s >= self.slots.len() {
            return;
        }
        if self.geo().result == Some(s) {
            return self.pickup_result(button);
        }
        let (a, c) = (self.slots[s], self.carried);
        if empty(a) {
            if !empty(c) && self.may_place(s, c) {
                let want = if button == 0 { c.count } else { 1 };
                let n = want.min(self.slot_max(s, c.item));
                self.slots[s] = split(&mut self.carried, n);
            }
        } else if !self.may_pickup(s) {
        } else if empty(c) {
            let n = if button == 0 { a.count } else { (a.count + 1) / 2 };
            self.carried = split(&mut self.slots[s], n);
        } else if self.may_place(s, c) {
            if same(a, c) {
                let want = if button == 0 { c.count } else { 1 };
                let room = (self.slot_max(s, a.item) - a.count).max(0);
                let n = want.min(room);
                self.slots[s].count += n;
                self.carried.count -= n;
                if self.carried.count <= 0 {
                    self.carried = ItemStack::default();
                }
            } else if c.count <= self.slot_max(s, c.item) {
                self.slots[s] = c;
                self.carried = a;
            }
        } else if same(a, c) {
            // Cannot place here, but the stacks match: take from the slot onto the cursor.
            let n = a.count.min(max_stack(c.item) - c.count).max(0);
            let taken = split(&mut self.slots[s], n);
            self.carried.count += taken.count;
        }
    }

    /// A click on the crafting result: it can only be taken whole, onto an empty cursor or one
    /// holding the same item with room for all of it.
    fn pickup_result(&mut self, button: i32) {
        let Some(result) = self.geo().result else { return };
        let (a, c) = (self.slots[result], self.carried);
        if empty(a) {
            return;
        }
        if empty(c) {
            self.carried = a;
            self.consume_ingredients();
        } else if same(a, c) && c.count + a.count <= max_stack(c.item) {
            self.carried.count += a.count;
            self.consume_ingredients();
        }
        let _ = button;
    }

    /// Moves stack `slot` to where shift-click sends it; returns what was moved (empty if
    /// nothing).
    fn quick_move(&mut self, slot: usize) -> ItemStack {
        if empty(self.slots[slot]) {
            return ItemStack::default();
        }
        let g = self.geo();
        let original = self.slots[slot];
        let mut stack = original;
        let inventory_end = g.hotbar.end;
        if g.result == Some(slot) {
            // Craft into the inventory (preferring the hotbar's end), consuming one craft.
            if !self.move_to(&mut stack, g.main.start, inventory_end, true) {
                return ItemStack::default();
            }
            if stack.count == original.count {
                return ItemStack::default();
            }
            self.consume_ingredients();
            self.refresh_result();
            return original;
        }
        let moved = match self.layout {
            Layout::Generic(rows) => {
                if slot < 9 * rows {
                    self.move_to(&mut stack, g.main.start, inventory_end, true)
                } else {
                    self.move_to(&mut stack, 0, 9 * rows, false)
                }
            }
            Layout::Crafting => {
                if g.grid.contains(&slot) {
                    self.move_to(&mut stack, g.main.start, inventory_end, false)
                } else {
                    self.move_main_hotbar(&mut stack, slot)
                }
            }
            Layout::Player => {
                if g.grid.contains(&slot) || g.armor.contains(&slot) || g.offhand == Some(slot) {
                    self.move_to(&mut stack, g.main.start, inventory_end, false)
                } else if let Some(armor) = armor_slot(stack.item).filter(|a| empty(self.slots[*a]))
                {
                    let m = self.move_to(&mut stack, armor, armor + 1, false);
                    m || self.move_main_hotbar(&mut stack, slot)
                } else if item_info(stack.item).and_then(|i| i.equip) == Some(EquipSlot::Offhand)
                    && empty(self.slots[45])
                {
                    self.move_to(&mut stack, 45, 46, false)
                        || self.move_main_hotbar(&mut stack, slot)
                } else {
                    self.move_main_hotbar(&mut stack, slot)
                }
            }
        };
        if !moved {
            return ItemStack::default();
        }
        self.slots[slot] = if stack.count <= 0 { ItemStack::default() } else { stack };
        original
    }

    /// Main inventory to hotbar and back (any other slot goes to either).
    fn move_main_hotbar(&mut self, stack: &mut ItemStack, slot: usize) -> bool {
        let g = self.geo();
        if g.main.contains(&slot) {
            self.move_to(stack, g.hotbar.start, g.hotbar.end, false)
        } else if g.hotbar.contains(&slot) {
            self.move_to(stack, g.main.start, g.main.end, false)
        } else {
            self.move_to(stack, g.main.start, g.hotbar.end, false)
        }
    }

    /// `moveItemStackTo`: first merge into matching stacks in `start..end`, then fill empty
    /// slots. Returns whether anything moved.
    fn move_to(&mut self, stack: &mut ItemStack, start: usize, end: usize, reverse: bool) -> bool {
        let order: Vec<usize> =
            if reverse { (start..end).rev().collect() } else { (start..end).collect() };
        let mut moved = false;
        if max_stack(stack.item) > 1 {
            for &s in &order {
                if stack.count <= 0 {
                    break;
                }
                let cur = self.slots[s];
                if same(cur, *stack) {
                    let room = self.slot_max(s, cur.item) - cur.count;
                    if room > 0 {
                        let n = room.min(stack.count);
                        self.slots[s].count += n;
                        stack.count -= n;
                        moved = true;
                    }
                }
            }
        }
        for &s in &order {
            if stack.count <= 0 {
                break;
            }
            if empty(self.slots[s]) && self.may_place(s, *stack) {
                let n = stack.count.min(self.slot_max(s, stack.item));
                self.slots[s] = ItemStack { count: n, item: stack.item };
                stack.count -= n;
                moved = true;
            }
        }
        moved
    }

    fn swap(&mut self, slot: usize, button: i32, drops: &mut Vec<ItemStack>) {
        let g = self.geo();
        if button == 40 && g.offhand.is_none() {
            return self.swap_offhand(slot, drops);
        }
        let other = if button == 40 { 45 } else { g.hotbar.start + button as usize };
        if other == slot {
            return;
        }
        let (a, b) = (self.slots[slot], self.slots[other]);
        if empty(a) && empty(b) {
            return;
        }
        if empty(b) {
            if self.may_pickup(slot) {
                self.slots[other] = a;
                self.slots[slot] = ItemStack::default();
            }
        } else if empty(a) {
            if self.may_place(slot, b) {
                let m = self.slot_max(slot, b.item);
                let mut rest = b;
                self.slots[slot] = split(&mut rest, m);
                self.slots[other] = rest;
            }
        } else if self.may_pickup(slot) && self.may_place(slot, b) {
            let m = self.slot_max(slot, b.item);
            if b.count > m {
                let mut rest = b;
                self.slots[slot] = split(&mut rest, m);
                self.slots[other] = rest;
                // The displaced stack goes into the inventory, or to the floor.
                let mut a = a;
                if !self.add_anywhere(&mut a) {
                    drops.push(a);
                }
            } else {
                self.slots[other] = a;
                self.slots[slot] = b;
            }
        }
    }

    /// Number-key 40 in a window that has no off-hand slot: swaps with the player's off hand.
    fn swap_offhand(&mut self, slot: usize, drops: &mut Vec<ItemStack>) {
        let (a, b) = (self.slots[slot], self.offhand);
        if empty(a) && empty(b) {
            return;
        }
        if empty(b) {
            self.offhand = a;
            self.slots[slot] = ItemStack::default();
        } else if empty(a) {
            if self.may_place(slot, b) {
                let m = self.slot_max(slot, b.item);
                let mut rest = b;
                self.slots[slot] = split(&mut rest, m);
                self.offhand = rest;
            }
        } else if self.may_place(slot, b) {
            let m = self.slot_max(slot, b.item);
            if b.count > m {
                let mut rest = b;
                self.slots[slot] = split(&mut rest, m);
                self.offhand = rest;
                let mut a = a;
                if !self.add_anywhere(&mut a) {
                    drops.push(a);
                }
            } else {
                self.offhand = a;
                self.slots[slot] = b;
            }
        }
    }

    /// Adds to the main inventory and hotbar (hotbar first), as `Inventory.add` does.
    fn add_anywhere(&mut self, stack: &mut ItemStack) -> bool {
        let g = self.geo();
        // Gather the player's slots in the add() layout, add, and put them back.
        let mut inv = [ItemStack::default(); 46];
        for (i, s) in g.main.clone().enumerate() {
            inv[9 + i] = self.slots[s];
        }
        for (i, s) in g.hotbar.clone().enumerate() {
            inv[36 + i] = self.slots[s];
        }
        crate::items::add_to_inventory(&mut inv, 0, stack);
        for (i, s) in g.main.clone().enumerate() {
            self.slots[s] = inv[9 + i];
        }
        for (i, s) in g.hotbar.clone().enumerate() {
            self.slots[s] = inv[36 + i];
        }
        stack.count <= 0
    }

    fn pickup_all(&mut self, slot: i32, button: i32) {
        let Ok(s) = usize::try_from(slot) else { return };
        if s >= self.slots.len() || empty(self.carried) {
            return;
        }
        if !(empty(self.slots[s]) || !self.may_pickup(s)) {
            return;
        }
        let n = self.slots.len();
        let order: Vec<usize> = if button == 0 { (0..n).collect() } else { (0..n).rev().collect() };
        let max = max_stack(self.carried.item);
        for pass in 0..2 {
            for &i in &order {
                if self.carried.count >= max {
                    return;
                }
                let cur = self.slots[i];
                if self.geo().result == Some(i) || !same(cur, self.carried) || !self.may_pickup(i) {
                    continue;
                }
                if pass == 0 && cur.count == max_stack(cur.item) {
                    continue;
                }
                let n = cur.count.min(max - self.carried.count);
                let taken = split(&mut self.slots[i], n);
                self.carried.count += taken.count;
            }
        }
    }

    fn quick_craft(&mut self, slot: i32, button: i32, drops: &mut Vec<ItemStack>) {
        let header = button & 3;
        let kind = ((button >> 2) & 3) as u8;
        match header {
            0 => {
                self.drag = (kind <= 2).then(|| Drag { kind, slots: Vec::new() });
            }
            1 => {
                let Ok(s) = usize::try_from(slot) else { return };
                if s >= self.slots.len() {
                    return;
                }
                let (carried, cur) = (self.carried, self.slots[s]);
                let fits = empty(cur) || same(cur, carried);
                let placeable = self.geo().result != Some(s) && self.may_place(s, carried);
                let Some(drag) = self.drag.as_mut() else { return };
                if !empty(carried)
                    && fits
                    && placeable
                    && (drag.kind == 2 || carried.count > drag.slots.len() as i32)
                    && !drag.slots.contains(&s)
                {
                    drag.slots.push(s);
                }
            }
            2 => {
                let Some(drag) = self.drag.take() else { return };
                if drag.slots.is_empty() {
                    return;
                }
                if drag.slots.len() == 1 {
                    self.pickup(drag.slots[0] as i32, i32::from(drag.kind), drops);
                    return;
                }
                let mut remaining = self.carried.count;
                let item = self.carried.item;
                for &s in &drag.slots {
                    let existing = if empty(self.slots[s]) { 0 } else { self.slots[s].count };
                    let mut count = match drag.kind {
                        0 => self.carried.count / drag.slots.len() as i32,
                        1 => 1,
                        _ => max_stack(item),
                    };
                    count += existing;
                    count = count.min(self.slot_max(s, item));
                    remaining -= count - existing;
                    self.slots[s] = ItemStack { count, item };
                }
                self.carried.count = remaining;
                if remaining <= 0 {
                    self.carried = ItemStack::default();
                }
            }
            _ => self.drag = None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STONE: i32 = 1;
    const DIRT: i32 = 2;

    fn stack(item: i32, count: i32) -> ItemStack {
        ItemStack { count, item }
    }

    fn menu() -> Menu {
        Menu::default()
    }

    #[test]
    fn left_click_picks_up_and_puts_down_whole_stacks() {
        let mut m = menu();
        m.slots[36] = stack(STONE, 10);
        m.click(36, 0, Input::Pickup, false);
        assert_eq!((m.carried, m.slots[36].count), (stack(STONE, 10), 0));
        m.click(10, 0, Input::Pickup, false);
        assert_eq!((m.slots[10], m.carried.count), (stack(STONE, 10), 0));
    }

    #[test]
    fn right_click_takes_half_rounded_up_and_places_one() {
        let mut m = menu();
        m.slots[9] = stack(STONE, 5);
        m.click(9, 1, Input::Pickup, false);
        assert_eq!((m.carried.count, m.slots[9].count), (3, 2));
        m.click(10, 1, Input::Pickup, false);
        assert_eq!((m.carried.count, m.slots[10].count), (2, 1));
    }

    #[test]
    fn different_items_swap_and_equal_ones_merge() {
        let mut m = menu();
        m.slots[9] = stack(STONE, 5);
        m.carried = stack(DIRT, 3);
        m.click(9, 0, Input::Pickup, false);
        assert_eq!((m.slots[9], m.carried), (stack(DIRT, 3), stack(STONE, 5)));
        m.slots[10] = stack(STONE, 60);
        m.click(10, 0, Input::Pickup, false);
        assert_eq!((m.slots[10].count, m.carried.count), (64, 1));
    }

    #[test]
    fn clicking_outside_drops_the_cursor_stack() {
        let mut m = menu();
        m.carried = stack(STONE, 5);
        assert_eq!(m.click(-999, 1, Input::Pickup, false), vec![stack(STONE, 1)]);
        assert_eq!(m.click(-999, 0, Input::Pickup, false), vec![stack(STONE, 4)]);
        assert!(m.carried.count == 0);
    }

    #[test]
    fn shift_click_moves_between_hotbar_and_main_inventory() {
        let mut m = menu();
        m.slots[36] = stack(STONE, 10);
        m.click(36, 0, Input::QuickMove, false);
        assert_eq!((m.slots[36].count, m.slots[9]), (0, stack(STONE, 10)));
        m.click(9, 0, Input::QuickMove, false);
        assert_eq!((m.slots[9].count, m.slots[36]), (0, stack(STONE, 10)));
    }

    #[test]
    fn number_keys_swap_with_the_hotbar() {
        let mut m = menu();
        m.slots[9] = stack(STONE, 4);
        m.slots[38] = stack(DIRT, 2);
        m.click(9, 2, Input::Swap, false);
        assert_eq!((m.slots[9], m.slots[38]), (stack(DIRT, 2), stack(STONE, 4)));
    }

    #[test]
    fn q_throws_one_or_the_whole_stack() {
        let mut m = menu();
        m.slots[36] = stack(STONE, 5);
        assert_eq!(m.click(36, 0, Input::Throw, false), vec![stack(STONE, 1)]);
        assert_eq!(m.click(36, 1, Input::Throw, false), vec![stack(STONE, 4)]);
    }

    #[test]
    fn dragging_spreads_the_cursor_stack() {
        let mut m = menu();
        m.carried = stack(STONE, 10);
        m.click(-999, 0, Input::QuickCraft, false); // start, left-drag
        for s in [9, 10, 11] {
            m.click(s, 1, Input::QuickCraft, false);
        }
        m.click(-999, 2, Input::QuickCraft, false);
        assert_eq!(
            (m.slots[9].count, m.slots[10].count, m.slots[11].count, m.carried.count),
            (3, 3, 3, 1)
        );
        // A right-drag puts one in each.
        let mut m = menu();
        m.carried = stack(STONE, 10);
        m.click(-999, 4, Input::QuickCraft, false);
        for s in [9, 10] {
            m.click(s, 5, Input::QuickCraft, false);
        }
        m.click(-999, 6, Input::QuickCraft, false);
        assert_eq!((m.slots[9].count, m.slots[10].count, m.carried.count), (1, 1, 8));
    }

    #[test]
    fn double_click_gathers_matching_stacks() {
        let mut m = menu();
        m.carried = stack(STONE, 1);
        m.slots[9] = stack(STONE, 10);
        m.slots[10] = stack(STONE, 64);
        m.slots[11] = stack(DIRT, 5);
        m.click(20, 0, Input::PickupAll, false);
        assert_eq!(m.carried.count, 64);
        assert_eq!((m.slots[9].count, m.slots[10].count, m.slots[11].count), (0, 11, 5));
    }

    #[test]
    fn armor_only_goes_in_its_own_slot() {
        let helmet = ferrite_data::registries::item::by_name["minecraft:iron_helmet"] as i32;
        let mut m = menu();
        m.carried = stack(helmet, 1);
        m.click(6, 0, Input::Pickup, false); // chest slot
        assert_eq!(m.slots[6].count, 0);
        m.click(5, 0, Input::Pickup, false);
        assert_eq!(m.slots[5], stack(helmet, 1));
        // Shift-click from the hotbar equips it.
        let mut m = menu();
        m.slots[36] = stack(helmet, 1);
        m.click(36, 0, Input::QuickMove, false);
        assert_eq!(m.slots[5], stack(helmet, 1));
    }
}
