# Justfile Command Reference

## Overview

The agent-game-engine uses `just` for cross-platform build automation. All commands follow a consistent naming pattern for easy discovery.

## Installation

```bash
cargo install just
```

## Command Patterns

### Test Commands

Run tests for specific features using `just test:{feature}`:

| Command | Description | Example |
|---------|-------------|---------|
| `just test` | Run all tests | `just test` |
| `just test:serialization` | Test serialization module (Phase 1.3) | 12 integration + 13 property tests |
| `just test:ecs` | Test ECS system (Phase 1.1-1.2) | Core entity/component tests |
| `just test:physics` | Test physics integration (Phase 3) | Rapier integration tests |
| `just test:renderer` | Test rendering (Phase 1.5-1.6) | Vulkan renderer tests |
| `just test:networking` | Test networking (Phase 2) | TCP/UDP, state sync tests |
| `just test:math` | Test math/SIMD operations | Vector/matrix tests |
| `just test:profiling` | Test profiling system (Phase 0.5) | Puffin integration tests |
| `just test:assets` | Test asset loading | Mesh/texture loading tests |
| `just test:observability` | Test observability features | Metrics, budgets tests |
| `just test:macros` | Test procedural macros | Derive macro tests |

### Benchmark Commands

Run benchmarks for specific features using `just benchmark:{feature}`:

| Command | Description | Performance Target |
|---------|-------------|-------------------|
| `just benchmark:all` | Run all benchmarks | Full suite |
| `just benchmark:serialization` | Benchmark serialization | 1000 entities < 5ms ✅ (99.3µs) |
| `just benchmark:ecs` | Benchmark ECS operations | 10M entities/sec |
| `just benchmark:physics` | Benchmark physics | 1000 bodies at 60 FPS |
| `just benchmark:renderer` | Benchmark rendering | 60 FPS at 1080p |
| `just benchmark:networking` | Benchmark network protocols | TCP/UDP throughput |
| `just benchmark:math` | Benchmark SIMD operations | Vector/matrix perf |
| `just benchmark:profiling` | Benchmark profiling overhead | <1% overhead |
| `just benchmark:spatial` | Benchmark spatial structures | Grid/octree queries |
| `just benchmark:allocators` | Benchmark memory allocators | Arena/pool perf |
| `just benchmark:assets` | Benchmark asset loading | Load times |
| `just benchmark:platform` | Benchmark platform abstraction | Cross-platform perf |

**Benchmark Output:**
- Results saved to `target/criterion/`
- HTML reports: `target/criterion/report/index.html`
- Use `just benchmark:compare` to compare with industry standards

### Development Workflow Commands

| Command | Description |
|---------|-------------|
| `just dev` | Start development environment |
| `just dev-profiler` | Start with profiling enabled |
| `just dev-logs` | Start with verbose logging |
| `just dev-tracy` | Start with Tracy profiler |
| `just dev-multi N` | Start N client instances |
| `just dev-clean` | Clean and restart dev environment |
| `just dev-status` | Check development environment status |

### Build Commands

| Command | Description |
|---------|-------------|
| `just build` | Build client + server (debug) |
| `just build-release` | Build optimized release |
| `just build-client` | Build client only |
| `just build-server` | Build server only |
| `just clean` | Clean build artifacts |

### Code Quality Commands

| Command | Description |
|---------|-------------|
| `just check` | Run all checks (format + clippy + test) |
| `just fmt` | Format all code |
| `just fmt-check` | Check formatting |
| `just clippy` | Run clippy lints |
| `just clippy-fix` | Auto-fix clippy issues |

### Profile-Guided Optimization (PGO)

| Command | Description |
|---------|-------------|
| `just pgo-build` | Build with PGO (full workflow) |
| `just pgo-instrumented` | Build instrumented binary |
| `just pgo-workload` | Run PGO workload |
| `just pgo-optimized` | Build PGO-optimized binary |
| `just pgo-test` | Test full PGO workflow |
| `just pgo-compare` | Compare PGO vs non-PGO performance |

## Usage Examples

### Testing Specific Features

```bash
# Test serialization after implementing new format
just test:serialization

# Test ECS after adding new component type
just test:ecs

# Run all tests
just test
```

### Benchmarking

```bash
# Benchmark serialization performance
just benchmark:serialization

# Compare with industry standards
just benchmark:compare

# Quick smoke test
just benchmark:quick
```

### Development Workflow

```bash
# Start dev environment
just dev

# Start with profiling
just dev-profiler

# Start multiple clients for multiplayer testing
just dev-multi 4

# Check status
just dev-status

# Clean restart
just dev-clean
```

### Full Release Build

```bash
# PGO-optimized release
just pgo-build

# Standard release
just build-release
```

## Adding New Commands

When adding new features, follow the naming pattern:

1. **Tests**: `test:{feature}` - Add to justfile under `# Run tests by feature`
2. **Benchmarks**: `benchmark:{feature}` - Add to justfile under `# === Benchmarks ===`
3. **Document**: Update this file with the new command

Example:
```just
# Test new feature
test\:myfeature:
    @echo "Running myfeature tests..."
    cargo test --package engine-myfeature

# Benchmark new feature
benchmark\:myfeature:
    @echo "Running myfeature benchmarks..."
    cargo bench --package engine-myfeature
```

### Benchmark Comparison

| Command | Description |
|---------|-------------|
| `just benchmark-compare` | Generate comparison report (agent-game-engine vs industry) |
| `just benchmark-thresholds` | Quick reference of all industry thresholds |

**Example:**
```bash
# View industry thresholds
just benchmark-thresholds

# Generate full comparison report
just benchmark-compare

# Custom output location
just benchmark-compare output=docs/MY_REPORT.md
```

**Industry Standards Included:**
- Unity DOTS/ECS performance
- Unreal Engine 5 (Nanite, Mass Entity)
- Bevy Engine (Rust ECS)
- AAA game industry standards
- Rapier physics benchmarks
- Competitive FPS networking (Valorant, CS2)
- MMO networking (WoW, FFXIV)

## Command Discovery

```bash
# List all commands
just --list

# Search for specific commands
just --list | grep test
just --list | grep benchmark
```

## See Also

- [Development Workflow](../development-workflow.md)
- [Benchmarking Guide](../benchmarking.md)
- [Testing Strategy](../testing-strategy.md)
- [Profiling Guide](../profiling.md)
