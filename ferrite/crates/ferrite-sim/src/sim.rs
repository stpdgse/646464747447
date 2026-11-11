use ferrite_ecs::World;
use rayon::prelude::*;

use crate::layout::{Bounds, Layout, RegionId};

/// One tile of the world: an ECS world plus the mailboxes it uses to talk to other regions.
pub struct Region<M> {
    pub id: RegionId,
    pub bounds: Bounds,
    pub world: World,
    /// Messages delivered by the last exchange. A phase must drain this.
    pub inbox: Vec<M>,
    outbox: Vec<(RegionId, M)>,
}

impl<M> std::fmt::Debug for Region<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Region")
            .field("id", &self.id)
            .field("entities", &self.world.len())
            .field("inbox", &self.inbox.len())
            .finish()
    }
}

impl<M> Region<M> {
    /// Queues a message for `to`; it is delivered at the next exchange.
    pub fn send(&mut self, to: RegionId, message: M) {
        self.outbox.push((to, message));
    }
}

/// The scheduler: owns every region and the thread pool they tick on.
pub struct Sim<M: Send> {
    layout: Layout,
    regions: Vec<Region<M>>,
    pool: rayon::ThreadPool,
    tick: u64,
}

impl<M: Send> std::fmt::Debug for Sim<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sim")
            .field("layout", &self.layout)
            .field("threads", &self.pool.current_num_threads())
            .field("tick", &self.tick)
            .finish()
    }
}

impl<M: Send> Sim<M> {
    /// Creates `layout.len()` empty regions that will tick on `threads` worker threads.
    pub fn new(layout: Layout, threads: usize) -> Self {
        let regions = (0..layout.len() as u32)
            .map(|i| {
                let id = RegionId(i);
                Region {
                    id,
                    bounds: layout.bounds(id),
                    world: World::new(),
                    inbox: Vec::new(),
                    outbox: Vec::new(),
                }
            })
            .collect();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads.max(1))
            .thread_name(|i| format!("ferrite-sim-{i}"))
            .build()
            .expect("building the simulation thread pool");
        Self { layout, regions, pool, tick: 0 }
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn threads(&self) -> usize {
        self.pool.current_num_threads()
    }

    /// Number of completed ticks.
    pub fn tick_count(&self) -> u64 {
        self.tick
    }

    pub fn regions(&self) -> &[Region<M>] {
        &self.regions
    }

    pub fn regions_mut(&mut self) -> &mut [Region<M>] {
        &mut self.regions
    }

    pub fn region_mut(&mut self, id: RegionId) -> &mut Region<M> {
        &mut self.regions[id.0 as usize]
    }

    /// Total entities across all regions.
    pub fn entity_count(&self) -> usize {
        self.regions.iter().map(|r| r.world.len()).sum()
    }

    /// Runs `phase` on every region in parallel.
    ///
    /// `phase` gets exclusive access to one region and nothing else, so the order in which the
    /// regions finish cannot influence any result.
    pub fn run_phase(&mut self, phase: impl Fn(&mut Region<M>) + Sync) {
        let regions = &mut self.regions;
        let phase = &phase;
        self.pool.install(|| regions.par_iter_mut().for_each(phase));
    }

    /// Delivers every queued message.
    ///
    /// Outboxes are read in region-id order and each is read in emission order, so a destination's
    /// inbox is ordered by (source region, emission order): a function of the messages alone. This
    /// runs on the calling thread, in the same order whatever the thread count.
    pub fn exchange(&mut self) {
        let mut batches: Vec<Vec<(RegionId, M)>> =
            self.regions.iter_mut().map(|r| std::mem::take(&mut r.outbox)).collect();
        for batch in &mut batches {
            for (to, message) in batch.drain(..) {
                self.regions[to.0 as usize].inbox.push(message);
            }
        }
    }

    /// A phase followed by an exchange: the unit a tick is built from.
    pub fn step(&mut self, phase: impl Fn(&mut Region<M>) + Sync) {
        self.run_phase(phase);
        self.exchange();
    }

    /// Marks a tick as complete.
    pub fn finish_tick(&mut self) {
        self.tick += 1;
    }
}
