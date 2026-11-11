//! Block behaviour.
//!
//! [`World`] owns loaded blocks and what makes them change: vanilla's neighbour- and shape-update
//! propagation, scheduled ticks, and (so far) water and lava. The rules are implemented from
//! observation of the official server and are compared against recordings of it in the tests;
//! geometry comes from [`ferrite_data::shapes`].

#![forbid(unsafe_code)]

pub mod fluid;
mod generated_placement;
mod kinds;
mod level;
mod piston;
mod placement;
mod pos;
mod redstone;
pub mod shape;
mod survival;
pub mod ticks;
mod wall;
mod world;

pub use kinds::{Behaviour, Kinds, Piston, Redstone};
pub use level::{BlockAccess, Chunks};
pub use placement::{Formula, PlaceContext, horizontal_direction, looking_directions};
pub use pos::{BlockPos, Direction};
pub use world::{NEIGHBOR_UPDATE_ORDER, Rules, SHAPE_UPDATE_ORDER, World, update};
