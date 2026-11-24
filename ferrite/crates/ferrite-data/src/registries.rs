//! Typed access to the generated registries.
//!
//! Each registry lives in its own module under [`crate::generated::registries`] and exposes
//! `names` (indexed by protocol id), `by_name` (a perfect hash map), `registry` and `count`, plus
//! one SHOUTY_CASE `const` per entry. The lowercase metadata can never collide with an entry.

pub use crate::generated::registries::*;

/// Common interface implemented by every generated registry module via [`Registry`].
#[derive(Clone, Copy, Debug)]
pub struct Registry {
    /// Registry key, e.g. `minecraft:item`.
    pub name: &'static str,
    /// Protocol id of the registry itself (used by the registry-sync packets).
    pub protocol_id: u32,
    /// Entry names, indexed by protocol id.
    pub entries: &'static [&'static str],
    /// Name to protocol id lookup.
    pub by_name: &'static phf::Map<&'static str, u32>,
    /// Default entry name, if the registry declares one.
    pub default: Option<&'static str>,
}

impl Registry {
    /// Resolves an entry name to its protocol id.
    #[inline]
    pub fn id_of(&self, name: &str) -> Option<u32> {
        self.by_name.get(name).copied()
    }

    /// Resolves a protocol id to its entry name.
    #[inline]
    pub fn name_of(&self, id: u32) -> Option<&'static str> {
        self.entries.get(id as usize).copied()
    }

    /// Finds a registry by its key.
    pub fn find(name: &str) -> Option<&'static Registry> {
        ALL.iter().copied().find(|r| r.name == name)
    }
}
