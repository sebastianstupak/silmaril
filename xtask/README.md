# XTask - Build Automation for Silmaril Engine

This directory contains the build automation system for the Silmaril game engine, replacing the previous 1595-line justfile with a type-safe, zero-dependency Rust-based solution.

## Why XTask?

The xtask pattern is a standard way to implement build automation in Rust projects without external dependencies:

- **Zero Dependencies**: No need to install `just`, `make`, or any other tools
- **Cross-Platform**: Works identically on Windows, Linux, and macOS
- **Type-Safe**: Commands are validated at compile time using clap
- **Discoverable**: Full `--help` system on every command
- **Maintainable**: All logic is in Rust, not shell scripts
- **Fast**: Compiled once, runs instantly

## Architecture

```
xtask/
├── Cargo.toml          # Package definition
├── src/
│   ├── main.rs         # CLI entry point with clap
│   ├── build.rs        # Build commands (build, clean, etc.)
│   ├── test.rs         # Test commands (test, test-ecs, etc.)
│   ├── benchmark.rs    # Benchmark commands (bench, bench-ecs, etc.)
│   ├── dev.rs          # Dev commands (dev, dev-profiler, etc.)
│   ├── docker.rs       # Docker commands (dev-docker, etc.)
│   ├── quality.rs      # Quality commands (fmt, clippy, check)
│   ├── pgo.rs          # PGO commands (pgo-build-instrumented, etc.)
│   └── utils.rs        # Shared utilities (run_cargo, print_*, etc.)
└── README.md           # This file
```

## Usage

All commands follow the pattern:

```bash
cargo xtask <category> <command> [options]
```

### Quick Reference

```bash
# Build
cargo xtask build client
cargo xtask build server
cargo xtask build both

# Test
cargo xtask test all
cargo xtask test ecs
cargo xtask test physics

# Benchmark
cargo xtask bench all
cargo xtask bench ecs
cargo xtask bench physics

# Development
cargo xtask dev server
cargo xtask dev profiler
cargo xtask dev multi --count 4

# Quality
cargo xtask check         # Run all checks
cargo xtask fmt           # Format code
cargo xtask clippy        # Lint code

# PGO
cargo xtask pgo build-instrumented
cargo xtask pgo run-workload
cargo xtask pgo build-optimized

# Docker
cargo xtask docker dev
cargo xtask docker prod
```

## Adding New Commands

To add a new command, edit the appropriate module in `src/`:

### Example: Adding a Test Command

Edit `src/test.rs`:

```rust
#[derive(Subcommand)]
pub enum TestCommand {
    // ... existing commands
    /// Test my new feature
    MyFeature,
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

Usage:
```bash
cargo xtask test my-feature
```

### Example: Adding a Benchmark Command

Edit `src/benchmark.rs`:

```rust
#[derive(Subcommand)]
pub enum BenchmarkCommand {
    // ... existing commands
    /// Benchmark my new feature
    MyFeature,
}

pub fn execute(cmd: BenchmarkCommand) -> Result<()> {
    match cmd {
        // ... existing cases
        BenchmarkCommand::MyFeature => {
            print_section("Benchmarking My Feature");
            run_cargo_streaming(&["bench", "--package", "engine-myfeature"])?;
            print_success("My feature benchmarks complete");
        }
    }
    Ok(())
}
```

Usage:
```bash
cargo xtask bench my-feature
```

## Utilities

The `utils.rs` module provides common functions:

### Running Commands

```rust
// Run cargo command (captures output)
let output = run_cargo(&["build", "--release"])?;

// Run cargo command (streams output)
run_cargo_streaming(&["test", "--all-features"])?;

// Run non-cargo command
run_command("docker", &["build", "-t", "myimage", "."])?;

// Run non-cargo command (streams output)
run_command_streaming("docker-compose", &["up"])?;
```

### Output Formatting

```rust
// Print section header
print_section("Running Tests");

// Print success message
print_success("All tests passed");

// Print error message
print_error("Build failed");

// Print info message
print_info("Building with profiling enabled");

// Print warning message
print_warning("Profiling will slow down execution");
```

### Project Info

```rust
// Get cargo command (respects CARGO env var)
let cargo = cargo();

// Get project root
let root = project_root()?;
```

## Migration from Justfile

The xtask system replaces the 1595-line justfile with 90+ tasks. Here's the mapping:

| Justfile Command | XTask Command | Notes |
|------------------|---------------|-------|
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

The old justfile is archived as `justfile.old` for reference.

## Testing XTask

To test the xtask system:

```bash
# Show help
cargo xtask --help

# Test a simple command
cargo xtask build client

# Test with streaming output
cargo xtask test all

# Test error handling
cargo xtask build nonexistent  # Should show helpful error
```

## Performance

XTask is fast:
- **First run**: ~10s (compile time)
- **Subsequent runs**: <100ms (instant)
- **VS justfile**: Same runtime performance, no external dependency

## Dependencies

Minimal dependencies for xtask:

- `clap` - CLI parsing with derive macros
- `anyhow` - Error handling
- `cargo_metadata` - Cargo workspace introspection
- `serde` + `serde_json` - JSON handling
- `walkdir` - Directory traversal
- `colored` - Terminal colors
- `indicatif` - Progress bars
- `which` - Find executables

All dependencies are development-only and don't affect the engine.

## See Also

- [XTask Command Reference](../docs/rules/xtask-commands.md) - Full command documentation
- [Development Workflow](../docs/development-workflow.md) - How to use xtask in daily work
- [CLAUDE.md](../CLAUDE.md) - Updated with xtask usage

## Credits

The xtask pattern is described in the [Cargo Book](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#cargo-alias) and widely used in the Rust ecosystem (e.g., rust-analyzer, miri).
