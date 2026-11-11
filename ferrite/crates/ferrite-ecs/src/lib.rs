//! A small archetype ECS.
//!
//! Design goals, in order:
//!
//! 1. **Deterministic.** Given the same sequence of operations the world ends in the same state,
//!    with the same iteration order, on every run and on every machine. Nothing here uses hashing
//!    order, pointer values, or thread timing to decide anything observable.
//! 2. **Safe.** The whole crate is `#![forbid(unsafe_code)]`. Queries hand out disjoint borrows by
//!    *taking* each column out of a slot table, so asking for the same component twice panics
//!    instead of aliasing.
//! 3. **Fast to iterate.** Components live in one `Vec<T>` per archetype and a query is a zip of
//!    slice iterators, which the compiler turns into a tight loop.
//!
//! Parallelism is not inside the world: a [`World`] is `Send` and is meant to be owned by *one*
//! region at a time, with regions ticking in parallel (see the `ferrite-sim` crate). That keeps the
//! hot path free of synchronisation and makes scheduling order irrelevant to results.

#![forbid(unsafe_code)]

mod archetype;
mod bundle;
mod commands;
mod component;
mod entity;
mod query;
mod world;

pub use bundle::Bundle;
pub use commands::Commands;
pub use component::Component;
pub use entity::Entity;
pub use query::{Fetch, Filter, Query, With, Without};
pub use world::World;
