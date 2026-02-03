//! End-to-End integration tests
//!
//! Cross-crate E2E tests for networking stack.
//! MANDATORY: These tests use multiple engine crates, so they MUST be in engine/shared/tests/

#[path = "e2e/mod.rs"]
mod e2e;

// Re-export for test discovery
pub use e2e::*;
