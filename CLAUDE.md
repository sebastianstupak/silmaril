# CLAUDE.md - AI Agent Development Guide

> **Primary reference for AI agents working on silmaril**
>
> This document contains critical rules, architectural decisions, and development practices that MUST be followed when contributing code.

---

## 🎯 **Project Mission**

Build a **fully automatable game engine** optimized for AI agent workflows with:
- Complete visual feedback loops (render → analyze → iterate)
- Server-authoritative multiplayer from day one
- Data-driven everything (ECS, scenes -> state, configs)
- Cross-platform support (Windows, Linux, macOS, later WASM, Android, iOS)
- Production-ready performance and scalability

---

## 📚 **Required Reading by Context**

### **When Working On:**

- **Any Code** → [docs/rules/coding-standards.md](docs/rules/coding-standards.md) ⚠️ MANDATORY
- **Error Handling** → [docs/error-handling.md](docs/error-handling.md) ⚠️ MANDATORY
- **Platform-Specific Code** → [docs/platform-abstraction.md](docs/platform-abstraction.md) ⚠️ MANDATORY
- **Testing** → [docs/TESTING_ARCHITECTURE.md](docs/TESTING_ARCHITECTURE.md) ⚠️ MANDATORY
- **Profiling & Performance** → [docs/profiling.md](docs/profiling.md) ⚠️ MANDATORY

- **ECS System** → [docs/ecs.md](docs/ecs.md)
- **Networking** → [docs/networking.md](docs/networking.md)
- **Rendering** → [docs/rendering.md](docs/rendering.md)
- **Physics** → [docs/physics.md](docs/physics.md)
- **Audio** → [docs/audio.md](docs/audio.md)
- **LOD System** → [docs/lod.md](docs/lod.md)
- **Interest Management** → [docs/interest-management.md](docs/interest-management.md)

- **Profiling & Observability** → [docs/profiling.md](docs/profiling.md)
- **Performance** → [docs/performance-targets.md](docs/performance-targets.md)
- **Benchmarking** → [docs/benchmarking.md](docs/benchmarking.md) ⚠️ **NEW - USE `cargo xtask bench` COMMANDS**
- **Architecture** → [docs/architecture.md](docs/architecture.md)
- **Dev Workflow** → [docs/development-workflow.md](docs/development-workflow.md)

---

## 🚨 **Critical Rules (MUST FOLLOW)**

### **1. No Printing - Use Structured Logging Only**

```rust
// ❌ FORBIDDEN
println!("Player joined: {}", player_id);
eprintln!("Error: {}", e);
dbg!(value);

// ✅ CORRECT
use tracing::{info, warn, error, debug};

info!(
    player_id = %player_id,
    username = %player.name,
    "Player joined"
);

error!(
    error = ?e,
    context = "player_login",
    "Login failed"
);
```

**Enforcement:** Compile-time via lints (see [docs/rules/coding-standards.md](docs/rules/coding-standards.md))

---

### **2. Custom Error Types - Always**

```rust
// ❌ FORBIDDEN
fn load_asset(path: &str) -> Result<Asset, Box<dyn Error>> { }
fn init() -> anyhow::Result<()> { }

// ✅ CORRECT
use silmaril_core::define_error;

define_error! {
    pub enum AssetError {
        NotFound { path: String } = ErrorCode::AssetNotFound, ErrorSeverity::Error,
        LoadFailed { path: String, reason: String } = ErrorCode::AssetLoadFailed, ErrorSeverity::Error,
    }
}

fn load_asset(path: &str) -> Result<Asset, AssetError> { }
```

**See:** [docs/error-handling.md](docs/error-handling.md) for complete error handling architecture

---

### **3. Platform Abstraction - Hard Enforced**

