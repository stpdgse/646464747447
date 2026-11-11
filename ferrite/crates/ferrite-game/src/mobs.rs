//! Mobs: physics, goals and pathfinding for creatures that walk about the world.
//!
//! A mob has a hitbox, health and a movement speed from the game's registries. Each tick its
//! goals decide where it wants to go (wander, flee after being hurt, look at a nearby player),
//! an A* search over the block grid turns that into a path, and the same movement maths as
//! vanilla's `LivingEntity.travel` moves it: acceleration scaled by the floor's friction, gravity,
//! drag, steps up to 0.6 blocks, jumps for taller ones.

use ferrite_blocks::collision::Aabb;
use ferrite_blocks::shape::presents_face;
use ferrite_blocks::{BlockPos, Chunks, Direction, World};
use ferrite_data::entities::EntityInfo;
use ferrite_data::shapes::{facts, flag};
use uuid::Uuid;

/// A small random source for AI decisions (xorshift64*).
#[derive(Clone, Debug)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0 | 1;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: u32) -> u32 {
        ((self.next_u64() >> 33) % u64::from(n.max(1))) as u32
    }

    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn between(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }
}

/// Whether the block cell can be walked through (no collision, not liquid).
fn passable(world: &World<Chunks>, pos: BlockPos) -> bool {
    world.block(pos).is_some_and(|s| {
        let f = facts(s);
        f.collision_shape().is_empty() && !f.has(flag::LIQUID)
    })
}

/// Whether the top of the block at `pos` can be stood on.
fn floor(world: &World<Chunks>, pos: BlockPos) -> bool {
    world.block(pos).is_some_and(|s| {
        let f = facts(s);
        !f.has(flag::LIQUID) && presents_face(f.collision_shape(), Direction::Up)
    })
}

/// A cell where a 2-high mob can stand: free space for its body on a floor.
fn standable(world: &World<Chunks>, pos: BlockPos) -> bool {
    passable(world, pos) && passable(world, pos.above()) && floor(world, pos.below())
}

/// A* over standable cells: four directions, one step up, drops of up to three blocks.
pub fn find_path(
    world: &World<Chunks>,
    start: BlockPos,
    goal: BlockPos,
    max_nodes: usize,
) -> Option<Vec<BlockPos>> {
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashMap};
    if start == goal {
        return Some(vec![start]);
    }
    let h =
        |p: BlockPos| ((p.x - goal.x).abs() + (p.y - goal.y).abs() + (p.z - goal.z).abs()) as u32;
    let mut open = BinaryHeap::new();
    let mut came: HashMap<BlockPos, BlockPos> = HashMap::new();
    let mut cost: HashMap<BlockPos, u32> = HashMap::new();
    cost.insert(start, 0);
    open.push(Reverse((h(start), 0u32, (start.x, start.y, start.z))));
    let mut expanded = 0;
    let mut best = (h(start), start);
    while let Some(Reverse((_, g, (x, y, z)))) = open.pop() {
        let cur = BlockPos::new(x, y, z);
        if cost.get(&cur).is_some_and(|c| *c < g) {
            continue;
        }
        if cur == goal {
            return Some(rebuild(&came, cur));
        }
        expanded += 1;
        if expanded > max_nodes {
            break;
        }
        for dir in Direction::HORIZONTAL {
            let side = cur.offset(dir);
            let mut candidates = Vec::with_capacity(5);
            if standable(world, side) {
                candidates.push(side);
            } else if passable(world, cur.above().above())
                && passable(world, side.above())
                && standable(world, side.above())
            {
                // A step up: needs headroom over the current cell and the one beside it.
                candidates.push(side.above());
            }
            if passable(world, side) && passable(world, side.above()) {
                // Walk off an edge: the first floor within three blocks below.
                for drop in 1..=3 {
                    let below = side.offset(Direction::Down);
                    let at = BlockPos::new(below.x, side.y - drop, below.z);
                    if standable(world, at) {
                        candidates.push(at);
                        break;
                    }
                    if !passable(world, at) {
                        break;
                    }
                }
            }
            for next in candidates {
                let ng = g + 1 + u32::from(next.y != cur.y);
                if cost.get(&next).is_none_or(|c| ng < *c) {
                    cost.insert(next, ng);
                    came.insert(next, cur);
                    let f = ng + h(next);
                    if f < best.0 + 8 && h(next) < best.0 {
                        best = (h(next), next);
                    }
                    open.push(Reverse((f, ng, (next.x, next.y, next.z))));
                }
            }
        }
    }
    // No complete path: walk to the closest cell found (what vanilla's partial paths do).
    (best.1 != start).then(|| rebuild(&came, best.1))
}

