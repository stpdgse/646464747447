//! The block simulation: setting blocks, propagating updates, running scheduled ticks.

use ferrite_data::blocks::STATE_TO_BLOCK;
use ferrite_data::shapes::{Support as DataSupport, facts, flag};

use crate::fluid::{self, FluidState};
use crate::kinds::{Behaviour, Connects, Kinds, Support};
use crate::level::BlockAccess;
use crate::pos::{BlockPos, Direction};
use crate::shape::OcclusionCache;
use crate::ticks::TickQueue;

/// Flags of [`World::set_block`], with vanilla's meanings and values.
pub mod update {
    /// Tell the six neighbours that this block changed.
    pub const NEIGHBORS: u32 = 1;
    /// Send the change to clients.
    pub const CLIENTS: u32 = 2;
    /// Do not let neighbours adapt their shape to the change.
    pub const KNOWN_SHAPE: u32 = 16;
    /// Do not drop items when a shape update destroys a block.
    pub const SUPPRESS_DROPS: u32 = 32;
    pub const ALL: u32 = NEIGHBORS | CLIENTS;
}

/// The depth limit vanilla gives chains of shape updates.
pub const UPDATE_LIMIT: i32 = 512;

/// Order in which a block's six neighbours are told it changed.
pub const NEIGHBOR_UPDATE_ORDER: [Direction; 6] = [
    Direction::West,
    Direction::East,
    Direction::Down,
    Direction::Up,
    Direction::North,
    Direction::South,
];

/// Order in which a block's six neighbours are asked to adapt their shape.
pub const SHAPE_UPDATE_ORDER: [Direction; 6] = [
    Direction::West,
    Direction::East,
    Direction::North,
    Direction::South,
    Direction::Down,
    Direction::Up,
];

/// World settings the block rules depend on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    /// Two adjacent water sources over solid ground make a third (game rule).
    pub water_source_conversion: bool,
    pub lava_source_conversion: bool,
    /// Nether-like dimension: lava flows as far and as fast as water does elsewhere.
    pub ultrawarm: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self { water_source_conversion: true, lava_source_conversion: false, ultrawarm: false }
    }
}

#[derive(Clone, Copy, Debug)]
enum Update {
    /// Ask the block at `pos` to adapt its shape to `neighbor_state` in direction `dir`.
    Shape {
        dir: Direction,
        neighbor_state: u16,
        pos: BlockPos,
        neighbor_pos: BlockPos,
        flags: u32,
        limit: i32,
    },
    /// Tell exactly one block that `block` changed next to it.
    Single { pos: BlockPos, block: u16 },
    /// Tell the neighbours of `source` (except `skip`) that it changed; `next` is the progress.
    Multi { source: BlockPos, block: u16, skip: Option<Direction>, next: usize },
}

/// Loaded blocks plus everything that makes them change over time.
#[derive(Debug)]
pub struct World<A: BlockAccess> {
    pub blocks: A,
    pub rules: Rules,
    time: i64,
    pub(crate) fluid_ticks: TickQueue<u8>,
    block_ticks: TickQueue<u16>,
    running_ticks: Vec<(BlockPos, u16)>,
    running_next: usize,
    pub(crate) wires_give_power: bool,
    pub(crate) toggles: Vec<(BlockPos, i64)>,
    occlusion: OcclusionCache,
    stack: Vec<Update>,
    added: Vec<Update>,
    chained: usize,
    rng: u64,
}

/// Vanilla abandons a chain of neighbour updates after this many.
const MAX_CHAINED_UPDATES: usize = 1_000_000;

impl<A: BlockAccess> World<A> {
    pub fn new(blocks: A, rules: Rules) -> Self {
        Self {
            blocks,
            rules,
            time: 0,
            fluid_ticks: TickQueue::new(),
            block_ticks: TickQueue::new(),
            running_ticks: Vec::new(),
            running_next: 0,
            wires_give_power: true,
            toggles: Vec::new(),
            occlusion: OcclusionCache::default(),
            stack: Vec::new(),
            added: Vec::new(),
            chained: 0,
            rng: 0x9E37_79B9_7F4A_7C15,
        }
    }

