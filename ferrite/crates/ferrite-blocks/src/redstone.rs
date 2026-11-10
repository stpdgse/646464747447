//! Redstone power and the components built on it.
//!
//! Power follows vanilla's two-level model. A *weak* signal is what a block emits toward a
//! neighbour; a *strong* ("direct") signal additionally powers a conductor block, which then
//! passes it on to everything touching it. [`signal`] is the combined query components use.

use ferrite_data::blocks::STATE_TO_BLOCK;
use ferrite_data::shapes::{Support as DataSupport, facts, flag};

use crate::kinds::{Kinds, Redstone};
use crate::level::BlockAccess;
use crate::pos::{BlockPos, Direction};
use crate::world::{World, update};

/// A repeater's tick priorities (vanilla's `TickPriority` values).
const HIGH: i8 = -1;
const VERY_HIGH: i8 = -2;
const EXTREMELY_HIGH: i8 = -3;

fn block_of(state: u16) -> u16 {
    STATE_TO_BLOCK[usize::from(state)]
}

fn prop(state: u16, name: &str) -> Option<&'static str> {
    Kinds::get().property(state, name)
}

fn is_true(state: u16, name: &str) -> bool {
    prop(state, name) == Some("true")
}

fn facing(state: u16) -> Direction {
    prop(state, "facing").and_then(Direction::from_name).expect("component has a facing")
}

fn set_prop(state: u16, name: &str, value: &str) -> u16 {
    Kinds::get().with_property(state, name, value).expect("component has the property")
}

fn bool_str(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

/// The direction a face-attached block (lever, button) points toward the block it is attached
/// to's *opposite*: up for floor mounting, down for ceiling, its facing on a wall.
pub(crate) fn attached_direction(state: u16) -> Direction {
    match prop(state, "face") {
        Some("floor") => Direction::Up,
        Some("ceiling") => Direction::Down,
        _ => facing(state),
    }
}

fn redstone_of(state: u16) -> Option<Redstone> {
    Kinds::get().of_state(state).redstone
}

// ---- power queries -----------------------------------------------------------------------

/// What the block `state` emits toward a neighbour that is `dir` away from the neighbour's
/// point of view (the neighbour asks the block in direction `dir`).
fn state_signal(state: u16, dir: Direction) -> u8 {
    match redstone_of(state) {
        Some(Redstone::PowerBlock) => 15,
        Some(Redstone::Lever | Redstone::Button) => {
            if is_true(state, "powered") {
                15
            } else {
                0
            }
        }
        Some(Redstone::Torch) => {
            if is_true(state, "lit") && dir != Direction::Up {
                15
            } else {
                0
            }
        }
        Some(Redstone::WallTorch) => {
            if is_true(state, "lit") && facing(state) != dir {
                15
            } else {
                0
            }
        }
        Some(Redstone::Repeater) => state_direct_signal(state, dir),
        _ => 0,
    }
}

fn state_direct_signal(state: u16, dir: Direction) -> u8 {
    match redstone_of(state) {
        Some(Redstone::Lever | Redstone::Button) => {
            if is_true(state, "powered") && attached_direction(state) == dir { 15 } else { 0 }
        }
        Some(Redstone::Torch | Redstone::WallTorch) => {
            if dir == Direction::Down {
                state_signal(state, dir)
            } else {
                0
            }
        }
        Some(Redstone::Repeater) if is_true(state, "powered") && facing(state) == dir => 15,
        _ => 0,
    }
}

fn direct_signal_to<A: BlockAccess>(world: &World<A>, pos: BlockPos) -> u8 {
    let mut best = 0;
    for dir in Direction::ALL {
        if let Some(s) = world.block(pos.offset(dir)) {
            best = best.max(state_direct_signal(s, dir));
        }
    }
    best
}

/// `Level.getSignal`: the strongest signal reaching `pos` from the block in direction `dir`.
pub(crate) fn signal<A: BlockAccess>(world: &World<A>, pos: BlockPos, dir: Direction) -> u8 {
    let Some(state) = world.block(pos) else { return 0 };
    let s = state_signal(state, dir);
    if facts(state).has(flag::REDSTONE_CONDUCTOR) { s.max(direct_signal_to(world, pos)) } else { s }
}

pub(crate) fn has_neighbor_signal<A: BlockAccess>(world: &World<A>, pos: BlockPos) -> bool {
    Direction::ALL.into_iter().any(|d| signal(world, pos.offset(d), d) > 0)
}

// ---- hooks called by the world ------------------------------------------------------------

/// A redstone block was just placed (or its state changed) at `pos`.
pub(crate) fn on_place<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, state: u16, _old: u16) {
    match redstone_of(state) {
        Some(Redstone::Torch | Redstone::WallTorch) => notify_all_around(world, pos, state),
        Some(Redstone::Repeater) => update_neighbors_in_front(world, pos, state),
        _ => {}
    }
}

