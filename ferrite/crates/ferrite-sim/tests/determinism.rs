use std::collections::HashSet;
use std::time::Duration;

use ferrite_sim::particles::{
    self, Params, Particle, ParticleSim, Reference, clustered, fingerprint, fingerprint_of,
    initial, populate, snapshot, tick, tick_without_halo,
};
use ferrite_sim::{Layout, RegionId, Sim, StableHasher};
use proptest::prelude::*;

fn params() -> Params {
    Params::default()
}

fn square(cols: u32, extent: f64) -> Layout {
    Layout::new(cols, cols, extent / f64::from(cols))
}

fn sim_with(layout: Layout, threads: usize, particles: &[Particle]) -> ParticleSim {
    let mut sim = Sim::new(layout, threads);
    populate(&mut sim, particles);
    sim
}

#[test]
fn layout_geometry() {
    let l = Layout::new(4, 3, 10.0);
    assert_eq!((l.len(), l.extent()), (12, [40.0, 30.0]));
    assert_eq!(l.id(3, 2), RegionId(11));
    assert_eq!(l.coords(RegionId(11)), (3, 2));
    // Bounds are half-open, so every point has exactly one owner.
    let b = l.bounds(RegionId(0));
    assert!(b.contains([0.0, 0.0]) && b.contains([9.999, 9.999]) && !b.contains([10.0, 5.0]));
    assert_eq!(l.region_of([10.0, 0.0]), RegionId(1));
    assert_eq!(l.region_of([39.999, 29.999]), RegionId(11));
    // Points outside the world belong to the nearest edge region instead of panicking.
    assert_eq!(l.region_of([-5.0, -5.0]), RegionId(0));
    assert_eq!(l.region_of([1e9, 1e9]), RegionId(11));
    // Neighbour counts: corner 3, edge 5, interior 8, and never the region itself.
    let count = |c, r| l.neighbors(l.id(c, r)).count();
    assert_eq!((count(0, 0), count(1, 0), count(1, 1)), (3, 5, 8));
    assert!(l.neighbors(l.id(1, 1)).all(|n| n != l.id(1, 1)));
    assert_eq!(Layout::new(1, 1, 5.0).neighbors(RegionId(0)).count(), 0);
}

#[test]
fn stable_hasher_is_deterministic_and_bit_sensitive() {
    let hash = |vals: &[f64]| {
        let mut h = StableHasher::default();
        vals.iter().for_each(|v| h.write_f64(*v));
        h.finish()
    };
    assert_eq!(hash(&[1.0, 2.0]), hash(&[1.0, 2.0]));
    assert_ne!(hash(&[1.0, 2.0]), hash(&[2.0, 1.0]));
    assert_ne!(hash(&[0.0]), hash(&[-0.0]), "the hash sees the exact bits");
    assert_ne!(hash(&[1.0]), hash(&[1.0 + f64::EPSILON]));
    // A pinned value, computed independently in Python from the algorithm's definition, so the
    // hash cannot drift unnoticed between versions or platforms.
    assert_eq!(hash(&[1.0, 2.0, 3.0]), 0x5c80_c17d_f77b_ba7a);
}

/// The headline guarantee: for every layout and every thread count, every tick equals the plain
/// single-threaded reference bit for bit.
#[test]
fn matches_the_reference_for_every_layout_and_thread_count() {
    let extent = 40.0;
    let particles = initial(700, [extent, extent], 7);
    let mut reference = Reference::new(particles.clone(), params(), [extent, extent]);
    let mut expected = Vec::new();
    for _ in 0..40 {
        reference.tick();
        expected.push(reference.fingerprint());
    }

    for cols in [1, 2, 4, 5, 8, 10, 20] {
        for threads in [1, 2, 3, 4, 8] {
            let mut sim = sim_with(square(cols, extent), threads, &particles);
            for (t, want) in expected.iter().enumerate() {
                tick(&mut sim, &params());
                assert_eq!(
                    fingerprint(&mut sim),
                    *want,
                    "{cols}x{cols} regions, {threads} threads, tick {t}"
                );
            }
            assert_eq!(
                snapshot(&mut sim),
                reference.particles(),
                "{cols}x{cols}, {threads} threads"
            );
        }
    }
}

