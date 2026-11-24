#![allow(dead_code)]
//! Ferrite's ECS against `bevy_ecs` and `hecs`, plus plain-`Vec` baselines that show the speed of
//! light for the same arithmetic.

use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};

const N: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos {
    x: f32,
    y: f32,
    z: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Vel {
    x: f32,
    y: f32,
    z: f32,
}
#[derive(Clone, Copy)]
struct A(u32);
#[derive(Clone, Copy)]
struct B(u32);
#[derive(Clone, Copy)]
struct C(u32);
#[derive(Clone, Copy)]
struct D(u32);

#[derive(bevy_ecs::component::Component, Clone, Copy)]
struct BPos {
    x: f32,
    y: f32,
    z: f32,
}
#[derive(bevy_ecs::component::Component, Clone, Copy)]
struct BVel {
    x: f32,
    y: f32,
    z: f32,
}
#[derive(bevy_ecs::component::Component, Clone, Copy)]
struct BA(u32);
#[derive(bevy_ecs::component::Component, Clone, Copy)]
struct BB(u32);
#[derive(bevy_ecs::component::Component, Clone, Copy)]
struct BC(u32);
#[derive(bevy_ecs::component::Component, Clone, Copy)]
struct BD(u32);

fn pos(i: usize) -> Pos {
    Pos { x: i as f32, y: 0.0, z: 1.0 }
}
const VEL: Vel = Vel { x: 1.0, y: 2.0, z: 3.0 };

fn ferrite_world(n: usize) -> ferrite_ecs::World {
    let mut w = ferrite_ecs::World::new();
    for i in 0..n {
        w.spawn((pos(i), VEL));
    }
    w
}

fn ferrite_mixed(n: usize) -> ferrite_ecs::World {
    let mut w = ferrite_ecs::World::new();
    for i in 0..n {
        match i % 4 {
            0 => w.spawn((pos(i), VEL, A(0))),
            1 => w.spawn((pos(i), VEL, B(0))),
            2 => w.spawn((pos(i), VEL, C(0))),
            _ => w.spawn((pos(i), VEL, D(0))),
        };
    }
    w
}

fn bench_iterate(c: &mut Criterion) {
    let mut g = c.benchmark_group("iterate pos += vel (1M entities)");
    g.throughput(Throughput::Elements(N as u64));

    g.bench_function("ferrite", |b| {
        let mut w = ferrite_world(N);
        b.iter(|| {
            for (p, v) in w.query::<(&mut Pos, &Vel)>() {
                p.x += v.x;
                p.y += v.y;
                p.z += v.z;
            }
        })
    });

    g.bench_function("bevy_ecs", |b| {
        let mut w = bevy_ecs::world::World::new();
        for i in 0..N {
            w.spawn((BPos { x: i as f32, y: 0.0, z: 1.0 }, BVel { x: 1.0, y: 2.0, z: 3.0 }));
        }
        let mut q = w.query::<(&mut BPos, &BVel)>();
        b.iter(|| {
            for (mut p, v) in q.iter_mut(&mut w) {
                p.x += v.x;
                p.y += v.y;
                p.z += v.z;
            }
        })
    });

    g.bench_function("hecs", |b| {
        let mut w = hecs::World::new();
        for i in 0..N {
            w.spawn((pos(i), VEL));
        }
        b.iter(|| {
            for (p, v) in w.query_mut::<(&mut Pos, &Vel)>() {
                p.x += v.x;
                p.y += v.y;
                p.z += v.z;
            }
        })
    });

    g.bench_function("baseline: two Vecs (SoA)", |b| {
        let mut ps: Vec<Pos> = (0..N).map(pos).collect();
        let vs = vec![VEL; N];
        b.iter(|| {
            for (p, v) in ps.iter_mut().zip(&vs) {
                p.x += v.x;
                p.y += v.y;
                p.z += v.z;
            }
            black_box(&ps);
        })
    });

    g.bench_function("baseline: one Vec of structs (AoS)", |b| {
        let mut e: Vec<(Pos, Vel)> = (0..N).map(|i| (pos(i), VEL)).collect();
        b.iter(|| {
            for (p, v) in e.iter_mut() {
                p.x += v.x;
                p.y += v.y;
                p.z += v.z;
            }
            black_box(&e);
        })
    });
    g.finish();
}

fn bench_mixed(c: &mut Criterion) {
    let mut g = c.benchmark_group("iterate across 4 archetypes (1M entities)");
    g.throughput(Throughput::Elements(N as u64));
    g.bench_function("ferrite", |b| {
        let mut w = ferrite_mixed(N);
        b.iter(|| {
            for (p, v) in w.query::<(&mut Pos, &Vel)>() {
                p.x += v.x;
            }
        })
    });
    g.bench_function("bevy_ecs", |b| {
        let mut w = bevy_ecs::world::World::new();
        for i in 0..N {
            let (p, v) = (BPos { x: i as f32, y: 0.0, z: 1.0 }, BVel { x: 1.0, y: 2.0, z: 3.0 });
            match i % 4 {
                0 => w.spawn((p, v, BA(0))),
                1 => w.spawn((p, v, BB(0))),
                2 => w.spawn((p, v, BC(0))),
                _ => w.spawn((p, v, BD(0))),
            };
        }
        let mut q = w.query::<(&mut BPos, &BVel)>();
        b.iter(|| {
            for (mut p, v) in q.iter_mut(&mut w) {
                p.x += v.x;
            }
        })
    });
    g.bench_function("hecs", |b| {
        let mut w = hecs::World::new();
        for i in 0..N {
            match i % 4 {
                0 => w.spawn((pos(i), VEL, A(0))),
                1 => w.spawn((pos(i), VEL, B(0))),
                2 => w.spawn((pos(i), VEL, C(0))),
                _ => w.spawn((pos(i), VEL, D(0))),
            };
        }
        b.iter(|| {
            for (p, v) in w.query_mut::<(&mut Pos, &Vel)>() {
                p.x += v.x;
            }
        })
    });
    g.finish();
}

fn bench_spawn(c: &mut Criterion) {
    let n = 100_000;
    let mut g = c.benchmark_group("spawn 100k (pos, vel)");
    g.throughput(Throughput::Elements(n as u64));
    g.bench_function("ferrite", |b| b.iter(|| black_box(ferrite_world(n))));
    g.bench_function("bevy_ecs", |b| {
        b.iter(|| {
            let mut w = bevy_ecs::world::World::new();
            for i in 0..n {
                w.spawn((BPos { x: i as f32, y: 0.0, z: 1.0 }, BVel { x: 1.0, y: 2.0, z: 3.0 }));
            }
            black_box(w)
        })
    });
    g.bench_function("hecs", |b| {
        b.iter(|| {
            let mut w = hecs::World::new();
            for i in 0..n {
                w.spawn((pos(i), VEL));
            }
            black_box(w)
        })
    });
    g.finish();
}

fn bench_structural(c: &mut Criterion) {
    let n = 100_000;
    let mut g = c.benchmark_group("insert then remove a component on 100k entities");
    g.throughput(Throughput::Elements(2 * n as u64));
    g.bench_function("ferrite", |b| {
        b.iter_batched(
            || {
                let mut w = ferrite_ecs::World::new();
                let es: Vec<_> = (0..n).map(|i| w.spawn((pos(i), VEL))).collect();
                (w, es)
            },
            |(mut w, es)| {
                for &e in &es {
                    w.insert(e, A(1));
                }
                for &e in &es {
                    w.remove::<A>(e);
                }
                black_box(w)
            },
            BatchSize::LargeInput,
        )
    });
    g.bench_function("bevy_ecs", |b| {
        b.iter_batched(
            || {
                let mut w = bevy_ecs::world::World::new();
                let es: Vec<_> = (0..n)
                    .map(|i| {
                        w.spawn((
                            BPos { x: i as f32, y: 0.0, z: 1.0 },
                            BVel { x: 1.0, y: 2.0, z: 3.0 },
                        ))
                        .id()
                    })
                    .collect();
                (w, es)
            },
            |(mut w, es)| {
                for &e in &es {
                    w.entity_mut(e).insert(BA(1));
                }
                for &e in &es {
                    w.entity_mut(e).remove::<BA>();
                }
                black_box(w)
            },
            BatchSize::LargeInput,
        )
    });
    g.bench_function("hecs", |b| {
        b.iter_batched(
            || {
                let mut w = hecs::World::new();
                let es: Vec<_> = (0..n).map(|i| w.spawn((pos(i), VEL))).collect();
                (w, es)
            },
            |(mut w, es)| {
                for &e in &es {
                    w.insert_one(e, A(1)).unwrap();
                }
                for &e in &es {
                    w.remove_one::<A>(e).unwrap();
                }
                black_box(w)
            },
            BatchSize::LargeInput,
        )
    });
    g.finish();
}

criterion_group!(benches, bench_iterate, bench_mixed, bench_spawn, bench_structural);
criterion_main!(benches);
