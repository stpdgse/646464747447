//! Pistons: powering rules, the structure resolver, block events, and the moving-piston block
//! entities that carry blocks across for two ticks.

use ferrite_data::blocks::STATE_TO_BLOCK;
use ferrite_data::shapes::{facts, flag};

use crate::kinds::{Kinds, Piston};
use crate::level::BlockAccess;
use crate::pos::{BlockPos, Direction};
use crate::world::{World, update};

/// Values of [`ferrite_data::shapes::StateFacts::piston_reaction`].
mod reaction {
    pub const NORMAL: u8 = 0;
    pub const PUSH_ONLY: u8 = 1;
    pub const DESTROY: u8 = 2;
    pub const BLOCK: u8 = 3;
}

/// A piston pushes at most this many blocks.
const PUSH_LIMIT: usize = 12;

/// Halves of a block travelled; two ticks of one half each.
const DONE: u8 = 2;

fn block_of(state: u16) -> u16 {
    STATE_TO_BLOCK[usize::from(state)]
}

fn kind_of(state: u16) -> Option<Piston> {
    Kinds::get().of_state(state).piston
}

fn prop(state: u16, name: &str) -> Option<&'static str> {
    Kinds::get().property(state, name)
}

pub(crate) fn facing_of(state: u16) -> Direction {
    prop(state, "facing").and_then(Direction::from_name).expect("piston parts have a facing")
}

fn is_air(state: u16) -> bool {
    facts(state).has(flag::AIR)
}

fn set_prop(state: u16, name: &str, value: &str) -> u16 {
    Kinds::get().with_property(state, name, value).expect("piston property")
}

/// A block entity of a `moving_piston` block.
#[derive(Clone, Debug)]
pub(crate) struct MovingBe {
    pub pos: BlockPos,
    pub moved_state: u16,
    pub facing: Direction,
    pub extending: bool,
    pub source: bool,
    pub progress: u8,
    pub progress_o: u8,
    pub last_ticked: i64,
    pub removed: bool,
}

// ---- block entities -----------------------------------------------------------------------

fn add_be<A: BlockAccess>(world: &mut World<A>, be: MovingBe) {
    let pos = be.pos;
    // Like `Level.setBlockEntity`: a block entity already at this position is replaced.
    remove_be(world, pos);
    let idx = world.bes.len();
    world.bes.push(be);
    world.be_at.insert(pos, idx);
    world.be_tickers.push(idx);
}

fn remove_be<A: BlockAccess>(world: &mut World<A>, pos: BlockPos) {
    if let Some(idx) = world.be_at.remove(&pos) {
        world.bes[idx].removed = true;
    }
}

fn be_index<A: BlockAccess>(world: &World<A>, pos: BlockPos) -> Option<usize> {
    world.be_at.get(&pos).copied().filter(|i| !world.bes[*i].removed)
}

pub(crate) fn tick_block_entities<A: BlockAccess>(world: &mut World<A>) {
    // Entities added while ticking (a piston extending adds some) tick in the same pass.
    let mut i = 0;
    while i < world.be_tickers.len() {
        let idx = world.be_tickers[i];
        i += 1;
        if world.bes[idx].removed {
            continue;
        }
        tick_be(world, idx);
    }
    let bes = &world.bes;
    world.be_tickers.retain(|i| !bes[*i].removed);
}

fn tick_be<A: BlockAccess>(world: &mut World<A>, idx: usize) {
    let time = world.time();
    world.bes[idx].last_ticked = time;
    world.bes[idx].progress_o = world.bes[idx].progress;
    if world.bes[idx].progress_o >= DONE {
        let be = world.bes[idx].clone();
        remove_be(world, be.pos);
        if world.block(be.pos).is_some_and(|s| kind_of(s) == Some(Piston::Moving)) {
            let adapted = world.update_from_neighbor_shapes(be.moved_state, be.pos);
            if is_air(adapted) {
                world.set_block(be.pos, be.moved_state, 84);
                update_or_destroy(world, be.moved_state, adapted, be.pos, update::ALL);
            } else {
                let placed = unwaterlog(adapted);
                world.set_block(be.pos, placed, 67);
                world.neighbor_changed_at(be.pos, block_of(placed));
            }
        }
    } else {
        world.bes[idx].progress = (world.bes[idx].progress + 1).min(DONE);
    }
}

