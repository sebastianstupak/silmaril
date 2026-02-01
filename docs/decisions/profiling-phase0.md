# Decision Record: Profiling Infrastructure in Phase 0

**Date:** 2026-02-01
**Status:** ✅ Approved
**Decision Makers:** User + AI Agent (Claude)

---

## 📋 **Decision Summary**

**Move profiling infrastructure from Phase 4 to Phase 0.**

Profiling and observability infrastructure will be implemented as a foundation layer (Phase 0) rather than a polish feature (Phase 4).

---

## 🎯 **Context**

### **Original Plan (ROADMAP.md)**
- Phase 0: Documentation, CI/CD
- Phase 1: ECS + Rendering (with performance targets like "Query 10k entities < 0.5ms")
- Phase 4: Tracy Profiling integration

### **Problem Identified**
1. **Can't validate performance targets without measurement tools**
   - Phase 1 specifies "Spawn 10k entities < 1ms" but no way to verify
   - Criterion benchmarks are good but insufficient for runtime profiling

2. **Contradiction in documentation**
   - CLAUDE.md says "Profile early and often (Tracy)"
   - Risk management: "Profile early and often (Tracy)"
   - But roadmap defers profiling to week 14+

3. **Technical debt compounds**
   - Performance issues found in Phase 4 are 10x harder to fix
   - May require architectural changes that are expensive late

4. **AI agent feedback loops**
   - Project goal: "Complete visual feedback loops (render → analyze → iterate)"
   - Feedback requires metrics from frame 1

---

## 🔍 **Research Conducted**

### **AAA Game Engine Practices**

