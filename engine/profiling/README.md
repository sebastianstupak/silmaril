# engine-profiling

> **Zero-cost profiling infrastructure for AI-driven game development**

Performance profiling and observability system enabling real-time feedback loops for AI agents.

---

## 📋 **Features**

- ✅ **Zero overhead in release** - Completely compiled away without feature flags
- ✅ **Three-tier system** - Lightweight metrics → deep profiling → GPU profiling
- ✅ **AI agent first** - Structured metrics API for programmatic access
- ✅ **Industry standard formats** - Chrome Tracing JSON export
- ✅ **Thread-aware** - Fiber-style timeline visualization showing parallelism
- ✅ **Performance budgets** - Automatic warnings when thresholds exceeded
- ✅ **Configurable** - YAML config + env vars + runtime API

---

## 🚀 **Quick Start**

### **Installation**

```toml
[dependencies]
engine-profiling = { path = "../profiling" }

[features]
# Development mode (Puffin profiler + metrics)
dev = ["engine-profiling/profiling-puffin"]

# Just lightweight metrics
metrics = ["engine-profiling/metrics"]
```

### **Basic Usage**

```rust
use engine_profiling::{Profiler, ProfilerConfig, profile_scope, ProfileCategory};

fn main() {
    // Initialize profiler
    let mut profiler = Profiler::new(ProfilerConfig::default_dev());

    loop {
        profiler.begin_frame();

        // Profile a scope
        {
            let _guard = profiler.scope("Physics", ProfileCategory::Physics);
            physics_step();
        }

        {
            let _guard = profiler.scope("Rendering", ProfileCategory::Rendering);
            render_frame();
        }

        // Get metrics
        let metrics = profiler.end_frame();
        println!("Frame time: {:.2}ms", metrics.frame_time_ms);
    }
}
```

### **Using Macros**

```rust
#[profile(category = "ECS")]
fn expensive_function() {
    profile_scope!("inner_loop");

    for item in items {
        profile_scope!("process_item");
        // Work
    }
}
```

**Note:** Macros compile to nothing without feature flags (zero overhead).

---

## 🎛️ **Feature Flags**

```toml
[features]
default = []

# Lightweight metrics (frame time, FPS, memory)
metrics = []

# Deep CPU profiling with Puffin
profiling-puffin = ["dep:puffin", "metrics"]

# Deep CPU profiling with Tracy (optional, advanced)
profiling-tracy = ["dep:tracy-client", "metrics"]

# Development mode (recommended for dev builds)
dev = ["profiling-puffin"]
```

### **Build Configurations**

```bash
# Release (no profiling, zero overhead)
cargo build --release

# Development (Puffin + metrics)
cargo build --features dev

# Advanced profiling (Tracy)
cargo build --features profiling-tracy

# Just metrics (no deep profiling)
cargo build --features metrics
```

---

## 📊 **Profiling Visualization**

### **Puffin Web Viewer (Recommended)**

The easiest way to visualize profiling data in real-time:

```rust
use silmaril_profiling::backends::PuffinBackend;

// Initialize Puffin backend
let mut backend = PuffinBackend::new();

// Optional: Start HTTP server for remote viewing
#[cfg(feature = "puffin_http")]
backend.start_server("0.0.0.0:8585");

// In your game loop
loop {
    backend.begin_frame();

    // Your game code with profile_scope!() macros
    {
        profile_scope!("game_logic", ProfileCategory::ECS);
        // ...
    }

    backend.end_frame();
}
```

**View in browser:**

1. Install the Puffin viewer:
   ```bash
   cargo install puffin_viewer
   ```

2. Run the viewer:
   ```bash
   puffin_viewer
   ```

3. Connect to `localhost:8585` in the viewer

The Puffin viewer provides:
- Real-time frame-by-frame profiling
- Thread visualization
- Scope timing histograms
- Frame time graphs
- Zoom and filter capabilities

### **Chrome Tracing Export**

Export profiling data for offline analysis:

```rust
use silmaril_profiling::backends::PuffinBackend;

let backend = PuffinBackend::new();

// ... run your game with profiling ...

// Export to Chrome Trace format
let trace_json = backend.export_chrome_trace();
std::fs::write("trace.json", trace_json)?;
```

**Visualize:**
- Load `trace.json` in `chrome://tracing` (Chrome/Chromium browsers)
- Load in Perfetto UI: https://ui.perfetto.dev/

**Timeline view:**
```
Main Thread:    [GameLoop──────────────────────────────────────]
                 ├[Physics]──────┐[Wait]┌[Rendering]──────────]
                                 ↓      ↑
Worker Thread 1:           [Query Chunk 0]
Worker Thread 2:           [Query Chunk 1]
Worker Thread 3:           [Query Chunk 2]
```

---

## 🤖 **AI Agent Integration**

### **Feedback Metrics**

