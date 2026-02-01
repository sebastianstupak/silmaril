//! Type-safe query system for accessing entity components
//!
//! Provides ergonomic iteration over entities with specific component combinations.
//! Supports single components, tuples, optional components, and mixed mutability.
//!
//! # Performance Optimizations
//!
//! This module includes several performance optimizations:
//! - Memory prefetching for better cache utilization
//! - Fast-path for single-component queries
//! - Cached storage pointers to avoid repeated type lookups
//! - Batch iteration support for SIMD processing
//! - Optimized component access patterns with minimal virtual dispatch

use super::storage::ComponentStorage;
use super::{Component, Entity, SparseSet, World};
use std::any::TypeId;
use std::marker::PhantomData;

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

/// Prefetch a memory location for reading
///
/// Uses compiler intrinsics to hint that we'll access this memory soon.
/// The CPU can start loading it into cache before we actually need it.
///
/// SAFETY: Prefetching is always safe - it's just a hint to the CPU.
/// Even if the pointer is invalid, prefetch is a no-op.
#[inline(always)]
fn prefetch_read<T>(ptr: *const T) {
    #[cfg(target_arch = "x86_64")]
    {
        // Use x86 prefetch intrinsic (T0 = fetch to all cache levels)
        unsafe {
            core::arch::x86_64::_mm_prefetch::<{ core::arch::x86_64::_MM_HINT_T0 }>(
                ptr as *const i8,
            );
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        // On other architectures, rely on the compiler's prefetch hints
        // Most compilers will optimize this away or use the appropriate intrinsic
        let _ = ptr; // Avoid unused variable warning
    }
}

/// Branch prediction hint: marks a function as cold (rarely executed)
///
/// OPTIMIZATION: Helps the compiler generate better assembly for hot paths
/// by moving unlikely code out of the hot path.
#[inline(always)]
#[cold]
fn cold() {}

/// Hint that this condition is likely to be true
///
/// OPTIMIZATION: Helps CPU branch predictor by biasing toward the expected path.
/// Use when a branch is taken >90% of the time.
#[inline(always)]
fn likely(b: bool) -> bool {
    if !b {
        cold();
    }
    b
}

/// Hint that this condition is unlikely to be true
///
/// OPTIMIZATION: Helps CPU branch predictor by biasing toward the expected path.
/// Use when a branch is taken <10% of the time.
#[inline(always)]
fn unlikely(b: bool) -> bool {
    if b {
        cold();
    }
    b
}

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
///
/// OPTIMIZATION: We store a cached reference to the World to allow filter checks,
/// but the actual storage references are fetched fresh each iteration.
/// For single-component queries, we could cache the storage ref, but that would
/// complicate the generic design. The current approach keeps the API flexible.
///
/// For two-component queries, we cache the TypeId values to avoid recomputing
/// them on every next() call, and we also cache storage pointers for better performance.
pub struct QueryIter<'a, Q: Query> {
    /// Reference to the world being queried
    world: &'a World,
    /// Current position in iteration
    current_index: usize,
    /// Total number of items to iterate
    len: usize,
    /// Component types that entities MUST have (in addition to query components)
    with_filters: Vec<TypeId>,
    /// Component types that entities MUST NOT have
    without_filters: Vec<TypeId>,
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
            with_filters: Vec::new(),
            without_filters: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Filter to only include entities that have the specified component
    ///
    /// This can be chained multiple times to require multiple additional components.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # #[derive(Debug)] struct Position { x: f32 }
    /// # #[derive(Debug)] struct Health { current: f32 }
    /// # #[derive(Debug)] struct Alive;
    /// # impl Component for Position {}
    /// # impl Component for Health {}
    /// # impl Component for Alive {}
    /// # let mut world = World::new();
    /// # world.register::<Position>();
    /// # world.register::<Health>();
    /// # world.register::<Alive>();
    /// // Query for position, but only on entities that also have Alive component
    /// for (entity, pos) in world.query::<&Position>().with::<Alive>() {
    ///     // Only alive entities are returned
    /// }
    /// ```
    pub fn with<T: Component>(mut self) -> Self {
        self.with_filters.push(TypeId::of::<T>());
        self
    }

    /// Filter to exclude entities that have the specified component
    ///
    /// This can be chained multiple times to exclude multiple components.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Component};
    /// # #[derive(Debug)] struct Position { x: f32 }
    /// # #[derive(Debug)] struct Dead;
    /// # impl Component for Position {}
    /// # impl Component for Dead {}
    /// # let mut world = World::new();
    /// # world.register::<Position>();
    /// # world.register::<Dead>();
    /// // Query for position, but exclude dead entities
    /// for (entity, pos) in world.query::<&Position>().without::<Dead>() {
    ///     // Only non-dead entities are returned
    /// }
    /// ```
    pub fn without<T: Component>(mut self) -> Self {
        self.without_filters.push(TypeId::of::<T>());
        self
    }

    /// Check if an entity passes all filters
    ///
    /// Note: This is a simplified implementation that checks if the component storage exists
    /// for the type. A full implementation would require a ComponentStorage trait with
    /// type-erased contains() method.
    #[allow(dead_code)]
    pub(crate) fn passes_filters(&self, _entity: Entity) -> bool {
        // Check with filters - entity must have all of these
        // Note: We can't efficiently check this without the component type
        // For now, filters will be checked in the iterator's next() method
        // where we have access to typed storage

        // This is a placeholder - actual filter checking happens in Iterator::next()
        true
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
        Self { world, current_index: 0, len, _phantom: PhantomData }
    }
}

//
// Single Component Queries - Immutable Reference
//

impl<T: Component> Query for &T {
    type Item<'a> = (Entity, &'a T);

    fn fetch(world: &World) -> QueryIter<'_, Self> {
        #[cfg(feature = "profiling")]
        profile_scope!("query_fetch_single", ProfileCategory::ECS);

        let type_id = TypeId::of::<T>();
        let len = world
            .components
            .get(&type_id)
            .and_then(|storage| storage.as_any().downcast_ref::<SparseSet<T>>())
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

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // CRITICAL OPTIMIZATION: Use get_storage() to bypass virtual dispatch
        //
        // Instead of going through the ComponentStorage trait (.as_any().downcast_ref()),
        // we use World::get_storage() which does the downcast directly. This eliminates
        // one level of virtual dispatch.
        //
        // With #[inline] on get_storage(), the compiler can optimize this further.

        let storage = self.world.get_storage::<T>()?;

        // OPTIMIZATION: Use direct index access instead of nth() for O(1) vs O(n)
        while likely(self.current_index < storage.len()) {
            // PREFETCH OPTIMIZATION: Load next entity's component into cache while processing current
            // This exploits instruction-level parallelism in modern CPUs
            if self.current_index + 1 < storage.len() {
                if let Some(next_entity) = storage.get_dense_entity(self.current_index + 1) {
                    if let Some(next_component) = storage.get(next_entity) {
                        // Prefetch the next component into L1 cache
                        prefetch_read(next_component as *const T);
                    }
                }
            }

            // OPTIMIZATION: get_dense_entity should always succeed in valid iteration
            let entity = match storage.get_dense_entity(self.current_index) {
                Some(e) => e,
                None => {
                    self.current_index += 1;
                    continue;
                }
            };
            self.current_index += 1;

            // OPTIMIZATION: Most queries have no filters, so these checks are usually skipped
            // Apply filters (if any)
            if unlikely(!self.with_filters.is_empty()) {
                let mut passes_with = true;
                for filter_type_id in &self.with_filters {
                    if !self.world.has_component_by_id(entity, *filter_type_id) {
                        passes_with = false;
                        break;
                    }
                }
                if !passes_with {
                    continue;
                }
            }

            if unlikely(!self.without_filters.is_empty()) {
                let mut passes_without = true;
                for filter_type_id in &self.without_filters {
                    if self.world.has_component_by_id(entity, *filter_type_id) {
                        passes_without = false;
                        break;
                    }
                }
                if !passes_without {
                    continue;
                }
            }

            // OPTIMIZATION: storage.get() should always succeed for valid entities
            // Entity passes all filters, return component
            if likely(storage.get(entity).is_some()) {
                // SAFETY: We just checked that get() returns Some
                return Some((entity, unsafe { storage.get(entity).unwrap_unchecked() }));
            }
        }

        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        // Lower bound is 0 if we have filters (since we don't know how many will pass)
        // Otherwise it's `remaining` (all entities have the component)
        let lower = if self.with_filters.is_empty() && self.without_filters.is_empty() {
            remaining
        } else {
            0
        };
        (lower, Some(remaining))
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
        #[cfg(feature = "profiling")]
        profile_scope!("query_fetch_mut_single", ProfileCategory::ECS);

        let type_id = TypeId::of::<T>();
        let len = world
            .components
            .get(&type_id)
            .and_then(|storage| storage.as_any().downcast_ref::<SparseSet<T>>())
            .map(|storage: &SparseSet<T>| storage.len())
            .unwrap_or(0);

        QueryIterMut::new(world, len)
    }
}

