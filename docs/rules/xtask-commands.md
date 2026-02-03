# XTask Command Reference

## Overview

The Silmaril engine uses `xtask` for cross-platform build automation with **zero external dependencies**. All commands are built directly into the repository using Cargo.

## Why xtask?

- ✅ **Zero external dependencies** - No need to install `just`, `make`, or any other tools
- ✅ **Cross-platform** - Works identically on Windows, Linux, and macOS
- ✅ **Type-safe** - Commands are validated at compile time
- ✅ **Discoverable** - Full help system with `--help` on every command
- ✅ **Maintainable** - All build logic is in Rust, not shell scripts

## Usage

All commands follow the pattern: `cargo xtask <category> <command>`

```bash
# Show all available commands
cargo xtask --help

# Show commands in a category
cargo xtask build --help
cargo xtask test --help
cargo xtask bench --help
```

## Command Categories

### Build Commands

```bash
cargo xtask build client         # Build client (dev)
cargo xtask build server         # Build server (dev)
cargo xtask build both           # Build both binaries (dev)
cargo xtask build client-release # Build client (release)
cargo xtask build server-release # Build server (release-server profile)
cargo xtask build release        # Build both (release)
cargo xtask build clean          # Clean build artifacts
```

### Test Commands

Run tests for specific features:

```bash
cargo xtask test all             # Run all tests
cargo xtask test client          # Client tests only
cargo xtask test server          # Server tests only
cargo xtask test ecs             # ECS system tests
cargo xtask test serialization   # Serialization tests
cargo xtask test physics         # Physics tests
cargo xtask test renderer        # Renderer tests
cargo xtask test networking      # Network tests
cargo xtask test math            # Math/SIMD tests
cargo xtask test profiling       # Profiling system tests
cargo xtask test macros          # Procedural macro tests
cargo xtask test verbose         # Run with --nocapture
```

### Benchmark Commands

Run benchmarks for specific features:

```bash
cargo xtask bench all            # Run all benchmarks
cargo xtask bench all-save       # Run all + save baseline
cargo xtask bench ecs            # ECS benchmarks
cargo xtask bench physics        # Physics benchmarks
cargo xtask bench renderer       # Renderer benchmarks
cargo xtask bench audio          # Audio system benchmarks (< 0.5ms update, < 0.1ms play, < 0.05ms position)
cargo xtask bench math           # Math/SIMD benchmarks
cargo xtask bench profiling      # Profiling overhead benchmarks
cargo xtask bench serialization  # Serialization benchmarks
cargo xtask bench networking     # Network benchmarks
cargo xtask bench platform       # Platform abstraction benchmarks
cargo xtask bench compare        # Industry comparison benchmarks
cargo xtask bench baseline       # Compare with saved baseline
cargo xtask bench save-baseline  # Save current as baseline
cargo xtask bench smoke          # Quick smoke test (CI-friendly)
cargo xtask bench view           # Open HTML report in browser
```

**Benchmark Output:**
- Results saved to `target/criterion/`
- HTML reports: `target/criterion/report/index.html`
- Use `cargo xtask bench view` to open report

### Development Commands

```bash
cargo xtask dev full             # Start full dev environment
cargo xtask dev client           # Run client with auto-reload
cargo xtask dev server           # Run server with auto-reload
cargo xtask dev logs             # Start with verbose logging
cargo xtask dev profiler         # Start with Puffin profiler
cargo xtask dev debug            # Start with debug symbols
cargo xtask dev release          # Start in release mode
cargo xtask dev validation       # Start with Vulkan validation layers
cargo xtask dev trace            # Start with full tracing
cargo xtask dev benchmark        # Quick dev benchmarks
cargo xtask dev multi --count 4  # Start 4 clients + 1 server
cargo xtask dev headless         # Start without rendering
cargo xtask dev clean            # Clean dev environment
cargo xtask dev status           # Check dev environment status
```

### Docker Commands

```bash
cargo xtask docker dev           # Start dev Docker environment
cargo xtask docker dev-detached  # Start dev (background)
cargo xtask docker dev-stop      # Stop dev environment
cargo xtask docker dev-logs      # View dev logs
cargo xtask docker dev-rebuild   # Rebuild dev images
cargo xtask docker prod          # Start production
cargo xtask docker prod-stop     # Stop production
cargo xtask docker prod-logs     # View production logs
cargo xtask docker rebuild       # Rebuild all images
cargo xtask docker sizes         # Show image sizes
cargo xtask docker clean         # Clean Docker artifacts
```

### Code Quality Commands

```bash
cargo xtask quality fmt          # Format all code
cargo xtask quality fmt-check    # Check formatting
cargo xtask quality clippy       # Run clippy lints
cargo xtask quality clippy-fix   # Auto-fix clippy issues
cargo xtask quality check        # Run all checks (fmt + clippy + test)

# Shortcuts
cargo xtask fmt                  # Shorthand for quality fmt
cargo xtask clippy               # Shorthand for quality clippy
cargo xtask fmt-check            # Shorthand for quality fmt-check
cargo xtask check                # Shorthand for quality check
```

### Profile-Guided Optimization (PGO)

```bash
cargo xtask pgo build-instrumented  # Step 1: Build instrumented
cargo xtask pgo run-workload        # Step 2: Run workload
cargo xtask pgo build-optimized     # Step 3: Build optimized
cargo xtask pgo compare             # Compare PGO vs non-PGO
cargo xtask pgo test                # Test PGO workflow
```

