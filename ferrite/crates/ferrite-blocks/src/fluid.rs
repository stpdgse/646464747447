//! Water and lava.
//!
//! A fluid cell is a block state; its fluid facts (amount 1..=8, source, falling) come from the
//! data tables. Flow is driven by scheduled fluid ticks: a non-source cell first recomputes what
//! it should be from its neighbours, then every cell tries to spread down or, failing that,
//! sideways toward the nearest drop.

use ferrite_data::shapes::facts;

use crate::kinds::Kinds;
use crate::level::BlockAccess;
use crate::pos::{BlockPos, Direction};
use crate::world::{Rules, World, update};

/// Ids in the `minecraft:fluid` registry, which double as scheduled-tick types.
pub const TYPE_FLOWING_WATER: u8 = ferrite_data::generated::registries::fluid::FLOWING_WATER as u8;
pub const TYPE_WATER: u8 = ferrite_data::generated::registries::fluid::WATER as u8;
pub const TYPE_FLOWING_LAVA: u8 = ferrite_data::generated::registries::fluid::FLOWING_LAVA as u8;
pub const TYPE_LAVA: u8 = ferrite_data::generated::registries::fluid::LAVA as u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Water,
    Lava,
}

/// The fluid content of a block state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluidState {
    pub kind: Kind,
    /// 0 = no fluid; 8 = a full block.
    pub amount: u8,
    pub source: bool,
    pub falling: bool,
}

impl FluidState {
    pub const EMPTY: FluidState =
        FluidState { kind: Kind::Water, amount: 0, source: false, falling: false };

    #[inline]
    pub fn of(state: u16) -> FluidState {
        let f = facts(state);
        if f.fluid_amount() == 0 {
            return FluidState::EMPTY;
        }
        FluidState {
            kind: if f.fluid() == TYPE_WATER || f.fluid() == TYPE_FLOWING_WATER {
                Kind::Water
            } else {
                Kind::Lava
            },
            amount: f.fluid_amount(),
            source: f.fluid_is_source(),
            falling: f.fluid_is_falling(),
        }
    }

    pub const fn source(kind: Kind) -> FluidState {
        FluidState { kind, amount: 8, source: true, falling: false }
    }

    pub const fn flowing(kind: Kind, amount: u8, falling: bool) -> FluidState {
        FluidState { kind, amount, source: false, falling }
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.amount == 0
    }

    #[inline]
    pub fn is(&self, kind: Kind) -> bool {
        self.amount != 0 && self.kind == kind
    }

    /// The registry id of this fluid (source and flowing fluids are distinct entries).
    pub fn type_id(&self) -> u8 {
        match (self.kind, self.source) {
            _ if self.is_empty() => 0,
            (Kind::Water, true) => TYPE_WATER,
            (Kind::Water, false) => TYPE_FLOWING_WATER,
            (Kind::Lava, true) => TYPE_LAVA,
            (Kind::Lava, false) => TYPE_FLOWING_LAVA,
        }
    }

    /// The plain liquid block holding this fluid (air if empty).
    pub fn block(&self) -> u16 {
        let kinds = Kinds::get();
        if self.is_empty() {
            return kinds.air;
        }
        let level =
            if self.source { 0 } else { 8 - self.amount.min(8) + if self.falling { 8 } else { 0 } };
        match self.kind {
            Kind::Water => kinds.water_level(level),
            Kind::Lava => kinds.lava_level(level),
        }
    }
}

pub fn tick_delay(kind: Kind, rules: &Rules) -> i64 {
    match kind {
        Kind::Water => 5,
        Kind::Lava if rules.ultrawarm => 10,
        Kind::Lava => 30,
    }
}

fn slope_find_distance(kind: Kind, rules: &Rules) -> u32 {
    match kind {
        Kind::Water => 4,
        Kind::Lava if rules.ultrawarm => 4,
        Kind::Lava => 2,
    }
}

