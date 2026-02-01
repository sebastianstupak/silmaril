//! Entity management with generational indices
//!
//! Entities are unique identifiers for game objects. This module provides
//! generational indices to safely handle entity deletion and prevent use-after-free bugs.

use serde::{Deserialize, Serialize};

/// Entity handle - opaque, copyable, hashable
///
/// Entities use generational indices to prevent use-after-free bugs.
/// When an entity is freed, its generation is incremented, invalidating
/// all old handles to that entity.
///
/// # Memory Layout Optimization
///
/// Entity is exactly 8 bytes (2 × u32) with no padding. This ensures:
/// - Cache-friendly: 8 entities fit in a 64-byte cache line
/// - Efficient copying: Single 64-bit load/store on modern CPUs
/// - Dense packing: No wasted space in arrays
///
/// # Examples
///
/// ```
/// # use engine_core::ecs::entity::{Entity, EntityAllocator};
/// let mut allocator = EntityAllocator::new();
/// let entity = allocator.allocate();
/// assert!(allocator.is_alive(entity));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(C)] // Ensure consistent layout across platforms
pub struct Entity {
    id: u32,
    generation: u32,
}

impl Entity {
    /// Get the entity's ID (index in the allocator)
    ///
    /// Note: The ID may be reused after the entity is freed, so always
    /// check generation when validating entity handles.
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get the entity's generation
    ///
    /// The generation increments each time an entity ID is reused.
    #[inline]
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Create a new entity (internal use only, prefer EntityAllocator)
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn new(id: u32, generation: u32) -> Self {
        Self { id, generation }
    }
}

/// Allocates and tracks entities
///
/// Uses a free list to efficiently reuse entity IDs while maintaining
/// generational safety.
///
/// # Examples
///
/// ```
/// # use engine_core::ecs::entity::EntityAllocator;
/// let mut allocator = EntityAllocator::new();
///
/// // Allocate entities
/// let e1 = allocator.allocate();
/// let e2 = allocator.allocate();
///
/// // Free an entity
/// allocator.free(e1);
///
/// // ID is reused but generation differs
/// let e3 = allocator.allocate();
/// assert_eq!(e1.id(), e3.id());
/// assert_ne!(e1.generation(), e3.generation());
/// ```
pub struct EntityAllocator {
    generations: Vec<u32>,
    free_list: Vec<u32>,
}

impl EntityAllocator {
    /// Create a new entity allocator
    pub fn new() -> Self {
        Self { generations: Vec::new(), free_list: Vec::new() }
    }

    /// Allocate a new entity or reuse a freed one
    ///
    /// If there are freed entity IDs in the free list, this will reuse
    /// the most recently freed ID with an incremented generation.
    /// Otherwise, it allocates a new ID.
    ///
    /// # Panics
    ///
    /// Panics if the maximum number of entities (2^32) is exceeded.
    #[inline]
    pub fn allocate(&mut self) -> Entity {
        if let Some(id) = self.free_list.pop() {
            // Reuse freed ID with incremented generation
            let id_usize = id as usize;

            // Extra defensive: verify the ID is valid in debug mode only
            debug_assert!(
                id_usize < self.generations.len(),
                "Free list contained invalid ID: {}",
                id
            );

            // SAFETY: We just verified the ID is valid in debug mode
            // In release, we trust the free_list only contains valid IDs
            let generation = unsafe { *self.generations.get_unchecked(id_usize) };

            Entity { id, generation }
        } else {
            // Allocate new ID
            let id = self.generations.len();

            // Extra defensive: check for overflow in debug mode only
            debug_assert!(
                id <= u32::MAX as usize,
                "Entity ID overflow: cannot allocate more than {} entities",
                u32::MAX
            );

            let id = id as u32;
            self.generations.push(0);
            Entity { id, generation: 0 }
        }
    }

    /// Free an entity (doesn't delete immediately, increments generation)
    ///
    /// Returns `true` if the entity was alive and successfully freed,
    /// `false` if the entity was already dead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::entity::EntityAllocator;
    /// let mut allocator = EntityAllocator::new();
    /// let entity = allocator.allocate();
    ///
    /// assert!(allocator.free(entity));
    /// assert!(!allocator.free(entity)); // Already freed
    /// ```
    #[inline]
    pub fn free(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        let id = entity.id as usize;

        // Extra defensive: verify generation won't overflow in debug mode only
        debug_assert!(
            self.generations[id] < u32::MAX,
            "Generation overflow for entity ID {}: generation {}",
            entity.id,
            self.generations[id]
        );

        // SAFETY: is_alive() already verified this ID is valid
        unsafe {
            *self.generations.get_unchecked_mut(id) += 1;
        }
        self.free_list.push(entity.id);

        true
    }