**PGO Expected Gains:** 5-15% performance improvement on typical workloads

### Utility Commands

```bash
cargo xtask doc                  # Build documentation
cargo xtask doc --open           # Build and open docs
cargo xtask watch                # Watch and rebuild
cargo xtask watch --test         # Watch and run tests
cargo xtask check-compile        # Fast compilation check
cargo xtask sizes                # Show binary sizes
cargo xtask update               # Update dependencies
cargo xtask outdated             # Show outdated deps
cargo xtask setup-hooks          # Install git hooks
cargo xtask run-client           # Run client
cargo xtask run-server           # Run server
```

## Usage Examples

### Testing Specific Features

```bash
# Test serialization after implementing new format
cargo xtask test serialization

# Test ECS after adding new component type
cargo xtask test ecs

# Run all tests
cargo xtask test all
```

### Benchmarking

```bash
# Benchmark serialization performance
cargo xtask bench serialization

# Compare with industry standards
cargo xtask bench compare

# Quick smoke test for CI
cargo xtask bench smoke

# View results
cargo xtask bench view
```

### Development Workflow

```bash
# Start dev environment
cargo xtask dev server

# Start with profiling
cargo xtask dev profiler

# Start multiple clients for multiplayer testing
cargo xtask dev multi --count 4

# Check status
cargo xtask dev status

# Clean restart
cargo xtask dev clean
```

### Full Release Build

```bash
# PGO-optimized release (recommended for production)
cargo xtask pgo build-instrumented
cargo xtask pgo run-workload
cargo xtask pgo build-optimized

# Or use automated comparison
cargo xtask pgo compare

# Standard release
cargo xtask build release
```

### Pre-commit Quality Checks

```bash
# Run all checks (recommended before committing)
cargo xtask check

# This runs:
# 1. cargo fmt --check
# 2. cargo clippy
# 3. cargo test --all-features
```

## Command Discovery

```bash
# Show all top-level commands
cargo xtask --help

# Show all build commands
cargo xtask build --help

# Show all test commands
cargo xtask test --help

# Show all benchmark commands
cargo xtask bench --help

# Show all dev commands
cargo xtask dev --help
```

## Comparison with Justfile

| Just Command | XTask Equivalent | Notes |
|--------------|------------------|-------|
| `just build` | `cargo xtask build both` | Build both binaries |
| `just test` | `cargo xtask test all` | Run all tests |
| `just test:ecs` | `cargo xtask test ecs` | Test ECS |
| `just bench` | `cargo xtask bench all` | Run all benchmarks |
| `just benchmark:ecs` | `cargo xtask bench ecs` | Benchmark ECS |
| `just dev` | `cargo xtask dev full` | Start dev environment |
| `just dev-server` | `cargo xtask dev server` | Run server |
| `just check` | `cargo xtask check` | All quality checks |
| `just fmt` | `cargo xtask fmt` | Format code |
| `just clippy` | `cargo xtask clippy` | Run clippy |
| `just pgo-build-instrumented` | `cargo xtask pgo build-instrumented` | PGO step 1 |
| `just dev-docker` | `cargo xtask docker dev` | Docker dev |

## Performance Targets

| Feature | Command | Performance Target | Status |
|---------|---------|-------------------|--------|
| Serialization | `cargo xtask bench serialization` | 1000 entities < 5ms | ✅ (99.3µs) |
| ECS | `cargo xtask bench ecs` | 10M entities/sec | ✅ |
| Physics | `cargo xtask bench physics` | 1000 bodies @ 60 FPS | ✅ |
| Renderer | `cargo xtask bench renderer` | 60 FPS @ 1080p | ✅ |
| Profiling | `cargo xtask bench profiling` | <1% overhead | ✅ |

## Adding New Commands

To add new commands, edit files in `xtask/src/`:

1. **Build commands** → `xtask/src/build.rs`
2. **Test commands** → `xtask/src/test.rs`
3. **Benchmark commands** → `xtask/src/benchmark.rs`
4. **Dev commands** → `xtask/src/dev.rs`
5. **Docker commands** → `xtask/src/docker.rs`
6. **Quality commands** → `xtask/src/quality.rs`
7. **PGO commands** → `xtask/src/pgo.rs`

Example:
```rust
// In xtask/src/test.rs
#[derive(Subcommand)]
pub enum TestCommand {
    // ... existing commands
    MyFeature, // Add new test command
}

pub fn execute(cmd: TestCommand) -> Result<()> {
    match cmd {
        // ... existing cases
        TestCommand::MyFeature => {
            print_section("Testing My Feature");
            run_cargo_streaming(&["test", "--package", "engine-myfeature"])?;
            print_success("My feature tests passed");
        }
    }
    Ok(())
}
```

## Industry Standards Comparison

Run `cargo xtask bench compare` to benchmark against:

- Unity DOTS/ECS performance
- Unreal Engine 5 (Nanite, Mass Entity)
- Bevy Engine (Rust ECS)
- AAA game industry standards
- Rapier physics benchmarks
- Competitive FPS networking (Valorant, CS2)
- MMO networking (WoW, FFXIV)

## See Also

- [Development Workflow](../development-workflow.md)
- [Benchmarking Guide](../benchmarking.md)
- [Testing Strategy](../TESTING_ARCHITECTURE.md)
- [Profiling Guide](../profiling.md)
- [xtask implementation](../../xtask/src/)