    /// Seeds the generator used where vanilla consults the world's shared random source.
    pub fn seed(&mut self, seed: u64) {
        self.rng = seed | 1;
    }

    pub(crate) fn next_random(&mut self, bound: u32) -> u32 {
        // xorshift64*; only used where vanilla itself is not reproducible (shared world random).
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        ((self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) % u64::from(bound)) as u32
    }

    pub fn time(&self) -> i64 {
        self.time
    }

    #[inline]
    pub fn block(&self, pos: BlockPos) -> Option<u16> {
        self.blocks.block(pos)
    }

    #[inline]
    pub(crate) fn fluid(&self, pos: BlockPos) -> Option<FluidState> {
        self.blocks.block(pos).map(FluidState::of)
    }

    /// Pending fluid ticks in execution order.
    pub fn pending_fluid_ticks(&self) -> Vec<crate::ticks::Pending<u8>> {
        self.fluid_ticks.pending(self.time)
    }

    /// Pending block ticks in saved order.
    pub fn pending_block_ticks(&self) -> Vec<crate::ticks::Pending<u16>> {
        self.block_ticks.pending(self.time)
    }

    /// Schedules a block tick (`block` is the block id) unless one is already pending.
    pub fn schedule_block_tick(&mut self, pos: BlockPos, block: u16, delay: i64, priority: i8) {
        self.block_ticks.schedule(pos, block, self.time + delay, priority);
    }

    /// Whether a tick of `block` at `pos` is waiting to run later in the current game tick.
    pub fn will_tick_this_tick(&self, pos: BlockPos, block: u16) -> bool {
        self.running_ticks.iter().skip(self.running_next).any(|t| *t == (pos, block))
    }

    pub(crate) fn schedule_fluid_tick(&mut self, pos: BlockPos, fluid_type: u8, delay: i64) {
        self.fluid_ticks.schedule(pos, fluid_type, self.time + delay, 0);
    }

    pub(crate) fn faces_occlude(&mut self, from: u16, to: u16, dir: Direction) -> bool {
        self.occlusion.collision_faces_occlude(from, to, dir)
    }

    /// Sets a block with vanilla's update semantics. Returns whether anything changed.
    pub fn set_block(&mut self, pos: BlockPos, state: u16, flags: u32) -> bool {
        self.set_block_limited(pos, state, flags, UPDATE_LIMIT)
    }

    pub(crate) fn set_block_limited(
        &mut self,
        pos: BlockPos,
        state: u16,
        flags: u32,
        limit: i32,
    ) -> bool {
        let Some(old) = self.blocks.block(pos) else { return false };
        if old == state {
            return false;
        }
        self.blocks.set_block_raw(pos, state);
        if STATE_TO_BLOCK[usize::from(old)] != STATE_TO_BLOCK[usize::from(state)]
            && Kinds::get().of_state(old).redstone.is_some()
        {
            crate::redstone::on_remove(self, pos, old);
        }
        self.on_place(pos, state, old);
        // A non-liquid block that now holds a different fluid (waterlogging) gets that fluid's
        // tick; liquid blocks schedule their own in `on_place`.
        let new_fluid = FluidState::of(state);
        if !new_fluid.is_empty()
            && new_fluid != FluidState::of(old)
            && Kinds::get().of_state(state).behaviour != Behaviour::Liquid
            && self.blocks.block(pos) == Some(state)
        {
            let delay = fluid::tick_delay(new_fluid.kind, &self.rules);
            self.schedule_fluid_tick(pos, new_fluid.type_id(), delay);
        }
        // Placement hooks may already have replaced the block again.
        if self.blocks.block(pos) != Some(state) {
            return true;
        }
        if flags & update::NEIGHBORS != 0 {
            self.update_neighbors_at(pos, STATE_TO_BLOCK[usize::from(old)]);
        }
        if flags & update::KNOWN_SHAPE == 0 && limit > 0 {
            let f = flags & !(update::NEIGHBORS | update::SUPPRESS_DROPS);
            self.indirect_shape_updates(pos, old, f, limit - 1);
            for dir in SHAPE_UPDATE_ORDER {
                self.add_and_run(Update::Shape {
                    dir: dir.opposite(),
                    neighbor_state: state,
                    pos: pos.offset(dir),
                    neighbor_pos: pos,
                    flags: f,
                    limit: limit - 1,
                });
            }
            self.indirect_shape_updates(pos, state, f, limit - 1);
        }
        true
    }

