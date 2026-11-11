//! Placing a block the way a player does.
//!
//! The state a block takes when placed depends on how it was clicked: which face, where on the
//! face, and which way the player looks. Most of that is a handful of formulas, kept in a table
//! generated from recordings of the official server ([`crate::generated_placement`]); what depends
//! on the neighbours, or places several blocks, is code in this module. Everything is checked
//! against the recordings by `tests/placement.rs`.

use ferrite_data::blocks::block_by_name;
use ferrite_data::shapes::{facts, flag};

use ferrite_data::blocks::block_of_state;

use crate::fluid::{FluidState, Kind};
use crate::kinds::Kinds;
use crate::level::BlockAccess;
use crate::pos::{BlockPos, Direction};
use crate::world::{World, update};

/// How a click looks to the placement rules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceContext {
    /// The block that was clicked.
    pub clicked: BlockPos,
    /// The face of it that was clicked.
    pub face: Direction,
    /// Where on that block the click landed, each coordinate in `0.0..=1.0`.
    pub cursor: [f32; 3],
    /// The player's look direction in degrees (yaw 0 faces south, -90 east).
    pub yaw: f32,
    pub pitch: f32,
    pub sneaking: bool,
    /// Whether the player may place game-master blocks (command blocks, jigsaws, ...).
    pub operator: bool,
}

/// A rule for one property of a block, evaluated against the click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formula {
    /// A fixed value.
    Const(&'static str),
    /// The clicked face, or its opposite.
    DirF,
    DirFOpp,
    /// The horizontal direction the player faces, its opposite, or the one clockwise of it.
    DirH,
    DirHOpp,
    DirHCw,
    DirHCcw,
    /// The direction the player looks along most, or its opposite.
    DirN,
    DirNOpp,
    /// The axis of the clicked face / look direction / facing direction.
    AxisF,
    AxisN,
    AxisH,
    /// `bottom` or `top` by where the block sits relative to the clicked face and hit height.
    HalfSlab,
    /// `floor`, `ceiling` or `wall` by the clicked face.
    FaceAttach,
    /// The 16-step rotation of the player's yaw (signs, banners).
    Rot16,
    /// `east_west` when the player faces east or west, else `north_south` (rails).
    RailShape,
}

/// The horizontal direction a player with this yaw faces.
pub fn horizontal_direction(yaw: f32) -> Direction {
    match ((f64::from(yaw) / 90.0 + 0.5).floor() as i64) & 3 {
        0 => Direction::South,
        1 => Direction::West,
        2 => Direction::North,
        _ => Direction::East,
    }
}

/// The game's sine table: 65536 float entries, indexed from a double-precision angle.
fn sin_table() -> &'static [f32; 65536] {
    static TABLE: std::sync::OnceLock<Box<[f32; 65536]>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = Box::new([0f32; 65536]);
        for (i, v) in table.iter_mut().enumerate() {
            *v = (i as f64 * std::f64::consts::PI * 2.0 / 65536.0).sin() as f32;
        }
        table
    })
}

fn table_sin(x: f32) -> f32 {
    sin_table()[((f64::from(x) * 10430.378350470453) as i32 & 65535) as usize]
}

fn table_cos(x: f32) -> f32 {
    sin_table()[((f64::from(x) * 10430.378350470453 + 16384.0) as i32 & 65535) as usize]
}

/// The six directions ordered by how well they match where the player looks (vanilla's
/// `Direction.orderedByNearest`): the best three, then their opposites in reverse.
///
/// The arithmetic is the game's, bit for bit (float angles, the table sine, float products),
/// because players looking along a diagonal produce exact ties and the tie-break falls out of
/// rounding. Checked against 141 recorded placements, including every yaw and pitch multiple
/// of 45 degrees.
pub fn looking_directions(yaw: f32, pitch: f32) -> [Direction; 6] {
    const DEG: f32 = std::f32::consts::PI / 180.0;
    let f = pitch * DEG;
    let g = -yaw * DEG;
    let (h, i, j, k) = (table_sin(f), table_cos(f), table_sin(g), table_cos(g));
    let (east, up, south) = (j > 0.0, h < 0.0, k > 0.0);
    let l = if east { j } else { -j };
    let m = if up { -h } else { h };
    let n = if south { k } else { -k };
    let (o, p) = (l * i, n * i);
    let d1 = if east { Direction::East } else { Direction::West };
    let d2 = if up { Direction::Up } else { Direction::Down };
    let d3 = if south { Direction::South } else { Direction::North };
    let [a, b, c] = if l > n {
        if m > o {
            [d2, d1, d3]
        } else if p > m {
            [d1, d3, d2]
        } else {
            [d1, d2, d3]
        }
    } else if m > p {
        [d2, d3, d1]
    } else if o > m {
        [d3, d1, d2]
    } else {
        [d3, d2, d1]
    };
    [a, b, c, c.opposite(), b.opposite(), a.opposite()]
}

