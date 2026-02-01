//! Serialization for WorldState and ECS data
//!
//! Supports multiple formats:
//! - **YAML**: Human-readable, editable by AI agents
//! - **Bincode**: Fast local serialization
//! - **FlatBuffers**: Zero-copy network serialization

pub mod component_data;
pub mod delta;
pub mod error;
pub mod format;
pub mod world_state;

// Re-export commonly used types
pub use component_data::ComponentData;
pub use delta::WorldStateDelta;
pub use error::SerializationError;
pub use format::{Format, Serializable};
pub use world_state::{EntityMetadata, WorldMetadata, WorldState};
