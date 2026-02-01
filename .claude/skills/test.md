---
name: test
description: Run comprehensive test suite for agent-game-engine
trigger: /test
---

# Comprehensive Test Runner

Runs the complete test suite for the agent-game-engine project with detailed reporting.

## Instructions

1. **Run All Tests**
   Execute tests in this order:

   ```bash
   # Run all workspace tests with all features
   cargo test --workspace --all-features

   # Run documentation tests
   cargo test --doc

   # Run integration tests separately (if they exist)
   cargo test --workspace --test '*'
   ```

2. **Platform-Specific Tests**
   If platform-specific code exists, run:
   ```bash
   # Check current platform
   rustc -vV | grep host

   # Run platform-specific tests
   cargo test --features platform-windows  # if on Windows
   cargo test --features platform-linux    # if on Linux
   cargo test --features platform-macos    # if on macOS
   ```

3. **Client/Server Feature Tests**
   Test feature-gated builds:
   ```bash
   # Client features
   cargo test --features client --no-default-features

   # Server features
   cargo test --features server --no-default-features

   # Both (if supported)
   cargo test --all-features
   ```

4. **Generate Test Summary**
   After all tests complete:
   - Count total tests passed/failed
   - Show which test suites passed
   - Highlight any failures with details
   - Report total execution time
   - Show code coverage if available

5. **Handle Test Failures**
   If any tests fail:
   - Show the full error output
   - Identify which crate/module failed
   - Show the specific test that failed
   - Suggest potential fixes based on error messages
   - Reference relevant documentation from docs/

6. **Optional: Run Benchmarks**
   If user requests benchmarks or performance tests:
   ```bash
   cargo bench --workspace
   ```

## Output Format

Provide a clear summary like:

```
Test Results Summary
====================

Unit Tests:           PASSED (245/245)
Documentation Tests:  PASSED (23/23)
Integration Tests:    PASSED (18/18)
Platform Tests:       PASSED (12/12)

Total: 298 tests passed
Time: 45.3 seconds

All tests passed!
```

Or if failures occur:

```
Test Results Summary
====================

Unit Tests:           FAILED (243/245)
  - Failed: engine/core/src/ecs.rs::tests::test_entity_spawn
  - Failed: engine/networking/src/tcp.rs::tests::test_connection

Documentation Tests:  PASSED (23/23)
Integration Tests:    PASSED (18/18)

Total: 296/298 tests passed (2 failures)
Time: 43.1 seconds

Failures:
---------
1. test_entity_spawn: assertion failed - expected entity to be alive
   Location: engine/core/src/ecs.rs:345

2. test_connection: connection timeout after 5s
   Location: engine/networking/src/tcp.rs:123
```

## Performance Tracking

If running benchmarks:
- Compare against previous baseline if available
- Show performance regressions/improvements
- Highlight metrics that exceed performance targets from docs/performance-targets.md

## Notes

- Always run with `--all-features` for comprehensive coverage
- Never skip doc tests - they're part of the documentation contract
- If tests hang, timeout after 5 minutes and report
- Capture and preserve test output for debugging
- Reference docs/testing-strategy.md for testing approach