fn unwaterlog(state: u16) -> u16 {
    if prop(state, "waterlogged") == Some("true") {
        set_prop(state, "waterlogged", "false")
    } else {
        state
    }
}

/// `PistonMovingBlockEntity.finalTick`: complete the move immediately.
fn final_tick<A: BlockAccess>(world: &mut World<A>, idx: usize) {
    if world.bes[idx].progress_o >= DONE {
        return;
    }
    world.bes[idx].progress = DONE;
    world.bes[idx].progress_o = DONE;
    let be = world.bes[idx].clone();
    remove_be(world, be.pos);
    if world.block(be.pos).is_some_and(|s| kind_of(s) == Some(Piston::Moving)) {
        let placed = if be.source {
            Kinds::get().air
        } else {
            world.update_from_neighbor_shapes(be.moved_state, be.pos)
        };
        world.set_block(be.pos, placed, update::ALL);
        world.neighbor_changed_at(be.pos, block_of(placed));
    }
}

/// `Block.updateOrDestroy`.
fn update_or_destroy<A: BlockAccess>(
    world: &mut World<A>,
    old: u16,
    new: u16,
    pos: BlockPos,
    flags: u32,
) {
    if old == new {
        return;
    }
    if is_air(new) {
        if !is_air(old) {
            destroy_block(world, pos);
        }
    } else {
        world.set_block(pos, new, flags & !update::SUPPRESS_DROPS);
    }
}

/// `Level.destroyBlock` without drops: leaves the fluid the block held, if any.
fn destroy_block<A: BlockAccess>(world: &mut World<A>, pos: BlockPos) {
    let left =
        world.block(pos).map_or(Kinds::get().air, |s| crate::fluid::FluidState::of(s).block());
    world.set_block(pos, left, update::ALL);
}

/// `Level.removeBlock`.
fn remove_block<A: BlockAccess>(world: &mut World<A>, pos: BlockPos) {
    destroy_block(world, pos);
}

// ---- block events ------------------------------------------------------------------------

pub(crate) fn run_block_events<A: BlockAccess>(world: &mut World<A>) {
    while let Some((pos, block, id, param)) = world.block_events.pop_front() {
        let Some(state) = world.block(pos) else { continue };
        if block_of(state) == block {
            trigger_event(world, pos, state, id, param);
        }
    }
}

// ---- hooks -------------------------------------------------------------------------------

pub(crate) fn on_place<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, state: u16, old: u16) {
    if let Some(Piston::Base { .. }) = kind_of(state)
        && block_of(old) != block_of(state)
        && be_index(world, pos).is_none()
    {
        check_if_extend(world, pos, state);
    }
}

pub(crate) fn on_remove<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, old: u16) {
    match kind_of(old) {
        Some(Piston::Head) => {
            let base = pos.offset(facing_of(old).opposite());
            if world.block(base).is_some_and(|b| is_fitting_base(old, b)) {
                destroy_block(world, base);
            }
        }
        Some(Piston::Moving) => {
            if let Some(idx) = be_index(world, pos) {
                final_tick(world, idx);
            }
        }
        _ => {}
    }
}

pub(crate) fn neighbor_changed<A: BlockAccess>(
    world: &mut World<A>,
    pos: BlockPos,
    state: u16,
    _source: u16,
) {
    match kind_of(state) {
        Some(Piston::Base { .. }) => check_if_extend(world, pos, state),
        Some(Piston::Head) => {
            if head_can_survive(world, state, pos) {
                let base = pos.offset(facing_of(state).opposite());
                world.neighbor_changed_at(base, block_of(state));
            }
        }
        Some(Piston::Moving) if be_index(world, pos).is_none() => remove_block(world, pos),
        Some(Piston::Moving) | None => {}
    }
}

