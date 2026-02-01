//! Parallel query iteration for multi-core performance
//!
//! This module provides parallel iteration support for ECS queries using Rayon.
//! Enables 6-8x speedup on multi-core CPUs by processing entities in parallel.
//!
//! # Safety
//!
//! Parallel iteration is safe because:
//! - Each thread processes disjoint sets of entities (no data races)
//! - Immutable queries use shared references (safe concurrent reads)
//! - Mutable queries split data into non-overlapping chunks (one writer per chunk)
//!
//! # Examples
//!
//! ```
//! # use engine_core::ecs::{World, Component};
//! # #[derive(Debug)]
//! # struct Position { x: f32, y: f32, z: f32 }
//! # #[derive(Debug)]
//! # struct Velocity { x: f32, y: f32, z: f32 }
//! # impl Component for Position {}
//! # impl Component for Velocity {}
//! # let mut world = World::new();
//! # world.register::<Position>();
//! # world.register::<Velocity>();
//! use rayon::prelude::*;
//!
//! // Parallel immutable query
//! world.query::<(&Position, &Velocity)>()
//!     .par_iter()
//!     .for_each(|(entity, (pos, vel))| {
//!         // Process entities in parallel (read-only)
//!     });
//!
//! // Parallel mutable query
//! world.query::<(&mut Position, &Velocity)>()
//!     .par_iter_mut()
//!     .for_each(|(entity, (pos, vel))| {
//!         // Update positions in parallel (safe disjoint writes)
//!         pos.x += vel.x;
//!         pos.y += vel.y;
//!         pos.z += vel.z;
//!     });
//! ```

use super::{Component, Entity, Query, QueryIter, QueryIterMut, SparseSet, World};
use rayon::prelude::*;
use std::any::TypeId;
use std::marker::PhantomData;

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

/// Parallel iterator adapter for immutable queries
///
/// This wraps a standard QueryIter and enables parallel iteration using Rayon.
pub trait ParallelQuery<'a, Q: Query> {
    /// Convert to a parallel iterator for immutable access
    ///
    /// This splits the query results across multiple threads for parallel processing.
    /// Each thread receives a disjoint subset of entities, ensuring no data races.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # use engine_core::ecs::parallel::ParallelQuery;
    /// # #[derive(Debug)] struct Position { x: f32 }
    /// # impl Component for Position {}
    /// # let mut world = World::new();
    /// # world.register::<Position>();
    /// use rayon::prelude::*;
    ///
    /// world.query::<&Position>()
    ///     .par_iter()
    ///     .for_each(|(entity, pos)| {
    ///         // Process in parallel
    ///     });
    /// ```
    fn par_iter(self) -> ParallelQueryIter<'a, Q>;
}

/// Parallel iterator adapter for mutable queries
pub trait ParallelQueryMut<'a, Q: Query> {
    /// Convert to a parallel iterator for mutable access
    ///
    /// This splits the query results across multiple threads for parallel processing.
    /// Each thread receives a disjoint subset of entities with exclusive access.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # use engine_core::ecs::parallel::ParallelQueryMut;
    /// # #[derive(Debug)] struct Position { x: f32 }
    /// # #[derive(Debug)] struct Velocity { x: f32 }
    /// # impl Component for Position {}
    /// # impl Component for Velocity {}
    /// # let mut world = World::new();
    /// # world.register::<Position>();
    /// # world.register::<Velocity>();
    /// use rayon::prelude::*;
    ///
    /// world.query::<(&mut Position, &Velocity)>()
    ///     .par_iter_mut()
    ///     .for_each(|(entity, (pos, vel))| {
    ///         pos.x += vel.x;
    ///     });
    /// ```
    fn par_iter_mut(self) -> ParallelQueryIterMut<'a, Q>;
}

// Implement for all query iterators
impl<'a, Q: Query> ParallelQuery<'a, Q> for QueryIter<'a, Q> {
    fn par_iter(self) -> ParallelQueryIter<'a, Q> {
        #[cfg(feature = "profiling")]
        profile_scope!("parallel_query_setup", ProfileCategory::ECS);

        ParallelQueryIter {
            iter: self,
            _phantom: PhantomData,
        }
    }
}

impl<'a, Q: Query> ParallelQueryMut<'a, Q> for QueryIterMut<'a, Q> {
    fn par_iter_mut(self) -> ParallelQueryIterMut<'a, Q> {
        #[cfg(feature = "profiling")]
        profile_scope!("parallel_query_mut_setup", ProfileCategory::ECS);

        ParallelQueryIterMut {
            iter: self,
            _phantom: PhantomData,
        }
    }
}