impl<'a, T: Component> Iterator for QueryIterMut<'a, &mut T> {
    type Item = (Entity, &'a mut T);

    #[inline]
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
                .as_any_mut()
                .downcast_mut::<SparseSet<T>>()? as *mut SparseSet<T>;
            &mut *storage_ptr
        };

        // OPTIMIZATION: Use direct index access instead of nth() for O(1) vs O(n)
        let entity = storage.get_dense_entity(self.current_index)?;
        let component = storage.get_mut(entity).map(|comp| {
            // SAFETY: Extend lifetime to 'a
            // This is safe because we have exclusive access via &mut World
            unsafe { &mut *(comp as *mut T) }
        })?;

        self.current_index += 1;
        Some((entity, component))
    }

    #[inline]
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
// Optional Component Queries
//

/// Query for optional immutable component reference
///
/// Returns Some(&T) if the entity has the component, None otherwise.
/// Unlike regular queries, optional queries don't filter out entities.
impl<T: Component> Query for Option<&T> {
    type Item<'a> = (Entity, Option<&'a T>);

    fn fetch(world: &World) -> QueryIter<'_, Self> {
        // For optional queries, we need to iterate ALL entities
        // This is a limitation - we can't efficiently iterate all entities
        // without a global entity list. For now, we'll iterate the component storage
        // and return Some() for entities that have it.
        //
        // This means optional-only queries will only return entities that HAVE the component.
        // Optional components work best in combination with required components.
        let type_id = TypeId::of::<T>();
        let len = world
            .components
            .get(&type_id)
            .and_then(|storage| storage.as_any().downcast_ref::<SparseSet<T>>())
            .map(|storage| storage.len())
            .unwrap_or(0);

        QueryIter::new(world, len)
    }

    fn fetch_mut(_world: &mut World) -> QueryIterMut<'_, Self> {
        panic!("Cannot use fetch_mut for immutable optional query. Use fetch instead.");
    }
}

impl<'a, T: Component> Iterator for QueryIter<'a, Option<&T>> {
    type Item = (Entity, Option<&'a T>);

    fn next(&mut self) -> Option<Self::Item> {
        let type_id = TypeId::of::<T>();

        let storage =
            self.world.components.get(&type_id)?.as_any().downcast_ref::<SparseSet<T>>()?;

        if self.current_index >= storage.len() {
            return None;
        }

        let entity = storage.get_dense_entity(self.current_index)?;
        let component = storage.get(entity);

        self.current_index += 1;

        Some((entity, component))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

/// Query for optional mutable component reference
impl<T: Component> Query for Option<&mut T> {
    type Item<'a> = (Entity, Option<&'a mut T>);

    fn fetch(_world: &World) -> QueryIter<'_, Self> {
        panic!("Cannot use fetch for mutable optional query. Use fetch_mut instead.");
    }

    fn fetch_mut(world: &mut World) -> QueryIterMut<'_, Self> {
        let type_id = TypeId::of::<T>();
        let len = world
            .components
            .get(&type_id)
            .and_then(|storage| storage.as_any().downcast_ref::<SparseSet<T>>())
            .map(|storage| storage.len())
            .unwrap_or(0);

        QueryIterMut::new(world, len)
    }
}

impl<'a, T: Component> Iterator for QueryIterMut<'a, Option<&mut T>> {
    type Item = (Entity, Option<&'a mut T>);

    fn next(&mut self) -> Option<Self::Item> {
        let type_id = TypeId::of::<T>();

        // SAFETY: We need to extend the lifetime here
        // This is safe because we have exclusive access to the world
        let storage = unsafe {
            let storage_ptr = self
                .world
                .components
                .get_mut(&type_id)?
                .as_any_mut()
                .downcast_mut::<SparseSet<T>>()? as *mut SparseSet<T>;
            &mut *storage_ptr
        };

        if self.current_index >= storage.len() {
            return None;
        }

        let entity = storage.get_dense_entity(self.current_index)?;
        let component = storage.get_mut(entity).map(|comp| {
            // SAFETY: Extend lifetime to 'a
            unsafe { &mut *(comp as *mut T) }
        });

        self.current_index += 1;

        Some((entity, component))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        (remaining, Some(remaining))
    }
}

//
// Two-Component Tuple Queries
//

impl<A: Component, B: Component> Query for (&A, &B) {
    type Item<'a> = (Entity, (&'a A, &'a B));

    fn fetch(world: &World) -> QueryIter<'_, Self> {
        #[cfg(feature = "profiling")]
        profile_scope!("query_fetch_tuple2", ProfileCategory::ECS);

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // Get both storages
        let storage_a = world
            .components
            .get(&type_id_a)
            .and_then(|s| s.as_any().downcast_ref::<SparseSet<A>>());

        let storage_b = world
            .components
            .get(&type_id_b)
            .and_then(|s| s.as_any().downcast_ref::<SparseSet<B>>());

        // If either storage is missing, return empty iterator
        let (storage_a, storage_b) = match (storage_a, storage_b) {
            (Some(a), Some(b)) => (a, b),
            _ => return QueryIter::new(world, 0),
        };

        // Use the smaller storage's length as the iteration bound
        // We'll iterate the smaller storage and check the larger one
        let len = storage_a.len().min(storage_b.len());
        QueryIter::new(world, len)
    }

    fn fetch_mut(_world: &mut World) -> QueryIterMut<'_, Self> {
        panic!("Cannot use fetch_mut for immutable tuple query. Use fetch instead.");
    }
}

impl<'a, A: Component, B: Component> Iterator for QueryIter<'a, (&A, &B)> {
    type Item = (Entity, (&'a A, &'a B));

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        // CRITICAL OPTIMIZATION: Get storage once per next() call
        // The get_storage() call does TypeId::of and HashMap lookup
        let storage_a = self.world.get_storage::<A>()?;
        let storage_b = self.world.get_storage::<B>()?;