fn axis_name(d: Direction) -> &'static str {
    match d {
        Direction::Down | Direction::Up => "y",
        Direction::North | Direction::South => "z",
        Direction::West | Direction::East => "x",
    }
}

impl PlaceContext {
    /// The cursor's height relative to the block being placed (not the clicked block).
    fn local_cursor_y(&self) -> f32 {
        self.cursor[1] - self.face.step().1 as f32
    }

    /// The direction the player looks along most.
    fn nearest(&self) -> Direction {
        looking_directions(self.yaw, self.pitch)[0]
    }

    fn horizontal(&self) -> Direction {
        horizontal_direction(self.yaw)
    }

    /// `getNearestLookingDirections`: the look order, except that when the click did not replace
    /// the clicked block the direction into the clicked block comes first.
    fn ordered(&self, replacing: bool) -> [Direction; 6] {
        let mut dirs = looking_directions(self.yaw, self.pitch);
        if !replacing {
            let into = self.face.opposite();
            if let Some(i) = dirs.iter().position(|d| *d == into) {
                dirs[..=i].rotate_right(1);
            }
        }
        dirs
    }

    fn evaluate(&self, f: Formula) -> &'static str {
        let n = self.nearest();
        let h = self.horizontal();
        match f {
            Formula::Const(v) => v,
            Formula::DirF => self.face.name(),
            Formula::DirFOpp => self.face.opposite().name(),
            Formula::DirH => h.name(),
            Formula::DirHOpp => h.opposite().name(),
            Formula::DirHCw => h.clockwise().name(),
            Formula::DirHCcw => h.counter_clockwise().name(),
            Formula::DirN => n.name(),
            Formula::DirNOpp => n.opposite().name(),
            Formula::AxisF => axis_name(self.face),
            Formula::AxisN => axis_name(n),
            Formula::AxisH => axis_name(h),
            Formula::HalfSlab => {
                if self.face != Direction::Down
                    && (self.face == Direction::Up || self.local_cursor_y() <= 0.5)
                {
                    "bottom"
                } else {
                    "top"
                }
            }
            Formula::FaceAttach => match self.face {
                Direction::Up => "floor",
                Direction::Down => "ceiling",
                _ => "wall",
            },
            Formula::Rot16 => ROTATIONS[self.segment(0.0)],
            Formula::RailShape => {
                if h.is_x_axis() {
                    "east_west"
                } else {
                    "north_south"
                }
            }
        }
    }

    /// The 16-step rotation segment of the player's yaw plus `extra` degrees.
    fn segment(&self, extra: f32) -> usize {
        ((((f64::from(self.yaw) + f64::from(extra) + 180.0) * 16.0 / 360.0 + 0.5).floor() as i64)
            & 15) as usize
    }
}

const ROTATIONS: [&str; 16] =
    ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15"];

fn rules_for(name: &str) -> &'static [(&'static str, Formula)] {
    crate::generated_placement::RULES
        .binary_search_by(|(n, _)| (*n).cmp(name))
        .map_or(&[], |i| crate::generated_placement::RULES[i].1)
}

/// What placing a block puts into the world: the block itself and, for two-part blocks, the
/// second part.
struct Placed {
    state: u16,
    second: Option<(BlockPos, u16)>,
}

