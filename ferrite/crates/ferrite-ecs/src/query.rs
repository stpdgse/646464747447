//! Typed queries.
//!
//! A query is a tuple of fetches (`&T`, `&mut T`, `Option<&T>`, `Option<&mut T>`, [`Entity`]) plus
//! an optional filter ([`With`], [`Without`]). For every matching archetype the fetches become
//! slice iterators that are zipped together, so iterating is a plain loop over dense memory.

use std::marker::PhantomData;

use crate::archetype::Archetype;
use crate::component::{Column, Component, ComponentId, Registry};
use crate::entity::Entity;

/// What a query asks for. Implemented for component references, [`Entity`], `Option`s of those, and
/// tuples of up to eight of them.
pub trait Fetch {
    type Item<'w>;
    type Iter<'w>: Iterator<Item = Self::Item<'w>>;
    #[doc(hidden)]
    type State: Clone;
    #[doc(hidden)]
    fn prepare(registry: &Registry) -> Option<Self::State>;
    #[doc(hidden)]
    fn matches(state: &Self::State, archetype: &Archetype) -> bool;
    #[doc(hidden)]
    fn init<'w>(
        state: &Self::State,
        ids: &[ComponentId],
        slots: &mut [Option<&'w mut dyn Column>],
        entities: &'w [Entity],
    ) -> Self::Iter<'w>;
}

/// Takes the column for `id` out of the slot table. A second request for the same component
/// finds the slot empty, which is how conflicting borrows are caught without `unsafe`.
fn take<'w>(
    ids: &[ComponentId],
    slots: &mut [Option<&'w mut dyn Column>],
    id: ComponentId,
) -> &'w mut dyn Column {
    let pos = ids.binary_search(&id).expect("matched archetype lacks a fetched component");
    slots[pos]
        .take()
        .expect("a query may name each component only once (no `&T` together with `&mut T`)")
}

fn vec_of<T: Component>(column: &mut dyn Column) -> &mut Vec<T> {
    column.as_any_mut().downcast_mut::<Vec<T>>().expect("column type mismatch")
}

impl<T: Component> Fetch for &T {
    type Item<'w> = &'w T;
    type Iter<'w> = std::slice::Iter<'w, T>;
    type State = ComponentId;
    fn prepare(registry: &Registry) -> Option<ComponentId> {
        registry.id_of::<T>()
    }
    fn matches(id: &ComponentId, archetype: &Archetype) -> bool {
        archetype.has(*id)
    }
    fn init<'w>(
        id: &ComponentId,
        ids: &[ComponentId],
        slots: &mut [Option<&'w mut dyn Column>],
        _: &'w [Entity],
    ) -> Self::Iter<'w> {
        let v: &'w Vec<T> = vec_of::<T>(take(ids, slots, *id));
        v.iter()
    }
}

impl<T: Component> Fetch for &mut T {
    type Item<'w> = &'w mut T;
    type Iter<'w> = std::slice::IterMut<'w, T>;
    type State = ComponentId;
    fn prepare(registry: &Registry) -> Option<ComponentId> {
        registry.id_of::<T>()
    }
    fn matches(id: &ComponentId, archetype: &Archetype) -> bool {
        archetype.has(*id)
    }
    fn init<'w>(
        id: &ComponentId,
        ids: &[ComponentId],
        slots: &mut [Option<&'w mut dyn Column>],
        _: &'w [Entity],
    ) -> Self::Iter<'w> {
        vec_of::<T>(take(ids, slots, *id)).iter_mut()
    }
}

impl Fetch for Entity {
    type Item<'w> = Entity;
    type Iter<'w> = std::iter::Copied<std::slice::Iter<'w, Entity>>;
    type State = ();
    fn prepare(_: &Registry) -> Option<()> {
        Some(())
    }
    fn matches(_: &(), _: &Archetype) -> bool {
        true
    }
    fn init<'w>(
        _: &(),
        _: &[ComponentId],
        _: &mut [Option<&'w mut dyn Column>],
        entities: &'w [Entity],
    ) -> Self::Iter<'w> {
        entities.iter().copied()
    }
}

/// Iterator for `Option<&T>` / `Option<&mut T>`: the component's slice, or `None` for every row of
/// an archetype that lacks it.
#[derive(Debug)]
pub enum OptIter<I> {
    Present(I),
    Absent(usize),
}

impl<I: Iterator> Iterator for OptIter<I> {
    type Item = Option<I::Item>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            OptIter::Present(it) => it.next().map(Some),
            OptIter::Absent(0) => None,
            OptIter::Absent(n) => {
                *n -= 1;
                Some(None)
            }
        }
    }
}

impl<T: Component> Fetch for Option<&T> {
    type Item<'w> = Option<&'w T>;
    type Iter<'w> = OptIter<std::slice::Iter<'w, T>>;
    type State = Option<ComponentId>;
    fn prepare(registry: &Registry) -> Option<Self::State> {
        Some(registry.id_of::<T>())
    }
    fn matches(_: &Self::State, _: &Archetype) -> bool {
        true
    }
    fn init<'w>(
        state: &Self::State,
        ids: &[ComponentId],
        slots: &mut [Option<&'w mut dyn Column>],
        entities: &'w [Entity],
    ) -> Self::Iter<'w> {
        match state.filter(|id| ids.binary_search(id).is_ok()) {
            Some(id) => {
                let v: &'w Vec<T> = vec_of::<T>(take(ids, slots, id));
                OptIter::Present(v.iter())
            }
            None => OptIter::Absent(entities.len()),
        }
    }
}

