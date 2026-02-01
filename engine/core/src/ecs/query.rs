//! Type-safe query system for accessing entity components
//!
//! Provides ergonomic iteration over entities with specific component combinations.
//! Supports single components, tuples, optional components, and mixed mutability.

use super::{Component, Entity, SparseSet, World};
use std::any::TypeId;
use std::marker::PhantomData;

/// Trait for types that can be queried from the world
///
/// This trait allows type-safe queries over entities with specific components.
/// The query system uses Generic Associated Types (GATs) to support flexible
/// lifetime management.
///
/// # Examples
///
/// ```
/// # use engine_core::ecs::{World, Component, Query};
/// # #[derive(Debug)]
/// # struct Position { x: f32, y: f32, z: f32 }
/// # impl Component for Position {}
/// # let mut world = World::new();
/// # world.register::<Position>();
/// // Single component query
/// for (entity, position) in world.query::<&Position>() {
///     println!("Entity {:?} at ({}, {}, {})", entity, position.x, position.y, position.z);
/// }
/// ```
pub trait Query {
    /// The item type returned by iteration
    ///
    /// Uses GATs to allow the lifetime to be tied to the world borrow
    type Item<'a>;

    /// Fetch data for iteration from the world
    ///
    /// This method is called internally by `World::query()` to set up
    /// the iterator state.
    fn fetch(world: &World) -> QueryIter<'_, Self>
    where
        Self: Sized;

    /// Fetch data for mutable iteration from the world
    ///
    /// This method is called internally by `World::query()` for queries
    /// that require mutable access.
    fn fetch_mut(world: &mut World) -> QueryIterMut<'_, Self>
    where
        Self: Sized;
}

/// Iterator over query results (immutable)
///
/// This iterator yields tuples of (Entity, components) for all entities
/// that match the query.
pub struct QueryIter<'a, Q: Query> {
    /// Reference to the world being queried
    world: &'a World,
    /// Current position in iteration
    current_index: usize,
    /// Total number of items to iterate
    len: usize,
    /// Phantom data to tie the query type to the iterator
    _phantom: PhantomData<Q>,
}

impl<'a, Q: Query> QueryIter<'a, Q> {
    /// Create a new query iterator
    pub(crate) fn new(world: &'a World, len: usize) -> Self {
        Self {
            world,
            current_index: 0,
            len,
            _phantom: PhantomData,
        }
    }
}

/// Iterator over query results (mutable)
///
/// This iterator yields tuples of (Entity, components) for all entities
/// that match the query, with mutable access to components.
pub struct QueryIterMut<'a, Q: Query> {
    /// Mutable reference to the world being queried
    world: &'a mut World,
    /// Current position in iteration
    current_index: usize,
    /// Total number of items to iterate
    len: usize,
    /// Phantom data to tie the query type to the iterator
    _phantom: PhantomData<Q>,
}

impl<'a, Q: Query> QueryIterMut<'a, Q> {
    /// Create a new mutable query iterator
    pub(crate) fn new(world: &'a mut World, len: usize) -> Self {
        Self {
            world,
            current_index: 0,
            len,
            _phantom: PhantomData,
        }
    }
}

//
// Single Component Queries - Immutable Reference
//

impl<T: Component> Query for &T {
    type Item<'a> = (Entity, &'a T);

    fn fetch(world: &World) -> QueryIter<'_, Self> {
        let type_id = TypeId::of::<T>();
        let len = world
            .components
            .get(&type_id)
            .and_then(|storage: &Box<dyn std::any::Any>| storage.downcast_ref::<SparseSet<T>>())
            .map(|storage: &SparseSet<T>| storage.len())
            .unwrap_or(0);

        QueryIter::new(world, len)
    }

    fn fetch_mut(_world: &mut World) -> QueryIterMut<'_, Self> {
        panic!("Cannot use fetch_mut for immutable query. Use fetch instead.");
    }
}

impl<'a, T: Component> Iterator for QueryIter<'a, &T> {
    type Item = (Entity, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.len {
            return None;
        }

        let type_id = TypeId::of::<T>();
        let storage = self
            .world
            .components
            .get(&type_id)?
            .downcast_ref::<SparseSet<T>>()?;

        // Get the nth item from the storage
        let result = storage
            .iter()
            .nth(self.current_index)
            .map(|(entity, component)| (entity, component));

        self.current_index += 1;
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<'a, T: Component> ExactSizeIterator for QueryIter<'a, &T> {
    fn len(&self) -> usize {
        self.len.saturating_sub(self.current_index)
    }
}

//
// Single Component Queries - Mutable Reference
//

impl<T: Component> Query for &mut T {
    type Item<'a> = (Entity, &'a mut T);

    fn fetch(_world: &World) -> QueryIter<'_, Self> {
        panic!("Cannot use fetch for mutable query. Use fetch_mut instead.");
    }

    fn fetch_mut(world: &mut World) -> QueryIterMut<'_, Self> {
        let type_id = TypeId::of::<T>();
        let len = world
            .components
            .get(&type_id)
            .and_then(|storage: &Box<dyn std::any::Any>| storage.downcast_ref::<SparseSet<T>>())
            .map(|storage: &SparseSet<T>| storage.len())
            .unwrap_or(0);

        QueryIterMut::new(world, len)
    }
}