/// A block's wall-mounted twin, for items that become one when clicked on a side face.
fn wall_variant(name: &str) -> Option<String> {
    let wall = if let Some(prefix) = name.strip_suffix("hanging_sign") {
        format!("{prefix}wall_hanging_sign")
    } else if let Some(prefix) = name.strip_suffix("torch") {
        format!("{prefix}wall_torch")
    } else if let Some(prefix) = name.strip_suffix("_sign") {
        format!("{prefix}_wall_sign")
    } else if let Some(prefix) = name.strip_suffix("_banner") {
        format!("{prefix}_wall_banner")
    } else if let Some(prefix) = name.strip_suffix("_head") {
        format!("{prefix}_wall_head")
    } else if let Some(prefix) = name.strip_suffix("_skull") {
        format!("{prefix}_wall_skull")
    } else {
        let prefix = name.strip_suffix("_coral_fan")?;
        format!("{prefix}_coral_wall_fan")
    };
    block_by_name(&wall).map(|_| wall)
}

fn is_game_master_block(name: &str) -> bool {
    matches!(
        name,
        "command_block"
            | "chain_command_block"
            | "repeating_command_block"
            | "structure_block"
            | "jigsaw"
            | "test_block"
            | "test_instance_block"
    )
}

impl<A: BlockAccess> World<A> {
    fn make(&self, block: &str, props: &[(&str, &str)]) -> Option<u16> {
        let kinds = Kinds::get();
        let mut state = block_by_name(block)?.default_state;
        for (k, v) in props {
            state = kinds.with_property(state, k, v)?;
        }
        Some(state)
    }

    fn is_replaceable_at(&self, pos: BlockPos) -> bool {
        self.block(pos).is_some_and(|s| facts(s).has(flag::REPLACEABLE))
    }

    /// Sets `waterlogged` from the fluid at `pos`, for blocks that have the property.
    fn waterlog(&self, state: u16, pos: BlockPos) -> u16 {
        let kinds = Kinds::get();
        if kinds.of_state(state).waterlogged.is_none() {
            return state;
        }
        let water = self.block(pos).is_some_and(|s| {
            let f = FluidState::of(s);
            f.is(Kind::Water) && f.source
        });
        kinds
            .with_property(state, "waterlogged", if water { "true" } else { "false" })
            .unwrap_or(state)
    }