#[test]
fn rectangular_worlds_work_too() {
    let (w, h) = (60.0, 20.0);
    let particles = initial(500, [w, h], 11);
    let mut reference = Reference::new(particles.clone(), params(), [w, h]);
    for (cols, rows, cell) in [(1, 1, 60.0), (3, 1, 20.0), (6, 2, 10.0), (12, 4, 5.0)] {
        if (f64::from(cols) * cell, f64::from(rows) * cell) != (w, h) {
            continue; // only layouts that tile this world exactly
        }
        let mut sim = sim_with(Layout::new(cols, rows, cell), 4, &particles);
        let mut r = reference.clone();
        for _ in 0..30 {
            r.tick();
            tick(&mut sim, &params());
        }
        assert_eq!(snapshot(&mut sim), r.particles(), "{cols}x{rows}");
    }
    reference.tick();
}

/// The same simulation run repeatedly with real parallelism must never differ: thread timing has
/// no influence on the result.
#[test]
fn repeated_parallel_runs_are_identical() {
    let particles = initial(600, [32.0, 32.0], 3);
    let run = || {
        let mut sim = sim_with(square(8, 32.0), 4, &particles);
        (0..25)
            .map(|_| {
                tick(&mut sim, &params());
                fingerprint(&mut sim)
            })
            .collect::<Vec<_>>()
    };
    let first = run();
    for i in 0..12 {
        assert_eq!(run(), first, "run {i} diverged");
    }
}

#[test]
fn a_heavily_unbalanced_world_still_matches() {
    // Everything starts in one corner: one region is hot while most are empty, and the cloud
    // spreads across borders as it relaxes.
    let extent = 48.0;
    let particles = clustered(900, [extent, extent], 0.2, 5);
    let mut reference = Reference::new(particles.clone(), params(), [extent, extent]);
    let mut sim = sim_with(square(8, extent), 4, &particles);
    for t in 0..50 {
        reference.tick();
        tick(&mut sim, &params());
        assert_eq!(fingerprint(&mut sim), reference.fingerprint(), "tick {t}");
    }
}

/// Negative control: the equality above is only meaningful if the scenario really needs the halo
/// exchange and migration. Without them the result must differ.
#[test]
fn the_scenario_really_exercises_halos_and_migration() {
    let extent = 40.0;
    let particles = initial(700, [extent, extent], 7);
    let mut reference = Reference::new(particles.clone(), params(), [extent, extent]);

    let mut broken = sim_with(square(8, extent), 4, &particles);
    let mut working = sim_with(square(8, extent), 4, &particles);
    let layout = *working.layout();
    let mut migrations = 0usize;
    let mut previous = snapshot(&mut working);
    for _ in 0..40 {
        reference.tick();
        tick_without_halo(&mut broken, &params());
        tick(&mut working, &params());
        let now = snapshot(&mut working);
        migrations += previous
            .iter()
            .zip(&now)
            .filter(|(a, b)| layout.region_of(a.pos) != layout.region_of(b.pos))
            .count();
        previous = now;
    }
    assert!(migrations > 200, "particles must cross region borders ({migrations} did)");
    assert_ne!(
        fingerprint(&mut broken),
        reference.fingerprint(),
        "without halo exchange the result must differ, otherwise the test proves nothing"
    );
    assert_eq!(fingerprint(&mut working), reference.fingerprint());

    // And the physics itself matters: with interaction switched off the outcome is different too.
    let mut free =
        Reference::new(particles, Params { strength: 0.0, ..params() }, [extent, extent]);
    (0..40).for_each(|_| free.tick());
    assert_ne!(free.fingerprint(), reference.fingerprint());
}