fn rebuild(
    came: &std::collections::HashMap<BlockPos, BlockPos>,
    mut at: BlockPos,
) -> Vec<BlockPos> {
    let mut path = vec![at];
    while let Some(prev) = came.get(&at) {
        at = *prev;
        path.push(at);
    }
    path.reverse();
    path
}

/// What the mob is doing.
#[derive(Clone, Debug, PartialEq)]
pub enum Behaviour {
    Idle,
    /// Walking a path (wandering or fleeing).
    Walking {
        path: Vec<BlockPos>,
        next: usize,
        speed_modifier: f64,
        stuck: u32,
    },
}

#[derive(Clone, Debug)]
pub struct Mob {
    pub id: i32,
    pub uuid: Uuid,
    /// Id in the entity type registry.
    pub kind: u32,
    pub info: &'static EntityInfo,
    pub pos: [f64; 3],
    pub vel: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub health: f32,
    pub on_ground: bool,
    pub age: u64,
    pub behaviour: Behaviour,
    /// Ticks of fleeing left after being hurt.
    pub panic: u32,
    /// Ticks of invulnerability left, and the strongest hit taken during them.
    pub invulnerable: u32,
    pub last_hurt: f32,
    /// Counts up after death; the mob is removed at 20.
    pub death_time: Option<u32>,
    /// A player the mob is looking at, and for how long.
    pub looking: Option<(usize, u32)>,
    pub rng: Rng,
    /// Position and rotation (1/4096 blocks, angle bytes) other players last saw.
    pub sent_pos: [i64; 3],
    pub sent_rot: (u8, u8),
    pub sent_head: u8,
}

fn quantize(v: f64) -> i64 {
    (v * 4096.0).floor() as i64
}

pub fn angle_byte(deg: f32) -> u8 {
    ((deg * 256.0 / 360.0).floor() as i32 & 0xFF) as u8
}

fn wrap_degrees(mut d: f32) -> f32 {
    d %= 360.0;
    if d >= 180.0 {
        d -= 360.0;
    } else if d < -180.0 {
        d += 360.0;
    }
    d
}

/// Turns `from` toward `to` by at most `max` degrees.
fn rotate_toward(from: f32, to: f32, max: f32) -> f32 {
    from + wrap_degrees(to - from).clamp(-max, max)
}

impl Mob {
    pub fn new(
        id: i32,
        uuid: Uuid,
        kind: u32,
        info: &'static EntityInfo,
        pos: [f64; 3],
        yaw: f32,
    ) -> Self {
        let health = info.attributes.and_then(|a| a.max_health).unwrap_or(1.0) as f32;
        Mob {
            id,
            uuid,
            kind,
            info,
            pos,
            vel: [0.0; 3],
            yaw,
            pitch: 0.0,
            head_yaw: yaw,
            health,
            on_ground: false,
            age: 0,
            behaviour: Behaviour::Idle,
            panic: 0,
            invulnerable: 0,
            last_hurt: 0.0,
            death_time: None,
            looking: None,
            rng: Rng(uuid.as_u128() as u64 ^ 0x9E37_79B9_7F4A_7C15),
            sent_pos: pos.map(quantize),
            sent_rot: (angle_byte(yaw), 0),
            sent_head: angle_byte(yaw),
        }
    }

