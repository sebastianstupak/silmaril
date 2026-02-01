# Testing Guide

> **Comprehensive guide to writing, organizing, and running tests for the Agent Game Engine**
>
> Last Updated: 2026-02-01

---

## Table of Contents

1. [Overview](#overview)
2. [Test Organization](#test-organization)
3. [Writing Unit Tests](#writing-unit-tests)
4. [Writing Integration Tests](#writing-integration-tests)
5. [Writing Benchmarks](#writing-benchmarks)
6. [Test Utilities](#test-utilities)
7. [Running Tests](#running-tests)
8. [Code Coverage](#code-coverage)
9. [Property-Based Testing](#property-based-testing)
10. [E2E Testing](#e2e-testing)
11. [Best Practices](#best-practices)
12. [Troubleshooting](#troubleshooting)

---

## Overview

The Agent Game Engine uses a comprehensive testing strategy that includes:

- **Unit Tests**: Fast, isolated tests for individual functions and methods
- **Integration Tests**: Tests for multiple components working together
- **Doc Tests**: Executable examples in documentation
- **Benchmarks**: Performance measurements using Criterion
- **Property Tests**: Randomized testing using proptest
- **E2E Tests**: Full system tests with real components

### Test Pyramid

```
         /\
        /  \  E2E Tests (Slow, High Value)
       /____\
      /      \  Integration Tests (Medium)
     /________\
    /          \  Unit Tests (Fast, Many)
   /__Property__\
```

### Coverage Goals

| Component | Target | Critical |
|-----------|--------|----------|
| Core ECS | > 90% | > 80% |
| Renderer | > 80% | > 70% |
| Networking | > 85% | > 75% |
| Physics | > 80% | > 70% |
| Overall | > 80% | > 70% |

---

## Test Organization

### Directory Structure

```
agent-game-engine/
├── engine/
│   ├── core/
│   │   ├── src/
│   │   │   ├── lib.rs                 # Unit tests inline
│   │   │   ├── ecs/
│   │   │   │   ├── mod.rs             # Unit tests inline
│   │   │   │   ├── world.rs           # Unit tests inline
│   │   │   │   └── entity.rs          # Unit tests inline
│   │   │   └── ...
│   │   ├── tests/                     # Integration tests
│   │   │   ├── ecs_integration.rs
│   │   │   ├── serialization.rs
│   │   │   └── ...
│   │   ├── benches/                   # Benchmarks
│   │   │   ├── ecs_benchmarks.rs
│   │   │   └── ...
│   │   └── examples/                  # Doc tests
│   │       └── ...
│   └── renderer/
│       └── ...
├── tests/                             # Workspace-level tests
│   ├── common/
│   │   └── mod.rs                     # Shared test utilities
│   └── integration/
│       └── mod.rs                     # Integration test helpers
└── docs/
    └── testing-guide.md               # This document
```

### Test File Naming

- **Unit tests**: Inline with `#[cfg(test)] mod tests { ... }`
- **Integration tests**: `tests/test_name.rs` or `tests/category/test_name.rs`
- **Benchmarks**: `benches/benchmark_name.rs`
- **Examples**: `examples/example_name.rs` (also serves as doc tests)

---

## Writing Unit Tests

### Basic Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let mut world = World::new();
        let entity = world.spawn();
        assert!(world.is_alive(entity));
    }

    #[test]
    fn test_component_operations() {
        let mut world = World::new();
        let entity = world.spawn();

        // Add component
        world.add(entity, Position { x: 1.0, y: 2.0, z: 3.0 });
        assert!(world.has::<Position>(entity));

        // Get component
        let pos = world.get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 1.0);

        // Remove component
        world.remove::<Position>(entity);
        assert!(!world.has::<Position>(entity));
    }
}
```

### Testing Error Cases

```rust
#[test]
fn test_invalid_entity_access() {
    let world = World::new();
    let invalid_entity = Entity::from_raw(999);

    // Should return None for invalid entity
    assert!(world.get::<Position>(invalid_entity).is_none());
}

#[test]
#[should_panic(expected = "entity not found")]
fn test_panic_on_invalid_operation() {
    let mut world = World::new();
    let entity = world.spawn();
    world.despawn(entity);

    // This should panic
    world.add(entity, Position::default());
}
```

### Testing Async Code

```rust
#[tokio::test]
async fn test_async_operation() {
    let client = NetworkClient::new().await.unwrap();

    client.connect("localhost:8080").await.unwrap();
    assert!(client.is_connected());

    client.disconnect().await.unwrap();
    assert!(!client.is_connected());
}
```

### Using Test Utilities

```rust
use crate::tests::common::{
    MockPosition, MockVelocity, TestEntityBuilder,
    assert_approx_eq, assert_position_eq,
};

#[test]
fn test_with_utilities() {
    let mut world = World::new();

    // Use test builders
    let entity = TestEntityBuilder::new()
        .with_position(1.0, 2.0, 3.0)
        .with_velocity(0.5, 0.0, -0.5)
        .as_player();

    // Use custom assertions
    let pos = MockPosition::new(1.0, 2.0, 3.0);
    let expected = MockPosition::new(1.0, 2.0, 3.0001);
    assert_position_eq!(pos, expected, 0.001);
}
```

---

## Writing Integration Tests

### Basic Integration Test

```rust
// tests/ecs_integration.rs
use engine_core::{World, Entity, Component};

#[test]
fn test_ecs_query_system() {
    let mut world = World::new();

    // Spawn entities
    for i in 0..100 {
        let entity = world.spawn();
        world.add(entity, Position {
            x: i as f32,
            y: i as f32,
            z: 0.0
        });

        if i % 2 == 0 {
            world.add(entity, Velocity {
                x: 1.0,
                y: 0.0,
                z: 0.0
            });
        }
    }

    // Query and count
    let mut count = 0;
    for (_entity, (_pos, _vel)) in world.query::<(&Position, &Velocity)>() {
        count += 1;
    }

    assert_eq!(count, 50); // Only entities with both components
}
```

### Multi-Crate Integration Test

```rust
// tests/renderer_networking_integration.rs
use engine_core::World;
use engine_renderer::Renderer;
use engine_networking::NetworkClient;

#[tokio::test]
async fn test_networked_rendering() {
    let mut world = World::new();
    let renderer = Renderer::new_headless().unwrap();
    let client = NetworkClient::new().await.unwrap();

    // Spawn networked entity
    let entity = world.spawn();
    world.add(entity, Position::default());
    world.add(entity, Renderable::default());
    world.add(entity, NetworkSync::default());

    // Sync with server
    client.sync_entity(&world, entity).await.unwrap();

    // Render
    renderer.render(&world).unwrap();

    assert!(client.is_synced(entity));
}
```

### Using Integration Test Helpers

```rust
use tests::integration::{
    init_test_environment,
    MultiFrameTest,
    PerformanceMeasurement,
};

#[test]
fn test_multi_frame_simulation() {
    init_test_environment();

    let mut world = World::new();
    let mut test = MultiFrameTest::new(60); // 60 frames

    test.run(|frame| {
        // Update physics
        physics_system(&mut world);

        // Every 10 frames, spawn enemy
        if frame % 10 == 0 {
            spawn_enemy(&mut world);
        }
    });

    // Verify final state
    assert_eq!(count_enemies(&world), 6);
}
```

---

## Writing Benchmarks

### Basic Benchmark

```rust
// benches/ecs_benchmarks.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use engine_core::{World, Position, Velocity};

fn bench_entity_spawn(c: &mut Criterion) {
    c.bench_function("spawn 10k entities", |b| {
        b.iter(|| {
            let mut world = World::new();
            for _ in 0..10_000 {
                black_box(world.spawn());
            }
        });
    });
}

fn bench_query_iteration(c: &mut Criterion) {
    let mut world = World::new();
    for _ in 0..10_000 {
        let entity = world.spawn();
        world.add(entity, Position::default());
        world.add(entity, Velocity::default());
    }

    c.bench_function("query 10k entities", |b| {
        b.iter(|| {
            for (_entity, (pos, vel)) in world.query::<(&Position, &Velocity)>() {
                black_box(pos);
                black_box(vel);
            }
        });
    });
}

criterion_group!(benches, bench_entity_spawn, bench_query_iteration);
criterion_main!(benches);
```

### Parameterized Benchmarks

```rust
fn bench_entity_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("entity_spawn");

    for count in [100, 1_000, 10_000, 100_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            count,
            |b, &count| {
                b.iter(|| {
                    let mut world = World::new();
                    for _ in 0..count {
                        black_box(world.spawn());
                    }
                });
            },
        );
    }

    group.finish();
}
```

### Throughput Benchmarks

```rust
fn bench_serialization_throughput(c: &mut Criterion) {
    use criterion::Throughput;

    let data = generate_test_data(1024 * 1024); // 1MB

    let mut group = c.benchmark_group("serialization");
    group.throughput(Throughput::Bytes(data.len() as u64));

    group.bench_function("serialize", |b| {
        b.iter(|| {
            black_box(serialize(&data).unwrap());
        });
    });

    group.finish();
}
```

---

## Test Utilities

### Common Test Utilities (tests/common/mod.rs)

The `tests/common` module provides shared utilities:

#### Mock Components

```rust
use tests::common::{
    MockPosition,
    MockVelocity,
    MockHealth,
    MockName,
    MockPlayer,
    MockEnemy,
};

let pos = MockPosition::new(1.0, 2.0, 3.0);
let vel = MockVelocity::new(0.5, 0.0, -0.5);
let health = MockHealth::full(100);
```

#### Test Builders

```rust
use tests::common::TestEntityBuilder;

let entity = TestEntityBuilder::new()
    .with_position(1.0, 2.0, 3.0)
    .with_velocity(0.5, 0.0, -0.5)
    .with_health(50, 100)
    .with_name("Player")
    .as_player();
```

#### Custom Assertions

```rust
use tests::common::{assert_approx_eq, assert_position_eq, assert_velocity_eq};

// Approximate equality for floats
assert_approx_eq!(actual, expected);
assert_approx_eq!(actual, expected, epsilon);

// Position/velocity equality
assert_position_eq!(pos1, pos2);
assert_velocity_eq!(vel1, vel2);
```

#### Test Data Generators

```rust
use tests::common::{random_position, random_velocity, create_test_entities};

let pos = random_position(100.0); // Random position in -100..100 range
let vel = random_velocity(10.0);  // Random velocity with max speed 10
let entities = create_test_entities(1000); // 1000 test entities
```

### Integration Test Utilities (tests/integration/mod.rs)

#### Test Environment Setup

```rust
use tests::integration::{init_test_environment, IntegrationTestConfig};

init_test_environment(); // Initialize logging/tracing

let config = IntegrationTestConfig::new()
    .with_logging()
    .with_timeout(10000);
```

#### Temporary Directories

```rust
use tests::integration::TempTestDir;

let temp_dir = TempTestDir::new("my_test");
let path = temp_dir.path();

// Use temp_dir...
// Automatically cleaned up on drop
```

#### Performance Measurement

```rust
use tests::integration::PerformanceMeasurement;

let mut perf = PerformanceMeasurement::new("operation");

for _ in 0..100 {
    perf.measure(|| {
        // Operation to measure
    });
}

perf.print_stats();
```

---

## Running Tests

### Basic Test Commands

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p engine-core

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests in release mode
cargo test --release
```

### Running Different Test Types

```bash
# Unit tests only (inline tests)
cargo test --lib

# Integration tests only
cargo test --tests

# Doc tests only
cargo test --doc

# Benchmarks
cargo bench

# Specific benchmark
cargo bench bench_name
```

### Running with Features

```bash
# Test with all features
cargo test --all-features

# Test with specific features
cargo test --features networking,renderer

# Test without default features
cargo test --no-default-features
```

### Platform-Specific Testing

```bash
# Windows
cargo test --target x86_64-pc-windows-msvc

# Linux
cargo test --target x86_64-unknown-linux-gnu

# macOS (Intel)
cargo test --target x86_64-apple-darwin

# macOS (ARM)
cargo test --target aarch64-apple-darwin
```

### Parallel and Serial Testing

```bash
# Run tests in parallel (default)
cargo test

# Run tests serially
cargo test -- --test-threads=1

# Limit parallel threads
cargo test -- --test-threads=4
```

### Test Filtering

```bash
# Run tests matching pattern
cargo test integration

# Run tests in specific module
cargo test ecs::tests::

# Exclude tests
cargo test -- --skip slow_test

# Run ignored tests
cargo test -- --ignored

# Run both normal and ignored tests
cargo test -- --include-ignored
```

---

## Code Coverage

### Using cargo-tarpaulin (Linux/macOS)

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html

# Coverage with all features
cargo tarpaulin --all-features --out Html

# Coverage for specific package
cargo tarpaulin -p engine-core --out Html

# Upload to codecov
cargo tarpaulin --all-features --out Xml
bash <(curl -s https://codecov.io/bash)
```

### Using cargo-llvm-cov (All Platforms)

```bash
# Install llvm-cov
cargo install cargo-llvm-cov

# Generate coverage
cargo llvm-cov --html

# Coverage with all features
cargo llvm-cov --all-features --html

# Generate lcov format
cargo llvm-cov --all-features --lcov --output-path lcov.info

# Open coverage report
cargo llvm-cov --all-features --open
```

### Coverage in CI

```yaml
# .github/workflows/coverage.yml
name: Coverage

on: [push, pull_request]

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: llvm-tools-preview
      - uses: taiki-e/install-action@cargo-llvm-cov
      - run: cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v3
        with:
          files: lcov.info
```

---

## Property-Based Testing

### Basic Property Test

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_entity_id_uniqueness(count in 1usize..1000) {
        let mut world = World::new();
        let entities: Vec<Entity> = (0..count)
            .map(|_| world.spawn())
            .collect();

        // All IDs must be unique
        let mut ids: Vec<_> = entities.iter().map(|e| e.id()).collect();
        ids.sort();
        ids.dedup();

        prop_assert_eq!(ids.len(), entities.len());
    }
}
```

### Complex Property Tests

```rust
proptest! {
    #[test]
    fn test_transform_serialization_roundtrip(
        pos_x in -1000.0f32..1000.0,
        pos_y in -1000.0f32..1000.0,
        pos_z in -1000.0f32..1000.0,
    ) {
        let transform = Transform {
            position: Vec3::new(pos_x, pos_y, pos_z),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };

        let bytes = transform.serialize().unwrap();
        let decoded = Transform::deserialize(&bytes).unwrap();

        prop_assert!((transform.position - decoded.position).length() < 0.001);
    }
}
```

### Custom Strategies

```rust
fn valid_position() -> impl Strategy<Value = Position> {
    (-1000.0f32..1000.0, -1000.0f32..1000.0, -1000.0f32..1000.0)
        .prop_map(|(x, y, z)| Position { x, y, z })
}

proptest! {
    #[test]
    fn test_with_custom_strategy(pos in valid_position()) {
        prop_assert!(pos.x.abs() <= 1000.0);
        prop_assert!(pos.y.abs() <= 1000.0);
        prop_assert!(pos.z.abs() <= 1000.0);
    }
}
```

---

## E2E Testing

### Docker Compose Setup

```yaml
# tests/e2e/docker-compose.test.yml
version: '3.8'

services:
  server:
    build:
      context: ../../
      dockerfile: engine/binaries/server/Dockerfile
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 5s
      timeout: 3s
      retries: 5

  client:
    build:
      context: ../../
      dockerfile: engine/binaries/client/Dockerfile
    environment:
      - SERVER_URL=ws://server:8080
      - RUST_LOG=info
    depends_on:
      server:
        condition: service_healthy
```

### E2E Test Implementation

```rust
// tests/e2e/multiplayer_test.rs
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_client_server_connection() {
    // Start server
    let server = TestServer::start("0.0.0.0:8080").await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = TestClient::connect("ws://localhost:8080")
        .await
        .unwrap();

    // Verify connection
    assert!(client.is_connected());

    // Test state sync
    let entity = server.spawn_entity_with_position(1.0, 2.0, 3.0);
    sleep(Duration::from_millis(100)).await;

    let synced_entities = client.get_visible_entities();
    assert_eq!(synced_entities.len(), 1);

    // Cleanup
    client.disconnect().await.unwrap();
    server.stop().await.unwrap();
}
```

### Running E2E Tests

```bash
# Run E2E tests with Docker Compose
docker-compose -f tests/e2e/docker-compose.test.yml up --abort-on-container-exit

# Run E2E tests locally
cargo test --test e2e_* -- --nocapture

# Clean up
docker-compose -f tests/e2e/docker-compose.test.yml down -v
```

---

## Best Practices

### Test Organization

1. **Keep unit tests close to code**: Use `#[cfg(test)] mod tests { ... }` in the same file
2. **Use descriptive test names**: `test_entity_spawn_increments_counter` not `test1`
3. **One assertion per test**: Makes failures easier to diagnose
4. **Use setup functions**: Extract common setup code into helper functions

### Test Quality

1. **Test edge cases**: Empty inputs, maximum values, invalid data
2. **Test error conditions**: Not just the happy path
3. **Use property tests**: For math and serialization code
4. **Keep tests fast**: Unit tests should be < 1ms, integration tests < 100ms
5. **Avoid test interdependence**: Each test should be independently runnable

### Performance Testing

1. **Use benchmarks**: Don't rely on timing in tests
2. **Test with realistic data**: Use representative dataset sizes
3. **Test degradation**: Ensure performance doesn't regress
4. **Profile benchmarks**: Use `cargo bench` with `--profile-time`

### Async Testing

1. **Use `#[tokio::test]`**: For async tests
2. **Set timeouts**: Prevent hanging tests
3. **Test cancellation**: Ensure cleanup on abort
4. **Avoid sleep**: Use proper synchronization primitives

### Mock and Stub Usage

1. **Mock external dependencies**: Network, filesystem, time
2. **Use dependency injection**: Makes testing easier
3. **Keep mocks simple**: Don't test the mock
4. **Prefer real implementations**: For integration tests

---

## Troubleshooting

### Common Issues

#### Tests Fail on CI but Pass Locally

**Possible causes:**
- Platform-specific behavior
- Race conditions (timing issues)
- Missing environment variables
- Different dependency versions

**Solutions:**
```bash
# Run tests in CI mode locally
CI=true cargo test

# Run with specific platform target
cargo test --target x86_64-unknown-linux-gnu

# Check for race conditions
cargo test -- --test-threads=1
```

#### Flaky Tests

**Possible causes:**
- Race conditions
- Non-deterministic behavior
- Timing dependencies

**Solutions:**
```rust
// Use proper synchronization
use std::sync::Barrier;
let barrier = Arc::new(Barrier::new(2));

// Avoid sleep, use channels
let (tx, rx) = channel();
tx.send(data).unwrap();
let result = rx.recv_timeout(Duration::from_secs(1)).unwrap();

// Use deterministic randomness
use rand::SeedableRng;
let mut rng = rand::rngs::StdRng::seed_from_u64(42);
```

#### Out of Memory

**Possible causes:**
- Large test datasets
- Memory leaks
- Too many parallel tests

**Solutions:**
```bash
# Limit parallel tests
cargo test -- --test-threads=2

# Run tests sequentially
cargo test -- --test-threads=1

# Increase memory limit (Docker)
docker run --memory=4g ...
```

#### Slow Tests

**Solutions:**
```bash
# Identify slow tests
cargo test -- --nocapture --test-threads=1 | grep -E "test.*ok"

# Run specific fast tests
cargo test --lib

# Skip slow tests
cargo test -- --skip slow_integration
```

---

## Additional Resources

- [Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Proptest Book](https://proptest-rs.github.io/proptest/)
- [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov)
- [Testing Strategy Document](./testing-strategy.md)

---

**Last Updated:** 2026-02-01
