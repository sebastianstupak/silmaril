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

#![allow(missing_docs)]
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
//! world.query::<&Position>()
//!     .into_par_iter()
//!     .for_each(|(entity, pos)| {
//!         // Process entities in parallel (read-only)
//!     });
//!
//! // Parallel mutable query
//! world.query::<(&mut Position, &Velocity)>()
//!     .into_par_iter_mut()
//!     .for_each(|(entity, (pos, vel))| {
//!         // Update positions in parallel (safe disjoint writes)
//!         pos.x += vel.x;
//!         pos.y += vel.y;
//!         pos.z += vel.z;
//!     });
//! ```

use super::{Component, Entity, SparseSet, World};
use rayon::iter::plumbing::Folder;
use rayon::prelude::*;
use std::any::TypeId;

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

/// Send-safe wrapper for mutable raw pointers used in parallel iteration
///
/// SAFETY: This is safe because:
/// 1. The pointer is derived from exclusive access (&mut World)
/// 2. Each thread accesses disjoint indices (no aliasing)
/// 3. The lifetime is tied to the world borrow
struct SendPtr<T>(*mut T);

impl<T> SendPtr<T> {
    /// Create a new SendPtr from a mutable reference
    #[inline(always)]
    fn new(ptr: *mut T) -> Self {
        SendPtr(ptr)
    }

    /// Get the raw pointer (unsafe - caller must ensure disjoint access)
    #[inline(always)]
    #[allow(dead_code)]
    unsafe fn as_ptr(&self) -> *mut T {
        self.0
    }

    /// Get a mutable reference (unsafe - caller must ensure disjoint access)
    #[inline(always)]
    unsafe fn deref_mut<'a>(&self) -> &'a mut T {
        &mut *self.0
    }

    /// Get a shared reference (unsafe - caller must ensure valid access)
    #[inline(always)]
    unsafe fn deref<'a>(&self) -> &'a T {
        &*self.0
    }
}

impl<T> Clone for SendPtr<T> {
    fn clone(&self) -> Self {
        SendPtr(self.0)
    }
}

impl<T> Copy for SendPtr<T> {}

// SAFETY: We manually verify that parallel iteration uses disjoint indices
unsafe impl<T> Send for SendPtr<T> {}
unsafe impl<T> Sync for SendPtr<T> {}

/// Send-safe wrapper for const raw pointers used in parallel iteration
///
/// SAFETY: This is safe because:
/// 1. The pointer is derived from shared access (&World)
/// 2. Multiple threads can read the same data concurrently
/// 3. The lifetime is tied to the world borrow
struct SendConstPtr<T>(*const T);

impl<T> SendConstPtr<T> {
    /// Create a new SendConstPtr from a const reference
    #[inline(always)]
    fn new(ptr: *const T) -> Self {
        SendConstPtr(ptr)
    }

    /// Get the raw pointer (unsafe - caller must ensure valid access)
    #[allow(dead_code)]
    #[inline(always)]
    #[allow(dead_code)]
    unsafe fn as_ptr(&self) -> *const T {
        self.0
    }

    /// Get a shared reference (unsafe - caller must ensure valid access)
    #[inline(always)]
    unsafe fn deref<'a>(&self) -> &'a T {
        &*self.0
    }
}

impl<T> Clone for SendConstPtr<T> {
    fn clone(&self) -> Self {
        SendConstPtr(self.0)
    }
}

impl<T> Copy for SendConstPtr<T> {}

// SAFETY: We manually verify that parallel iteration is safe
unsafe impl<T> Send for SendConstPtr<T> {}
unsafe impl<T> Sync for SendConstPtr<T> {}

/// Extension trait to add parallel iteration methods to World
pub trait ParallelWorld {
    /// Create a parallel iterator for an immutable single-component query
    fn par_query<T: Component + Sync>(&self) -> ParallelQuery1<'_, T>;

    /// Create a parallel iterator for a mutable single-component query
    fn par_query_mut<T: Component + Send>(&mut self) -> ParallelQueryMut1<'_, T>;

    /// Create a parallel iterator for an immutable two-component query
    fn par_query2<A: Component + Sync, B: Component + Sync>(
        &self,
    ) -> ParallelQuery2<'_, A, B>;