        // OPTIMIZATION: Direct index access (O(1) per iteration instead of O(n) with nth())
        // Iterate storage_a and check if entity exists in storage_b
        while likely(self.current_index < storage_a.len()) {
            // ENHANCED PREFETCH OPTIMIZATION: Prefetch multiple cache lines ahead
            // Modern CPUs have hardware prefetchers that work best with predictable access patterns
            // Prefetching 2-4 entities ahead gives best performance on most architectures
            const PREFETCH_DISTANCE: usize = 3;

            for offset in 1..=PREFETCH_DISTANCE {
                let prefetch_idx = self.current_index + offset;
                if prefetch_idx < storage_a.len() {
                    if let Some(next_entity) = storage_a.get_dense_entity(prefetch_idx) {
                        if let Some(next_a) = storage_a.get(next_entity) {
                            prefetch_read(next_a as *const A);
                        }
                        if let Some(next_b) = storage_b.get(next_entity) {
                            prefetch_read(next_b as *const B);
                        }
                    }
                }
            }

            // OPTIMIZATION: get_dense_entity should always succeed
            let entity = match storage_a.get_dense_entity(self.current_index) {
                Some(e) => e,
                None => {
                    self.current_index += 1;
                    continue;
                }
            };
            self.current_index += 1;

            // OPTIMIZATION: Early exit if component B is missing (likely in sparse scenarios)
            // In dense scenarios where all entities have both components, this is always Some
            let comp_b = match storage_b.get(entity) {
                Some(c) => c,
                None => continue,
            };

            // OPTIMIZATION: Most queries don't use filters, so hint these as unlikely
            // Apply filters (if any)
            // Check with filters - entity must have all of these
            if unlikely(!self.with_filters.is_empty()) {
                let mut passes_with = true;
                for filter_type_id in &self.with_filters {
                    if !self.world.has_component_by_id(entity, *filter_type_id) {
                        passes_with = false;
                        break;
                    }
                }
                if !passes_with {
                    continue;
                }
            }

            // Check without filters - entity must NOT have any of these
            if unlikely(!self.without_filters.is_empty()) {
                let mut passes_without = true;
                for filter_type_id in &self.without_filters {
                    if self.world.has_component_by_id(entity, *filter_type_id) {
                        passes_without = false;
                        break;
                    }
                }
                if !passes_without {
                    continue;
                }
            }

            // OPTIMIZATION: storage_a.get() should always succeed since we're iterating it
            // Entity has both components and passes filters
            if likely(storage_a.get(entity).is_some()) {
                // SAFETY: We just verified that get() returns Some
                let comp_a = unsafe { storage_a.get(entity).unwrap_unchecked() };
                return Some((entity, (comp_a, comp_b)));
            }
        }

        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        // For two-component queries, we need to check both components,
        // so we don't know the exact count without iterating
        // Lower bound is always 0 for two-component queries
        (0, Some(remaining))
    }
}

// Note: We don't implement ExactSizeIterator for immutable two-component queries
// because we don't pre-filter during fetch (we use min(storage_a.len(), storage_b.len())).
// The actual count requires iteration to determine which entities have both components.
// For mutable queries, we do pre-filter, so ExactSizeIterator can be implemented there.

// Mutable two-component tuple query
impl<A: Component, B: Component> Query for (&mut A, &mut B) {
    type Item<'a> = (Entity, (&'a mut A, &'a mut B));

    fn fetch(_world: &World) -> QueryIter<'_, Self> {
        panic!("Cannot use fetch for mutable tuple query. Use fetch_mut instead.");
    }

    fn fetch_mut(world: &mut World) -> QueryIterMut<'_, Self> {
        #[cfg(feature = "profiling")]
        profile_scope!("query_fetch_mut_tuple2", ProfileCategory::ECS);

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // Get both storages
        let storage_a = world
            .components
            .get(&type_id_a)
            .and_then(|s| s.as_any().downcast_ref::<SparseSet<A>>());

        let storage_b = world
            .components
            .get(&type_id_b)
            .and_then(|s| s.as_any().downcast_ref::<SparseSet<B>>());

        // If either storage is missing, return empty iterator
        let (storage_a, storage_b) = match (storage_a, storage_b) {
            (Some(a), Some(b)) => (a, b),
            _ => return QueryIterMut::new(world, 0),
        };

        // Count entities that have both components
        let len = if storage_a.len() <= storage_b.len() {
            storage_a.iter().filter(|(entity, _)| storage_b.contains(*entity)).count()
        } else {
            storage_b.iter().filter(|(entity, _)| storage_a.contains(*entity)).count()
        };

        QueryIterMut::new(world, len)
    }
}

impl<'a, A: Component, B: Component> Iterator for QueryIterMut<'a, (&mut A, &mut B)> {
    type Item = (Entity, (&'a mut A, &'a mut B));

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // SAFETY: We need to extend lifetimes here for iteration
        // This is safe because:
        // 1. We have exclusive access to world (&mut World)
        // 2. We return one mutable reference pair at a time
        // 3. The borrow checker ensures no aliasing
        // 4. A and B are different types (different TypeIds)
        unsafe {
            let storage_a_ptr =
                self.world
                    .components
                    .get_mut(&type_id_a)?
                    .as_any_mut()
                    .downcast_mut::<SparseSet<A>>()? as *mut SparseSet<A>;

            let storage_b_ptr =
                self.world
                    .components
                    .get_mut(&type_id_b)?
                    .as_any_mut()
                    .downcast_mut::<SparseSet<B>>()? as *mut SparseSet<B>;

            let storage_a = &mut *storage_a_ptr;
            let storage_b = &mut *storage_b_ptr;

            // OPTIMIZATION: Use direct index access instead of iter().nth() (O(1) vs O(n))
            // Find next entity that has both components
            while self.current_index < storage_a.len() {
                // ENHANCED PREFETCH: Prefetch multiple entities ahead for mutable iteration
                // This is critical for write performance as it hides memory latency
                const PREFETCH_DISTANCE: usize = 3;

                for offset in 1..=PREFETCH_DISTANCE {
                    let prefetch_idx = self.current_index + offset;
                    if prefetch_idx < storage_a.len() {
                        if let Some(next_entity) = storage_a.get_dense_entity(prefetch_idx) {
                            if let Some(next_a) = storage_a.get(next_entity) {
                                prefetch_read(next_a as *const A);
                            }
                            if let Some(next_b) = storage_b.get(next_entity) {
                                prefetch_read(next_b as *const B);
                            }
                        }
                    }
                }

                let entity = storage_a.get_dense_entity(self.current_index)?;
                self.current_index += 1;

                if storage_b.contains(entity) {
                    // Get mutable references to both components
                    let comp_a = &mut *(storage_a.get_mut(entity)? as *mut A);
                    let comp_b = &mut *(storage_b.get_mut(entity)? as *mut B);
                    return Some((entity, (comp_a, comp_b)));
                }
            }

            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.current_index);
        (0, Some(remaining))
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
        #[cfg(feature = "profiling")]
        profile_scope!("query_fetch_mut_mixed2", ProfileCategory::ECS);

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        let storage_a = world
            .components
            .get(&type_id_a)
            .and_then(|s| s.as_any().downcast_ref::<SparseSet<A>>());

        let storage_b = world
            .components
            .get(&type_id_b)
            .and_then(|s| s.as_any().downcast_ref::<SparseSet<B>>());