    /// The state `block` would take if placed by this click at `pos`, before any survival check.
    fn state_for_placement(
        &self,
        block: &str,
        pos: BlockPos,
        ctx: &PlaceContext,
        replacing: bool,
    ) -> Option<Placed> {
        let kinds = Kinds::get();
        let info = block_by_name(block)?;
        let one = |state: Option<u16>| state.map(|state| Placed { state, second: None });

        if is_game_master_block(block) && !ctx.operator {
            return None;
        }
        if let Some(wall) = wall_variant(block) {
            return one(self.standing_or_wall(block, &wall, pos, ctx, replacing));
        }
        if block.ends_with("_bed") {
            let facing = ctx.horizontal();
            let head_pos = pos.offset(facing);
            if !self.is_replaceable_at(head_pos) {
                return None;
            }
            let foot = self.make(block, &[("facing", facing.name()), ("part", "foot")])?;
            let head = self.make(block, &[("facing", facing.name()), ("part", "head")])?;
            return Some(Placed { state: foot, second: Some((head_pos, head)) });
        }
        if kinds.property(info.default_state, "half") == Some("lower")
            && kinds.property(info.default_state, "hinge").is_some()
        {
            return self.door_state(block, pos, ctx);
        }
        if kinds.property(info.default_state, "half") == Some("lower") {
            // Tall plants: the upper half goes above, which must be free.
            let above = pos.above();
            if !self.is_replaceable_at(above) {
                return None;
            }
            let lower = self.waterlog(info.default_state, pos);
            let upper =
                self.waterlog(kinds.with_property(info.default_state, "half", "upper")?, above);
            return Some(Placed { state: lower, second: Some((above, upper)) });
        }
        if block == "tripwire_hook" {
            return one(self.wall_state(block, pos, ctx, replacing).map(|s| {
                let s = kinds.with_property(s, "attached", "false").unwrap_or(s);
                kinds.with_property(s, "powered", "false").unwrap_or(s)
            }));
        }
        if block.ends_with("lantern") && kinds.property(info.default_state, "hanging").is_some() {
            for dir in ctx.ordered(replacing) {
                if !matches!(dir, Direction::Up | Direction::Down) {
                    continue;
                }
                let hanging = if dir == Direction::Up { "true" } else { "false" };
                let state = self
                    .waterlog(kinds.with_property(info.default_state, "hanging", hanging)?, pos);
                if self.can_survive(state, pos) {
                    return one(Some(state));
                }
            }
            return None;
        }
        if block == "chorus_plant" {
            return one(Some(self.chorus_plant_arms(info.default_state, pos)));
        }
        if block == "shelf_mushroom" {
            return one(self.wall_state(block, pos, ctx, replacing));
        }
        if block == "scaffolding" {
            let distance = self.scaffolding_distance(pos);
            let below_is_scaffolding =
                self.block(pos.below()).is_some_and(|b| crate::kinds::block_id_of(b) == info.id);
            let bottom = distance > 0 && !below_is_scaffolding;
            let mut state =
                kinds.with_property(info.default_state, "distance", &distance.to_string())?;
            state = kinds.with_property(state, "bottom", if bottom { "true" } else { "false" })?;
            return one(Some(self.waterlog(state, pos)));
        }
        if matches!(block, "pointed_dripstone" | "sulfur_spike") {
            return one(self.spike_state(block, pos, ctx));
        }
        if block == "bell" {
            return one(self.bell_state(pos, ctx));
        }
        if matches!(block, "glow_lichen" | "sculk_vein" | "resin_clump") {
            return one(self.multiface_state(block, pos, ctx, replacing));
        }
        if block == "vine" {
            return one(self.vine_state(pos, ctx, replacing));
        }
        if block == "farmland" {
            let farmland = self.waterlog(info.default_state, pos);
            let above_solid = self.block(pos.above()).is_some_and(|s| facts(s).has(flag::SOLID));
            let state = if above_solid { block_by_name("dirt")?.default_state } else { farmland };
            return one(Some(state));
        }
        if matches!(block, "kelp" | "seagrass") {
            let water = self.block(pos).is_some_and(|s| {
                let f = FluidState::of(s);
                f.is(Kind::Water) && f.source
            });
            if !water {
                return None;
            }
        }
        if block == "mangrove_propagule" {
            return one(self.make(block, &[("age", "4")]).map(|s| self.waterlog(s, pos)));
        }
        if block == "bamboo" {
            return one(self.bamboo_state(pos));
        }
        if block == "lever" || block == "grindstone" || block.ends_with("_button") {
            return one(self.face_attached(block, pos, ctx, replacing));
        }
        if kinds.of_state(info.default_state).is_trapdoor {
            return one(Some(self.trapdoor_state(block, pos, ctx, replacing)?));
        }
        let mut state = info.default_state;
        match block {
            "hopper" => {
                let dir = ctx.face.opposite();
                let facing = if matches!(dir, Direction::Up | Direction::Down) {
                    "down"
                } else {
                    dir.name()
                };
                state = kinds.with_property(state, "facing", facing)?;
            }
            "crafter" => {
                let front = ctx.nearest().opposite();
                let top = match front {
                    Direction::Down => ctx.horizontal().opposite(),
                    Direction::Up => ctx.horizontal(),
                    _ => Direction::Up,
                };
                let value = format!("{}_{}", front.name(), top.name());
                state = kinds.with_property(state, "orientation", &value)?;
            }
            "jigsaw" => {
                let top = if matches!(ctx.face, Direction::Up | Direction::Down) {
                    ctx.horizontal().opposite()
                } else {
                    Direction::Up
                };
                let value = format!("{}_{}", ctx.face.name(), top.name());
                state = kinds.with_property(state, "orientation", &value)?;
            }
            _ => {
                for (prop, formula) in rules_for(block) {
                    let value = ctx.evaluate(*formula);
                    state = kinds.with_property(state, prop, value).unwrap_or(state);
                }
            }
        }
        one(Some(self.waterlog(state, pos)))
    }