    /// Allocate multiple entities in a single batch
    ///
    /// This is more efficient than calling allocate() repeatedly because it:
    /// - Reduces per-allocation overhead
    /// - Improves cache locality by processing allocations together
    /// - Can pre-allocate the output vector with the exact size needed
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::entity::EntityAllocator;
    /// let mut allocator = EntityAllocator::new();
    /// let entities = allocator.allocate_batch(100);
    /// assert_eq!(entities.len(), 100);
    /// for entity in &entities {
    ///     assert!(allocator.is_alive(*entity));
    /// }
    /// ```
    #[inline]
    pub fn allocate_batch(&mut self, count: usize) -> Vec<Entity> {
        let mut entities = Vec::with_capacity(count);

        // First, drain as many from the free list as possible
        let from_free_list = count.min(self.free_list.len());
        for _ in 0..from_free_list {
            // SAFETY: We know free_list has at least from_free_list elements
            let id = unsafe { self.free_list.pop().unwrap_unchecked() };
            let id_usize = id as usize;

            debug_assert!(
                id_usize < self.generations.len(),
                "Free list contained invalid ID: {}",
                id
            );

            let generation = unsafe { *self.generations.get_unchecked(id_usize) };
            entities.push(Entity { id, generation });
        }

        // Allocate the rest as new IDs
        let remaining = count - from_free_list;
        if remaining > 0 {
            let start_id = self.generations.len();
            debug_assert!(
                start_id + remaining <= u32::MAX as usize,
                "Entity ID overflow: cannot allocate {} more entities",
                remaining
            );

            // Reserve space for all new generations at once
            self.generations.reserve(remaining);

            for i in 0..remaining {
                let id = (start_id + i) as u32;
                self.generations.push(0);
                entities.push(Entity { id, generation: 0 });
            }
        }

        entities
    }

    /// Check if entity handle is still valid
    ///
    /// An entity is alive if its ID exists and the generation matches.
    ///
    /// This is called extremely frequently in queries, so it's optimized
    /// to avoid unnecessary bounds checking and branching.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::entity::EntityAllocator;
    /// let mut allocator = EntityAllocator::new();
    /// let entity = allocator.allocate();
    ///
    /// assert!(allocator.is_alive(entity));
    /// allocator.free(entity);
    /// assert!(!allocator.is_alive(entity));
    /// ```
    #[inline(always)]
    pub fn is_alive(&self, entity: Entity) -> bool {
        let id = entity.id as usize;
        // Fast path: bounds check then direct comparison
        // This is faster than .get().map() chain due to reduced branching
        id < self.generations.len() && unsafe {
            // SAFETY: We just verified id < len
            *self.generations.get_unchecked(id) == entity.generation
        }
    }

    /// Get the number of allocated entities (alive + freed)
    ///
    /// This is the total number of entity IDs that have been created,
    /// not the number of currently alive entities.
    #[inline]
    pub fn len(&self) -> usize {
        self.generations.len()
    }

    /// Check if no entities have been allocated
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.generations.is_empty()
    }

    /// Get the number of currently alive entities
    #[inline]
    pub fn alive_count(&self) -> usize {
        self.generations.len() - self.free_list.len()
    }

    /// Clear all entities
    ///
    /// This removes all entity data and resets the allocator to its initial state.
    #[inline]
    pub fn clear(&mut self) {
        self.generations.clear();
        self.free_list.clear();
    }
}

