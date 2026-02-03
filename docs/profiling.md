# Profiling & Observability Architecture

> **Foundation for Performance**: Complete profiling infrastructure enabling AI-driven game development with real-time performance feedback loops.

---

## 🎯 **Overview**

The silmaril profiling system provides three tiers of observability:
1. **Lightweight metrics** (always available in dev)
2. **Deep CPU profiling** (opt-in via feature flag)
3. **GPU profiling** (Phase 4, advanced)

**Key Design Principles:**
- ✅ Zero overhead in release builds (compile-time eliminated)
- ✅ AI agent first (structured data export for ML training)
- ✅ Industry standard formats (Chrome Tracing JSON)
- ✅ Thread-aware timeline visualization
- ✅ Performance budget enforcement

---

## 🏗️ **Architecture**

### **Crate Structure**

```
engine/profiling/
├── src/
│   ├── lib.rs              # Public API, macros
│   ├── profiler.rs         # Core Profiler struct
│   ├── config.rs           # Configuration system
│   ├── feedback_metrics.rs # AI agent metrics
│   ├── query.rs            # Query API for agents
│   ├── budgets.rs          # Performance budgets
│   ├── backends/
│   │   ├── mod.rs
│   │   ├── puffin_backend.rs   # Puffin integration
│   │   └── tracy_backend.rs    # Tracy (optional)
│   └── export/
│       ├── chrome_trace.rs     # Chrome Tracing format
│       └── json.rs             # Structured JSON
├── tests/
│   ├── integration_tests.rs
│   └── benchmark_tests.rs
├── benches/
│   └── profiling_overhead.rs
└── Cargo.toml
```

### **Feature Flags**

```toml
[features]
default = []

# Metrics tier (lightweight, <0.1ms overhead)
metrics = []

# CPU profiling backends (pick one)
profiling-puffin = ["dep:puffin", "metrics"]
profiling-tracy = ["dep:tracy-client", "metrics"]

# Development mode (enables puffin + metrics)
dev = ["profiling-puffin"]
```

**Build configurations:**
```bash
# Release (no profiling)
cargo build --release

# Development (lightweight metrics + Puffin)
cargo build --features dev

# Advanced profiling (Tracy)
cargo build --features profiling-tracy

# Just metrics (no deep profiling)
cargo build --features metrics
```

---

## 🎨 **Three-Tier System**

### **Tier 0: Always-Available Metrics**

Basic performance metrics with negligible overhead (~1-2 microseconds/frame):

```rust
pub struct FrameMetrics {
    pub frame_time_ms: f32,
    pub fps: f32,
    pub memory_mb: usize,
    pub entity_count: u32,
}

// Always compiled, but only collected with feature flag
#[cfg(feature = "metrics")]
let metrics = engine.get_metrics();
```

**When to use:**
- User bug reports (optional `--features metrics` in release)
- High-level monitoring
- AI agent training (lightweight feedback)

### **Tier 1: Lightweight System Profiling**

Per-system timing with minimal overhead (~50 scopes, <0.1ms/frame):

```rust
#[cfg(feature = "metrics")]
fn update_systems() {
    metrics::begin_scope("Physics");
    physics_system();
    metrics::end_scope();

    metrics::begin_scope("Rendering");
    render_system();
    metrics::end_scope();
}
```

**When to use:**
- Development builds
- Identifying which major system is slow
- High-level performance tracking

### **Tier 2: Deep CPU Profiling**

Per-function instrumentation with acceptable dev overhead (~200-500 scopes, 0.1-0.6ms/frame):

```rust
#[profile]  // Compiles to nothing without feature
fn expensive_function() {
    profile_scope!("inner_loop");

    for item in items {
        profile_scope!("process_item");
        // ...
    }
}
```

**When to use:**
- Explicit `--features profiling-puffin` builds
- Performance optimization sessions
- Identifying micro-bottlenecks
- Generating flamegraphs

---

## 📊 **Data Formats**

### **Chrome Tracing Format (Primary)**

Industry standard JSON format for timeline visualization:

```json
[
  {
    "name": "Physics",
    "cat": "Physics",
    "ph": "X",
    "pid": 1,
    "tid": 2,
    "ts": 1100,
    "dur": 3200,
    "args": {"entity_count": 1000}
  },
  {
    "name": "Rendering",
    "cat": "Rendering",
    "ph": "X",
    "pid": 1,
    "tid": 1,
    "ts": 4500,
    "dur": 8500
  }
]
```

