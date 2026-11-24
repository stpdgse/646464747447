use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// Anything that can be stored on an entity. `Send + Sync` so that whole worlds can move between
/// threads, which is what region-parallel ticking relies on.
pub trait Component: 'static + Send + Sync {}
impl<T: 'static + Send + Sync> Component for T {}

/// A dense per-archetype id for a component type, assigned in registration order.
pub type ComponentId = usize;

/// Type-erased column of one component type, one value per entity row.
pub trait Column: Any + Send + Sync {
    fn len(&self) -> usize;
    fn swap_remove_drop(&mut self, row: usize);
    fn swap_remove_boxed(&mut self, row: usize) -> Box<dyn Any>;
    /// Moves `row` into `dst` (same concrete type), preserving the order of the other rows the way
    /// `swap_remove` does.
    fn move_row_to(&mut self, row: usize, dst: &mut dyn Column);
    fn push_boxed(&mut self, value: Box<dyn Any>);
    fn empty_like(&self) -> Box<dyn Column>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Component> Column for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
    fn swap_remove_drop(&mut self, row: usize) {
        drop(self.swap_remove(row));
    }
    fn swap_remove_boxed(&mut self, row: usize) -> Box<dyn Any> {
        Box::new(self.swap_remove(row))
    }
    fn move_row_to(&mut self, row: usize, dst: &mut dyn Column) {
        let dst = dst.as_any_mut().downcast_mut::<Vec<T>>().expect("column type mismatch");
        dst.push(self.swap_remove(row));
    }
    fn push_boxed(&mut self, value: Box<dyn Any>) {
        self.push(*value.downcast::<T>().expect("component type mismatch"));
    }
    fn empty_like(&self) -> Box<dyn Column> {
        Box::new(Vec::<T>::new())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl std::fmt::Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry").field("components", &self.prototypes.len()).finish()
    }
}

/// `TypeId` is already a well-mixed hash, so hashing it again with SipHash is wasted work. This
/// hasher just folds its bytes. It is only used for point lookups; no iteration order observable
/// from outside this crate ever depends on it.
#[derive(Default)]
pub struct TypeIdHasher(u64);

impl Hasher for TypeIdHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut b = [0u8; 8];
            b[..chunk.len()].copy_from_slice(chunk);
            self.0 ^= u64::from_le_bytes(b);
        }
    }
    fn write_u64(&mut self, v: u64) {
        self.0 ^= v;
    }
    fn write_u128(&mut self, v: u128) {
        self.0 ^= (v as u64) ^ ((v >> 64) as u64);
    }
}

pub type TypeIdMap<V> = HashMap<TypeId, V, BuildHasherDefault<TypeIdHasher>>;

#[derive(Default)]
pub struct Registry {
    by_type: TypeIdMap<ComponentId>,
    /// An empty column per registered type, cloned when a new archetype needs one.
    prototypes: Vec<Box<dyn Column>>,
}

impl Registry {
    pub fn id_of<T: Component>(&self) -> Option<ComponentId> {
        self.by_type.get(&TypeId::of::<T>()).copied()
    }

    pub fn register<T: Component>(&mut self) -> ComponentId {
        *self.by_type.entry(TypeId::of::<T>()).or_insert_with(|| {
            self.prototypes.push(Box::new(Vec::<T>::new()));
            self.prototypes.len() - 1
        })
    }

    pub fn new_column(&self, id: ComponentId) -> Box<dyn Column> {
        self.prototypes[id].empty_like()
    }
}