fn drop_off(kind: Kind, rules: &Rules) -> u8 {
    match kind {
        Kind::Water => 1,
        Kind::Lava if rules.ultrawarm => 1,
        Kind::Lava => 2,
    }
}

fn converts_to_source(kind: Kind, rules: &Rules) -> bool {
    match kind {
        Kind::Water => rules.water_source_conversion,
        Kind::Lava => rules.lava_source_conversion,
    }
}

/// Whether fluid can be in this block at all (ignoring what the block itself contains).
fn can_hold_any_fluid(state: u16) -> bool {
    let kind = Kinds::get().of_state(state);
    if kind.liquid_container {
        return true;
    }
    let blocks_motion = !kind.never_blocks_motion && World::<crate::level::Chunks>::is_solid(state);
    !blocks_motion && !kind.refuses_fluid
}

/// Whether flowing fluid can enter this block. Containers only ever take a water *source*, which
/// flow never delivers.
fn can_hold_flowing(state: u16) -> bool {
    can_hold_any_fluid(state) && !Kinds::get().of_state(state).liquid_container
}

/// Whether `incoming` may displace the fluid `existing` when arriving in direction `dir`.
fn can_be_replaced_with<A: BlockAccess>(
    world: &World<A>,
    existing: FluidState,
    pos: BlockPos,
    incoming: Kind,
    dir: Direction,
) -> bool {
    if existing.is_empty() {
        return true;
    }
    match existing.kind {
        Kind::Water => dir == Direction::Down && incoming != Kind::Water,
        Kind::Lava => {
            // Height is amount/9, or a full block under more of the same; the threshold is 4/9.
            let full = world.fluid(pos.above()).is_some_and(|f| f.is(Kind::Lava));
            (full || existing.amount >= 4) && incoming == Kind::Water
        }
    }
}

/// Called when a liquid block is placed or a neighbour changes: lava next to water hardens,
/// anything else gets a tick.
pub(crate) fn liquid_block_changed<A: BlockAccess>(
    world: &mut World<A>,
    pos: BlockPos,
    state: u16,
) {
    let fluid = FluidState::of(state);
    if should_spread(world, pos, fluid) {
        let delay = tick_delay(fluid.kind, &world.rules);
        world.schedule_fluid_tick(pos, fluid.type_id(), delay);
    }
}

fn should_spread<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, fluid: FluidState) -> bool {
    if fluid.kind != Kind::Lava {
        return true;
    }
    let kinds = Kinds::get();
    let on_soul_soil = world.block(pos.below()) == Some(kinds.soul_soil);
    // The side the other block is on, in the order vanilla checks them.
    for side in
        [Direction::Up, Direction::North, Direction::South, Direction::West, Direction::East]
    {
        let other = pos.offset(side);
        let Some(other_state) = world.block(other) else { continue };
        if FluidState::of(other_state).is(Kind::Water) {
            let product = if fluid.source { kinds.obsidian } else { kinds.cobblestone };
            world.set_block(pos, product, update::ALL);
            return false;
        }
        if on_soul_soil && other_state == kinds.blue_ice {
            world.set_block(pos, kinds.basalt, update::ALL);
            return false;
        }
    }
    true
}

/// Runs a fluid tick for the fluid `fluid` in block `state` at `pos`.
pub(crate) fn tick<A: BlockAccess>(
    world: &mut World<A>,
    pos: BlockPos,
    mut state: u16,
    mut fluid: FluidState,
) {
    if !fluid.source {
        let new = new_liquid(world, pos, state, fluid.kind);
        let delay = spread_delay(world, fluid, new);
        if new.is_empty() {
            fluid = new;
            state = Kinds::get().air;
            world.set_block(pos, state, update::ALL);
        } else if new != fluid {
            fluid = new;
            state = new.block();
            world.set_block(pos, state, update::ALL);
            world.schedule_fluid_tick(pos, new.type_id(), delay);
        }
    }
    spread(world, pos, state, fluid);
}

