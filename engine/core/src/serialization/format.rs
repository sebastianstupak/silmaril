//! Serialization format trait and types

use super::SerializationError;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// Serialization format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// YAML - Human-readable, editable by AI agents
    Yaml,
    /// Bincode - Fast local serialization
    Bincode,
    /// FlatBuffers - Zero-copy network serialization
    FlatBuffers,
}

/// Trait for types that can be serialized in multiple formats
pub trait Serializable: Serialize + for<'de> Deserialize<'de> {
    /// Serialize to bytes using specified format
    fn serialize(&self, format: Format) -> Result<Vec<u8>, SerializationError>;

    /// Deserialize from bytes
    fn deserialize(data: &[u8], format: Format) -> Result<Self, SerializationError>
    where
        Self: Sized;

    /// Serialize to writer
    fn serialize_to<W: Write>(&self, writer: W, format: Format) -> Result<(), SerializationError>;

    /// Deserialize from reader
    fn deserialize_from<R: Read>(reader: R, format: Format) -> Result<Self, SerializationError>
    where
        Self: Sized;
}
