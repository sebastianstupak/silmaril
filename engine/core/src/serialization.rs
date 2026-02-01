//! Serialization utilities
//!
//! This module will contain serialization utilities once Phase 1 is complete.
//! For now, it provides basic placeholder types.

/// Placeholder for serialization errors
#[derive(Debug)]
pub struct SerializationError;

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Serialization error")
    }
}

impl std::error::Error for SerializationError {}