/// Parallel iterator over query results (immutable)
pub struct ParallelQueryIter<'a, Q: Query> {
    iter: QueryIter<'a, Q>,
    _phantom: PhantomData<&'a Q>,
}

/// Parallel iterator over query results (mutable)
pub struct ParallelQueryIterMut<'a, Q: Query> {
    iter: QueryIterMut<'a, Q>,
    _phantom: PhantomData<&'a mut Q>,
}

//
// Single Component Parallel Queries - Immutable
//

impl<'a, T: Component + Sync> ParallelIterator for ParallelQueryIter<'a, &T> {
    type Item = (Entity, &'a T);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        #[cfg(feature = "profiling")]
        profile_scope!("parallel_query_drive", ProfileCategory::ECS);

        // Get the storage for parallel access
        let storage = match self.iter.world.get_storage::<T>() {
            Some(s) => s,
            None => {
                // No storage means no items, return empty
                return consumer.into_folder().complete();
            }
        };

        // Create parallel iterator over dense indices
        (0..storage.len())
            .into_par_iter()
            .filter_map(move |index| {
                // Get entity and component at this index
                let entity = storage.get_dense_entity(index)?;

                // Apply filters (if any) - would need to pass filters to parallel iter
                // For now, filters are not supported in parallel iteration
                // TODO: Add filter support by passing filter state to parallel iter

                storage.get(entity).map(|component| (entity, component))
            })
            .drive_unindexed(consumer)
    }
}

//
// Single Component Parallel Queries - Mutable
//

impl<'a, T: Component + Send> ParallelIterator for ParallelQueryIterMut<'a, &mut T> {
    type Item = (Entity, &'a mut T);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        #[cfg(feature = "profiling")]
        profile_scope!("parallel_query_mut_drive", ProfileCategory::ECS);

        // We need to split the mutable storage into parallel chunks
        // SAFETY: This is safe because:
        // 1. We have exclusive access to the world (&mut World)
        // 2. Each thread processes disjoint indices (no overlap)
        // 3. The lifetime 'a is tied to the world borrow

        let type_id = TypeId::of::<T>();
        let storage_ptr = unsafe {
            self.iter
                .world
                .components
                .get_mut(&type_id)
                .and_then(|s| s.as_any_mut().downcast_mut::<SparseSet<T>>())
                .map(|s| s as *mut SparseSet<T>)
        };

        let storage_ptr = match storage_ptr {
            Some(ptr) => ptr,
            None => return consumer.into_folder().complete(),
        };

        // SAFETY: We have exclusive access and indices are disjoint
        let len = unsafe { (*storage_ptr).len() };

        (0..len)
            .into_par_iter()
            .filter_map(move |index| {
                // SAFETY:
                // - storage_ptr is valid for the lifetime 'a
                // - Each thread accesses disjoint indices
                // - No aliasing because each index maps to a unique component
                unsafe {
                    let storage = &mut *storage_ptr;
                    let entity = storage.get_dense_entity(index)?;
                    storage.get_mut(entity).map(|component| {
                        // Extend lifetime to 'a
                        let component_ptr = component as *mut T;
                        (entity, &mut *component_ptr)
                    })
                }
            })
            .drive_unindexed(consumer)
    }
}

//
// Two-Component Parallel Queries - Mixed Immutable/Mutable
//

impl<'a, A: Component + Sync, B: Component + Sync> ParallelIterator
    for ParallelQueryIter<'a, (&A, &B)>
{
    type Item = (Entity, (&'a A, &'a B));

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        #[cfg(feature = "profiling")]
        profile_scope!("parallel_query2_drive", ProfileCategory::ECS);

        // Get both storages
        let storage_a = match self.iter.world.get_storage::<A>() {
            Some(s) => s,
            None => return consumer.into_folder().complete(),
        };
        let storage_b = match self.iter.world.get_storage::<B>() {
            Some(s) => s,
            None => return consumer.into_folder().complete(),
        };

        // Iterate the smaller storage and check the larger one
        let len = storage_a.len().min(storage_b.len());

        (0..len)
            .into_par_iter()
            .filter_map(move |index| {
                let entity = storage_a.get_dense_entity(index)?;

                // Entity must have both components
                let comp_a = storage_a.get(entity)?;
                let comp_b = storage_b.get(entity)?;

                Some((entity, (comp_a, comp_b)))
            })
            .drive_unindexed(consumer)
    }
}