    /// Torches, signs, banners, heads and coral fans: the standing block when the player looks
    /// down onto the clicked face, the wall block otherwise.
    fn standing_or_wall(
        &self,
        block: &str,
        wall: &str,
        pos: BlockPos,
        ctx: &PlaceContext,
        replacing: bool,
    ) -> Option<u16> {
        let kinds = Kinds::get();
        let hanging = block.ends_with("hanging_sign");
        let attachment = if hanging { Direction::Up } else { Direction::Down };
        let is_skull = wall.ends_with("_wall_head") || wall.ends_with("_wall_skull");
        let dirs = ctx.ordered(replacing);
        for dir in dirs {
            if dir == attachment.opposite() {
                continue;
            }
            let state = if dir == attachment {
                self.standing_state(block, pos, ctx)
            } else if hanging {
                self.wall_hanging_sign_state(wall, pos, ctx, replacing)
            } else if is_skull {
                self.wall_skull_state(wall, pos, ctx, replacing)
            } else {
                self.wall_state(wall, pos, ctx, replacing)
            };
            let Some(state) = state else { continue };
            if self.can_survive(state, pos) || (is_skull && !hanging) {
                let _ = kinds;
                return Some(state);
            }
        }
        None
    }

    fn standing_state(&self, block: &str, pos: BlockPos, ctx: &PlaceContext) -> Option<u16> {
        let kinds = Kinds::get();
        let mut state = block_by_name(block)?.default_state;
        if block.ends_with("hanging_sign") {
            let above = pos.above();
            let above_state = self.block(above)?;
            let above_is_hanging =
                block_of_state(above_state).is_some_and(|b| b.name.ends_with("hanging_sign"));
            let dir = ctx.horizontal();
            let mut attached = !crate::shape::merged_face_occludes(
                facts(above_state).collision_shape(),
                &[],
                Direction::Down,
            ) || ctx.sneaking;
            if above_is_hanging && !ctx.sneaking {
                if let Some(f) =
                    kinds.property(above_state, "facing").and_then(Direction::from_name)
                {
                    if axis_name(f) == axis_name(dir) {
                        attached = false;
                    }
                } else if let Some(r) = kinds.property(above_state, "rotation") {
                    let r: i32 = r.parse().ok()?;
                    if r % 4 == 0 {
                        let d =
                            [Direction::South, Direction::West, Direction::North, Direction::East]
                                [(r / 4) as usize];
                        if axis_name(d) == axis_name(dir) {
                            attached = false;
                        }
                    }
                }
            }
            let rotation = if attached {
                ctx.segment(0.0)
            } else {
                match dir.opposite() {
                    Direction::South => 0,
                    Direction::West => 4,
                    Direction::North => 8,
                    _ => 12,
                }
            };
            state =
                kinds.with_property(state, "attached", if attached { "true" } else { "false" })?;
            state = kinds.with_property(state, "rotation", ROTATIONS[rotation])?;
        } else if kinds.property(state, "rotation").is_some() {
            // Heads turn by the plain yaw; signs and banners face away from the player.
            let skull = block.ends_with("_head") || block.ends_with("_skull");
            let segment = if skull { ctx.segment(-180.0) } else { ctx.segment(0.0) };
            state = kinds.with_property(state, "rotation", ROTATIONS[segment])?;
        }
        Some(self.waterlog(state, pos))
    }

    /// Wall torches, signs and banners: the first horizontal direction in look order that has
    /// something to hold on to.
    fn wall_state(
        &self,
        wall: &str,
        pos: BlockPos,
        ctx: &PlaceContext,
        replacing: bool,
    ) -> Option<u16> {
        let kinds = Kinds::get();
        for dir in ctx.ordered(replacing) {
            if matches!(dir, Direction::Up | Direction::Down) {
                continue;
            }
            let state = kinds.with_property(
                block_by_name(wall)?.default_state,
                "facing",
                dir.opposite().name(),
            )?;
            let state = self.waterlog(state, pos);
            if self.can_survive(state, pos) {
                return Some(state);
            }
        }
        None
    }

    fn wall_skull_state(
        &self,
        wall: &str,
        pos: BlockPos,
        ctx: &PlaceContext,
        replacing: bool,
    ) -> Option<u16> {
        let kinds = Kinds::get();
        for dir in ctx.ordered(replacing) {
            if matches!(dir, Direction::Up | Direction::Down) {
                continue;
            }
            if !self.is_replaceable_at(pos.offset(dir)) {
                let state = kinds.with_property(
                    block_by_name(wall)?.default_state,
                    "facing",
                    dir.opposite().name(),
                )?;
                return Some(self.waterlog(state, pos));
            }
        }
        None
    }