    /// Updates that reach blocks *diagonal* to `pos` (wire stairs), for the state `state` that
    /// was, or now is, at `pos`.
    fn indirect_shape_updates(&mut self, pos: BlockPos, state: u16, flags: u32, limit: i32) {
        if Kinds::get().of_state(state).redstone != Some(crate::kinds::Redstone::Wire) {
            return;
        }
        for dir in Direction::HORIZONTAL {
            let arm = Kinds::get().property(state, dir.name());
            let beside = pos.offset(dir);
            let beside_is_wire = self.blocks.block(beside).is_some_and(|b| {
                Kinds::get().of_state(b).redstone == Some(crate::kinds::Redstone::Wire)
            });
            if arm == Some("none") || beside_is_wire {
                continue;
            }
            // The wire below-ahead sees `pos`'s lower neighbour as changed, and likewise above.
            for (target, back) in [(beside.below(), pos.below()), (beside.above(), pos.above())] {
                let Some(t) = self.blocks.block(target) else { continue };
                if Kinds::get().of_state(t).redstone != Some(crate::kinds::Redstone::Wire) {
                    continue;
                }
                let Some(neighbor_state) = self.blocks.block(back) else { continue };
                self.add_and_run(Update::Shape {
                    dir: dir.opposite(),
                    neighbor_state,
                    pos: target,
                    neighbor_pos: back,
                    flags,
                    limit,
                });
            }
        }
    }

    /// What the `/setblock` command does with a bare block name: the state adapts to its
    /// neighbours before it is placed.
    pub fn command_set_block(&mut self, pos: BlockPos, state: u16) -> bool {
        self.command_set_block_with(pos, state, &[])
    }

    /// `/setblock` with `defined` naming the properties the command spelled out (`oak_door[open=true]`
    /// defines `open`). The state adapts to its neighbours, falling back to the given state if
    /// that would destroy it, and the defined properties then override whatever adapted.
    pub fn command_set_block_with(&mut self, pos: BlockPos, state: u16, defined: &[&str]) -> bool {
        let adapted = self.update_from_neighbor_shapes(state, pos);
        let mut placed = if facts(adapted).has(flag::AIR) { state } else { adapted };
        if STATE_TO_BLOCK[usize::from(placed)] == STATE_TO_BLOCK[usize::from(state)] {
            let kinds = Kinds::get();
            for name in defined {
                if let Some(value) = kinds.property(state, name) {
                    placed = kinds.with_property(placed, name, value).unwrap_or(placed);
                }
            }
        }
        if !self.set_block(pos, placed, update::CLIENTS) {
            return false;
        }
        self.update_neighbors_at(pos, STATE_TO_BLOCK[usize::from(placed)]);
        true
    }

    /// Lets a state that is about to be placed at `pos` adapt to each neighbour in turn.
    pub fn update_from_neighbor_shapes(&mut self, mut state: u16, pos: BlockPos) -> u16 {
        for dir in SHAPE_UPDATE_ORDER {
            let neighbor_pos = pos.offset(dir);
            if let Some(neighbor) = self.blocks.block(neighbor_pos) {
                state = self.update_shape(state, dir, neighbor, pos, neighbor_pos);
            }
        }
        state
    }

    /// Tells the six neighbours of `pos` that it changed.
    pub fn update_neighbors_at(&mut self, pos: BlockPos, block: u16) {
        self.add_and_run(Update::Multi { source: pos, block, skip: None, next: 0 });
    }