Get comprehensive metrics for AI training loops:

```rust
let metrics: AgentFeedbackMetrics = profiler.get_agent_metrics(world);

// Check performance
if !metrics.is_frame_budget_met {
    println!("Frame time: {:.2}ms (budget exceeded!)", metrics.frame_time_ms);
}

// Identify bottleneck
let bottleneck = metrics.time_by_category
    .iter()
    .max_by_key(|(_, time)| *time)
    .unwrap();

println!("Bottleneck: {:?} ({:.2}ms)", bottleneck.0, bottleneck.1);

// Export for ML training
let json = serde_json::to_string(&metrics)?;
```

### **Query API**

Programmatically query profiling data:

```rust
// Get aggregate statistics
let stats = profiler.query()
    .frames(1000..2000)
    .category(ProfileCategory::Physics)
    .aggregate();

println!("Physics p95: {}us", stats.p95_us);

// Check if optimization worked
let before = profiler.query()
    .frames(0..1000)
    .scope("expensive_function")
    .aggregate();

// ... apply optimization ...

let after = profiler.query()
    .frames(1000..2000)
    .scope("expensive_function")
    .aggregate();

let improvement = (before.p95_us - after.p95_us) as f32 / before.p95_us as f32 * 100.0;
println!("Optimization improved p95 by {:.1}%", improvement);
```

---

## ⚙️ **Configuration**

### **YAML Config File**

```yaml
# engine.config.yaml
profiling:
  enabled: true
  persist: true
  output_dir: "profiling_data/"
  max_file_size_mb: 100
  format: chrome_trace

  retention:
    circular_buffer_frames: 1000
    save_on_budget_exceeded: true
    save_on_crash: true

  backends:
    puffin:
      enabled: true
      web_viewer_port: 8585

budgets:
  game_loop: 16.0ms
  physics_step: 5.0ms
  rendering: 8.0ms
  networking: 2.0ms
```

### **Environment Variables**

Override config via environment:

```bash
PROFILE_ENABLE=1 \
PROFILE_PERSIST=1 \
PROFILE_DIR=./data \
cargo run --features dev
```

### **Runtime API**

Configure programmatically:

```rust
let config = ProfilerConfig {
    enabled: true,
    persist_to_disk: true,
    output_dir: PathBuf::from("profiling_data"),
    budgets: [
        ("game_loop", Duration::from_millis(16)),
        ("physics_step", Duration::from_millis(5)),
    ].into(),
    ..Default::default()
};

let profiler = Profiler::new(config);
```

---

## 🎯 **Performance Budgets**

Set performance budgets and get automatic warnings:

```rust
// Set budgets
profiler.set_budget("game_loop", Duration::from_millis(16));
profiler.set_budget("physics_step", Duration::from_millis(5));

// Warnings logged automatically when exceeded:
// [WARN] Budget exceeded: game_loop took 18.2ms (budget: 16.0ms)
```

**AI agents can adjust budgets dynamically:**

```rust
// Relax budget for complex scenes
if entity_count > 10_000 {
    profiler.set_budget("rendering", Duration::from_millis(20));
}
```

---

## 📂 **Categories**

```rust
pub enum ProfileCategory {
    ECS,           // Entity/component operations
    Rendering,     // Vulkan rendering
    Physics,       // Rapier integration
    Networking,    // Client/server sync
    Audio,         // Sound system
    Serialization, // State encoding
    Scripts,       // Game logic
    Unknown,
}
```

Use categories for organized profiling:

```rust
let _guard = profiler.scope("update_physics", ProfileCategory::Physics);
```

---

## 🧪 **Testing**

```bash
# Run tests
cargo test

# Run benchmarks
cargo bench

# Measure profiling overhead
cargo bench profiling_overhead
```

**Benchmark targets:**
- Overhead when OFF: <1ns per scope
- Overhead when ON: <200ns per scope
- Export 1000 frames: <100ms

---

## 🔥 **Tracy Profiler (Advanced)**

For ultra-low overhead profiling of hot paths:

**See:** [TRACY_QUICKSTART.md](TRACY_QUICKSTART.md) for complete Tracy guide

### **Quick Start**

```bash
# 1. Download Tracy profiler
#    https://github.com/wolfpld/tracy/releases

# 2. Build with Tracy
cargo build --release --features profiling-tracy

# 3. Run your app
./target/release/your_app

# 4. Launch Tracy and connect to localhost
```

### **Why Tracy?**

- **< 10ns overhead** (5-20x faster than Puffin)
- Ideal for hot paths called thousands of times per frame
- Real-time analysis with nanosecond precision
- Remote profiling support

### **Example**

```rust
use silmaril_profiling::{profile_scope, ProfileCategory};

// Hot path - only ~10ns overhead per call
fn process_entity() {
    profile_scope!("process_entity", ProfileCategory::ECS);

    // Even nested scopes have minimal impact
    for component in components {
        profile_scope!("update_component");
        // ...
    }
}
```