```rust
// ❌ FORBIDDEN - Platform-specific code in business logic
fn create_window() {
    #[cfg(windows)]
    { /* windows code */ }
    #[cfg(linux)]
    { /* linux code */ }
}

// ✅ CORRECT - Abstraction layer
pub trait WindowBackend {
    fn create(&self, config: WindowConfig) -> Result<Window, WindowError>;
}

#[cfg(windows)]
mod windows_backend;
#[cfg(linux)]
mod linux_backend;
#[cfg(macos)]
mod macos_backend;

// Business logic uses trait, not platform code
fn create_window(backend: &dyn WindowBackend) -> Result<Window, WindowError> {
    backend.create(WindowConfig::default())
}
```

**See:** [docs/platform-abstraction.md](docs/platform-abstraction.md) for all abstraction points

---

### **4. Client/Server Split - Use Macros**

```rust
use silmaril_macros::{client_only, server_only};

// ❌ FORBIDDEN - Manual cfg attributes scattered everywhere
#[cfg(feature = "client")]
fn render_health_bars() { }

// ✅ CORRECT - Use provided macros
#[client_only]
fn render_health_bars(world: &World, renderer: &mut Renderer) {
    // Only compiled when building client
}

#[server_only]
fn apply_damage(world: &mut World, target: Entity, damage: f32) {
    // Only compiled when building server
    // Anti-cheat logic here
}
```

**See:** [docs/networking.md](docs/networking.md) for client/server architecture

---

### **5. Profiling - Instrument Performance-Critical Code**

```rust
use engine_profiling::{profile_scope, ProfileCategory};

// ❌ FORBIDDEN - Performance-critical code without profiling
fn expensive_physics_loop() {
    for entity in entities {
        // Can't measure if this is slow!
    }
}

// ✅ CORRECT - Instrument to enable performance validation
#[profile(category = "Physics")]
fn expensive_physics_loop() {
    profile_scope!("physics_loop");
    for entity in entities {
        // Now we can measure and optimize
    }
}
```

**Key Rules:**
- Instrument ALL performance-critical paths (systems, queries, rendering)
- Use appropriate categories (ECS, Rendering, Physics, Networking, etc.)
- Don't over-instrument (not every getter/setter)
- Profiling compiles to nothing in release builds (zero cost)

**See:** [docs/profiling.md](docs/profiling.md) for complete profiling architecture

---

### **6. Testing - 3-Tier Hierarchy (MANDATORY)**

The engine uses a **3-tier test hierarchy** to enforce clean architecture:

#### **Tier 1: Unit Tests** (Single-Crate Only)

**Location:** `engine/{crate}/tests/` or inline `#[cfg(test)]`

**Rule:** MUST NOT import from other engine crates (except helpers like `engine-math`)

```rust
// ✅ CORRECT - Physics-only test
// File: engine/physics/tests/raycast_tests.rs
use engine_math::{Vec3, Quat};
use engine_physics::{PhysicsWorld, Collider};

#[test]
fn test_raycast_hits_ground() {
    let mut world = PhysicsWorld::new();
    // Test physics in isolation
}
```

#### **Tier 2: Cross-Crate Integration Tests** ⚠️ **ENFORCED**

**Location:** `engine/shared/tests/` ONLY

**Rule:** ANY test importing from 2+ engine crates MUST go here

```rust
// ✅ CORRECT - Cross-crate test in shared location
// File: engine/shared/tests/physics_ecs_integration.rs
use engine_core::ecs::World;           // ← Multiple crates
use engine_physics::PhysicsWorld;      // ← Must be in shared/

#[test]
fn test_physics_syncs_to_ecs() {
    // Test integration between physics and ECS
}
```

```rust
// ❌ FORBIDDEN - Cross-crate test in wrong location
// File: engine/physics/tests/bad_test.rs
use engine_core::ecs::World;           // ❌ Imports engine-core
use engine_physics::PhysicsWorld;      // ❌ This violates architecture!

#[test]
fn test_physics_syncs_to_ecs() {
    // This MUST be in engine/shared/tests/
}
```

**Enforcement:** Build will fail if dependencies are wrong. Code review MUST check test location.

#### **Tier 3: End-to-End System Tests**