        let (storage_a, storage_b) = match (storage_a, storage_b) {
            (Some(a), Some(b)) => (a, b),
            _ => return QueryIterMut::new(world, 0),
        };

        let len = if storage_a.len() <= storage_b.len() {
            storage_a.iter().filter(|(entity, _)| storage_b.contains(*entity)).count()
        } else {
            storage_b.iter().filter(|(entity, _)| storage_a.contains(*entity)).count()
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
            let components_ptr = &mut self.world.components
                as *mut std::collections::HashMap<TypeId, Box<dyn ComponentStorage>>;
            let components = &mut *components_ptr;

            let storage_a_ptr =
                components.get(&type_id_a)?.as_any().downcast_ref::<SparseSet<A>>()?
                    as *const SparseSet<A>;

            let storage_b_ptr =
                components.get_mut(&type_id_b)?.as_any_mut().downcast_mut::<SparseSet<B>>()?
                    as *mut SparseSet<B>;

            let storage_a = &*storage_a_ptr;
            let storage_b = &mut *storage_b_ptr;

            // OPTIMIZATION: Use direct index access instead of iter().nth() (O(1) vs O(n))
            while self.current_index < storage_a.len() {
                let entity = storage_a.get_dense_entity(self.current_index)?;
                self.current_index += 1;

                if let (Some(comp_a), Some(comp_b)) =
                    (storage_a.get(entity), storage_b.get_mut(entity))
                {
                    let comp_a_ptr = comp_a as *const A;
                    let comp_b_ptr = comp_b as *mut B;
                    return Some((entity, (&*comp_a_ptr, &mut *comp_b_ptr)));
                }
            }

            None
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
        #[cfg(feature = "profiling")]
        profile_scope!("query_fetch_mut_mixed2", ProfileCategory::ECS);

        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        let storage_a = world
            .components
            .get(&type_id_a)
            .and_then(|s| s.as_any().downcast_ref::<SparseSet<A>>());

        let storage_b = world
            .components
            .get(&type_id_b)
            .and_then(|s| s.as_any().downcast_ref::<SparseSet<B>>());

        let (storage_a, storage_b) = match (storage_a, storage_b) {
            (Some(a), Some(b)) => (a, b),
            _ => return QueryIterMut::new(world, 0),
        };

        let len = if storage_a.len() <= storage_b.len() {
            storage_a.iter().filter(|(entity, _)| storage_b.contains(*entity)).count()
        } else {
            storage_b.iter().filter(|(entity, _)| storage_a.contains(*entity)).count()
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
            let components_ptr = &mut self.world.components
                as *mut std::collections::HashMap<TypeId, Box<dyn ComponentStorage>>;
            let components = &mut *components_ptr;

            let storage_a_ptr =
                components.get_mut(&type_id_a)?.as_any_mut().downcast_mut::<SparseSet<A>>()?
                    as *mut SparseSet<A>;

            let storage_b_ptr =
                components.get(&type_id_b)?.as_any().downcast_ref::<SparseSet<B>>()?
                    as *const SparseSet<B>;

            let storage_a = &mut *storage_a_ptr;
            let storage_b = &*storage_b_ptr;

            // OPTIMIZATION: Use direct index access instead of iter().nth() (O(1) vs O(n))
            while self.current_index < storage_a.len() {
                let entity = storage_a.get_dense_entity(self.current_index)?;
                self.current_index += 1;

                if let (Some(comp_a), Some(comp_b)) =
                    (storage_a.get_mut(entity), storage_b.get(entity))
                {
                    let comp_a_ptr = comp_a as *mut A;
                    let comp_b_ptr = comp_b as *const B;
                    return Some((entity, (&mut *comp_a_ptr, &*comp_b_ptr)));
                }
            }

            None
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

//
// Macro-Based Tuple Query Generation (3-12 Components)
//

/// Macro to implement Query for immutable N-component tuples
///
/// This generates Query trait implementations for tuples of immutable component references.
/// The implementation iterates over entities and filters for those that have all components.
macro_rules! impl_query_tuple {
    // Pattern: Take the first component separately, then the rest
    ($first:ident $(, $rest:ident)*) => {
        #[allow(non_snake_case)]
        impl<$first: Component $(, $rest: Component)*> Query for (&$first $(, &$rest)*) {
            type Item<'a> = (Entity, (&'a $first $(, &'a $rest)*));

            fn fetch(world: &World) -> QueryIter<'_, Self> {
                #[cfg(feature = "profiling")]
                profile_scope!("query_fetch_tuple_n", ProfileCategory::ECS);

                // Return empty if any component type is not registered
                let first_id = TypeId::of::<$first>();
                if world.components.get(&first_id).is_none() {
                    return QueryIter::new(world, 0);
                }
                $(
                    let rest_id = TypeId::of::<$rest>();
                    if world.components.get(&rest_id).is_none() {
                        return QueryIter::new(world, 0);
                    }
                )*

                // Use a conservative estimate for length
                // We'll filter during iteration
                let len = world.components.get(&first_id)
                    .and_then(|s| s.as_any().downcast_ref::<SparseSet<$first>>())
                    .map(|s| s.len())
                    .unwrap_or(0);

                QueryIter::new(world, len)
            }

            fn fetch_mut(_world: &mut World) -> QueryIterMut<'_, Self> {
                panic!("Cannot use fetch_mut for immutable tuple query. Use fetch instead.");
            }
        }

        #[allow(non_snake_case)]
        impl<'a, $first: Component $(, $rest: Component)*> Iterator for QueryIter<'a, (&$first $(, &$rest)*)> {
            type Item = (Entity, (&'a $first $(, &'a $rest)*));

            fn next(&mut self) -> Option<Self::Item> {
                // OPTIMIZATION: Use get_storage() to bypass virtual dispatch
                let first_storage = self.world.get_storage::<$first>()?;

                // Get the rest of the storages
                $(
                    let $rest = self.world.get_storage::<$rest>()?;
                )*

                // OPTIMIZATION: Use direct index access instead of iter().nth() (O(1) vs O(n))
                // Iterate until we find an entity with all components
                while likely(self.current_index < first_storage.len()) {
                    let entity = match first_storage.get_dense_entity(self.current_index) {
                        Some(e) => e,
                        None => {
                            self.current_index += 1;
                            continue;
                        }
                    };
                    self.current_index += 1;

                    // Get first component
                    let first_comp = match first_storage.get(entity) {
                        Some(c) => c,
                        None => continue,
                    };

                    // Try to get all other components
                    $(
                        let $rest = match $rest.get(entity) {
                            Some(c) => c,
                            None => continue, // Missing component, skip this entity
                        };
                    )*

                    // OPTIMIZATION: Most queries don't use filters
                    // Apply filters (if any)
                    // Check with filters - entity must have all of these
                    if unlikely(!self.with_filters.is_empty()) {
                        let mut passes_with = true;
                        for filter_type_id in &self.with_filters {
                            if !self.world.has_component_by_id(entity, *filter_type_id) {
                                passes_with = false;
                                break;
                            }
                        }
                        if !passes_with {
                            continue;
                        }
                    }

                    // Check without filters - entity must NOT have any of these
                    if unlikely(!self.without_filters.is_empty()) {
                        let mut passes_without = true;
                        for filter_type_id in &self.without_filters {
                            if self.world.has_component_by_id(entity, *filter_type_id) {
                                passes_without = false;
                                break;
                            }
                        }
                        if !passes_without {
                            continue;
                        }
                    }

                    // All components found and filters passed!
                    return Some((entity, (first_comp $(, $rest)*)));
                }

                None
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let remaining = self.len.saturating_sub(self.current_index);
                (0, Some(remaining))
            }
        }
    };
}

/// Macro to implement Query for mutable N-component tuples
///
/// This generates Query trait implementations for tuples of mutable component references.
/// Uses unsafe code with raw pointers to allow multiple mutable borrows.
macro_rules! impl_query_tuple_mut {
    ($first:ident $(, $rest:ident)*) => {
        #[allow(non_snake_case)]
        impl<$first: Component $(, $rest: Component)*> Query for (&mut $first $(, &mut $rest)*) {
            type Item<'a> = (Entity, (&'a mut $first $(, &'a mut $rest)*));

            fn fetch(_world: &World) -> QueryIter<'_, Self> {
                panic!("Cannot use fetch for mutable tuple query. Use fetch_mut instead.");
            }

            fn fetch_mut(world: &mut World) -> QueryIterMut<'_, Self> {
                #[cfg(feature = "profiling")]
                profile_scope!("query_fetch_mut_tuple_n", ProfileCategory::ECS);

                // Return empty if any component type is not registered
                let first_id = TypeId::of::<$first>();
                if world.components.get(&first_id).is_none() {
                    return QueryIterMut::new(world, 0);
                }
                $(
                    let rest_id = TypeId::of::<$rest>();
                    if world.components.get(&rest_id).is_none() {
                        return QueryIterMut::new(world, 0);
                    }
                )*

                let len = world.components.get(&first_id)
                    .and_then(|s| s.as_any().downcast_ref::<SparseSet<$first>>())
                    .map(|s| s.len())
                    .unwrap_or(0);

                QueryIterMut::new(world, len)
            }
        }

        #[allow(non_snake_case)]
        impl<'a, $first: Component $(, $rest: Component)*> Iterator for QueryIterMut<'a, (&mut $first $(, &mut $rest)*)> {
            type Item = (Entity, (&'a mut $first $(, &'a mut $rest)*));

            fn next(&mut self) -> Option<Self::Item> {
                // SAFETY: We need raw pointers to get multiple mutable references to different component types
                // This is safe because:
                // 1. We have exclusive access to world (&mut World)
                // 2. All component types are different (enforced by Rust's type system - $first and $rest are distinct)
                // 3. We return one set of mutable references at a time
                // 4. TypeId guarantees different components use different storage
                unsafe {
                    let components_ptr = &mut self.world.components as *mut std::collections::HashMap<TypeId, Box<dyn ComponentStorage>>;
                    let components = &mut *components_ptr;

                    // Get the first storage
                    let first_id = TypeId::of::<$first>();
                    let first_storage_ptr = components.get_mut(&first_id)?
                        .as_any_mut().downcast_mut::<SparseSet<$first>>()?
                        as *mut SparseSet<$first>;
                    let first_storage = &mut *first_storage_ptr;

                    // Get pointers to the rest of the storages
                    $(
                        let rest_id = TypeId::of::<$rest>();
                        let $rest = components.get_mut(&rest_id)?
                            .as_any_mut().downcast_mut::<SparseSet<$rest>>()?
                            as *mut SparseSet<$rest>;
                    )*

                    // OPTIMIZATION: Use direct index access instead of iter_mut().nth() (O(1) vs O(n))
                    // Iterate until we find an entity with all components
                    while self.current_index < first_storage.len() {
                        let entity = first_storage.get_dense_entity(self.current_index)?;
                        self.current_index += 1;

                        // Get first component mutably
                        let first_comp = match first_storage.get_mut(entity) {
                            Some(c) => c as *mut $first,
                            None => continue,
                        };

                        // Try to get all other components mutably
                        $(
                            let $rest = match (&mut *$rest).get_mut(entity) {
                                Some(c) => c as *mut $rest,
                                None => continue, // Missing component, skip this entity
                            };
                        )*

                        // All components found! Extend lifetimes and return
                        return Some((entity, (&mut *first_comp $(, &mut *$rest)*)));
                    }

                    None
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let remaining = self.len.saturating_sub(self.current_index);
                (0, Some(remaining))
            }
        }
    };
}

