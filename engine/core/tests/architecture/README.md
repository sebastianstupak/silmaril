# Architecture Tests

This directory contains comprehensive runtime tests that validate architectural invariants for Phase 1.4 platform abstractions.

## Test Files

### 1. platform_traits.rs (17 tests)
Tests that platform trait implementations are correct and consistent:
- **Trait Implementation**: All platform backends properly implement their respective traits
- **Send + Sync**: All backends are thread-safe
- **Factory Functions**: Platform factory functions work correctly on all platforms
- **Time Precision**: Time backend provides microsecond precision and monotonic guarantees
- **Unicode Support**: Filesystem backend handles Unicode paths and content
- **Binary Data**: Filesystem backend correctly handles binary data
- **Thread Safety**: Backends can be safely shared across threads
- **Error Handling**: Platform errors are properly propagated

### 2. module_boundaries.rs (15 tests)
Tests that module boundaries are properly enforced:
- **ECS Independence**: ECS can be used without platform dependencies
- **Serialization Independence**: Serialization works without platform code
- **Error Module Independence**: Error types don't require platform code
- **Component Independence**: Components are platform-agnostic
- **Query System**: Queries work without platform dependencies
- **Platform Isolation**: Platform code is isolated to platform/ directory
- **No cfg in Business Logic**: Business logic modules don't contain platform-specific code
- **Trait Boundaries**: Platform abstractions use trait objects correctly
- **API Cleanliness**: Public API doesn't expose platform internals
- **Dependency Direction**: Module dependencies are one-directional

### 3. error_handling.rs (23 tests)
Tests that error handling infrastructure is consistent and correct:
- **EngineError Trait**: All error types implement EngineError
- **Error Codes**: Error codes are unique and in correct ranges (1000-1999)
- **Subsystem Mapping**: Error codes correctly map to subsystems
- **Severity Levels**: Severity levels are properly ordered
- **Display Formatting**: Errors display all relevant information
- **Send + Sync**: All errors are thread-safe
- **Result Usage**: Errors work correctly in Result types
- **Error Conversions**: From implementations work correctly (io::Error, FromUtf8Error)
- **Logging Integration**: Error logging doesn't panic
- **Error Downcast**: Trait object downcasting works

## Running Tests

Run all architecture tests:
```bash
cargo test --tests
```

Run specific test suite:
```bash
cargo test --test platform_traits_test
cargo test --test module_boundaries_test
cargo test --test error_handling_test
```

Run a specific test:
```bash
cargo test test_time_precision_is_acceptable
```

## Test Coverage

Total: **55 architecture tests**
- Platform Traits: 17 tests
- Module Boundaries: 15 tests
- Error Handling: 23 tests

## Integration with CI

These tests are automatically run as part of the CI pipeline alongside:
- Unit tests (in src/)
- Static analysis (cargo-deny, build.rs)
- Benchmarks (benches/)

## Design Principles

1. **Runtime Validation**: These tests validate architecture at runtime, complementing compile-time checks
2. **No Mocking**: Tests use real implementations, not mocks
3. **Cross-Platform**: All tests pass on Windows, Linux, and macOS
4. **Fast**: All tests complete in <1 second
5. **Self-Documenting**: Each test includes clear comments explaining what it validates

## Future Additions

Planned architecture tests:
- Window abstraction tests (when implemented)
- Input abstraction tests (when implemented)
- Platform factory pattern tests
- Cross-compilation tests