impl<T: Component> Fetch for Option<&mut T> {
    type Item<'w> = Option<&'w mut T>;
    type Iter<'w> = OptIter<std::slice::IterMut<'w, T>>;
    type State = Option<ComponentId>;
    fn prepare(registry: &Registry) -> Option<Self::State> {
        Some(registry.id_of::<T>())
    }
    fn matches(_: &Self::State, _: &Archetype) -> bool {
        true
    }
    fn init<'w>(
        state: &Self::State,
        ids: &[ComponentId],
        slots: &mut [Option<&'w mut dyn Column>],
        entities: &'w [Entity],
    ) -> Self::Iter<'w> {
        match state.filter(|id| ids.binary_search(id).is_ok()) {
            Some(id) => OptIter::Present(vec_of::<T>(take(ids, slots, id)).iter_mut()),
            None => OptIter::Absent(entities.len()),
        }
    }
}

macro_rules! impl_fetch_tuple {
    ($zip:ident; $(($name:ident, $idx:tt)),+) => {
        /// Zips the per-column iterators of a tuple query.
        #[derive(Debug)]
        pub struct $zip<$($name),+>($($name),+);
        impl<$($name: Iterator),+> Iterator for $zip<$($name),+> {
            type Item = ($($name::Item,)+);
            fn next(&mut self) -> Option<Self::Item> {
                Some(($(self.$idx.next()?,)+))
            }
        }
        impl<$($name: Fetch),+> Fetch for ($($name,)+) {
            type Item<'w> = ($($name::Item<'w>,)+);
            type Iter<'w> = $zip<$($name::Iter<'w>),+>;
            type State = ($($name::State,)+);
            fn prepare(registry: &Registry) -> Option<Self::State> {
                Some(($($name::prepare(registry)?,)+))
            }
            fn matches(state: &Self::State, archetype: &Archetype) -> bool {
                $($name::matches(&state.$idx, archetype))&&+
            }
            fn init<'w>(
                state: &Self::State,
                ids: &[ComponentId],
                slots: &mut [Option<&'w mut dyn Column>],
                entities: &'w [Entity],
            ) -> Self::Iter<'w> {
                $zip($($name::init(&state.$idx, ids, slots, entities)),+)
            }
        }
    };
}
impl_fetch_tuple!(Zip1; (A, 0));
impl_fetch_tuple!(Zip2; (A, 0), (B, 1));
impl_fetch_tuple!(Zip3; (A, 0), (B, 1), (C, 2));
impl_fetch_tuple!(Zip4; (A, 0), (B, 1), (C, 2), (D, 3));
impl_fetch_tuple!(Zip5; (A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
impl_fetch_tuple!(Zip6; (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
impl_fetch_tuple!(Zip7; (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
impl_fetch_tuple!(Zip8; (A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7));

/// Restricts which archetypes a query visits, without fetching anything.
pub trait Filter {
    #[doc(hidden)]
    fn passes(registry: &Registry, archetype: &Archetype) -> bool;
}

impl Filter for () {
    fn passes(_: &Registry, _: &Archetype) -> bool {
        true
    }
}

/// Only entities that also have `T`.
pub struct With<T>(PhantomData<T>);
/// Only entities that do *not* have `T`.
pub struct Without<T>(PhantomData<T>);

impl<T> std::fmt::Debug for With<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("With")
    }
}
impl<T> std::fmt::Debug for Without<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Without")
    }
}

impl<T: Component> Filter for With<T> {
    fn passes(registry: &Registry, archetype: &Archetype) -> bool {
        registry.id_of::<T>().is_some_and(|id| archetype.has(id))
    }
}
impl<T: Component> Filter for Without<T> {
    fn passes(registry: &Registry, archetype: &Archetype) -> bool {
        registry.id_of::<T>().is_none_or(|id| !archetype.has(id))
    }
}
macro_rules! impl_filter_tuple {
    ($($name:ident),+) => {
        impl<$($name: Filter),+> Filter for ($($name,)+) {
            fn passes(registry: &Registry, archetype: &Archetype) -> bool {
                $($name::passes(registry, archetype))&&+
            }
        }
    };
}
impl_filter_tuple!(A);
impl_filter_tuple!(A, B);
impl_filter_tuple!(A, B, C);
impl_filter_tuple!(A, B, C, D);

/// The iterator returned by [`World::query`](crate::World::query). Archetypes are visited in the
/// order they were created and rows in storage order, which is deterministic.
pub struct Query<'w, Q: Fetch> {
    parts: std::vec::IntoIter<Q::Iter<'w>>,
    current: Option<Q::Iter<'w>>,
}

impl<Q: Fetch> std::fmt::Debug for Query<'_, Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Query")
    }
}

impl<'w, Q: Fetch> Query<'w, Q> {
    pub(crate) fn new(parts: Vec<Q::Iter<'w>>) -> Self {
        Self { parts: parts.into_iter(), current: None }
    }
}

impl<'w, Q: Fetch> Iterator for Query<'w, Q> {
    type Item = Q::Item<'w>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(it) = &mut self.current
                && let Some(item) = it.next()
            {
                return Some(item);
            }
            self.current = Some(self.parts.next()?);
        }
    }
}