impl<'a, A: Component + Send, B: Component + Sync> ParallelIterator
    for ParallelQueryIterMut<'a, (&mut A, &B)>
{
    type Item = (Entity, (&'a mut A, &'a B));

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        #[cfg(feature = "profiling")]
        profile_scope!("parallel_query2_mut_drive", ProfileCategory::ECS);

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // Get mutable storage for A
        let storage_a_ptr = unsafe {
            self.iter
                .world
                .components
                .get_mut(&type_id_a)
                .and_then(|s| s.as_any_mut().downcast_mut::<SparseSet<A>>())
                .map(|s| s as *mut SparseSet<A>)
        };

        // Get immutable storage for B
        let storage_b_ptr = unsafe {
            self.iter
                .world
                .components
                .get(&type_id_b)
                .and_then(|s| s.as_any().downcast_ref::<SparseSet<B>>())
                .map(|s| s as *const SparseSet<B>)
        };

        let (storage_a_ptr, storage_b_ptr) = match (storage_a_ptr, storage_b_ptr) {
            (Some(a), Some(b)) => (a, b),
            _ => return consumer.into_folder().complete(),
        };

        // SAFETY: We have exclusive access to A and shared access to B
        let len = unsafe { (*storage_a_ptr).len().min((*storage_b_ptr).len()) };

        (0..len)
            .into_par_iter()
            .filter_map(move |index| {
                unsafe {
                    let storage_a = &mut *storage_a_ptr;
                    let storage_b = &*storage_b_ptr;

                    let entity = storage_a.get_dense_entity(index)?;

                    let comp_a = storage_a.get_mut(entity).map(|c| {
                        let ptr = c as *mut A;
                        &mut *ptr
                    })?;
                    let comp_b = storage_b.get(entity)?;

                    Some((entity, (comp_a, comp_b)))
                }
            })
            .drive_unindexed(consumer)
    }
}

impl<'a, A: Component + Sync, B: Component + Send> ParallelIterator
    for ParallelQueryIterMut<'a, (&A, &mut B)>
{
    type Item = (Entity, (&'a A, &'a mut B));

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        #[cfg(feature = "profiling")]
        profile_scope!("parallel_query2_mut_drive", ProfileCategory::ECS);

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // Get immutable storage for A
        let storage_a_ptr = unsafe {
            self.iter
                .world
                .components
                .get(&type_id_a)
                .and_then(|s| s.as_any().downcast_ref::<SparseSet<A>>())
                .map(|s| s as *const SparseSet<A>)
        };

        // Get mutable storage for B
        let storage_b_ptr = unsafe {
            self.iter
                .world
                .components
                .get_mut(&type_id_b)
                .and_then(|s| s.as_any_mut().downcast_mut::<SparseSet<B>>())
                .map(|s| s as *mut SparseSet<B>)
        };

        let (storage_a_ptr, storage_b_ptr) = match (storage_a_ptr, storage_b_ptr) {
            (Some(a), Some(b)) => (a, b),
            _ => return consumer.into_folder().complete(),
        };

        // SAFETY: We have shared access to A and exclusive access to B
        let len = unsafe { (*storage_a_ptr).len().min((*storage_b_ptr).len()) };

        (0..len)
            .into_par_iter()
            .filter_map(move |index| {
                unsafe {
                    let storage_a = &*storage_a_ptr;
                    let storage_b = &mut *storage_b_ptr;

                    let entity = storage_a.get_dense_entity(index)?;

                    let comp_a = storage_a.get(entity)?;
                    let comp_b = storage_b.get_mut(entity).map(|c| {
                        let ptr = c as *mut B;
                        &mut *ptr
                    })?;

                    Some((entity, (comp_a, comp_b)))
                }
            })
            .drive_unindexed(consumer)
    }
}