fn spread_delay<A: BlockAccess>(world: &mut World<A>, old: FluidState, new: FluidState) -> i64 {
    let delay = tick_delay(old.kind, &world.rules);
    // Rising lava is slowed at random; vanilla draws this from the world's shared random source.
    if old.kind == Kind::Lava
        && !old.is_empty()
        && !new.is_empty()
        && !old.falling
        && !new.falling
        && new.amount > old.amount
        && world.next_random(4) != 0
    {
        return delay * 4;
    }
    delay
}

/// Whether fluid can cross the boundary between two neighbouring blocks.
fn can_pass_through_wall<A: BlockAccess>(
    world: &mut World<A>,
    dir: Direction,
    state: u16,
    to_state: u16,
) -> bool {
    !world.faces_occlude(state, to_state, dir)
}

/// What the fluid at `pos` should be, judging by its surroundings.
fn new_liquid<A: BlockAccess>(
    world: &mut World<A>,
    pos: BlockPos,
    state: u16,
    kind: Kind,
) -> FluidState {
    let mut highest = 0u8;
    let mut sources = 0;
    for dir in Direction::HORIZONTAL {
        let Some(side_state) = world.block(pos.offset(dir)) else { continue };
        let side = FluidState::of(side_state);
        if side.is(kind) && can_pass_through_wall(world, dir, state, side_state) {
            if side.source {
                sources += 1;
            }
            highest = highest.max(side.amount);
        }
    }
    if sources >= 2
        && converts_to_source(kind, &world.rules)
        && let Some(below) = world.block(pos.below())
    {
        let below_fluid = FluidState::of(below);
        if World::<A>::is_solid(below) || (below_fluid.is(kind) && below_fluid.source) {
            return FluidState::source(kind);
        }
    }
    if let Some(above) = world.block(pos.above())
        && FluidState::of(above).is(kind)
        && can_pass_through_wall(world, Direction::Up, state, above)
    {
        return FluidState::flowing(kind, 8, true);
    }
    let amount = highest.saturating_sub(drop_off(kind, &world.rules));
    if amount == 0 { FluidState::EMPTY } else { FluidState::flowing(kind, amount, false) }
}

/// Fluid could move from `state` into the neighbour in direction `dir`, as far as the two blocks
/// themselves are concerned.
fn can_maybe_pass_through<A: BlockAccess>(
    world: &mut World<A>,
    kind: Kind,
    state: u16,
    dir: Direction,
    to_state: u16,
) -> bool {
    let to_fluid = FluidState::of(to_state);
    !(to_fluid.is(kind) && to_fluid.source)
        && can_hold_any_fluid(to_state)
        && can_pass_through_wall(world, dir, state, to_state)
}

fn can_pass_through<A: BlockAccess>(
    world: &mut World<A>,
    kind: Kind,
    state: u16,
    dir: Direction,
    to_state: u16,
) -> bool {
    can_maybe_pass_through(world, kind, state, dir, to_state) && can_hold_flowing(to_state)
}

/// Whether fluid at `pos` could fall into the block below.
fn is_hole<A: BlockAccess>(world: &mut World<A>, kind: Kind, pos: BlockPos, state: u16) -> bool {
    let Some(below) = world.block(pos.below()) else { return false };
    if !can_pass_through_wall(world, Direction::Down, state, below) {
        return false;
    }
    FluidState::of(below).is(kind) || can_hold_flowing(below)
}

fn spread<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, state: u16, fluid: FluidState) {
    if fluid.is_empty() {
        return;
    }
    let kind = fluid.kind;
    let below_pos = pos.below();
    if let Some(below) = world.block(below_pos) {
        let below_fluid = FluidState::of(below);
        if can_maybe_pass_through(world, kind, state, Direction::Down, below) {
            let new_below = new_liquid(world, below_pos, below, kind);
            if can_be_replaced_with(world, below_fluid, below_pos, kind, Direction::Down)
                && can_hold_flowing(below)
            {
                spread_to(world, below_pos, below, Direction::Down, new_below);
                if source_neighbors(world, pos, kind) >= 3 {
                    spread_to_sides(world, pos, fluid, state);
                }
                return;
            }
        }
    }
    if fluid.source || !is_hole(world, kind, pos, state) {
        spread_to_sides(world, pos, fluid, state);
    }
}

