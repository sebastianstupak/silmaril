//! Shared test infrastructure for cross-crate integration tests
//!
//! This crate contains:
//! - Integration tests that span multiple engine crates (ECS + Physics, etc.)
//! - Cross-crate benchmarks that measure integrated system performance
//!
//! # Test Organization
//!
//! The Silmaril uses a 3-tier test hierarchy:
//!
//! ## Tier 1: Unit Tests (in each crate)
//! - Test single functions/modules in isolation
//! - Located in `crate/src/` files or `crate/tests/` directory
//! - Use only dependencies from that crate
//! - Examples: `engine/physics/tests/raycast_tests.rs`
//!
//! ## Tier 2: Cross-Crate Integration Tests (THIS CRATE)
//! - Test interaction between multiple engine crates
//! - Located in `engine/shared/tests/`
//! - Use dependencies from multiple crates (e.g., `engine-core` + `engine-physics`)
//! - Examples: Physics + ECS integration, Renderer + ECS integration
//!
//! ## Tier 3: End-to-End System Tests (in examples/)
//! - Test complete game scenarios from user perspective
//! - Located in `examples/` directory
//! - Run actual game servers/clients
//! - Examples: Multiplayer matchmaking, full game loops
//!
//! # When to Add Tests Here
//!
//! Add tests to `engine/shared/tests/` when:
//! 1. Your test imports from 2+ engine crates (e.g., `engine-core` + `engine-physics`)
//! 2. You're testing the integration/interaction between systems
//! 3. Your test verifies that components from different crates work together
//!
//! # When to Add Benchmarks Here
//!
//! Add benchmarks to `engine/shared/benches/` when:
//! 1. You're benchmarking integrated system performance (e.g., ECS + Physics)
//! 2. Your benchmark uses components from multiple crates
//! 3. You're measuring end-to-end performance of cross-crate workflows
//!
//! # Examples
//!
//! ## Cross-Crate Test
//! ```rust,ignore
//! // engine/shared/tests/physics_ecs_integration.rs
//! use engine_core::ecs::World;
//! use engine_physics::PhysicsWorld;
//!
//! #[test]
//! fn test_physics_syncs_to_ecs() {
//!     let mut world = World::new();
//!     let mut physics = PhysicsWorld::new();
//!     // Test physics state synchronization to ECS
//! }
//! ```
//!
//! ## Cross-Crate Benchmark
//! ```rust,ignore
//! // engine/shared/benches/physics_ecs_bench.rs
//! use criterion::Criterion;
//! use engine_core::ecs::World;
//! use engine_physics::PhysicsWorld;
//!
//! fn bench_physics_ecs_sync(c: &mut Criterion) {
//!     // Benchmark integrated physics + ECS performance
//! }
//! ```

#![allow(dead_code)] // Shared test utilities may not be used by all tests

// Re-export commonly used test utilities
pub use engine_core;
pub use engine_math;
pub use engine_physics;

/// Stress testing utilities for performance and load testing
pub mod stress_testing;
