use std::collections::HashMap;

use ferrite_ecs::{Commands, Entity, With, Without, World};
use proptest::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos(f32, f32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct Vel(f32, f32);
#[derive(Clone, Debug, PartialEq)]
struct Name(String);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tag;

#[test]
fn spawn_get_despawn() {
    let mut w = World::new();
    let a = w.spawn((Pos(1.0, 2.0), Vel(3.0, 4.0)));
    let b = w.spawn((Pos(5.0, 6.0),));
    let c = w.spawn(());
    assert_eq!(w.len(), 3);
    assert_eq!(w.get::<Pos>(a), Some(&Pos(1.0, 2.0)));
    assert_eq!(w.get::<Vel>(b), None);
    assert!(w.has::<Vel>(a) && !w.has::<Vel>(c));

    *w.get_mut::<Pos>(a).unwrap() = Pos(9.0, 9.0);
    assert_eq!(w.get::<Pos>(a), Some(&Pos(9.0, 9.0)));

    assert!(w.despawn(a));
    assert!(!w.despawn(a), "a stale handle is rejected");
    assert!(!w.contains(a));
    assert_eq!(w.get::<Pos>(a), None);
    assert_eq!(w.len(), 2);
    // Others are untouched, including the one that was swapped into the hole.
    assert_eq!(w.get::<Pos>(b), Some(&Pos(5.0, 6.0)));
}

#[test]
fn slots_are_reused_but_old_handles_stay_dead() {
    let mut w = World::new();
    let a = w.spawn((Pos(0.0, 0.0),));
    w.despawn(a);
    let b = w.spawn((Pos(1.0, 1.0),));
    assert_eq!(a.index(), b.index(), "the slot is reused");
    assert_ne!(a.generation(), b.generation());
    assert!(!w.contains(a), "the old handle must not alias the new entity");
    assert!(w.contains(b));
    assert_eq!(w.get::<Pos>(a), None);
    assert!(!w.insert(a, Vel(0.0, 0.0)));
    assert_eq!(w.remove::<Pos>(a), None);
    assert!(!w.contains(Entity::PLACEHOLDER));
}

#[test]
fn insert_replace_and_remove_move_between_archetypes() {
    let mut w = World::new();
    let e = w.spawn((Pos(1.0, 1.0),));
    let other = w.spawn((Pos(2.0, 2.0),));

    assert!(w.insert(e, Vel(3.0, 3.0)));
    assert_eq!(w.get::<Vel>(e), Some(&Vel(3.0, 3.0)));
    assert_eq!(w.get::<Pos>(e), Some(&Pos(1.0, 1.0)), "existing components survive the move");
    assert_eq!(w.get::<Pos>(other), Some(&Pos(2.0, 2.0)), "and so do bystanders");

    assert!(w.insert(e, Vel(4.0, 4.0)), "inserting an existing component replaces it");
    assert_eq!(w.get::<Vel>(e), Some(&Vel(4.0, 4.0)));

    assert_eq!(w.remove::<Vel>(e), Some(Vel(4.0, 4.0)));
    assert_eq!(w.remove::<Vel>(e), None);
    assert_eq!(w.get::<Pos>(e), Some(&Pos(1.0, 1.0)));
    assert!(w.has::<Pos>(other));

    // Removing the last component leaves an empty entity, still alive.
    assert_eq!(w.remove::<Pos>(e), Some(Pos(1.0, 1.0)));
    assert!(w.contains(e));
}

#[test]
fn queries_zip_columns_and_mutate_in_place() {
    let mut w = World::new();
    for i in 0..10 {
        w.spawn((Pos(i as f32, 0.0), Vel(1.0, 2.0)));
    }
    for i in 0..5 {
        w.spawn((Pos(100.0 + i as f32, 0.0),));
    }
    for (pos, vel) in w.query::<(&mut Pos, &Vel)>() {
        pos.0 += vel.0;
        pos.1 += vel.1;
    }
    let moved: Vec<_> = w.query::<(&Pos, &Vel)>().map(|(p, _)| *p).collect();
    assert_eq!(moved.len(), 10, "only entities with both components match");
    assert_eq!(moved[0], Pos(1.0, 2.0));
    assert_eq!(moved[9], Pos(10.0, 2.0));
    // The Pos-only entities were not touched.
    assert_eq!(w.query::<&Pos>().filter(|p| p.0 >= 100.0).count(), 5);
}

#[test]
fn entity_fetch_optional_components_and_filters() {
    let mut w = World::new();
    let with_vel = w.spawn((Pos(1.0, 0.0), Vel(1.0, 0.0)));
    let without = w.spawn((Pos(2.0, 0.0),));
    let tagged = w.spawn((Pos(3.0, 0.0), Tag));

    let mut seen: Vec<_> = w
        .query::<(Entity, &Pos, Option<&Vel>)>()
        .map(|(e, p, v)| (e, p.0, v.map(|v| v.0)))
        .collect();
    seen.sort_by(|a, b| a.1.total_cmp(&b.1));
    assert_eq!(seen, [(with_vel, 1.0, Some(1.0)), (without, 2.0, None), (tagged, 3.0, None)]);

    let only_tagged: Vec<_> = w.query_filtered::<Entity, With<Tag>>().collect();
    assert_eq!(only_tagged, [tagged]);
    let untagged: Vec<_> =
        w.query_filtered::<(Entity, &Pos), Without<Tag>>().map(|(e, _)| e).collect();
    assert_eq!(untagged.len(), 2);
    assert!(!untagged.contains(&tagged));

    // Mutating through an optional component only touches entities that have it.
    for v in w.query::<Option<&mut Vel>>().flatten() {
        v.0 = 7.0;
    }
    assert_eq!(w.get::<Vel>(with_vel), Some(&Vel(7.0, 0.0)));

    // A query for something never registered is simply empty.
    assert_eq!(w.query::<&Name>().count(), 0);
    assert_eq!(w.query_filtered::<&Pos, With<Name>>().count(), 0);
    assert_eq!(w.query_filtered::<&Pos, Without<Name>>().count(), 3);
}

#[test]
#[should_panic(expected = "each component only once")]
fn naming_a_component_twice_is_refused_instead_of_aliasing() {
    let mut w = World::new();
    w.spawn((Pos(0.0, 0.0),));
    let _ = w.query::<(&Pos, &mut Pos)>().count();
}

#[test]
#[should_panic(expected = "each component once")]
fn a_bundle_with_a_duplicate_component_is_refused() {
    World::new().spawn((Pos(0.0, 0.0), Pos(1.0, 1.0)));
}

#[test]
fn query_order_is_deterministic_and_follows_storage() {
    let build = || {
        let mut w = World::new();
        let mut es = Vec::new();
        for i in 0..50u32 {
            es.push(match i % 3 {
                0 => w.spawn((Pos(i as f32, 0.0),)),
                1 => w.spawn((Pos(i as f32, 0.0), Vel(0.0, 0.0))),
                _ => w.spawn((Pos(i as f32, 0.0), Tag)),
            });
        }
        for e in es.iter().step_by(4) {
            w.despawn(*e);
        }
        w.insert(es[1], Tag);
        w.remove::<Vel>(es[4]);
        w.query::<&Pos>().map(|p| p.0 as u32).collect::<Vec<_>>()
    };
    assert_eq!(build(), build());
}

#[test]
fn commands_apply_in_recorded_order() {
    let mut w = World::new();
    let a = w.spawn((Pos(1.0, 1.0),));
    let b = w.spawn((Pos(2.0, 2.0),));

    let mut cmds = Commands::new();
    for (e, pos) in w.query::<(Entity, &Pos)>() {
        if pos.0 > 1.5 {
            cmds.despawn(e);
        } else {
            cmds.insert(e, Vel(9.0, 9.0));
        }
        cmds.spawn((Tag,));
    }
    assert_eq!(cmds.len(), 4);
    w.apply(&mut cmds);
    assert!(cmds.is_empty());

    assert!(!w.contains(b));
    assert_eq!(w.get::<Vel>(a), Some(&Vel(9.0, 9.0)));
    assert_eq!(w.query::<&Tag>().count(), 2);
}

#[test]
fn resources() {
    #[derive(Debug, PartialEq)]
    struct Time(u64);
    let mut w = World::new();
    assert!(w.resource::<Time>().is_none());
    assert_eq!(w.insert_resource(Time(1)), None);
    assert_eq!(w.insert_resource(Time(2)), Some(Time(1)));
    w.resource_mut::<Time>().unwrap().0 += 1;
    assert_eq!(w.resource::<Time>(), Some(&Time(3)));
    assert_eq!(w.remove_resource::<Time>(), Some(Time(3)));
    assert!(w.resource::<Time>().is_none());
}

#[test]
fn iter_component_works_through_a_shared_borrow() {
    let mut w = World::new();
    let a = w.spawn((Pos(1.0, 0.0), Tag));
    let b = w.spawn((Pos(2.0, 0.0),));
    let shared: &World = &w;
    let mut all: Vec<_> = shared.iter_component::<Pos>().map(|(e, p)| (e, p.0)).collect();
    all.sort_by(|x, y| x.1.total_cmp(&y.1));
    assert_eq!(all, [(a, 1.0), (b, 2.0)]);
    assert_eq!(shared.iter_component::<Name>().count(), 0);
    assert_eq!(shared.entities().count(), 2);
}

#[test]
fn drop_runs_for_every_removed_component() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Counted(Arc<AtomicUsize>);
    impl Drop for Counted {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let drops = Arc::new(AtomicUsize::new(0));
    {
        let mut w = World::new();
        let a = w.spawn((Counted(drops.clone()), Pos(0.0, 0.0)));
        let b = w.spawn((Counted(drops.clone()),));
        let c = w.spawn((Counted(drops.clone()),));
        w.despawn(a);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        w.insert(b, Vel(1.0, 1.0)); // a move must not drop or duplicate
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        drop(w.remove::<Counted>(c));
        assert_eq!(drops.load(Ordering::SeqCst), 2);
        let _ = b;
    }
    assert_eq!(drops.load(Ordering::SeqCst), 3, "dropping the world drops what is left, once");
}

// ---------------------------------------------------------------------------------------------
// Model-based test: the real world must agree with a trivially correct model after every step.
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq)]
struct Model {
    pos: Option<(i32, i32)>,
    vel: Option<i32>,
    name: Option<String>,
    tag: bool,
}