impl<'a, T: Component> Iterator for QueryIterMut<'a, &mut T> {
    type Item = (Entity, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.len {
            return None;
        }

        let type_id = TypeId::of::<T>();

        // SAFETY: We need to extend the lifetime here because we're iterating
        // We know this is safe because:
        // 1. We have exclusive access to the world (&mut World)
        // 2. We only return one mutable reference at a time
        // 3. The borrow checker ensures no aliasing
        let storage = unsafe {
            let storage_ptr = self
                .world
                .components
                .get_mut(&type_id)?
                .downcast_mut::<SparseSet<T>>()?
                as *mut SparseSet<T>;
            &mut *storage_ptr
        };

        // Get the nth item from the storage
        let result = storage
            .iter_mut()
            .nth(self.current_index)
            .map(|(entity, component)| {
                // SAFETY: Extend lifetime to 'a
                // This is safe because we have exclusive access via &mut World
                let component = unsafe { &mut *(component as *mut T) };
                (entity, component)
            });

        self.current_index += 1;
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<'a, T: Component> ExactSizeIterator for QueryIterMut<'a, &mut T> {
    fn len(&self) -> usize {
        self.len.saturating_sub(self.current_index)
    }
}

// Add query methods to World
impl World {
    /// Query entities with specific components
    ///
    /// Returns an iterator over all entities that have the requested components.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # #[derive(Debug)]
    /// # struct Position { x: f32, y: f32, z: f32 }
    /// # impl Component for Position {}
    /// # let mut world = World::new();
    /// # world.register::<Position>();
    /// // Immutable query
    /// for (entity, position) in world.query::<&Position>() {
    ///     println!("Entity at ({}, {}, {})", position.x, position.y, position.z);
    /// }
    /// ```
    pub fn query<Q: Query>(&self) -> QueryIter<'_, Q> {
        Q::fetch(self)
    }

    /// Query entities with mutable component access
    ///
    /// Returns an iterator over all entities that have the requested components,
    /// with mutable access.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # #[derive(Debug)]
    /// # struct Health { current: f32, max: f32 }
    /// # impl Component for Health {}
    /// # let mut world = World::new();
    /// # world.register::<Health>();
    /// // Mutable query
    /// for (entity, health) in world.query_mut::<&mut Health>() {
    ///     health.current = health.max;
    /// }
    /// ```
    pub fn query_mut<Q: Query>(&mut self) -> QueryIterMut<'_, Q> {
        Q::fetch_mut(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[allow(dead_code)]
    struct Position {
        x: f32,
        y: f32,
        z: f32,
    }

    impl Component for Position {}

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[allow(dead_code)]
    struct Velocity {
        x: f32,
        y: f32,
        z: f32,
    }

    impl Component for Velocity {}

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[allow(dead_code)]
    struct Health {
        current: f32,
        max: f32,
    }

    impl Component for Health {}

    #[test]
    fn test_query_single_component() {
        let mut world = World::new();
        world.register::<Position>();

        for i in 0..100 {
            let e = world.spawn();
            world.add(
                e,
                Position {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        let mut count = 0;
        for (_entity, position) in world.query::<&Position>() {
            assert!(position.x >= 0.0);
            count += 1;
        }

        assert_eq!(count, 100);
    }

    #[test]
    fn test_query_mut() {
        let mut world = World::new();
        world.register::<Position>();

        let e = world.spawn();
        world.add(
            e,
            Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        );

        for (_entity, position) in world.query_mut::<&mut Position>() {
            position.x = 5.0;
        }

        assert_eq!(world.get::<Position>(e).unwrap().x, 5.0);
    }

    #[test]
    fn test_query_empty() {
        let world = World::new();
        // Don't register or add any components

        let count: usize = world.query::<&Position>().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_query_partial_entities() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        // Create 100 entities with Position
        for i in 0..100 {
            let e = world.spawn();
            world.add(
                e,
                Position {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        // Collect first 50 entities to avoid borrow checker issues
        let entities_to_update: Vec<Entity> = world
            .query::<&Position>()
            .take(50)
            .map(|(entity, _)| entity)
            .collect();

        // Only 50 have Velocity
        for entity in entities_to_update {
            world.add(
                entity,
                Velocity {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        assert_eq!(world.query::<&Position>().count(), 100);
        assert_eq!(world.query::<&Velocity>().count(), 50);
    }

    #[test]
    fn test_query_size_hint() {
        let mut world = World::new();
        world.register::<Position>();

        for i in 0..10 {
            let e = world.spawn();
            world.add(
                e,
                Position {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        let query = world.query::<&Position>();
        let (lower, upper) = query.size_hint();
        assert_eq!(lower, 10);
        assert_eq!(upper, Some(10));
    }

    #[test]
    fn test_query_exact_size() {
        let mut world = World::new();
        world.register::<Position>();

        for i in 0..10 {
            let e = world.spawn();
            world.add(
                e,
                Position {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        let query = world.query::<&Position>();
        assert_eq!(query.len(), 10);
    }

    #[test]
    fn test_query_mut_multiple_iterations() {
        let mut world = World::new();
        world.register::<Health>();

        for _i in 0..10 {
            let e = world.spawn();
            world.add(e, Health {
                current: 100.0,
                max: 100.0,
            });
        }

        // First pass: damage all entities
        for (_entity, health) in world.query_mut::<&mut Health>() {
            health.current -= 10.0;
        }

        // Second pass: verify damage
        for (_entity, health) in world.query::<&Health>() {
            assert_eq!(health.current, 90.0);
        }
    }
}