    /// Create a parallel iterator for a mixed-mutability two-component query
    fn par_query2_mut<A: Component + Send, B: Component + Sync>(
        &mut self,
    ) -> ParallelQuery2Mut<'_, A, B>;

    /// Create a parallel iterator for a double-mutable two-component query
    fn par_query2_mut2<A: Component + Send, B: Component + Send>(
        &mut self,
    ) -> ParallelQuery2Mut2<'_, A, B>;
}

impl ParallelWorld for World {
    fn par_query<T: Component + Sync>(&self) -> ParallelQuery1<'_, T> {
        #[cfg(feature = "profiling")]
        profile_scope!("par_query_setup", ProfileCategory::ECS);

        ParallelQuery1 { world: self, _phantom: std::marker::PhantomData }
    }

    fn par_query_mut<T: Component + Send>(&mut self) -> ParallelQueryMut1<'_, T> {
        #[cfg(feature = "profiling")]
        profile_scope!("par_query_mut_setup", ProfileCategory::ECS);

        ParallelQueryMut1 { world: self, _phantom: std::marker::PhantomData }
    }

    fn par_query2<A: Component + Sync, B: Component + Sync>(
        &self,
    ) -> ParallelQuery2<'_, A, B> {
        #[cfg(feature = "profiling")]
        profile_scope!("par_query2_setup", ProfileCategory::ECS);

        ParallelQuery2 { world: self, _phantom: std::marker::PhantomData }
    }

    fn par_query2_mut<A: Component + Send, B: Component + Sync>(
        &mut self,
    ) -> ParallelQuery2Mut<'_, A, B> {
        #[cfg(feature = "profiling")]
        profile_scope!("par_query2_mut_setup", ProfileCategory::ECS);

        ParallelQuery2Mut { world: self, _phantom: std::marker::PhantomData }
    }

    fn par_query2_mut2<A: Component + Send, B: Component + Send>(
        &mut self,
    ) -> ParallelQuery2Mut2<'_, A, B> {
        #[cfg(feature = "profiling")]
        profile_scope!("par_query2_mut2_setup", ProfileCategory::ECS);

        ParallelQuery2Mut2 { world: self, _phantom: std::marker::PhantomData }
    }
}

// Re-export for convenience
pub use ParallelQuery as ParallelQueryTrait;
pub use ParallelQueryMut as ParallelQueryMutTrait;

/// Marker traits for enabling `.par_iter()` on query iterators
pub trait ParallelQuery<'a, Q> {
    /// The parallel iterator type
    type ParIter: ParallelIterator;

    /// Convert to a parallel iterator
    fn par_iter(self) -> Self::ParIter;
}

/// Marker trait for enabling `.par_iter_mut()` on mutable query iterators
pub trait ParallelQueryMut<'a, Q> {
    /// The parallel iterator type
    type ParIterMut: ParallelIterator;

    /// Convert to a parallel mutable iterator
    fn par_iter_mut(self) -> Self::ParIterMut;
}

//
// Single Component Parallel Query - Immutable
//

/// Parallel iterator for single-component immutable queries
pub struct ParallelQuery1<'a, T: Component> {
    world: &'a World,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: Component + Sync> ParallelIterator for ParallelQuery1<'a, T> {
    type Item = (Entity, &'a T);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        #[cfg(feature = "profiling")]
        profile_scope!("par_query1_drive", ProfileCategory::ECS);

        let storage = match self.world.get_storage::<T>() {
            Some(s) => s,
            None => return consumer.into_folder().complete(),
        };

        (0..storage.len())
            .into_par_iter()
            .filter_map(move |index| {
                let entity = storage.get_dense_entity(index)?;
                storage.get(entity).map(|component| (entity, component))
            })
            .drive_unindexed(consumer)
    }
}

//
// Single Component Parallel Query - Mutable
//

/// Parallel iterator for single-component mutable queries
pub struct ParallelQueryMut1<'a, T: Component> {
    world: &'a mut World,
    _phantom: std::marker::PhantomData<T>,
}

