# CLAUDE.md - AI Agent Development Guide

> **Primary reference for AI agents working on agent-game-engine**
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
- **Testing** → [docs/testing-strategy.md](docs/testing-strategy.md) ⚠️ MANDATORY
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
- **Benchmarking** → [docs/benchmarking.md](docs/benchmarking.md) ⚠️ **NEW - USE `just benchmark:*` COMMANDS**
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
use agent_game_engine_core::define_error;

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
use agent_game_engine_macros::{client_only, server_only};

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

### **6. Testing - All Types Required**

Every feature MUST include:
- **Unit tests** (in same file or `tests/` module)
- **Integration tests** (in crate's `tests/` directory)
- **E2E tests** (for user-facing features)
- **Property-based tests** (for serialization, math, etc.)

```rust
// Unit test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_spawn() {
        let mut world = World::new();
        let entity = world.spawn();
        assert!(world.is_alive(entity));
    }
}

// Property-based test
#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_transform_serialization_roundtrip(
            pos in prop::array::uniform3(-1000.0f32..1000.0)
        ) {
            let transform = Transform::from_translation(pos.into());
            let bytes = transform.to_bytes();
            let decoded = Transform::from_bytes(&bytes).unwrap();
            assert_eq!(transform, decoded);
        }
    }
}
```

**See:** [docs/testing-strategy.md](docs/testing-strategy.md)

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
agent-game-engine/
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
just check                     # Format + clippy + test

# Or run individually
just fmt-check                 # Format
just clippy                    # Lints
just test                      # All tests

# Test specific features
just test:serialization        # Test serialization
just test:ecs                  # Test ECS
just test:physics              # Test physics

# Benchmarks (if changed performance-sensitive code)
just benchmark:serialization   # Benchmark serialization
just benchmark:ecs             # Benchmark ECS
just benchmark:all             # All benchmarks
```

**See:**
- [docs/development-workflow.md](docs/development-workflow.md)
- [docs/rules/justfile-commands.md](docs/rules/justfile-commands.md)

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

### **Code Review Checklist**

Before marking work as complete:
- [ ] All tests pass on all platforms
- [ ] No println!/eprintln!/dbg! (use tracing)
- [ ] Custom error types (no anyhow/Box<dyn Error>)
- [ ] Platform code abstracted (no #[cfg] in business logic)
- [ ] Documented (rustdoc with examples)
- [ ] Benchmarked (if performance-sensitive)
- [ ] Cross-platform CI passes

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
RUST_LOG=agent_game_engine=debug cargo run
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
RUST_LOG=agent_game_engine_networking=trace cargo run --bin server

# Client
RUST_LOG=agent_game_engine_networking=trace cargo run --bin client
```

---

## 📚 **Further Reading**

- [ROADMAP.md](ROADMAP.md) - Implementation timeline
- [docs/architecture.md](docs/architecture.md) - System architecture
- [docs/performance-targets.md](docs/performance-targets.md) - Performance goals
- [docs/testing-strategy.md](docs/testing-strategy.md) - Testing approach