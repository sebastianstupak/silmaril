# Profiling Quick Reference

> **Cheat sheet for using the profiling system**

---

## 🚀 **Quick Start**

### **Enable Profiling**

```bash
# Development build (Puffin profiler)
cargo run --features dev

# Release build (no profiling, zero overhead)
cargo run --release
```

### **Basic Instrumentation**

```rust
use engine_profiling::{profile_scope, ProfileCategory};

#[profile(category = "ECS")]
fn my_system() {
    profile_scope!("inner_work");
    // Your code here
}
```

---

## 🎛️ **Feature Flags**

| Feature | Use Case | Overhead |
|---------|----------|----------|
| (none) | Release builds | 0ns |
| `metrics` | Lightweight metrics only | ~1ns |
| `profiling-puffin` | Deep CPU profiling (recommended) | ~50-200ns |
| `profiling-tracy` | Advanced Tracy profiler | ~15ns |
| `dev` | Development (enables puffin + metrics) | ~50-200ns |

---

## 📂 **Categories**

```rust
ProfileCategory::ECS           // Entity/component operations
ProfileCategory::Rendering     // Vulkan rendering
ProfileCategory::Physics       // Rapier integration
ProfileCategory::Networking    // Client/server sync
ProfileCategory::Audio         // Sound system
ProfileCategory::Serialization // State encoding
ProfileCategory::Scripts       // Game logic
ProfileCategory::Unknown       // Uncategorized
```

---

## 📊 **AI Agent Metrics**

```rust
// Get comprehensive metrics for AI training
let metrics = profiler.get_agent_metrics(world);

println!("Frame time: {:.2}ms", metrics.frame_time_ms);
println!("FPS: {:.1}", metrics.fps);
println!("Entities: {}", metrics.entity_count);
println!("Memory: {}MB", metrics.memory_used_mb);

// Check performance budget
if !metrics.is_frame_budget_met {
    println!("WARNING: Frame budget exceeded!");
}

// Find bottleneck
let bottleneck = metrics.time_by_category
    .iter()
    .max_by_key(|(_, time)| *time);
```

---

## 🔍 **Query API**

```rust
// Get statistics for specific scope
let stats = profiler.query()
    .frames(0..1000)
    .category(ProfileCategory::Physics)
    .aggregate();

println!("Physics p95: {}us", stats.p95_us);
println!("Avg time: {:.2}us", stats.avg_time_us);
println!("Calls: {}", stats.call_count);

// Export Chrome Trace
let trace = profiler.query()
    .frames(0..1000)
    .chrome_trace();

std::fs::write("trace.json", trace)?;
```

---

## 📤 **Export Formats**

### **Chrome Tracing (Visualization)**

```rust
let trace = profiler.export_chrome_trace(0..1000);
std::fs::write("session.json", trace)?;
```

Load in `chrome://tracing` for timeline view.

### **JSON (ML Training)**

```rust
let metrics = profiler.get_agent_metrics(world);
let json = serde_json::to_string(&metrics)?;
std::fs::write("metrics.json", json)?;
```

---

## ⚙️ **Configuration**

### **YAML File**

```yaml
# engine.config.yaml
profiling:
  enabled: true
  persist: true
  output_dir: "profiling_data/"

budgets:
  game_loop: 16.0ms
  physics_step: 5.0ms
```

### **Environment Variables**

```bash
PROFILE_ENABLE=1 \
PROFILE_PERSIST=1 \
PROFILE_DIR=./data \
cargo run --features dev
```

### **Runtime API**

```rust
let config = ProfilerConfig {
    enabled: true,
    persist_to_disk: true,
    budgets: [
        ("game_loop", Duration::from_millis(16)),
    ].into(),
    ..Default::default()
};
```

---

## 🎯 **Performance Budgets**

```rust
// Set budget
profiler.set_budget("game_loop", Duration::from_millis(16));

// Automatic warnings when exceeded:
// [WARN] Budget exceeded: game_loop took 18.2ms (budget: 16.0ms)
```

---

## ✅ **Best Practices**

### **DO**

✅ Instrument all performance-critical paths
✅ Use appropriate categories
✅ Profile outer loops, not inner loops
✅ Export for analysis after profiling session

### **DON'T**

❌ Over-instrument (not every function)
❌ Profile code called 1000+ times per frame
❌ Forget to use categories
❌ Block game loop on export

---

## 🎨 **Common Patterns**

### **System Profiling**

```rust
impl System for PhysicsSystem {
    #[profile(category = "Physics")]
    fn run(&mut self, world: &mut World) {
        profile_scope!("physics_step");
        // ...
    }
}
```

### **Query Profiling**

```rust
#[profile(category = "ECS")]
fn update_transforms(world: &World) {
    profile_scope!("query_transforms");

    for (transform, velocity) in world.query::<(&Transform, &Velocity)>() {
        // ...
    }
}
```

### **Render Pass Profiling**

```rust
impl Renderer {
    #[profile(category = "Rendering")]
    fn render_frame(&mut self) {
        profile_scope!("render_frame");

        {
            profile_scope!("update_buffers");
            self.update_buffers();
        }

        {
            profile_scope!("record_commands");
            self.record_commands();
        }
    }
}
```

---

## 🧪 **Testing Profiling Code**

```rust
#[test]
fn test_my_system_performance() {
    let mut profiler = Profiler::new(ProfilerConfig::default());
    profiler.set_budget("my_system", Duration::from_millis(5));

    profiler.begin_frame();
    my_system();
    profiler.end_frame();

    let stats = profiler.query()
        .scope("my_system")
        .aggregate();

    assert!(stats.p95_us < 5000); // Under 5ms budget
}
```

---

## 🚨 **Troubleshooting**

### **"Profiling not working"**

```bash
# Check feature flag is enabled
cargo run --features profiling-puffin

# Or use dev feature
cargo run --features dev
```

### **"High overhead"**

```rust
// ❌ BAD: Too many scopes
for i in 0..1_000_000 {
    profile_scope!("item");
}

// ✅ GOOD: One outer scope
{
    profile_scope!("process_items");
    for i in 0..1_000_000 {
        // ...
    }
}
```

### **"Can't see thread parallelism"**

Make sure you're exporting Chrome Trace format and viewing in `chrome://tracing`:

```rust
let trace = profiler.export_chrome_trace(0..1000);
std::fs::write("trace.json", trace)?;
```

---

## 📊 **Performance Overhead Table**

| Scopes | Profiling OFF | Profiling ON (Puffin) |
|--------|---------------|----------------------|
| 50 | 0ns | ~2.5-10μs |
| 200 | 0ns | ~10-40μs |
| 500 | 0ns | ~25-100μs |

**Recommendation:** 200-500 scopes for AAA-quality profiling

---

## 🔗 **Full Documentation**

- **Architecture:** [docs/profiling.md](profiling.md)
- **Implementation:** [docs/tasks/phase0-profiling.md](tasks/phase0-profiling.md)
- **Decision Record:** [docs/decisions/profiling-phase0.md](decisions/profiling-phase0.md)
- **Crate README:** [engine/profiling/README.md](../engine/profiling/README.md)

---

**Last Updated:** 2026-02-01