**Fields:**
- `name`: Scope name
- `cat`: Category (ECS, Rendering, Physics, etc.)
- `ph`: Event type ("X" = complete event with duration)
- `pid`: Process ID (1 = client, 2 = server)
- `tid`: Thread ID (shows parallelism)
- `ts`: Timestamp (microseconds)
- `dur`: Duration (microseconds)
- `args`: Optional metadata

**Visualization:**
Load in `chrome://tracing` for fiber-style timeline:

```
Main Thread:    [GameLoop──────────────────────────────────────]
                 ├[Physics]──────┐[Wait]┌[Rendering]──────────]
                                 ↓      ↑
Worker Thread 1:           [Query Chunk 0]
Worker Thread 2:           [Query Chunk 1]
Worker Thread 3:           [Query Chunk 2]
```

### **Structured Query API**

For AI agents to programmatically query profiling data:

```rust
// Aggregate metrics for specific scope
let physics_stats = profiler.query()
    .frames(1000..2000)
    .category(ProfileCategory::Physics)
    .aggregate();

println!("Physics p95: {}us", physics_stats.p95_us);

// Get raw timeline events
let events = profiler.query()
    .frame(1234)
    .timeline();

// Export to Chrome Trace
let trace_json = profiler.query()
    .frames(0..1000)
    .chrome_trace();
```

---

## 🤖 **AI Agent Integration**

### **Feedback Metrics**

Comprehensive metrics designed for AI agent training loops:

```rust
pub struct AgentFeedbackMetrics {
    // Frame timing
    pub frame_time_ms: f32,
    pub frame_time_p95_ms: f32,
    pub fps: f32,
    pub is_frame_budget_met: bool,

    // System breakdown (which system is bottleneck?)
    pub time_by_category: HashMap<ProfileCategory, f32>,

    // ECS stats
    pub entity_count: u32,
    pub archetype_count: u32,
    pub component_counts: HashMap<ComponentTypeId, u32>,

    // Memory
    pub memory_used_mb: usize,
    pub memory_peak_mb: usize,
    pub allocation_count: usize,

    // Rendering stats (Phase 1+)
    pub draw_calls: u32,
    pub triangle_count: u32,
    pub texture_memory_mb: usize,
    pub shader_switches: u32,

    // Networking stats (Phase 2+)
    pub bandwidth_bytes_per_sec: usize,
    pub packet_loss_percent: f32,
    pub latency_ms: f32,

    // Game state (extensible)
    pub game_time: f32,
    pub custom: HashMap<String, f32>,
}
```

### **Example AI Training Loop**

```rust
use silmaril::{Engine, profiling::AgentFeedbackMetrics};

fn main() {
    let mut engine = Engine::new();
    let mut ai_agent = AgentController::new();

    loop {
        // Render frame
        let result = engine.render_frame();

        // Get comprehensive metrics
        let metrics = result.feedback_metrics;

        // AI validates performance
        if !metrics.is_frame_budget_met {
            let bottleneck = metrics.time_by_category
                .iter()
                .max_by_key(|(_, time)| *time)
                .unwrap();

            ai_agent.log(format!(
                "Performance issue: {:?} taking {:.2}ms",
                bottleneck.0, bottleneck.1
            ));
        }

        // AI detects memory leaks
        if metrics.memory_used_mb > ai_agent.previous_memory * 1.1 {
            ai_agent.log("Warning: Memory increased 10%");
        }

        // Feed to AI model
        ai_agent.observe(result.color_buffer, metrics);
        let action = ai_agent.decide();
        engine.apply_action(action);

        ai_agent.previous_memory = metrics.memory_used_mb;
    }
}
```

### **Exporting Training Data**

```rust
// Export 10,000 frames for ML training
let trace = profiler.export_chrome_trace(0..10_000);
std::fs::write("training_data/profiling.json", trace)?;

// Or export structured metrics
let metrics_json = profiler.query()
    .frames(0..10_000)
    .to_json();
std::fs::write("training_data/metrics.json", metrics_json)?;
```

---

## 📈 **Performance Budgets**

### **Configuration**

Define budgets in YAML or at runtime:

