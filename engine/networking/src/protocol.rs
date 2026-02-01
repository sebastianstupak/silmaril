//! Network protocol implementation
//!
//! Defines message types and serialization for client-server communication.
//! Uses both FlatBuffers (zero-copy) and Bincode (fast) serialization.

use engine_core::ecs::Entity;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Protocol version for version negotiation
pub const PROTOCOL_VERSION: u32 = 1;

/// Maximum message size (16MB)
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Client -> Server messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Player movement input
    PlayerMove {
        /// X position
        x: f32,
        /// Y position
        y: f32,
        /// Z position
        z: f32,
        /// Timestamp (ms)
        timestamp: u64,
    },

    /// Player action (jump, attack, interact, etc.)
    PlayerAction {
        /// Action type ID
        action_id: u8,
        /// Target entity (if any)
        target: Option<Entity>,
        /// Timestamp (ms)
        timestamp: u64,
    },

    /// Chat message
    ChatMessage {
        /// Message content
        message: String,
        /// Channel (0=global, 1=team, etc.)
        channel: u8,
    },

    /// Request to spawn entity
    SpawnRequest {
        /// Prefab ID
        prefab_id: u32,
        /// Spawn position X
        x: f32,
        /// Spawn position Y
        y: f32,
        /// Spawn position Z
        z: f32,
    },

    /// Ping request (for latency measurement)
    Ping {
        /// Client timestamp
        client_time: u64,
    },

    /// Protocol version handshake
    Handshake {
        /// Protocol version
        version: u32,
        /// Client name
        client_name: String,
    },
}

/// Server -> Client messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Full state update (initial sync or after disconnect)
    StateUpdate {
        /// Timestamp (ms)
        timestamp: u64,
        /// Entity states (serialized)
        entities: Vec<EntityState>,
    },

    /// Entity spawned
    EntitySpawned {
        /// Entity ID
        entity: Entity,
        /// Prefab ID
        prefab_id: u32,
        /// Position X
        x: f32,
        /// Position Y
        y: f32,
        /// Position Z
        z: f32,
    },

    /// Entity despawned
    EntityDespawned {
        /// Entity ID
        entity: Entity,
    },

    /// Entity transform update (position/rotation)
    EntityTransform {
        /// Entity ID
        entity: Entity,
        /// Position X
        x: f32,
        /// Position Y
        y: f32,
        /// Position Z
        z: f32,
        /// Rotation quaternion X
        qx: f32,
        /// Rotation quaternion Y
        qy: f32,
        /// Rotation quaternion Z
        qz: f32,
        /// Rotation quaternion W
        qw: f32,
    },

    /// Chat message broadcast
    ChatBroadcast {
        /// Sender name
        sender: String,
        /// Message content
        message: String,
        /// Channel
        channel: u8,
    },

    /// Pong response
    Pong {
        /// Original client timestamp
        client_time: u64,
        /// Server timestamp
        server_time: u64,
    },

    /// Handshake response
    HandshakeResponse {
        /// Accepted protocol version
        version: u32,
        /// Server name
        server_name: String,
        /// Player's assigned entity ID
        player_entity: Entity,
    },
}

/// Entity state for full sync
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityState {
    /// Entity ID
    pub entity: Entity,
    /// Position X
    pub x: f32,
    /// Position Y
    pub y: f32,
    /// Position Z
    pub z: f32,
    /// Rotation quaternion X
    pub qx: f32,
    /// Rotation quaternion Y
    pub qy: f32,
    /// Rotation quaternion Z
    pub qz: f32,
    /// Rotation quaternion W
    pub qw: f32,
    /// Health (if applicable)
    pub health: Option<f32>,
    /// Max health (if applicable)
    pub max_health: Option<f32>,
}

/// Message framing with length prefix
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    /// Bincode (fast, compact)
    Bincode,
    /// FlatBuffers (zero-copy, currently uses bincode placeholder)
    FlatBuffers,
}

/// Framed message with length prefix (4 bytes) + payload
pub struct FramedMessage {
    /// Message length (not including the 4-byte length prefix)
    pub length: u32,
    /// Serialized payload
    pub payload: Vec<u8>,
}

impl FramedMessage {
    /// Create a new framed message from bytes
    pub fn new(payload: Vec<u8>) -> Result<Self, ProtocolError> {
        if payload.len() > MAX_MESSAGE_SIZE {
            return Err(ProtocolError::MessageTooLarge {
                size: payload.len(),
                max_size: MAX_MESSAGE_SIZE,
            });
        }

        Ok(Self {
            length: payload.len() as u32,
            payload,
        })
    }