    /// Like [`update_neighbors_at`](Self::update_neighbors_at), leaving out the neighbour in
    /// direction `skip`.
    pub fn update_neighbors_at_except(&mut self, pos: BlockPos, block: u16, skip: Direction) {
        let first = if NEIGHBOR_UPDATE_ORDER[0] == skip { 1 } else { 0 };
        self.add_and_run(Update::Multi { source: pos, block, skip: Some(skip), next: first });
    }

    /// Tells the block at `pos` that a neighbour of type `block` changed.
    pub fn neighbor_changed_at(&mut self, pos: BlockPos, block: u16) {
        self.add_and_run(Update::Single { pos, block });
    }

    fn add_and_run(&mut self, update: Update) {
        let running = self.chained > 0;
        let too_deep = self.chained >= MAX_CHAINED_UPDATES;
        self.chained += 1;
        if !too_deep {
            if running {
                self.added.push(update);
            } else {
                self.stack.push(update);
            }
        }
        if !running {
            self.run_updates();
        }
    }

    /// Runs queued updates depth-first: everything an update causes runs before its siblings,
    /// in the order it was caused, but only after the causing update has finished.
    fn run_updates(&mut self) {
        while !self.stack.is_empty() || !self.added.is_empty() {
            while let Some(u) = self.added.pop() {
                self.stack.push(u);
            }
            while self.added.is_empty() {
                let Some(top) = self.stack.last().copied() else { break };
                let index = self.stack.len() - 1;
                let more = match top {
                    Update::Shape { dir, neighbor_state, pos, neighbor_pos, flags, limit } => {
                        self.run_shape_update(dir, neighbor_state, pos, neighbor_pos, flags, limit);
                        false
                    }
                    Update::Single { pos, block } => {
                        if let Some(state) = self.blocks.block(pos) {
                            self.neighbor_changed(pos, state, block);
                        }
                        false
                    }
                    Update::Multi { source, block, skip, mut next } => {
                        let dir = NEIGHBOR_UPDATE_ORDER[next];
                        next += 1;
                        if next < 6 && Some(NEIGHBOR_UPDATE_ORDER[next]) == skip {
                            next += 1;
                        }
                        self.stack[index] = Update::Multi { source, block, skip, next };
                        let target = source.offset(dir);
                        if let Some(state) = self.blocks.block(target) {
                            self.neighbor_changed(target, state, block);
                        }
                        next < 6
                    }
                };
                if !more {
                    self.stack.remove(index);
                    break;
                }
            }
        }
        self.chained = 0;
    }

    fn run_shape_update(
        &mut self,
        dir: Direction,
        neighbor_state: u16,
        pos: BlockPos,
        neighbor_pos: BlockPos,
        flags: u32,
        limit: i32,
    ) {
        let Some(state) = self.blocks.block(pos) else { return };
        let new = self.update_shape(state, dir, neighbor_state, pos, neighbor_pos);
        if new == state {
            return;
        }
        if facts(new).has(flag::AIR) {
            // Destroyed: what remains is the fluid the block contained, if any.
            let left = FluidState::of(state).block();
            self.set_block_limited(pos, left, update::ALL, limit);
        } else {
            self.set_block_limited(pos, new, flags & !update::SUPPRESS_DROPS, limit);
        }
    }

    // ---- per-block hooks ------------------------------------------------------------------

    fn on_place(&mut self, pos: BlockPos, state: u16, old: u16) {
        let kind = Kinds::get().of_state(state);
        if kind.behaviour == Behaviour::Liquid {
            fluid::liquid_block_changed(self, pos, state);
        }
        if kind.redstone.is_some() {
            crate::redstone::on_place(self, pos, state, old);
        }
    }

    fn neighbor_changed(&mut self, pos: BlockPos, state: u16, source_block: u16) {
        let kind = Kinds::get().of_state(state);
        if kind.behaviour == Behaviour::Liquid {
            fluid::liquid_block_changed(self, pos, state);
        }
        if kind.redstone.is_some() {
            crate::redstone::neighbor_changed(self, pos, state, source_block);
        }
    }