```yaml
# performance_budgets.yaml
budgets:
  game_loop: 16.0ms      # 60 FPS target
  physics_step: 5.0ms
  rendering: 8.0ms
  networking: 2.0ms
  ecs_queries: 1.0ms
```

```rust
// Runtime API (for AI agents to adjust)
profiler.set_budget("game_loop", Duration::from_millis(16));
profiler.set_budget("physics_step", Duration::from_millis(5));
```

### **Budget Enforcement**

Automatic warnings when budgets exceeded:

```rust
impl Profiler {
    pub fn end_scope(&mut self, name: &str) {
        let duration = self.measure_duration(name);

        if let Some(budget) = self.budgets.get(name) {
            if duration > *budget {
                warn!(
                    scope = name,
                    actual_ms = duration.as_secs_f32() * 1000.0,
                    budget_ms = budget.as_secs_f32() * 1000.0,
                    "Performance budget exceeded"
                );

                self.violations.push(BudgetViolation {
                    scope: name.to_string(),
                    actual: duration,
                    budget: *budget,
                    frame: self.current_frame,
                });
            }
        }
    }
}
```

### **CI Integration**

Fail builds if critical budgets exceeded:

```yaml
# .github/workflows/performance.yml
- name: Run benchmarks with budgets
  run: cargo bench --features profiling-puffin

- name: Check budget violations
  run: |
    python scripts/check_budgets.py \
      --max-violations 0 \
      --critical "game_loop,physics_step"
```

---

## 🔧 **Configuration System**

### **Priority Order**

1. **Environment variables** (highest priority)
2. **Config file** (`engine.config.yaml`)
3. **Runtime API** (programmatic)
4. **Defaults** (lowest priority)

### **Environment Variables**

```bash
# Enable profiling
PROFILE_ENABLE=1

# Persistence
PROFILE_PERSIST=1
PROFILE_DIR=./profiling_data

# Format
PROFILE_FORMAT=chrome_trace

# Retention
PROFILE_CIRCULAR_BUFFER_FRAMES=1000
PROFILE_SAVE_ON_BUDGET_EXCEEDED=1
```

### **Config File**

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
    tracy:
      enabled: false
      server_address: "localhost:8086"

budgets:
  game_loop: 16.0ms
  physics_step: 5.0ms
  rendering: 8.0ms
```

### **Runtime API**

```rust
let config = ProfilerConfig {
    enabled: true,
    persist_to_disk: true,
    output_dir: PathBuf::from("profiling_data"),
    max_file_size_mb: 100,
    format: ProfileFormat::ChromeTrace,
    retention: RetentionConfig {
        circular_buffer_frames: 1000,
        save_on_budget_exceeded: true,
        save_on_crash: true,
    },
    budgets: [
        ("game_loop", Duration::from_millis(16)),
        ("physics_step", Duration::from_millis(5)),
    ].into(),
};

let profiler = Profiler::new(config);
```

---

## 🧪 **Testing Strategy**

### **Unit Tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_timing_accuracy() {
        let mut profiler = Profiler::new(ProfilerConfig::default());

        profiler.begin_scope("test", ProfileCategory::Unknown);
        std::thread::sleep(Duration::from_millis(10));
        profiler.end_scope("test");

        let metrics = profiler.query()
            .scope("test")
            .aggregate();

        assert!(metrics.total_time_us >= 10_000);
        assert!(metrics.total_time_us < 11_000);
    }

    #[test]
    fn test_budget_violation_detected() {
        let mut profiler = Profiler::new(ProfilerConfig::default());
        profiler.set_budget("test", Duration::from_millis(5));

        profiler.begin_scope("test", ProfileCategory::Unknown);
        std::thread::sleep(Duration::from_millis(10));
        profiler.end_scope("test");

        assert_eq!(profiler.violations.len(), 1);
        assert_eq!(profiler.violations[0].scope, "test");
    }

    #[test]
    fn test_chrome_trace_export() {
        let mut profiler = Profiler::new(ProfilerConfig::default());

        profiler.begin_frame();
        profiler.begin_scope("Physics", ProfileCategory::Physics);
        std::thread::sleep(Duration::from_millis(1));
        profiler.end_scope("Physics");
        profiler.end_frame();

        let trace = profiler.export_chrome_trace(0..1);

        // Verify JSON format
        let events: Vec<serde_json::Value> = serde_json::from_str(&trace).unwrap();
        assert_eq!(events[0]["name"], "Physics");
        assert_eq!(events[0]["cat"], "Physics");
    }
}
```

