# Profiling Quick Start Guide

> **Get started with profiling in 5 minutes**

---

## 📦 **Installation**

Add profiling to your crate:

```toml
# Cargo.toml
[dependencies]
silmaril-profiling = { path = "../profiling" }

[features]
# Development mode (recommended for dev builds)
dev = ["silmaril-profiling/profiling-puffin"]
```

---

## 🚀 **Basic Usage**

### **Step 1: Initialize Profiler**

```rust
use silmaril_profiling::{Profiler, ProfilerConfig};

fn main() {
    // Create profiler with dev defaults
    let profiler = Profiler::new(ProfilerConfig::default_dev());

    // Run your game loop
    game_loop(profiler);
}
```

### **Step 2: Profile Your Game Loop**

```rust
use silmaril_profiling::ProfileCategory;

fn game_loop(profiler: Profiler) {
    loop {
        // Mark frame start
        profiler.begin_frame();

        // Profile physics
        {
            let _guard = profiler.scope("Physics", ProfileCategory::Physics);
            update_physics();
        }

        // Profile rendering
        {
            let _guard = profiler.scope("Rendering", ProfileCategory::Rendering);
            render_frame();
        }

        // Get frame metrics
        let metrics = profiler.end_frame();

        // Optional: Print metrics
        if metrics.frame_time_ms > 16.0 {
            println!("Frame took {:.2}ms (budget exceeded!)", metrics.frame_time_ms);
        }
    }
}
```

### **Step 3: Run with Profiling**

```bash
# Development build with profiling
cargo run --features dev

# Release build (no profiling, zero overhead)
cargo run --release
```

---

## 📊 **Viewing Results**

### **Option 1: Chrome Tracing (Recommended)**

Export profiling data to view in Chrome:

```rust
// After running your game for a while...
let trace = profiler.export_chrome_trace(0..1000);
std::fs::write("session.json", trace)?;
```

**Then:**
1. Open Chrome/Chromium browser
2. Navigate to `chrome://tracing`
3. Click "Load" and select `session.json`
4. View interactive timeline with thread visualization

### **Option 2: Puffin Viewer (Real-time)**

For real-time profiling during development:

```bash
# Install Puffin viewer
cargo install puffin_viewer

# Run viewer
puffin_viewer
```

Then in your code:

```rust
use silmaril_profiling::backends::PuffinBackend;

let mut backend = PuffinBackend::new();

#[cfg(feature = "puffin_http")]
backend.start_server("0.0.0.0:8585");

// In game loop
loop {
    backend.begin_frame();
    // Your game code...
    backend.end_frame();
}
```

Connect to `localhost:8585` in the Puffin viewer for live profiling.

---

## 🤖 **AI Agent Integration**

Get structured metrics for AI training:

```rust
use silmaril_profiling::AgentFeedbackMetrics;

// Get comprehensive metrics
let metrics: AgentFeedbackMetrics = profiler.get_agent_metrics(world);

// Check performance
if !metrics.is_frame_budget_met {
    println!("Performance issue detected!");

    // Find bottleneck
    let bottleneck = metrics.time_by_category
        .iter()
        .max_by_key(|(_, time)| *time)
        .unwrap();

    println!("Bottleneck: {:?} taking {:.2}ms", bottleneck.0, bottleneck.1);
}

// Export for ML training
let json = serde_json::to_string(&metrics)?;
std::fs::write("training_data.json", json)?;
```

---

## ⚙️ **Configuration**

### **Using YAML Config**

Create `engine.config.yaml`:

```yaml
profiling:
  enabled: true
  persist: true
  output_dir: "profiling_data/"

budgets:
  game_loop: 16.0ms    # 60 FPS
  physics_step: 5.0ms
  rendering: 8.0ms
```

Load in code:

```rust
let config = ProfilerConfig::from_file("engine.config.yaml")?;
let profiler = Profiler::new(config);
```

### **Using Environment Variables**

```bash
# Enable profiling
PROFILE_ENABLE=1 \
PROFILE_PERSIST=1 \
PROFILE_DIR=./profiling_data \
cargo run --features dev
```

---

## 🎯 **Performance Budgets**

Set budgets to get automatic warnings:

```rust
// Set budgets
profiler.set_budget("game_loop", Duration::from_millis(16));
profiler.set_budget("physics_step", Duration::from_millis(5));

// Warnings logged automatically when exceeded:
// [WARN] Budget exceeded: game_loop took 18.2ms (budget: 16.0ms)
```

---

## 🔍 **Advanced: Query API**

Analyze profiling data programmatically:

