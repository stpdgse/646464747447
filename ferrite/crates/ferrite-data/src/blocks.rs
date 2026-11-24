//! Block and block-state tables.
//!
//! Block states are numbered contiguously per block. Within a block, the state offset is a
//! mixed-radix number over the block's properties **sorted by name**, with the **last property
//! varying fastest** and values in the order listed. `ferrite-datagen` asserts this layout for
//! every block of every generated version, so [`BlockInfo::state_id`] is exact.

pub use crate::generated::blocks::{BLOCKS, BY_NAME, STATE_COUNT, STATE_TO_BLOCK};

/// One block property and its possible values, in state-numbering order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Property {
    pub name: &'static str,
    pub values: &'static [&'static str],
}

/// Static description of a block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockInfo {
    /// Registry name, e.g. `minecraft:stone`.
    pub name: &'static str,
    /// Protocol id in the `minecraft:block` registry (also the index into [`BLOCKS`]).
    pub id: u16,
    /// First block-state id owned by this block.
    pub first_state: u16,
    /// Number of states (product of the property value counts).
    pub state_count: u16,
    /// State id of the default state.
    pub default_state: u16,
    /// Properties sorted by name, i.e. in state-numbering order.
    pub properties: &'static [Property],
}

impl BlockInfo {
    /// Last state id owned by this block (inclusive).
    #[inline]
    pub const fn last_state(&self) -> u16 {
        self.first_state + self.state_count - 1
    }

    /// Computes a state id from one value index per property (in property order).
    ///
    /// Returns `None` if the slice length differs from the property count or an index is out of
    /// range.
    pub fn state_id(&self, value_indices: &[u8]) -> Option<u16> {
        if value_indices.len() != self.properties.len() {
            return None;
        }
        let mut offset = 0u32;
        for (prop, &i) in self.properties.iter().zip(value_indices) {
            if usize::from(i) >= prop.values.len() {
                return None;
            }
            offset = offset * prop.values.len() as u32 + u32::from(i);
        }
        Some(self.first_state + offset as u16)
    }

    /// Inverse of [`state_id`](Self::state_id): writes one value index per property into `out`
    /// and returns how many were written. `None` if `state` does not belong to this block or
    /// `out` is too small.
    pub fn value_indices(&self, state: u16, out: &mut [u8]) -> Option<usize> {
        if state < self.first_state
            || state > self.last_state()
            || out.len() < self.properties.len()
        {
            return None;
        }
        let mut rest = u32::from(state - self.first_state);
        for (prop, slot) in self.properties.iter().zip(out.iter_mut()).rev() {
            let n = prop.values.len() as u32;
            *slot = (rest % n) as u8;
            rest /= n;
        }
        Some(self.properties.len())
    }

    /// Looks up a property by name.
    pub fn property(&self, name: &str) -> Option<(usize, &'static Property)> {
        self.properties.iter().enumerate().find(|(_, p)| p.name == name)
    }
}

/// Returns the block that owns `state`.
#[inline]
pub fn block_of_state(state: u16) -> Option<&'static BlockInfo> {
    let id = *STATE_TO_BLOCK.get(usize::from(state))?;
    BLOCKS.get(usize::from(id))
}

/// Returns a block by registry name (with or without the `minecraft:` prefix).
pub fn block_by_name(name: &str) -> Option<&'static BlockInfo> {
    let id = BY_NAME.get(name.strip_prefix("minecraft:").unwrap_or(name))?;
    BLOCKS.get(usize::from(*id))
}