#[test]
fn no_particle_is_lost_or_duplicated() {
    let particles = initial(500, [30.0, 30.0], 9);
    let mut sim = sim_with(square(6, 30.0), 3, &particles);
    for _ in 0..60 {
        tick(&mut sim, &params());
        let all = snapshot(&mut sim);
        assert_eq!(all.len(), 500);
        let ids: HashSet<u64> = all.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), 500, "an id appears twice");
        assert!(
            all.iter()
                .all(|p| p.pos[0] >= 0.0 && p.pos[0] < 30.0 && p.pos[1] >= 0.0 && p.pos[1] < 30.0)
        );
        // Every particle lives in the region that owns its position.
        let layout = *sim.layout();
        for region in sim.regions_mut() {
            let id = region.id;
            for p in particles::snapshot_region_for_tests(&mut region.world) {
                assert_eq!(layout.region_of(p.pos), id, "particle {} is in the wrong region", p.id);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The exchange on its own, with scheduling made as hostile as possible.
// ---------------------------------------------------------------------------------------------

fn run_exchange(threads: usize, jitter: bool) -> Vec<Vec<(u32, u32)>> {
    let layout = Layout::new(6, 6, 10.0);
    let mut sim: Sim<(u32, u32)> = Sim::new(layout, threads);
    // Every region sends several messages to several destinations, in a region-specific pattern.
    sim.run_phase(|region| {
        let me = region.id.0;
        if jitter {
            // Make finishing order differ from region order.
            std::thread::sleep(Duration::from_micros(u64::from((me * 7919) % 400)));
        }
        for k in 0..5u32 {
            let to = RegionId((me * 5 + k * 11) % 36);
            region.send(to, (me, k));
        }
    });
    sim.exchange();
    sim.regions().iter().map(|r| r.inbox.clone()).collect()
}

#[test]
fn the_exchange_is_ordered_by_source_region_then_emission() {
    let reference = run_exchange(1, false);
    // The documented order: by source region, then emission order.
    for inbox in &reference {
        let mut sorted = inbox.clone();
        sorted.sort();
        assert_eq!(*inbox, sorted);
    }
    assert_eq!(reference.iter().map(Vec::len).sum::<usize>(), 36 * 5, "nothing lost");
    for threads in [2, 3, 4, 8, 16] {
        for _ in 0..5 {
            assert_eq!(run_exchange(threads, true), reference, "{threads} threads");
        }
    }
}

#[test]
fn outboxes_are_emptied_by_an_exchange() {
    let mut sim: Sim<u8> = Sim::new(Layout::new(2, 1, 5.0), 2);
    sim.run_phase(|r| r.send(RegionId(1 - r.id.0), 7));
    sim.exchange();
    assert_eq!(sim.regions()[0].inbox, [7]);
    sim.exchange(); // a second exchange delivers nothing new
    assert_eq!(sim.regions()[0].inbox, [7]);
    assert_eq!((sim.tick_count(), sim.threads(), sim.entity_count()), (0, 2, 0));
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 24, ..ProptestConfig::default() })]

    #[test]
    fn random_worlds_match_the_reference(
        n in 1usize..250,
        cols in prop::sample::select(vec![1u32, 2, 3, 4, 6, 8]),
        threads in 1usize..6,
        ticks in 1usize..25,
        seed in any::<u64>(),
    ) {
        let extent = 24.0;
        let particles = initial(n, [extent, extent], seed);
        let mut reference = Reference::new(particles.clone(), params(), [extent, extent]);
        let mut sim = sim_with(square(cols, extent), threads, &particles);
        for _ in 0..ticks {
            reference.tick();
            tick(&mut sim, &params());
        }
        prop_assert_eq!(fingerprint(&mut sim), reference.fingerprint());
        prop_assert_eq!(fingerprint_of(&snapshot(&mut sim)), fingerprint_of(reference.particles()));
    }
}
