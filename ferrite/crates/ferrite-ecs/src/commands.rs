use crate::bundle::Bundle;
use crate::component::Component;
use crate::entity::Entity;
use crate::world::World;

type Op = Box<dyn FnOnce(&mut World) + Send>;

/// Structural changes recorded while a query is borrowing the world, applied afterwards with
/// [`World::apply`] in exactly the order they were recorded.
#[derive(Default)]
pub struct Commands {
    queue: Vec<Op>,
}

impl std::fmt::Debug for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Commands({} pending)", self.queue.len())
    }
}

impl Commands {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn spawn<B: Bundle>(&mut self, bundle: B) {
        self.queue.push(Box::new(move |w| {
            w.spawn(bundle);
        }));
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.queue.push(Box::new(move |w| {
            w.despawn(entity);
        }));
    }

    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) {
        self.queue.push(Box::new(move |w| {
            w.insert(entity, value);
        }));
    }

    pub fn remove<T: Component>(&mut self, entity: Entity) {
        self.queue.push(Box::new(move |w| {
            w.remove::<T>(entity);
        }));
    }

    /// Runs arbitrary world mutation at apply time.
    pub fn push(&mut self, f: impl FnOnce(&mut World) + Send + 'static) {
        self.queue.push(Box::new(f));
    }

    pub(crate) fn drain(&mut self) -> std::vec::Drain<'_, Op> {
        self.queue.drain(..)
    }
}