fn is_fitting_base(head: u16, base: u16) -> bool {
    let sticky = prop(head, "type") == Some("sticky");
    matches!(kind_of(base), Some(Piston::Base { sticky: s }) if s == sticky)
        && prop(base, "extended") == Some("true")
        && facing_of(base) == facing_of(head)
}

pub(crate) fn head_can_survive<A: BlockAccess>(world: &World<A>, head: u16, pos: BlockPos) -> bool {
    let facing = facing_of(head);
    world.block(pos.offset(facing.opposite())).is_some_and(|b| {
        is_fitting_base(head, b) || (kind_of(b) == Some(Piston::Moving) && facing_of(b) == facing)
    })
}

// ---- powering ----------------------------------------------------------------------------

/// Whether the piston at `pos` is powered, including the "quasi-connected" power from the
/// block above it.
fn neighbor_signal<A: BlockAccess>(world: &World<A>, pos: BlockPos, facing: Direction) -> bool {
    use crate::redstone::signal;
    for d in Direction::ALL {
        if d != facing && signal(world, pos.offset(d), d) > 0 {
            return true;
        }
    }
    if signal(world, pos, Direction::Down) > 0 {
        return true;
    }
    let above = pos.above();
    for d in Direction::ALL {
        if d != Direction::Down && signal(world, above.offset(d), d) > 0 {
            return true;
        }
    }
    false
}

fn check_if_extend<A: BlockAccess>(world: &mut World<A>, pos: BlockPos, state: u16) {
    let facing = facing_of(state);
    let extended = prop(state, "extended") == Some("true");
    let should = neighbor_signal(world, pos, facing);
    let block = block_of(state);
    if should && !extended {
        if Resolver::new(world, pos, facing, true).resolve().is_some() {
            world.block_event(pos, block, 0, facing.index() as i32);
        }
    } else if !should && extended {
        let front2 = pos.offset(facing).offset(facing);
        let mut kind = 1;
        if let Some(s) = world.block(front2)
            && kind_of(s) == Some(Piston::Moving)
            && facing_of(s) == facing
            && let Some(idx) = be_index(world, front2)
        {
            let be = &world.bes[idx];
            if be.extending
                && (be.progress < 1 || world.time() == be.last_ticked || world.handling_tick)
            {
                kind = 2;
            }
        }
        world.block_event(pos, block, kind, facing.index() as i32);
    }
}

// ---- the event ---------------------------------------------------------------------------

fn trigger_event<A: BlockAccess>(
    world: &mut World<A>,
    pos: BlockPos,
    state: u16,
    id: i32,
    param: i32,
) -> bool {
    let Some(Piston::Base { sticky }) = kind_of(state) else { return false };
    let facing = facing_of(state);
    let extended_state = set_prop(state, "extended", "true");
    let signal = neighbor_signal(world, pos, facing);
    if signal && (id == 1 || id == 2) {
        world.set_block(pos, extended_state, update::CLIENTS);
        return false;
    }
    if !signal && id == 0 {
        return false;
    }
    if id == 0 {
        if !move_blocks(world, pos, facing, true, sticky) {
            return false;
        }
        world.set_block(pos, extended_state, 67);
    } else if id == 1 || id == 2 {
        let front = pos.offset(facing);
        if let Some(idx) = be_index(world, front) {
            final_tick(world, idx);
        }
        let kinds = Kinds::get();
        let moving = set_prop(
            set_prop(kinds.moving_piston, "facing", facing.name()),
            "type",
            if sticky { "sticky" } else { "normal" },
        );
        world.set_block(pos, moving, 20);
        let base = set_prop(state, "facing", Direction::ALL[(param & 7) as usize].name());
        let base = set_prop(base, "extended", "false");
        add_be(
            world,
            MovingBe {
                pos,
                moved_state: base,
                facing,
                extending: false,
                source: true,
                progress: 0,
                progress_o: 0,
                last_ticked: world.time(),
                removed: false,
            },
        );
        world.update_neighbors_at(pos, block_of(moving));
        world.neighbour_shape_updates(pos, moving, 2, 512);
        if sticky {
            let p = front.offset(facing);
            let s = world.block(p).unwrap_or(kinds.air);
            let mut handled = false;
            if kind_of(s) == Some(Piston::Moving)
                && let Some(idx) = be_index(world, p)
            {
                let be = &world.bes[idx];
                if be.facing == facing && be.extending {
                    final_tick(world, idx);
                    handled = true;
                }
            }
            if !handled {
                let kind_reaction = facts(s).piston_reaction();
                let pullable = id == 1
                    && !is_air(s)
                    && is_pushable(world, s, p, facing.opposite(), false, facing)
                    && (kind_reaction == reaction::NORMAL
                        || matches!(kind_of(s), Some(Piston::Base { .. })));
                if pullable {
                    move_blocks(world, pos, facing, false, sticky);
                } else {
                    remove_block(world, front);
                }
            }
        } else {
            remove_block(world, front);
        }
    }
    true
}

