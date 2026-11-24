//! Stair corners: a stair turns into an inner or outer corner next to a stair across it.

use crate::kinds::{Kinds, block_id_of, block_tag};
use crate::level::BlockAccess;
use crate::pos::{BlockPos, Direction};
use crate::world::World;

fn is_stairs(state: u16) -> bool {
    block_tag("minecraft:stairs").contains(&block_id_of(state))
}

impl<A: BlockAccess> World<A> {
    pub(crate) fn is_stair_state(state: u16) -> bool {
        is_stairs(state)
    }

    /// The state of the stair `state` at `pos` with its `shape` set from the stairs around it.
    pub(crate) fn stairs_shape(&self, state: u16, pos: BlockPos) -> u16 {
        let kinds = Kinds::get();
        let facing = kinds.property(state, "facing").and_then(Direction::from_name);
        let Some(facing) = facing else { return state };
        let half = kinds.property(state, "half");
        let facing_of = |s: u16| kinds.property(s, "facing").and_then(Direction::from_name);
        let same_half = |s: u16| is_stairs(s) && kinds.property(s, "half") == half;
        // A corner only forms if the stair beside it would not already own that side.
        let can_take = |dir: Direction| {
            self.block(pos.offset(dir)).is_none_or(|s| {
                !is_stairs(s) || facing_of(s) != Some(facing) || kinds.property(s, "half") != half
            })
        };
        let left = facing.counter_clockwise();
        let shape = 'shape: {
            if let Some(front) = self.block(pos.offset(facing))
                && same_half(front)
                && let Some(f) = facing_of(front)
                && f.is_x_axis() != facing.is_x_axis()
                && can_take(f.opposite())
            {
                break 'shape if f == left { "outer_left" } else { "outer_right" };
            }
            if let Some(back) = self.block(pos.offset(facing.opposite()))
                && same_half(back)
                && let Some(f) = facing_of(back)
                && f.is_x_axis() != facing.is_x_axis()
                && can_take(f)
            {
                break 'shape if f == left { "inner_left" } else { "inner_right" };
            }
            "straight"
        };
        kinds.with_property(state, "shape", shape).unwrap_or(state)
    }
}
