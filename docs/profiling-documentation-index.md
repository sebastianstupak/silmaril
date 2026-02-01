# Profiling Documentation Index

> **Complete guide to profiling documentation for agent-game-engine**

---

## 📚 **Documentation Structure**

### **🚀 Quick Start (Start Here)**
- **[PROFILING_QUICK_START.md](PROFILING_QUICK_START.md)** - Get up and running in 5 minutes
  - Installation
  - Basic usage with copy-paste examples
  - Viewing results
  - Common pitfalls

### **📖 Reference Documentation**
- **[PROFILING_QUICK_REFERENCE.md](PROFILING_QUICK_REFERENCE.md)** - Cheat sheet for daily use
  - Feature flags
  - Categories
  - AI agent metrics
  - Query API
  - Configuration options
  - Best practices

### **🏗️ Architecture Documentation**
- **[profiling.md](profiling.md)** - Complete architecture and design
  - Three-tier system design
  - Data formats (Chrome Tracing)
  - AI agent integration
  - Performance budgets
  - Configuration system
  - Testing strategy

### **📋 Implementation Plan**
- **[tasks/phase0-profiling.md](tasks/phase0-profiling.md)** - Detailed task breakdown
  - 10 tasks with time estimates
  - Success criteria
  - Dependencies
  - References

### **✅ Completion Report**
- **[../PHASE_0_5_PROFILING_COMPLETE.md](../PHASE_0_5_PROFILING_COMPLETE.md)** - Phase 0.5 completion summary
  - All completed tasks
  - Performance metrics achieved
  - Files created
  - Next steps

### **📦 Crate Documentation**
- **[../engine/profiling/README.md](../engine/profiling/README.md)** - Crate-level documentation
  - Features overview
  - Installation
  - API examples
  - Puffin integration
  - Chrome Trace export
  - Common pitfalls

### **🔧 API Documentation**
- **Rustdoc:** `cargo doc --open --features profiling-puffin,config,metrics`
  - Complete API reference
  - Type documentation
  - Method examples
  - Module organization

---

## 🎯 **Which Document Should I Read?**

### **I want to start using profiling NOW**
→ [PROFILING_QUICK_START.md](PROFILING_QUICK_START.md)

### **I need a quick reference while coding**
→ [PROFILING_QUICK_REFERENCE.md](PROFILING_QUICK_REFERENCE.md)

### **I want to understand the architecture**
→ [profiling.md](profiling.md)

### **I need to implement profiling in my system**
→ [tasks/phase0-profiling.md](tasks/phase0-profiling.md)

### **I want to see what was completed**
→ [../PHASE_0_5_PROFILING_COMPLETE.md](../PHASE_0_5_PROFILING_COMPLETE.md)

### **I need crate-level details**
→ [../engine/profiling/README.md](../engine/profiling/README.md)

### **I need API documentation**
→ Run `cargo doc --open --features profiling-puffin,config,metrics`

---

## 📝 **Examples**

Live code examples are available in `engine/profiling/examples/`:

1. **basic_usage.rs** - Basic profiling with scope guards
2. **agent_feedback.rs** - AI agent metrics integration
3. **query_api_demo.rs** - Querying profiling data programmatically
4. **puffin_basic.rs** - Puffin backend usage

**Run examples:**
```bash
cd engine/profiling
cargo run --example basic_usage --features profiling-puffin,config,metrics
cargo run --example agent_feedback --features profiling-puffin,config,metrics
```

---

## 🧪 **Testing**

### **Unit Tests**
```bash
cd engine/profiling
cargo test --features profiling-puffin,config,metrics
```

### **Integration Tests**
```bash
cd engine/profiling
cargo test --test config_integration --features profiling-puffin,config,metrics
```

### **Benchmarks**
```bash
cd engine/profiling
cargo bench --features profiling-puffin
```

---

## 🔗 **External Resources**

### **Tools**
- [Puffin Profiler](https://github.com/EmbarkStudios/puffin) - Primary profiler
- [Chrome Tracing](chrome://tracing) - Timeline visualization
- [Perfetto UI](https://ui.perfetto.dev/) - Alternative visualization

### **Standards**
- [Chrome Tracing Format Spec](https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview)

### **Best Practices**
- [Unity Profiling](https://unity.com/how-to/best-practices-for-profiling-game-performance)
- [Riot Games Profiling](https://technology.riotgames.com/news/profiling-measurement-and-analysis)

---

## 📊 **Quick Reference Tables**

### **Feature Flags**

| Feature | Use Case | Overhead |
|---------|----------|----------|
| (none) | Release builds | 0ns |
| `metrics` | Lightweight metrics | ~1ns |
| `profiling-puffin` | Deep profiling | ~50-200ns |
| `dev` | Development mode | ~50-200ns |

### **Categories**

| Category | Use For |
|----------|---------|
| `ECS` | Entity/component operations |
| `Rendering` | Vulkan rendering |
| `Physics` | Physics simulation |
| `Networking` | Network sync |
| `Audio` | Sound system |
| `Serialization` | State encoding |
| `Scripts` | Game logic |
| `Unknown` | Uncategorized |

### **Configuration Priority**

1. **Environment variables** (highest)
2. **Config file** (YAML)
3. **Runtime API**
4. **Defaults** (lowest)

---

## ✅ **Documentation Checklist**

For contributors adding new profiling features:

- [ ] Update [profiling.md](profiling.md) if architecture changes
- [ ] Update [PROFILING_QUICK_REFERENCE.md](PROFILING_QUICK_REFERENCE.md) with new APIs
- [ ] Update [PROFILING_QUICK_START.md](PROFILING_QUICK_START.md) if basic usage changes
- [ ] Update [../engine/profiling/README.md](../engine/profiling/README.md) for crate changes
- [ ] Add rustdoc comments to all public APIs
- [ ] Add examples if introducing major new features
- [ ] Update this index if adding new documents

---

## 🚀 **Getting Started Checklist**

New developers should:

1. [ ] Read [PROFILING_QUICK_START.md](PROFILING_QUICK_START.md)
2. [ ] Run `basic_usage` example
3. [ ] Export Chrome Trace and view in browser
4. [ ] Instrument a simple function in your code
5. [ ] Set a performance budget
6. [ ] Export metrics for analysis
7. [ ] Bookmark [PROFILING_QUICK_REFERENCE.md](PROFILING_QUICK_REFERENCE.md)

---

**Last Updated:** 2026-02-01
**Status:** Complete
**Phase:** 0.5 - Profiling Infrastructure