### **Benchmarks**

```rust
// benches/profiling_overhead.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_profiling_overhead_off(c: &mut Criterion) {
    c.bench_function("profiling_off", |b| {
        b.iter(|| {
            profile_scope!("test");
            black_box(42);
        });
    });
}

fn bench_profiling_overhead_on(c: &mut Criterion) {
    let mut profiler = Profiler::new(ProfilerConfig::default());

    c.bench_function("profiling_on", |b| {
        b.iter(|| {
            let _guard = profiler.scope("test", ProfileCategory::Unknown);
            black_box(42);
        });
    });
}

criterion_group!(benches, bench_profiling_overhead_off, bench_profiling_overhead_on);
criterion_main!(benches);
```

**Performance targets:**
- Overhead when OFF: <1ns per scope
- Overhead when ON: <200ns per scope
- Export 1000 frames: <100ms

---

## 🔗 **Integration with Other Crates**

### **engine-core**

Instrument critical ECS paths:

```rust
// engine/core/src/world.rs
use engine_profiling::profile_scope;

impl World {
    #[profile(category = "ECS")]
    pub fn tick(&mut self, dt: f32) {
        profile_scope!("world_tick");

        for system in &mut self.systems {
            profile_scope!(&system.name);
            system.run(self, dt);
        }
    }

    #[profile(category = "ECS")]
    pub fn spawn(&mut self) -> Entity {
        profile_scope!("spawn_entity");
        self.entities.allocate()
    }
}
```

### **engine-renderer**

Instrument rendering pipeline:

```rust
// engine/renderer/src/renderer.rs
impl Renderer {
    #[profile(category = "Rendering")]
    pub fn render_frame(&mut self, world: &World) {
        profile_scope!("render_frame");

        {
            profile_scope!("update_buffers");
            self.update_buffers(world);
        }

        {
            profile_scope!("record_commands");
            self.record_commands();
        }

        {
            profile_scope!("submit_queue");
            self.submit();
        }
    }
}
```

### **engine-networking**

Instrument network operations:

```rust
// engine/networking/src/server.rs
impl Server {
    #[profile(category = "Networking")]
    pub fn tick(&mut self) {
        profile_scope!("server_tick");

        {
            profile_scope!("receive_inputs");
            self.receive_client_inputs();
        }

        {
            profile_scope!("process_inputs");
            self.process_inputs();
        }

        {
            profile_scope!("send_state");
            self.broadcast_state();
        }
    }
}
```

---

## 📚 **Usage Examples**

### **Basic Profiling**

```rust
use engine_profiling::{Profiler, ProfilerConfig, ProfileCategory};

fn main() {
    let mut profiler = Profiler::new(ProfilerConfig::default_dev());

    loop {
        profiler.begin_frame();

        {
            let _guard = profiler.scope("Physics", ProfileCategory::Physics);
            physics_step();
        }

        {
            let _guard = profiler.scope("Rendering", ProfileCategory::Rendering);
            render_frame();
        }

        let metrics = profiler.end_frame();
        println!("Frame time: {:.2}ms", metrics.frame_time_ms);
    }
}
```

### **Exporting for Analysis**

```rust
// Save profiling session
let trace = profiler.export_chrome_trace(0..profiler.frame_count());
std::fs::write("session.json", trace)?;

// Load in chrome://tracing for visualization
```

### **AI Agent Querying**

```rust
// Check if optimization worked
let before_stats = profiler.query()
    .frames(0..1000)
    .scope("expensive_function")
    .aggregate();

// ... make optimization ...

let after_stats = profiler.query()
    .frames(1000..2000)
    .scope("expensive_function")
    .aggregate();

let improvement = (before_stats.p95_us - after_stats.p95_us) as f32
    / before_stats.p95_us as f32 * 100.0;

println!("Optimization improved p95 by {:.1}%", improvement);
```

---

## 🚨 **Common Pitfalls**

### **Don't Over-Instrument**

