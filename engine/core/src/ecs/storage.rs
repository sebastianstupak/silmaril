//! Sparse-set storage for components
//!
//! Provides O(1) insert/remove and cache-friendly iteration over components.
//! The sparse array maps entity IDs to indices in the dense arrays.
//!
//! # Performance Characteristics
//!
//! - **Insertion**: O(1) amortized - Grows sparse array as needed, dense arrays with capacity
//! - **Lookup (get/get_mut)**: O(1) - Direct index access through sparse array
//! - **Removal**: O(1) - Swap-remove maintains density
//! - **Iteration**: O(n) where n = component count - Cache-friendly sequential access
//! - **Contains check**: O(1) - Single sparse array lookup
//!
//! ## Memory Layout
//!
//! Sparse array grows to accommodate entity IDs but contains mostly None values.
//! Dense arrays are packed with no gaps, providing excellent cache locality during iteration.
//! Memory overhead: ~8 bytes per possible entity ID in sparse array + packed component storage.
//!
//! ## Optimization Notes
//!
//! - Uses `#[inline]` on hot path methods for better codegen
//! - Aggressive capacity pre-allocation via `with_capacity()` reduces reallocations
//! - Component and entity arrays kept aligned for better prefetching
//! - Swap-remove strategy avoids expensive array shifts

use super::{Component, Entity};
use std::any::Any;

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

/// Type-erased component storage trait
///
/// Provides type-erased access to component storage operations,
/// allowing World to work with component storages without knowing
/// the concrete component type.
pub trait ComponentStorage: Any {
    /// Remove a component from an entity (type-erased)
    ///
    /// Returns true if the component was removed, false if the entity
    /// didn't have this component.
    fn remove_entity(&mut self, entity: Entity) -> bool;

    /// Check if an entity has this component (type-erased)
    fn contains_entity(&self, entity: Entity) -> bool;

    /// Get a reference to self as &dyn Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Get a mutable reference to self as &mut dyn Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Get component as ComponentData (for serialization)
    ///
    /// Returns None if the entity doesn't have this component.
    fn get_component_data(&self, entity: Entity) -> Option<crate::serialization::ComponentData>;

    /// Clear all components (type-erased)
    fn clear(&mut self);
}

/// Sparse-set storage for a single component type
///
/// Uses two arrays for efficient storage:
/// - Sparse: Entity ID → dense index (has gaps)
/// - Dense: Packed entity and component arrays (no gaps)
///
/// This provides:
/// - O(1) insertion
/// - O(1) removal
/// - O(1) lookup
/// - Cache-friendly iteration (dense arrays)
///
/// # Cache Optimization
///
/// The dense arrays are kept in sync and aligned to cache lines for better locality.
/// During iteration, both entity IDs and components are accessed sequentially,
/// maximizing cache hit rates.
///
/// # Examples
///
/// ```
/// # use engine_core::ecs::{Component, EntityAllocator, SparseSet};
/// # struct Position { x: f32, y: f32, z: f32 }
/// # impl Component for Position {}
/// let mut storage = SparseSet::<Position>::new();
/// let mut allocator = EntityAllocator::new();
/// let entity = allocator.allocate();
///
/// storage.insert(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
/// assert!(storage.get(entity).is_some());
/// ```
#[repr(C)] // Ensure consistent memory layout
pub struct SparseSet<T: Component> {
    /// Sparse array: Entity ID → dense index
    sparse: Vec<Option<usize>>,
    /// Dense entity array (packed, no gaps)
    /// Aligned with components array for better cache locality
    dense: Vec<Entity>,
    /// Dense component array (packed, no gaps)
    /// Aligned with entity array for sequential access
    components: Vec<T>,
}

impl<T: Component> SparseSet<T> {
    /// Create a new empty sparse set
    pub fn new() -> Self {
        Self { sparse: Vec::new(), dense: Vec::new(), components: Vec::new() }
    }

