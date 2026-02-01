//! Architecture tests for runtime validation.
//!
//! This module contains tests that validate architectural invariants at runtime:
//! - Platform traits are properly implemented
//! - Module boundaries are respected
//! - Error handling is consistent and correct
//!
//! These tests complement the static analysis performed by cargo-deny and build.rs.

pub mod error_handling;
pub mod module_boundaries;
pub mod platform_traits;
