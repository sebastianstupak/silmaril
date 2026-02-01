//! Engine LOD
//!
//! Provides level-of-detail system:
//! - Automatic mesh simplification
//! - LOD selection based on distance
//! - Network LOD integration
//! - Smooth transitions

#![warn(missing_docs)]

pub mod generator;
pub mod selector;
pub mod network;

// Re-export commonly used types
pub use generator::LodGenerator;
pub use selector::LodLevel;
