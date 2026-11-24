//! Walls: arms that are low or tall depending on what is above them, and a post that is only
//! there when the shape needs one.

use ferrite_data::shapes::{Box6, Support as FaceSupport, facts};

use crate::kinds::{Kinds, block_tag};
use crate::level::BlockAccess;
use crate::pos::{BlockPos, Direction};
use crate::world::World;

const EPS: f64 = 1.0e-7;

/// The cross-section (x, z) a block presents downwards: the boxes at the lowest layer of its
/// collision shape, as `[min_x, min_z, max_x, max_z]`.
fn lowest_layer(shape: &[Box6]) -> Vec<[f64; 4]> {
    let Some(floor) = shape.iter().map(|b| b[1]).min_by(f64::total_cmp) else {
        return Vec::new();
    };
    shape.iter().filter(|b| (b[1] - floor).abs() < EPS).map(|b| [b[0], b[2], b[3], b[5]]).collect()
}

/// Whether the union of `rects` covers `target` (`[min_x, min_z, max_x, max_z]`).
fn covers(rects: &[[f64; 4]], target: [f64; 4]) -> bool {
    let mut xs = vec![target[0], target[2]];
    let mut zs = vec![target[1], target[3]];
    for r in rects {
        for x in [r[0], r[2]] {
            if x > target[0] && x < target[2] {
                xs.push(x);
            }
        }
        for z in [r[1], r[3]] {
            if z > target[1] && z < target[3] {
                zs.push(z);
            }
        }
    }
    xs.sort_by(f64::total_cmp);
    zs.sort_by(f64::total_cmp);
    for wx in xs.windows(2) {
        if wx[1] - wx[0] < EPS {
            continue;
        }
        let mx = (wx[0] + wx[1]) / 2.0;
        for wz in zs.windows(2) {
            if wz[1] - wz[0] < EPS {
                continue;
            }
            let mz = (wz[0] + wz[1]) / 2.0;
            if !rects.iter().any(|r| r[0] < mx && mx < r[2] && r[1] < mz && mz < r[3]) {
                return false;
            }
        }
    }
    true
}

/// The region an arm in `dir` occupies, which the block above must cover for the arm to be tall.
fn arm_test(dir: Direction) -> [f64; 4] {
    match dir {
        Direction::North => [7.0 / 16.0, 0.0, 9.0 / 16.0, 9.0 / 16.0],
        Direction::South => [7.0 / 16.0, 7.0 / 16.0, 9.0 / 16.0, 1.0],
        Direction::East => [7.0 / 16.0, 7.0 / 16.0, 1.0, 9.0 / 16.0],
        _ => [0.0, 7.0 / 16.0, 9.0 / 16.0, 9.0 / 16.0],
    }
}

const POST_TEST: [f64; 4] = [7.0 / 16.0, 7.0 / 16.0, 9.0 / 16.0, 9.0 / 16.0];

impl<A: BlockAccess> World<A> {
    /// `WallBlock.connectsTo`: whether a wall joins the block `neighbor` on its side facing
    /// `toward_wall`.
    pub(crate) fn wall_connects(&self, neighbor: u16, toward_wall: Direction) -> bool {
        let kinds = Kinds::get();
        let other = kinds.of_state(neighbor);
        let sturdy = facts(neighbor).is_face_sturdy(toward_wall.index(), FaceSupport::Full);
        let gate = other.is_fence_gate
            && kinds
                .property(neighbor, "facing")
                .and_then(Direction::from_name)
                .is_some_and(|f| f.is_x_axis() != toward_wall.is_x_axis());
        other.is_wall
            || (!other.connection_exception && sturdy)
            || other.connects == Some(crate::kinds::Connects::Pane)
            || gate
    }

    /// Recomputes a wall after its neighbour in `dir` became `neighbor_state`.
    pub(crate) fn wall_update_shape(
        &self,
        state: u16,
        dir: Direction,
        neighbor_state: u16,
        pos: BlockPos,
    ) -> u16 {
        let kinds = Kinds::get();
        if dir == Direction::Down {
            return state;
        }
        let arm = |d: Direction| kinds.property(state, d.name()).is_some_and(|v| v != "none");
        let mut connected =
            [Direction::North, Direction::East, Direction::South, Direction::West].map(|d| {
                if dir == d { self.wall_connects(neighbor_state, d.opposite()) } else { arm(d) }
            });
        if dir == Direction::Up {
            connected =
                [Direction::North, Direction::East, Direction::South, Direction::West].map(arm);
        }
        let above_state = if dir == Direction::Up {
            neighbor_state
        } else {
            self.block(pos.above()).unwrap_or(kinds.air)
        };
        self.wall_shape(state, above_state, connected)
    }

    /// `WallBlock.updateShape(level, state, above, aboveState, n, e, s, w)`.
    pub(crate) fn wall_shape(&self, state: u16, above_state: u16, connected: [bool; 4]) -> u16 {
        let kinds = Kinds::get();
        let above_rects = lowest_layer(facts(above_state).collision_shape());
        let dirs = [Direction::North, Direction::East, Direction::South, Direction::West];
        let mut s = state;
        let mut sides = [0u8; 4]; // 0 none, 1 low, 2 tall
        for (i, d) in dirs.into_iter().enumerate() {
            let value = if !connected[i] {
                "none"
            } else if covers(&above_rects, arm_test(d)) {
                "tall"
            } else {
                "low"
            };
            sides[i] = match value {
                "none" => 0,
                "low" => 1,
                _ => 2,
            };
            s = kinds.with_property(s, d.name(), value).unwrap_or(s);
        }
        let raise = self.should_raise_post(above_state, &above_rects, sides);
        kinds.with_property(s, "up", if raise { "true" } else { "false" }).unwrap_or(s)
    }

    fn should_raise_post(
        &self,
        above_state: u16,
        above_rects: &[[f64; 4]],
        sides: [u8; 4],
    ) -> bool {
        let kinds = Kinds::get();
        let [n, e, s, w] = sides;
        if kinds.of_state(above_state).is_wall && kinds.property(above_state, "up") == Some("true")
        {
            return true;
        }
        let (n_none, e_none, s_none, w_none) = (n == 0, e == 0, s == 0, w == 0);
        if (n_none && s_none && w_none && e_none) || n_none != s_none || w_none != e_none {
            return true;
        }
        if (n == 2 && s == 2) || (e == 2 && w == 2) {
            return false;
        }
        block_tag("minecraft:wall_post_override").contains(&crate::kinds::block_id_of(above_state))
            || covers(above_rects, POST_TEST)
    }
}