impl Default for EntityAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_allocate() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.allocate();
        let e2 = alloc.allocate();

        assert_ne!(e1.id, e2.id);
        assert!(alloc.is_alive(e1));
        assert!(alloc.is_alive(e2));
    }

    #[test]
    fn test_entity_free_and_reuse() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.allocate();

        alloc.free(e1);
        assert!(!alloc.is_alive(e1));

        // Reuse same ID, different generation
        let e2 = alloc.allocate();
        assert_eq!(e1.id, e2.id);
        assert_ne!(e1.generation, e2.generation);
        assert!(alloc.is_alive(e2));
        assert!(!alloc.is_alive(e1)); // Old handle invalid
    }

    #[test]
    fn test_many_entities() {
        let mut alloc = EntityAllocator::new();
        let entities: Vec<_> = (0..10_000).map(|_| alloc.allocate()).collect();

        // All unique IDs
        let mut ids: Vec<_> = entities.iter().map(|e| e.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 10_000);
    }

    #[test]
    fn test_free_twice() {
        let mut alloc = EntityAllocator::new();
        let entity = alloc.allocate();

        assert!(alloc.free(entity));
        assert!(!alloc.free(entity)); // Should return false
    }

    #[test]
    fn test_is_alive_invalid_id() {
        let alloc = EntityAllocator::new();
        let fake_entity = Entity::new(999, 0);

        assert!(!alloc.is_alive(fake_entity));
    }

    #[test]
    fn test_alive_count() {
        let mut alloc = EntityAllocator::new();

        assert_eq!(alloc.alive_count(), 0);

        let e1 = alloc.allocate();
        let e2 = alloc.allocate();
        let e3 = alloc.allocate();

        assert_eq!(alloc.alive_count(), 3);

        alloc.free(e2);
        assert_eq!(alloc.alive_count(), 2);

        alloc.free(e1);
        alloc.free(e3);
        assert_eq!(alloc.alive_count(), 0);
    }

    #[test]
    fn test_clear() {
        let mut alloc = EntityAllocator::new();

        alloc.allocate();
        alloc.allocate();

        assert_eq!(alloc.len(), 2);

        alloc.clear();

        assert_eq!(alloc.len(), 0);
        assert_eq!(alloc.alive_count(), 0);
        assert!(alloc.is_empty());
    }

    #[test]
    fn test_generation_increment() {
        let mut alloc = EntityAllocator::new();
        let e1 = alloc.allocate();

        assert_eq!(e1.generation(), 0);

        alloc.free(e1);
        let e2 = alloc.allocate();

        assert_eq!(e2.generation(), 1);

        alloc.free(e2);
        let e3 = alloc.allocate();

        assert_eq!(e3.generation(), 2);
    }

    #[test]
    fn test_entity_copy_clone() {
        let entity = Entity::new(42, 7);
        let copied = entity;
        let cloned = entity.clone();

        assert_eq!(entity, copied);
        assert_eq!(entity, cloned);
    }

    #[test]
    fn test_entity_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        let e1 = Entity::new(1, 0);
        let e2 = Entity::new(2, 0);
        let e3 = Entity::new(1, 1); // Same ID, different generation

        set.insert(e1);
        set.insert(e2);
        set.insert(e3);

        assert_eq!(set.len(), 3);
        assert!(set.contains(&e1));
        assert!(set.contains(&e2));
        assert!(set.contains(&e3));
    }

    #[test]
    fn test_allocate_batch_new() {
        let mut alloc = EntityAllocator::new();
        let entities = alloc.allocate_batch(100);

        assert_eq!(entities.len(), 100);

        // All entities should be alive
        for entity in &entities {
            assert!(alloc.is_alive(*entity));
        }

        // All IDs should be unique
        let mut ids: Vec<_> = entities.iter().map(|e| e.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 100);
    }

    #[test]
    fn test_allocate_batch_with_free_list() {
        let mut alloc = EntityAllocator::new();

        // Allocate and free 50 entities to populate free list
        let first_batch = alloc.allocate_batch(50);
        for entity in first_batch {
            alloc.free(entity);
        }

        // Batch allocate 100 entities (50 from free list, 50 new)
        let entities = alloc.allocate_batch(100);

        assert_eq!(entities.len(), 100);

        // All entities should be alive
        for entity in &entities {
            assert!(alloc.is_alive(*entity));
        }

        // First 50 should have generation 1, last 50 should have generation 0
        let gen_1_count = entities.iter().filter(|e| e.generation() == 1).count();
        let gen_0_count = entities.iter().filter(|e| e.generation() == 0).count();
        assert_eq!(gen_1_count, 50);
        assert_eq!(gen_0_count, 50);
    }

    #[test]
    fn test_allocate_batch_empty() {
        let mut alloc = EntityAllocator::new();
        let entities = alloc.allocate_batch(0);
        assert_eq!(entities.len(), 0);
    }
}