```rust
// ❌ BAD: Too granular, dominates actual work
for i in 0..1_000_000 {
    profile_scope!("inner_loop");  // Called 1M times!
    do_tiny_work();
}

// ✅ GOOD: Instrument outer loop
{
    profile_scope!("process_million_items");
    for i in 0..1_000_000 {
        do_tiny_work();
    }
}
```

### **Don't Forget Categories**

```rust
// ❌ BAD: No category, harder to analyze
profile_scope!("update");

// ✅ GOOD: Categorized
let _guard = profiler.scope("update", ProfileCategory::Physics);
```

### **Don't Block on Export**

```rust
// ❌ BAD: Blocks game loop
let trace = profiler.export_chrome_trace(0..100_000);
std::fs::write("trace.json", trace)?;  // Slow!

// ✅ GOOD: Export in background
std::thread::spawn(move || {
    let trace = profiler.export_chrome_trace(0..100_000);
    std::fs::write("trace.json", trace).ok();
});
```

---

## 🔥 **Tracy Profiler Integration**

### **What is Tracy?**

Tracy is a real-time, nanosecond precision profiler designed for games and real-time applications. It provides:

- **Ultra-low overhead**: < 10ns per scope (5-20x faster than Puffin)
- **Real-time visualization**: See performance data as it happens
- **Remote profiling**: Profile on one machine, analyze on another
- **Frame-accurate timeline**: Perfect for identifying frame spikes
- **Memory profiling**: Track allocations and memory usage
- **GPU profiling**: Timeline for GPU operations (future support)

### **When to Use Tracy vs Puffin**

**Use Tracy when:**
- Profiling hot paths called thousands of times per frame
- Need real-time feedback during optimization
- Require nanosecond precision timing
- Working on performance-critical code
- Remote profiling on embedded/mobile devices

**Use Puffin when:**
- Need web-based viewer (no separate download)
- Want Chrome Tracing export built-in
- Prefer self-contained Rust solution
- Acceptable overhead (50-200ns per scope)

**Performance Comparison:**
```
Overhead per scope:
  Tracy:  < 10ns  (best for hot paths)
  Puffin: 50-200ns (good for systems)
  Metrics: 1-2μs (lightweight always-on)
```

### **Setup**

#### **1. Download Tracy Profiler**

Get the latest release from: https://github.com/wolfpld/tracy/releases

Windows users can use the pre-built binary. Linux/macOS users may need to compile from source.

#### **2. Build with Tracy Enabled**

```bash
# Build application with Tracy
cargo build --features profiling-tracy --release

# Or for development
cargo build --features profiling-tracy
```

**Important:** Tracy and Puffin are mutually exclusive. Only one can be enabled at a time.

#### **3. Run and Connect**

```bash
# Run your application
./target/release/your_app

# In another terminal/window, launch Tracy profiler
tracy

# Connect to localhost in the Tracy GUI
```

### **Usage**

#### **Basic Profiling**

```rust
use silmaril_profiling::{profile_scope, ProfileCategory};

fn game_loop() {
    profile_scope!("game_loop");

    physics_update();
    render_frame();
}

fn physics_update() {
    profile_scope!("physics_update", ProfileCategory::Physics);

    // Physics code here
}
```

#### **Hot Path Instrumentation**

Tracy's ultra-low overhead makes it ideal for hot paths:

```rust
// SIMD batch processing (called 1000s of times per frame)
fn process_batch_8_simd(transforms: &mut [Transform], velocities: &[Vec3], dt: f32) {
    profile_scope!("process_batch_8_simd", ProfileCategory::Physics);

    // With Tracy's < 10ns overhead, this has negligible impact
    // even when called 1000+ times per frame
}

// Transform composition (called for every entity)
pub fn compose(&self, other: &Transform) -> Transform {
    profile_scope!("transform_compose", ProfileCategory::ECS);

    // Ultra-low overhead profiling
    let composed_affine = other.affine * self.affine;
    // ...
}
```

#### **Frame Markers**

Use Tracy backend for frame management:

```rust
use silmaril_profiling::TracyBackend;

let mut backend = TracyBackend::new();

loop {
    backend.begin_frame();

    // Game loop code with profile_scope! calls

    backend.end_frame();
}
```

### **Example**

See `engine/profiling/examples/tracy_profiling.rs` for a complete example:

```bash
cargo run --example tracy_profiling --features profiling-tracy
```

This example demonstrates:
- Frame markers and timeline
- Categorized scopes
- Nested scope hierarchies
- Real-time performance visualization