    /// Whether the block `state` at `pos` still has the support it needs, given that the
    /// neighbour in direction `dir` changed. Blocks without needs always do.
    fn survives(&self, state: u16, dir: Direction, pos: BlockPos) -> bool {
        let kinds = Kinds::get();
        let Some(rule) = kinds.of_state(state).support else { return true };
        // Each rule only reacts to the neighbour it depends on.
        let sturdy = |at: BlockPos, face: Direction, support: DataSupport| {
            self.blocks.block(at).is_some_and(|s| facts(s).is_face_sturdy(face.index(), support))
        };
        match rule {
            Support::CenterBelow => {
                dir != Direction::Down || sturdy(pos.below(), Direction::Up, DataSupport::Center)
            }
            Support::FullBehind => {
                let facing = kinds
                    .property(state, "facing")
                    .and_then(Direction::from_name)
                    .expect("wall-mounted blocks have a facing");
                dir != facing.opposite()
                    || sturdy(pos.offset(facing.opposite()), facing, DataSupport::Full)
            }
            Support::Lantern => {
                let hanging = kinds.property(state, "hanging") == Some("true");
                let toward = if hanging { Direction::Up } else { Direction::Down };
                dir != toward || sturdy(pos.offset(toward), toward.opposite(), DataSupport::Center)
            }
            Support::NotAirBelow => {
                self.blocks.block(pos.below()).is_some_and(|s| !facts(s).has(flag::AIR))
            }
            Support::SnowLayer => {
                let Some(below) = self.blocks.block(pos.below()) else { return false };
                let below_kind = kinds.of_state(below);
                if below_kind.snow_never {
                    false
                } else if below_kind.snow_always {
                    true
                } else {
                    // A full top face, or a full stack of snow layers.
                    crate::shape::merged_face_occludes(
                        facts(below).collision_shape(),
                        &[],
                        Direction::Up,
                    ) || (below_kind.support == Some(Support::SnowLayer)
                        && kinds.property(below, "layers") == Some("8"))
                }
            }
            Support::RigidOrCenterBelow => {
                dir != Direction::Down
                    || sturdy(pos.below(), Direction::Up, DataSupport::Rigid)
                    || sturdy(pos.below(), Direction::Up, DataSupport::Center)
            }
            Support::WireBase => {
                dir != Direction::Down
                    || self
                        .blocks
                        .block(pos.below())
                        .is_some_and(crate::redstone::wire_can_survive_on)
            }
            Support::RigidBelow => {
                dir != Direction::Down || sturdy(pos.below(), Direction::Up, DataSupport::Rigid)
            }
            Support::Attached => {
                let toward = crate::redstone::attached_direction(state);
                dir != toward.opposite()
                    || sturdy(pos.offset(toward.opposite()), toward, DataSupport::Full)
            }
            Support::Door => {
                // The other-half rule is handled in `update_shape`; this is the floor.
                let lower = kinds.property(state, "half") == Some("lower");
                !(lower && dir == Direction::Down)
                    || sturdy(pos.below(), Direction::Up, DataSupport::Full)
            }
        }
    }

    fn update_shape(
        &mut self,
        state: u16,
        dir: Direction,
        neighbor_state: u16,
        pos: BlockPos,
        _neighbor_pos: BlockPos,
    ) -> u16 {
        let kinds = Kinds::get();
        let mut state = self.update_shape_inner(state, neighbor_state, pos);
        if kinds.of_state(state).redstone == Some(crate::kinds::Redstone::Wire) {
            state = crate::redstone::wire_update_shape(self, state, dir, pos);
        }
        if kinds.of_state(state).redstone == Some(crate::kinds::Redstone::Repeater)
            && dir != Direction::Down
            && dir != Direction::Up
            && dir.is_x_axis()
                != kinds
                    .property(state, "facing")
                    .and_then(Direction::from_name)
                    .is_some_and(|f| f.is_x_axis())
        {
            state = crate::redstone::repeater_lock_state(self, pos, state);
        }
        if kinds.of_state(state).support == Some(Support::Door) {
            let half = kinds.property(state, "half").expect("doors have halves");
            let toward_other = if half == "lower" { Direction::Up } else { Direction::Down };
            if dir == toward_other {
                // The other half must be the matching half of the same door; this half then
                // copies everything else (open, hinge, powered, facing) from it.
                let same_door = STATE_TO_BLOCK[usize::from(neighbor_state)]
                    == STATE_TO_BLOCK[usize::from(state)];
                return match kinds.property(neighbor_state, "half") {
                    Some(other) if same_door && other != half => {
                        kinds.with_property(neighbor_state, "half", half).expect("valid half")
                    }
                    _ => kinds.air,
                };
            }
        }
        let state = self.connect(state, dir, neighbor_state);
        if self.survives(state, dir, pos) { state } else { kinds.air }
    }

