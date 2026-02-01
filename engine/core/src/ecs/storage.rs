//! Sparse-set storage for components
//!
//! Provides O(1) insert/remove and cache-friendly iteration over components.
//! The sparse array maps entity IDs to indices in the dense arrays.

use super::{Component, Entity};

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
    pub fn new() -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::new(),
            components: Vec::new(),
        }
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
    pub fn insert(&mut self, entity: Entity, component: T) {
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
            debug_assert_eq!(
                self.dense.len(),
                self.components.len(),
                "Dense arrays out of sync"
            );
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
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
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
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.dense.iter().copied().zip(self.components.iter())
    }

    /// Iterate over all (entity, component) pairs with mutable component access
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
    pub fn clear(&mut self) {
        self.sparse.clear();
        self.dense.clear();
        self.components.clear();
    }

    /// Reserve capacity for at least `additional` more components
    pub fn reserve(&mut self, additional: usize) {
        self.dense.reserve(additional);
        self.components.reserve(additional);
    }
}

impl<T: Component> Default for SparseSet<T> {
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