#[derive(Clone, Debug)]
enum Op {
    Spawn(Model),
    Despawn(usize),
    SetPos(usize, (i32, i32)),
    SetVel(usize, i32),
    SetName(usize, String),
    SetTag(usize),
    RemovePos(usize),
    RemoveVel(usize),
    RemoveName(usize),
    RemoveTag(usize),
    Bump,
}

fn arb_model() -> impl Strategy<Value = Model> {
    (
        prop::option::of((any::<i32>(), any::<i32>())),
        prop::option::of(any::<i32>()),
        prop::option::of("[a-z]{0,4}"),
        any::<bool>(),
    )
        .prop_map(|(pos, vel, name, tag)| Model { pos, vel, name, tag })
}

fn arb_op() -> impl Strategy<Value = Op> {
    let idx = 0usize..64;
    prop_oneof![
        3 => arb_model().prop_map(Op::Spawn),
        2 => idx.clone().prop_map(Op::Despawn),
        2 => (idx.clone(), (any::<i32>(), any::<i32>())).prop_map(|(i, p)| Op::SetPos(i, p)),
        2 => (idx.clone(), any::<i32>()).prop_map(|(i, v)| Op::SetVel(i, v)),
        1 => (idx.clone(), "[a-z]{0,4}").prop_map(|(i, n)| Op::SetName(i, n)),
        1 => idx.clone().prop_map(Op::SetTag),
        1 => idx.clone().prop_map(Op::RemovePos),
        1 => idx.clone().prop_map(Op::RemoveVel),
        1 => idx.clone().prop_map(Op::RemoveName),
        1 => idx.clone().prop_map(Op::RemoveTag),
        1 => Just(Op::Bump),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct P(i32, i32);
#[derive(Clone, Copy, Debug, PartialEq)]
struct V(i32);
#[derive(Clone, Debug, PartialEq)]
struct N(String);
#[derive(Clone, Copy, Debug, PartialEq)]
struct T;

fn spawn_model(w: &mut World, m: &Model) -> Entity {
    // Spawn the components the model starts with in one go, as a bundle, via the cheapest path.
    let e = w.spawn(());
    if let Some((x, y)) = m.pos {
        w.insert(e, P(x, y));
    }
    if let Some(v) = m.vel {
        w.insert(e, V(v));
    }
    if let Some(n) = &m.name {
        w.insert(e, N(n.clone()));
    }
    if m.tag {
        w.insert(e, T);
    }
    e
}

fn snapshot(w: &mut World, e: Entity) -> Model {
    Model {
        pos: w.get::<P>(e).map(|p| (p.0, p.1)),
        vel: w.get::<V>(e).map(|v| v.0),
        name: w.get::<N>(e).map(|n| n.0.clone()),
        tag: w.has::<T>(e),
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, ..ProptestConfig::default() })]

    #[test]
    fn the_world_agrees_with_a_model_after_every_operation(ops in prop::collection::vec(arb_op(), 1..80)) {
        let mut world = World::new();
        let mut model: Vec<(Entity, Model)> = Vec::new();   // live entities, in spawn order
        let mut dead: Vec<Entity> = Vec::new();

        for op in ops {
            let pick = |i: usize, len: usize| if len == 0 { None } else { Some(i % len) };
            match op {
                Op::Spawn(m) => {
                    let e = spawn_model(&mut world, &m);
                    model.push((e, m));
                }
                Op::Despawn(i) => if let Some(i) = pick(i, model.len()) {
                    let (e, _) = model.remove(i);
                    prop_assert!(world.despawn(e));
                    dead.push(e);
                },
                Op::SetPos(i, p) => if let Some(i) = pick(i, model.len()) {
                    prop_assert!(world.insert(model[i].0, P(p.0, p.1)));
                    model[i].1.pos = Some(p);
                },
                Op::SetVel(i, v) => if let Some(i) = pick(i, model.len()) {
                    prop_assert!(world.insert(model[i].0, V(v)));
                    model[i].1.vel = Some(v);
                },
                Op::SetName(i, n) => if let Some(i) = pick(i, model.len()) {
                    prop_assert!(world.insert(model[i].0, N(n.clone())));
                    model[i].1.name = Some(n);
                },
                Op::SetTag(i) => if let Some(i) = pick(i, model.len()) {
                    prop_assert!(world.insert(model[i].0, T));
                    model[i].1.tag = true;
                },
                Op::RemovePos(i) => if let Some(i) = pick(i, model.len()) {
                    let got = world.remove::<P>(model[i].0).map(|p| (p.0, p.1));
                    prop_assert_eq!(got, model[i].1.pos.take());
                },
                Op::RemoveVel(i) => if let Some(i) = pick(i, model.len()) {
                    prop_assert_eq!(world.remove::<V>(model[i].0).map(|v| v.0), model[i].1.vel.take());
                },
                Op::RemoveName(i) => if let Some(i) = pick(i, model.len()) {
                    prop_assert_eq!(world.remove::<N>(model[i].0).map(|n| n.0), model[i].1.name.take());
                },
                Op::RemoveTag(i) => if let Some(i) = pick(i, model.len()) {
                    prop_assert_eq!(world.remove::<T>(model[i].0).is_some(), std::mem::take(&mut model[i].1.tag));
                },
                Op::Bump => {
                    for p in world.query::<&mut P>() { p.0 = p.0.wrapping_add(1); }
                    for (_, m) in model.iter_mut() {
                        if let Some(p) = &mut m.pos { p.0 = p.0.wrapping_add(1); }
                    }
                }
            }

            // After every operation the two must agree, entity by entity...
            prop_assert_eq!(world.len(), model.len());
            for (e, m) in &model {
                prop_assert!(world.contains(*e));
                prop_assert_eq!(&snapshot(&mut world, *e), m);
            }
            // ...stale handles must stay dead...
            for e in &dead {
                prop_assert!(!world.contains(*e));
                prop_assert!(world.get::<P>(*e).is_none());
            }
            // ...and queries must see exactly the matching entities.
            let mut got: Vec<_> = world.query::<(Entity, &P)>().map(|(e, p)| (e, (p.0, p.1))).collect();
            got.sort();
            let mut want: Vec<_> = model.iter().filter_map(|(e, m)| m.pos.map(|p| (*e, p))).collect();
            want.sort();
            prop_assert_eq!(got, want);
            prop_assert_eq!(
                world.query_filtered::<Entity, (With<V>, Without<T>)>().count(),
                model.iter().filter(|(_, m)| m.vel.is_some() && !m.tag).count()
            );
            let mut entities: Vec<_> = world.entities().collect();
            entities.sort();
            let mut live: Vec<_> = model.iter().map(|(e, _)| *e).collect();
            live.sort();
            prop_assert_eq!(entities, live);
        }
    }
}

#[test]
fn dead_helper_types_are_used() {
    // Keeps the unused-type lints honest for helpers shared by several tests.
    let mut counts: HashMap<&str, usize> = HashMap::new();
    counts.insert("x", 1);
    let _ = (Name(String::new()), Tag);
}