// ---- moving blocks -----------------------------------------------------------------------

fn move_blocks<A: BlockAccess>(
    world: &mut World<A>,
    pos: BlockPos,
    facing: Direction,
    extending: bool,
    sticky: bool,
) -> bool {
    let kinds = Kinds::get();
    if !extending {
        remove_block(world, pos.offset(facing));
    }
    let Some((to_push, to_destroy)) = Resolver::new(world, pos, facing, extending).resolve() else {
        return false;
    };
    // (position, state) of every block that is about to move, in push order.
    let mut map: Vec<(BlockPos, u16)> = Vec::new();
    let mut pushed_states = Vec::new();
    for &p in &to_push {
        let s = world.block(p).unwrap_or(kinds.air);
        pushed_states.push(s);
        map.push((p, s));
    }
    let dir = if extending { facing } else { facing.opposite() };
    let mut states: Vec<u16> = Vec::new();
    for &p in to_destroy.iter().rev() {
        let s = world.block(p).unwrap_or(kinds.air);
        world.set_block(p, kinds.air, 18);
        states.push(s);
    }
    for (j, &p) in to_push.iter().enumerate().rev() {
        let s = world.block(p).unwrap_or(kinds.air);
        let dest = p.offset(dir);
        map.retain(|(q, _)| *q != dest);
        let moving = set_prop(kinds.moving_piston, "facing", facing.name());
        world.set_block(dest, moving, 68);
        add_be(
            world,
            MovingBe {
                pos: dest,
                moved_state: pushed_states[j],
                facing,
                extending,
                source: false,
                progress: 0,
                progress_o: 0,
                last_ticked: world.time(),
                removed: false,
            },
        );
        states.push(s);
    }
    if extending {
        let ty = if sticky { "sticky" } else { "normal" };
        let head = set_prop(set_prop(kinds.piston_head, "facing", facing.name()), "type", ty);
        let moving = set_prop(set_prop(kinds.moving_piston, "facing", facing.name()), "type", ty);
        let dest = pos.offset(facing);
        map.retain(|(q, _)| *q != dest);
        world.set_block(dest, moving, 68);
        add_be(
            world,
            MovingBe {
                pos: dest,
                moved_state: head,
                facing,
                extending: true,
                source: true,
                progress: 0,
                progress_o: 0,
                last_ticked: world.time(),
                removed: false,
            },
        );
    }
    // Blocks that left their cell without being replaced become air, in Java HashMap order.
    let keys: Vec<BlockPos> = map.iter().map(|(p, _)| *p).collect();
    let ordered = crate::redstone::java_hash_set_order(&keys);
    for p in &ordered {
        world.set_block(*p, kinds.air, 82);
    }
    for p in &ordered {
        let s = map.iter().find(|(q, _)| q == p).map(|(_, s)| *s).expect("key from the map");
        world.indirect_shape_updates(*p, s, 2, 512);
        world.neighbour_shape_updates(*p, kinds.air, 2, 512);
        world.indirect_shape_updates(*p, kinds.air, 2, 512);
    }
    let mut i = 0;
    for &p in to_destroy.iter().rev() {
        let s = states[i];
        i += 1;
        world.indirect_shape_updates(p, s, 2, 512);
        world.update_neighbors_at(p, block_of(s));
    }
    for &p in to_push.iter().rev() {
        let s = states[i];
        i += 1;
        world.update_neighbors_at(p, block_of(s));
    }
    if extending {
        world.update_neighbors_at(pos.offset(facing), block_of(kinds.piston_head));
    }
    true
}