    fn wall_hanging_sign_state(
        &self,
        wall: &str,
        pos: BlockPos,
        ctx: &PlaceContext,
        replacing: bool,
    ) -> Option<u16> {
        let kinds = Kinds::get();
        for dir in ctx.ordered(replacing) {
            if matches!(dir, Direction::Up | Direction::Down)
                || axis_name(dir) == axis_name(ctx.face)
            {
                continue;
            }
            let facing = dir.opposite();
            let state = self.waterlog(
                kinds.with_property(block_by_name(wall)?.default_state, "facing", facing.name())?,
                pos,
            );
            if self.wall_hanging_sign_attached(facing, pos) {
                return Some(state);
            }
        }
        None
    }

    /// A wall hanging sign hangs between two blocks beside it: either side must hold it.
    fn wall_hanging_sign_attached(&self, facing: Direction, pos: BlockPos) -> bool {
        let kinds = Kinds::get();
        [facing.clockwise(), facing.counter_clockwise()].into_iter().any(|side| {
            let at = pos.offset(side);
            self.block(at).is_some_and(|s| {
                if block_of_state(s).is_some_and(|b| b.name.ends_with("wall_hanging_sign")) {
                    kinds
                        .property(s, "facing")
                        .and_then(Direction::from_name)
                        .is_some_and(|f| axis_name(f) == axis_name(facing))
                } else {
                    facts(s).is_face_sturdy(
                        side.opposite().index(),
                        ferrite_data::shapes::Support::Full,
                    )
                }
            })
        })
    }

    /// Pointed dripstone points along the vertical direction the player looks against, or the
    /// other way if that has nothing to grow from; its thickness follows its neighbours.
    fn spike_state(&self, block: &str, pos: BlockPos, ctx: &PlaceContext) -> Option<u16> {
        let kinds = Kinds::get();
        let base = self.make(block, &[])?;
        let look = looking_directions(ctx.yaw, ctx.pitch)
            .into_iter()
            .find(|d| matches!(d, Direction::Up | Direction::Down))?
            .opposite();
        let tip =
            [look, look.opposite()].into_iter().find(|d| self.spike_base_holds(base, pos, *d))?;
        let pointing = |at: BlockPos, dir: Direction| {
            self.block(at).is_some_and(|s| self.is_spike_pointing(base, s, dir))
        };
        let in_front = pos.offset(tip);
        let behind = pos.offset(tip.opposite());
        let merge_tip = !ctx.sneaking;
        let thickness = if pointing(in_front, tip.opposite()) {
            let front = self.block(in_front)?;
            if merge_tip || kinds.property(front, "thickness") == Some("tip_merge") {
                "tip_merge"
            } else {
                "tip"
            }
        } else if !pointing(in_front, tip) {
            "tip"
        } else {
            let front = self.block(in_front)?;
            match kinds.property(front, "thickness") {
                Some("tip" | "tip_merge") => {
                    if pointing(behind, tip) {
                        "frustum"
                    } else {
                        "base"
                    }
                }
                _ => {
                    if pointing(behind, tip) {
                        "middle"
                    } else {
                        "base"
                    }
                }
            }
        };
        let state = kinds.with_property(base, "vertical_direction", tip.name())?;
        let state = kinds.with_property(state, "thickness", thickness)?;
        Some(self.waterlog(state, pos))
    }

    fn bell_state(&self, pos: BlockPos, ctx: &PlaceContext) -> Option<u16> {
        let base = self.make("bell", &[])?;
        let kinds = Kinds::get();
        let state = if matches!(ctx.face, Direction::Up | Direction::Down) {
            let attachment = if ctx.face == Direction::Down { "ceiling" } else { "floor" };
            let s = kinds.with_property(base, "attachment", attachment)?;
            kinds.with_property(s, "facing", ctx.horizontal().name())?
        } else {
            // Held on both sides by blocks in front and behind, or by one.
            let full = |at: BlockPos, face: Direction| {
                self.block(at).is_some_and(|s| {
                    crate::shape::merged_face_occludes(facts(s).collision_shape(), &[], face)
                })
            };
            let behind = pos.offset(ctx.face.opposite());
            let ahead = pos.offset(ctx.face);
            let both = full(behind, ctx.face) && full(ahead, ctx.face.opposite());
            let s = kinds.with_property(
                base,
                "attachment",
                if both { "double_wall" } else { "single_wall" },
            )?;
            kinds.with_property(s, "facing", ctx.face.opposite().name())?
        };
        self.can_survive(state, pos).then_some(state)
    }

