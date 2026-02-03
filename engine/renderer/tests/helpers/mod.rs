//! Test helper utilities
//!
//! Provides common test infrastructure for renderer tests.

pub mod visual_test;

// Re-export commonly used types
pub use visual_test::{create_gradient_frame, create_test_frame, VisualTest};