**Run example:**
```bash
cargo run --example tracy_profiling --features profiling-tracy
```

---

## 📚 **Documentation**

### **Architecture**
- [docs/profiling.md](../../docs/profiling.md) - Complete architecture documentation
- [TRACY_QUICKSTART.md](TRACY_QUICKSTART.md) - Tracy profiler quick start guide
- [docs/tasks/phase0-profiling.md](../../docs/tasks/phase0-profiling.md) - Implementation plan

### **Examples**
- [examples/tracy_profiling.rs](examples/tracy_profiling.rs) - **Tracy profiler example** ⭐
- [examples/basic_profiling.rs](examples/basic_profiling.rs) - Basic usage
- [examples/ai_agent_metrics.rs](examples/ai_agent_metrics.rs) - AI agent integration
- [examples/chrome_trace_export.rs](examples/chrome_trace_export.rs) - Export and visualization

### **API Reference**
```bash
cargo doc --open --features dev
```

---

## 🔗 **Integration with Other Crates**

### **engine-core**

```rust
// Instrument ECS operations
impl World {
    #[profile(category = "ECS")]
    pub fn tick(&mut self, dt: f32) {
        profile_scope!("world_tick");
        // ...
    }
}
```

### **engine-renderer**

```rust
// Instrument rendering pipeline
impl Renderer {
    #[profile(category = "Rendering")]
    pub fn render_frame(&mut self) {
        profile_scope!("render_frame");
        // ...
    }
}
```

### **engine-networking**

```rust
// Instrument network operations
impl Server {
    #[profile(category = "Networking")]
    pub fn tick(&mut self) {
        profile_scope!("server_tick");
        // ...
    }
}
```

---

## 🚨 **Common Pitfalls**

### **Don't Over-Instrument**

```rust
// ❌ BAD: Called millions of times, overhead dominates
for i in 0..1_000_000 {
    profile_scope!("inner");
    do_tiny_work();
}

// ✅ GOOD: Instrument outer scope only
{
    profile_scope!("process_million_items");
    for i in 0..1_000_000 {
        do_tiny_work();
    }
}
```

### **Always Use Categories**

```rust
// ❌ BAD: No category
profile_scope!("update");

// ✅ GOOD: Categorized
let _guard = profiler.scope("update", ProfileCategory::Physics);
```

### **Don't Block on Export**

```rust
// ❌ BAD: Blocks game loop
let trace = profiler.export_chrome_trace(0..100_000);
std::fs::write("trace.json", trace)?;

// ✅ GOOD: Export in background
std::thread::spawn(move || {
    let trace = profiler.export_chrome_trace(0..100_000);
    std::fs::write("trace.json", trace).ok();
});
```

---

## 📖 **References**

### **External Resources**
- [Puffin Profiler](https://github.com/EmbarkStudios/puffin)
- [Chrome Tracing Format](https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview)
- [Unity Profiling Best Practices](https://unity.com/how-to/best-practices-for-profiling-game-performance)
- [Riot Games Profiling](https://technology.riotgames.com/news/profiling-measurement-and-analysis)

### **Industry Standards**
- AAA games use 200-500 profiling scopes
- Frame time (ms) is preferred over FPS
- Fiber-style timeline visualization (Tracy, Unreal Insights)

---

## 📊 **Performance Overhead**

| Configuration | Overhead per scope | Frame overhead (500 scopes) | Best For |
|---------------|-------------------|-----------------------------|----------|
| **Release (no features)** | 0ns (compiled away) | 0ms | Production builds |
| **--features metrics** | ~1ns | ~0.0005ms | Always-on monitoring |
| **--features profiling-tracy** | **< 10ns** | **< 0.005ms** | **Hot path profiling** ⭐ |
| **--features profiling-puffin** | ~50-200ns | ~0.1-0.6ms | System-level profiling |

**Recommendations:**
- **Production:** No features (zero overhead)
- **Development:** `--features dev` (Puffin for system profiling)
- **Hot path optimization:** `--features profiling-tracy` (minimal overhead)
- **Always-on metrics:** `--features metrics` (lightweight monitoring)

---

## ⚖️ **License**

Apache-2.0 (same as parent project)

---

## 🤝 **Contributing**

See [CLAUDE.md](../../CLAUDE.md) for development guidelines.

**Key requirements:**
- All new features must have tests
- Benchmarks for performance-critical code
- Update documentation
- Zero overhead in release builds (verified)

---

**Last Updated:** 2026-02-01
**Status:** ✅ Complete and Production-Ready (Phase 0.5)
**Completion Report:** [../../PHASE_0_5_PROFILING_COMPLETE.md](../../PHASE_0_5_PROFILING_COMPLETE.md)
