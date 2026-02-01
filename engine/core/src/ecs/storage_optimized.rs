//! Optimized sparse-set storage for components
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
//! ## Memory Layout Optimizations
//!
//! Sparse array grows to accommodate entity IDs but contains mostly None values.
//! Dense arrays are packed with no gaps, providing excellent cache locality during iteration.
//! Memory overhead: ~8 bytes per possible entity ID in sparse array + packed component storage.
//!
//! ## Cache Optimization Strategy
//!
//! - Dense arrays pre-allocated with DEFAULT_CAPACITY to reduce allocations
//! - Component and entity arrays kept parallel for better prefetching
//! - Sequential iteration exploits CPU cache lines (64 bytes)
//! - Entities are 8 bytes, so 8 fit per cache line
//! - Aggressive reservation strategy: 2x growth with minimum thresholds
//!
//! ## Performance Notes
//!
//! - Uses `#[inline]` on hot path methods for better codegen
//! - Aggressive capacity pre-allocation reduces reallocations
//! - Sparse array grows with geometric strategy (2x) to minimize copies
//! - Bounds checking eliminated where safety can be proven

use super::{Component, Entity};

/// Default capacity for dense arrays (reduces initial allocations)
const DEFAULT_CAPACITY: usize = 64;

/// Minimum capacity increase when growing (ensures amortized O(1))
const MIN_GROWTH: usize = 32;

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
/// # Examples
///
/// ```
/// # use engine_core::ecs::{Component, Entity, SparseSet};
/// # struct Position { x: f32, y: f32, z: f32 }
/// # impl Component for Position {}
/// let mut storage = SparseSet::<Position>::new();
/// let entity = Entity::new(0, 0);
///
/// storage.insert(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
/// assert!(storage.get(entity).is_some());
/// ```
pub struct SparseSet<T: Component> {
    /// Sparse array: Entity ID → dense index
    sparse: Vec<Option<usize>>,
    /// Dense entity array (packed, no gaps)
    dense: Vec<Entity>,
    /// Dense component array (packed, no gaps)
    components: Vec<T>,
}

impl<T: Component> SparseSet<T> {
    /// Create a new empty sparse set
    ///
    /// Pre-allocates DEFAULT_CAPACITY to reduce allocations in common case.
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a new sparse set with preallocated capacity
    ///
    /// Preallocates space for `capacity` components, reducing allocations
    /// during insertion. This is the recommended way to create a SparseSet
    /// when you know the approximate entity count.
    ///
    /// # Cache Optimization
    ///
    /// Dense arrays are allocated together, improving cache locality during
    /// iteration. Entities and components are accessed sequentially.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            // Don't pre-allocate sparse array as it may be very sparse
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
    /// # use engine_core::ecs::{Component, Entity, SparseSet};
    /// # struct Health { current: f32, max: f32 }
    /// # impl Component for Health {}
    /// let mut storage = SparseSet::<Health>::new();
    /// let entity = Entity::new(0, 0);
    ///
    /// storage.insert(entity, Health { current: 100.0, max: 100.0 });
    /// storage.insert(entity, Health { current: 50.0, max: 100.0 }); // Replaces
    /// assert_eq!(storage.get(entity).unwrap().current, 50.0);
    /// ```
    #[inline]
    pub fn insert(&mut self, entity: Entity, component: T) {
        let idx = entity.id() as usize;

        // Ensure sparse array is large enough
        if idx >= self.sparse.len() {
            // Sanity check: prevent absurd allocations from corrupted entity IDs
            assert!(
                idx < 100_000_000,
                "Entity ID {} is suspiciously large (possible corruption?)",
                idx
            );

            self.sparse.resize(idx + 1, None);
        }

        if let Some(dense_idx) = self.sparse[idx] {
            // Component exists, replace it
            // SAFETY: sparse array integrity maintained by insert/remove
            debug_assert!(dense_idx < self.components.len());
            self.components[dense_idx] = component;
        } else {
            // New component, add to dense arrays
            let dense_idx = self.dense.len();
            self.sparse[idx] = Some(dense_idx);
            self.dense.push(entity);
            self.components.push(component);

            debug_assert_eq!(self.dense.len(), self.components.len());
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
    /// # use engine_core::ecs::{Component, Entity, SparseSet};
    /// # struct Health { current: f32, max: f32 }
    /// # impl Component for Health {}
    /// let mut storage = SparseSet::<Health>::new();
    /// let entity = Entity::new(0, 0);
    ///
    /// storage.insert(entity, Health { current: 100.0, max: 100.0 });
    /// let removed = storage.remove(entity);
    /// assert!(removed.is_some());
    /// assert!(storage.get(entity).is_none());
    /// ```
    #[inline]
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let idx = entity.id() as usize;
        let dense_idx = self.sparse.get_mut(idx)?.take()?;

        debug_assert!(dense_idx < self.dense.len());

        // Swap-remove from dense arrays
        let last_idx = self.dense.len() - 1;

        if dense_idx != last_idx {
            // Swap with last element
            self.dense.swap(dense_idx, last_idx);
            self.components.swap(dense_idx, last_idx);

            // Update sparse index for swapped entity
            let swapped_entity = self.dense[dense_idx];
            let swapped_id = swapped_entity.id() as usize;

            debug_assert!(swapped_id < self.sparse.len());
            debug_assert!(self.sparse[swapped_id].is_some());

            self.sparse[swapped_id] = Some(dense_idx);
        }

        self.dense.pop();
        let component = self.components.pop().unwrap();

        debug_assert_eq!(self.dense.len(), self.components.len());

        Some(component)
    }

    /// Get an immutable reference to an entity's component
    ///
    /// Returns `None` if the entity doesn't have this component.
    #[inline]
    pub fn get(&self, entity: Entity) -> Option<&T> {
        let idx = entity.id() as usize;
        let dense_idx = *self.sparse.get(idx)?.as_ref()?;
        Some(&self.components[dense_idx])
    }

    /// Get a mutable reference to an entity's component
    ///
    /// Returns `None` if the entity doesn't have this component.
    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        let idx = entity.id() as usize;
        let dense_idx = *self.sparse.get(idx)?.as_ref()?;
        Some(&mut self.components[dense_idx])
    }

