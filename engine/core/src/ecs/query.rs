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

//
// Two-Component Tuple Queries
//

impl<A: Component, B: Component> Query for (&A, &B) {
    type Item<'a> = (Entity, (&'a A, &'a B));

    fn fetch(world: &World) -> QueryIter<'_, Self> {
        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // Get both storages
        let storage_a = world
            .components
            .get(&type_id_a)
            .and_then(|s| s.downcast_ref::<SparseSet<A>>());

        let storage_b = world
            .components
            .get(&type_id_b)
            .and_then(|s| s.downcast_ref::<SparseSet<B>>());

        // If either storage is missing, return empty iterator
        let (storage_a, storage_b) = match (storage_a, storage_b) {
            (Some(a), Some(b)) => (a, b),
            _ => return QueryIter::new(world, 0),
        };

        // Collect entities that have both components
        // Iterate over the smaller storage for efficiency
        let entities: Vec<Entity> = if storage_a.len() <= storage_b.len() {
            storage_a
                .iter()
                .filter_map(|(entity, _)| {
                    if storage_b.contains(entity) {
                        Some(entity)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            storage_b
                .iter()
                .filter_map(|(entity, _)| {
                    if storage_a.contains(entity) {
                        Some(entity)
                    } else {
                        None
                    }
                })
                .collect()
        };

        let len = entities.len();
        QueryIter::new(world, len)
    }

    fn fetch_mut(_world: &mut World) -> QueryIterMut<'_, Self> {
        panic!("Cannot use fetch_mut for immutable tuple query. Use fetch instead.");
    }
}

impl<'a, A: Component, B: Component> Iterator for QueryIter<'a, (&A, &B)> {
    type Item = (Entity, (&'a A, &'a B));

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.len {
            return None;
        }

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        let storage_a = self
            .world
            .components
            .get(&type_id_a)?
            .downcast_ref::<SparseSet<A>>()?;

        let storage_b = self
            .world
            .components
            .get(&type_id_b)?
            .downcast_ref::<SparseSet<B>>()?;