    fn multiface_state(
        &self,
        block: &str,
        pos: BlockPos,
        ctx: &PlaceContext,
        replacing: bool,
    ) -> Option<u16> {
        let kinds = Kinds::get();
        for dir in ctx.ordered(replacing) {
            let at = pos.offset(dir);
            let holds = self.block(at).is_some_and(|s| {
                facts(s).is_face_sturdy(dir.opposite().index(), ferrite_data::shapes::Support::Full)
                    || crate::shape::merged_face_occludes(
                        facts(s).collision_shape(),
                        &[],
                        dir.opposite(),
                    )
            });
            if holds {
                let state = self.waterlog(self.make(block, &[])?, pos);
                return kinds.with_property(state, dir.name(), "true");
            }
        }
        None
    }

    fn vine_state(&self, pos: BlockPos, ctx: &PlaceContext, replacing: bool) -> Option<u16> {
        let kinds = Kinds::get();
        let vine = block_by_name("vine")?.id;
        for dir in ctx.ordered(replacing) {
            if dir == Direction::Down {
                continue;
            }
            let at = pos.offset(dir);
            // Vines hold on to sturdy faces and to full cubes that are not sturdy (leaves).
            let holds = self.block(at).is_some_and(|s| {
                facts(s).is_face_sturdy(dir.opposite().index(), ferrite_data::shapes::Support::Full)
                    || facts(s).has(flag::FULL_COLLISION_BLOCK)
            });
            let hangs = dir != Direction::Up
                && self.block(pos.above()).is_some_and(|s| {
                    crate::kinds::block_id_of(s) == vine
                        && kinds.property(s, dir.name()) == Some("true")
                });
            if holds || hangs {
                return kinds.with_property(self.make("vine", &[])?, dir.name(), "true");
            }
        }
        None
    }

    /// Bamboo planted on ground starts as a sapling and grows on existing bamboo.
    fn bamboo_state(&self, pos: BlockPos) -> Option<u16> {
        if self.block(pos).is_some_and(|s| !FluidState::of(s).is_empty()) {
            return None;
        }
        let below = self.block(pos.below())?;
        let id = crate::kinds::block_id_of(below);
        if !crate::kinds::block_tag("minecraft:supports_bamboo").contains(&id) {
            return None;
        }
        if id == block_by_name("bamboo_sapling")?.id || id == block_by_name("bamboo")?.id {
            self.make("bamboo", &[])
        } else {
            self.make("bamboo_sapling", &[])
        }
    }

    fn face_attached(
        &self,
        block: &str,
        pos: BlockPos,
        ctx: &PlaceContext,
        replacing: bool,
    ) -> Option<u16> {
        let kinds = Kinds::get();
        let base = block_by_name(block)?.default_state;
        for dir in ctx.ordered(replacing) {
            let state = if matches!(dir, Direction::Up | Direction::Down) {
                let face = if dir == Direction::Up { "ceiling" } else { "floor" };
                let s = kinds.with_property(base, "face", face)?;
                kinds.with_property(s, "facing", ctx.horizontal().name())?
            } else {
                let s = kinds.with_property(base, "face", "wall")?;
                kinds.with_property(s, "facing", dir.opposite().name())?
            };
            // Grindstones hang in mid-air; everything else needs something to stick to.
            if block == "grindstone" || self.can_survive(state, pos) {
                return Some(state);
            }
        }
        None
    }

    fn trapdoor_state(
        &self,
        block: &str,
        pos: BlockPos,
        ctx: &PlaceContext,
        replacing: bool,
    ) -> Option<u16> {
        let kinds = Kinds::get();
        let mut state = block_by_name(block)?.default_state;
        if !replacing && !matches!(ctx.face, Direction::Up | Direction::Down) {
            state = kinds.with_property(state, "facing", ctx.face.name())?;
            let half = if ctx.cursor[1] > 0.5 { "top" } else { "bottom" };
            state = kinds.with_property(state, "half", half)?;
        } else {
            state = kinds.with_property(state, "facing", ctx.horizontal().opposite().name())?;
            let half = if ctx.face == Direction::Up { "bottom" } else { "top" };
            state = kinds.with_property(state, "half", half)?;
        }
        if crate::redstone::has_neighbor_signal(self, pos) {
            state = kinds.with_property(state, "open", "true")?;
            state = kinds.with_property(state, "powered", "true")?;
        }
        Some(self.waterlog(state, pos))
    }