**Location:** `examples/` or `scripts/e2e-tests/`

**Rule:** Test complete user workflows with actual binaries

```bash
#!/bin/bash
# File: scripts/e2e-tests/test-multiplayer-match.sh
cargo run --bin server &
cargo run --bin client &
# Verify match completed successfully
```

#### **Benchmark Organization (Same Rules)**

**Single-crate benchmarks:** `engine/{crate}/benches/`
**Cross-crate benchmarks:** `engine/shared/benches/` ⚠️ **MANDATORY**

```rust
// ❌ FORBIDDEN - Cross-crate benchmark in wrong location
// File: engine/physics/benches/bad_bench.rs
use engine_core::ecs::World;  // ❌ Must be in engine/shared/benches/
```

**See:** [docs/TESTING_ARCHITECTURE.md](docs/TESTING_ARCHITECTURE.md) for complete architecture

---

### **7. Performance - Industry Standards**

All code must meet these targets:

| Metric | Target | Critical |
|--------|--------|----------|
| Frame time (client) | < 16.67ms | < 33ms |
| Server tick | < 16ms (60 TPS) | < 33ms |
| Network latency overhead | < 5ms | < 10ms |
| Memory (client) | < 2GB | < 4GB |
| Memory (server/1000 players) | < 8GB | < 16GB |

**See:** [docs/performance-targets.md](docs/performance-targets.md)

**Validation:** Use profiling infrastructure (Phase 0) to verify all targets

---

### **8. Code Organization - Vertical Slices**

Each crate/module must be:
- **< 1000 lines** in main lib.rs
- **Single responsibility**
- **No circular dependencies**
- **Complete with tests**

```
✅ GOOD:
engine/physics/
├── src/
│   ├── lib.rs          (150 lines - public API only)
│   ├── backend.rs      (80 lines - trait)
│   ├── rapier.rs       (400 lines - implementation)
│   └── components.rs   (200 lines)
└── tests/
    └── integration.rs  (300 lines)

❌ BAD:
engine/game-logic/
└── src/
    └── lib.rs          (5000 lines - everything mixed)
```

---

### **9. NO Summary/Implementation Documents - STRICTLY FORBIDDEN** ⚠️ **CRITICAL**

**NEVER create temporary summary, implementation, status, or completion documents.**

```
❌ FORBIDDEN - DO NOT CREATE THESE FILES:
- *_COMPLETE.md
- *_SUMMARY.md
- *_IMPLEMENTATION*.md
- *_STATUS.md
- *_REPORT.md
- *_RESULTS.md
- *_PLAN.md
- *_VALIDATION.md
- *_COMPARISON.md
- PHASE_*.md (in root)
- TASK_*.md (in root)
- AAA_*.md

❌ Examples of FORBIDDEN files:
- SERIALIZATION_COMPLETE.md
- PHYSICS_IMPLEMENTATION_SUMMARY.md
- NETWORKING_VALIDATION_RESULTS.md
- PHASE_1_6_4_5_COMPLETE.md
- TASK_8_PROCEDURAL_GENERATION_COMPLETE.md
- AAA_AUTHENTICATION_AUTO_UPDATE_TLS_IMPLEMENTATION_PLAN.md
```

**Why this is forbidden:**
- These files clutter the repository
- They become stale immediately
- Information belongs in git commits, PR descriptions, or permanent docs
- They provide no long-term value

**✅ What to do instead:**

1. **For implementation progress** → Use git commits with clear messages
2. **For architecture decisions** → Update permanent docs in `docs/`
3. **For completion status** → Update checkboxes in ROADMAP.md
4. **For validation results** → Include in PR description or code comments
5. **For benchmark results** → Save to `docs/benchmarks/` with meaningful names

