# Phase 1.3: WorldState Serialization

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** High (needed for networking, save/load)

---

## 🎯 **Objective**

Implement multi-format serialization for WorldState (entire ECS state) with support for YAML (debug/agents), Bincode (local IPC), and FlatBuffers (network). Enable full state snapshots and delta compression.

**Target Formats:**
- **YAML** - Human-readable, editable by AI agents
- **Bincode** - Fast local serialization
- **FlatBuffers** - Zero-copy network serialization

---

## 📋 **Detailed Tasks**

### **1. Serialization Trait** (Day 1)

**File:** `engine/core/src/serialization/mod.rs`

```rust
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Serialization format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Yaml,
    Bincode,
    FlatBuffers,
}

/// Errors during serialization
define_error! {
    pub enum SerializationError {
        YamlSerialize { details: String } = ErrorCode::YamlSerializeFailed, ErrorSeverity::Error,
        YamlDeserialize { details: String } = ErrorCode::YamlDeserializeFailed, ErrorSeverity::Error,
        BincodeSerialize { details: String } = ErrorCode::BincodeSerializeFailed, ErrorSeverity::Error,
        BincodeDeserialize { details: String } = ErrorCode::BincodeDeserializeFailed, ErrorSeverity::Error,
        FlatBuffersSerialize { details: String } = ErrorCode::FlatBuffersSerializeFailed, ErrorSeverity::Error,
        FlatBuffersDeserialize { details: String } = ErrorCode::FlatBuffersDeserializeFailed, ErrorSeverity::Error,
        IoError { source: std::io::Error } = ErrorCode::SerializationIoError, ErrorSeverity::Error,
    }
}

/// Trait for serializable types
pub trait Serializable: Serialize + for<'de> Deserialize<'de> {
    /// Serialize to bytes using specified format
    fn serialize(&self, format: Format) -> Result<Vec<u8>, SerializationError>;

    /// Deserialize from bytes
    fn deserialize(data: &[u8], format: Format) -> Result<Self, SerializationError>
    where
        Self: Sized;

    /// Serialize to writer
    fn serialize_to<W: Write>(
        &self,
        writer: W,
        format: Format,
    ) -> Result<(), SerializationError>;

    /// Deserialize from reader
    fn deserialize_from<R: Read>(
        reader: R,
        format: Format,
    ) -> Result<Self, SerializationError>
    where
        Self: Sized;
}
```

---

### **2. Component Data Enum** (Day 1-2)

**File:** `engine/core/src/ecs/component_data.rs`

All component types must be wrapped in an enum for type-erasure:

```rust
use serde::{Deserialize, Serialize};

/// Type-erased component data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentData {
    Transform(Transform),
    Velocity(Velocity),
    Health(Health),
    MeshRenderer(MeshRenderer),
    // Add more as needed
}

impl ComponentData {
    /// Get type ID
    pub fn type_id(&self) -> std::any::TypeId {
        match self {
            Self::Transform(_) => std::any::TypeId::of::<Transform>(),
            Self::Velocity(_) => std::any::TypeId::of::<Velocity>(),
            Self::Health(_) => std::any::TypeId::of::<Health>(),
            Self::MeshRenderer(_) => std::any::TypeId::of::<MeshRenderer>(),
        }
    }

    /// Get type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Transform(_) => "Transform",
            Self::Velocity(_) => "Velocity",
            Self::Health(_) => "Health",
            Self::MeshRenderer(_) => "MeshRenderer",
        }
    }
}
```

**Problem:** This requires manual updates when adding components.

**Solution:** Procedural macro (Phase 2) to auto-generate this enum:

```rust
// Future goal (Phase 2):
#[derive(Component)]
pub struct NewComponent { }

// Automatically adds to ComponentData enum
```

---

### **3. WorldState Snapshot** (Day 2)