    /// Write framed message to writer (length prefix + payload)
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize, ProtocolError> {
        // Write length prefix (big-endian)
        writer.write_all(&self.length.to_be_bytes())
            .map_err(|e| ProtocolError::IoError(e.to_string()))?;

        // Write payload
        writer.write_all(&self.payload)
            .map_err(|e| ProtocolError::IoError(e.to_string()))?;

        Ok(4 + self.payload.len())
    }

    /// Read framed message from reader
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, ProtocolError> {
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes)
            .map_err(|e| ProtocolError::IoError(e.to_string()))?;

        let length = u32::from_be_bytes(len_bytes);

        if length as usize > MAX_MESSAGE_SIZE {
            return Err(ProtocolError::MessageTooLarge {
                size: length as usize,
                max_size: MAX_MESSAGE_SIZE,
            });
        }

        // Read payload
        let mut payload = vec![0u8; length as usize];
        reader.read_exact(&mut payload)
            .map_err(|e| ProtocolError::IoError(e.to_string()))?;

        Ok(Self { length, payload })
    }

    /// Get total size (including 4-byte length prefix)
    pub fn total_size(&self) -> usize {
        4 + self.payload.len()
    }
}

/// Serialize a ClientMessage
pub fn serialize_client_message(
    msg: &ClientMessage,
    format: SerializationFormat,
) -> Result<FramedMessage, ProtocolError> {
    let payload = match format {
        SerializationFormat::Bincode => {
            bincode::serialize(msg)
                .map_err(|e| ProtocolError::SerializationError(e.to_string()))?
        }
        SerializationFormat::FlatBuffers => {
            // TODO: Implement FlatBuffers serialization
            // For now, use bincode as placeholder
            bincode::serialize(msg)
                .map_err(|e| ProtocolError::SerializationError(e.to_string()))?
        }
    };

    FramedMessage::new(payload)
}

/// Deserialize a ClientMessage
pub fn deserialize_client_message(
    framed: &FramedMessage,
    format: SerializationFormat,
) -> Result<ClientMessage, ProtocolError> {
    match format {
        SerializationFormat::Bincode => {
            bincode::deserialize(&framed.payload)
                .map_err(|e| ProtocolError::DeserializationError(e.to_string()))
        }
        SerializationFormat::FlatBuffers => {
            // TODO: Implement FlatBuffers deserialization
            // For now, use bincode as placeholder
            bincode::deserialize(&framed.payload)
                .map_err(|e| ProtocolError::DeserializationError(e.to_string()))
        }
    }
}

/// Serialize a ServerMessage
pub fn serialize_server_message(
    msg: &ServerMessage,
    format: SerializationFormat,
) -> Result<FramedMessage, ProtocolError> {
    let payload = match format {
        SerializationFormat::Bincode => {
            bincode::serialize(msg)
                .map_err(|e| ProtocolError::SerializationError(e.to_string()))?
        }
        SerializationFormat::FlatBuffers => {
            // TODO: Implement FlatBuffers serialization
            bincode::serialize(msg)
                .map_err(|e| ProtocolError::SerializationError(e.to_string()))?
        }
    };

    FramedMessage::new(payload)
}

/// Deserialize a ServerMessage
pub fn deserialize_server_message(
    framed: &FramedMessage,
    format: SerializationFormat,
) -> Result<ServerMessage, ProtocolError> {
    match format {
        SerializationFormat::Bincode => {
            bincode::deserialize(&framed.payload)
                .map_err(|e| ProtocolError::DeserializationError(e.to_string()))
        }
        SerializationFormat::FlatBuffers => {
            // TODO: Implement FlatBuffers deserialization
            bincode::deserialize(&framed.payload)
                .map_err(|e| ProtocolError::DeserializationError(e.to_string()))
        }
    }
}

/// Protocol errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// IO error
    IoError(String),
    /// Serialization error
    SerializationError(String),
    /// Deserialization error
    DeserializationError(String),
    /// Message too large
    MessageTooLarge {
        /// Actual size
        size: usize,
        /// Maximum allowed size
        max_size: usize,
    },
    /// Unsupported protocol version
    UnsupportedVersion {
        /// Client version
        client_version: u32,
        /// Server version
        server_version: u32,
    },
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "IO error: {}", e),
            Self::SerializationError(e) => write!(f, "Serialization error: {}", e),
            Self::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
            Self::MessageTooLarge { size, max_size } => {
                write!(f, "Message too large: {} bytes (max {})", size, max_size)
            }
            Self::UnsupportedVersion { client_version, server_version } => {
                write!(
                    f,
                    "Unsupported protocol version: client={}, server={}",
                    client_version, server_version
                )
            }
        }
    }
}