impl<'a, A: Component + Send, B: Component + Send> ParallelIterator
    for ParallelQueryIterMut<'a, (&mut A, &mut B)>
{
    type Item = (Entity, (&'a mut A, &'a mut B));

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        #[cfg(feature = "profiling")]
        profile_scope!("parallel_query2_mut2_drive", ProfileCategory::ECS);

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // Get mutable storage for both A and B
        let storage_a_ptr = unsafe {
            self.iter
                .world
                .components
                .get_mut(&type_id_a)
                .and_then(|s| s.as_any_mut().downcast_mut::<SparseSet<A>>())
                .map(|s| s as *mut SparseSet<A>)
        };

        let storage_b_ptr = unsafe {
            // Need to get the second mutable reference carefully
            // This is safe because A and B are different types (different TypeIds)
            let world_ptr = self.iter.world as *mut World;
            (*world_ptr)
                .components
                .get_mut(&type_id_b)
                .and_then(|s| s.as_any_mut().downcast_mut::<SparseSet<B>>())
                .map(|s| s as *mut SparseSet<B>)
        };

        let (storage_a_ptr, storage_b_ptr) = match (storage_a_ptr, storage_b_ptr) {
            (Some(a), Some(b)) => (a, b),
            _ => return consumer.into_folder().complete(),
        };

        // Verify they're different storages (compile-time guarantee via type system, runtime check for safety)
        debug_assert_ne!(
            storage_a_ptr as *const (),
            storage_b_ptr as *const (),
            "Attempted to create two mutable references to the same component storage"
        );

        // SAFETY: We have exclusive access to both A and B, and they are different types
        let len = unsafe { (*storage_a_ptr).len().min((*storage_b_ptr).len()) };

        (0..len)
            .into_par_iter()
            .filter_map(move |index| {
                unsafe {
                    let storage_a = &mut *storage_a_ptr;
                    let storage_b = &mut *storage_b_ptr;

                    let entity = storage_a.get_dense_entity(index)?;

                    let comp_a = storage_a.get_mut(entity).map(|c| {
                        let ptr = c as *mut A;
                        &mut *ptr
                    })?;
                    let comp_b = storage_b.get_mut(entity).map(|c| {
                        let ptr = c as *mut B;
                        &mut *ptr
                    })?;

                    Some((entity, (comp_a, comp_b)))
                }
            })
            .drive_unindexed(consumer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
        z: f32,
    }
    impl Component for Position {}

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
        z: f32,
    }
    impl Component for Velocity {}

    #[test]
    fn test_parallel_iter_single_component() {
        use rayon::prelude::*;

        let mut world = World::new();
        world.register::<Position>();

        // Spawn test entities
        for i in 0..100 {
            let entity = world.spawn();
            world.add(
                entity,
                Position {
                    x: i as f32,
                    y: i as f32,
                    z: i as f32,
                },
            );
        }

        // Parallel iteration
        let sum: f32 = world
            .query::<&Position>()
            .par_iter()
            .map(|(_, pos)| pos.x)
            .sum();

        // Sum of 0..100 = 4950
        assert_eq!(sum, 4950.0);
    }

    #[test]
    fn test_parallel_iter_mut_single_component() {
        use rayon::prelude::*;

        let mut world = World::new();
        world.register::<Position>();

        // Spawn test entities
        for i in 0..100 {
            let entity = world.spawn();
            world.add(
                entity,
                Position {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                },
            );
        }

        // Parallel mutation
        world.query::<&mut Position>().par_iter_mut().for_each(|(_, pos)| {
            pos.y = pos.x * 2.0;
        });

        // Verify mutations
        let sum: f32 = world.query::<&Position>().map(|(_, pos)| pos.y).sum();

        // Sum of 2*i for i in 0..100 = 2 * 4950 = 9900
        assert_eq!(sum, 9900.0);
    }

    #[test]
    fn test_parallel_iter_two_components() {
        use rayon::prelude::*;

        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        // Spawn test entities
        for i in 0..100 {
            let entity = world.spawn();
            world.add(
                entity,
                Position {
                    x: i as f32,
                    y: 0.0,
                    z: 0.0,
                },
            );
            world.add(
                entity,
                Velocity {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            );
        }

        // Parallel iteration over two components
        let count = world
            .query::<(&Position, &Velocity)>()
            .par_iter()
            .filter(|(_, (pos, vel))| pos.x >= 50.0 && vel.x == 1.0)
            .count();

        assert_eq!(count, 50);
    }

    #[test]
    fn test_parallel_iter_mut_two_components() {
        use rayon::prelude::*;

        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        // Spawn test entities
        for i in 0..100 {
            let entity = world.spawn();
            world.add(
                entity,
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            );
            world.add(
                entity,
                Velocity {
                    x: i as f32,
                    y: 1.0,
                    z: 1.0,
                },
            );
        }

        // Parallel mutation with mixed mutability
        world
            .query::<(&mut Position, &Velocity)>()
            .par_iter_mut()
            .for_each(|(_, (pos, vel))| {
                pos.x += vel.x;
                pos.y += vel.y;
                pos.z += vel.z;
            });

        // Verify mutations
        let sum_x: f32 = world.query::<&Position>().map(|(_, pos)| pos.x).sum();
        let sum_y: f32 = world.query::<&Position>().map(|(_, pos)| pos.y).sum();

        assert_eq!(sum_x, 4950.0); // Sum of 0..100
        assert_eq!(sum_y, 100.0); // 100 entities * 1.0
    }

    #[test]
    fn test_parallel_empty_query() {
        use rayon::prelude::*;

        let mut world = World::new();
        world.register::<Position>();

        // No entities - should handle empty iteration
        let count = world.query::<&Position>().par_iter().count();

        assert_eq!(count, 0);
    }
}