**File:** `engine/core/src/ecs/world_state.rs`

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete snapshot of ECS world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    /// Entity metadata
    pub entities: Vec<EntityMetadata>,

    /// Components by entity
    pub components: HashMap<Entity, Vec<ComponentData>>,

    /// Metadata
    pub metadata: WorldMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetadata {
    pub entity: Entity,
    pub generation: u32,
    pub alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMetadata {
    pub version: u32,
    pub timestamp: u64,
    pub entity_count: usize,
    pub component_count: usize,
}

impl WorldState {
    /// Create snapshot from World
    pub fn snapshot(world: &World) -> Self {
        let mut entities = Vec::new();
        let mut components = HashMap::new();

        // Iterate all entities
        for entity in world.entities() {
            if !world.is_alive(entity) {
                continue;
            }

            entities.push(EntityMetadata {
                entity,
                generation: entity.generation(),
                alive: true,
            });

            // Collect all components for this entity
            let entity_components = world.get_all_components(entity);
            components.insert(entity, entity_components);
        }

        Self {
            entities,
            components,
            metadata: WorldMetadata {
                version: 1,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                entity_count: entities.len(),
                component_count: components.values().map(|v| v.len()).sum(),
            },
        }
    }

    /// Restore World from snapshot
    pub fn restore(&self, world: &mut World) {
        world.clear();

        for entity_meta in &self.entities {
            if !entity_meta.alive {
                continue;
            }

            // Spawn entity with same ID/generation
            let entity = world.spawn_with_id(entity_meta.entity);

            // Add components
            if let Some(entity_components) = self.components.get(&entity_meta.entity) {
                for component_data in entity_components {
                    world.add_component_data(entity, component_data.clone());
                }
            }
        }
    }
}

impl Serializable for WorldState {
    fn serialize(&self, format: Format) -> Result<Vec<u8>, SerializationError> {
        match format {
            Format::Yaml => {
                serde_yaml::to_string(self)
                    .map(|s| s.into_bytes())
                    .map_err(|e| SerializationError::YamlSerialize {
                        details: e.to_string(),
                    })
            }
            Format::Bincode => {
                bincode::serialize(self).map_err(|e| SerializationError::BincodeSerialize {
                    details: e.to_string(),
                })
            }
            Format::FlatBuffers => {
                // Implemented in next section
                todo!("FlatBuffers serialization")
            }
        }
    }

    fn deserialize(data: &[u8], format: Format) -> Result<Self, SerializationError> {
        match format {
            Format::Yaml => {
                let s = std::str::from_utf8(data)
                    .map_err(|e| SerializationError::YamlDeserialize {
                        details: e.to_string(),
                    })?;
                serde_yaml::from_str(s).map_err(|e| SerializationError::YamlDeserialize {
                    details: e.to_string(),
                })
            }
            Format::Bincode => {
                bincode::deserialize(data).map_err(|e| {
                    SerializationError::BincodeDeserialize {
                        details: e.to_string(),
                    }
                })
            }
            Format::FlatBuffers => {
                // Implemented in next section
                todo!("FlatBuffers deserialization")
            }
        }
    }

    fn serialize_to<W: Write>(
        &self,
        mut writer: W,
        format: Format,
    ) -> Result<(), SerializationError> {
        let bytes = self.serialize(format)?;
        writer
            .write_all(&bytes)
            .map_err(|e| SerializationError::IoError { source: e })
    }

    fn deserialize_from<R: Read>(
        mut reader: R,
        format: Format,
    ) -> Result<Self, SerializationError> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|e| SerializationError::IoError { source: e })?;
        Self::deserialize(&bytes, format)
    }
}
```

---

### **4. FlatBuffers Schema** (Day 2-3)

**File:** `engine/core/schemas/world_state.fbs`

```flatbuffers
namespace AgentGameEngine.Core;

// Entity
table Entity {
  id: uint32;
  generation: uint32;
}

// Component types
union ComponentDataUnion {
  Transform,
  Velocity,
  Health,
  MeshRenderer,
}

// Components
table Transform {
  position: Vec3;
  rotation: Quat;
  scale: Vec3;
}

table Velocity {
  x: float;
  y: float;
  z: float;
}

table Health {
  current: float;
  max: float;
}

table MeshRenderer {
  mesh_id: uint64;
  material_id: uint64;
}

// Math types
struct Vec3 {
  x: float;
  y: float;
  z: float;
}

struct Quat {
  x: float;
  y: float;
  z: float;
  w: float;
}

// Component wrapper
table Component {
  type: ComponentDataUnion;
  data: [ubyte];  // Serialized component data
}

// Entity with components
table EntityState {
  entity: Entity;
  components: [Component];
}

// World state
table WorldState {
  version: uint32;
  timestamp: uint64;
  entities: [EntityState];
}

root_type WorldState;
```

**Build Script:** `engine/core/build.rs`

```rust
fn main() {
    // Compile FlatBuffers schema
    flatc::run(flatc::Args {
        inputs: &["schemas/world_state.fbs"],
        out_dir: "src/serialization/generated/",
        lang: "rust",
        ..Default::default()
    })
    .expect("Failed to compile FlatBuffers schema");
}
```

**FlatBuffers Implementation:**

```rust
// engine/core/src/serialization/flatbuffers.rs

use crate::serialization::generated::world_state as fb;