    /// Panes and fences: the arm toward `dir` follows what the neighbour there offers.
    fn connect(&self, state: u16, dir: Direction, neighbor: u16) -> u16 {
        let kinds = Kinds::get();
        let Some(rule) = kinds.of_state(state).connects else { return state };
        if !dir.is_horizontal() {
            return state;
        }
        let other = kinds.of_state(neighbor);
        let sturdy = facts(neighbor).is_face_sturdy(dir.opposite().index(), DataSupport::Full)
            && !other.connection_exception;
        let attach = sturdy
            || match rule {
                Connects::Pane => other.connects == Some(Connects::Pane) || other.is_wall,
                Connects::Fence { wooden } => {
                    other.connects == Some(Connects::Fence { wooden })
                        || (other.is_fence_gate
                            // A gate joins fences on its sides, i.e. across its facing.
                            && kinds
                                .property(neighbor, "facing")
                                .and_then(Direction::from_name)
                                .is_some_and(|facing| facing.is_x_axis() != dir.is_x_axis()))
                }
            };
        kinds
            .with_property(state, dir.name(), if attach { "true" } else { "false" })
            .expect("connecting blocks have an arm per horizontal direction")
    }

    fn update_shape_inner(&mut self, state: u16, neighbor_state: u16, pos: BlockPos) -> u16 {
        let kinds = Kinds::get();
        match kinds.of_state(state).behaviour {
            Behaviour::Liquid => {
                let own = FluidState::of(state);
                if own.source || FluidState::of(neighbor_state).source {
                    let delay = fluid::tick_delay(own.kind, &self.rules);
                    self.schedule_fluid_tick(pos, own.type_id(), delay);
                }
            }
            Behaviour::Waterloggable => {
                if kinds.is_waterlogged(state) {
                    let delay = fluid::tick_delay(fluid::Kind::Water, &self.rules);
                    self.schedule_fluid_tick(pos, fluid::TYPE_WATER, delay);
                }
            }
            Behaviour::Inert => {}
        }
        state
    }

    // ---- ticking --------------------------------------------------------------------------

    /// Advances the world by one game tick.
    pub fn tick(&mut self) {
        self.time += 1;
        // Block ticks run before fluid ticks.
        let mut due = Vec::new();
        self.block_ticks.take_due(self.time, &mut due);
        self.running_ticks = due;
        self.running_next = 0;
        while self.running_next < self.running_ticks.len() {
            let (pos, block) = self.running_ticks[self.running_next];
            self.running_next += 1;
            let Some(state) = self.blocks.block(pos) else { continue };
            if STATE_TO_BLOCK[usize::from(state)] == block
                && Kinds::get().of_state(state).redstone.is_some()
            {
                crate::redstone::tick(self, pos, state);
            }
        }
        self.running_ticks.clear();
        let mut due = Vec::new();
        self.fluid_ticks.take_due(self.time, &mut due);
        for (pos, kind) in due {
            let Some(state) = self.blocks.block(pos) else { continue };
            let fluid = FluidState::of(state);
            if !fluid.is_empty() && fluid.type_id() == kind {
                fluid::tick(self, pos, state, fluid);
            }
        }
    }

    pub(crate) fn is_solid(state: u16) -> bool {
        facts(state).has(flag::SOLID)
    }
}
