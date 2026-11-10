//! The game server.
//!
//! * [`chunkenc`]: chunks to and from the network format (sections, heightmaps, light).
//! * [`flat`]: a flat-world generator (the real world generator is a later milestone).
//!
//! Players, the tick loop and block interaction build on these.

#![forbid(unsafe_code)]

pub mod chunkenc;
pub mod flat;

use ferrite_data::configuration::SYNCHRONIZED_REGISTRIES;

/// Index of `name` in a synchronized registry, i.e. the id the client assigns it (entries are
/// sent in this order during configuration).
pub fn synchronized_id(registry: &str, name: &str) -> Option<u32> {
    SYNCHRONIZED_REGISTRIES
        .iter()
        .find(|r| r.id == registry)?
        .entries
        .iter()
        .position(|e| *e == name)
        .map(|i| i as u32)
}