    pub fn bbox(&self) -> Aabb {
        let half = f64::from(self.info.width) / 2.0;
        let [x, y, z] = self.pos;
        [x - half, y, z - half, x + half, y + f64::from(self.info.height), z + half]
    }

    pub fn movement_speed(&self) -> f64 {
        self.info.attributes.and_then(|a| a.movement_speed).unwrap_or(0.2)
    }

    pub fn is_dead(&self) -> bool {
        self.death_time.is_some()
    }

    /// Takes a hit: invulnerability frames, knockback away from `from`, and flight.
    /// Returns whether it landed.
    pub fn hurt(&mut self, amount: f32, from: [f64; 3]) -> bool {
        if self.is_dead() || amount <= 0.0 {
            return false;
        }
        let dealt = if self.invulnerable > 10 {
            if amount <= self.last_hurt {
                return false;
            }
            let extra = amount - self.last_hurt;
            self.last_hurt = amount;
            extra
        } else {
            self.last_hurt = amount;
            self.invulnerable = 20;
            amount
        };
        self.health -= dealt;
        // Knockback: halve the old velocity and push away from the attacker.
        let (mut dx, mut dz) = (from[0] - self.pos[0], from[2] - self.pos[2]);
        let len = (dx * dx + dz * dz).sqrt();
        if len < 1.0e-4 {
            dx = self.rng.between(-1.0, 1.0);
            dz = self.rng.between(-1.0, 1.0);
        }
        let len = (dx * dx + dz * dz).sqrt().max(1.0e-4);
        let strength =
            0.4 * (1.0 - self.info.attributes.and_then(|a| a.knockback_resistance).unwrap_or(0.0));
        self.vel[0] = self.vel[0] / 2.0 - dx / len * strength;
        self.vel[2] = self.vel[2] / 2.0 - dz / len * strength;
        if self.on_ground {
            self.vel[1] = (self.vel[1] / 2.0 + strength).min(0.4);
        }
        self.panic = 60;
        self.behaviour = Behaviour::Idle;
        if self.health <= 0.0 {
            self.health = 0.0;
            self.death_time = Some(0);
        }
        true
    }

    /// Picks a place to wander to: the best of a few random spots nearby, preferring grass.
    fn pick_wander_target(&mut self, world: &World<Chunks>, range: i32) -> Option<BlockPos> {
        let here = BlockPos::new(
            self.pos[0].floor() as i32,
            self.pos[1].floor() as i32,
            self.pos[2].floor() as i32,
        );
        let mut best: Option<(f64, BlockPos)> = None;
        for _ in 0..10 {
            let dx = self.rng.below(2 * range as u32 + 1) as i32 - range;
            let dz = self.rng.below(2 * range as u32 + 1) as i32 - range;
            let col = BlockPos::new(here.x + dx, here.y, here.z + dz);
            // The nearest standable cell within 7 blocks up or down.
            let Some(at) = (0..=7)
                .flat_map(|o| [o, -o])
                .map(|o| BlockPos::new(col.x, col.y + o, col.z))
                .find(|p| standable(world, *p))
            else {
                continue;
            };
            let grass = world
                .block(at.below())
                .and_then(ferrite_data::blocks::block_of_state)
                .is_some_and(|b| b.name == "minecraft:grass_block");
            let score = if grass { 10.0 } else { 0.0 } + self.rng.unit();
            if best.is_none_or(|(s, _)| score > s) {
                best = Some((score, at));
            }
        }
        best.map(|(_, p)| p)
    }

    fn start_walking(&mut self, world: &World<Chunks>, goal: BlockPos, modifier: f64) {
        let here = BlockPos::new(
            self.pos[0].floor() as i32,
            self.pos[1].floor() as i32,
            self.pos[2].floor() as i32,
        );
        self.behaviour = match find_path(world, here, goal, 300) {
            Some(path) if path.len() > 1 => {
                Behaviour::Walking { path, next: 1, speed_modifier: modifier, stuck: 0 }
            }
            _ => Behaviour::Idle,
        };
    }