// ---- pushability and the resolver --------------------------------------------------------

fn is_pushable<A: BlockAccess>(
    world: &World<A>,
    state: u16,
    pos: BlockPos,
    move_dir: Direction,
    destroy: bool,
    piston_facing: Direction,
) -> bool {
    let dim = ferrite_world::Dimension::OVERWORLD;
    let dim = {
        let d = world.blocks.dimension();
        let _ = dim;
        d
    };
    if pos.y < dim.min_y || pos.y > dim.max_y() - 1 {
        return false;
    }
    if is_air(state) {
        return true;
    }
    let kind = Kinds::get().of_state(state);
    if kind.piston_immovable {
        return false;
    }
    if move_dir == Direction::Down && pos.y == dim.min_y {
        return false;
    }
    if move_dir == Direction::Up && pos.y == dim.max_y() - 1 {
        return false;
    }
    if !matches!(kind.piston, Some(Piston::Base { .. })) {
        if facts(state).has(flag::UNBREAKABLE) {
            return false;
        }
        match facts(state).piston_reaction() {
            reaction::BLOCK => return false,
            reaction::DESTROY => return destroy,
            reaction::PUSH_ONLY => return move_dir == piston_facing,
            _ => {}
        }
    } else if prop(state, "extended") == Some("true") {
        return false;
    }
    !facts(state).has(flag::HAS_BLOCK_ENTITY)
}

struct Resolver<'a, A: BlockAccess> {
    world: &'a World<A>,
    piston: BlockPos,
    piston_dir: Direction,
    extending: bool,
    push_dir: Direction,
    start: BlockPos,
    to_push: Vec<BlockPos>,
    to_destroy: Vec<BlockPos>,
}

impl<'a, A: BlockAccess> Resolver<'a, A> {
    fn new(world: &'a World<A>, piston: BlockPos, dir: Direction, extending: bool) -> Self {
        let (push_dir, start) = if extending {
            (dir, piston.offset(dir))
        } else {
            (dir.opposite(), piston.offset(dir).offset(dir))
        };
        Resolver {
            world,
            piston,
            piston_dir: dir,
            extending,
            push_dir,
            start,
            to_push: Vec::new(),
            to_destroy: Vec::new(),
        }
    }

    fn state(&self, pos: BlockPos) -> u16 {
        self.world.block(pos).unwrap_or(
            ferrite_data::blocks::block_by_name("bedrock").expect("bedrock").default_state,
        )
    }

    fn sticky(&self, state: u16) -> bool {
        Kinds::get().of_state(state).sticky
    }

    fn can_stick(&self, a: u16, b: u16) -> bool {
        let k = Kinds::get();
        let (ba, bb) = (block_of(a), block_of(b));
        if ba == k.honey_block && bb == k.slime_block {
            return false;
        }
        if ba == k.slime_block && bb == k.honey_block {
            return false;
        }
        self.sticky(a) || self.sticky(b)
    }

