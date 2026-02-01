# Phase 2.2: Network Protocol (FlatBuffers)

**Status:** ⚪ Not Started
**Estimated Time:** 4-5 days
**Priority:** Critical (foundation for networking)

---

## 🎯 **Objective**

Design and implement network protocol using FlatBuffers for zero-copy serialization. Define messages for client-server communication, state sync, and commands.

**Protocol Features:**
- Full state snapshots
- Delta updates
- Entity spawning/despawning
- Component updates
- Player commands (input)
- Reliable (TCP) + Unreliable (UDP) messages

---

## 📋 **Detailed Tasks**

### **1. FlatBuffers Schema** (Day 1-2)

**File:** `engine/networking/schemas/protocol.fbs`

```flatbuffers
namespace AgentGameEngine.Network;

// ============================================================================
// Base Types
// ============================================================================

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

table Entity {
  id: uint32;
  generation: uint32;
}

// ============================================================================
// Components
// ============================================================================

table Transform {
  position: Vec3;
  rotation: Quat;
  scale: Vec3;
}

table Velocity {
  velocity: Vec3;
}

table Health {
  current: float;
  max: float;
}

// Component data union
union ComponentData {
  Transform,
  Velocity,
  Health,
  // Add more component types
}

table Component {
  entity: Entity;
  data: ComponentData;
}

// ============================================================================
// Messages (Client → Server)
// ============================================================================

/// Player input
table PlayerInput {
  sequence: uint32;
  timestamp: uint64;
  movement: Vec3;      // WASD input
  look_delta: Vec3;    // Mouse movement
  buttons: uint32;     // Bitfield for buttons
}

/// Join request
table JoinRequest {
  player_name: string;
  version: uint32;
}

/// Client ready (after loading)
table ClientReady {
  client_id: uint64;
}

/// Client command union
union ClientMessage {
  PlayerInput,
  JoinRequest,
  ClientReady,
}

table ClientPacket {
  message: ClientMessage;
}

root_type ClientPacket;

// ============================================================================
// Messages (Server → Client)
// ============================================================================

/// Join response
table JoinResponse {
  success: bool;
  client_id: uint64;
  error_message: string;  // If success = false
}

/// Full world state snapshot
table WorldSnapshot {
  tick: uint64;
  entities: [EntityState];
}

table EntityState {
  entity: Entity;
  components: [Component];
}

/// Delta update (changes since last snapshot)
table WorldDelta {
  base_tick: uint64;
  target_tick: uint64;

  // Added entities
  added_entities: [EntityState];

  // Removed entities
  removed_entities: [Entity];

  // Updated components
  updated_components: [Component];

  // Removed components (entity + component type ID)
  removed_components: [ComponentRemoval];
}

table ComponentRemoval {
  entity: Entity;
  component_type_id: uint32;
}

/// Entity spawned
table EntitySpawned {
  entity: Entity;
  components: [Component];
}

/// Entity despawned
table EntityDespawned {
  entity: Entity;
}

/// Component added
table ComponentAdded {
  entity: Entity;
  component: Component;
}

/// Component removed
table ComponentRemoved {
  entity: Entity;
  component_type_id: uint32;
}

/// Component updated
table ComponentUpdated {
  entity: Entity;
  component: Component;
}

/// Server message union
union ServerMessage {
  JoinResponse,
  WorldSnapshot,
  WorldDelta,
  EntitySpawned,
  EntityDespawned,
  ComponentAdded,
  ComponentRemoved,
  ComponentUpdated,
}

table ServerPacket {
  message: ServerMessage;
}

root_type ServerPacket;

// ============================================================================
// Connection Messages
// ============================================================================

table Ping {
  timestamp: uint64;
}

table Pong {
  client_timestamp: uint64;
  server_timestamp: uint64;
}

table Disconnect {
  reason: string;
}
```

---

### **2. Protocol Code Generation** (Day 2)

**File:** `engine/networking/build.rs`

