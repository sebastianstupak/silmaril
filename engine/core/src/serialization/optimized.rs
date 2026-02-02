//! Optimized serialization implementations
//!
//! Performance optimizations:
//! - FxHashMap instead of HashMap (faster hashing)
//! - Pre-allocated buffers
//! - Reduced allocations
//! - Parallel processing for large worlds

use super::{ComponentData, EntityMetadata, WorldMetadata, WorldState};
use crate::ecs::{Entity, World};
use rustc_hash::FxHashMap;

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

impl WorldState {
    /// Create snapshot with pre-allocated capacity
    ///
    /// Use this when you know the approximate entity count for better performance.
    pub fn snapshot_with_capacity(world: &World, capacity: usize) -> Self {
        #[cfg(feature = "profiling")]
        profile_scope!("world_snapshot_optimized", ProfileCategory::Serialization);

        // Pre-allocate with known capacity to avoid reallocation
        let mut entities = Vec::with_capacity(capacity);
        let mut components = std::collections::HashMap::with_capacity(capacity);

        // Single iteration over entities
        for entity in world.entities() {
            if !world.is_alive(entity) {
                continue;
            }

            entities.push(EntityMetadata { entity, generation: entity.generation(), alive: true });

            let entity_components = world.get_all_components(entity);
            if !entity_components.is_empty() {
                components.insert(entity, entity_components);
            }
        }

        let entity_count = entities.len();
        let component_count: usize = components.values().map(|v| v.len()).sum();

        Self {
            entities,
            components,
            metadata: WorldMetadata {
                version: 1,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                entity_count,
                component_count,
            },
        }
    }

    /// Restore with optimized processing
    ///
    /// Uses FxHashMap for faster lookups during restoration.
    pub fn restore_optimized(&self, world: &mut World) {
        #[cfg(feature = "profiling")]
        profile_scope!("world_restore_optimized", ProfileCategory::Serialization);

        world.clear();

        // Convert to FxHashMap for faster lookups
        let components_map: FxHashMap<Entity, &Vec<ComponentData>> =
            self.components.iter().map(|(k, v)| (*k, v)).collect();

        // Restore entities
        for entity_meta in &self.entities {
            if !entity_meta.alive {
                continue;
            }

            world.spawn_with_id(entity_meta.entity);
            let entity = entity_meta.entity;

            if let Some(components) = components_map.get(&entity) {
                for component in components.iter() {
                    world.add_component_data(entity, component.clone());
                }
            }
        }
    }
}

/// Optimized batch serialization for multiple entities
///
/// More efficient than serializing entities one-by-one.
pub struct BatchSerializer {
    buffer: Vec<u8>,
    #[allow(dead_code)]
    capacity: usize,
}

impl BatchSerializer {
    /// Create a new batch serializer with initial capacity
    pub fn new(capacity: usize) -> Self {
        Self { buffer: Vec::with_capacity(capacity), capacity }
    }

    /// Serialize a batch of world states
    ///
    /// Re-uses internal buffer to reduce allocations.
    pub fn serialize_batch(&mut self, states: &[WorldState]) -> Vec<Vec<u8>> {
        states
            .iter()
            .map(|state| {
                self.buffer.clear();
                // Serialize into buffer
                bincode::serialize_into(&mut self.buffer, state).unwrap();
                self.buffer.clone()
            })
            .collect()
    }

    /// Clear internal buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;
    use crate::math::Transform;

    #[test]
    fn test_snapshot_with_capacity() {
        let mut world = World::new();
        world.register::<Transform>();

        // Spawn 100 entities
        for _ in 0..100 {
            let entity = world.spawn();
            world.add(entity, Transform::default());
        }

        let snapshot = WorldState::snapshot_with_capacity(&world, 100);
        assert_eq!(snapshot.entities.len(), 100);
        assert_eq!(snapshot.components.len(), 100);
    }

    #[test]
    fn test_restore_optimized() {
        let mut world1 = World::new();
        world1.register::<Transform>();
        for _ in 0..50 {
            let entity = world1.spawn();
            world1.add(
                entity,
                Transform::new(
                    crate::math::Vec3::new(1.0, 2.0, 3.0),
                    crate::math::Quat::IDENTITY,
                    crate::math::Vec3::ONE,
                ),
            );
        }

        let snapshot = WorldState::snapshot_with_capacity(&world1, 50);

        let mut world2 = World::new();
        world2.register::<Transform>();
        snapshot.restore_optimized(&mut world2);

        assert_eq!(world2.entity_count(), 50);
    }

    #[test]
    fn test_batch_serializer() {
        let state1 = WorldState::new();
        let state2 = WorldState::new();

        let mut serializer = BatchSerializer::new(1024);
        let batches = serializer.serialize_batch(&[state1, state2]);

        assert_eq!(batches.len(), 2);
        assert!(batches[0].len() > 0);
        assert!(batches[1].len() > 0);
    }
}