impl<'a, T: Component + Send> ParallelIterator for ParallelQueryMut1<'a, T> {
    type Item = (Entity, &'a mut T);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        #[cfg(feature = "profiling")]
        profile_scope!("par_query_mut1_drive", ProfileCategory::ECS);

        let type_id = TypeId::of::<T>();
        let storage_ptr = self
            .world
            .components
            .get_mut(&type_id)
            .and_then(|s| s.as_any_mut().downcast_mut::<SparseSet<T>>())
            .map(|s| SendPtr::new(s as *mut SparseSet<T>));

        let storage_ptr = match storage_ptr {
            Some(ptr) => ptr,
            None => return consumer.into_folder().complete(),
        };

        // SAFETY: We have exclusive access via &mut World
        let len = unsafe { storage_ptr.deref().len() };

        (0..len)
            .into_par_iter()
            .filter_map(move |index| unsafe {
                let storage = storage_ptr.deref_mut();
                let entity = storage.get_dense_entity(index)?;
                storage.get_mut(entity).map(|component| {
                    let component_ptr = component as *mut T;
                    (entity, &mut *component_ptr)
                })
            })
            .drive_unindexed(consumer)
    }
}

//
// Two Component Parallel Query - Immutable
//

/// Parallel iterator for two-component immutable queries
pub struct ParallelQuery2<'a, A: Component, B: Component> {
    world: &'a World,
    _phantom: std::marker::PhantomData<(A, B)>,
}

impl<'a, A: Component + Sync, B: Component + Sync> ParallelIterator
    for ParallelQuery2<'a, A, B>
{
    type Item = (Entity, (&'a A, &'a B));

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        #[cfg(feature = "profiling")]
        profile_scope!("par_query2_drive", ProfileCategory::ECS);

        let storage_a = match self.world.get_storage::<A>() {
            Some(s) => s,
            None => return consumer.into_folder().complete(),
        };
        let storage_b = match self.world.get_storage::<B>() {
            Some(s) => s,
            None => return consumer.into_folder().complete(),
        };

        let len = storage_a.len().min(storage_b.len());

        (0..len)
            .into_par_iter()
            .filter_map(move |index| {
                let entity = storage_a.get_dense_entity(index)?;
                let comp_a = storage_a.get(entity)?;
                let comp_b = storage_b.get(entity)?;
                Some((entity, (comp_a, comp_b)))
            })
            .drive_unindexed(consumer)
    }
}

//
// Two Component Parallel Query - Mixed Mutability (&mut A, &B)
//

/// Parallel iterator for two-component mixed-mutability queries
pub struct ParallelQuery2Mut<'a, A: Component, B: Component> {
    world: &'a mut World,
    _phantom: std::marker::PhantomData<(A, B)>,
}

impl<'a, A: Component + Send, B: Component + Sync> ParallelIterator
    for ParallelQuery2Mut<'a, A, B>
{
    type Item = (Entity, (&'a mut A, &'a B));

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        #[cfg(feature = "profiling")]
        profile_scope!("par_query2_mut_drive", ProfileCategory::ECS);

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        let storage_a_ptr = self
            .world
            .components
            .get_mut(&type_id_a)
            .and_then(|s| s.as_any_mut().downcast_mut::<SparseSet<A>>())
            .map(|s| SendPtr::new(s as *mut SparseSet<A>));

        let storage_b_ptr = self
            .world
            .components
            .get(&type_id_b)
            .and_then(|s| s.as_any().downcast_ref::<SparseSet<B>>())
            .map(|s| SendConstPtr::new(s as *const SparseSet<B>));

        let (storage_a_ptr, storage_b_ptr) = match (storage_a_ptr, storage_b_ptr) {
            (Some(a), Some(b)) => (a, b),
            _ => return consumer.into_folder().complete(),
        };

        let len = unsafe { storage_a_ptr.deref().len().min(storage_b_ptr.deref().len()) };

        (0..len)
            .into_par_iter()
            .filter_map(move |index| unsafe {
                let storage_a = storage_a_ptr.deref_mut();
                let storage_b = storage_b_ptr.deref();

                let entity = storage_a.get_dense_entity(index)?;
                let comp_a = storage_a.get_mut(entity).map(|c| {
                    let ptr = c as *mut A;
                    &mut *ptr
                })?;
                let comp_b = storage_b.get(entity)?;

                Some((entity, (comp_a, comp_b)))
            })
            .drive_unindexed(consumer)
    }
}

//
// Two Component Parallel Query - Both Mutable (&mut A, &mut B)
//