```rust
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=schemas/protocol.fbs");

    // Compile FlatBuffers schema
    let output = Command::new("flatc")
        .args(&[
            "--rust",
            "--gen-mutable",
            "--gen-object-api",
            "-o",
            "src/generated/",
            "schemas/protocol.fbs",
        ])
        .output()
        .expect("Failed to compile FlatBuffers schema");

    if !output.status.success() {
        panic!(
            "FlatBuffers compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("FlatBuffers schema compiled successfully");
}
```

---

### **3. Protocol Wrapper** (Day 2-3)

**File:** `engine/networking/src/protocol/mod.rs`

```rust
pub mod generated {
    #![allow(unused_imports)]
    #![allow(dead_code)]
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/protocol_generated.rs"));
}

use generated::*;
use flatbuffers::FlatBufferBuilder;

/// Protocol encoder/decoder
pub struct Protocol;

impl Protocol {
    /// Encode client packet
    pub fn encode_client_packet(message: ClientMessage) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();

        let packet = ClientPacket::create(
            &mut builder,
            &ClientPacketArgs {
                message_type: message.message_type(),
                message: Some(message.to_union_value(&mut builder)),
            },
        );

        builder.finish(packet, None);
        builder.finished_data().to_vec()
    }

    /// Decode client packet
    pub fn decode_client_packet(data: &[u8]) -> Result<ClientPacket, ProtocolError> {
        flatbuffers::root::<ClientPacket>(data).map_err(|e| ProtocolError::DecodeFailed {
            details: e.to_string(),
        })
    }

    /// Encode server packet
    pub fn encode_server_packet(message: ServerMessage) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();

        let packet = ServerPacket::create(
            &mut builder,
            &ServerPacketArgs {
                message_type: message.message_type(),
                message: Some(message.to_union_value(&mut builder)),
            },
        );

        builder.finish(packet, None);
        builder.finished_data().to_vec()
    }

    /// Decode server packet
    pub fn decode_server_packet(data: &[u8]) -> Result<ServerPacket, ProtocolError> {
        flatbuffers::root::<ServerPacket>(data).map_err(|e| ProtocolError::DecodeFailed {
            details: e.to_string(),
        })
    }
}

/// Protocol errors
define_error! {
    pub enum ProtocolError {
        EncodeFailed { details: String } = ErrorCode::ProtocolEncodeFailed, ErrorSeverity::Error,
        DecodeFailed { details: String } = ErrorCode::ProtocolDecodeFailed, ErrorSeverity::Error,
        InvalidMessage { details: String } = ErrorCode::ProtocolInvalidMessage, ErrorSeverity::Error,
    }
}
```

---

### **4. Message Builders** (Day 3-4)

**File:** `engine/networking/src/protocol/builders.rs`

