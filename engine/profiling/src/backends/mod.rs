//! Profiling backend implementations.
//!
//! This module contains different profiling backends that can be selected via
//! feature flags. Each backend provides the same core functionality through
//! a common interface.

#[cfg(feature = "profiling-puffin")]
pub mod puffin_backend;

#[cfg(feature = "profiling-puffin")]
pub use puffin_backend::PuffinBackend;

#[cfg(feature = "profiling-tracy")]
pub mod tracy_backend;

#[cfg(feature = "profiling-tracy")]
pub use tracy_backend::TracyBackend;