impl WorldState {
    pub fn to_flatbuffers(&self) -> Vec<u8> {
        let mut builder = flatbuffers::FlatBufferBuilder::new();

        // Build entity states
        let mut entity_states = Vec::new();
        for (entity, components) in &self.components {
            let entity_fb = fb::Entity::create(
                &mut builder,
                &fb::EntityArgs {
                    id: entity.id(),
                    generation: entity.generation(),
                },
            );

            // Build components
            let components_fb: Vec<_> = components
                .iter()
                .map(|c| Self::component_to_fb(&mut builder, c))
                .collect();

            let components_vec = builder.create_vector(&components_fb);

            let entity_state = fb::EntityState::create(
                &mut builder,
                &fb::EntityStateArgs {
                    entity: Some(entity_fb),
                    components: Some(components_vec),
                },
            );

            entity_states.push(entity_state);
        }

        let entities_vec = builder.create_vector(&entity_states);

        // Build root
        let world_state = fb::WorldState::create(
            &mut builder,
            &fb::WorldStateArgs {
                version: self.metadata.version,
                timestamp: self.metadata.timestamp,
                entities: Some(entities_vec),
            },
        );

        builder.finish(world_state, None);

        builder.finished_data().to_vec()
    }

    pub fn from_flatbuffers(data: &[u8]) -> Result<Self, SerializationError> {
        let world_state = fb::root_as_world_state(data).map_err(|e| {
            SerializationError::FlatBuffersDeserialize {
                details: e.to_string(),
            }
        })?;

        let mut entities = Vec::new();
        let mut components = HashMap::new();

        if let Some(entity_states) = world_state.entities() {
            for entity_state in entity_states {
                let entity_fb = entity_state.entity().unwrap();
                let entity = Entity {
                    id: entity_fb.id(),
                    generation: entity_fb.generation(),
                };

                entities.push(EntityMetadata {
                    entity,
                    generation: entity.generation(),
                    alive: true,
                });

                // Parse components
                if let Some(comps) = entity_state.components() {
                    let entity_components: Vec<_> = comps
                        .iter()
                        .map(|c| Self::component_from_fb(c))
                        .collect();
                    components.insert(entity, entity_components);
                }
            }
        }

        Ok(Self {
            entities,
            components,
            metadata: WorldMetadata {
                version: world_state.version(),
                timestamp: world_state.timestamp(),
                entity_count: entities.len(),
                component_count: components.values().map(|v| v.len()).sum(),
            },
        })
    }
}
```

---

### **5. Delta Compression** (Day 3-4)

**File:** `engine/core/src/serialization/delta.rs`

```rust
use std::collections::HashSet;

/// Delta between two world states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateDelta {
    /// Base state version
    pub base_version: u32,

    /// Target state version
    pub target_version: u32,

    /// Added entities
    pub added_entities: Vec<EntityMetadata>,

    /// Removed entities
    pub removed_entities: Vec<Entity>,

    /// Modified components (entity, component data)
    pub modified_components: HashMap<Entity, Vec<ComponentData>>,

    /// Removed components (entity, type name)
    pub removed_components: HashMap<Entity, Vec<String>>,
}

impl WorldStateDelta {
    /// Compute delta from old to new state
    pub fn compute(old: &WorldState, new: &WorldState) -> Self {
        let old_entities: HashSet<_> = old.entities.iter().map(|e| e.entity).collect();
        let new_entities: HashSet<_> = new.entities.iter().map(|e| e.entity).collect();

        // Added entities
        let added_entities: Vec<_> = new
            .entities
            .iter()
            .filter(|e| !old_entities.contains(&e.entity))
            .cloned()
            .collect();

        // Removed entities
        let removed_entities: Vec<_> = old_entities
            .difference(&new_entities)
            .copied()
            .collect();

        // Modified/removed components
        let mut modified_components = HashMap::new();
        let mut removed_components = HashMap::new();

        for entity in new_entities.iter() {
            let old_comps = old.components.get(entity);
            let new_comps = new.components.get(entity);

            match (old_comps, new_comps) {
                (Some(old), Some(new)) => {
                    // Find modified components
                    let modified: Vec<_> = new
                        .iter()
                        .filter(|nc| {
                            !old.iter().any(|oc| {
                                oc.type_id() == nc.type_id()
                                    && bincode::serialize(oc).unwrap()
                                        == bincode::serialize(nc).unwrap()
                            })
                        })
                        .cloned()
                        .collect();

                    if !modified.is_empty() {
                        modified_components.insert(*entity, modified);
                    }

                    // Find removed components
                    let removed: Vec<_> = old
                        .iter()
                        .filter(|oc| !new.iter().any(|nc| nc.type_id() == oc.type_id()))
                        .map(|c| c.type_name().to_string())
                        .collect();

                    if !removed.is_empty() {
                        removed_components.insert(*entity, removed);
                    }
                }
                (None, Some(new)) => {
                    // All components are new
                    modified_components.insert(*entity, new.clone());
                }
                _ => {}
            }
        }

        Self {
            base_version: old.metadata.version,
            target_version: new.metadata.version,
            added_entities,
            removed_entities,
            modified_components,
            removed_components,
        }
    }