```rust
use super::generated::*;
use flatbuffers::FlatBufferBuilder;

/// Helper to build PlayerInput message
pub struct PlayerInputBuilder;

impl PlayerInputBuilder {
    pub fn build(
        sequence: u32,
        timestamp: u64,
        movement: glam::Vec3,
        look_delta: glam::Vec3,
        buttons: u32,
    ) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();

        let input = PlayerInput::create(
            &mut builder,
            &PlayerInputArgs {
                sequence,
                timestamp,
                movement: Some(&Vec3::new(movement.x, movement.y, movement.z)),
                look_delta: Some(&Vec3::new(look_delta.x, look_delta.y, look_delta.z)),
                buttons,
            },
        );

        let message = ClientMessage::PlayerInput(input);
        let packet = ClientPacket::create(
            &mut builder,
            &ClientPacketArgs {
                message_type: ClientMessage::ENUM_VALUES[0], // PlayerInput
                message: Some(message.to_union_value(&mut builder)),
            },
        );

        builder.finish(packet, None);
        builder.finished_data().to_vec()
    }
}

/// Helper to build WorldSnapshot message
pub struct WorldSnapshotBuilder;

impl WorldSnapshotBuilder {
    pub fn build(tick: u64, world: &World) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();

        // Build entity states
        let mut entity_states = Vec::new();

        for entity in world.entities() {
            let components = world.get_all_components(entity);

            // Build components
            let component_offsets: Vec<_> = components
                .iter()
                .map(|c| Self::build_component(&mut builder, entity, c))
                .collect();

            let components_vec = builder.create_vector(&component_offsets);

            let entity_state = EntityState::create(
                &mut builder,
                &EntityStateArgs {
                    entity: Some(&Entity::new(entity.id(), entity.generation())),
                    components: Some(components_vec),
                },
            );

            entity_states.push(entity_state);
        }

        let entities_vec = builder.create_vector(&entity_states);

        let snapshot = WorldSnapshot::create(
            &mut builder,
            &WorldSnapshotArgs {
                tick,
                entities: Some(entities_vec),
            },
        );

        let message = ServerMessage::WorldSnapshot(snapshot);
        let packet = ServerPacket::create(
            &mut builder,
            &ServerPacketArgs {
                message_type: ServerMessage::ENUM_VALUES[1], // WorldSnapshot
                message: Some(message.to_union_value(&mut builder)),
            },
        );

        builder.finish(packet, None);
        builder.finished_data().to_vec()
    }

    fn build_component(
        builder: &mut FlatBufferBuilder,
        entity: Entity,
        component_data: &ComponentData,
    ) -> flatbuffers::WIPOffset<Component> {
        match component_data {
            ComponentData::Transform(t) => {
                let transform = Transform::create(
                    builder,
                    &TransformArgs {
                        position: Some(&Vec3::new(t.position.x, t.position.y, t.position.z)),
                        rotation: Some(&Quat::new(
                            t.rotation.x,
                            t.rotation.y,
                            t.rotation.z,
                            t.rotation.w,
                        )),
                        scale: Some(&Vec3::new(t.scale.x, t.scale.y, t.scale.z)),
                    },
                );

                Component::create(
                    builder,
                    &ComponentArgs {
                        entity: Some(&Entity::new(entity.id(), entity.generation())),
                        data_type: ComponentData::ENUM_VALUES[0], // Transform
                        data: Some(ComponentData::Transform(transform).to_union_value(builder)),
                    },
                )
            }
            // Handle other component types...
            _ => todo!("Implement other component types"),
        }
    }
}

/// Helper to build WorldDelta message
pub struct WorldDeltaBuilder;

impl WorldDeltaBuilder {
    pub fn build(base_tick: u64, target_tick: u64, delta: &WorldStateDelta) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();

        // Build added entities
        let added_entities: Vec<_> = delta
            .added_entities
            .iter()
            .map(|e| Self::build_entity_state(&mut builder, e))
            .collect();
        let added_vec = builder.create_vector(&added_entities);

        // Build removed entities
        let removed_entities: Vec<_> = delta
            .removed_entities
            .iter()
            .map(|e| Entity::new(e.id(), e.generation()))
            .collect();
        let removed_vec = builder.create_vector(&removed_entities);

        // Build updated components
        let updated_components: Vec<_> = delta
            .modified_components
            .iter()
            .flat_map(|(entity, components)| {
                components
                    .iter()
                    .map(|c| WorldSnapshotBuilder::build_component(&mut builder, *entity, c))
            })
            .collect();
        let updated_vec = builder.create_vector(&updated_components);

        // Build removed components
        let removed_components: Vec<_> = delta
            .removed_components
            .iter()
            .flat_map(|(entity, type_names)| {
                type_names.iter().map(|type_name| {
                    let type_id = Self::component_type_name_to_id(type_name);
                    ComponentRemoval::create(
                        &mut builder,
                        &ComponentRemovalArgs {
                            entity: Some(&Entity::new(entity.id(), entity.generation())),
                            component_type_id: type_id,
                        },
                    )
                })
            })
            .collect();
        let removed_comp_vec = builder.create_vector(&removed_components);

        let delta_msg = WorldDelta::create(
            &mut builder,
            &WorldDeltaArgs {
                base_tick,
                target_tick,
                added_entities: Some(added_vec),
                removed_entities: Some(removed_vec),
                updated_components: Some(updated_vec),
                removed_components: Some(removed_comp_vec),
            },
        );

        let message = ServerMessage::WorldDelta(delta_msg);
        let packet = ServerPacket::create(
            &mut builder,
            &ServerPacketArgs {
                message_type: ServerMessage::ENUM_VALUES[2], // WorldDelta
                message: Some(message.to_union_value(&mut builder)),
            },
        );

        builder.finish(packet, None);
        builder.finished_data().to_vec()
    }

    fn build_entity_state(
        builder: &mut FlatBufferBuilder,
        entity_meta: &EntityMetadata,
    ) -> flatbuffers::WIPOffset<EntityState> {
        // Implementation similar to WorldSnapshotBuilder
        todo!()
    }

    fn component_type_name_to_id(type_name: &str) -> u32 {
        // Map component type name to ID
        match type_name {
            "Transform" => 0,
            "Velocity" => 1,
            "Health" => 2,
            _ => 0,
        }
    }
}
```

