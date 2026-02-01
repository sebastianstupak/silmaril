//! Pure mathematics library for game engine.
//!
//! Provides vector types, transforms, and SIMD-optimized operations.
//! This module is domain-agnostic and has no dependencies on other engine modules.
//!
//! # Performance
//!
//! This crate uses [glam](https://github.com/bitshifter/glam-rs) for Vec3 and Quat types,
//! which provides highly optimized SIMD implementations with 3-4x performance improvements
//! over scalar operations.
//!
//! For maximum performance, compile with `RUSTFLAGS="-C target-cpu=native"` to enable
//! all SIMD features available on your CPU (SSE, AVX, FMA, etc.).

#![warn(missing_docs)]

pub mod aligned;
pub mod quat;
pub mod transform;
pub mod vec3;

#[cfg(feature = "simd")]
pub mod simd;

// Re-exports
pub use quat::{Quat, QuatExt};
pub use transform::Transform;
pub use vec3::{Vec3, Vec3Ext};

#[cfg(feature = "simd")]
pub use simd::Vec3x4;
// Future: pub use simd::Vec3x8;

#[cfg(test)]
mod aligned_quick_test;
