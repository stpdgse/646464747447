//! A neighbour-interaction workload that exercises everything the scheduler has to get right:
//! halo exchange across region borders, entity migration between regions, and floating-point sums
//! whose value depends on the order they are added in.
//!
//! Particles repel each other within `radius`. Every particle is updated from the state *at the
//! start of the tick* (a Jacobi-style update), so the update order does not matter, and the
//! neighbours of each particle are always summed in ascending id order, so the result does not
//! depend on how the world is cut into regions or on how many threads run them.
//!
//! [`Reference`] is the same physics on a plain `Vec`, with no regions and no ECS. The tests
//! require the region-parallel simulation to match it bit for bit.

use ferrite_ecs::{Entity, World};

use crate::hash::StableHasher;
use crate::layout::Layout;
use crate::sim::{Region, Sim};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub id: u64,
    pub pos: [f64; 2],
    pub vel: [f64; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Interaction radius. Region cells must be at least this large.
    pub radius: f64,
    pub dt: f64,
    pub strength: f64,
    pub damping: f64,
    pub max_speed: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self { radius: 2.0, dt: 0.1, strength: 4.0, damping: 0.99, max_speed: 5.0 }
    }
}

#[derive(Clone, Copy, Debug)]
struct Id(u64);
#[derive(Clone, Copy, Debug)]
struct Pos([f64; 2]);
#[derive(Clone, Copy, Debug)]
struct Vel([f64; 2]);

/// What regions say to each other.
#[derive(Clone, Copy, Debug)]
pub enum Msg {
    /// A read-only copy of a particle near a border, for the neighbour's force computation.
    Ghost(Particle),
    /// A particle that moved out of the sender's region and now belongs to the receiver.
    Migrant(Particle),
}

pub type ParticleSim = Sim<Msg>;

/// Advances one particle given all particles near it, sorted by id.
///
/// Both the reference and the regions call this, so a mismatch can only come from the machinery
/// around it (which neighbours were found, in which order), not from duplicated physics.
pub fn step_particle(
    p: &Particle,
    neighbours: &[(u64, [f64; 2])],
    params: &Params,
    extent: [f64; 2],
) -> Particle {
    let r = params.radius;
    let r2 = r * r;
    let mut force = [0.0f64; 2];
    for &(id, pos) in neighbours {
        if id == p.id {
            continue;
        }
        let dx = p.pos[0] - pos[0];
        let dy = p.pos[1] - pos[1];
        let d2 = dx * dx + dy * dy;
        if d2 > 0.0 && d2 < r2 {
            let d = d2.sqrt();
            let w = (r - d) / r * params.strength;
            force[0] += dx / d * w;
            force[1] += dy / d * w;
        }
    }
    let mut vel = [
        (p.vel[0] + force[0] * params.dt) * params.damping,
        (p.vel[1] + force[1] * params.dt) * params.damping,
    ];
    let speed2 = vel[0] * vel[0] + vel[1] * vel[1];
    if speed2 > params.max_speed * params.max_speed {
        let scale = params.max_speed / speed2.sqrt();
        vel = [vel[0] * scale, vel[1] * scale];
    }
    let mut pos = [p.pos[0] + vel[0] * params.dt, p.pos[1] + vel[1] * params.dt];
    for axis in 0..2 {
        if pos[axis] < 0.0 {
            pos[axis] = -pos[axis];
            vel[axis] = -vel[axis];
        }
        if pos[axis] >= extent[axis] {
            pos[axis] = 2.0 * extent[axis] - pos[axis];
            vel[axis] = -vel[axis];
            if pos[axis] >= extent[axis] || pos[axis] < 0.0 {
                pos[axis] = extent[axis] * 0.5;
            }
        }
    }
    Particle { id: p.id, pos, vel }
}

/// A deterministic pseudo-random stream (SplitMix64) so that every run and every machine builds
/// the same world.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// `n` particles with ids `0..n`, spread over `extent`.
pub fn initial(n: usize, extent: [f64; 2], seed: u64) -> Vec<Particle> {
    let mut rng = Rng(seed);
    (0..n as u64)
        .map(|id| {
            let pos = [rng.unit() * extent[0], rng.unit() * extent[1]];
            let vel = [(rng.unit() - 0.5) * 2.0, (rng.unit() - 0.5) * 2.0];
            Particle { id, pos, vel }
        })
        .collect()
}

