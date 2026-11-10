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

/// What the block `state` at `pos` emits toward a neighbour that is `dir` away from the
/// neighbour's point of view (the neighbour asks the block in direction `dir`).
fn state_signal<A: BlockAccess>(world: &World<A>, pos: BlockPos, state: u16, dir: Direction) -> u8 {
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
        Some(Redstone::Repeater | Redstone::Observer) => {
            state_direct_signal(world, pos, state, dir)
        }
        Some(Redstone::Wire) => {
            if !world.wires_give_power || dir == Direction::Down {
                return 0;
            }
            let power = wire_power(state);
            if power == 0 {
                return 0;
            }
            if dir != Direction::Up {
                let conn = wire_connection_state(world, pos, state);
                if side_of(conn, dir.opposite()) == Arm::None {
                    return 0;
                }
            }
            power
        }
        _ => 0,
    }
}

fn state_direct_signal<A: BlockAccess>(
    world: &World<A>,
    pos: BlockPos,
    state: u16,
    dir: Direction,
) -> u8 {
    match redstone_of(state) {
        Some(Redstone::Lever | Redstone::Button) => {
            if is_true(state, "powered") && attached_direction(state) == dir { 15 } else { 0 }
        }
        Some(Redstone::Torch | Redstone::WallTorch) => {
            if dir == Direction::Down {
                state_signal(world, pos, state, dir)
            } else {
                0
            }
        }
        Some(Redstone::Repeater | Redstone::Observer)
            if is_true(state, "powered") && facing(state) == dir =>
        {
            15
        }
        Some(Redstone::Wire) if world.wires_give_power => state_signal(world, pos, state, dir),
        _ => 0,
    }
}

fn direct_signal_to<A: BlockAccess>(world: &World<A>, pos: BlockPos) -> u8 {
    let mut best = 0;
    for dir in Direction::ALL {
        let at = pos.offset(dir);
        if let Some(s) = world.block(at) {
            best = best.max(state_direct_signal(world, at, s, dir));
        }
    }
    best
}

/// `Level.getSignal`: the strongest signal reaching `pos` from the block in direction `dir`.
pub(crate) fn signal<A: BlockAccess>(world: &World<A>, pos: BlockPos, dir: Direction) -> u8 {
    let Some(state) = world.block(pos) else { return 0 };
    let s = state_signal(world, pos, state, dir);
    if facts(state).has(flag::REDSTONE_CONDUCTOR) { s.max(direct_signal_to(world, pos)) } else { s }
}

pub(crate) fn has_neighbor_signal<A: BlockAccess>(world: &World<A>, pos: BlockPos) -> bool {
    Direction::ALL.into_iter().any(|d| signal(world, pos.offset(d), d) > 0)
}

/// `Level.getBestNeighborSignal`.
fn best_neighbor_signal<A: BlockAccess>(world: &World<A>, pos: BlockPos) -> u8 {
    let mut best = 0;
    for d in Direction::ALL {
        let s = signal(world, pos.offset(d), d);
        if s >= 15 {
            return 15;
        }
        best = best.max(s);
    }
    best
}

// ---- hooks called by the world ------------------------------------------------------------

/// A redstone block was just placed (or its state changed) at `pos`.
pub(crate) fn on_place<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, state: u16, _old: u16) {
    match redstone_of(state) {
        Some(Redstone::Torch | Redstone::WallTorch) => notify_all_around(world, pos, state),
        Some(Redstone::Repeater) => update_neighbors_in_front(world, pos, state),
        Some(Redstone::Observer) if redstone_of(_old) != Some(Redstone::Observer) => {
            if is_true(state, "powered") && !world.has_scheduled_block_tick(pos, block_of(state)) {
                let off = set_prop(state, "powered", "false");
                world.set_block(pos, off, 18);
                update_neighbors_in_front(world, pos, off);
            }
        }
        Some(Redstone::Wire) if redstone_of(_old) != Some(Redstone::Wire) => {
            wire_update_power(world, pos, state);
            for d in [Direction::Up, Direction::Down] {
                world.update_neighbors_at(pos.offset(d), block_of(state));
            }
            update_neighboring_wires(world, pos);
        }
        _ => {}
    }
}

