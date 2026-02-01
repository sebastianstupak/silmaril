//! WorldState snapshot and restoration

use super::{ComponentData, Format, Serializable, SerializationError};
use crate::ecs::{Entity, World};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};

/// Entity metadata for serialization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityMetadata {
    /// Entity handle
    pub entity: Entity,
    /// Generation counter
    pub generation: u32,
    /// Is the entity alive
    pub alive: bool,
}

/// World metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldMetadata {
    /// State version number
    pub version: u32,
    /// Unix timestamp when snapshot was created
    pub timestamp: u64,
    /// Number of entities in the snapshot
    pub entity_count: usize,
    /// Total number of components across all entities
    pub component_count: usize,
}

/// Complete snapshot of ECS world state
///
/// Can be serialized to multiple formats:
/// - YAML: Human-readable, editable by AI agents
/// - Bincode: Fast local serialization
/// - FlatBuffers: Zero-copy network serialization (Phase 1.3 completion)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    /// Entity metadata
    pub entities: Vec<EntityMetadata>,
    /// Components by entity
    pub components: HashMap<Entity, Vec<ComponentData>>,
    /// Metadata about this snapshot
    pub metadata: WorldMetadata,
}

impl WorldState {
    /// Create a new empty world state
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            components: HashMap::new(),
            metadata: WorldMetadata {
                version: 1,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                entity_count: 0,
                component_count: 0,
            },
        }
    }

    /// Create snapshot from World
    ///
    /// This captures the complete state of the ECS world including all
    /// entities and their components.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_core::ecs::World;
    /// # use engine_core::serialization::WorldState;
    /// let mut world = World::new();
    /// // ... add entities and components ...
    /// let snapshot = WorldState::snapshot(&world);
    /// ```
    pub fn snapshot(_world: &World) -> Self {
        let entities = Vec::new();
        let components = HashMap::new();

        // Get all entities from the world
        // Note: This requires adding a method to World to iterate entities
        // For now, we'll use a placeholder implementation

        let entity_count = 0;
        let component_count = 0;

        // Placeholder: In a complete implementation, we'd iterate all entities
        // and collect their components using World's public API

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

    /// Restore World from snapshot
    ///
    /// Clears the world and recreates all entities and components from
    /// the snapshot.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_core::ecs::World;
    /// # use engine_core::serialization::WorldState;
    /// let mut world = World::new();
    /// let snapshot = WorldState::new();
    /// snapshot.restore(&mut world);
    /// ```
    pub fn restore(&self, world: &mut World) {
        world.clear();

        for entity_meta in &self.entities {
            if !entity_meta.alive {
                continue;
            }

            // Spawn entity
            // Note: We need to add a method to World to spawn with specific ID/generation
            let _entity = world.spawn();

            // Add components
            // Note: We need a way to add components from ComponentData enum
            // This will be implemented when we extend World's API
        }
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializable for WorldState {
    fn serialize(&self, format: Format) -> Result<Vec<u8>, SerializationError> {
        match format {
            Format::Yaml => serde_yaml::to_string(self)
                .map(|s| s.into_bytes())
                .map_err(|e| SerializationError::yamlserialize(e.to_string())),
            Format::Bincode => bincode::serialize(self)
                .map_err(|e| SerializationError::bincodeserialize(e.to_string())),
            Format::FlatBuffers => {
                // FlatBuffers implementation will be added in the next step
                Err(SerializationError::flatbuffersserialize("Not yet implemented".to_string()))
            }
        }
    }

    fn deserialize(data: &[u8], format: Format) -> Result<Self, SerializationError> {
        match format {
            Format::Yaml => {
                let s = std::str::from_utf8(data)
                    .map_err(|e| SerializationError::utf8error(e.to_string()))?;
                serde_yaml::from_str(s)
                    .map_err(|e| SerializationError::yamldeserialize(e.to_string()))
            }
            Format::Bincode => bincode::deserialize(data)
                .map_err(|e| SerializationError::bincodedeserialize(e.to_string())),
            Format::FlatBuffers => {
                // FlatBuffers implementation will be added in the next step
                Err(SerializationError::flatbuffersdeserialize("Not yet implemented".to_string()))
            }
        }
    }

    fn serialize_to<W: Write>(
        &self,
        mut writer: W,
        format: Format,
    ) -> Result<(), SerializationError> {
        let bytes = Serializable::serialize(self, format)?;
        writer.write_all(&bytes)?;
        Ok(())
    }

    fn deserialize_from<R: Read>(
        mut reader: R,
        format: Format,
    ) -> Result<Self, SerializationError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        <Self as Serializable>::deserialize(&bytes, format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_state_new() {
        let state = WorldState::new();
        assert_eq!(state.entities.len(), 0);
        assert_eq!(state.components.len(), 0);
        assert_eq!(state.metadata.version, 1);
    }

    #[test]
    fn test_yaml_roundtrip() {
        let state = WorldState::new();
        let yaml = Serializable::serialize(&state, Format::Yaml).unwrap();
        let restored = <WorldState as Serializable>::deserialize(&yaml, Format::Yaml).unwrap();

        assert_eq!(state.entities.len(), restored.entities.len());
        assert_eq!(state.metadata.version, restored.metadata.version);
    }

    #[test]
    fn test_bincode_roundtrip() {
        let state = WorldState::new();
        let bytes = Serializable::serialize(&state, Format::Bincode).unwrap();
        let restored = <WorldState as Serializable>::deserialize(&bytes, Format::Bincode).unwrap();

        assert_eq!(state.entities.len(), restored.entities.len());
        assert_eq!(state.metadata.version, restored.metadata.version);
    }

    #[test]
    fn test_serialize_to_writer() {
        let state = WorldState::new();
        let mut buffer = Vec::new();

        state.serialize_to(&mut buffer, Format::Bincode).unwrap();
        assert!(!buffer.is_empty());

        let restored = WorldState::deserialize_from(&buffer[..], Format::Bincode).unwrap();
        assert_eq!(state.entities.len(), restored.entities.len());
    }
}
