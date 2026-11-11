use crate::component::{Column, ComponentId};
use crate::entity::Entity;

impl std::fmt::Debug for Archetype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Archetype")
            .field("components", &self.ids)
            .field("len", &self.entities.len())
            .finish()
    }
}

/// All entities that have exactly the same set of components, stored column by column.
pub struct Archetype {
    /// Sorted component ids; `columns[i]` holds component `ids[i]`.
    #[doc(hidden)]
    pub ids: Vec<ComponentId>,
    pub columns: Vec<Box<dyn Column>>,
    pub entities: Vec<Entity>,
    /// Cached transitions: `add_edges[c]` is the archetype reached by adding component `c`.
    #[doc(hidden)]
    pub add_edges: Vec<usize>,
    #[doc(hidden)]
    pub remove_edges: Vec<usize>,
}

pub(crate) const NO_EDGE: usize = usize::MAX;

impl Archetype {
    pub fn position(&self, id: ComponentId) -> Option<usize> {
        self.ids.binary_search(&id).ok()
    }

    pub fn has(&self, id: ComponentId) -> bool {
        self.position(id).is_some()
    }

    pub fn edge(edges: &[usize], id: ComponentId) -> Option<usize> {
        edges.get(id).copied().filter(|&e| e != NO_EDGE)
    }

    pub fn set_edge(edges: &mut Vec<usize>, id: ComponentId, to: usize) {
        if edges.len() <= id {
            edges.resize(id + 1, NO_EDGE);
        }
        edges[id] = to;
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Removes `row`, dropping its components. Returns the entity that was moved into `row` to
    /// fill the hole, if any.
    pub fn swap_remove_drop(&mut self, row: usize) -> Option<Entity> {
        for c in &mut self.columns {
            c.swap_remove_drop(row);
        }
        self.entities.swap_remove(row);
        self.entities.get(row).copied()
    }
}
