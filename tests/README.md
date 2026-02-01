# Test Infrastructure

This directory contains shared test utilities and workspace-level integration tests.

## Directory Structure

```
tests/
├── common/
│   └── mod.rs              # Common test utilities (mock components, helpers, assertions)
├── integration/
│   └── mod.rs              # Integration test helpers (multi-frame tests, performance, etc.)
├── example_integration_test.rs  # Example showing how to use test utilities
└── README.md               # This file
```

## Common Test Utilities (`tests/common/`)

Provides shared utilities for all tests:

### Mock Components

Ready-to-use mock components for testing:
- `MockPosition` - 3D position component
- `MockVelocity` - 3D velocity component
- `MockHealth` - Health component with damage/heal
- `MockName` - String name component
- `MockPlayer` - Marker component (zero-sized)
- `MockEnemy` - Marker component (zero-sized)

### Test Builders

- `TestEntityBuilder` - Fluent API for creating test entities
- `TestIdGenerator` - Thread-safe ID generation

### Custom Assertions

- `assert_approx_eq!` - Compare floats with epsilon
- `assert_position_eq!` - Compare positions with epsilon
- `assert_velocity_eq!` - Compare velocities with epsilon

### Test Data Generators

- `random_position(range)` - Generate random positions
- `random_velocity(max_speed)` - Generate random velocities
- `create_test_entities(count)` - Create batch of test entities

### Performance Helpers

- `TestTimer` - Simple timer for measuring test performance
- `stress_test` - Run test multiple times
- `parallel_stress_test` - Run test in parallel threads

## Integration Test Utilities (`tests/integration/`)

Provides infrastructure for integration tests:

### Test Environment

- `init_test_environment()` - Initialize logging/tracing
- `IntegrationTestConfig` - Configuration for integration tests
- `TempTestDir` - RAII temporary directory management

### Multi-Frame Testing

- `MultiFrameTest` - Helper for testing across multiple frames

### Performance Measurement

- `PerformanceMeasurement` - Measure operation performance with statistics

### Thread Safety Testing

- `ThreadSafetyTest` - Helper for testing thread-safe operations

### Feature-Specific Helpers

- `MockNetworkClient` - Mock client for networking tests (feature: networking)
- `HeadlessRenderConfig` - Configuration for headless rendering (feature: renderer)
- `PhysicsTestSimulation` - Physics simulation helper (feature: physics)
- `SerializationTester` - Test serialization round-trips

## Usage Examples

### Using Mock Components

```rust
use tests::common::{MockPosition, MockVelocity, assert_position_eq};

#[test]
fn test_movement() {
    let mut pos = MockPosition::new(0.0, 0.0, 0.0);
    let vel = MockVelocity::new(1.0, 0.0, 0.0);

    pos.x += vel.x;

    assert_position_eq!(pos, MockPosition::new(1.0, 0.0, 0.0));
}
```

### Using Test Builders

```rust
use tests::common::TestEntityBuilder;

#[test]
fn test_entity() {
    let entity = TestEntityBuilder::new()
        .with_position(1.0, 2.0, 3.0)
        .with_velocity(0.5, 0.0, -0.5)
        .with_health(50, 100)
        .as_player();

    assert!(entity.is_player());
    assert!(entity.position().is_some());
}
```

### Multi-Frame Testing

```rust
use tests::integration::{init_test_environment, MultiFrameTest};

#[test]
fn test_simulation() {
    init_test_environment();

    let mut test = MultiFrameTest::new(60);
    let mut state = 0;

    test.run(|frame| {
        state += 1;
        // Run frame logic
    });

    assert_eq!(state, 60);
}
```

### Performance Measurement

```rust
use tests::integration::PerformanceMeasurement;

#[test]
fn test_performance() {
    let mut perf = PerformanceMeasurement::new("operation");

    for _ in 0..100 {
        perf.measure(|| {
            // Operation to measure
        });
    }

    perf.print_stats();
}
```

## Running Tests

```bash
# Run all integration tests
cargo test --tests

# Run specific integration test
cargo test --test example_integration_test

# Run with output
cargo test --tests -- --nocapture

# Run ignored tests (stress tests)
cargo test --tests -- --ignored
```

## Adding New Test Utilities

When adding new test utilities:

1. **Mock Components**: Add to `tests/common/mod.rs` in the "Mock Components" section
2. **Test Helpers**: Add to appropriate section in `tests/common/mod.rs`
3. **Integration Helpers**: Add to `tests/integration/mod.rs`
4. **Feature-Specific**: Use `#[cfg(feature = "...")]` attributes

Always include:
- Clear documentation
- Unit tests for the utility itself
- Examples in this README

## See Also

- [Testing Guide](../docs/testing-guide.md) - Comprehensive testing documentation
- [Testing Strategy](../docs/testing-strategy.md) - Overall testing strategy