    /// One tick: goals, movement control, physics.
    pub fn tick(&mut self, world: &World<Chunks>, players: &[[f64; 3]]) {
        self.age += 1;
        self.invulnerable = self.invulnerable.saturating_sub(1);
        if self.invulnerable == 0 {
            self.last_hurt = 0.0;
        }
        if let Some(t) = &mut self.death_time {
            *t += 1;
            // The dead do not think; they just fall.
            self.physics(world, 0.0, false);
            return;
        }
        // Goals, in priority order: flee, wander; looking runs beside them.
        let mut forward = 0.0;
        let mut jump = false;
        let mut turn: Option<f32> = None;
        if self.panic > 0 {
            self.panic -= 1;
            if matches!(self.behaviour, Behaviour::Idle)
                && let Some(goal) = self.pick_wander_target(world, 5)
            {
                self.start_walking(world, goal, 2.0);
            }
        } else if matches!(self.behaviour, Behaviour::Idle)
            && self.rng.below(120) == 0
            && let Some(goal) = self.pick_wander_target(world, 10)
        {
            self.start_walking(world, goal, 1.0);
        }
        let base_speed = self.movement_speed();
        let on_ground = self.on_ground;
        let pos = self.pos;
        if let Behaviour::Walking { path, next, speed_modifier, stuck } = &mut self.behaviour {
            let Some(target) = path.get(*next).copied() else {
                self.behaviour = Behaviour::Idle;
                return self.finish_tick(world, 0.0, false, players);
            };
            let (tx, ty, tz) =
                (f64::from(target.x) + 0.5, f64::from(target.y), f64::from(target.z) + 0.5);
            let (dx, dz) = (tx - pos[0], tz - pos[2]);
            let dist2 = dx * dx + dz * dz;
            if dist2 < 0.25 * 0.25 && (ty - pos[1]).abs() < 1.0 {
                *next += 1;
                *stuck = 0;
            } else {
                let want = (dz.atan2(dx).to_degrees() as f32) - 90.0;
                turn = Some(want);
                forward = base_speed * *speed_modifier;
                // Jump when the next node is higher than we are.
                jump = ty > pos[1] + 0.5 && on_ground;
                *stuck += 1;
                if *stuck > 60 {
                    self.behaviour = Behaviour::Idle;
                }
            }
            if matches!(&self.behaviour, Behaviour::Walking { path, next, .. } if *next >= path.len())
            {
                self.behaviour = Behaviour::Idle;
            }
        }
        if let Some(want) = turn {
            self.yaw = rotate_toward(self.yaw, want, 90.0);
        }
        self.finish_tick(world, forward, jump, players);
    }

    fn finish_tick(
        &mut self,
        world: &World<Chunks>,
        forward: f64,
        jump: bool,
        players: &[[f64; 3]],
    ) {
        self.update_look(players);
        self.physics(world, forward, jump);
    }