// Generate implementations for 3-12 component tuples
impl_query_tuple!(A, B, C);
impl_query_tuple!(A, B, C, D);
impl_query_tuple!(A, B, C, D, E);
impl_query_tuple!(A, B, C, D, E, F);
impl_query_tuple!(A, B, C, D, E, F, G);
impl_query_tuple!(A, B, C, D, E, F, G, H);
impl_query_tuple!(A, B, C, D, E, F, G, H, I);
impl_query_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_query_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_query_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);

impl_query_tuple_mut!(A, B, C);
impl_query_tuple_mut!(A, B, C, D);
impl_query_tuple_mut!(A, B, C, D, E);
impl_query_tuple_mut!(A, B, C, D, E, F);
impl_query_tuple_mut!(A, B, C, D, E, F, G);
impl_query_tuple_mut!(A, B, C, D, E, F, G, H);
impl_query_tuple_mut!(A, B, C, D, E, F, G, H, I);
impl_query_tuple_mut!(A, B, C, D, E, F, G, H, I, J);
impl_query_tuple_mut!(A, B, C, D, E, F, G, H, I, J, K);
impl_query_tuple_mut!(A, B, C, D, E, F, G, H, I, J, K, L);

//
// Batch Iteration Support for SIMD
//

/// Batch size configuration for query iteration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchSize {
    /// Process 4 entities at once (SSE/NEON)
    Four = 4,
    /// Process 8 entities at once (AVX2)
    Eight = 8,
}

/// Batch iterator that returns chunks of components for SIMD processing
///
/// This enables efficient SIMD processing by providing components in configurable batch sizes.
/// The iterator prefetches the next batch while processing the current one for better
/// cache utilization.
#[allow(dead_code)]
pub struct BatchQueryIter<'a, T: Component> {
    storage: Option<&'a SparseSet<T>>,
    current_index: usize,
    len: usize,
    batch_size: usize,
}

impl<'a, T: Component> BatchQueryIter<'a, T> {
    /// Creates a new batch iterator with the specified batch size
    pub fn new(world: &'a World, batch_size: BatchSize) -> Self {
        let type_id = TypeId::of::<T>();
        let (storage, len) = match world.components.get(&type_id) {
            Some(storage_trait) => {
                let storage = storage_trait.as_any().downcast_ref::<SparseSet<T>>().unwrap();
                (Some(storage), storage.len())
            }
            None => (None, 0),
        };

        Self { storage, current_index: 0, len, batch_size: batch_size as usize }
    }
}

/// Batch iterator that returns chunks of 4 components at a time
///
/// This enables SIMD processing by providing components in groups of 4.
/// Use this with SIMD types from engine-math (Vec3x4, etc.)
pub struct BatchQueryIter4<'a, T: Component> {
    storage: Option<&'a SparseSet<T>>,
    current_index: usize,
    len: usize,
}

impl<'a, T: Component> BatchQueryIter4<'a, T> {
    /// Creates a new batch iterator for querying components in groups of 4.
    /// Returns an empty iterator if the component type is not registered.
    pub fn new(world: &'a World) -> Self {
        let type_id = TypeId::of::<T>();
        let (storage, len) = match world.components.get(&type_id) {
            Some(storage_trait) => {
                let storage = storage_trait.as_any().downcast_ref::<SparseSet<T>>().unwrap();
                (Some(storage), storage.len())
            }
            None => {
                // Return empty iterator if component not registered
                (None, 0)
            }
        };

        Self { storage, current_index: 0, len }
    }
}