    fn door_state(&self, block: &str, pos: BlockPos, ctx: &PlaceContext) -> Option<Placed> {
        let kinds = Kinds::get();
        let above = pos.above();
        if !self.is_replaceable_at(above) {
            return None;
        }
        let dir = ctx.horizontal();
        let full =
            |at: BlockPos| self.block(at).is_some_and(|s| facts(s).has(flag::FULL_COLLISION_BLOCK));
        let is_door = |at: BlockPos| {
            self.block(at).is_some_and(|s| {
                block_of_state(s).is_some_and(|b| b.name.trim_start_matches("minecraft:") == block)
                    && kinds.property(s, "half") == Some("lower")
            })
        };
        let (ccw, cw) = (dir.counter_clockwise(), dir.clockwise());
        let (left, right) = (pos.offset(ccw), pos.offset(cw));
        let sides = (if full(left) { -1 } else { 0 })
            + (if full(above.offset(ccw)) { -1 } else { 0 })
            + (if full(right) { 1 } else { 0 })
            + (if full(above.offset(cw)) { 1 } else { 0 });
        let (left_door, right_door) = (is_door(left), is_door(right));
        let hinge = if (left_door && !right_door) || sides > 0 {
            "right"
        } else if (right_door && !left_door) || sides < 0 {
            "left"
        } else {
            let (sx, _, sz) = dir.step();
            let dx = ctx.cursor[0] - ctx.face.step().0 as f32;
            let dz = ctx.cursor[2] - ctx.face.step().2 as f32;
            if (sx < 0 && dz < 0.5)
                || (sx > 0 && dz > 0.5)
                || (sz < 0 && dx > 0.5)
                || (sz > 0 && dx < 0.5)
            {
                "right"
            } else {
                "left"
            }
        };
        let powered = crate::redstone::has_neighbor_signal(self, pos)
            || crate::redstone::has_neighbor_signal(self, above);
        let flag = if powered { "true" } else { "false" };
        let props = [("facing", dir.name()), ("hinge", hinge), ("open", flag), ("powered", flag)];
        let lower = self.make(block, &[&props[..], &[("half", "lower")]].concat())?;
        let upper = self.make(block, &[&props[..], &[("half", "upper")]].concat())?;
        Some(Placed { state: lower, second: Some((above, upper)) })
    }

    /// Whether `state` can stand at `pos` given what is around it.
    pub fn can_survive(&self, state: u16, pos: BlockPos) -> bool {
        Direction::ALL.into_iter().all(|d| self.survives(state, d, pos))
    }

    /// Places the block called `block` (an item's block, e.g. `oak_stairs`) the way a player's
    /// click does. `obstructed` says whether the new state at the position would overlap an
    /// entity. Returns where it was placed.
    pub fn place_block(
        &mut self,
        block: &str,
        ctx: &PlaceContext,
        obstructed: impl Fn(u16, BlockPos) -> bool,
        random_below: &mut dyn FnMut(u32) -> u32,
    ) -> Option<BlockPos> {
        let clicked_state = self.block(ctx.clicked)?;
        // Clicking something replaceable places into it, otherwise next to it.
        let replacing = facts(clicked_state).has(flag::REPLACEABLE);
        let pos = if replacing { ctx.clicked } else { ctx.clicked.offset(ctx.face) };
        if !self.is_replaceable_at(pos) {
            return None;
        }
        let placed = self.state_for_placement(block, pos, ctx, replacing)?;
        let mut state = placed.state;
        if matches!(block, "kelp" | "twisting_vines" | "weeping_vines") {
            // The growing tips start at a random age.
            let age = random_below(25).to_string();
            state = Kinds::get().with_property(state, "age", &age).unwrap_or(state);
        }
        if !self.can_survive(state, pos) || obstructed(state, pos) {
            return None;
        }
        let adapted = self.update_from_neighbor_shapes(state, pos);
        let first = if facts(adapted).has(flag::AIR) { state } else { adapted };
        self.set_block(pos, first, update::ALL);
        if let Some((at, second)) = placed.second {
            self.set_block(at, second, update::ALL);
        }
        Some(pos)
    }
}