/// The redstone block `old` at `pos` was replaced by a different kind of block.
pub(crate) fn on_remove<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, old: u16) {
    match redstone_of(old) {
        Some(Redstone::Torch | Redstone::WallTorch) => notify_all_around(world, pos, old),
        Some(Redstone::Repeater) => update_neighbors_in_front(world, pos, old),
        Some(Redstone::Lever | Redstone::Button) if is_true(old, "powered") => {
            let block = block_of(old);
            world.update_neighbors_at(pos, block);
            world.update_neighbors_at(pos.offset(attached_direction(old).opposite()), block);
        }
        _ => {}
    }
}

fn notify_all_around<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, state: u16) {
    let block = block_of(state);
    for d in Direction::ALL {
        world.update_neighbors_at(pos.offset(d), block);
    }
}

pub(crate) fn neighbor_changed<A: BlockAccess>(
    world: &mut World<A>,
    pos: BlockPos,
    state: u16,
    _source_block: u16,
) {
    let block = block_of(state);
    match redstone_of(state) {
        Some(Redstone::Lamp) => {
            let lit = is_true(state, "lit");
            let powered = has_neighbor_signal(world, pos);
            if lit && !powered {
                world.schedule_block_tick(pos, block, 4, 0);
            } else if !lit && powered {
                world.set_block(pos, set_prop(state, "lit", "true"), update::CLIENTS);
            }
        }
        Some(Redstone::Torch | Redstone::WallTorch) => {
            if is_true(state, "lit") == torch_input(world, pos, state)
                && !world.will_tick_this_tick(pos, block)
            {
                world.schedule_block_tick(pos, block, 2, 0);
            }
        }
        Some(Redstone::Repeater) => {
            let below = pos.below();
            let supported = world.block(below).is_some_and(|s| {
                facts(s).is_face_sturdy(Direction::Up.index(), DataSupport::Rigid)
            });
            if supported {
                check_tick_on_neighbor(world, pos, state);
            } else {
                world.set_block(pos, Kinds::get().air, update::ALL);
                for d in Direction::ALL {
                    world.update_neighbors_at(pos.offset(d), block);
                }
            }
        }
        _ => {}
    }
}

pub(crate) fn tick<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, state: u16) {
    let block = block_of(state);
    match redstone_of(state) {
        Some(Redstone::Lamp) => {
            if is_true(state, "lit") && !has_neighbor_signal(world, pos) {
                world.set_block(pos, set_prop(state, "lit", "false"), update::CLIENTS);
            }
        }
        Some(Redstone::Torch | Redstone::WallTorch) => {
            let input = torch_input(world, pos, state);
            let now = world.time();
            world.toggles.retain(|(_, t)| now - *t <= 60);
            if is_true(state, "lit") {
                if input {
                    world.set_block(pos, set_prop(state, "lit", "false"), update::ALL);
                    if too_frequent(world, pos, true) {
                        world.schedule_block_tick(pos, block, 160, 0);
                    }
                }
            } else if !input && !too_frequent(world, pos, false) {
                world.set_block(pos, set_prop(state, "lit", "true"), update::ALL);
            }
        }
        Some(Redstone::Repeater) => repeater_tick(world, pos, state),
        _ => {}
    }
}