fn source_neighbors<A: BlockAccess>(world: &World<A>, pos: BlockPos, kind: Kind) -> usize {
    Direction::HORIZONTAL
        .into_iter()
        .filter(|d| world.fluid(pos.offset(*d)).is_some_and(|f| f.is(kind) && f.source))
        .count()
}

fn spread_to_sides<A: BlockAccess>(
    world: &mut World<A>,
    pos: BlockPos,
    fluid: FluidState,
    state: u16,
) {
    let amount = if fluid.falling {
        7
    } else {
        fluid.amount.saturating_sub(drop_off(fluid.kind, &world.rules))
    };
    if amount == 0 {
        return;
    }
    // Indexed by direction so the result is visited in vanilla's enum order (N, S, W, E).
    let mut targets: [Option<FluidState>; 6] = [None; 6];
    let mut best = 1000u32;
    for dir in Direction::HORIZONTAL {
        let to = pos.offset(dir);
        let Some(to_state) = world.block(to) else { continue };
        if !can_maybe_pass_through(world, fluid.kind, state, dir, to_state) {
            continue;
        }
        let new = new_liquid(world, to, to_state, fluid.kind);
        if !can_hold_flowing(to_state) {
            continue;
        }
        let cost = if is_hole(world, fluid.kind, to, to_state) {
            0
        } else {
            slope_distance(world, fluid.kind, to, 1, dir.opposite(), to_state)
        };
        if cost < best {
            targets = [None; 6];
        }
        if cost <= best {
            if can_be_replaced_with(world, FluidState::of(to_state), to, fluid.kind, dir) {
                targets[dir.index()] = Some(new);
            }
            best = cost;
        }
    }
    for dir in Direction::ALL {
        if let Some(new) = targets[dir.index()] {
            let to = pos.offset(dir);
            if let Some(to_state) = world.block(to) {
                spread_to(world, to, to_state, dir, new);
            }
        }
    }
}

/// Distance, in blocks, from `pos` to the nearest place the fluid could fall, searching at most
/// the fluid's slope-find distance and never doubling back toward `from`.
fn slope_distance<A: BlockAccess>(
    world: &mut World<A>,
    kind: Kind,
    pos: BlockPos,
    pass: u32,
    from: Direction,
    state: u16,
) -> u32 {
    let mut best = 1000;
    for dir in Direction::HORIZONTAL {
        if dir == from {
            continue;
        }
        let to = pos.offset(dir);
        let Some(to_state) = world.block(to) else { continue };
        if !can_pass_through(world, kind, state, dir, to_state) {
            continue;
        }
        if is_hole(world, kind, to, to_state) {
            return pass;
        }
        if pass < slope_find_distance(kind, &world.rules) {
            best = best.min(slope_distance(world, kind, to, pass + 1, dir.opposite(), to_state));
        }
    }
    best
}

fn spread_to<A: BlockAccess>(
    world: &mut World<A>,
    pos: BlockPos,
    state: u16,
    dir: Direction,
    fluid: FluidState,
) {
    let kinds = Kinds::get();
    if fluid.kind == Kind::Lava && dir == Direction::Down && FluidState::of(state).is(Kind::Water) {
        // Lava falling onto water turns to stone.
        if kinds.of_state(state).behaviour == crate::kinds::Behaviour::Liquid {
            world.set_block(pos, kinds.stone, update::ALL);
        }
        return;
    }
    if kinds.of_state(state).liquid_container {
        return; // flow never fills a container
    }
    // A non-air block here is washed away (its drops are an entity matter, handled elsewhere).
    world.set_block(pos, fluid.block(), update::ALL);
}
