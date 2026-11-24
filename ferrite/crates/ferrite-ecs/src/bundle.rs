use crate::archetype::Archetype;
use crate::component::{Component, ComponentId, Registry};

/// A set of components spawned together. Implemented for tuples of up to eight components, and
/// for `()` (an entity with no components).
pub trait Bundle: 'static + Send {
    #[doc(hidden)]
    fn component_ids(registry: &mut Registry) -> Vec<ComponentId>;
    /// Pushes each component into its column. `columns[i]` is the column index of the `i`th tuple
    /// element in the archetype.
    #[doc(hidden)]
    fn push_into(self, archetype: &mut Archetype, columns: &[usize]);
}

fn column_mut<T: Component>(archetype: &mut Archetype, column: usize) -> &mut Vec<T> {
    archetype.columns[column].as_any_mut().downcast_mut::<Vec<T>>().expect("column type mismatch")
}

impl Bundle for () {
    fn component_ids(_: &mut Registry) -> Vec<ComponentId> {
        Vec::new()
    }
    fn push_into(self, _: &mut Archetype, _: &[usize]) {}
}

macro_rules! impl_bundle {
    ($(($name:ident, $idx:tt)),+) => {
        impl<$($name: Component),+> Bundle for ($($name,)+) {
            fn component_ids(registry: &mut Registry) -> Vec<ComponentId> {
                vec![$(registry.register::<$name>()),+]
            }
            fn push_into(self, archetype: &mut Archetype, columns: &[usize]) {
                $(column_mut::<$name>(archetype, columns[$idx]).push(self.$idx);)+
            }
        }
    };
}
impl_bundle!((A, 0));
impl_bundle!((A, 0), (B, 1));
impl_bundle!((A, 0), (B, 1), (C, 2));
impl_bundle!((A, 0), (B, 1), (C, 2), (D, 3));
impl_bundle!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
impl_bundle!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
impl_bundle!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
impl_bundle!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6), (H, 7));