// ---- torches ----------------------------------------------------------------------------

/// Whether the block the torch is attached to is powered (which turns the torch off).
fn torch_input<A: BlockAccess>(world: &World<A>, pos: BlockPos, state: u16) -> bool {
    match redstone_of(state) {
        Some(Redstone::WallTorch) => {
            let f = facing(state);
            signal(world, pos.offset(f.opposite()), f) > 0
        }
        _ => signal(world, pos.below(), Direction::Down) > 0,
    }
}

fn too_frequent<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, add: bool) -> bool {
    let now = world.time();
    if add {
        world.toggles.push((pos, now));
    }
    world.toggles.iter().filter(|(p, _)| *p == pos).count() >= 8
}

// ---- repeaters --------------------------------------------------------------------------

fn delay_ticks(state: u16) -> i64 {
    prop(state, "delay").and_then(|d| d.parse::<i64>().ok()).unwrap_or(1) * 2
}

fn update_neighbors_in_front<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, state: u16) {
    let out = facing(state).opposite();
    let front = pos.offset(out);
    let block = block_of(state);
    world.neighbor_changed_at(front, block);
    world.update_neighbors_at_except(front, block, out.opposite());
}

fn input_signal<A: BlockAccess>(world: &World<A>, pos: BlockPos, state: u16) -> u8 {
    let f = facing(state);
    let front = pos.offset(f);
    let s = signal(world, front, f);
    if s >= 15 {
        return s;
    }
    s
}

fn alternate_signal<A: BlockAccess>(world: &World<A>, pos: BlockPos, state: u16) -> u8 {
    let f = facing(state);
    let mut best = 0;
    for side in [f.clockwise(), f.counter_clockwise()] {
        let at = pos.offset(side);
        if let Some(s) = world.block(at)
            && redstone_of(s) == Some(Redstone::Repeater)
        {
            best = best.max(signal(world, at, side));
        }
    }
    best
}

pub(crate) fn repeater_locked<A: BlockAccess>(world: &World<A>, pos: BlockPos, state: u16) -> bool {
    alternate_signal(world, pos, state) > 0
}

fn check_tick_on_neighbor<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, state: u16) {
    if repeater_locked(world, pos, state) {
        return;
    }
    let powered = is_true(state, "powered");
    let should = input_signal(world, pos, state) > 0;
    let block = block_of(state);
    if powered != should && !world.will_tick_this_tick(pos, block) {
        let mut priority = HIGH;
        if should_prioritize(world, pos, state) {
            priority = EXTREMELY_HIGH;
        } else if powered {
            priority = VERY_HIGH;
        }
        world.schedule_block_tick(pos, block, delay_ticks(state), priority);
    }
}

fn should_prioritize<A: BlockAccess>(world: &World<A>, pos: BlockPos, state: u16) -> bool {
    let opposite = facing(state).opposite();
    world
        .block(pos.offset(opposite))
        .is_some_and(|b| redstone_of(b) == Some(Redstone::Repeater) && facing(b) != opposite)
}

fn repeater_tick<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, state: u16) {
    if repeater_locked(world, pos, state) {
        return;
    }
    let powered = is_true(state, "powered");
    let should = input_signal(world, pos, state) > 0;
    if powered && !should {
        world.set_block(pos, set_prop(state, "powered", "false"), update::CLIENTS);
    } else if !powered {
        world.set_block(pos, set_prop(state, "powered", "true"), update::CLIENTS);
        if !should {
            world.schedule_block_tick(pos, block_of(state), delay_ticks(state), VERY_HIGH);
        }
    }
}

/// The `locked` property a repeater should have, for the shape update.
pub(crate) fn repeater_lock_state<A: BlockAccess>(
    world: &World<A>,
    pos: BlockPos,
    state: u16,
) -> u16 {
    set_prop(state, "locked", bool_str(repeater_locked(world, pos, state)))
}