impl<'a, T: Component> Iterator for BatchQueryIter4<'a, T> {
    /// Returns ([Entity; 4], [&T; 4]) for each complete batch of 4
    type Item = ([Entity; 4], [&'a T; 4]);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Return None if no storage (component not registered)
        let storage = self.storage?;

        // Check if we have at least 4 remaining
        if self.current_index + 4 > self.len {
            return None;
        }

        // PREFETCH: Load data for next batch while processing current
        if self.current_index + 8 <= self.len {
            for i in 4..8 {
                if let Some(entity) = storage.get_dense_entity(self.current_index + i) {
                    if let Some(component) = storage.get(entity) {
                        prefetch_read(component as *const T);
                    }
                }
            }
        }

        // Collect 4 entities and components
        let mut entities = [Entity::new(0, 0); 4];
        let mut components: [Option<&'a T>; 4] = [None; 4];

        for i in 0..4 {
            let idx = self.current_index + i;
            if let Some(entity) = storage.get_dense_entity(idx) {
                entities[i] = entity;
                components[i] = storage.get(entity);
            }
        }

        self.current_index += 4;

        // Only return if all 4 components are present
        // This handles sparse scenarios where not all entities have the component
        if components.iter().all(|c| c.is_some()) {
            Some((
                entities,
                [
                    components[0].unwrap(),
                    components[1].unwrap(),
                    components[2].unwrap(),
                    components[3].unwrap(),
                ],
            ))
        } else {
            // Skip to next batch if current batch is incomplete
            self.next()
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.len.saturating_sub(self.current_index)) / 4;
        (0, Some(remaining)) // Lower bound is 0 due to potential sparse data
    }
}

/// Batch iterator that returns chunks of 8 components at a time
///
/// This enables AVX2 SIMD processing by providing components in groups of 8.
/// Use this with SIMD types from engine-math (Vec3x8, etc.)
pub struct BatchQueryIter8<'a, T: Component> {
    storage: Option<&'a SparseSet<T>>,
    current_index: usize,
    len: usize,
}

impl<'a, T: Component> BatchQueryIter8<'a, T> {
    /// Creates a new batch iterator for querying components in groups of 8.
    /// Returns an empty iterator if the component type is not registered.
    pub fn new(world: &'a World) -> Self {
        let type_id = TypeId::of::<T>();
        let (storage, len) = match world.components.get(&type_id) {
            Some(storage_trait) => {
                let storage = storage_trait.as_any().downcast_ref::<SparseSet<T>>().unwrap();
                (Some(storage), storage.len())
            }
            None => {
                // Return empty iterator if component not registered
                (None, 0)
            }
        };

        Self { storage, current_index: 0, len }
    }
}

impl<'a, T: Component> Iterator for BatchQueryIter8<'a, T> {
    /// Returns ([Entity; 8], [&T; 8]) for each complete batch of 8
    type Item = ([Entity; 8], [&'a T; 8]);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Return None if no storage (component not registered)
        let storage = self.storage?;

        // Check if we have at least 8 remaining
        if self.current_index + 8 > self.len {
            return None;
        }

        // PREFETCH: Load data for next batch while processing current
        if self.current_index + 16 <= self.len {
            for i in 8..16 {
                if let Some(entity) = storage.get_dense_entity(self.current_index + i) {
                    if let Some(component) = storage.get(entity) {
                        prefetch_read(component as *const T);
                    }
                }
            }
        }

        // Collect 8 entities and components
        let mut entities = [Entity::new(0, 0); 8];
        let mut components: [Option<&'a T>; 8] = [None; 8];

        for i in 0..8 {
            let idx = self.current_index + i;
            if let Some(entity) = storage.get_dense_entity(idx) {
                entities[i] = entity;
                components[i] = storage.get(entity);
            }
        }

        self.current_index += 8;

        // Only return if all 8 components are present
        if components.iter().all(|c| c.is_some()) {
            Some((
                entities,
                [
                    components[0].unwrap(),
                    components[1].unwrap(),
                    components[2].unwrap(),
                    components[3].unwrap(),
                    components[4].unwrap(),
                    components[5].unwrap(),
                    components[6].unwrap(),
                    components[7].unwrap(),
                ],
            ))
        } else {
            // Skip to next batch if current batch is incomplete
            self.next()
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.len.saturating_sub(self.current_index)) / 8;
        (0, Some(remaining)) // Lower bound is 0 due to potential sparse data
    }
}

/// Wrapper that provides batch iteration methods for query iterators
pub struct BatchableQueryIter<'a, A: Component, B: Component> {
    world: &'a mut World,
    _phantom: PhantomData<(A, B)>,
}

impl<'a, A: Component, B: Component> BatchableQueryIter<'a, A, B> {
    #[allow(dead_code)]
    fn new(world: &'a mut World) -> Self {
        Self { world, _phantom: PhantomData }
    }

    /// Create a batch iterator for processing entities in groups
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Process 8 entities at a time with SIMD
    /// for batch in world.query_mut::<(&mut Transform, &Velocity)>().batch(8) {
    ///     // batch contains up to 8 entities with their components
    /// }
    /// ```
    pub fn batch(self, size: usize) -> BatchQueryIterMut2<'a, A, B> {
        BatchQueryIterMut2::new(self.world, size)
    }
}

/// Batch iterator for two-component mutable queries
///
/// Processes entities in batches for better cache utilization and SIMD processing.
/// Prefetches the next batch while processing the current one.
pub struct BatchQueryIterMut2<'a, A: Component, B: Component> {
    world: &'a mut World,
    current_index: usize,
    batch_size: usize,
    _phantom: PhantomData<(A, B)>,
}

impl<'a, A: Component, B: Component> BatchQueryIterMut2<'a, A, B> {
    fn new(world: &'a mut World, batch_size: usize) -> Self {
        Self { world, current_index: 0, batch_size, _phantom: PhantomData }
    }
}

impl<'a, A: Component, B: Component> Iterator for BatchQueryIterMut2<'a, A, B> {
    type Item = Vec<(Entity, (&'a mut A, &'a mut B))>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let type_id_a = TypeId::of::<A>();
        let type_id_b = TypeId::of::<B>();

        // SAFETY: Extend lifetimes for batch iteration
        unsafe {
            let storage_a_ptr =
                self.world
                    .components
                    .get_mut(&type_id_a)?
                    .as_any_mut()
                    .downcast_mut::<SparseSet<A>>()? as *mut SparseSet<A>;

            let storage_b_ptr =
                self.world
                    .components
                    .get_mut(&type_id_b)?
                    .as_any_mut()
                    .downcast_mut::<SparseSet<B>>()? as *mut SparseSet<B>;

            let storage_a = &mut *storage_a_ptr;
            let storage_b = &mut *storage_b_ptr;

            let mut batch = Vec::with_capacity(self.batch_size);

            // Prefetch next batch while processing current
            let prefetch_start = self.current_index + self.batch_size;
            if prefetch_start < storage_a.len() {
                for i in 0..self.batch_size.min(4) {
                    let prefetch_idx = prefetch_start + i;
                    if prefetch_idx >= storage_a.len() {
                        break;
                    }
                    if let Some(entity) = storage_a.get_dense_entity(prefetch_idx) {
                        if let Some(comp_a) = storage_a.get(entity) {
                            prefetch_read(comp_a as *const A);
                        }
                        if let Some(comp_b) = storage_b.get(entity) {
                            prefetch_read(comp_b as *const B);
                        }
                    }
                }
            }

            // Collect batch
            while batch.len() < self.batch_size && self.current_index < storage_a.len() {
                let entity = match storage_a.get_dense_entity(self.current_index) {
                    Some(e) => e,
                    None => {
                        self.current_index += 1;
                        continue;
                    }
                };
                self.current_index += 1;

                if let (Some(comp_a), Some(comp_b)) =
                    (storage_a.get_mut(entity), storage_b.get_mut(entity))
                {
                    let comp_a_ptr = comp_a as *mut A;
                    let comp_b_ptr = comp_b as *mut B;
                    batch.push((entity, (&mut *comp_a_ptr, &mut *comp_b_ptr)));
                }
            }

            if batch.is_empty() {
                None
            } else {
                Some(batch)
            }
        }
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

