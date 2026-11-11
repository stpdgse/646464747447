//! How a tick of the particle workload scales with threads and with region count.
//!
//! Setup: 200,000 particles at a fixed density, so a bigger world means more regions of the same
//! size. Each iteration is one whole tick (publish, update, adopt and their exchanges).

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use ferrite_sim::particles::{self, Params, initial, populate, tick};
use ferrite_sim::{Layout, Sim};

const N: usize = 200_000;
/// About 8 neighbours within the interaction radius on average.
const DENSITY: f64 = 0.64;

fn world(cols: u32, threads: usize) -> (Sim<particles::Msg>, Params) {
    let extent = (N as f64 / DENSITY).sqrt();
    let layout = Layout::new(cols, cols, extent / f64::from(cols));
    let mut sim = Sim::new(layout, threads);
    populate(&mut sim, &initial(N, [extent, extent], 1));
    (sim, Params::default())
}

fn bench_threads(c: &mut Criterion) {
    let mut g = c.benchmark_group("tick, 200k particles, 16x16 regions");
    g.sample_size(10).throughput(Throughput::Elements(N as u64));
    for threads in [1, 2, 3, 4] {
        g.bench_function(format!("{threads} thread(s)"), |b| {
            let (mut sim, params) = world(16, threads);
            // Let the initial transient settle so every iteration measures steady-state work.
            for _ in 0..3 {
                tick(&mut sim, &params);
            }
            b.iter(|| tick(&mut sim, &params));
        });
    }
    g.finish();
}

fn bench_regions(c: &mut Criterion) {
    let mut g = c.benchmark_group("tick, 200k particles, 4 threads, varying region count");
    g.sample_size(10).throughput(Throughput::Elements(N as u64));
    for cols in [2, 4, 8, 16, 32] {
        g.bench_function(format!("{cols}x{cols} regions"), |b| {
            let (mut sim, params) = world(cols, 4);
            for _ in 0..3 {
                tick(&mut sim, &params);
            }
            b.iter(|| tick(&mut sim, &params));
        });
    }
    g.finish();
}

criterion_group!(benches, bench_threads, bench_regions);
criterion_main!(benches);