    /// Apply delta to a base state
    pub fn apply(&self, base: &mut WorldState) {
        // Remove entities
        for entity in &self.removed_entities {
            base.entities.retain(|e| e.entity != *entity);
            base.components.remove(entity);
        }

        // Add entities
        base.entities.extend(self.added_entities.clone());

        // Modify components
        for (entity, components) in &self.modified_components {
            let entry = base.components.entry(*entity).or_insert_with(Vec::new);

            for new_comp in components {
                // Remove old version of this component type
                entry.retain(|c| c.type_id() != new_comp.type_id());
                // Add new version
                entry.push(new_comp.clone());
            }
        }

        // Remove components
        for (entity, type_names) in &self.removed_components {
            if let Some(components) = base.components.get_mut(entity) {
                components.retain(|c| !type_names.contains(&c.type_name().to_string()));
            }
        }

        // Update metadata
        base.metadata.version = self.target_version;
        base.metadata.entity_count = base.entities.len();
        base.metadata.component_count = base.components.values().map(|v| v.len()).sum();
    }

    /// Check if delta is smaller than full state
    pub fn is_smaller_than(&self, full_state: &WorldState) -> bool {
        let delta_size = bincode::serialize(self).unwrap().len();
        let full_size = bincode::serialize(full_state).unwrap().len();
        delta_size < full_size
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] WorldState can snapshot entire ECS
- [ ] WorldState can restore from snapshot
- [ ] YAML serialization works (human-readable)
- [ ] Bincode serialization works (fast)
- [ ] FlatBuffers serialization works (zero-copy)
- [ ] Delta compression implemented
- [ ] Delta application works correctly
- [ ] All formats tested with round-trip (serialize → deserialize)
- [ ] ComponentData enum includes all components
- [ ] Performance targets met

---

## 🎯 **Performance Targets**

| Operation | Target (1000 entities) | Critical |
|-----------|----------------------|----------|
| Snapshot (YAML) | < 50ms | < 100ms |
| Snapshot (Bincode) | < 5ms | < 10ms |
| Snapshot (FlatBuffers) | < 3ms | < 8ms |
| Restore (Bincode) | < 10ms | < 20ms |
| Delta compute | < 5ms | < 10ms |
| Delta apply | < 3ms | < 8ms |

**Size Targets:**
- YAML: ~50-100 KB for 1000 entities
- Bincode: ~20-30 KB for 1000 entities
- FlatBuffers: ~15-25 KB for 1000 entities
- Delta: 60-80% reduction vs full state

---

## 🧪 **Tests**

```rust
#[test]
fn test_yaml_roundtrip() {
    let mut world = World::new();
    setup_test_world(&mut world);

    let state = WorldState::snapshot(&world);
    let yaml = state.serialize(Format::Yaml).unwrap();
    let restored = WorldState::deserialize(&yaml, Format::Yaml).unwrap();

    assert_eq!(state.entities.len(), restored.entities.len());
}

#[test]
fn test_bincode_roundtrip() {
    let mut world = World::new();
    setup_test_world(&mut world);

    let state = WorldState::snapshot(&world);
    let bytes = state.serialize(Format::Bincode).unwrap();
    let restored = WorldState::deserialize(&bytes, Format::Bincode).unwrap();

    assert_eq!(state.entities.len(), restored.entities.len());
}

#[test]
fn test_delta_compression() {
    let mut world1 = World::new();
    let mut world2 = World::new();

    // Create similar worlds with small differences
    for i in 0..1000 {
        let e = world1.spawn();
        world1.add(e, Transform::default());

        let e = world2.spawn();
        world2.add(e, Transform::default());
    }

    // Modify one entity in world2
    let e = world2.entities().next().unwrap();
    world2.get_mut::<Transform>(e).unwrap().position.x = 10.0;

    let state1 = WorldState::snapshot(&world1);
    let state2 = WorldState::snapshot(&world2);

    let delta = WorldStateDelta::compute(&state1, &state2);

    // Delta should be much smaller
    assert!(delta.is_smaller_than(&state2));

    // Apply delta
    let mut state1_copy = state1.clone();
    delta.apply(&mut state1_copy);

    // Should match state2
    assert_eq!(state1_copy.entities.len(), state2.entities.len());
}
```

---

**Dependencies:** [phase1-ecs-queries.md](phase1-ecs-queries.md)
**Next:** [phase1-platform.md](phase1-platform.md)
