# Performance Analysis and Profiling

This document covers performance profiling tools, optimization strategies, and profiling best practices for the Agent Game Engine.

## Table of Contents

- [Profiling Tools](#profiling-tools)
  - [Tracy Profiler](#tracy-profiler)
  - [Puffin Profiler](#puffin-profiler)
  - [Built-in Metrics](#built-in-metrics)
- [Feature Flags](#feature-flags)
- [Quick Start](#quick-start)
- [Tracy Integration](#tracy-integration)
- [Profiling Hot Paths](#profiling-hot-paths)
- [Performance Targets](#performance-targets)
- [Best Practices](#best-practices)

## Profiling Tools

The engine supports multiple profiling backends, each with different characteristics:

### Tracy Profiler

**Best for:** Real-time frame-by-frame analysis, production profiling, remote profiling

- **Overhead:** < 10ns per scope
- **GUI:** Powerful standalone client with timeline view
- **Features:**
  - Frame markers for identifying performance spikes
  - Nanosecond precision timing
  - Remote profiling over network
  - Memory profiling (future)
  - GPU profiling (future)
  - Call stack capture
  - Source code integration

**Download:** https://github.com/wolfpld/tracy/releases

### Puffin Profiler

**Best for:** Quick in-engine profiling, development builds

- **Overhead:** 50-200ns per scope
- **GUI:** Embedded viewer (via puffin_egui) or standalone
- **Features:**
  - Frame timeline view
  - Category-based filtering
  - Chrome Tracing export
  - In-process profiling

### Built-in Metrics

**Best for:** Always-on lightweight monitoring

- **Overhead:** ~1-2μs per frame
- **Features:**
  - Frame time and FPS
  - Per-category timing
  - Performance budget warnings
  - Historical query API
  - Agent feedback metrics

## Feature Flags

Enable profiling via Cargo feature flags:

```toml
# Cargo.toml
[features]
default = []
profiling = ["profiling-tracy"]  # or "profiling-puffin"
```

Available backends (mutually exclusive):

- `profiling-tracy` - Tracy profiler backend
- `profiling-puffin` - Puffin profiler backend
- `metrics` - Lightweight metrics only (no deep profiling)

## Quick Start

### 1. Build with Profiling

```bash
# Build with Tracy profiling
cargo build --release --features profiling

# Or with Puffin profiling
cargo build --release --features profiling-puffin

# Or just metrics
cargo build --release --features metrics
```

### 2. Run Your Application

```bash
# Run the client with profiling enabled
cargo run --bin client --release --features profiling
```

### 3. Connect Tracy Client

1. Download Tracy from: https://github.com/wolfpld/tracy/releases
2. Run the Tracy profiler GUI
3. Click "Connect" to attach to your application
4. The profiler will display real-time frame data

## Tracy Integration

### Automatic Instrumentation

The following hot paths are automatically instrumented when `profiling` feature is enabled:

#### Physics System

```rust
// engine/physics/src/systems/integration.rs
pub fn physics_integration_system(world: &mut World, dt: f32) {
    #[cfg(feature = "profiling")]
    profile_scope!("physics_integration_system", ProfileCategory::Physics);

    // ... physics update code
}
```

#### SIMD Physics System

```rust
// engine/physics/src/systems/integration_simd.rs
pub fn physics_integration_system_simd(world: &mut World, dt: f32) {
    #[cfg(feature = "profiling")]
    profile_scope!("physics_integration_system_simd", ProfileCategory::Physics);

    {
        #[cfg(feature = "profiling")]
        profile_scope!("ecs_query_iteration", ProfileCategory::ECS);

        // Collect entities for batch processing
        for (_entity, (transform, velocity)) in world.query_mut::<(...) >() {
            // ...
        }
    }

    {
        #[cfg(feature = "profiling")]
        profile_scope!("simd_batch_processing", ProfileCategory::Physics);

        // SIMD batch processing
    }
}
```

### Manual Instrumentation

Add profiling to your own code:

```rust
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

fn my_expensive_function() {
    #[cfg(feature = "profiling")]
    profile_scope!("my_expensive_function", ProfileCategory::ECS);

    // ... your code here
}
```

For nested scopes:

```rust
fn game_update() {
    #[cfg(feature = "profiling")]
    profile_scope!("game_update", ProfileCategory::ECS);

    {
        #[cfg(feature = "profiling")]
        profile_scope!("ai_update", ProfileCategory::Scripts);
        // AI logic
    }

    {
        #[cfg(feature = "profiling")]
        profile_scope!("physics_update", ProfileCategory::Physics);
        // Physics logic
    }
}
```

## Profiling Hot Paths

### Currently Instrumented

The following hot paths have Tracy profiling scopes:

1. **Physics Integration Systems**
   - `physics_integration_system` - Scalar physics update
   - `physics_integration_system_simd` - SIMD physics update
   - `ecs_query_iteration` - ECS query iteration
   - `simd_batch_processing` - SIMD batch processing

2. **ECS Operations** (via `profiling` feature in engine-core)
   - `component_register` - Component registration
   - Query iteration (when profiling enabled)

### Future Hot Paths

The following systems will be instrumented in future updates:

- **Rendering Pipeline**
  - Command buffer recording
  - Draw call submission
  - Swapchain presentation

- **Transform Operations**
  - Transform composition
  - Transform hierarchy updates

- **Main Game Loop**
  - Frame begin/end
  - Input processing
  - State updates

## Performance Targets

### Physics System

| Entity Count | Target Time (60 FPS) | Current Performance |
|--------------|---------------------|---------------------|
| 100 | < 0.1ms | ✅ 0.004ms (scalar), 0.0015ms (SIMD) |
| 1,000 | < 0.5ms | ✅ 0.037ms (scalar), 0.013ms (SIMD) |
| 10,000 | < 5ms | ✅ 0.4ms (scalar), 0.13ms (SIMD) |
| 100,000 | < 50ms | ⚠️ TBD |

### ECS Query Iteration

| Entity Count | Target Time | Notes |
|--------------|-------------|-------|
| 1,000 | < 0.1ms | Prefetching enabled |
| 10,000 | < 1ms | Parallel iteration |
| 100,000 | < 10ms | Optimal chunk size |

### Frame Budget (60 FPS = 16.67ms)

- Physics: 5ms (30%)
- Rendering: 8ms (48%)
- Game Logic: 2ms (12%)
- Networking: 1ms (6%)
- Other: 0.67ms (4%)

## Best Practices

### 1. Zero-Cost Abstraction

All profiling code compiles to nothing when features are disabled:

```rust
// With profiling disabled, this has ZERO runtime cost
#[cfg(feature = "profiling")]
profile_scope!("expensive_work");
```

### 2. Scope Naming Convention

Use descriptive names that indicate what's being measured:

```rust
// Good: Specific and searchable
profile_scope!("physics_integration_simd_batch8");

// Bad: Too generic
profile_scope!("update");
```

### 3. Category Organization

Use appropriate categories to organize profiling data:

- `ECS` - Entity component system operations
- `Physics` - Physics simulation and integration
- `Rendering` - Vulkan rendering and GPU operations
- `Networking` - Network communication and state sync
- `Audio` - Audio playback and processing
- `Serialization` - Binary serialization/deserialization
- `Scripts` - Game logic and scripts (future)

### 4. Avoid Profiling Fine-Grained Operations

Don't profile operations that take < 1μs:

```rust
// BAD: Too granular, overhead > actual work
for i in 0..1000 {
    profile_scope!("single_add");
    result += i;
}

// GOOD: Profile the whole loop
profile_scope!("sum_computation");
for i in 0..1000 {
    result += i;
}
```

### 5. Use Release Builds for Profiling

Always profile with release builds:

```bash
# GOOD: Optimizations enabled
cargo build --release --features profiling

# BAD: Debug overhead skews results
cargo build --features profiling
```

### 6. Profile on Target Hardware

Profile on the actual target platform:

- Desktop: Profile on minimum spec hardware
- Console: Profile on devkit
- Web: Profile in browser (WASM build)

### 7. Check for Performance Budgets

Set budgets to catch regressions:

```rust
use agent_game_engine_profiling::Profiler;
use std::time::Duration;

let profiler = Profiler::new(ProfilerConfig::default());

// Set a budget: physics should take < 5ms
profiler.set_budget("physics_update", Duration::from_millis(5));

// Warnings will be logged if budget is exceeded
```

### 8. Use Frame Markers

Mark frames for timeline visualization:

```rust
// In your main loop
loop {
    profiler.begin_frame();

    // ... game update

    profiler.end_frame();
}
```

## Tracy-Specific Tips

### Analyzing Frame Spikes

1. Open Tracy and connect to your application
2. Look for red bars in the timeline (frames exceeding target)
3. Zoom into the spike frame
4. Examine which scopes took longer than usual
5. Check CPU usage, memory allocations, and context switches

### Comparing Frames

1. Select a "good" frame (baseline)
2. Select a "bad" frame (spike)
3. Use "Compare" view to see differences
4. Identify which scopes regressed

### Remote Profiling

Tracy supports profiling over network:

```bash
# On target machine
cargo run --release --features profiling

# On development machine
./Tracy -a target_ip_address
```

## Puffin-Specific Tips

### Exporting Chrome Traces

```rust
use agent_game_engine_profiling::export::chrome_trace;

// Export profiling data
let json = chrome_trace::export(&profiler);
std::fs::write("trace.json", json)?;

// Open in Chrome: chrome://tracing
```

## Common Issues

### "Illegal instruction" crash

**Cause:** Binary built with CPU features not supported by target

**Solution:**
```bash
# Build for broader compatibility
RUSTFLAGS="-C target-feature=+sse4.2" cargo build --release --features profiling
```

### Tracy won't connect

**Cause:** Application not running or firewall blocking

**Solution:**
- Ensure application is running
- Check firewall settings (Tracy uses TCP port 8086 by default)
- Try localhost: `./Tracy -a 127.0.0.1`

### High profiling overhead

**Cause:** Too many fine-grained scopes

**Solution:**
- Remove scopes from functions < 1μs
- Profile larger operations
- Use sampling profilers (perf, VTune) for hot spot identification

## Related Documentation

- [D:\dev\agent-game-engine\docs\profiling.md](docs/profiling.md) - Profiling system architecture
- [D:\dev\agent-game-engine\docs\benchmarking.md](docs/benchmarking.md) - Benchmark guidelines
- [D:\dev\agent-game-engine\engine\math\PERFORMANCE.md](engine/math/PERFORMANCE.md) - Math library performance
- [D:\dev\agent-game-engine\docs\OPTIMIZATION-REPORT.md](docs/OPTIMIZATION-REPORT.md) - Optimization results

## References

- [Tracy Profiler Manual](https://github.com/wolfpld/tracy/releases/latest/download/tracy.pdf)
- [Puffin Documentation](https://docs.rs/puffin/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)

---

**Last Updated:** 2026-02-01
**Next Review:** When implementing rendering profiling (Phase 2)
