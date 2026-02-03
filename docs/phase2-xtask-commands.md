# Phase 2 Cargo Xtask Commands

> Quick reference for all Phase 2 networking commands via `cargo xtask`

## Overview

All Phase 2 functionality can be accessed through cargo xtask commands instead of shell scripts. This provides cross-platform compatibility and better integration with the build system.

## E2E Testing Commands

### Run All E2E Tests
```bash
cargo xtask test e2e
```
Runs all end-to-end integration tests for Phase 2 networking.

### Run E2E Tests with Output
```bash
cargo xtask test e2e-verbose
```
Same as above but shows detailed test output (useful for debugging).

## Phase 2 Specific Commands

### Full Phase 2 Demo
```bash
cargo xtask phase2 demo
```
Runs complete Phase 2 demo:
1. Starts server
2. Waits for initialization
3. Starts client
4. Checks Prometheus metrics endpoint
5. Cleans up

**Duration:** ~20 seconds

### Run Server and Client Together
```bash
cargo xtask phase2 run-both
```
Starts server in background, then runs client in foreground.

### Check Prometheus Metrics
```bash
cargo xtask phase2 check-metrics
```
Verifies that the Prometheus metrics endpoint is responding at `http://localhost:9090/metrics`.

### Phase 2 Validation Suite
```bash
cargo xtask phase2 validate
```
Runs complete Phase 2 validation:
1. ✅ Unit tests (engine-networking)
2. ✅ E2E tests (connectivity, handshake, etc.)
3. ✅ Protocol version check tests
4. ✅ Code quality (clippy)

**Use this before committing Phase 2 changes!**

### E2E Tests (Quick Access)
```bash
cargo xtask phase2 e2e-tests
```
Runs E2E tests with verbose output.

### Test Protocol Version Check
```bash
cargo xtask phase2 test-version-check
```
Tests protocol version mismatch detection.

### Test Connection Timeout
```bash
cargo xtask phase2 test-timeout
```
Tests client timeout detection (30-second idle).

## Standard Test Commands

### Test Networking Module
```bash
cargo xtask test networking
```
Runs all networking module tests (unit + integration).

### Run All Tests
```bash
cargo xtask test all
```
Runs all tests in the workspace.

## Running Binaries

### Run Server
```bash
cargo xtask run-server
```
Starts the game server at `0.0.0.0:7777` with metrics at `0.0.0.0:9090`.

### Run Client
```bash
cargo xtask run-client
```
Starts the game client (headless mode, runs 60 frames).

## Quick Workflows

### Before Committing Phase 2 Changes
```bash
# Run validation suite
cargo xtask phase2 validate

# If all pass, you're good to commit!
```

### Testing Metrics Endpoint
```bash
# Terminal 1: Start server
cargo xtask run-server

# Terminal 2: Check metrics
cargo xtask phase2 check-metrics

# Or manually:
curl http://localhost:9090/metrics
```

### Running E2E Tests During Development
```bash
# Quick check
cargo xtask test e2e

# With debug output
cargo xtask test e2e-verbose
```

## Environment Variables

### Metrics Port
```bash
# Change metrics port (default: 9090)
METRICS_PORT=8080 cargo xtask run-server
```

### Log Level
```bash
# Enable trace logging
RUST_LOG=trace cargo xtask run-server
RUST_LOG=trace cargo xtask run-client

# Specific modules
RUST_LOG=engine_networking=debug cargo xtask run-server
```

## Implementation Details

All Phase 2 commands are defined in:
- **xtask/src/phase2.rs** - Phase 2 specific commands
- **xtask/src/test.rs** - E2E test commands
- **xtask/src/main.rs** - Command routing

### Adding New Commands

To add a new Phase 2 command:

1. **Edit `xtask/src/phase2.rs`:**
```rust
#[derive(Subcommand)]
pub enum Phase2Command {
    // ... existing commands ...

    /// Your new command description
    MyCommand,
}

pub fn execute(cmd: Phase2Command) -> Result<()> {
    match cmd {
        // ... existing matches ...

        Phase2Command::MyCommand => {
            print_section("My Command");
            // Implementation here
            print_success("Done!");
        }
    }
    Ok(())
}
```

2. **Rebuild xtask:**
```bash
cargo build -p xtask
```

3. **Test:**
```bash
cargo xtask phase2 my-command
```

## Comparison with Scripts

### Old Way (Scripts - NO LONGER USED)
```bash
# ❌ Don't use these anymore
./scripts/test_prometheus_endpoint.ps1
./scripts/benchmark_all.sh
./examples/mmorpg/test_demo.bat
```

### New Way (Xtask)
```bash
# ✅ Use these instead
cargo xtask phase2 check-metrics
cargo xtask bench all
cargo xtask phase2 demo
```

**Benefits:**
- ✅ Cross-platform (Windows, Linux, macOS)
- ✅ Type-safe (compile-time checks)
- ✅ Integrated with cargo
- ✅ Self-documenting (`cargo xtask --help`)
- ✅ Consistent error handling

## Help

### List All Commands
```bash
cargo xtask --help
```

### Phase 2 Specific Help
```bash
cargo xtask phase2 --help
```

### Test Commands Help
```bash
cargo xtask test --help
```

## Quick Reference Table

| Task | Command |
|------|---------|
| Run E2E tests | `cargo xtask test e2e` |
| Run Phase 2 demo | `cargo xtask phase2 demo` |
| Validate Phase 2 | `cargo xtask phase2 validate` |
| Check metrics | `cargo xtask phase2 check-metrics` |
| Test version check | `cargo xtask phase2 test-version-check` |
| Test timeout | `cargo xtask phase2 test-timeout` |
| Run server | `cargo xtask run-server` |
| Run client | `cargo xtask run-client` |
| Run server + client | `cargo xtask phase2 run-both` |

---

**Last Updated:** 2026-02-03
**Phase 2 Status:** Quick Wins Complete (~85%)