### **Best Practices**

#### **1. Use for Hot Paths**

Tracy's low overhead makes it perfect for profiling code called frequently:

```rust
// ✅ GOOD: Hot path with Tracy
fn query_entities(world: &World) {
    profile_scope!("query_entities", ProfileCategory::ECS);

    for entity in entities {
        profile_scope!("entity_update");  // < 10ns overhead
        // Process entity
    }
}
```

#### **2. Combine with Metrics**

Use Tracy for development, metrics for production:

```rust
#[cfg(feature = "profiling-tracy")]
use silmaril_profiling::profile_scope;

#[cfg(not(feature = "profiling-tracy"))]
macro_rules! profile_scope {
    ($name:expr) => {};
    ($name:expr, $cat:expr) => {};
}

fn game_loop(profiler: &Profiler) {
    // Tracy scope (dev only)
    profile_scope!("game_loop");

    // Metrics (always available with feature flag)
    #[cfg(feature = "metrics")]
    let _guard = profiler.scope("game_loop", ProfileCategory::ECS);

    // Game code
}
```

#### **3. Remote Profiling**

Tracy supports remote profiling for consoles/mobile:

```rust
// On embedded device
cargo build --target aarch64-unknown-linux-gnu --features profiling-tracy

// Tracy automatically connects over network
// Configure tracy-client for your target IP
```

### **Instrumented Hot Paths**

The following hot paths are instrumented with Tracy:

**Physics (engine/physics/src/systems/integration_simd.rs):**
- `physics_integration_system_simd` - Main integration system
- `process_parallel` - Parallel processing
- `process_sequential` - Sequential SIMD
- `process_batch_8_simd` - AVX2 8-wide processing
- `process_batch_4_simd` - SSE 4-wide processing

**ECS (engine/core/src/ecs/query.rs):**
- Query iteration (via existing profiling)
- Component access patterns
- Filter evaluation

**Math (engine/math/src/transform.rs):**
- Transform operations are inlined, profiling at call sites
- Compose, transform_point, transform_vector

### **Troubleshooting**

#### **Tracy Client Won't Connect**

```bash
# Check firewall settings
# Tracy uses port 8086 by default

# On Linux, allow incoming connections
sudo ufw allow 8086
```

#### **Build Errors with tracy-client**

```bash
# Ensure tracy-client is up to date
cargo update -p tracy-client

# Check feature flags
cargo build --features profiling-tracy --verbose
```

#### **High Overhead**

If you see higher than expected overhead:
- Ensure release build: `--release`
- Check Tracy is not in capture mode when not needed
- Verify no debug symbols in release build

### **Zero-Cost Abstraction**

When Tracy is disabled, all code compiles to nothing:

```rust
// With profiling-tracy feature: ~10ns overhead
profile_scope!("my_function");

// Without profiling-tracy feature: 0ns overhead (compiled away)
profile_scope!("my_function");
```

Assembly verification:
```bash
# Build without Tracy
cargo build --release
objdump -d target/release/your_app > no_tracy.asm

# Build with Tracy
cargo build --release --features profiling-tracy
objdump -d target/release/your_app > with_tracy.asm

# Compare: only tracy_client::span calls added
diff no_tracy.asm with_tracy.asm
```

---

## 📖 **References**

### **Industry Standards**
- [Chrome Tracing Format Spec](https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview)
- [Unity Profiler Best Practices](https://unity.com/how-to/best-practices-for-profiling-game-performance)
- [Riot Games Profiling](https://technology.riotgames.com/news/profiling-measurement-and-analysis)

### **Tools**
- [Puffin Profiler](https://github.com/EmbarkStudios/puffin)
- [Tracy Profiler](https://github.com/wolfpld/tracy)
- [RAD Telemetry](https://www.radgametools.com/telemetry.htm)

### **Academic**
- [Intel Unreal Profiling Guide](https://www.intel.com/content/www/us/en/developer/articles/technical/unreal-engine-optimization-profiling-fundamentals.html)

---

**Last Updated:** 2026-02-01
**Status:** ✅ Implementation Complete (Phase 0.5)
**Completion Report:** [../PHASE_0_5_PROFILING_COMPLETE.md](../PHASE_0_5_PROFILING_COMPLETE.md)
