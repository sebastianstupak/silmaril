//! Network snapshot for world state synchronization
//!
//! This module provides efficient snapshot creation and application for
//! synchronizing world state over the network. It wraps the core WorldState
//! serialization with network-specific optimizations.

use engine_core::ecs::{Entity, World};
use engine_core::serialization::{Format, Serializable, SerializationError, WorldState};

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::profile_scope;

/// Network snapshot of world state
///
/// This is a thin wrapper around WorldState that provides network-specific
/// functionality like compression and delta encoding (future).
#[derive(Debug, Clone)]
pub struct WorldSnapshot {
    /// Underlying world state
    state: WorldState,
}

impl WorldSnapshot {
    /// Create a snapshot from a World
    ///
    /// Captures the complete state of all entities and components.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_core::ecs::World;
    /// # use engine_networking::snapshot::WorldSnapshot;
    /// let world = World::new();
    /// let snapshot = WorldSnapshot::from_world(&world);
    /// ```
    pub fn from_world(world: &World) -> Self {
        #[cfg(feature = "profiling")]
        profile_scope!("snapshot_from_world");

        Self { state: WorldState::snapshot(world) }
    }

    /// Apply snapshot to a World
    ///
    /// Clears the world and restores all entities and components from the snapshot.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_core::ecs::World;
    /// # use engine_networking::snapshot::WorldSnapshot;
    /// # let snapshot = WorldSnapshot::from_world(&World::new());
    /// let mut world = World::new();
    /// snapshot.apply_to_world(&mut world);
    /// ```
    pub fn apply_to_world(&self, world: &mut World) {
        #[cfg(feature = "profiling")]
        profile_scope!("snapshot_apply_to_world");

        self.state.restore(world);
    }

    /// Serialize snapshot to bytes using the specified format
    ///
    /// For network transmission, Bincode is recommended for performance.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_core::ecs::World;
    /// # use engine_core::serialization::Format;
    /// # use engine_networking::snapshot::WorldSnapshot;
    /// # let snapshot = WorldSnapshot::from_world(&World::new());
    /// let bytes = snapshot.to_bytes(Format::Bincode).unwrap();
    /// ```
    pub fn to_bytes(&self, format: Format) -> Result<Vec<u8>, SerializationError> {
        #[cfg(feature = "profiling")]
        profile_scope!("snapshot_to_bytes");

        self.state.serialize(format)
    }

    /// Deserialize snapshot from bytes
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_core::serialization::Format;
    /// # use engine_networking::snapshot::WorldSnapshot;
    /// # let bytes = vec![];
    /// let snapshot = WorldSnapshot::from_bytes(&bytes, Format::Bincode).unwrap();
    /// ```
    pub fn from_bytes(bytes: &[u8], format: Format) -> Result<Self, SerializationError> {
        #[cfg(feature = "profiling")]
        profile_scope!("snapshot_from_bytes");

        let state = WorldState::deserialize(bytes, format)?;
        Ok(Self { state })
    }

    /// Get entity count in snapshot
    pub fn entity_count(&self) -> usize {
        self.state.metadata.entity_count
    }

    /// Get component count in snapshot
    pub fn component_count(&self) -> usize {
        self.state.metadata.component_count
    }

    /// Get snapshot timestamp (Unix epoch)
    pub fn timestamp(&self) -> u64 {
        self.state.metadata.timestamp
    }

    /// Get snapshot version
    pub fn version(&self) -> u32 {
        self.state.metadata.version
    }

    /// Get list of entities in snapshot
    pub fn entities(&self) -> Vec<Entity> {
        self.state.entities.iter().filter(|e| e.alive).map(|e| e.entity).collect()
    }

    /// Get the underlying WorldState (for advanced use cases)
    pub fn state(&self) -> &WorldState {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::math::Transform;

    #[test]
    fn test_empty_snapshot() {
        let world = World::new();
        let snapshot = WorldSnapshot::from_world(&world);

        assert_eq!(snapshot.entity_count(), 0);
        assert_eq!(snapshot.component_count(), 0);
    }

    #[test]
    fn test_snapshot_roundtrip() {
        let mut world = World::new();
        world.register::<Transform>();

        let entity = world.spawn();
        world.add(entity, Transform::default());

        // Create snapshot
        let snapshot = WorldSnapshot::from_world(&world);
        assert_eq!(snapshot.entity_count(), 1);
        assert_eq!(snapshot.component_count(), 1);

        // Apply to new world
        let mut world2 = World::new();
        world2.register::<Transform>();
        snapshot.apply_to_world(&mut world2);

        // Verify
        assert_eq!(world2.entity_count(), 1);
    }

    #[test]
    fn test_snapshot_serialization_bincode() {
        let mut world = World::new();
        world.register::<Transform>();

        let entity = world.spawn();
        world.add(entity, Transform::default());

        // Snapshot -> Bytes
        let snapshot = WorldSnapshot::from_world(&world);
        let bytes = snapshot.to_bytes(Format::Bincode).unwrap();

        // Bytes -> Snapshot
        let snapshot2 = WorldSnapshot::from_bytes(&bytes, Format::Bincode).unwrap();

        assert_eq!(snapshot.entity_count(), snapshot2.entity_count());
        assert_eq!(snapshot.component_count(), snapshot2.component_count());
    }

    #[test]
    fn test_snapshot_serialization_yaml() {
        let mut world = World::new();
        world.register::<Transform>();

        let entity = world.spawn();
        world.add(entity, Transform::default());

        // Snapshot -> Bytes
        let snapshot = WorldSnapshot::from_world(&world);
        let bytes = snapshot.to_bytes(Format::Yaml).unwrap();

        // Should be human-readable
        let yaml_str = std::str::from_utf8(&bytes).unwrap();
        assert!(yaml_str.contains("entities"));

        // Bytes -> Snapshot
        let snapshot2 = WorldSnapshot::from_bytes(&bytes, Format::Yaml).unwrap();

        assert_eq!(snapshot.entity_count(), snapshot2.entity_count());
        assert_eq!(snapshot.component_count(), snapshot2.component_count());
    }

    #[test]
    fn test_snapshot_full_roundtrip() {
        let mut world1 = World::new();
        world1.register::<Transform>();

        // Create entities
        for _ in 0..10 {
            let entity = world1.spawn();
            world1.add(entity, Transform::default());
        }

        // World1 -> Snapshot -> Bytes
        let snapshot1 = WorldSnapshot::from_world(&world1);
        let bytes = snapshot1.to_bytes(Format::Bincode).unwrap();

        // Bytes -> Snapshot -> World2
        let snapshot2 = WorldSnapshot::from_bytes(&bytes, Format::Bincode).unwrap();
        let mut world2 = World::new();
        world2.register::<Transform>();
        snapshot2.apply_to_world(&mut world2);

        // Verify
        assert_eq!(world1.entity_count(), world2.entity_count());
        assert_eq!(world1.entity_count(), 10);
    }

    #[test]
    fn test_snapshot_entity_list() {
        let mut world = World::new();

        let e1 = world.spawn();
        let e2 = world.spawn();
        let e3 = world.spawn();

        let snapshot = WorldSnapshot::from_world(&world);
        let entities = snapshot.entities();

        assert_eq!(entities.len(), 3);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
        assert!(entities.contains(&e3));
    }
}