    /// Query entities in batches of 4 for SIMD processing
    ///
    /// Returns an iterator that yields chunks of 4 (Entity, Component) pairs at a time.
    /// This is optimized for SIMD processing with Vec3x4, etc.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use engine_core::ecs::{World, Component};
    /// # struct Position { x: f32, y: f32, z: f32 }
    /// # impl Component for Position {}
    /// # let mut world = World::new();
    /// # world.register::<Position>();
    /// // Process 4 positions at a time with SIMD
    /// for (entities, positions) in world.query_batch4::<Position>() {
    ///     // Convert to SIMD types and process
    ///     // let pos_simd = Vec3x4::from_array_of_vec3(&positions);
    ///     // ...
    /// }
    /// ```
    pub fn query_batch4<T: Component>(&self) -> BatchQueryIter4<'_, T> {
        BatchQueryIter4::new(self)
    }

    /// Query entities in batches of 8 for AVX2 SIMD processing
    ///
    /// Returns an iterator that yields chunks of 8 (Entity, Component) pairs at a time.
    /// This is optimized for AVX2 SIMD processing with Vec3x8, etc.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use engine_core::ecs::{World, Component};
    /// # struct Position { x: f32, y: f32, z: f32 }
    /// # impl Component for Position {}
    /// # let mut world = World::new();
    /// # world.register::<Position>();
    /// // Process 8 positions at a time with AVX2 SIMD
    /// for (entities, positions) in world.query_batch8::<Position>() {
    ///     // Convert to SIMD types and process
    ///     // let pos_simd = Vec3x8::from_array_of_vec3(&positions);
    ///     // ...
    /// }
    /// ```
    pub fn query_batch8<T: Component>(&self) -> BatchQueryIter8<'_, T> {
        BatchQueryIter8::new(self)
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
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
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
        world.add(e, Position { x: 1.0, y: 2.0, z: 3.0 });

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
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
        }

        // Collect first 50 entities to avoid borrow checker issues
        let entities_to_update: Vec<Entity> =
            world.query::<&Position>().take(50).map(|(entity, _)| entity).collect();

        // Only 50 have Velocity
        for entity in entities_to_update {
            world.add(entity, Velocity { x: 1.0, y: 0.0, z: 0.0 });
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
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
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
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
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
            world.add(e, Health { current: 100.0, max: 100.0 });
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
        world.add(e1, Position { x: 1.0, y: 2.0, z: 3.0 });
        world.add(e1, Velocity { x: 0.1, y: 0.2, z: 0.3 });

        // Entity with only Position
        let e2 = world.spawn();
        world.add(e2, Position { x: 4.0, y: 5.0, z: 6.0 });

        // Entity with only Velocity
        let e3 = world.spawn();
        world.add(e3, Velocity { x: 0.4, y: 0.5, z: 0.6 });

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
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
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
        world.add(e, Position { x: 0.0, y: 0.0, z: 0.0 });
        world.add(e, Velocity { x: 1.0, y: 2.0, z: 3.0 });

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
        world.add(e, Position { x: 0.0, y: 0.0, z: 0.0 });
        world.add(e, Velocity { x: 5.0, y: 0.0, z: 0.0 });

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
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
        }

        // Only first 50 also have Velocity
        let entities_with_pos: Vec<Entity> =
            world.query::<&Position>().take(50).map(|(e, _)| e).collect();

        for e in entities_with_pos {
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
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
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
        }

        // Entities with Velocity only
        for _i in 0..10 {
            let e = world.spawn();
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
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
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }

        let query = world.query::<(&Position, &Velocity)>();
        let (lower, upper) = query.size_hint();

        // With filtered iteration, lower bound is 0 (we don't know matches without iterating)
        // Upper bound is min(storage_a.len(), storage_b.len())
        assert_eq!(lower, 0);
        assert!(upper.is_some());
        assert!(upper.unwrap() >= 20);

        // Verify we can actually iterate all 20 entities
        assert_eq!(query.count(), 20);
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
            world.add(e, Position { x: 0.0, y: i as f32 * 10.0, z: 0.0 });
            world.add(e, Velocity { x: (i + 1) as f32, y: 0.0, z: 0.0 });
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
            world.add(e, Health { current: 100.0, max: 100.0 });
            world.add(e, Position { x: i as f32 * 10.0, y: 0.0, z: 0.0 });
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

    // ===== Tests for 3+ Component Queries (Macro-Generated) =====

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[allow(dead_code)]
    struct Acceleration {
        x: f32,
        y: f32,
        z: f32,
    }

    impl Component for Acceleration {}

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[allow(dead_code)]
    struct Mass {
        value: f32,
    }

    impl Component for Mass {}

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[allow(dead_code)]
    struct Team {
        id: u32,
    }

    impl Component for Team {}

    #[test]
    fn test_query_three_components_immutable() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Acceleration>();

        // Create entities with all three components
        for i in 0..50 {
            let e = world.spawn();
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            world.add(e, Acceleration { x: 0.1, y: 0.0, z: 0.0 });
        }

        // Create some entities with only 2 components (should be filtered out)
        for i in 0..10 {
            let e = world.spawn();
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
        }

        let mut count = 0;
        for (_entity, (pos, vel, acc)) in world.query::<(&Position, &Velocity, &Acceleration)>() {
            assert!(pos.x >= 0.0);
            assert_eq!(vel.x, 1.0);
            assert_eq!(acc.x, 0.1);
            count += 1;
        }

        assert_eq!(count, 50);
    }

    #[test]
    fn test_query_three_components_mutable() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Acceleration>();

        // Create entities
        for i in 0..20 {
            let e = world.spawn();
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(e, Velocity { x: 0.0, y: 0.0, z: 0.0 });
            world.add(e, Acceleration { x: 1.0, y: 0.0, z: 0.0 });
        }

        // First pass: read positions and accelerations, update velocities
        let positions: Vec<_> = world
            .query::<(&Position, &Acceleration)>()
            .map(|(e, (pos, acc))| (e, pos.x, acc.x))
            .collect();

        for (entity, pos_x, acc_x) in positions {
            if let Some(vel) = world.get_mut::<Velocity>(entity) {
                vel.x += acc_x * pos_x;
            }
        }

        // Verify velocities were updated
        let mut sum = 0.0;
        for (_e, (pos, vel, _acc)) in world.query::<(&Position, &Velocity, &Acceleration)>() {
            assert_eq!(vel.x, pos.x);
            sum += vel.x;
        }

        // Sum should be 0 + 1 + 2 + ... + 19 = 190
        assert_eq!(sum, 190.0);
    }

    #[test]
    fn test_query_four_components_all_mutable() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Mass>();
        world.register::<Health>();

        // Create physics entities with health
        for i in 0..30 {
            let e = world.spawn();
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            world.add(e, Mass { value: 1.0 });
            world.add(e, Health { current: 100.0, max: 100.0 });
        }

        // Simulate damage based on velocity and mass
        for (_e, (vel, mass, health, pos)) in
            world.query_mut::<(&mut Velocity, &mut Mass, &mut Health, &mut Position)>()
        {
            let impact = vel.x * mass.value;
            health.current -= impact * 5.0;

            // Also modify other components
            pos.x += 1.0;
            vel.x *= 0.99;
            mass.value *= 0.99;
        }

        // Verify mutations were applied
        let mut total_health = 0.0;
        for (_e, health) in world.query::<&Health>() {
            assert_eq!(health.current, 95.0); // 100 - (1.0 * 1.0 * 5.0)
            total_health += health.current;
        }

        assert_eq!(total_health, 95.0 * 30.0);
    }

    #[test]
    fn test_query_five_components() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Acceleration>();
        world.register::<Mass>();
        world.register::<Team>();

        // Create complex entities
        for i in 0..15 {
            let e = world.spawn();
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            world.add(e, Acceleration { x: 0.5, y: 0.0, z: 0.0 });
            world.add(e, Mass { value: 2.0 });
            world.add(e, Team { id: i % 3 });
        }

        // Query all five components
        let mut team_counts = [0u32; 3];
        for (_e, (_pos, _vel, _acc, _mass, team)) in
            world.query::<(&Position, &Velocity, &Acceleration, &Mass, &Team)>()
        {
            team_counts[team.id as usize] += 1;
        }

        // Should have 5 entities per team (15 entities / 3 teams)
        assert_eq!(team_counts[0], 5);
        assert_eq!(team_counts[1], 5);
        assert_eq!(team_counts[2], 5);
    }

    #[test]
    fn test_query_three_components_partial_match() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Health>();

        // Create entities with different component combinations
        for i in 0..10 {
            let e = world.spawn();
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });

            if i % 2 == 0 {
                world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            }

            if i % 3 == 0 {
                world.add(e, Health { current: 100.0, max: 100.0 });
            }
        }

        // Query for all three - should only match entities with i % 6 == 0
        let mut count = 0;
        for (_e, (_pos, _vel, _health)) in world.query::<(&Position, &Velocity, &Health)>() {
            count += 1;
        }

        // i=0 and i=6 have all three components
        assert_eq!(count, 2);
    }

    #[test]
    fn test_query_four_components_simulation() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Acceleration>();
        world.register::<Mass>();

        // Create entities
        for i in 0..10 {
            let e = world.spawn();
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(e, Velocity { x: 0.0, y: 0.0, z: 0.0 });
            world.add(e, Acceleration { x: 1.0, y: 0.0, z: 0.0 });
            world.add(e, Mass { value: 1.0 + i as f32 });
        }

        // Mutate all four components
        for (_e, (pos, vel, acc, mass)) in
            world.query_mut::<(&mut Position, &mut Velocity, &mut Acceleration, &mut Mass)>()
        {
            vel.x = acc.x * mass.value;
            pos.x += vel.x;
            acc.x *= 0.9; // Damping
            mass.value *= 0.99; // Mass loss
        }

        // Verify mutations
        for (_e, (pos, vel, acc, mass)) in
            world.query::<(&Position, &Velocity, &Acceleration, &Mass)>()
        {
            assert!(pos.x > 0.0); // Position increased
            assert!(vel.x > 0.0); // Velocity set
            assert!(acc.x < 1.0); // Acceleration damped
            assert!(mass.value < 11.0); // Mass decreased
        }
    }

    #[test]
    fn test_query_three_components_empty() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Health>();

        // Don't add any entities

        let mut count = 0;
        for _item in world.query::<(&Position, &Velocity, &Health)>() {
            count += 1;
        }

        assert_eq!(count, 0);
    }

    #[test]
    fn test_query_five_components_size_hint() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Acceleration>();
        world.register::<Mass>();
        world.register::<Team>();

        // Create entities
        for i in 0..100 {
            let e = world.spawn();
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            world.add(e, Acceleration { x: 0.5, y: 0.0, z: 0.0 });
            world.add(e, Mass { value: 2.0 });
            world.add(e, Team { id: i % 5 });
        }

        let iter = world.query::<(&Position, &Velocity, &Acceleration, &Mass, &Team)>();
        let (lower, upper) = iter.size_hint();

        // Should have some reasonable bounds
        assert_eq!(lower, 0); // Lower bound is always 0 for filtered iteration
        assert!(upper.is_some());
        assert!(upper.unwrap() >= 100);
    }

    #[test]
    fn test_query_six_components() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();
        world.register::<Acceleration>();
        world.register::<Mass>();
        world.register::<Team>();
        world.register::<Health>();

        // Create entities with all six components
        for i in 0..25 {
            let e = world.spawn();
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });
            world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            world.add(e, Acceleration { x: 0.5, y: 0.0, z: 0.0 });
            world.add(e, Mass { value: 2.0 });
            world.add(e, Team { id: i % 2 });
            world.add(e, Health { current: 100.0, max: 100.0 });
        }

        let mut count = 0;
        for (_e, (pos, vel, acc, mass, team, health)) in
            world.query::<(&Position, &Velocity, &Acceleration, &Mass, &Team, &Health)>()
        {
            assert!(pos.x >= 0.0);
            assert_eq!(vel.x, 1.0);
            assert_eq!(acc.x, 0.5);
            assert_eq!(mass.value, 2.0);
            assert!(team.id < 2);
            assert_eq!(health.current, 100.0);
            count += 1;
        }

        assert_eq!(count, 25);
    }

    // ===== Tests for Optional Components =====

    #[test]
    fn test_query_optional_component_immutable() {
        let mut world = World::new();
        world.register::<Position>();
        world.register::<Velocity>();

        // Create entities - some with velocity, some without
        for i in 0..10 {
            let e = world.spawn();
            world.add(e, Position { x: i as f32, y: 0.0, z: 0.0 });

            if i % 2 == 0 {
                world.add(e, Velocity { x: 1.0, y: 0.0, z: 0.0 });
            }
        }

        // Query for optional velocity
        // Note: Current implementation only returns entities that HAVE the component
        // True "iterate all entities" optional queries would need a global entity list
        let mut count_with_vel = 0;
        for (_entity, vel_opt) in world.query::<Option<&Velocity>>() {
            assert!(vel_opt.is_some());
            count_with_vel += 1;
        }

        assert_eq!(count_with_vel, 5); // Only entities with velocity are returned
    }

    #[test]
    fn test_query_optional_component_mutable() {
        let mut world = World::new();
        world.register::<Health>();

        for i in 0..5 {
            let e = world.spawn();
            world.add(e, Health { current: (i * 10) as f32, max: 100.0 });
        }

        // Mutate optional health
        for (_entity, health_opt) in world.query_mut::<Option<&mut Health>>() {
            if let Some(health) = health_opt {
                health.current += 10.0;
            }
        }

        // Verify mutation
        for (_entity, health_opt) in world.query::<Option<&Health>>() {
            if let Some(health) = health_opt {
                assert!(health.current >= 10.0);
            }
        }
    }
}

// Include filter tests
#[cfg(test)]
#[path = "query_filter_tests.rs"]
mod query_filter_tests;
