//! Data the server sends while a client is in the configuration phase.
//!
//! Vanilla clients that already have the `minecraft:core` data pack (version-matched) only need
//! the *names* of synchronized registry entries, in id order, plus the resolved tags.

pub use crate::generated::configuration::{SYNCHRONIZED_REGISTRIES, TAG_REGISTRIES};

/// A registry whose contents are synchronized to the client with `registry_data`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncRegistry {
    /// Registry key, e.g. `minecraft:dimension_type`.
    pub id: &'static str,
    /// Entry names; an entry's numeric id is its index here.
    pub entries: &'static [&'static str],
}

/// All tags of one registry, as sent in `update_tags`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TagRegistry {
    pub registry: &'static str,
    pub tags: &'static [TagDef],
}

/// One tag with `#references` already expanded to numeric ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TagDef {
    pub name: &'static str,
    pub entries: &'static [u16],
}