    fn resolve(mut self) -> Option<(Vec<BlockPos>, Vec<BlockPos>)> {
        let s = self.state(self.start);
        if !is_pushable(self.world, s, self.start, self.push_dir, false, self.piston_dir) {
            if self.extending && facts(s).piston_reaction() == reaction::DESTROY {
                return Some((Vec::new(), vec![self.start]));
            }
            return None;
        }
        if !self.add_block_line(self.start, self.push_dir) {
            return None;
        }
        let mut i = 0;
        while i < self.to_push.len() {
            let p = self.to_push[i];
            if self.sticky(self.state(p)) && !self.add_branching(p) {
                return None;
            }
            i += 1;
        }
        Some((self.to_push, self.to_destroy))
    }

    fn add_block_line(&mut self, origin: BlockPos, dir: Direction) -> bool {
        let mut s = self.state(origin);
        if is_air(s) {
            return true;
        }
        if !is_pushable(self.world, s, origin, self.push_dir, false, dir) {
            return true;
        }
        if origin == self.piston || self.to_push.contains(&origin) {
            return true;
        }
        let mut i = 1usize;
        if i + self.to_push.len() > PUSH_LIMIT {
            return false;
        }
        while self.sticky(s) {
            let p = back(origin, self.push_dir, i);
            let prev = s;
            s = self.state(p);
            if is_air(s)
                || !self.can_stick(prev, s)
                || !is_pushable(self.world, s, p, self.push_dir, false, self.push_dir.opposite())
                || p == self.piston
            {
                break;
            }
            i += 1;
            if i + self.to_push.len() > PUSH_LIMIT {
                return false;
            }
        }
        let mut j = 0usize;
        for k in (0..i).rev() {
            self.to_push.push(back(origin, self.push_dir, k));
            j += 1;
        }
        let mut k = 1usize;
        loop {
            let p = ahead(origin, self.push_dir, k);
            if let Some(l) = self.to_push.iter().position(|q| *q == p) {
                self.reorder(j, l);
                for m in 0..=(l + j) {
                    let q = self.to_push[m];
                    if self.sticky(self.state(q)) && !self.add_branching(q) {
                        return false;
                    }
                }
                return true;
            }
            s = self.state(p);
            if is_air(s) {
                return true;
            }
            if !is_pushable(self.world, s, p, self.push_dir, true, self.push_dir)
                || p == self.piston
            {
                return false;
            }
            if facts(s).piston_reaction() == reaction::DESTROY {
                self.to_destroy.push(p);
                return true;
            }
            if self.to_push.len() >= PUSH_LIMIT {
                return false;
            }
            self.to_push.push(p);
            j += 1;
            k += 1;
        }
    }

    fn reorder(&mut self, offset: usize, index: usize) {
        let n = self.to_push.len();
        let a: Vec<_> = self.to_push[..index].to_vec();
        let b: Vec<_> = self.to_push[n - offset..].to_vec();
        let c: Vec<_> = self.to_push[index..n - offset].to_vec();
        self.to_push.clear();
        self.to_push.extend(a);
        self.to_push.extend(b);
        self.to_push.extend(c);
    }

    fn add_branching(&mut self, from: BlockPos) -> bool {
        let s = self.state(from);
        for d in Direction::ALL {
            if axis(d) != axis(self.push_dir) {
                let p = from.offset(d);
                let t = self.state(p);
                if self.can_stick(t, s) && !self.add_block_line(p, d) {
                    return false;
                }
            }
        }
        true
    }
}

fn axis(d: Direction) -> u8 {
    match d {
        Direction::Down | Direction::Up => 1,
        Direction::North | Direction::South => 2,
        _ => 0,
    }
}

fn ahead(origin: BlockPos, dir: Direction, n: usize) -> BlockPos {
    let (dx, dy, dz) = dir.step();
    let n = n as i32;
    BlockPos::new(origin.x + dx * n, origin.y + dy * n, origin.z + dz * n)
}

fn back(origin: BlockPos, dir: Direction, n: usize) -> BlockPos {
    ahead(origin, dir.opposite(), n)
}