/// `n` particles packed into the lower-left `fraction` of the world, to stress load balancing.
pub fn clustered(n: usize, extent: [f64; 2], fraction: f64, seed: u64) -> Vec<Particle> {
    let mut rng = Rng(seed);
    (0..n as u64)
        .map(|id| {
            let pos = [rng.unit() * extent[0] * fraction, rng.unit() * extent[1] * fraction];
            let vel = [(rng.unit() - 0.5) * 2.0, (rng.unit() - 0.5) * 2.0];
            Particle { id, pos, vel }
        })
        .collect()
}

/// Puts each particle into the region that owns its position.
pub fn populate(sim: &mut ParticleSim, particles: &[Particle]) {
    let layout = *sim.layout();
    for p in particles {
        let region = sim.region_mut(layout.region_of(p.pos));
        region.world.spawn((Id(p.id), Pos(p.pos), Vel(p.vel)));
    }
}

fn snapshot_region(world: &mut World) -> Vec<(Entity, Particle)> {
    world
        .query::<(Entity, &Id, &Pos, &Vel)>()
        .map(|(e, id, p, v)| (e, Particle { id: id.0, pos: p.0, vel: v.0 }))
        .collect()
}

/// Turns arrivals from the last tick into entities, then publishes border particles to neighbours.
fn adopt_and_publish(region: &mut Region<Msg>, layout: &Layout, params: &Params) {
    adopt(region);
    let own = region.bounds;
    // Particles deeper inside the region than the interaction radius cannot be seen by anyone else.
    let inner = own.expanded(-params.radius);
    let neighbours: Vec<_> = layout
        .neighbors(region.id)
        .map(|n| (n, layout.bounds(n).expanded(params.radius)))
        .collect();
    let mut out: Vec<(crate::layout::RegionId, Particle)> = Vec::new();
    for (id, pos, vel) in region.world.query::<(&Id, &Pos, &Vel)>() {
        if inner.contains(pos.0) {
            continue;
        }
        for (nid, reach) in &neighbours {
            if reach.contains(pos.0) {
                out.push((*nid, Particle { id: id.0, pos: pos.0, vel: vel.0 }));
            }
        }
    }
    for (to, particle) in out {
        region.send(to, Msg::Ghost(particle));
    }
}

fn adopt(region: &mut Region<Msg>) {
    let mut arrivals: Vec<Particle> = Vec::new();
    region.inbox.retain(|m| match m {
        Msg::Migrant(p) => {
            arrivals.push(*p);
            false
        }
        Msg::Ghost(_) => true,
    });
    // Spawn order only affects storage order, which nothing depends on; sorting makes even that
    // independent of how the world is partitioned.
    arrivals.sort_by_key(|p| p.id);
    for p in arrivals {
        region.world.spawn((Id(p.id), Pos(p.pos), Vel(p.vel)));
    }
}

/// Computes new states from the start-of-tick snapshot, then applies them and sends away anything
/// that left the region.
fn update(region: &mut Region<Msg>, layout: &Layout, params: &Params) {
    let extent = layout.extent();
    let ghosts: Vec<Particle> = region
        .inbox
        .drain(..)
        .filter_map(|m| match m {
            Msg::Ghost(p) => Some(p),
            Msg::Migrant(_) => None,
        })
        .collect();
    let own = snapshot_region(&mut region.world);
    if own.is_empty() {
        return;
    }

    // A uniform grid with cells of one radius, over the region plus a one-radius halo.
    let r = params.radius;
    let origin = [region.bounds.min[0] - r, region.bounds.min[1] - r];
    let cols = ((region.bounds.max[0] - region.bounds.min[0]) / r).ceil() as usize + 2;
    let rows = ((region.bounds.max[1] - region.bounds.min[1]) / r).ceil() as usize + 2;
    let cell_of = |p: [f64; 2]| -> Option<usize> {
        let cx = ((p[0] - origin[0]) / r).floor();
        let cy = ((p[1] - origin[1]) / r).floor();
        (cx >= 0.0 && cy >= 0.0 && (cx as usize) < cols && (cy as usize) < rows)
            .then(|| cy as usize * cols + cx as usize)
    };
    let mut counts = vec![0u32; cols * rows + 1];
    let all = own.iter().map(|(_, p)| p).chain(ghosts.iter());
    let cells: Vec<Option<usize>> = all.clone().map(|p| cell_of(p.pos)).collect();
    for c in cells.iter().flatten() {
        counts[c + 1] += 1;
    }
    for i in 1..counts.len() {
        counts[i] += counts[i - 1];
    }
    let mut fill = counts.clone();
    let mut slots = vec![(0u64, [0.0f64; 2]); counts[cols * rows] as usize];
    for (p, c) in all.zip(&cells) {
        if let Some(c) = c {
            slots[fill[*c] as usize] = (p.id, p.pos);
            fill[*c] += 1;
        }
    }

    let mut near: Vec<(u64, [f64; 2])> = Vec::new();
    let mut updated: Vec<Particle> = Vec::with_capacity(own.len());
    for (_, p) in &own {
        near.clear();
        let cx = ((p.pos[0] - origin[0]) / r).floor() as usize;
        let cy = ((p.pos[1] - origin[1]) / r).floor() as usize;
        for y in cy.saturating_sub(1)..=(cy + 1).min(rows - 1) {
            for x in cx.saturating_sub(1)..=(cx + 1).min(cols - 1) {
                let c = y * cols + x;
                near.extend_from_slice(&slots[counts[c] as usize..counts[c + 1] as usize]);
            }
        }
        near.sort_unstable_by_key(|n| n.0);
        updated.push(step_particle(p, &near, params, extent));
    }

    // Apply in the same order the snapshot was taken, and send away whatever left the region.
    let mut leaving: Vec<Entity> = Vec::new();
    let mut migrants: Vec<Particle> = Vec::new();
    for (((e, _), new), (pos, vel)) in
        own.iter().zip(&updated).zip(region.world.query::<(&mut Pos, &mut Vel)>())
    {
        if region.bounds.contains(new.pos) {
            pos.0 = new.pos;
            vel.0 = new.vel;
        } else {
            leaving.push(*e);
            migrants.push(*new);
        }
    }
    for e in leaving {
        region.world.despawn(e);
    }
    for m in migrants {
        let to = layout.region_of(m.pos);
        region.send(to, Msg::Migrant(m));
    }
}