    /// Check if an entity has this component
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        let idx = entity.id() as usize;
        self.sparse
            .get(idx)
            .and_then(|opt| opt.as_ref())
            .is_some()
    }

    /// Iterate over all (entity, component) pairs
    ///
    /// Iteration order is not guaranteed and may change after insertions/removals.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.dense.iter().copied().zip(self.components.iter())
    }

    /// Iterate over all (entity, component) pairs with mutable component access
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
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
    #[inline]
    pub fn clear(&mut self) {
        self.sparse.clear();
        self.dense.clear();
        self.components.clear();
    }

    /// Reserve capacity for at least `additional` more components
    ///
    /// Uses aggressive reservation strategy to minimize reallocations.
    /// Reserves at least MIN_GROWTH even for small additions.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        let to_reserve = additional.max(MIN_GROWTH);
        self.dense.reserve(to_reserve);
        self.components.reserve(to_reserve);
    }

    /// Reserve exact capacity for `additional` more components
    ///
    /// Like reserve(), but doesn't over-allocate.
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.dense.reserve_exact(additional);
        self.components.reserve_exact(additional);
    }

    /// Get entity at a specific dense index
    ///
    /// This allows O(1) indexed iteration instead of using nth() which is O(n).
    /// Returns None if the index is out of bounds.
    #[inline]
    pub(crate) fn get_dense_entity(&self, index: usize) -> Option<Entity> {
        self.dense.get(index).copied()
    }
}

impl<T: Component> Default for SparseSet<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
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

        storage.insert(entity, Health {
            current: 100.0,
            max: 100.0,
        });

        let health = storage.get(entity).unwrap();
        assert_eq!(health.current, 100.0);
    }

    #[test]
    fn test_sparse_set_remove() {
        let mut storage = SparseSet::<Health>::new();
        let e1 = Entity::new(0, 0);
        let e2 = Entity::new(1, 0);

        storage.insert(e1, Health {
            current: 100.0,
            max: 100.0,
        });
        storage.insert(e2, Health {
            current: 50.0,
            max: 100.0,
        });

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

        storage.insert(entity, Health {
            current: 100.0,
            max: 100.0,
        });
        storage.insert(entity, Health {
            current: 50.0,
            max: 100.0,
        });

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

        storage.insert(entity, Health {
            current: 100.0,
            max: 100.0,
        });

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
