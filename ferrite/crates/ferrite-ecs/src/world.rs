use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::archetype::Archetype;
use crate::bundle::Bundle;
use crate::commands::Commands;
use crate::component::{Column, Component, ComponentId, Registry, TypeIdMap};
use crate::entity::{Entity, Location, Slot};
use crate::query::{Fetch, Filter, Query};

/// Entities, their components, and global resources.
///
/// A world is `Send`: it is meant to be owned by one thread at a time. Parallelism comes from
/// running many independent worlds (regions) at once, never from sharing one.
pub struct World {
    slots: Vec<Slot>,
    free: Vec<u32>,
    registry: Registry,
    archetypes: Vec<Archetype>,
    /// Sorted component ids -> archetype index.
    archetype_index: HashMap<Vec<ComponentId>, usize>,
    resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    /// Per bundle type: where its components go, resolved once.
    bundles: TypeIdMap<BundleInfo>,
    len: usize,
}

struct BundleInfo {
    archetype: usize,
    /// Column index in that archetype for each tuple element.
    columns: Vec<usize>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("entities", &self.len)
            .field("archetypes", &self.archetypes.len())
            .finish()
    }
}

/// Mutable access to two distinct elements of a slice.
fn two_mut<T>(v: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    assert_ne!(a, b);
    if a < b {
        let (l, r) = v.split_at_mut(b);
        (&mut l[a], &mut r[0])
    } else {
        let (l, r) = v.split_at_mut(a);
        (&mut r[0], &mut l[b])
    }
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            slots: Vec::new(),
            free: Vec::new(),
            registry: Registry::default(),
            archetypes: Vec::new(),
            archetype_index: HashMap::new(),
            resources: HashMap::new(),
            bundles: TypeIdMap::default(),
            len: 0,
        };
        // Archetype 0 is always "no components".
        world.archetype_for(Vec::new());
        world
    }

    /// Number of live entities.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether `entity` refers to a live entity (stale handles are rejected).
    pub fn contains(&self, entity: Entity) -> bool {
        self.location(entity).is_some()
    }

    fn location(&self, entity: Entity) -> Option<Location> {
        let slot = self.slots.get(entity.index as usize)?;
        if slot.generation != entity.generation {
            return None;
        }
        slot.location
    }

    fn archetype_for(&mut self, ids: Vec<ComponentId>) -> usize {
        if let Some(&i) = self.archetype_index.get(&ids) {
            return i;
        }
        let columns = ids.iter().map(|&id| self.registry.new_column(id)).collect();
        self.archetypes.push(Archetype {
            ids: ids.clone(),
            columns,
            entities: Vec::new(),
            add_edges: Vec::new(),
            remove_edges: Vec::new(),
        });
        let index = self.archetypes.len() - 1;
        self.archetype_index.insert(ids, index);
        index
    }

    fn alloc_slot(&mut self) -> Entity {
        match self.free.pop() {
            Some(index) => Entity { index, generation: self.slots[index as usize].generation },
            None => {
                let index = u32::try_from(self.slots.len()).expect("more than u32::MAX entities");
                self.slots.push(Slot { generation: 0, location: None });
                Entity { index, generation: 0 }
            }
        }
    }

    /// Creates an entity with the given components.
    ///
    /// # Panics
    /// If the bundle names the same component type twice.
    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        let key = TypeId::of::<B>();
        if !self.bundles.contains_key(&key) {
            let in_order = B::component_ids(&mut self.registry);
            let mut sorted = in_order.clone();
            sorted.sort_unstable();
            assert!(
                sorted.windows(2).all(|w| w[0] != w[1]),
                "a bundle may contain each component once"
            );
            let columns = in_order
                .iter()
                .map(|id| sorted.binary_search(id).expect("id is in its own sorted list"))
                .collect();
            let archetype = self.archetype_for(sorted);
            self.bundles.insert(key, BundleInfo { archetype, columns });
        }
        let entity = self.alloc_slot();
        let info = &self.bundles[&key];
        let arch = info.archetype;
        let archetype = &mut self.archetypes[arch];
        let row = archetype.entities.len();
        bundle.push_into(archetype, &info.columns);
        archetype.entities.push(entity);
        self.slots[entity.index as usize].location = Some(Location { archetype: arch, row });
        self.len += 1;
        entity
    }

    /// Destroys an entity. Returns `false` if the handle was stale.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        let Some(loc) = self.location(entity) else { return false };
        if let Some(moved) = self.archetypes[loc.archetype].swap_remove_drop(loc.row) {
            self.slots[moved.index as usize].location = Some(loc);
        }
        let slot = &mut self.slots[entity.index as usize];
        slot.location = None;
        // Bumping the generation invalidates every outstanding handle to this slot.
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(entity.index);
        self.len -= 1;
        true
    }

    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        let loc = self.location(entity)?;
        let id = self.registry.id_of::<T>()?;
        let archetype = &self.archetypes[loc.archetype];
        let pos = archetype.position(id)?;
        archetype.columns[pos].as_any().downcast_ref::<Vec<T>>()?.get(loc.row)
    }

    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let loc = self.location(entity)?;
        let id = self.registry.id_of::<T>()?;
        let archetype = &mut self.archetypes[loc.archetype];
        let pos = archetype.position(id)?;
        archetype.columns[pos].as_any_mut().downcast_mut::<Vec<T>>()?.get_mut(loc.row)
    }

    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    /// The archetype reached from `from` by adding component `id`, cached after the first time.
    fn add_target(&mut self, from: usize, id: ComponentId) -> usize {
        if let Some(to) = Archetype::edge(&self.archetypes[from].add_edges, id) {
            return to;
        }
        let mut ids = self.archetypes[from].ids.clone();
        ids.push(id);
        ids.sort_unstable();
        let to = self.archetype_for(ids);
        Archetype::set_edge(&mut self.archetypes[from].add_edges, id, to);
        to
    }

    /// The archetype reached from `from` by removing component `id`, cached after the first time.
    fn remove_target(&mut self, from: usize, id: ComponentId) -> usize {
        if let Some(to) = Archetype::edge(&self.archetypes[from].remove_edges, id) {
            return to;
        }
        let ids = self.archetypes[from].ids.iter().copied().filter(|&c| c != id).collect();
        let to = self.archetype_for(ids);
        Archetype::set_edge(&mut self.archetypes[from].remove_edges, id, to);
        to
    }

    /// Moves `entity` from `loc` to archetype `dst_index`, carrying every component except `skip`.
    /// `skipped` receives the skipped column (to take its value); `added` runs on the destination
    /// before the entity is registered there (to push a new value). No value is ever boxed.
    fn move_entity(
        &mut self,
        entity: Entity,
        loc: Location,
        dst_index: usize,
        skip: Option<ComponentId>,
        skipped: impl FnOnce(&mut dyn Column, usize),
        added: impl FnOnce(&mut Archetype),
    ) {
        debug_assert_ne!(dst_index, loc.archetype);
        let (src, dst) = two_mut(&mut self.archetypes, loc.archetype, dst_index);
        let mut skipped = Some(skipped);
        for (i, &id) in src.ids.iter().enumerate() {
            if Some(id) == skip {
                (skipped.take().expect("only one column is skipped"))(
                    &mut *src.columns[i],
                    loc.row,
                );
            } else {
                let j = dst.position(id).expect("destination keeps every other component");
                let (from, to) = (&mut src.columns[i], &mut dst.columns[j]);
                from.move_row_to(loc.row, &mut **to);
            }
        }
        added(dst);
        src.entities.swap_remove(loc.row);
        let moved_into_hole = src.entities.get(loc.row).copied();
        dst.entities.push(entity);
        let new_row = dst.entities.len() - 1;

        if let Some(moved) = moved_into_hole {
            self.slots[moved.index as usize].location = Some(loc);
        }
        self.slots[entity.index as usize].location =
            Some(Location { archetype: dst_index, row: new_row });
    }

    /// Adds `value`, or replaces it if the entity already has a `T`. Returns `false` if the
    /// handle was stale.
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) -> bool {
        let Some(loc) = self.location(entity) else { return false };
        let id = self.registry.register::<T>();
        if self.archetypes[loc.archetype].has(id) {
            *self.get_mut::<T>(entity).expect("component is present") = value;
            return true;
        }
        let dst = self.add_target(loc.archetype, id);
        let mut value = Some(value);
        self.move_entity(
            entity,
            loc,
            dst,
            None,
            |_, _| unreachable!(),
            |dst| {
                let pos = dst.position(id).expect("destination has the added component");
                dst.columns[pos]
                    .as_any_mut()
                    .downcast_mut::<Vec<T>>()
                    .expect("column type mismatch")
                    .push(value.take().expect("added once"));
            },
        );
        true
    }

    /// Removes and returns the entity's `T`, if it has one.
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let loc = self.location(entity)?;
        let id = self.registry.id_of::<T>()?;
        if !self.archetypes[loc.archetype].has(id) {
            return None;
        }
        let dst = self.remove_target(loc.archetype, id);
        let mut removed = None;
        self.move_entity(
            entity,
            loc,
            dst,
            Some(id),
            |column, row| {
                let v = column.as_any_mut().downcast_mut::<Vec<T>>().expect("column type mismatch");
                removed = Some(v.swap_remove(row));
            },
            |_| {},
        );
        removed
    }

    /// Iterates the entities matching `Q`, e.g. `world.query::<(&Position, &mut Velocity)>()`.
    ///
    /// # Panics
    /// If `Q` names the same component more than once.
    pub fn query<Q: Fetch>(&mut self) -> Query<'_, Q> {
        self.query_filtered::<Q, ()>()
    }

    /// [`query`](Self::query) restricted by a [`Filter`] such as `With<A>` or `Without<B>`.
    pub fn query_filtered<Q: Fetch, F: Filter>(&mut self) -> Query<'_, Q> {
        let Some(state) = Q::prepare(&self.registry) else { return Query::new(Vec::new()) };
        let mut parts = Vec::new();
        for archetype in self.archetypes.iter_mut() {
            if archetype.entities.is_empty()
                || !Q::matches(&state, archetype)
                || !F::passes(&self.registry, archetype)
            {
                continue;
            }
            let Archetype { ids, columns, entities, .. } = archetype;
            let mut slots: Vec<Option<&mut dyn Column>> =
                columns.iter_mut().map(|c| Some(&mut **c as &mut dyn Column)).collect();
            parts.push(Q::init(&state, ids, &mut slots, entities));
        }
        Query::new(parts)
    }

    /// Every live entity, archetype by archetype in creation order and row by row.
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.archetypes.iter().flat_map(|a| a.entities.iter().copied())
    }

    /// Read-only iteration over every `T` (a shared borrow of the world is enough for this).
    pub fn iter_component<T: Component>(&self) -> impl Iterator<Item = (Entity, &T)> + '_ {
        let id = self.registry.id_of::<T>();
        self.archetypes
            .iter()
            .filter_map(move |a| {
                let pos = a.position(id?)?;
                let column = a.columns[pos].as_any().downcast_ref::<Vec<T>>()?;
                Some(a.entities.iter().copied().zip(column.iter()))
            })
            .flatten()
    }

    /// Applies recorded commands in order.
    pub fn apply(&mut self, commands: &mut Commands) {
        for op in commands.drain() {
            op(self);
        }
    }

    pub fn insert_resource<R: Any + Send + Sync>(&mut self, value: R) -> Option<R> {
        let old = self.resources.insert(TypeId::of::<R>(), Box::new(value))?;
        old.downcast::<R>().ok().map(|b| *b)
    }

    pub fn resource<R: Any + Send + Sync>(&self) -> Option<&R> {
        self.resources.get(&TypeId::of::<R>())?.downcast_ref()
    }

    pub fn resource_mut<R: Any + Send + Sync>(&mut self) -> Option<&mut R> {
        self.resources.get_mut(&TypeId::of::<R>())?.downcast_mut()
    }

    pub fn remove_resource<R: Any + Send + Sync>(&mut self) -> Option<R> {
        self.resources.remove(&TypeId::of::<R>())?.downcast().ok().map(|b| *b)
    }
}