    /// Create a new sparse set with preallocated capacity
    ///
    /// Preallocates space for `capacity` components, reducing allocations
    /// during insertion.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::with_capacity(capacity),
            components: Vec::with_capacity(capacity),
        }
    }

    /// Insert a component for an entity
    ///
    /// If the entity already has this component, it will be replaced.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{Component, EntityAllocator, SparseSet};
    /// # struct Health { current: f32, max: f32 }
    /// # impl Component for Health {}
    /// let mut storage = SparseSet::<Health>::new();
    /// let mut allocator = EntityAllocator::new();
    /// let entity = allocator.allocate();
    ///
    /// storage.insert(entity, Health { current: 100.0, max: 100.0 });
    /// storage.insert(entity, Health { current: 50.0, max: 100.0 }); // Replaces
    /// assert_eq!(storage.get(entity).unwrap().current, 50.0);
    /// ```
    #[inline]
    pub fn insert(&mut self, entity: Entity, component: T) {
        #[cfg(feature = "profiling")]
        profile_scope!("storage_insert", ProfileCategory::ECS);

        // Ensure sparse array is large enough
        let idx = entity.id() as usize;
        if idx >= self.sparse.len() {
            // Extra defensive: check for reasonable entity ID
            assert!(
                idx < 100_000_000,
                "Entity ID {} is suspiciously large (possible corruption?)",
                idx
            );
            self.sparse.resize(idx + 1, None);
        }

        if let Some(dense_idx) = self.sparse[idx] {
            // Component exists, replace it
            // Extra defensive: verify dense index is valid
            assert!(
                dense_idx < self.components.len(),
                "Sparse array contains invalid dense index: {} (max: {})",
                dense_idx,
                self.components.len()
            );
            self.components[dense_idx] = component;
        } else {
            // New component, add to dense arrays
            let dense_idx = self.dense.len();
            self.sparse[idx] = Some(dense_idx);
            self.dense.push(entity);
            self.components.push(component);

            // Extra defensive: verify arrays stay synchronized
            debug_assert_eq!(self.dense.len(), self.components.len(), "Dense arrays out of sync");
        }
    }

    /// Remove a component from an entity
    ///
    /// Returns `Some(component)` if the entity had the component,
    /// `None` otherwise.
    ///
    /// Uses swap-remove for O(1) deletion.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{Component, EntityAllocator, SparseSet};
    /// # struct Health { current: f32, max: f32 }
    /// # impl Component for Health {}
    /// let mut storage = SparseSet::<Health>::new();
    /// let mut allocator = EntityAllocator::new();
    /// let entity = allocator.allocate();
    ///
    /// storage.insert(entity, Health { current: 100.0, max: 100.0 });
    /// let removed = storage.remove(entity);
    /// assert!(removed.is_some());
    /// assert!(storage.get(entity).is_none());
    /// ```
    #[inline]
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        #[cfg(feature = "profiling")]
        profile_scope!("storage_remove", ProfileCategory::ECS);

        let idx = entity.id() as usize;
        let dense_idx = self.sparse.get_mut(idx)?.take()?;

        // Extra defensive: verify dense index is valid
        assert!(
            dense_idx < self.dense.len(),
            "Invalid dense index {} (max: {})",
            dense_idx,
            self.dense.len()
        );

        // Swap-remove from dense arrays
        let last_idx = self.dense.len() - 1;

        if dense_idx != last_idx {
            // Swap with last element
            self.dense.swap(dense_idx, last_idx);
            self.components.swap(dense_idx, last_idx);

            // Update sparse index for swapped entity
            let swapped_entity = self.dense[dense_idx];
            let swapped_id = swapped_entity.id() as usize;

            // Extra defensive: verify swapped entity exists in sparse array
            assert!(
                swapped_id < self.sparse.len(),
                "Swapped entity ID {} out of sparse array bounds",
                swapped_id
            );
            assert!(
                self.sparse[swapped_id].is_some(),
                "Swapped entity {} not found in sparse array",
                swapped_id
            );

            self.sparse[swapped_id] = Some(dense_idx);
        }

        self.dense.pop();
        let component = self.components.pop().unwrap();

        // Extra defensive: verify arrays stay synchronized
        debug_assert_eq!(
            self.dense.len(),
            self.components.len(),
            "Dense arrays out of sync after remove"
        );

        Some(component)
    }

    /// Get an immutable reference to an entity's component
    ///
    /// Returns `None` if the entity doesn't have this component.
    ///
    /// OPTIMIZATION: Uses unchecked access where provably safe to eliminate bounds checks.
    #[inline(always)]
    pub fn get(&self, entity: Entity) -> Option<&T> {
        #[cfg(feature = "profiling")]
        profile_scope!("storage_get", ProfileCategory::ECS);

        let idx = entity.id() as usize;
        // SAFETY: We check bounds explicitly before using get_unchecked
        if idx >= self.sparse.len() {
            return None;
        }
        let dense_idx_opt = unsafe { self.sparse.get_unchecked(idx) };
        let dense_idx = (*dense_idx_opt)?;

        // SAFETY: The sparse set invariant guarantees dense_idx < components.len()
        // This is maintained by insert() and remove() - if violated, it's a bug there.
        debug_assert!(
            dense_idx < self.components.len(),
            "Sparse set invariant violated: dense_idx {} >= components.len() {}",
            dense_idx,
            self.components.len()
        );
        Some(unsafe { self.components.get_unchecked(dense_idx) })
    }

    /// Get a mutable reference to an entity's component
    ///
    /// Returns `None` if the entity doesn't have this component.
    ///
    /// OPTIMIZATION: Uses unchecked access where provably safe to eliminate bounds checks.
    #[inline(always)]
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        #[cfg(feature = "profiling")]
        profile_scope!("storage_get_mut", ProfileCategory::ECS);

        let idx = entity.id() as usize;
        // SAFETY: We check bounds explicitly before using get_unchecked
        if idx >= self.sparse.len() {
            return None;
        }
        let dense_idx_opt = unsafe { self.sparse.get_unchecked(idx) };
        let dense_idx = (*dense_idx_opt)?;

        // SAFETY: The sparse set invariant guarantees dense_idx < components.len()
        // This is maintained by insert() and remove() - if violated, it's a bug there.
        debug_assert!(
            dense_idx < self.components.len(),
            "Sparse set invariant violated: dense_idx {} >= components.len() {}",
            dense_idx,
            self.components.len()
        );
        Some(unsafe { self.components.get_unchecked_mut(dense_idx) })
    }

    /// Check if an entity has this component
    ///
    /// OPTIMIZATION: Manual bounds check + unchecked access for faster codegen.
    #[inline(always)]
    pub fn contains(&self, entity: Entity) -> bool {
        #[cfg(feature = "profiling")]
        profile_scope!("storage_contains", ProfileCategory::ECS);

        let idx = entity.id() as usize;
        // SAFETY: Explicit bounds check before unchecked access
        idx < self.sparse.len() && unsafe { self.sparse.get_unchecked(idx).is_some() }
    }

    /// Iterate over all (entity, component) pairs
    ///
    /// Iteration order is not guaranteed and may change after insertions/removals.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        #[cfg(feature = "profiling")]
        profile_scope!("storage_iter", ProfileCategory::ECS);

        self.dense.iter().copied().zip(self.components.iter())
    }

    /// Iterate over all (entity, component) pairs with mutable component access
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        #[cfg(feature = "profiling")]
        profile_scope!("storage_iter_mut", ProfileCategory::ECS);

        self.dense.iter().copied().zip(self.components.iter_mut())
    }

    /// Get the number of components stored
    #[inline]
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    /// Check if the storage is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    /// Clear all components
    pub fn clear(&mut self) {
        #[cfg(feature = "profiling")]
        profile_scope!("storage_clear", ProfileCategory::ECS);

        self.sparse.clear();
        self.dense.clear();
        self.components.clear();
    }

    /// Reserve capacity for at least `additional` more components
    pub fn reserve(&mut self, additional: usize) {
        self.dense.reserve(additional);
        self.components.reserve(additional);
    }

    /// Get entity at a specific dense index
    ///
    /// This allows O(1) indexed iteration instead of using nth() which is O(n).
    /// Returns None if the index is out of bounds.
    ///
    /// OPTIMIZATION: Manual bounds check + unchecked access.
    /// Queries call this in a tight loop, so eliminating bounds checks helps.
    #[inline(always)]
    pub(crate) fn get_dense_entity(&self, index: usize) -> Option<Entity> {
        // SAFETY: Explicit bounds check before unchecked access
        if index < self.dense.len() {
            Some(unsafe { *self.dense.get_unchecked(index) })
        } else {
            None
        }
    }

    /// Get multiple components in a batch (optimized for SIMD processing)
    ///
    /// Returns up to `N` components starting from `start_index`.
    /// This is more cache-friendly than individual gets as it accesses
    /// the dense array sequentially.
    ///
    /// OPTIMIZATION: Sequential access pattern maximizes cache hits.
    #[inline]
    pub fn get_batch<const N: usize>(&self, start_index: usize) -> Option<([Entity; N], [&T; N])> {
        if start_index + N > self.dense.len() {
            return None;
        }

        // SAFETY: We verified bounds above
        unsafe {
            let mut entities = std::mem::MaybeUninit::<[Entity; N]>::uninit();
            let mut components = std::mem::MaybeUninit::<[&T; N]>::uninit();

            let entities_ptr = entities.as_mut_ptr() as *mut Entity;
            let components_ptr = components.as_mut_ptr() as *mut &T;

            for i in 0..N {
                let idx = start_index + i;
                let entity = *self.dense.get_unchecked(idx);
                let component = self.components.get_unchecked(idx);

                entities_ptr.add(i).write(entity);
                components_ptr.add(i).write(component);
            }

            Some((entities.assume_init(), components.assume_init()))
        }
    }

    /// Get raw pointers to dense arrays for unsafe batch processing
    ///
    /// Returns (entities_ptr, components_ptr, len).
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - Pointers are not used beyond the lifetime of the SparseSet
    /// - No mutable access occurs while these pointers are live
    /// - Index access is bounds-checked
    #[inline]
    #[allow(dead_code)]
    pub(crate) unsafe fn get_dense_ptrs(&self) -> (*const Entity, *const T, usize) {
        (self.dense.as_ptr(), self.components.as_ptr(), self.dense.len())
    }
}

