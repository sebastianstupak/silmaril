# Testing Quick Reference

> **Quick commands and patterns for testing**

---

## Quick Commands

### Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --tests

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Parallel control
cargo test -- --test-threads=4

# Ignored tests
cargo test -- --ignored
```

### Benchmarks

```bash
# All benchmarks
cargo bench

# Specific package
cargo bench -p engine-core

# Specific benchmark
cargo bench bench_name

# Save baseline
cargo bench -- --save-baseline my-baseline

# Compare to baseline
cargo bench -- --baseline my-baseline
```

### Coverage

```bash
# HTML report
cargo llvm-cov --html --open

# All features
cargo llvm-cov --all-features --workspace --html --open

# LCOV format
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# Specific package
cargo llvm-cov -p engine-core --html --open
```

### Using Aliases (from .cargo/config.toml)

```bash
# Coverage
cargo cov              # HTML coverage with browser
cargo cov-all          # All features coverage
cargo cov-lcov         # Generate lcov.info

# Testing
cargo test-all         # All tests with all features
cargo test-unit        # Unit tests only
cargo test-integration # Integration tests only
cargo test-doc         # Doc tests only
cargo test-quick       # Quick unit tests

# Benchmarks
cargo bench-all        # All benchmarks
cargo bench-core       # Core benchmarks
cargo bench-renderer   # Renderer benchmarks
cargo bench-networking # Networking benchmarks
```

---

## Test Patterns

### Basic Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        let value = create_value();
        assert_eq!(value, expected);
    }
}
```

### Async Test

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### Property Test

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_property(value in 0..100) {
        prop_assert!(check_property(value));
    }
}
```

### Benchmark

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_operation(c: &mut Criterion) {
    c.bench_function("operation", |b| {
        b.iter(|| black_box(operation()));
    });
}

criterion_group!(benches, bench_operation);
criterion_main!(benches);
```

---

## Common Assertions

```rust
// Equality
assert_eq!(actual, expected);
assert_ne!(actual, unexpected);

// Boolean
assert!(condition);
assert!(!condition);

// Approximate equality (custom)
assert_approx_eq!(actual, expected);
assert_approx_eq!(actual, expected, epsilon);

// Position/Velocity (custom)
assert_position_eq!(pos1, pos2);
assert_velocity_eq!(vel1, vel2);

// Results
assert!(result.is_ok());
assert!(result.is_err());
assert_eq!(result.unwrap(), value);

// Options
assert!(option.is_some());
assert!(option.is_none());
assert_eq!(option.unwrap(), value);
```

---

## Test Utilities Import

```rust
// Common test utilities
use tests::common::{
    // Mock components
    MockPosition, MockVelocity, MockHealth, MockName,
    MockPlayer, MockEnemy,

    // Builders and generators
    TestEntityBuilder, TestIdGenerator,
    create_test_entities, random_position, random_velocity,

    // Assertions
    assert_approx_eq, assert_position_eq, assert_velocity_eq,

    // Performance
    TestTimer, stress_test, parallel_stress_test,
};

// Integration helpers
use tests::integration::{
    init_test_environment, IntegrationTestConfig,
    MultiFrameTest, PerformanceMeasurement,
    TempTestDir, ThreadSafetyTest,
};
```

---

## Mock Component Usage

```rust
// Create components
let pos = MockPosition::new(1.0, 2.0, 3.0);
let vel = MockVelocity::new(0.5, 0.0, -0.5);
let mut health = MockHealth::full(100);
let name = MockName::new("Entity");

// Use components
let distance = pos.distance_to(&other_pos);
let speed = vel.magnitude();
health.damage(30);
health.heal(20);
```

---

## Test Builder Pattern

```rust
let entity = TestEntityBuilder::new()
    .with_position(1.0, 2.0, 3.0)
    .with_velocity(0.5, 0.0, -0.5)
    .with_health(50, 100)
    .with_name("Player")
    .as_player();

assert!(entity.is_player());
```

---

## Multi-Frame Testing

```rust
let mut test = MultiFrameTest::new(60);

test.run(|frame| {
    // Update simulation
    update_physics(&mut world);

    if test.is_first_frame() {
        // First frame setup
    }

    if frame % 10 == 0 {
        // Every 10 frames
    }

    if test.is_last_frame() {
        // Final frame
    }
});
```

---

## Performance Measurement

```rust
let mut perf = PerformanceMeasurement::new("operation");

for _ in 0..100 {
    perf.measure(|| {
        // Operation to measure
    });
}

perf.print_stats();
// Average: 1.23ms
// Min:     0.98ms
// Max:     2.45ms
```

---

## Stress Testing

```rust
// Sequential stress test
stress_test(10000, |iteration| {
    // Test operation
});

// Parallel stress test
parallel_stress_test(4, 1000, |id| {
    // Thread-safe operation
});
```

---

## Temporary Directories

```rust
let temp_dir = TempTestDir::new("my_test");
let path = temp_dir.path();

// Use directory...
write_test_file(path.join("test.txt"))?;

// Automatically cleaned up on drop
```

---

## See Also

- [Testing Guide](./testing-guide.md) - Comprehensive testing documentation
- [Testing Strategy](./testing-strategy.md) - Overall testing strategy
- [Test Infrastructure README](../tests/README.md) - Test utilities documentation