**Permanent documentation locations:**
```
✅ ALLOWED documentation:
docs/
├── architecture/           ← Architectural decisions
│   ├── ecs-design.md
│   └── networking-architecture.md
├── benchmarks/             ← Benchmark results (permanent)
│   ├── physics-baseline-2025-01.md
│   └── serialization-comparison.md
├── ecs.md                  ← Subsystem docs
├── networking.md
├── physics.md
└── ...

ROADMAP.md                  ← Implementation progress (checkboxes only)
```

**Enforcement:** Pre-commit hook blocks these files (see rule #10)

---

### **10. Tests vs Benchmarks - Clear Separation (MANDATORY)** ⚠️ **CRITICAL**

**Tests and benchmarks serve different purposes and MUST be placed in the correct location.**

#### **When to use tests/ (engine/{crate}/tests/)**

**Purpose:** Verify correctness, functionality, edge cases, error handling

```rust
// ✅ CORRECT - Functional test in tests/
#[test]
fn test_raycast_hits_ground() {
    let mut world = PhysicsWorld::new();
    let hit = world.raycast(origin, direction);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().entity_id, ground_id);
}
```

**Use tests/ for:**
- ✅ Correctness validation (assert values, error conditions)
- ✅ Edge case testing (empty inputs, boundary conditions)
- ✅ Integration tests (multiple components working together)
- ✅ Regression tests (prevent bugs from reappearing)
- ✅ Error handling validation

#### **When to use benches/ (engine/{crate}/benches/)**

**Purpose:** Measure performance, track regressions, validate targets

```rust
// ✅ CORRECT - Performance benchmark in benches/
fn bench_raycast_performance(c: &mut Criterion) {
    let mut world = create_complex_scene();

    c.bench_function("raycast_1000_objects", |b| {
        b.iter(|| {
            world.raycast(black_box(origin), black_box(direction))
        });
    });
}
```

**Use benches/ for:**
- ✅ Performance measurement (time, throughput, latency)
- ✅ Scalability testing (1, 100, 1000, 10000 entities)
- ✅ Regression tracking (ensure performance doesn't degrade)
- ✅ Target validation (< 1ms, > 100 fps, etc.)
- ✅ Profiling scenarios

#### **FORBIDDEN: Mixing concerns**

```rust
// ❌ FORBIDDEN - Performance test in tests/
#[test]
fn test_snapshot_performance() {
    let start = Instant::now();
    let snapshot = world.create_debug_snapshot(0);
    let elapsed = start.elapsed();

    // This belongs in benches/, not tests/
    println!("Took {:?}", elapsed);
    assert!(elapsed < Duration::from_millis(1));
}

// ❌ FORBIDDEN - Functional test in benches/
fn bench_raycast_correctness(c: &mut Criterion) {
    c.bench_function("raycast", |b| {
        b.iter(|| {
            let hit = world.raycast(origin, direction);
            // Don't test correctness in benchmarks!
            assert!(hit.is_some());
        });
    });
}
```

**Quick Decision Tree:**

```
Does it measure time/performance? → benches/
Does it assert correctness? → tests/
Does it measure AND assert? → Split into 2 files (1 test + 1 bench)
```

**Enforcement:** Code review MUST catch tests in wrong location

---

### **11. NO examples/ Directories in Engine Crates - STRICTLY FORBIDDEN** ⚠️ **CRITICAL**

**Engine crates MUST NOT have `examples/` directories. Top-level examples/ MUST be full game demos only.**

```
❌ FORBIDDEN:
engine/physics/examples/      ← NO!
engine/renderer/examples/     ← NO!
engine/networking/examples/   ← NO!
engine/interest/examples/     ← NO!
examples/ai_agent_debugger/   ← NO! (not a full game)
examples/simple_demo/         ← NO! (use test or benchmark)

✅ CORRECT:
engine/physics/tests/         ← Functional tests
engine/physics/benches/       ← Performance benchmarks
examples/singleplayer/        ← Full game demo (OK)
examples/mmorpg/              ← Full game demo (OK)
```

**Why this is forbidden:**
- Examples are not tested in CI
- Examples can go stale and break
- Examples duplicate test coverage
- No clear ownership or maintenance
- Examples blur the line between test/bench/demo

**✅ What to do instead:**

| Use Case | Replace With | Location |
|----------|-------------|----------|
| Demonstrate API usage | Documentation examples in rustdoc | `src/lib.rs` or `src/module.rs` |
| Test functionality | Integration test | `engine/{crate}/tests/` |
| Verify performance | Benchmark | `engine/{crate}/benches/` |
| E2E demonstration | Top-level example game | `examples/` (root only, full games) |
| Interactive demo | Test with `#[ignore]` flag | `engine/{crate}/tests/` |
| Show debugging workflow | Benchmark with println | `engine/{crate}/benches/` |

**Examples:**

```rust
// ❌ FORBIDDEN - engine/physics/examples/character_demo.rs
pub fn main() {
    let mut world = PhysicsWorld::new();
    // Demo code that isn't tested
}

// ✅ CORRECT - engine/physics/tests/character_controller_tests.rs
#[test]
fn test_character_controller_movement() {
    let mut world = PhysicsWorld::new();
    // Same logic, but tested in CI
    assert!(world.character_count() > 0);
}

// ✅ CORRECT - engine/physics/benches/character_benches.rs
fn bench_character_movement(b: &mut Bencher) {
    let mut world = PhysicsWorld::new();
    b.iter(|| {
        // Same logic, but performance is tracked
        world.step(0.016);
    });
}

// ✅ CORRECT - src/character_controller.rs (rustdoc example)
/// Character controller for player movement.
///
/// # Example
/// ```
/// use engine_physics::{PhysicsWorld, CharacterController};
///
/// let mut world = PhysicsWorld::new();
/// let controller = CharacterController::new();
/// world.add_character(controller);
/// ```
pub struct CharacterController { }
```

**Enforcement:** Pre-commit hook blocks `engine/*/examples/` directories

**Allowed examples locations:**
```
✅ ONLY allowed examples directory:
examples/                   ← Root level ONLY (full game demos)
├── singleplayer/
├── mmorpg/
└── moba/
```

---

## 🏗️ **Architecture Decisions**

### **Core Technology Stack**

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Language | Rust | Memory safety, performance, concurrency |
| Graphics | Vulkan (Ash) | Low-level control, cross-platform via MoltenVK |
| ECS | Custom | Optimized for agent workflows, data-driven |
| Serialization | FlatBuffers (network), Bincode (local), YAML (debug) | Zero-copy for network, speed for local, readability for debug |
| Networking | TCP + UDP | Critical data via TCP, positions via UDP |
| Logging | `tracing` | Structured, async, production-ready |
| Testing | Cargo test + proptest | Standard + property-based |
| Profiling | Puffin | Rust-native profiler with Chrome Tracing export |

**See:** [docs/architecture.md](docs/architecture.md) for complete architecture

---

### **Repository Structure**

```
silmaril/
├── CLAUDE.md                    ← YOU ARE HERE
├── ROADMAP.md                   ← Implementation plan
├── LICENSE                      ← Apache-2.0
├── README.md                    ← Public-facing docs
│
├── docs/                        ← Technical documentation
│   ├── architecture.md
│   ├── ecs.md
│   ├── networking.md
│   ├── rendering.md
│   ├── platform-abstraction.md
│   ├── error-handling.md
│   ├── testing-strategy.md
│   ├── performance-targets.md
│   ├── development-workflow.md
│   ├── rules/
│   │   └── coding-standards.md
│   └── tasks/                   ← Detailed task breakdowns
│       ├── phase1-core-ecs.md
│       ├── phase2-renderer.md
│       └── ...
│
├── engine/                      ← All engine code
│   ├── core/                    ← ECS, math, assets
│   ├── renderer/                ← Vulkan rendering
│   ├── networking/              ← Client + server
│   ├── physics/
│   ├── audio/
│   ├── lod/
│   ├── interest/
│   ├── auto-update/
│   ├── observability/
│   ├── macros/                  ← Proc macros
│   ├── binaries/
│   │   ├── client/
│   │   └── server/
│   └── dev-tools/
│       ├── hot-reload/
│       └── docker/
│
├── examples/                    ← Reference games
│   ├── singleplayer/
│   ├── mmorpg/
│   ├── turn-based/
│   └── moba/
│
└── scripts/                     ← Build/dev utilities
    ├── dev.sh
    ├── test-all-platforms.sh
    └── docker-compose.yml
```

---

## 🔧 **Development Workflow**

### **Before Writing Code**

1. **Read relevant docs** (see "Required Reading" above)
2. **Check ROADMAP.md** for current phase
3. **Read task file** in `docs/tasks/` if exists
4. **Write tests first** (TDD encouraged)

### **While Writing Code**

1. **Follow coding standards** ([docs/rules/coding-standards.md](docs/rules/coding-standards.md))
2. **Use structured logging** (never print!)
3. **Use custom error types** (never anyhow/Box<dyn Error>)
4. **Abstract platform code** (never #[cfg] in business logic)
5. **Document public APIs** (rustdoc with examples)

### **Before Committing**

```bash
# Quick checks (recommended)
cargo xtask check              # Format + clippy + test

# Or run individually
cargo xtask fmt                # Format
cargo xtask clippy             # Lints
cargo xtask test all           # All tests

# Test specific features
cargo xtask test serialization # Test serialization
cargo xtask test ecs           # Test ECS
cargo xtask test physics       # Test physics

# Benchmarks (if changed performance-sensitive code)
cargo xtask bench serialization # Benchmark serialization
cargo xtask bench ecs          # Benchmark ECS
cargo xtask bench all          # All benchmarks
```

**See:**
- [docs/development-workflow.md](docs/development-workflow.md)
- [docs/rules/xtask-commands.md](docs/rules/xtask-commands.md)

---

## 📊 **Metrics & Monitoring**

When adding instrumentation:

```rust
use tracing::{instrument, info, warn};

#[instrument(skip(world))]  // Auto-trace function
pub fn spawn_entity(world: &mut World, name: &str) -> Entity {
    let start = std::time::Instant::now();

    let entity = world.spawn_internal();

    info!(
        entity_id = ?entity,
        entity_name = name,
        duration_us = start.elapsed().as_micros(),
        "Entity spawned"
    );

    entity
}
```

**Metrics to track:**
- Frame time percentiles (p50, p95, p99)
- Network bandwidth (bytes/sec)
- Entity count
- Component query performance
- Memory allocations
- GPU memory usage

---

## 🎯 **MVP Scope (Phase 1-4)**

The engine is production-ready when it supports:

### **Core Systems**
- ✅ ECS with full query support
- ✅ Vulkan renderer (PBR, lighting, shadows)
- ✅ Client/server networking (TCP+UDP)
- ✅ Physics (Rapier integration)
- ✅ Audio (Kira integration)
- ✅ LOD (rendering + network)
- ✅ Interest management (fog of war)

### **Developer Experience**
- ✅ Hot-reload dev environment
- ✅ Comprehensive docs
- ✅ Working examples (singleplayer, multiplayer)
- ✅ E2E tests

### **Production Features**
- ✅ Auto-update system
- ✅ Structured logging
- ✅ Performance profiling
- ✅ Cross-platform CI

**See:** [ROADMAP.md](ROADMAP.md) for detailed implementation plan

---

## 🤝 **Contributing Guidelines**

### **For AI Agents**

1. **Always read CLAUDE.md first** (you're doing it!)
2. **Check current phase** in ROADMAP.md
3. **Read relevant task file** in docs/tasks/
4. **Write tests FIRST** (TDD)
5. **Run all checks** before committing
6. **Update docs** if changing public APIs
7. **NEVER create summary/implementation/status documents** ⚠️ **CRITICAL**
8. **NEVER create examples/ directories in engine crates** ⚠️ **CRITICAL**

### **Code Review Checklist**

Before marking work as complete:
- [ ] All tests pass on all platforms
- [ ] No println!/eprintln!/dbg! (use tracing)
- [ ] Custom error types (no anyhow/Box<dyn Error>)
- [ ] Platform code abstracted (no #[cfg] in business logic)
- [ ] Documented (rustdoc with examples)
- [ ] Benchmarked (if performance-sensitive)
- [ ] Cross-platform CI passes
- [ ] **NO summary/implementation/status documents created** ⚠️ **CRITICAL**
- [ ] **NO engine/*/examples/ directories** ⚠️ **CRITICAL**

---

## 📖 **Common Patterns**

### **Adding a New Component**

```rust
// 1. Define component
#[derive(Component, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

// 2. Add to ComponentData enum
pub enum ComponentData {
    Transform(Transform),
    Health(Health),  // ← Add here
    // ...
}

// 3. Add serialization
impl WorldState {
    fn add_component_from_data(&mut self, entity: Entity, data: ComponentData) {
        match data {
            ComponentData::Health(h) => self.add(entity, h),
            // ...
        }
    }
}

// 4. Add tests
#[cfg(test)]
mod tests {
    #[test]
    fn test_health_component() { }
}
```

### **Adding a New System**

```rust
// 1. Define system function
pub fn health_regeneration_system(
    query: Query<(&mut Health, &RegenerationRate)>,
    dt: f32,
) {
    for (health, regen_rate) in query.iter_mut() {
        if health.current < health.max {
            health.current = (health.current + regen_rate.0 * dt).min(health.max);
        }
    }
}

// 2. Register in app
app.add_system(health_regeneration_system);

// 3. Test
#[test]
fn test_health_regeneration() {
    let mut world = World::new();
    let entity = world.spawn();
    world.add(entity, Health { current: 50.0, max: 100.0 });
    world.add(entity, RegenerationRate(10.0));

    health_regeneration_system(&mut world, 1.0);

    let health = world.get::<Health>(entity).unwrap();
    assert_eq!(health.current, 60.0);
}
```

### **Adding Platform-Specific Code**

```rust
// 1. Define trait in platform.rs
pub trait PlatformBackend {
    fn get_time(&self) -> f64;
}

// 2. Implement per platform
#[cfg(windows)]
mod windows {
    impl PlatformBackend for WindowsPlatform {
        fn get_time(&self) -> f64 {
            // Windows-specific code
        }
    }
}

#[cfg(unix)]
mod unix {
    impl PlatformBackend for UnixPlatform {
        fn get_time(&self) -> f64 {
            // Unix-specific code
        }
    }
}

// 3. Select at compile time
pub fn create_platform() -> Box<dyn PlatformBackend> {
    #[cfg(windows)]
    return Box::new(windows::WindowsPlatform::new());

    #[cfg(unix)]
    return Box::new(unix::UnixPlatform::new());
}
```

---

## 🐛 **Debugging Tips**

### **Enable Verbose Logging**

```bash
RUST_LOG=trace cargo run
RUST_LOG=silmaril=debug cargo run
```

### **Use Tracy Profiler**

```bash
cargo build --features profiling
./target/debug/client
# Open Tracy, connect to localhost
```

### **Vulkan Validation Layers**

```bash
# Enable Vulkan validation (dev builds)
VK_LAYER_PATH=/usr/share/vulkan/explicit_layer.d cargo run
```

### **Network Debug**

```bash
# Server
RUST_LOG=silmaril_networking=trace cargo run --bin server

# Client
RUST_LOG=silmaril_networking=trace cargo run --bin client
```

---

## 📚 **Further Reading**

- [ROADMAP.md](ROADMAP.md) - Implementation timeline
- [docs/architecture.md](docs/architecture.md) - System architecture
- [docs/performance-targets.md](docs/performance-targets.md) - Performance goals
- [docs/testing-strategy.md](docs/testing-strategy.md) - Testing approach