impl std::error::Error for ProtocolError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_serialization_bincode() {
        let msg = ClientMessage::PlayerMove {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            timestamp: 12345,
        };

        let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();
        let deserialized = deserialize_client_message(&framed, SerializationFormat::Bincode).unwrap();

        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_server_message_serialization_bincode() {
        let msg = ServerMessage::EntitySpawned {
            entity: Entity::new(42, 0),
            prefab_id: 100,
            x: 10.0,
            y: 20.0,
            z: 30.0,
        };

        let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();
        let deserialized = deserialize_server_message(&framed, SerializationFormat::Bincode).unwrap();

        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_framed_message_roundtrip() {
        let payload = vec![1, 2, 3, 4, 5];
        let framed = FramedMessage::new(payload.clone()).unwrap();

        let mut buffer = Vec::new();
        framed.write_to(&mut buffer).unwrap();

        let mut cursor = std::io::Cursor::new(buffer);
        let decoded = FramedMessage::read_from(&mut cursor).unwrap();

        assert_eq!(framed.payload, decoded.payload);
        assert_eq!(framed.length, decoded.length);
    }

    #[test]
    fn test_message_too_large() {
        let large_payload = vec![0u8; MAX_MESSAGE_SIZE + 1];
        let result = FramedMessage::new(large_payload);

        assert!(matches!(
            result,
            Err(ProtocolError::MessageTooLarge { .. })
        ));
    }

    #[test]
    fn test_chat_message_overhead() {
        let msg = ClientMessage::ChatMessage {
            message: "Hello".to_string(),
            channel: 0,
        };

        let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();

        // Overhead should be minimal (enum discriminant + string length + framing)
        // With "Hello" (5 bytes), total should be < 50 bytes
        assert!(framed.total_size() < 50, "Message overhead too large: {} bytes", framed.total_size());
    }

    #[test]
    fn test_player_move_overhead() {
        let msg = ClientMessage::PlayerMove {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            timestamp: 12345,
        };

        let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();

        // PlayerMove should be very compact (3 f32s + 1 u64 + overhead)
        // Expected: ~30 bytes total
        assert!(framed.total_size() < 50, "PlayerMove overhead too large: {} bytes", framed.total_size());
    }

    #[test]
    fn test_entity_transform_overhead() {
        let msg = ServerMessage::EntityTransform {
            entity: Entity::new(42, 0),
            x: 1.0,
            y: 2.0,
            z: 3.0,
            qx: 0.0,
            qy: 0.0,
            qz: 0.0,
            qw: 1.0,
        };

        let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();

        // Transform update should be compact (entity + 7 f32s + overhead)
        // Expected: ~40 bytes total
        assert!(framed.total_size() < 60, "EntityTransform overhead too large: {} bytes", framed.total_size());
    }

    #[test]
    fn test_handshake_roundtrip() {
        let msg = ClientMessage::Handshake {
            version: PROTOCOL_VERSION,
            client_name: "TestClient".to_string(),
        };

        let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();
        let deserialized = deserialize_client_message(&framed, SerializationFormat::Bincode).unwrap();

        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_ping_pong() {
        let client_time = 123456789;

        let ping = ClientMessage::Ping { client_time };
        let ping_framed = serialize_client_message(&ping, SerializationFormat::Bincode).unwrap();

        let pong = ServerMessage::Pong {
            client_time,
            server_time: 123456800,
        };
        let pong_framed = serialize_server_message(&pong, SerializationFormat::Bincode).unwrap();

        // Ping/pong should be very small for low latency measurement
        assert!(ping_framed.total_size() < 30);
        assert!(pong_framed.total_size() < 40);
    }

    #[test]
    fn test_state_update_batch() {
        let entities = vec![
            EntityState {
                entity: Entity::new(1, 0),
                x: 1.0, y: 2.0, z: 3.0,
                qx: 0.0, qy: 0.0, qz: 0.0, qw: 1.0,
                health: Some(100.0),
                max_health: Some(100.0),
            },
            EntityState {
                entity: Entity::new(2, 0),
                x: 4.0, y: 5.0, z: 6.0,
                qx: 0.0, qy: 0.0, qz: 0.0, qw: 1.0,
                health: None,
                max_health: None,
            },
        ];

        let msg = ServerMessage::StateUpdate {
            timestamp: 12345,
            entities,
        };

        let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();
        let deserialized = deserialize_server_message(&framed, SerializationFormat::Bincode).unwrap();

        assert_eq!(msg, deserialized);
    }
}
