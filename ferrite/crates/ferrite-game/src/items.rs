//! Item entities (dropped stacks) and the player inventory rules that pick them up.

use ferrite_blocks::collision::Aabb;
use ferrite_blocks::{BlockPos, Chunks, World};
use ferrite_data::items::item_info;
use ferrite_protocol::packets::play::ItemStack;
use uuid::Uuid;

/// Ticks until a dropped item despawns (five minutes).
pub const LIFESPAN: u32 = 6000;
/// Ticks before a freshly dropped item can be picked up.
pub const DEFAULT_PICKUP_DELAY: u32 = 10;

#[derive(Clone, Debug)]
pub struct ItemEntity {
    pub id: i32,
    pub uuid: Uuid,
    pub pos: [f64; 3],
    pub vel: [f64; 3],
    pub stack: ItemStack,
    pub age: u32,
    pub pickup_delay: u32,
    pub on_ground: bool,
    /// Position in 1/4096 blocks as other players last saw it.
    pub sent: [i64; 3],
    /// Whether the velocity changed in a way clients must be told about.
    pub dirty_velocity: bool,
}

impl ItemEntity {
    pub fn new(id: i32, uuid: Uuid, pos: [f64; 3], vel: [f64; 3], stack: ItemStack) -> Self {
        let sent = pos.map(|v| (v * 4096.0).floor() as i64);
        ItemEntity {
            id,
            uuid,
            pos,
            vel,
            stack,
            age: 0,
            pickup_delay: DEFAULT_PICKUP_DELAY,
            on_ground: false,
            sent,
            dirty_velocity: false,
        }
    }

    pub fn bbox(&self) -> Aabb {
        let [x, y, z] = self.pos;
        [x - 0.125, y, z - 0.125, x + 0.125, y + 0.25, z + 0.125]
    }

    /// One tick of item physics (`ItemEntity.tick`): gravity, movement with block collisions,
    /// then ground friction. Returns false when the item has expired.
    pub fn tick(&mut self, world: &World<Chunks>) -> bool {
        if self.stack.count <= 0 {
            return false;
        }
        if self.pickup_delay > 0 && self.pickup_delay != 32767 {
            self.pickup_delay -= 1;
        }
        self.vel[1] -= 0.04;
        let moving = !self.on_ground
            || self.vel[0] * self.vel[0] + self.vel[2] * self.vel[2] > 1.0e-5
            || (self.age as i64 + i64::from(self.id)) % 4 == 0;
        if moving {
            let moved = world.move_box(&self.bbox(), self.vel);
            for i in 0..3 {
                self.pos[i] += moved.delta[i];
                if moved.blocked[i] {
                    self.vel[i] = 0.0;
                }
            }
            self.on_ground = moved.blocked[1] && self.vel_before_y_negative(moved.delta[1]);
            let mut friction = 0.98;
            if self.on_ground {
                let below = BlockPos::new(
                    self.pos[0].floor() as i32,
                    (self.pos[1] - 0.500001).floor() as i32,
                    self.pos[2].floor() as i32,
                );
                friction = block_friction(world.block(below)) * 0.98;
            }
            self.vel[0] *= friction;
            self.vel[1] *= 0.98;
            self.vel[2] *= friction;
            if self.on_ground && self.vel[1] < 0.0 {
                self.vel[1] *= -0.5;
            }
        }
        self.age += 1;
        self.age < LIFESPAN
    }

    /// Landing means the downward move was cut short.
    fn vel_before_y_negative(&self, applied: f64) -> bool {
        // The move was blocked on the y axis; it counts as ground only if it was heading down.
        applied <= 0.0
    }
}

fn block_friction(state: Option<u16>) -> f64 {
    let Some(state) = state else { return 0.6 };
    match ferrite_data::blocks::block_of_state(state).map(|b| b.name) {
        Some("minecraft:ice" | "minecraft:frosted_ice") => 0.98,
        Some("minecraft:packed_ice" | "minecraft:blue_ice") => 0.989,
        Some("minecraft:slime_block") => 0.8,
        _ => 0.6,
    }
}

pub fn max_stack(item: i32) -> i32 {
    item_info(item).map_or(64, |i| i32::from(i.max_stack))
}

