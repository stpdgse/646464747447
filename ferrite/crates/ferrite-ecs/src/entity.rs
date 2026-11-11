/// A handle to an entity. Handles are generational: a handle to a despawned entity is detected,
/// even after its slot has been reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl Entity {
    /// The slot index. Stable for the life of the entity; not unique over time.
    pub fn index(self) -> u32 {
        self.index
    }

    pub fn generation(self) -> u32 {
        self.generation
    }

    /// A handle that never refers to a live entity.
    pub const PLACEHOLDER: Entity = Entity { index: u32::MAX, generation: u32::MAX };
}

/// Where an entity currently lives.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Location {
    pub archetype: usize,
    pub row: usize,
}

#[derive(Debug)]
pub(crate) struct Slot {
    pub generation: u32,
    pub location: Option<Location>,
}