    /// Looks at a player within six blocks now and then; the head turns up to 10 degrees a tick,
    /// the body follows when the head gets too far round.
    fn update_look(&mut self, players: &[[f64; 3]]) {
        if self.looking.is_none() && self.rng.unit() < 0.02 {
            let near = players
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    let d = [p[0] - self.pos[0], p[1] - self.pos[1], p[2] - self.pos[2]];
                    d[0] * d[0] + d[1] * d[1] + d[2] * d[2] < 36.0
                })
                .map(|(i, _)| i)
                .next();
            if let Some(i) = near {
                self.looking = Some((i, 40 + self.rng.below(40)));
            }
        }
        if let Some((i, left)) = self.looking {
            match players.get(i) {
                Some(p) if left > 0 => {
                    let (dx, dz) = (p[0] - self.pos[0], p[2] - self.pos[2]);
                    let want = (dz.atan2(dx).to_degrees() as f32) - 90.0;
                    self.head_yaw = rotate_toward(self.head_yaw, want, 10.0);
                    let dy = (p[1] + 1.62) - (self.pos[1] + f64::from(self.info.eye_height));
                    let horiz = (dx * dx + dz * dz).sqrt();
                    self.pitch =
                        rotate_toward(self.pitch, (-dy.atan2(horiz).to_degrees()) as f32, 40.0);
                    self.looking = Some((i, left - 1));
                }
                _ => self.looking = None,
            }
        } else {
            // Head returns to face the body.
            self.head_yaw = rotate_toward(self.head_yaw, self.yaw, 10.0);
            self.pitch = rotate_toward(self.pitch, 0.0, 10.0);
        }
        // The body follows a head turned more than 75 degrees away.
        let diff = wrap_degrees(self.head_yaw - self.yaw);
        if diff.abs() > 75.0 {
            self.yaw = rotate_toward(self.yaw, self.head_yaw, diff.abs() - 75.0);
        }
    }

    /// `LivingEntity.travel`: accelerate along the facing, move with collisions, then apply
    /// gravity and the floor's friction.
    fn physics(&mut self, world: &World<Chunks>, forward: f64, jump: bool) {
        let feet = BlockPos::new(
            self.pos[0].floor() as i32,
            self.pos[1].floor() as i32,
            self.pos[2].floor() as i32,
        );
        let in_water = world.block(feet).is_some_and(|s| facts(s).has(flag::LIQUID));
        if jump && self.on_ground {
            self.vel[1] = 0.42;
        }
        if in_water {
            // Swimming: slow, floaty, and pushing up toward the surface.
            self.accelerate(forward, 0.02);
            let moved = world.move_box_stepping(&self.bbox(), self.vel, 0.6, self.on_ground);
            self.apply(moved);
            self.vel[0] *= 0.8;
            self.vel[2] *= 0.8;
            self.vel[1] = self.vel[1] * 0.8 - 0.02 + 0.04;
            return;
        }
        let friction = if self.on_ground {
            let below = BlockPos::new(feet.x, (self.pos[1] - 0.500001).floor() as i32, feet.z);
            block_friction(world.block(below)) * 0.91
        } else {
            0.91
        };
        let influenced = if self.on_ground {
            self.movement_speed() * (0.216_000_02 / (friction * friction * friction))
        } else {
            0.02
        };
        self.accelerate(forward, influenced);
        let moved = world.move_box_stepping(&self.bbox(), self.vel, 0.6, self.on_ground);
        self.apply(moved);
        self.vel[1] -= 0.08;
        self.vel[0] *= friction;
        self.vel[1] *= 0.98;
        self.vel[2] *= friction;
    }

    /// `moveRelative`: adds `forward * factor` along the yaw to the velocity.
    fn accelerate(&mut self, forward: f64, factor: f64) {
        if forward == 0.0 {
            return;
        }
        let yaw = f64::from(self.yaw).to_radians();
        let (sin, cos) = (yaw.sin(), yaw.cos());
        self.vel[0] += -sin * forward * factor;
        self.vel[2] += cos * forward * factor;
    }

    fn apply(&mut self, moved: ferrite_blocks::collision::Moved) {
        let falling = self.vel[1] < 0.0;
        for i in 0..3 {
            self.pos[i] += moved.delta[i];
            if moved.blocked[i] {
                self.vel[i] = 0.0;
            }
        }
        // Standing on something means a downward move was cut short.
        self.on_ground = moved.blocked[1] && falling;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ferrite_blocks::{BlockAccess, Rules};
    use ferrite_data::blocks::block_by_name;
    use ferrite_world::{Chunk, ChunkPos, Dimension};

    fn world() -> World<Chunks> {
        let air = block_by_name("air").unwrap().default_state;
        let mut chunks = Chunks::new(Dimension::OVERWORLD);
        for cx in -2..=2 {
            for cz in -2..=2 {
                chunks.insert(ChunkPos::new(cx, cz), Chunk::filled(Dimension::OVERWORLD, air, 0));
            }
        }
        let mut w = World::new(chunks, Rules::default());
        let grass = block_by_name("grass_block").unwrap().default_state;
        for x in -20..20 {
            for z in -20..20 {
                w.blocks.set_block_raw(BlockPos::new(x, 63, z), grass);
            }
        }
        w
    }

    fn cow(world: &World<Chunks>) -> Mob {
        let _ = world;
        let (id, info) = ferrite_data::entities::entity_by_name("minecraft:cow").unwrap();
        Mob::new(1, Uuid::from_u128(77), id, info, [0.5, 64.0, 0.5], 0.0)
    }

    #[test]
    fn a_path_goes_around_walls_and_up_steps() {
        let mut w = world();
        let stone = block_by_name("stone").unwrap().default_state;
        // A wall of height 2 with a gap, and a one block step.
        for z in -5..=5 {
            for y in 64..66 {
                if z != 3 {
                    w.blocks.set_block_raw(BlockPos::new(2, y, z), stone);
                }
            }
        }
        let path =
            find_path(&w, BlockPos::new(0, 64, 0), BlockPos::new(4, 64, 0), 500).expect("path");
        assert_eq!(*path.last().unwrap(), BlockPos::new(4, 64, 0));
        assert!(path.iter().any(|p| p.x == 2 && p.z == 3), "goes through the gap: {path:?}");
        w.blocks.set_block_raw(BlockPos::new(1, 64, 0), stone);
        let up =
            find_path(&w, BlockPos::new(0, 64, 0), BlockPos::new(1, 65, 0), 100).expect("step");
        assert_eq!(up.last().unwrap().y, 65);
    }

    #[test]
    fn a_mob_settles_on_the_ground_and_wanders() {
        let w = world();
        let mut m = cow(&w);
        for _ in 0..40 {
            m.tick(&w, &[]);
        }
        assert!(m.on_ground);
        assert!((m.pos[1] - 64.0).abs() < 1e-3, "{:?}", m.pos);
        let start = m.pos;
        let mut moved = false;
        for _ in 0..4000 {
            m.tick(&w, &[]);
            moved |= (m.pos[0] - start[0]).hypot(m.pos[2] - start[2]) > 2.0;
        }
        assert!(moved, "a cow wanders within a few minutes");
        assert!(m.pos[0].abs() < 25.0 && (m.pos[1] - 64.0).abs() < 1e-3);
    }

    #[test]
    fn walking_speed_matches_the_movement_attribute() {
        // A cow (0.2) walking in a straight line on grass settles near 0.117 blocks a tick.
        let w = world();
        let mut m = cow(&w);
        for _ in 0..10 {
            m.tick(&w, &[]);
        }
        m.behaviour = Behaviour::Walking {
            path: vec![BlockPos::new(0, 64, 0), BlockPos::new(0, 64, 18)],
            next: 1,
            speed_modifier: 1.0,
            stuck: 0,
        };
        let z0 = m.pos[2];
        for _ in 0..40 {
            m.tick(&w, &[]);
        }
        let v = (m.pos[2] - z0) / 40.0;
        assert!((0.08..0.13).contains(&v), "speed {v}");
    }

    #[test]
    fn hits_knock_back_hurt_and_kill() {
        let w = world();
        let mut m = cow(&w);
        for _ in 0..20 {
            m.tick(&w, &[]);
        }
        assert!(m.hurt(4.0, [m.pos[0] - 1.0, m.pos[1], m.pos[2]]));
        assert_eq!(m.health, 6.0);
        assert!(m.vel[0] > 0.0, "pushed away from the attacker");
        assert!(!m.hurt(3.0, [0.0; 3]), "invulnerability frames absorb a weaker hit");
        assert!(m.hurt(6.0, [0.0; 3]), "a stronger one lands for the difference");
        assert_eq!(m.health, 4.0);
        for _ in 0..30 {
            m.tick(&w, &[]);
        }
        assert!(m.hurt(100.0, [0.0; 3]));
        assert!(m.is_dead());
    }
}