/// Runs one whole tick: publish, update, adopt, each followed by an exchange.
pub fn tick(sim: &mut ParticleSim, params: &Params) {
    let layout = *sim.layout();
    let params = *params;
    sim.step(|r| adopt_and_publish(r, &layout, &params));
    sim.step(|r| update(r, &layout, &params));
    sim.step(adopt);
    sim.finish_tick();
}

/// A deliberately **broken** tick that never publishes border particles, so regions cannot see
/// their neighbours. It exists as a negative control for the tests: if the equivalence tests
/// passed even with this, they would be proving nothing.
#[doc(hidden)]
pub fn tick_without_halo(sim: &mut ParticleSim, params: &Params) {
    let layout = *sim.layout();
    let params = *params;
    sim.step(adopt);
    sim.step(|r| update(r, &layout, &params));
    sim.step(adopt);
    sim.finish_tick();
}

/// Particles of one region, for tests that check which region owns what.
#[doc(hidden)]
pub fn snapshot_region_for_tests(world: &mut World) -> Vec<Particle> {
    snapshot_region(world).into_iter().map(|(_, p)| p).collect()
}

/// Every particle in the simulation, sorted by id.
pub fn snapshot(sim: &mut ParticleSim) -> Vec<Particle> {
    let mut all: Vec<Particle> = Vec::with_capacity(sim.entity_count());
    for region in sim.regions_mut() {
        all.extend(snapshot_region(&mut region.world).into_iter().map(|(_, p)| p));
    }
    all.sort_by_key(|p| p.id);
    all
}

/// Fingerprint of the exact state (every bit of every position and velocity).
pub fn fingerprint_of(particles: &[Particle]) -> u64 {
    let mut h = StableHasher::default();
    h.write_u64(particles.len() as u64);
    for p in particles {
        h.write_u64(p.id);
        p.pos.iter().chain(&p.vel).for_each(|v| h.write_f64(*v));
    }
    h.finish()
}

pub fn fingerprint(sim: &mut ParticleSim) -> u64 {
    fingerprint_of(&snapshot(sim))
}

/// The same physics with no regions, no ECS and no threads: every particle looks at every other.
#[derive(Clone, Debug)]
pub struct Reference {
    particles: Vec<Particle>,
    params: Params,
    extent: [f64; 2],
}

impl Reference {
    pub fn new(mut particles: Vec<Particle>, params: Params, extent: [f64; 2]) -> Self {
        particles.sort_by_key(|p| p.id);
        Self { particles, params, extent }
    }

    pub fn tick(&mut self) {
        let all: Vec<(u64, [f64; 2])> = self.particles.iter().map(|p| (p.id, p.pos)).collect();
        let next: Vec<Particle> = self
            .particles
            .iter()
            .map(|p| step_particle(p, &all, &self.params, self.extent))
            .collect();
        self.particles = next;
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    pub fn fingerprint(&self) -> u64 {
        fingerprint_of(&self.particles)
    }
}
