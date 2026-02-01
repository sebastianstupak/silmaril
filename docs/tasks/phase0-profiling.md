# Phase 0: Profiling & Observability Infrastructure

> **Critical Foundation**: Profiling infrastructure MUST be implemented in Phase 0 to enable performance validation throughout development. You can't fix what you don't measure.

---

## 🎯 **Overview**

Implement comprehensive profiling and observability infrastructure to support:
- **AI agent feedback loops**: Structured metrics for automated game development
- **Performance validation**: Verify all performance targets as code is written
- **Development velocity**: Profile-guided optimization from day one
- **Zero release overhead**: Profiling completely compiled away in release builds

---

## 📐 **Architecture Decisions**

### **Decision 1: Primary Profiler - Puffin**

**Choice:** Puffin as primary profiler, Tracy as optional advanced option

**Rationale:**
- Puffin: Rust-native, 1ns overhead when off, 50-200ns when on
- Embark Studios (Rust gamedev leaders) maintains it
- Simple web-based viewer
- Chrome Tracing export built-in
- Optional Tracy support via `profiling` crate abstraction

**References:**
- [Puffin GitHub](https://github.com/EmbarkStudios/puffin)
- [Profiling abstraction crate](https://github.com/aclysma/profiling)

### **Decision 2: Three-Tier Profiling System**

**Tier 0: Always-available metrics** (optional in release via feature flag)
```rust
pub struct FrameMetrics {
    pub frame_time_ms: f32,
    pub fps: f32,
    pub memory_mb: usize,
    pub entity_count: u32,
}
```
- Negligible overhead (~1-2 microseconds)
- Available for user bug reports if compiled with `--features metrics`
- Zero cost if not compiled

**Tier 1: Lightweight profiling** (dev builds, `--features metrics`)
```rust
// Per-system timing
metrics::begin_scope("Physics");
physics_system();
metrics::end_scope();
```
- ~50 scopes total (major systems only)
- <0.1ms overhead per frame
- For high-level performance tracking

**Tier 2: Deep profiling** (explicit `--features profiling`)
```rust
#[profile]  // Macro-based instrumentation
fn expensive_function() {
    profile_scope!("inner_loop");
    // ...
}
```
- ~200-500 scopes total (AAA industry standard)
- Strategic placement (not blanket instrumentation)
- 0.1-0.6ms overhead per frame acceptable in dev

### **Decision 3: Data Format - Chrome Tracing + Query API**

**Primary format:** Chrome Tracing JSON (industry standard)
```json
[
  {"name": "Physics", "cat": "Physics", "ph": "X", "pid": 1, "tid": 2, "ts": 1100, "dur": 3200},
  {"name": "Rendering", "cat": "Rendering", "ph": "X", "pid": 1, "tid": 1, "ts": 4500, "dur": 8500}
]
```

**Why:**
- Shows thread parallelism (fiber-style visualization like Tracy)
- Visualizable in `chrome://tracing`
- Standard format for ML training data
- Easy to parse programmatically

**Plus: Query API for AI agents**
```rust
let metrics = profiler.query()
    .frame(1234)
    .category(ProfileCategory::Rendering)
    .aggregate();
```

### **Decision 4: Profiling Categories**

```rust
pub enum ProfileCategory {
    ECS,           // Entity/component operations
    Rendering,     // Vulkan rendering
    Physics,       // Rapier integration
    Networking,    // Client/server sync
    Audio,         // Sound system
    Serialization, // State encoding
    Scripts,       // Game logic (future)
    Unknown,
}
```

Matches Unity/Unreal industry standards.

### **Decision 5: Zero-Cost Abstraction**

**Compilation strategy:**
```rust
// Macros compile to nothing without feature flag
#[profile]
fn expensive_function() {
    profile_scope!("inner");
}

// Implementation:
#[cfg(feature = "profiling")]
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => { let _guard = profiling::scope($name); };
}

#[cfg(not(feature = "profiling"))]
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => { }; // Compiled away
}
```

**Hard enforcement:**
- `profiling` MUST NOT be in `default` features
- CI checks release binaries contain no profiling symbols
- `cargo deny` prevents accidental inclusion

### **Decision 6: Persistence & Configuration**

**Configuration hierarchy** (env vars > config file > defaults):
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
```

**Runtime API for AI agents:**
```rust
profiler.configure(ProfilerConfig {
    persist_to_disk: true,
    output_dir: "training_data/",
});
```

### **Decision 7: Performance Budgets**

**Config file + runtime API:**
```yaml
# performance_budgets.yaml
budgets:
  game_loop: 16.0ms
  physics_step: 5.0ms
  rendering: 8.0ms
  networking: 2.0ms
```

```rust
// Runtime adjustment by AI agent
profiler.set_budget("game_loop", Duration::from_millis(16));
```

**Warnings when exceeded:**
```
[WARN] Budget exceeded: game_loop took 18.2ms (budget: 16.0ms)
```

### **Decision 8: Visualization Style**

**Fiber-style timeline** (like Tracy, Unreal Insights):
```
Main Thread:    [GameLoop──────────────────────────────────────]
                 ├[Physics]──────┐[Wait]┌[Rendering]──────────]
                                 ↓      ↑
Worker Thread 1:           [Query Chunk 0]
Worker Thread 2:           [Query Chunk 1]
Worker Thread 3:           [Query Chunk 2]
```

Shows parallelism, blocking, and worker utilization clearly.

---

## 📋 **Implementation Tasks**

### **Task 0.5.1: Core Profiling Infrastructure** (2 days)

**File:** `engine/profiling/src/lib.rs`

**Deliverables:**
```rust
// Public API
pub struct Profiler { /* ... */ }

pub enum ProfileCategory { /* 8 categories */ }

#[macro_export]
macro_rules! profile_scope { /* ... */ }

pub struct FrameMetrics {
    pub frame_time_ms: f32,
    pub fps: f32,
    pub memory_mb: usize,
    pub entity_count: u32,
    pub time_by_category: HashMap<ProfileCategory, f32>,
    // ... (see "AI Agent Metrics" below)
}

pub struct ProfilerConfig { /* ... */ }

impl Profiler {
    pub fn new(config: ProfilerConfig) -> Self;
    pub fn begin_frame(&mut self);
    pub fn end_frame(&mut self) -> FrameMetrics;
    pub fn scope(&mut self, name: &str, category: ProfileCategory) -> ScopeGuard;
    pub fn set_budget(&mut self, scope: &str, duration: Duration);
    pub fn export_chrome_trace(&self, frames: Range<usize>) -> String;
    pub fn query(&self) -> QueryBuilder;
}
```

**Tests:**
- Unit tests for scope timing accuracy
- Test budget warnings
- Test Chrome Trace export format
- Test zero-cost when feature disabled

**Benchmarks:**
- Measure overhead when profiling ON (target: <200ns per scope)
- Measure overhead when profiling OFF (target: <1ns per scope)

**Time estimate:** 2 days

---

### **Task 0.5.2: Puffin Integration** (1 day)

**File:** `engine/profiling/src/backends/puffin_backend.rs`

**Deliverables:**
```rust
#[cfg(feature = "profiling-puffin")]
pub struct PuffinBackend {
    global_profiler: Arc<puffin::GlobalProfiler>,
}

impl ProfilerBackend for PuffinBackend {
    fn begin_scope(&mut self, name: &str, category: ProfileCategory);
    fn end_scope(&mut self);
    fn export_chrome_trace(&self) -> String;
}
```

**Integration:**
- Wrap `puffin::profile_scope!()` macro
- Map categories to Puffin categories
- Export Chrome Trace format from Puffin data
- Web viewer integration (Puffin viewer UI)

**Tests:**
- Integration test: verify Puffin captures scopes
- Test Chrome Trace export matches expected format
- Test web viewer can load exported data

**Time estimate:** 1 day

---

### **Task 0.5.3: Tracy Integration (Optional)** (1 day)

**File:** `engine/profiling/src/backends/tracy_backend.rs`

**Deliverables:**
```rust
#[cfg(feature = "profiling-tracy")]
pub struct TracyBackend {
    // tracy-client integration
}

impl ProfilerBackend for TracyBackend {
    // Same interface as Puffin
}
```

**Note:** Optional for advanced users. Not required for Phase 0 completion.

**Time estimate:** 1 day (optional)

---

### **Task 0.5.4: AI Agent Feedback Metrics** (1 day)

**File:** `engine/profiling/src/feedback_metrics.rs`

**Deliverables:**
```rust
pub struct AgentFeedbackMetrics {
    // Frame timing
    pub frame_time_ms: f32,
    pub frame_time_p95_ms: f32,  // 95th percentile
    pub fps: f32,
    pub is_frame_budget_met: bool,

    // System breakdown
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

impl AgentFeedbackMetrics {
    pub fn to_json(&self) -> String;
    pub fn from_profiler(profiler: &Profiler, world: &World) -> Self;
}
```

**Tests:**
- Test JSON serialization roundtrip
- Test metrics collection accuracy
- Property test: verify percentile calculations

**Time estimate:** 1 day

---

### **Task 0.5.5: Query API for AI Agents** (1 day)

**File:** `engine/profiling/src/query.rs`

**Deliverables:**
```rust
pub struct QueryBuilder<'a> {
    profiler: &'a Profiler,
    frame_range: Option<Range<usize>>,
    category_filter: Option<ProfileCategory>,
    scope_filter: Option<String>,
}

impl<'a> QueryBuilder<'a> {
    pub fn frame(mut self, frame: usize) -> Self;
    pub fn frames(mut self, range: Range<usize>) -> Self;
    pub fn category(mut self, cat: ProfileCategory) -> Self;
    pub fn scope(mut self, name: impl Into<String>) -> Self;

    pub fn aggregate(self) -> AggregateMetrics;
    pub fn timeline(self) -> Vec<TimelineEvent>;
    pub fn chrome_trace(self) -> String;
}

pub struct AggregateMetrics {
    pub total_time_us: u64,
    pub call_count: u32,
    pub avg_time_us: f32,
    pub p50_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
}
```

**Example usage:**
```rust
// AI agent queries specific scope
let physics_stats = profiler.query()
    .frames(1000..2000)
    .category(ProfileCategory::Physics)
    .aggregate();

println!("Physics p95: {}us", physics_stats.p95_us);
```

**Tests:**
- Test query filtering works correctly
- Test percentile calculations
- Test empty query results

**Time estimate:** 1 day

---

### **Task 0.5.6: Configuration System** (1 day)

**File:** `engine/profiling/src/config.rs`

**Deliverables:**
```rust
pub struct ProfilerConfig {
    pub enabled: bool,
    pub persist_to_disk: bool,
    pub output_dir: PathBuf,
    pub max_file_size_mb: usize,
    pub format: ProfileFormat,
    pub retention: RetentionConfig,
    pub budgets: HashMap<String, Duration>,
}

pub struct RetentionConfig {
    pub circular_buffer_frames: usize,
    pub save_on_budget_exceeded: bool,
    pub save_on_crash: bool,
}

impl ProfilerConfig {
    pub fn from_file(path: &Path) -> Result<Self, ConfigError>;
    pub fn from_env() -> Self;  // Env vars override
    pub fn default_dev() -> Self;
    pub fn default_release() -> Self;  // Everything off
}
```

**Config file format (YAML):**
```yaml
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

budgets:
  game_loop: 16.0ms
  physics_step: 5.0ms
  rendering: 8.0ms
```

**Tests:**
- Test YAML parsing
- Test env var override
- Test invalid config handling

**Time estimate:** 1 day

---

### **Task 0.5.7: Budget Warning System** (0.5 days)

**File:** `engine/profiling/src/budgets.rs`

**Deliverables:**
```rust
pub struct BudgetTracker {
    budgets: HashMap<String, Duration>,
    violations: Vec<BudgetViolation>,
}

pub struct BudgetViolation {
    pub scope: String,
    pub actual: Duration,
    pub budget: Duration,
    pub frame: usize,
    pub timestamp: Instant,
}

impl BudgetTracker {
    pub fn check(&mut self, scope: &str, duration: Duration, frame: usize);
    pub fn get_violations(&self) -> &[BudgetViolation];
    pub fn clear_violations(&mut self);
}
```

**Integration with logging:**
```rust
if let Some(violation) = budget_tracker.check("physics", duration, frame) {
    warn!(
        scope = violation.scope,
        actual_ms = violation.actual.as_secs_f32() * 1000.0,
        budget_ms = violation.budget.as_secs_f32() * 1000.0,
        "Performance budget exceeded"
    );
}
```

**Tests:**
- Test budget violations detected
- Test warnings logged correctly
- Test violation history tracking

**Time estimate:** 0.5 days

---

### **Task 0.5.8: CI Integration - Benchmark Regression** (1 day)

**File:** `.github/workflows/benchmark-regression.yml`

**Deliverables:**

**Criterion baseline comparison:**
```yaml
- name: Run benchmarks
  run: cargo bench --features profiling-puffin -- --save-baseline pr-${{ github.event.pull_request.number }}

- name: Compare with main
  run: |
    cargo bench --features profiling-puffin -- --baseline main --baseline pr-${{ github.event.pull_request.number }}
```

**Iai-cachegrind deterministic benchmarks:**
```yaml
- name: Iai benchmarks (deterministic)
  run: cargo iai

- name: Check for regressions
  run: |
    python scripts/check_iai_regression.py --threshold 10%
```

**Bencher integration (optional):**
- Track benchmark history over time
- Automatic regression detection
- Charts in PR comments

**Tests:**
- Test CI workflow runs on PRs
- Test regression detection works
- Test baseline storage

**Time estimate:** 1 day

---

### **Task 0.5.9: Integration with engine-core** (0.5 days)

**File:** `engine/core/src/profiling_integration.rs`

**Deliverables:**

Add profiling to critical ECS paths:
```rust
// In entity.rs
#[profile(category = "ECS")]
pub fn spawn(&mut self) -> Entity {
    profile_scope!("spawn_entity");
    // ...
}

// In query.rs
#[profile(category = "ECS")]
pub fn query<Q: Query>(&self) -> QueryIter<Q> {
    profile_scope!("query_setup");
    // ...
}

// In world.rs
impl World {
    #[profile(category = "ECS")]
    pub fn tick(&mut self, dt: f32) {
        profile_scope!("world_tick");
        // Run systems
    }
}
```

**Categories to instrument:**
- Entity spawn/despawn
- Component add/remove
- Query creation/iteration
- System execution
- Serialization

**Tests:**
- Test profiling data captured correctly
- Test zero overhead when disabled
- Benchmark overhead when enabled

**Time estimate:** 0.5 days

---

### **Task 0.5.10: Documentation** (0.5 days)

**Files:**
- `engine/profiling/README.md`
- `docs/profiling.md`
- API docs (rustdoc)

**Content:**
1. Getting started guide
2. Feature flag explanation
3. AI agent integration examples
4. Chrome Trace visualization guide
5. Performance budget configuration
6. CI benchmark setup

**Time estimate:** 0.5 days

---

## 🎯 **Success Criteria**

### **Functionality**
- [ ] Profiling works with `--features profiling-puffin`
- [ ] Zero overhead in release builds (verified by binary size check)
- [ ] Chrome Trace export works
- [ ] AI agent can query metrics programmatically
- [ ] Budget warnings logged correctly
- [ ] CI benchmark regression detection works

### **Performance**
- [ ] Overhead when profiling ON: <200ns per scope (measured)
- [ ] Overhead when profiling OFF: <1ns per scope (measured)
- [ ] Export 1000 frames to Chrome Trace: <100ms

### **Code Quality**
- [ ] 100% test coverage for public API
- [ ] All benchmarks pass
- [ ] Documentation complete (rustdoc + guides)
- [ ] CI green on all platforms

### **Integration**
- [ ] engine-core instrumented with ~20 scopes
- [ ] Metrics available to AI agents
- [ ] Config file loading works
- [ ] Env var overrides work

---

## 📊 **Time Estimate**

| Task | Days |
|------|------|
| 0.5.1 Core Infrastructure | 2.0 |
| 0.5.2 Puffin Integration | 1.0 |
| 0.5.3 Tracy Integration (optional) | 1.0 |
| 0.5.4 AI Feedback Metrics | 1.0 |
| 0.5.5 Query API | 1.0 |
| 0.5.6 Configuration System | 1.0 |
| 0.5.7 Budget Warnings | 0.5 |
| 0.5.8 CI Integration | 1.0 |
| 0.5.9 engine-core Integration | 0.5 |
| 0.5.10 Documentation | 0.5 |

**Total (without Tracy):** 8.5 days (~2 weeks)
**Total (with Tracy):** 9.5 days

---

## 🔗 **Dependencies**

**Rust crates:**
```toml
[dependencies]
puffin = "0.19"
tracy-client = { version = "0.17", optional = true }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
tracing = "0.1"

[dev-dependencies]
criterion = "0.5"
iai-callgrind = "0.13"
```

---

## 📚 **References**

### **Research Sources**
- [Puffin Profiler](https://github.com/EmbarkStudios/puffin)
- [Profiling abstraction crate](https://github.com/aclysma/profiling)
- [RAD Telemetry](https://www.radgametools.com/telemetry.htm)
- [Unity profiling best practices](https://unity.com/how-to/best-practices-for-profiling-game-performance)
- [Riot Games profiling](https://technology.riotgames.com/news/profiling-measurement-and-analysis)
- [Intel Unreal profiling guide](https://www.intel.com/content/www/us/en/developer/articles/technical/unreal-engine-optimization-profiling-fundamentals.html)

### **Standards**
- [Chrome Tracing Format](https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview)

---

## 🚨 **Critical Notes**

1. **Must be Phase 0**: Can't validate performance targets in later phases without profiling
2. **Zero release overhead**: Hard requirement, enforced by CI
3. **AI agent first**: Metrics API designed for programmatic access
4. **Industry standards**: Use Chrome Tracing, match Unity/Unreal granularity
5. **Modular**: `engine/profiling` is self-contained, other crates integrate via public API

---

**Last Updated:** 2026-02-01
**Status:** Ready for implementation
**Dependencies:** None (foundation layer)
