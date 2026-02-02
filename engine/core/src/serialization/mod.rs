//! Serialization for WorldState and ECS data
//!
//! Supports multiple formats:
//! - **YAML**: Human-readable, editable by AI agents
//! - **Bincode**: Fast local serialization
//! - **FlatBuffers**: Zero-copy network serialization

pub mod component_data;
pub mod compression;
pub mod delta;
pub mod delta_optimized;
pub mod error;
pub mod flatbuffers_impl;
pub mod format;
pub mod optimized;
pub mod validation;
pub mod versioning;
pub mod world_state;

// Re-export commonly used types
pub use component_data::ComponentData;
pub use compression::{CompressedData, CompressionAlgorithm};
pub use delta::WorldStateDelta;
pub use delta_optimized::{ComponentChange, DeltaStats, OptimizedDelta, RunLengthEncoded};
pub use error::SerializationError;
pub use flatbuffers_impl::{FlatBuffersSerializer, ZeroCopyDeserializer};
pub use format::{Format, Serializable};
pub use optimized::BatchSerializer;
pub use validation::{
    ChecksumAlgorithm, RecoveryOptions, RecoveryResult, RecoveryStats, RecoveryStrategy,
    ValidatedWorldState, ValidationResult, WorldStateValidator,
};
pub use versioning::{
    global_registry, initialize_global_registry, MigrationRegistry, SchemaVersion,
    VersionedWorldState, CURRENT_SCHEMA_VERSION, MAX_SUPPORTED_VERSION, MIN_SUPPORTED_VERSION,
};
pub use world_state::{EntityMetadata, WorldMetadata, WorldState};