/// The redstone block `old` at `pos` was replaced by a different kind of block.
pub(crate) fn on_remove<A: BlockAccess>(
    world: &mut World<A>,
    pos: BlockPos,
    old: u16,
    moved: bool,
) {
    match redstone_of(old) {
        Some(Redstone::Torch | Redstone::WallTorch) if !moved => notify_all_around(world, pos, old),
        Some(Redstone::Repeater) if !moved => update_neighbors_in_front(world, pos, old),
        Some(Redstone::Observer) => {
            if is_true(old, "powered") && world.has_scheduled_block_tick(pos, block_of(old)) {
                update_neighbors_in_front(world, pos, set_prop(old, "powered", "false"));
            }
        }
        Some(Redstone::Wire) if !moved => {
            let block = block_of(old);
            for d in Direction::ALL {
                world.update_neighbors_at(pos.offset(d), block);
            }
            wire_update_power(world, pos, old);
            update_neighboring_wires(world, pos);
        }
        Some(Redstone::Lever | Redstone::Button) if is_true(old, "powered") && !moved => {
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
        Some(Redstone::Wire) => {
            if world.block(pos.below()).is_some_and(wire_can_survive_on) {
                wire_update_power(world, pos, state);
            } else {
                world.set_block(pos, Kinds::get().air, update::ALL);
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
        Some(Redstone::Observer) => {
            if is_true(state, "powered") {
                world.set_block(pos, set_prop(state, "powered", "false"), update::CLIENTS);
            } else {
                world.set_block(pos, set_prop(state, "powered", "true"), update::CLIENTS);
                world.schedule_block_tick(pos, block, 2, 0);
            }
            update_neighbors_in_front(world, pos, state);
        }
        _ => {}
    }
}

/// An observer saw its watched face change: pulse (after a short delay) unless already pulsing.
pub(crate) fn observer_shape<A: BlockAccess>(
    world: &mut World<A>,
    state: u16,
    dir: Direction,
    pos: BlockPos,
) {
    if facing(state) == dir
        && !is_true(state, "powered")
        && !world.has_scheduled_block_tick(pos, block_of(state))
    {
        world.schedule_block_tick(pos, block_of(state), 2, 0);
    }
}

// ---- torches ----------------------------------------------------------------------------

/// Whether the block the torch is attached to is powered (which turns the torch off).
fn torch_input<A: BlockAccess>(world: &World<A>, pos: BlockPos, state: u16) -> bool {
    match redstone_of(state) {
        Some(Redstone::WallTorch) => {
            // Ask the block it is attached to, looking toward it.
            let toward = facing(state).opposite();
            signal(world, pos.offset(toward), toward) > 0
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
    let wire_power_in_front = world
        .block(front)
        .filter(|b| redstone_of(*b) == Some(Redstone::Wire))
        .map_or(0, wire_power);
    s.max(wire_power_in_front)
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

// ---- wire -------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arm {
    None,
    Flat,
    Up,
}

fn wire_power(state: u16) -> u8 {
    prop(state, "power").and_then(|p| p.parse().ok()).unwrap_or(0)
}

pub(crate) fn wire_can_survive_on(state: u16) -> bool {
    facts(state).is_face_sturdy(Direction::Up.index(), DataSupport::Full)
        || Kinds::get().of_state(state).is_hopper
}

fn side_name(side: Arm) -> &'static str {
    match side {
        Arm::None => "none",
        Arm::Flat => "side",
        Arm::Up => "up",
    }
}

fn side_of(state: u16, dir: Direction) -> Arm {
    match prop(state, dir.name()) {
        Some("side") => Arm::Flat,
        Some("up") => Arm::Up,
        _ => Arm::None,
    }
}

fn with_side(state: u16, dir: Direction, side: Arm) -> u16 {
    set_prop(state, dir.name(), side_name(side))
}

fn is_dot(state: u16) -> bool {
    Direction::HORIZONTAL.into_iter().all(|d| side_of(state, d) == Arm::None)
}

fn is_cross(state: u16) -> bool {
    Direction::HORIZONTAL.into_iter().all(|d| side_of(state, d) != Arm::None)
}

fn is_wire(state: u16) -> bool {
    redstone_of(state) == Some(Redstone::Wire)
}

/// Whether a wire connects to this block (`dir` is the direction from the wire to it, if known).
fn should_connect_to(state: u16, dir: Option<Direction>) -> bool {
    match redstone_of(state) {
        Some(Redstone::Wire) => true,
        Some(Redstone::Repeater) => {
            let f = facing(state);
            Some(f) == dir || Some(f.opposite()) == dir
        }
        _ => {
            let name = ferrite_data::blocks::block_of_state(state).map(|b| b.name);
            if name == Some("minecraft:observer") {
                return dir == prop(state, "facing").and_then(Direction::from_name);
            }
            facts(state).has(flag::SIGNAL_SOURCE) && dir.is_some()
        }
    }
}

fn conductor<A: BlockAccess>(world: &World<A>, pos: BlockPos) -> bool {
    world.block(pos).is_some_and(|s| facts(s).has(flag::REDSTONE_CONDUCTOR))
}

fn connecting_side<A: BlockAccess>(
    world: &World<A>,
    pos: BlockPos,
    dir: Direction,
    can_go_up: bool,
) -> Arm {
    let rel = pos.offset(dir);
    let rel_state = world.block(rel).unwrap_or(Kinds::get().air);
    if can_go_up {
        let supports =
            Kinds::get().of_state(rel_state).is_trapdoor || wire_can_survive_on(rel_state);
        if supports && world.block(rel.above()).is_some_and(|s| should_connect_to(s, None)) {
            return if facts(rel_state).is_face_sturdy(dir.opposite().index(), DataSupport::Full) {
                Arm::Up
            } else {
                Arm::Flat
            };
        }
    }
    if should_connect_to(rel_state, Some(dir))
        || (!conductor(world, rel)
            && world.block(rel.below()).is_some_and(|s| should_connect_to(s, None)))
    {
        return Arm::Flat;
    }
    Arm::None
}

fn missing_connections<A: BlockAccess>(world: &World<A>, mut state: u16, pos: BlockPos) -> u16 {
    let can_go_up = !conductor(world, pos.above());
    for dir in Direction::HORIZONTAL {
        if side_of(state, dir) == Arm::None {
            state = with_side(state, dir, connecting_side(world, pos, dir, can_go_up));
        }
    }
    state
}

/// The wire state at `pos` with all four arms recomputed from its surroundings.
fn wire_connection_state<A: BlockAccess>(world: &World<A>, pos: BlockPos, state: u16) -> u16 {
    let was_dot = is_dot(state);
    let fresh = set_prop(Kinds::get().wire_default(), "power", &wire_power(state).to_string());
    let mut state = missing_connections(world, fresh, pos);
    if was_dot && is_dot(state) {
        return state;
    }
    let n = side_of(state, Direction::North) != Arm::None;
    let s = side_of(state, Direction::South) != Arm::None;
    let e = side_of(state, Direction::East) != Arm::None;
    let w = side_of(state, Direction::West) != Arm::None;
    let ns_free = !n && !s;
    let ew_free = !e && !w;
    if !w && ns_free {
        state = with_side(state, Direction::West, Arm::Flat);
    }
    if !e && ns_free {
        state = with_side(state, Direction::East, Arm::Flat);
    }
    if !n && ew_free {
        state = with_side(state, Direction::North, Arm::Flat);
    }
    if !s && ew_free {
        state = with_side(state, Direction::South, Arm::Flat);
    }
    state
}

/// `updateShape` for wire.
pub(crate) fn wire_update_shape<A: BlockAccess>(
    world: &World<A>,
    state: u16,
    dir: Direction,
    pos: BlockPos,
) -> u16 {
    match dir {
        Direction::Down => state, // support is checked by the generic rule
        Direction::Up => wire_connection_state(world, pos, state),
        _ => {
            let can_go_up = !conductor(world, pos.above());
            let side = connecting_side(world, pos, dir, can_go_up);
            if (side != Arm::None) == (side_of(state, dir) != Arm::None) && !is_cross(state) {
                return with_side(state, dir, side);
            }
            wire_connection_state(world, pos, with_side(state, dir, side))
        }
    }
}

fn wire_signal_of(state: Option<u16>) -> u8 {
    state.filter(|s| is_wire(*s)).map_or(0, wire_power)
}

fn incoming_wire_signal<A: BlockAccess>(world: &World<A>, pos: BlockPos) -> u8 {
    let mut best = 0;
    for dir in Direction::HORIZONTAL {
        let rel = pos.offset(dir);
        let rel_state = world.block(rel);
        best = best.max(wire_signal_of(rel_state));
        let rel_conductor = conductor(world, rel);
        if rel_conductor && !conductor(world, pos.above()) {
            best = best.max(wire_signal_of(world.block(rel.above())));
        } else if !rel_conductor {
            best = best.max(wire_signal_of(world.block(rel.below())));
        }
    }
    best.saturating_sub(1)
}

fn target_strength<A: BlockAccess>(world: &mut World<A>, pos: BlockPos) -> u8 {
    world.wires_give_power = false;
    let outside = best_neighbor_signal(world, pos);
    world.wires_give_power = true;
    if outside == 15 { outside } else { outside.max(incoming_wire_signal(world, pos)) }
}

/// Java's `HashSet<BlockPos>` iteration order for up to 12 elements: buckets of a 16-slot table
/// in ascending order, insertion order within a bucket.
pub(crate) fn java_hash_set_order(items: &[BlockPos]) -> Vec<BlockPos> {
    let hash = |p: &BlockPos| -> usize {
        let h = p.y.wrapping_add(p.z.wrapping_mul(31)).wrapping_mul(31).wrapping_add(p.x) as u32;
        ((h ^ (h >> 16)) & 15) as usize
    };
    let mut keyed: Vec<(usize, usize, BlockPos)> = Vec::new();
    for (i, p) in items.iter().enumerate() {
        if !keyed.iter().any(|(_, _, q)| q == p) {
            keyed.push((hash(p), i, *p));
        }
    }
    keyed.sort_by_key(|(bucket, i, _)| (*bucket, *i));
    keyed.into_iter().map(|(_, _, p)| p).collect()
}

fn wire_update_power<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, state: u16) {
    let target = target_strength(world, pos);
    if wire_power(state) == target {
        return;
    }
    if world.block(pos) == Some(state) {
        world.set_block(pos, set_prop(state, "power", &target.to_string()), update::CLIENTS);
    }
    let mut set = vec![pos];
    set.extend(Direction::ALL.map(|d| pos.offset(d)));
    let wire_block = block_of(Kinds::get().wire_default());
    for p in java_hash_set_order(&set) {
        world.update_neighbors_at(p, wire_block);
    }
}

fn check_corner_change_at<A: BlockAccess>(world: &mut World<A>, pos: BlockPos) {
    if world.block(pos).is_some_and(is_wire) {
        let wire_block = block_of(Kinds::get().wire_default());
        world.update_neighbors_at(pos, wire_block);
        for d in Direction::ALL {
            world.update_neighbors_at(pos.offset(d), wire_block);
        }
    }
}

fn update_neighboring_wires<A: BlockAccess>(world: &mut World<A>, pos: BlockPos) {
    for d in Direction::HORIZONTAL {
        check_corner_change_at(world, pos.offset(d));
    }
    for d in Direction::HORIZONTAL {
        let p = pos.offset(d);
        if conductor(world, p) {
            check_corner_change_at(world, p.above());
        } else {
            check_corner_change_at(world, p.below());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 400 orderings produced by the real `java.util.HashSet<net.minecraft.core.BlockPos>`
    /// (see `tests/data/HashOrder.java`), including `BlockPos.hashCode` itself.
    const JAVA: &str = include_str!("../tests/data/java_hashset_order.txt");

    fn parse(list: &str) -> Vec<BlockPos> {
        list.split_whitespace()
            .map(|p| {
                let v: Vec<i32> = p.split(',').map(|n| n.parse().expect("number")).collect();
                BlockPos::new(v[0], v[1], v[2])
            })
            .collect()
    }

    #[test]
    fn hash_set_order_matches_real_java() {
        let mut checked = 0;
        let mut reordered = 0;
        for line in JAVA.lines().filter(|l| !l.starts_with('#')) {
            let (inserted, iterated) = line.split_once('|').expect("separator");
            let inserted = parse(inserted);
            let want = parse(iterated);
            assert_eq!(java_hash_set_order(&inserted), want, "insertion order {inserted:?}");
            checked += 1;
            if inserted != want {
                reordered += 1;
            }
        }
        assert_eq!(checked, 400);
        assert!(reordered > 300, "the data must actually exercise reordering ({reordered})");
    }
}