impl<T: Component> Default for SparseSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Component> ComponentStorage for SparseSet<T> {
    #[inline]
    fn remove_entity(&mut self, entity: Entity) -> bool {
        self.remove(entity).is_some()
    }

    #[inline]
    fn contains_entity(&self, entity: Entity) -> bool {
        self.contains(entity)
    }

    #[inline(always)]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline(always)]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_component_data(&self, entity: Entity) -> Option<crate::serialization::ComponentData> {
        // Import ComponentData and all component types
        use crate::serialization::ComponentData;
        use crate::gameplay::Health;
        use crate::math::Transform;
        use crate::physics_components::Velocity;
        use crate::rendering::MeshRenderer;
        use std::any::TypeId;

        let type_id = TypeId::of::<T>();

        // Match on the type and convert to ComponentData
        if type_id == TypeId::of::<Transform>() {
            let storage = self.as_any().downcast_ref::<SparseSet<Transform>>()?;
            storage.get(entity).cloned().map(ComponentData::Transform)
        } else if type_id == TypeId::of::<Health>() {
            let storage = self.as_any().downcast_ref::<SparseSet<Health>>()?;
            storage.get(entity).cloned().map(ComponentData::Health)
        } else if type_id == TypeId::of::<Velocity>() {
            let storage = self.as_any().downcast_ref::<SparseSet<Velocity>>()?;
            storage.get(entity).cloned().map(ComponentData::Velocity)
        } else if type_id == TypeId::of::<MeshRenderer>() {
            let storage = self.as_any().downcast_ref::<SparseSet<MeshRenderer>>()?;
            storage.get(entity).cloned().map(ComponentData::MeshRenderer)
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Health {
        current: f32,
        max: f32,
    }

    impl Component for Health {}

    // Implement Component for i32 for testing purposes
    impl Component for i32 {}

    #[test]
    fn test_sparse_set_insert_get() {
        let mut storage = SparseSet::<Health>::new();
        let entity = Entity::new(0, 0);

        storage.insert(entity, Health { current: 100.0, max: 100.0 });

        let health = storage.get(entity).unwrap();
        assert_eq!(health.current, 100.0);
    }

    #[test]
    fn test_sparse_set_remove() {
        let mut storage = SparseSet::<Health>::new();
        let e1 = Entity::new(0, 0);
        let e2 = Entity::new(1, 0);

        storage.insert(e1, Health { current: 100.0, max: 100.0 });
        storage.insert(e2, Health { current: 50.0, max: 100.0 });

        storage.remove(e1);

        assert!(storage.get(e1).is_none());
        assert!(storage.get(e2).is_some());
    }

    #[test]
    fn test_sparse_set_iteration() {
        let mut storage = SparseSet::<i32>::new();

        for i in 0..100 {
            storage.insert(Entity::new(i, 0), i as i32);
        }

        let count: usize = storage.iter().count();
        assert_eq!(count, 100);
    }

    #[test]
    fn test_sparse_set_replace() {
        let mut storage = SparseSet::<Health>::new();
        let entity = Entity::new(0, 0);

        storage.insert(entity, Health { current: 100.0, max: 100.0 });
        storage.insert(entity, Health { current: 50.0, max: 100.0 });

        assert_eq!(storage.len(), 1);
        assert_eq!(storage.get(entity).unwrap().current, 50.0);
    }

    #[test]
    fn test_sparse_set_swap_remove() {
        let mut storage = SparseSet::<i32>::new();

        // Insert multiple components
        let e0 = Entity::new(0, 0);
        let e1 = Entity::new(1, 0);
        let e2 = Entity::new(2, 0);

        storage.insert(e0, 100);
        storage.insert(e1, 200);
        storage.insert(e2, 300);

        // Remove middle element
        let removed = storage.remove(e1);
        assert_eq!(removed, Some(200));

        // Verify other elements still accessible
        assert_eq!(storage.get(e0), Some(&100));
        assert_eq!(storage.get(e2), Some(&300));
        assert_eq!(storage.len(), 2);
    }

    #[test]
    fn test_sparse_set_contains() {
        let mut storage = SparseSet::<i32>::new();
        let entity = Entity::new(0, 0);

        assert!(!storage.contains(entity));

        storage.insert(entity, 42);
        assert!(storage.contains(entity));

        storage.remove(entity);
        assert!(!storage.contains(entity));
    }

    #[test]
    fn test_sparse_set_get_mut() {
        let mut storage = SparseSet::<Health>::new();
        let entity = Entity::new(0, 0);

        storage.insert(entity, Health { current: 100.0, max: 100.0 });

        if let Some(health) = storage.get_mut(entity) {
            health.current = 50.0;
        }

        assert_eq!(storage.get(entity).unwrap().current, 50.0);
    }

    #[test]
    fn test_sparse_set_clear() {
        let mut storage = SparseSet::<i32>::new();

        for i in 0..10 {
            storage.insert(Entity::new(i, 0), i as i32);
        }

        assert_eq!(storage.len(), 10);

        storage.clear();

        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
    }

    #[test]
    fn test_sparse_set_with_capacity() {
        let storage = SparseSet::<i32>::with_capacity(100);
        assert_eq!(storage.len(), 0);
        // Can't directly test capacity, but it shouldn't panic
    }

    #[test]
    fn test_sparse_set_iter_mut() {
        let mut storage = SparseSet::<i32>::new();

        for i in 0..10 {
            storage.insert(Entity::new(i, 0), i as i32);
        }

        // Double all values
        for (_entity, value) in storage.iter_mut() {
            *value *= 2;
        }

        for (i, (_entity, &value)) in storage.iter().enumerate() {
            assert_eq!(value, (i as i32) * 2);
        }
    }

    #[test]
    fn test_sparse_set_sparse_ids() {
        let mut storage = SparseSet::<i32>::new();

        // Use sparse entity IDs
        storage.insert(Entity::new(0, 0), 100);
        storage.insert(Entity::new(100, 0), 200);
        storage.insert(Entity::new(1000, 0), 300);

        assert_eq!(storage.get(Entity::new(0, 0)), Some(&100));
        assert_eq!(storage.get(Entity::new(100, 0)), Some(&200));
        assert_eq!(storage.get(Entity::new(1000, 0)), Some(&300));
        assert_eq!(storage.len(), 3);
    }
}
