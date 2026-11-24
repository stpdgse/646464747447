//! Region-parallel simulation with deterministic results.
//!
//! The world is cut into a grid of regions. Each [`Region`] owns an ECS [`World`](ferrite_ecs::World)
//! and nothing else owns any of its data, so regions can tick on different threads with no locks.
//!
//! A tick is a list of *phases*. Within a phase every region runs the same function **in
//! parallel**, and may only touch its own world and push messages to its outbox. Between phases
//! comes an **exchange**: all outboxes are gathered in region order and delivered to inboxes in an
//! order that depends only on the messages themselves, never on which thread finished first.
//!
//! That gives the guarantee this crate exists for: **the result of a tick is identical for any
//! number of threads and any scheduling**, and (for rules written in the usual halo-exchange style
//! shown by [`particles`]) identical for any region layout, matching a plain single-threaded
//! implementation bit for bit.
//!
//! What this does *not* give is vanilla's exact global update order for rules that depend on it
//! (redstone, entity collision pushes). Those need an ordered mode and are designed per system.

mod hash;
mod layout;
pub mod particles;
mod sim;

pub use hash::StableHasher;
pub use layout::{Bounds, Layout, RegionId};
pub use sim::{Region, Sim};
