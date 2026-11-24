//! Moving a box through the blocks of the world (vanilla's `Entity.collide`).

use ferrite_data::shapes::facts;

use crate::level::BlockAccess;
use crate::pos::BlockPos;
use crate::world::World;

/// An axis-aligned box in world coordinates: `[min_x, min_y, min_z, max_x, max_y, max_z]`.
pub type Aabb = [f64; 6];

/// The outcome of a move.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moved {
    /// How far the box actually moved.
    pub delta: [f64; 3],
    /// Whether movement was cut short along each axis.
    pub blocked: [bool; 3],
}

const EPS: f64 = 1.0e-7;

impl<A: BlockAccess> World<A> {
    /// Every collision box of the blocks that could touch `region`, in world coordinates.
    fn collision_boxes(&self, region: &Aabb) -> Vec<Aabb> {
        let mut out = Vec::new();
        let lo = (
            region[0].floor() as i32 - 1,
            region[1].floor() as i32 - 1,
            region[2].floor() as i32 - 1,
        );
        let hi =
            (region[3].ceil() as i32 + 1, region[4].ceil() as i32 + 1, region[5].ceil() as i32 + 1);
        for y in lo.1..=hi.1 {
            for z in lo.2..=hi.2 {
                for x in lo.0..=hi.0 {
                    let Some(state) = self.block(BlockPos::new(x, y, z)) else {
                        // Unloaded chunks are solid so that nothing falls out of the world.
                        out.push([
                            f64::from(x),
                            f64::from(y),
                            f64::from(z),
                            f64::from(x + 1),
                            f64::from(y + 1),
                            f64::from(z + 1),
                        ]);
                        continue;
                    };
                    for b in facts(state).collision_shape() {
                        out.push([
                            f64::from(x) + b[0],
                            f64::from(y) + b[1],
                            f64::from(z) + b[2],
                            f64::from(x) + b[3],
                            f64::from(y) + b[4],
                            f64::from(z) + b[5],
                        ]);
                    }
                }
            }
        }
        out
    }

    /// `move_box`, plus walking up steps of at most `step_height` (vanilla's `Entity.collide`):
    /// when a horizontal move is blocked and the entity stands on the ground (or lands), it tries
    /// the same move raised by the step and keeps that if it gets further.
    pub fn move_box_stepping(
        &self,
        bbox: &Aabb,
        delta: [f64; 3],
        step_height: f64,
        on_ground: bool,
    ) -> Moved {
        let base = self.move_box(bbox, delta);
        let horizontal_blocked = base.blocked[0] || base.blocked[2];
        let lands = on_ground || (base.blocked[1] && delta[1] < 0.0);
        if step_height <= 0.0 || !lands || !horizontal_blocked {
            return base;
        }
        let raised = self.move_box(bbox, [delta[0], step_height, delta[2]]);
        let mut best = raised.delta;
        let up = self.move_box(bbox, [0.0, step_height, 0.0]).delta[1];
        if up < step_height {
            let lifted = [bbox[0], bbox[1] + up, bbox[2], bbox[3], bbox[4] + up, bbox[5]];
            let h = self.move_box(&lifted, [delta[0], 0.0, delta[2]]).delta;
            let alt = [h[0], up, h[2]];
            if alt[0] * alt[0] + alt[2] * alt[2] > best[0] * best[0] + best[2] * best[2] {
                best = alt;
            }
        }
        let base_h = base.delta[0] * base.delta[0] + base.delta[2] * base.delta[2];
        if best[0] * best[0] + best[2] * best[2] <= base_h {
            return base;
        }
        // Come back down to where the original vertical move would have ended.
        let stepped = [
            bbox[0] + best[0],
            bbox[1] + best[1],
            bbox[2] + best[2],
            bbox[3] + best[0],
            bbox[4] + best[1],
            bbox[5] + best[2],
        ];
        let down = self.move_box(&stepped, [0.0, -best[1] + delta[1], 0.0]);
        let out = [best[0], best[1] + down.delta[1], best[2]];
        Moved {
            delta: out,
            blocked: [
                (out[0] - delta[0]).abs() > EPS,
                (out[1] - delta[1]).abs() > EPS,
                (out[2] - delta[2]).abs() > EPS,
            ],
        }
    }