```rust
// Get aggregate statistics
let stats = profiler.query()
    .frames(1000..2000)
    .category(ProfileCategory::Physics)
    .aggregate();

println!("Physics Statistics:");
println!("  Average: {:.2}us", stats.avg_time_us);
println!("  p95: {}us", stats.p95_us);
println!("  p99: {}us", stats.p99_us);
println!("  Calls: {}", stats.call_count);

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

## 📂 **Categories Reference**

Use appropriate categories for organized profiling:

```rust
ProfileCategory::ECS           // Entity/component operations
ProfileCategory::Rendering     // Vulkan rendering
ProfileCategory::Physics       // Physics simulation
ProfileCategory::Networking    // Network sync
ProfileCategory::Audio         // Sound system
ProfileCategory::Serialization // State encoding
ProfileCategory::Scripts       // Game logic
ProfileCategory::Unknown       // Uncategorized
```

---

## ✅ **Best Practices**

### **DO:**

✅ **Profile outer loops, not inner loops**
```rust
// ✅ GOOD
{
    profile_scope!("process_entities");
    for entity in entities {
        // Work here
    }
}
```

✅ **Use appropriate categories**
```rust
let _guard = profiler.scope("update_physics", ProfileCategory::Physics);
```

✅ **Export after profiling session**
```rust
// In background thread
std::thread::spawn(move || {
    let trace = profiler.export_chrome_trace(0..10000);
    std::fs::write("trace.json", trace).ok();
});
```

### **DON'T:**

❌ **Over-instrument hot loops**
```rust
// ❌ BAD - Called 1M times per frame!
for i in 0..1_000_000 {
    profile_scope!("inner");
    do_work();
}
```

❌ **Forget categories**
```rust
// ❌ BAD - No category
profile_scope!("update");
```

❌ **Block game loop on export**
```rust
// ❌ BAD - Freezes game!
let trace = profiler.export_chrome_trace(0..100_000);
std::fs::write("trace.json", trace)?;  // Blocks!
```

---

## 🧪 **Example: Complete Game Loop**

```rust
use silmaril_profiling::{Profiler, ProfilerConfig, ProfileCategory};
use std::time::Duration;

fn main() {
    // Initialize profiler
    let profiler = Profiler::new(ProfilerConfig::default_dev());

    // Set performance budgets
    profiler.set_budget("game_loop", Duration::from_millis(16));
    profiler.set_budget("physics", Duration::from_millis(5));
    profiler.set_budget("rendering", Duration::from_millis(8));

    // Game loop
    for frame in 0..1000 {
        profiler.begin_frame();

        // Input
        {
            let _guard = profiler.scope("Input", ProfileCategory::ECS);
            process_input();
        }

        // Physics
        {
            let _guard = profiler.scope("Physics", ProfileCategory::Physics);
            update_physics();
        }

        // Game logic
        {
            let _guard = profiler.scope("Game Logic", ProfileCategory::ECS);
            update_game_logic();
        }

        // Rendering
        {
            let _guard = profiler.scope("Rendering", ProfileCategory::Rendering);
            render_frame();
        }

        // Get metrics
        let metrics = profiler.end_frame();

        // Check performance
        if !metrics.is_frame_budget_met {
            println!("Frame {}: {:.2}ms (OVER BUDGET)", frame, metrics.frame_time_ms);

            // Show breakdown
            for (category, time) in &metrics.time_by_category {
                println!("  {:?}: {:.2}ms", category, time);
            }
        }
    }

    // Export for analysis
    println!("Exporting profiling data...");
    let trace = profiler.export_chrome_trace(0..1000);
    std::fs::write("session.json", trace).expect("Failed to write trace");
    println!("Saved to session.json - load in chrome://tracing");
}
```

---

## 🚨 **Troubleshooting**

### **Problem: "Profiling macros do nothing"**

**Solution:** Enable the profiling feature flag:
```bash
cargo run --features profiling-puffin
# Or use dev feature
cargo run --features dev
```

### **Problem: "High overhead, game is slow"**

**Solution:** Check for over-instrumentation:
```rust
// Check how many scopes per frame
let metrics = profiler.end_frame();
println!("Scopes this frame: {}", metrics.scope_count);

// Target: 200-500 scopes per frame
// If >1000, you're over-instrumenting
```

### **Problem: "Can't see timeline visualization"**

**Solution:** Export Chrome Trace format:
```rust
let trace = profiler.export_chrome_trace(0..1000);
std::fs::write("trace.json", trace)?;
```

Then load in `chrome://tracing` or `https://ui.perfetto.dev/`

---

## 📚 **Next Steps**

- **Read Architecture:** [docs/profiling.md](profiling.md)
- **Quick Reference:** [docs/PROFILING_QUICK_REFERENCE.md](PROFILING_QUICK_REFERENCE.md)
- **API Documentation:** `cargo doc --open --features profiling-puffin`
- **Examples:** Check `engine/profiling/examples/`

---

## 📊 **Performance Overhead**

| Configuration | Per-scope Overhead | Frame Overhead (500 scopes) |
|---------------|-------------------|----------------------------|
| Release (no features) | 0ns | 0ms |
| `--features metrics` | ~1ns | ~0.0005ms |
| `--features profiling-puffin` | ~50-200ns | ~0.1-0.6ms |

**Recommendation:** Use `--features dev` for development, no features for release.

---

**Last Updated:** 2026-02-01
**Status:** Production Ready