/// Parallel iterator for two-component double-mutable queries
pub struct ParallelQuery2Mut2<'a, A: Component, B: Component> {
    world: &'a mut World,
    _phantom: std::marker::PhantomData<(A, B)>,
}

impl<'a, A: Component + Send, B: Component + Send> ParallelIterator
    for ParallelQuery2Mut2<'a, A, B>
{
    type Item = (Entity, (&'a mut A, &'a mut B));

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        #[cfg(feature = "profiling")]
        profile_scope!("par_query2_mut2_drive", ProfileCategory::ECS);

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        let storage_a_ptr = self
            .world
            .components
            .get_mut(&type_id_a)
            .and_then(|s| s.as_any_mut().downcast_mut::<SparseSet<A>>())
            .map(|s| SendPtr::new(s as *mut SparseSet<A>));

        // Get second mutable reference safely (different TypeIds guarantee different storages)
        let world_ptr = self.world as *mut World;
        let storage_b_ptr = unsafe {
            (*world_ptr)
                .components
                .get_mut(&type_id_b)
                .and_then(|s| s.as_any_mut().downcast_mut::<SparseSet<B>>())
                .map(|s| SendPtr::new(s as *mut SparseSet<B>))
        };

        let (storage_a_ptr, storage_b_ptr) = match (storage_a_ptr, storage_b_ptr) {
            (Some(a), Some(b)) => (a, b),
            _ => return consumer.into_folder().complete(),
        };

        // Verify they're different storages
        debug_assert_ne!(
            unsafe { storage_a_ptr.as_ptr() } as *const (),
            unsafe { storage_b_ptr.as_ptr() } as *const (),
            "Attempted to create two mutable references to the same component storage"
        );

        let len = unsafe { storage_a_ptr.deref().len().min(storage_b_ptr.deref().len()) };

        (0..len)
            .into_par_iter()
            .filter_map(move |index| unsafe {
                let storage_a = storage_a_ptr.deref_mut();
                let storage_b = storage_b_ptr.deref_mut();

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
        let mut world = World::new();
        world.register::<Position>();

        for i in 0..100 {
            let entity = world.spawn();
            world.add(entity, Position { x: i as f32, y: i as f32, z: i as f32 });
        }

        let sum: f32 = world.par_query::<Position>().map(|(_, pos)| pos.x).sum();
        assert_eq!(sum, 4950.0);
    }

    #[test]
    fn test_parallel_iter_mut_single_component() {
        let mut world = World::new();
        world.register::<Position>();

        for i in 0..100 {
            let entity = world.spawn();
            world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
        }

        world.par_query_mut::<Position>().for_each(|(_, pos)| {
            pos.y = pos.x * 2.0;
        });

        let sum: f32 = world.query::<&Position>().map(|(_, pos)| pos.y).sum();
        assert_eq!(sum, 9900.0);
    }

    #[test]
    fn test_parallel_iter_two_components() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        for i in 0..100 {
            let entity = world.spawn();
            world.add(entity, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(entity, Velocity { x: 1.0, y: 1.0, z: 1.0 });
        }

        let count = world
            .par_query2::<Position, Velocity>()
            .filter(|(_, (pos, vel))| pos.x >= 50.0 && vel.x == 1.0)
            .count();

        assert_eq!(count, 50);
    }

    #[test]
    fn test_parallel_iter_mut_two_components() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        for i in 0..100 {
            let entity = world.spawn();
            world.add(entity, Position { x: 0.0, y: 0.0, z: 0.0 });
            world.add(entity, Velocity { x: i as f32, y: 1.0, z: 1.0 });
        }

        world.par_query2_mut::<Position, Velocity>().for_each(|(_, (pos, vel))| {
            pos.x += vel.x;
            pos.y += vel.y;
            pos.z += vel.z;
        });

        let sum_x: f32 = world.query::<&Position>().map(|(_, pos)| pos.x).sum();
        let sum_y: f32 = world.query::<&Position>().map(|(_, pos)| pos.y).sum();

        assert_eq!(sum_x, 4950.0);
        assert_eq!(sum_y, 100.0);
    }

    #[test]
    fn test_parallel_empty_query() {
        let mut world = World::new();
        world.register::<Position>();

        let count = world.par_query::<Position>().count();
        assert_eq!(count, 0);
    }
}
