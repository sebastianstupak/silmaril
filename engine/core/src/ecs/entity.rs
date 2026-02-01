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
/// # Examples
///
/// ```
/// # use engine_core::ecs::entity::{Entity, EntityAllocator};
/// let mut allocator = EntityAllocator::new();
/// let entity = allocator.allocate();
/// assert!(allocator.is_alive(entity));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
        Self {
            generations: Vec::new(),
            free_list: Vec::new(),
        }
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
    pub fn allocate(&mut self) -> Entity {
        if let Some(id) = self.free_list.pop() {
            // Reuse freed ID with incremented generation
            let id_usize = id as usize;

            // Extra defensive: verify the ID is valid
            assert!(
                id_usize < self.generations.len(),
                "Free list contained invalid ID: {}",
                id
            );

            Entity {
                id,
                generation: self.generations[id_usize],
            }
        } else {
            // Allocate new ID
            let id = self.generations.len();

            // Extra defensive: check for overflow
            assert!(
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
    pub fn free(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        let id = entity.id as usize;

        // Extra defensive: verify generation won't overflow
        let current_gen = self.generations[id];
        assert!(
            current_gen < u32::MAX,
            "Generation overflow for entity ID {}: generation {}",
            entity.id,
            current_gen
        );

        // Increment generation to invalidate old handles
        self.generations[id] += 1;
        self.free_list.push(entity.id);

        true
    }

    /// Check if entity handle is still valid
    ///
    /// An entity is alive if its ID exists and the generation matches.
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
    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations
            .get(entity.id as usize)
            .map(|&gen| gen == entity.generation)
            .unwrap_or(false)
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
    pub fn alive_count(&self) -> usize {
        self.generations.len() - self.free_list.len()
    }

    /// Clear all entities
    ///
    /// This removes all entity data and resets the allocator to its initial state.
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
}