    /// Moves `bbox` by `delta` until it hits something: vertical first, then the longer of the
    /// two horizontal axes, as the game does.
    pub fn move_box(&self, bbox: &Aabb, delta: [f64; 3]) -> Moved {
        let region = [
            bbox[0].min(bbox[0] + delta[0]),
            bbox[1].min(bbox[1] + delta[1]),
            bbox[2].min(bbox[2] + delta[2]),
            bbox[3].max(bbox[3] + delta[0]),
            bbox[4].max(bbox[4] + delta[1]),
            bbox[5].max(bbox[5] + delta[2]),
        ];
        let boxes = self.collision_boxes(&region);
        let mut cur = *bbox;
        let mut out = [0.0; 3];
        let order: [usize; 3] = if delta[0].abs() < delta[2].abs() { [1, 2, 0] } else { [1, 0, 2] };
        for axis in order {
            let d = clip(&cur, &boxes, axis, delta[axis]);
            cur[axis] += d;
            cur[axis + 3] += d;
            out[axis] = d;
        }
        let blocked = [0, 1, 2].map(|a| (out[a] - delta[a]).abs() > EPS);
        Moved { delta: out, blocked }
    }
}

/// How far `bbox` can move along `axis` by at most `d` before touching one of `boxes`.
fn clip(bbox: &Aabb, boxes: &[Aabb], axis: usize, mut d: f64) -> f64 {
    if d == 0.0 {
        return 0.0;
    }
    let (a, b) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    for o in boxes {
        // Only boxes overlapping on the other two axes can get in the way.
        if bbox[a + 3] <= o[a] + EPS
            || bbox[a] >= o[a + 3] - EPS
            || bbox[b + 3] <= o[b] + EPS
            || bbox[b] >= o[b + 3] - EPS
        {
            continue;
        }
        if d > 0.0 && o[axis] >= bbox[axis + 3] - EPS {
            d = d.min(o[axis] - bbox[axis + 3]);
        } else if d < 0.0 && o[axis + 3] <= bbox[axis] + EPS {
            d = d.max(o[axis + 3] - bbox[axis]);
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use ferrite_data::blocks::block_by_name;
    use ferrite_world::{Chunk, ChunkPos, Dimension};

    use super::*;
    use crate::level::Chunks;
    use crate::world::Rules;

    fn world_with_floor() -> World<Chunks> {
        let air = block_by_name("air").unwrap().default_state;
        let mut chunks = Chunks::new(Dimension::OVERWORLD);
        for cx in -1..=1 {
            for cz in -1..=1 {
                chunks.insert(ChunkPos::new(cx, cz), Chunk::filled(Dimension::OVERWORLD, air, 0));
            }
        }
        let mut w = World::new(chunks, Rules::default());
        let stone = block_by_name("stone").unwrap().default_state;
        for x in 0..4 {
            for z in 0..4 {
                w.blocks.set_block_raw(BlockPos::new(x, 63, z), stone);
            }
        }
        w
    }

    #[test]
    fn a_falling_box_lands_on_the_floor() {
        let w = world_with_floor();
        let item = [1.0, 66.0, 1.0, 1.25, 66.25, 1.25];
        let m = w.move_box(&item, [0.0, -5.0, 0.0]);
        assert!((m.delta[1] - -2.0).abs() < 1e-6, "{m:?}");
        assert!(m.blocked[1] && !m.blocked[0]);
    }

    #[test]
    fn walls_stop_sideways_movement() {
        let mut w = world_with_floor();
        let stone = block_by_name("stone").unwrap().default_state;
        w.blocks.set_block_raw(BlockPos::new(3, 64, 1), stone);
        let item = [1.0, 64.0, 1.0, 1.25, 64.25, 1.25];
        let m = w.move_box(&item, [4.0, 0.0, 0.0]);
        assert!((m.delta[0] - 1.75).abs() < 1e-6, "{m:?}");
        assert!(m.blocked[0]);
    }
}