        // Find next entity that has both components
        loop {
            let (entity, _) = storage_a.iter().nth(self.current_index)?;
            self.current_index += 1;

            if let (Some(comp_a), Some(comp_b)) = (storage_a.get(entity), storage_b.get(entity)) {
                return Some((entity, (comp_a, comp_b)));
            }

            if self.current_index >= storage_a.len() {
                return None;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<'a, A: Component, B: Component> ExactSizeIterator for QueryIter<'a, (&A, &B)> {
    fn len(&self) -> usize {
        self.len.saturating_sub(self.current_index)
    }
}

// Mutable two-component tuple query
impl<A: Component, B: Component> Query for (&mut A, &mut B) {
    type Item<'a> = (Entity, (&'a mut A, &'a mut B));

    fn fetch(_world: &World) -> QueryIter<'_, Self> {
        panic!("Cannot use fetch for mutable tuple query. Use fetch_mut instead.");
    }

    fn fetch_mut(world: &mut World) -> QueryIterMut<'_, Self> {
        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // Get both storages
        let storage_a = world
            .components
            .get(&type_id_a)
            .and_then(|s| s.downcast_ref::<SparseSet<A>>());

        let storage_b = world
            .components
            .get(&type_id_b)
            .and_then(|s| s.downcast_ref::<SparseSet<B>>());

        // If either storage is missing, return empty iterator
        let (storage_a, storage_b) = match (storage_a, storage_b) {
            (Some(a), Some(b)) => (a, b),
            _ => return QueryIterMut::new(world, 0),
        };

        // Count entities that have both components
        let len = if storage_a.len() <= storage_b.len() {
            storage_a
                .iter()
                .filter(|(entity, _)| storage_b.contains(*entity))
                .count()
        } else {
            storage_b
                .iter()
                .filter(|(entity, _)| storage_a.contains(*entity))
                .count()
        };

        QueryIterMut::new(world, len)
    }
}

impl<'a, A: Component, B: Component> Iterator for QueryIterMut<'a, (&mut A, &mut B)> {
    type Item = (Entity, (&'a mut A, &'a mut B));

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.len {
            return None;
        }

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // SAFETY: We need to extend lifetimes here for iteration
        // This is safe because:
        // 1. We have exclusive access to world (&mut World)
        // 2. We return one mutable reference pair at a time
        // 3. The borrow checker ensures no aliasing
        unsafe {
            let storage_a_ptr = self
                .world
                .components
                .get_mut(&type_id_a)?
                .downcast_mut::<SparseSet<A>>()?
                as *mut SparseSet<A>;

            let storage_b_ptr = self
                .world
                .components
                .get_mut(&type_id_b)?
                .downcast_mut::<SparseSet<B>>()?
                as *mut SparseSet<B>;

            let storage_a = &mut *storage_a_ptr;
            let storage_b = &mut *storage_b_ptr;

            // Find next entity that has both components
            loop {
                let (entity, _) = storage_a.iter().nth(self.current_index)?;
                self.current_index += 1;

                if storage_b.contains(entity) {
                    // Get mutable references to both components
                    let comp_a = &mut *(storage_a.get_mut(entity)? as *mut A);
                    let comp_b = &mut *(storage_b.get_mut(entity)? as *mut B);
                    return Some((entity, (comp_a, comp_b)));
                }

                if self.current_index >= storage_a.len() {
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<'a, A: Component, B: Component> ExactSizeIterator for QueryIterMut<'a, (&mut A, &mut B)> {
    fn len(&self) -> usize {
        self.len.saturating_sub(self.current_index)
    }
}

// Mixed mutability: (&A, &mut B)
impl<A: Component, B: Component> Query for (&A, &mut B) {
    type Item<'a> = (Entity, (&'a A, &'a mut B));

    fn fetch(_world: &World) -> QueryIter<'_, Self> {
        panic!("Cannot use fetch for mixed mutability query. Use fetch_mut instead.");
    }

    fn fetch_mut(world: &mut World) -> QueryIterMut<'_, Self> {
        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        let storage_a = world
            .components
            .get(&type_id_a)
            .and_then(|s| s.downcast_ref::<SparseSet<A>>());

        let storage_b = world
            .components
            .get(&type_id_b)
            .and_then(|s| s.downcast_ref::<SparseSet<B>>());

        let (storage_a, storage_b) = match (storage_a, storage_b) {
            (Some(a), Some(b)) => (a, b),
            _ => return QueryIterMut::new(world, 0),
        };

        let len = if storage_a.len() <= storage_b.len() {
            storage_a
                .iter()
                .filter(|(entity, _)| storage_b.contains(*entity))
                .count()
        } else {
            storage_b
                .iter()
                .filter(|(entity, _)| storage_a.contains(*entity))
                .count()
        };

        QueryIterMut::new(world, len)
    }
}

impl<'a, A: Component, B: Component> Iterator for QueryIterMut<'a, (&A, &mut B)> {
    type Item = (Entity, (&'a A, &'a mut B));

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.len {
            return None;
        }

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // SAFETY: We need raw pointers to access both storages
        // This is safe because:
        // 1. We have exclusive access to world (&mut World)
        // 2. A and B are different component types (different TypeIds)
        // 3. We return one reference pair at a time
        unsafe {
            let components_ptr = &mut self.world.components as *mut std::collections::HashMap<TypeId, Box<dyn std::any::Any>>;
            let components = &mut *components_ptr;

            let storage_a_ptr = components
                .get(&type_id_a)?
                .downcast_ref::<SparseSet<A>>()?
                as *const SparseSet<A>;

            let storage_b_ptr = components
                .get_mut(&type_id_b)?
                .downcast_mut::<SparseSet<B>>()?
                as *mut SparseSet<B>;

            let storage_a = &*storage_a_ptr;
            let storage_b = &mut *storage_b_ptr;

            loop {
                let (entity, _) = storage_a.iter().nth(self.current_index)?;
                self.current_index += 1;

                if let (Some(comp_a), Some(comp_b)) =
                    (storage_a.get(entity), storage_b.get_mut(entity))
                {
                    let comp_a_ptr = comp_a as *const A;
                    let comp_b_ptr = comp_b as *mut B;
                    return Some((entity, (&*comp_a_ptr, &mut *comp_b_ptr)));
                }

                if self.current_index >= storage_a.len() {
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<'a, A: Component, B: Component> ExactSizeIterator for QueryIterMut<'a, (&A, &mut B)> {
    fn len(&self) -> usize {
        self.len.saturating_sub(self.current_index)
    }
}

// Mixed mutability: (&mut A, &B)
impl<A: Component, B: Component> Query for (&mut A, &B) {
    type Item<'a> = (Entity, (&'a mut A, &'a B));

    fn fetch(_world: &World) -> QueryIter<'_, Self> {
        panic!("Cannot use fetch for mixed mutability query. Use fetch_mut instead.");
    }

    fn fetch_mut(world: &mut World) -> QueryIterMut<'_, Self> {
        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        let storage_a = world
            .components
            .get(&type_id_a)
            .and_then(|s| s.downcast_ref::<SparseSet<A>>());

        let storage_b = world
            .components
            .get(&type_id_b)
            .and_then(|s| s.downcast_ref::<SparseSet<B>>());

        let (storage_a, storage_b) = match (storage_a, storage_b) {
            (Some(a), Some(b)) => (a, b),
            _ => return QueryIterMut::new(world, 0),
        };

        let len = if storage_a.len() <= storage_b.len() {
            storage_a
                .iter()
                .filter(|(entity, _)| storage_b.contains(*entity))
                .count()
        } else {
            storage_b
                .iter()
                .filter(|(entity, _)| storage_a.contains(*entity))
                .count()
        };

        QueryIterMut::new(world, len)
    }
}

impl<'a, A: Component, B: Component> Iterator for QueryIterMut<'a, (&mut A, &B)> {
    type Item = (Entity, (&'a mut A, &'a B));

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.len {
            return None;
        }

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // SAFETY: We need raw pointers to access both storages
        // This is safe because:
        // 1. We have exclusive access to world (&mut World)
        // 2. A and B are different component types (different TypeIds)
        // 3. We return one reference pair at a time
        unsafe {
            let components_ptr = &mut self.world.components as *mut std::collections::HashMap<TypeId, Box<dyn std::any::Any>>;
            let components = &mut *components_ptr;

            let storage_a_ptr = components
                .get_mut(&type_id_a)?
                .downcast_mut::<SparseSet<A>>()?
                as *mut SparseSet<A>;

            let storage_b_ptr = components
                .get(&type_id_b)?
                .downcast_ref::<SparseSet<B>>()?
                as *const SparseSet<B>;

            let storage_a = &mut *storage_a_ptr;
            let storage_b = &*storage_b_ptr;

            loop {
                let (entity, _) = storage_a.iter().nth(self.current_index)?;
                self.current_index += 1;

                if let (Some(comp_a), Some(comp_b)) =
                    (storage_a.get_mut(entity), storage_b.get(entity))
                {
                    let comp_a_ptr = comp_a as *mut A;
                    let comp_b_ptr = comp_b as *const B;
                    return Some((entity, (&mut *comp_a_ptr, &*comp_b_ptr)));
                }

                if self.current_index >= storage_a.len() {
                    return None;
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

impl<'a, A: Component, B: Component> ExactSizeIterator for QueryIterMut<'a, (&mut A, &B)> {
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

    // ============================================================
    // Two-Component Tuple Query Tests
    // ============================================================

    #[test]
    fn test_query_two_components_immutable() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        // Entity with both components
        let e1 = world.spawn();
        world.add(
            e1,
            Position {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        );
        world.add(
            e1,
            Velocity {
                x: 0.1,
                y: 0.2,
                z: 0.3,
            },
        );

        // Entity with only Position
        let e2 = world.spawn();
        world.add(
            e2,
            Position {
                x: 4.0,
                y: 5.0,
                z: 6.0,
            },
        );

        // Entity with only Velocity
        let e3 = world.spawn();
        world.add(
            e3,
            Velocity {
                x: 0.4,
                y: 0.5,
                z: 0.6,
            },
        );

        // Query should only return e1
        let mut count = 0;
        for (entity, (position, velocity)) in world.query::<(&Position, &Velocity)>() {
            assert_eq!(entity, e1);
            assert_eq!(position.x, 1.0);
            assert_eq!(velocity.x, 0.1);
            count += 1;
        }

        assert_eq!(count, 1);
    }

    #[test]
    fn test_query_two_components_mutable() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        // Create entities with both components
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
            world.add(
                e,
                Velocity {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        // Update positions using velocity
        let dt = 0.016; // ~60 FPS
        for (_entity, (position, velocity)) in world.query_mut::<(&mut Position, &mut Velocity)>() {
            position.x += velocity.x * dt;
            velocity.x *= 0.99; // Damping
        }

        // Verify updates
        let mut verified = 0;
        for (_entity, (position, velocity)) in world.query::<(&Position, &Velocity)>() {
            assert!(position.x > 0.0); // Moved forward
            assert!(velocity.x < 1.0); // Damped
            verified += 1;
        }
        assert_eq!(verified, 10);
    }

    #[test]
    fn test_query_mixed_mutability_immut_mut() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        let e = world.spawn();
        world.add(
            e,
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(
            e,
            Velocity {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        );

        // Read position, modify velocity
        for (_entity, (position, velocity)) in world.query_mut::<(&Position, &mut Velocity)>() {
            velocity.x = position.x + 10.0;
        }

        let velocity = world.get::<Velocity>(e).unwrap();
        assert_eq!(velocity.x, 10.0);
    }

    #[test]
    fn test_query_mixed_mutability_mut_immut() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        let e = world.spawn();
        world.add(
            e,
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        );
        world.add(
            e,
            Velocity {
                x: 5.0,
                y: 0.0,
                z: 0.0,
            },
        );

        // Modify position, read velocity
        for (_entity, (position, velocity)) in world.query_mut::<(&mut Position, &Velocity)>() {
            position.x += velocity.x;
        }

        let position = world.get::<Position>(e).unwrap();
        assert_eq!(position.x, 5.0);
    }

    #[test]
    fn test_query_two_components_many_entities() {
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

        // Only first 50 also have Velocity
        let entities_with_pos: Vec<Entity> = world
            .query::<&Position>()
            .take(50)
            .map(|(e, _)| e)
            .collect();

        for e in entities_with_pos {
            world.add(
                e,
                Velocity {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        // Query both should return 50
        let count = world.query::<(&Position, &Velocity)>().count();
        assert_eq!(count, 50);
    }

    #[test]
    fn test_query_two_components_empty() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        // No entities
        let count = world.query::<(&Position, &Velocity)>().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_query_two_components_no_overlap() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        // Entities with Position only
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

        // Entities with Velocity only
        for _i in 0..10 {
            let e = world.spawn();
            world.add(
                e,
                Velocity {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        // No entities have both
        let count = world.query::<(&Position, &Velocity)>().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_query_two_components_size_hint() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        for i in 0..20 {
            let e = world.spawn();
            world.add(
                e,
                Position {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                },
            );
            world.add(
                e,
                Velocity {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        let query = world.query::<(&Position, &Velocity)>();
        let (lower, upper) = query.size_hint();
        assert_eq!(lower, 20);
        assert_eq!(upper, Some(20));
    }

    #[test]
    fn test_query_physics_simulation() {
        // Realistic physics update scenario
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        // Create moving entities
        for i in 0..5 {
            let e = world.spawn();
            world.add(
                e,
                Position {
                    x: 0.0,
                    y: i as f32 * 10.0,
                    z: 0.0,
                },
            );
            world.add(
                e,
                Velocity {
                    x: (i + 1) as f32,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        // Simulate 10 frames
        let dt = 1.0 / 60.0;
        for _frame in 0..10 {
            for (_e, (pos, vel)) in world.query_mut::<(&mut Position, &Velocity)>() {
                pos.x += vel.x * dt;
                pos.y += vel.y * dt;
                pos.z += vel.z * dt;
            }
        }

        // Verify positions changed
        for (_e, (pos, _vel)) in world.query::<(&Position, &Velocity)>() {
            assert!(pos.x > 0.0, "Entities should have moved");
        }
    }

    #[test]
    fn test_query_health_system() {
        // Realistic health/damage system scenario
        let mut world = World::new();
        world.register::<Health>();
        world.register::<Position>();

        // Create entities with health at various positions
        for i in 0..10 {
            let e = world.spawn();
            world.add(e, Health {
                current: 100.0,
                max: 100.0,
            });
            world.add(
                e,
                Position {
                    x: i as f32 * 10.0,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        // Damage entities in a specific area (x < 50)
        for (_e, (health, pos)) in world.query_mut::<(&mut Health, &Position)>() {
            if pos.x < 50.0 {
                health.current -= 25.0;
            }
        }

        // Count damaged entities
        let mut damaged = 0;
        let mut undamaged = 0;
        for (_e, (health, _pos)) in world.query::<(&Health, &Position)>() {
            if health.current < health.max {
                damaged += 1;
            } else {
                undamaged += 1;
            }
        }

        assert_eq!(damaged, 5); // x: 0, 10, 20, 30, 40
        assert_eq!(undamaged, 5); // x: 50, 60, 70, 80, 90
    }
}