Sources:
- [Unity Profiling Best Practices](https://unity.com/how-to/best-practices-for-profiling-game-performance)
- [Riot Games Profiling](https://technology.riotgames.com/news/profiling-measurement-and-analysis)
- [Intel Unreal Guide](https://www.intel.com/content/www/us/en/developer/articles/technical/unreal-engine-optimization-profiling-fundamentals.html)

**Key findings:**
- Industry uses **tiered profiling** (lightweight → deep)
- **Profile before, during, and after** major changes
- Use **frame time (ms)** not FPS as primary metric
- Strategic instrumentation: 200-500 scopes for AAA games

### **Rust Profiling Ecosystem**

Sources:
- [Puffin Profiler](https://github.com/EmbarkStudios/puffin)
- [Profiling abstraction crate](https://github.com/aclysma/profiling)
- [Bencher CI](https://bencher.dev/learn/track-in-ci/rust/criterion/)

**Key findings:**
- **Puffin**: Rust-native, 1ns overhead when off, 50-200ns when on
- **Tracy**: Industry standard, more complex, C++ focused
- **Profiling crate**: Abstraction over multiple backends
- **Iai-cachegrind**: Deterministic benchmarks (instruction counts)

### **CI Benchmark Strategies**

**Key findings:**
- GitHub Actions VMs have high noise (10-30% variance)
- Solutions: Baseline comparison, Iai for determinism, Bencher for tracking
- Regression tolerance: 10% on Iai, 20% on Criterion

---

## ✅ **Decision Details**

### **1. Primary Profiler: Puffin**

**Choice:** Puffin as primary, Tracy as optional advanced option

**Rationale:**
- ✅ Rust-native (better integration)
- ✅ 1ns overhead when OFF (essentially free)
- ✅ 50-200ns when ON (acceptable in dev)
- ✅ Embark Studios maintains it (Rust gamedev leaders)
- ✅ Simple web-based viewer
- ✅ Chrome Tracing export built-in

**Alternative considered:** Tracy
- ❌ C++ focused, Rust support not first-class
- ✅ More powerful, industry standard
- **Decision:** Keep as optional for advanced users

### **2. Three-Tier System**

**Tier 0: Always-available metrics**
- Basic: frame time, FPS, memory
- ~1-2 microsecond overhead
- Optional in release via `--features metrics`

**Tier 1: Lightweight profiling**
- Per-system timing (~50 scopes)
- <0.1ms overhead
- Dev builds default

**Tier 2: Deep profiling**
- Per-function instrumentation (~200-500 scopes)
- 0.1-0.6ms overhead acceptable in dev
- Explicit `--features profiling-puffin`

**Rationale:** Matches industry best practices (Unity, Unreal)

### **3. Data Format: Chrome Tracing**

**Primary format:** Chrome Tracing JSON

**Rationale:**
- ✅ Industry standard
- ✅ Shows thread parallelism (fiber-style visualization)
- ✅ Visualizable in `chrome://tracing`
- ✅ Easy to parse for ML training
- ✅ Puffin exports to this format

**Alternative considered:** Custom binary format
- ❌ Reinventing the wheel
- ❌ No tooling ecosystem

### **4. Visualization: Fiber-Style Timeline**

**Choice:** Tracy/Chrome Tracing style (not hierarchical)

**Example:**
```
Main Thread:    [GameLoop──────────────────────────]
                 ├[Physics]──┐[Wait]┌[Rendering]──]
                             ↓      ↑
Worker Thread 1:       [Query Chunk 0]
Worker Thread 2:       [Query Chunk 1]
```

**Rationale:**
- ✅ Shows parallelism clearly
- ✅ Identifies blocking/waiting
- ✅ Industry standard (Tracy, Unreal Insights)
- ✅ Native Chrome Tracing support

### **5. Zero-Cost Abstraction**

**Implementation:**
```rust
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

**Enforcement:**
- ✅ `profiling` NOT in default features
- ✅ CI checks release binaries for profiling symbols
- ✅ `cargo deny` prevents accidental inclusion

**Rationale:**
- Hard requirement: zero overhead in release
- Matches Unity approach (ConditionalAttribute)

### **6. AI Agent Metrics**

**Comprehensive feedback metrics:**
```rust
pub struct AgentFeedbackMetrics {
    pub frame_time_ms: f32,
    pub frame_time_p95_ms: f32,
    pub time_by_category: HashMap<ProfileCategory, f32>,
    pub entity_count: u32,
    pub memory_used_mb: usize,
    pub draw_calls: u32,
    pub bandwidth_bytes_per_sec: usize,
    pub custom: HashMap<String, f32>,
}
```

**Rationale:**
- AI agents need structured data, not just visualization
- Enable automated performance regression detection
- Support ML training with historical data

### **7. Performance Budgets**

**Config + runtime API:**
```yaml
budgets:
  game_loop: 16.0ms
  physics_step: 5.0ms
  rendering: 8.0ms
```

```rust
profiler.set_budget("game_loop", Duration::from_millis(16));
```

**Warnings:** Automatic logging when exceeded

**Rationale:**
- Proactive performance management
- AI agents can adjust budgets dynamically
- Fail CI if critical budgets exceeded

### **8. Configuration: Multi-Source**

**Priority:** Env vars > Config file > Runtime API > Defaults

**Rationale:**
- Docker/CI: Use env vars
- Developers: Use config file
- AI agents: Use runtime API
- Flexibility for all use cases

### **9. Persistence: Configurable**

**Options:**
- Live only (ephemeral)
- Save to file
- Circular buffer (last N frames)
- Save on crash

**Rationale:**
- Different use cases need different strategies
- ML training needs historical data
- Debugging needs crash dumps

### **10. CI Integration**

**Dual strategy:**
- **Criterion**: Trend tracking (noisy but informative)
- **Iai-cachegrind**: Regression detection (deterministic)

**Thresholds:**
- Iai: 10% slowdown fails CI
- Criterion: 20% slowdown warns (due to VM noise)

**Rationale:**
- GitHub Actions VMs are noisy
- Need deterministic benchmarks for regressions
- Need time-based benchmarks for trends

---

## 📊 **Impact Analysis**

### **Timeline Impact**

**Before:**
- Phase 0: 1 week
- Phase 1-3: 9-12 weeks
- Phase 4: 2-3 weeks (includes profiling)
- **Total:** 12-16 weeks

**After:**
- Phase 0: 2-3 weeks (includes profiling)
- Phase 1-3: 9-12 weeks
- Phase 4: 2-3 weeks (advanced profiling only)
- **Total:** 13-18 weeks

**Net impact:** +1-2 weeks overall, but enables better development velocity

### **Risk Mitigation**

**Risks addressed:**
- ✅ Can't validate performance targets → Now can
- ✅ Late discovery of performance issues → Now caught early
- ✅ No AI feedback metrics → Now available from Phase 1
- ✅ Guessing at bottlenecks → Now have data

### **Developer Experience**

**Improvements:**
- ✅ Profile while developing (not after)
- ✅ Immediate feedback on performance impact
- ✅ CI catches regressions automatically
- ✅ AI agents have metrics from day one

---

## 🎯 **Success Criteria**

### **Phase 0 Completion**

Must have:
- [ ] Profiling works with `--features profiling-puffin`
- [ ] Zero overhead in release builds (verified)
- [ ] Chrome Trace export working
- [ ] AI agent metrics API complete
- [ ] Budget warnings implemented
- [ ] CI benchmark regression detection
- [ ] ~20 scopes instrumented in engine-core

### **Performance Targets**

- [ ] Overhead when OFF: <1ns per scope
- [ ] Overhead when ON: <200ns per scope
- [ ] Export 1000 frames: <100ms

### **Integration**

- [ ] Works on all platforms (Windows, Linux, macOS)
- [ ] Documentation complete
- [ ] Tests passing (unit + integration)
- [ ] Benchmarks establish baseline

---

## 📚 **References**

### **Discussion Thread**
- Initial question: "Shouldn't profiling be in early phases?"
- Research phase: AAA practices, Rust ecosystem
- Interview rounds: 3 rounds of Q&A
- Final approval: 2026-02-01

### **External Sources**
- [Unity Profiling Best Practices](https://unity.com/how-to/best-practices-for-profiling-game-performance)
- [Riot Games Profiling](https://technology.riotgames.com/news/profiling-measurement-and-analysis)
- [Puffin Profiler](https://github.com/EmbarkStudios/puffin)
- [RAD Telemetry](https://www.radgametools.com/telemetry.htm)
- [Chrome Tracing Format](https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview)

### **Internal Documents**
- [ROADMAP.md](../../ROADMAP.md) - Updated timeline
- [docs/tasks/phase0-profiling.md](../tasks/phase0-profiling.md) - Implementation plan
- [docs/profiling.md](../profiling.md) - Architecture documentation

---

## ✍️ **Signatures**

**Proposed by:** AI Agent (Claude Sonnet 4.5)
**Approved by:** User
**Date:** 2026-02-01
**Status:** ✅ Approved for implementation

---

**Next Steps:**
1. ✅ Update ROADMAP.md (completed)
2. ✅ Create phase0-profiling.md task file (completed)
3. ✅ Create profiling.md architecture doc (completed)
4. ✅ Create this decision record (completed)
5. ⏭️ Begin Phase 0.5 implementation