/// Container slot of an internal inventory index (hotbar `0..9`, main `9..36`).
pub fn container_slot(internal: usize) -> usize {
    if internal < 9 { 36 + internal } else { internal }
}

/// Adds `stack` to the player inventory the way the game does: the held slot and the off hand
/// first if they can take more, then any partial stack, then the first free slot (hotbar before
/// main inventory). Returns the container slots that changed; `stack.count` is what is left.
pub fn add_to_inventory(
    inventory: &mut [ItemStack],
    held: usize,
    stack: &mut ItemStack,
) -> Vec<usize> {
    let mut changed = Vec::new();
    let room = |inv: &[ItemStack], slot: usize, s: &ItemStack| {
        let cur = inv[slot];
        cur.count > 0 && cur.item == s.item && cur.count < max_stack(s.item)
    };
    while stack.count > 0 {
        let slot = if room(inventory, 36 + held, stack) {
            Some(36 + held)
        } else if room(inventory, 45, stack) {
            Some(45)
        } else {
            (0..36).map(container_slot).find(|s| room(inventory, *s, stack))
        };
        let slot = match slot {
            Some(s) => s,
            None => match (0..36).map(container_slot).find(|s| inventory[*s].count <= 0) {
                Some(s) => {
                    inventory[s] = ItemStack { count: 0, item: stack.item };
                    s
                }
                None => break,
            },
        };
        let cur = &mut inventory[slot];
        let space = max_stack(stack.item) - cur.count;
        let moved = stack.count.min(space);
        cur.count += moved;
        cur.item = stack.item;
        stack.count -= moved;
        if !changed.contains(&slot) {
            changed.push(slot);
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(item: i32, count: i32) -> ItemStack {
        ItemStack { count, item }
    }

    #[test]
    fn items_go_to_the_hotbar_first_and_fill_partial_stacks() {
        let mut inv = [ItemStack::default(); 46];
        let mut s = stack(1, 5);
        assert_eq!(add_to_inventory(&mut inv, 0, &mut s), vec![36]);
        assert_eq!((s.count, inv[36].count), (0, 5));
        let mut more = stack(1, 3);
        assert_eq!(add_to_inventory(&mut inv, 0, &mut more), vec![36]);
        assert_eq!(inv[36].count, 8);
        let mut other = stack(2, 1);
        assert_eq!(add_to_inventory(&mut inv, 0, &mut other), vec![37]);
    }

    #[test]
    fn overflow_spills_into_the_next_slot_and_a_full_inventory_keeps_the_rest() {
        let mut inv = [ItemStack::default(); 46];
        let mut big = stack(1, 70);
        let changed = add_to_inventory(&mut inv, 0, &mut big);
        assert_eq!(changed, vec![36, 37]);
        assert_eq!((inv[36].count, inv[37].count), (64, 6));
        let mut full = [stack(9, 64); 46];
        let mut s = stack(1, 4);
        assert!(add_to_inventory(&mut full, 0, &mut s).is_empty());
        assert_eq!(s.count, 4);
    }

    #[test]
    fn dropped_items_fall_and_come_to_rest() {
        use ferrite_blocks::{BlockAccess, Rules};
        use ferrite_data::blocks::block_by_name;
        use ferrite_world::{Chunk, ChunkPos, Dimension};
        let air = block_by_name("air").unwrap().default_state;
        let mut chunks = Chunks::new(Dimension::OVERWORLD);
        chunks.insert(ChunkPos::new(0, 0), Chunk::filled(Dimension::OVERWORLD, air, 0));
        let mut world = World::new(chunks, Rules::default());
        let stone = block_by_name("stone").unwrap().default_state;
        for x in 0..16 {
            for z in 0..16 {
                world.blocks.set_block_raw(BlockPos::new(x, 63, z), stone);
            }
        }
        let mut item = ItemEntity::new(1, Uuid::nil(), [8.5, 70.0, 8.5], [0.0; 3], stack(1, 1));
        for _ in 0..100 {
            assert!(item.tick(&world));
        }
        assert!(item.on_ground);
        assert!((item.pos[1] - 64.0).abs() < 1e-3, "{:?}", item.pos);
    }
}