---

### **5. Protocol Tests** (Day 4-5)

**File:** `engine/networking/src/protocol/tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_player_input() {
        let data = PlayerInputBuilder::build(
            1,
            1000,
            glam::Vec3::new(1.0, 0.0, 0.0),
            glam::Vec3::new(0.0, 0.1, 0.0),
            0b0001,
        );

        let packet = Protocol::decode_client_packet(&data).unwrap();

        match packet.message_type() {
            ClientMessage::PlayerInput => {
                let input = packet.message_as_player_input().unwrap();
                assert_eq!(input.sequence(), 1);
                assert_eq!(input.timestamp(), 1000);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_world_snapshot_roundtrip() {
        let mut world = World::new();
        world.register::<Transform>();

        let entity = world.spawn();
        world.add(entity, Transform::default());

        let data = WorldSnapshotBuilder::build(100, &world);
        let packet = Protocol::decode_server_packet(&data).unwrap();

        match packet.message_type() {
            ServerMessage::WorldSnapshot => {
                let snapshot = packet.message_as_world_snapshot().unwrap();
                assert_eq!(snapshot.tick(), 100);
                assert_eq!(snapshot.entities().unwrap().len(), 1);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_delta_compression() {
        let mut world1 = World::new();
        let mut world2 = World::new();

        // Setup similar worlds
        // ... (test delta is smaller than full state)

        let state1 = WorldState::snapshot(&world1);
        let state2 = WorldState::snapshot(&world2);

        let delta = WorldStateDelta::compute(&state1, &state2);

        let delta_data = WorldDeltaBuilder::build(100, 101, &delta);
        let full_data = WorldSnapshotBuilder::build(101, &world2);

        // Delta should be smaller
        assert!(delta_data.len() < full_data.len());
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] FlatBuffers schema defined
- [ ] Schema compiles to Rust code
- [ ] Client message encoding/decoding works
- [ ] Server message encoding/decoding works
- [ ] Full state snapshots work
- [ ] Delta updates work
- [ ] Delta is smaller than full state
- [ ] All message types tested
- [ ] Zero-copy deserialization verified
- [ ] Protocol version handling

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| Encode PlayerInput | < 0.1ms | < 0.5ms |
| Decode PlayerInput | < 0.05ms | < 0.2ms |
| Encode WorldSnapshot (1000 entities) | < 5ms | < 10ms |
| Decode WorldSnapshot (1000 entities) | < 3ms | < 8ms |
| Encode WorldDelta (100 changes) | < 1ms | < 3ms |
| Decode WorldDelta (100 changes) | < 0.5ms | < 2ms |

**Size Targets:**
- PlayerInput: ~50 bytes
- WorldSnapshot (1000 entities): ~50-100 KB
- WorldDelta (100 changes): ~5-10 KB (80-90% reduction)

---

## 📊 **Protocol Overhead**

FlatBuffers advantages:
- Zero-copy deserialization
- Forward/backward compatibility
- Efficient for large messages
- Type-safe

Compared to alternatives:
- vs JSON: 60-80% size reduction, 10x faster
- vs Bincode: Similar speed, better compatibility
- vs Protobuf: Faster (zero-copy), similar size

---

**Dependencies:** [phase2-proc-macros.md](phase2-proc-macros.md)
**Next:** [phase2-tcp-connection.md](phase2-tcp-connection.md)
