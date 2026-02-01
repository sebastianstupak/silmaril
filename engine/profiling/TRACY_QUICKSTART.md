# Tracy Profiler Quick Start

> **Ultra-low overhead real-time profiler for hot path analysis**

## Why Tracy?

Tracy provides **< 10ns overhead per scope**, making it 5-20x faster than Puffin. This makes it ideal for:

- Profiling hot paths called thousands of times per frame
- Real-time performance analysis
- Identifying frame spikes with nanosecond precision
- Remote profiling on embedded devices

## Quick Setup

### 1. Download Tracy Profiler

Get the latest release: https://github.com/wolfpld/tracy/releases

**Windows:** Download `Tracy-{version}.7z` and extract
**Linux/macOS:** Clone and build from source (requires CMake)

### 2. Build Your App with Tracy

```bash
# Development build
cargo build --features profiling-tracy

# Release build (recommended for accurate profiling)
cargo build --release --features profiling-tracy
```

### 3. Run and Profile

**Terminal 1:** Run your application
```bash
./target/release/your_app
```

**Terminal 2:** Launch Tracy
```bash
# Windows
Tracy.exe

# Linux/macOS
./tracy/profiler/build/Tracy-profiler
```

**Tracy GUI:** Click "Connect" → localhost

## Basic Usage

### Instrument Your Code

```rust
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

fn game_loop() {
    profile_scope!("game_loop");

    physics_update();
    render_frame();
}

fn physics_update() {
    // With category for better organization
    profile_scope!("physics_update", ProfileCategory::Physics);

    for entity in entities {
        profile_scope!("entity_physics");  // Hot path - only ~10ns overhead
        // Physics code
    }
}
```

### Frame Markers

```rust
use agent_game_engine_profiling::TracyBackend;

let mut backend = TracyBackend::new();

loop {
    backend.begin_frame();

    // Game loop with profile_scope! calls

    backend.end_frame();
}
```

## Example

Run the included example:

```bash
cargo run --example tracy_profiling --features profiling-tracy
```

Then connect with Tracy to see:
- Frame timeline
- Categorized scopes
- Nested hierarchies
- Real-time statistics

## What You'll See in Tracy

### Timeline View
```
Frame 1 ▼
  ├─ game_loop (15.8ms)
  │  ├─ ECS::entity_updates (0.1ms)
  │  ├─ Physics::physics_step (5.2ms)
  │  │  ├─ Physics::collision_detection (2.0ms)
  │  │  ├─ Physics::integration (1.5ms)
  │  │  │  └─ Physics::simd_batch_8 (0.05ms) ← Hot path visible!
  │  │  └─ Physics::constraint_solver (1.0ms)
  │  └─ Rendering::render_frame (10.3ms)
  │     ├─ Rendering::frustum_culling (0.5ms)
  │     ├─ Rendering::prepare_draw_calls (1.0ms)
  │     ├─ Rendering::record_commands (8.0ms)
  │     └─ Rendering::submit_queue (0.5ms)
```

### Statistics
- Min/Max/Mean frame time
- Frame time histogram
- Per-scope timing statistics
- Call count per scope
- Thread activity

## Performance Comparison

| Profiler | Overhead/Scope | Best For |
|----------|----------------|----------|
| **Tracy** | **< 10ns** | Hot paths, real-time analysis |
| Puffin | 50-200ns | System-level profiling |
| Metrics | 1-2μs | Always-on production metrics |

### When to Use Each

**Tracy (profiling-tracy feature):**
```rust
// ✅ Hot path - called 10,000 times/frame
fn process_entity() {
    profile_scope!("process_entity");  // Only ~10ns overhead
    // ...
}
```

**Puffin (profiling-puffin feature):**
```rust
// ✅ System level - called 10 times/frame
fn physics_system() {
    profile_scope!("physics_system");  // 50-200ns acceptable
    // ...
}
```

**Metrics (metrics feature):**
```rust
// ✅ Always-on production monitoring
fn game_loop() {
    #[cfg(feature = "metrics")]
    let _guard = profiler.scope("game_loop", ProfileCategory::ECS);
    // ...
}
```

## Hot Paths Already Instrumented

The following hot paths are instrumented with Tracy:

### Physics (`engine/physics/src/systems/integration_simd.rs`)
- ✅ `physics_integration_system_simd` - Main integration
- ✅ `process_parallel` - Parallel processing
- ✅ `process_sequential` - Sequential SIMD
- ✅ `process_batch_8_simd` - AVX2 8-wide (hot!)
- ✅ `process_batch_4_simd` - SSE 4-wide (hot!)

### ECS (`engine/core/src/ecs/query.rs`)
- ✅ Query iteration (via existing profiling)

### Math (`engine/math/src/transform.rs`)
- ⚠️  Inlined functions (profile at call sites)

## Zero-Cost When Disabled

Without `profiling-tracy` feature, all code compiles away:

```rust
// With feature: ~10ns
profile_scope!("my_function");

// Without feature: 0ns (compiled to nothing)
profile_scope!("my_function");
```

## Troubleshooting

### Tracy Won't Connect

**Check firewall:**
```bash
# Tracy uses port 8086
# Allow incoming connections on this port
```

**Verify application is running:**
```bash
# Tracy can only connect to running applications
# Start your app first, then Tracy
```

### Build Errors

**Update tracy-client:**
```bash
cargo update -p tracy-client
```

**Check feature flags:**
```bash
cargo build --features profiling-tracy --verbose
```

### High Overhead

- ✅ Use release build: `--release`
- ✅ Don't capture when not analyzing
- ✅ Disable debug symbols in release

## Advanced Features

### Remote Profiling

Profile on embedded device, analyze on PC:

```rust
// Configure tracy-client for remote IP
// See: https://github.com/wolfpld/tracy/releases
```

### Memory Profiling

Track allocations (future):

```rust
// Tracy supports memory profiling
// Integration planned for Phase 2+
```

### GPU Profiling

Vulkan/DX12 timeline (future):

```rust
// Tracy supports GPU profiling
// Integration planned for Phase 4+
```

## Resources

- **Tracy Documentation:** https://github.com/wolfpld/tracy/releases
- **tracy-client Crate:** https://docs.rs/tracy-client
- **Engine Profiling Docs:** [../../docs/profiling.md](../../docs/profiling.md)

## Quick Reference

```rust
// Import
use agent_game_engine_profiling::{profile_scope, ProfileCategory, TracyBackend};

// Frame markers
let mut backend = TracyBackend::new();
backend.begin_frame();
// ... game loop ...
backend.end_frame();

// Basic scope
profile_scope!("my_function");

// Categorized scope
profile_scope!("physics_step", ProfileCategory::Physics);

// Hot path (< 10ns overhead)
for entity in entities {
    profile_scope!("process_entity");
    // ...
}
```

---

**Status:** ✅ Ready for use (Task #54)
**Performance:** < 10ns overhead per scope
**Best For:** Hot path analysis, real-time profiling
