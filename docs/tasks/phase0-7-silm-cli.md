# Phase 0.7: Silm CLI Tool - Code-First Game Development

**Priority:** 🔴 **CRITICAL - SHOULD BE IMPLEMENTED NOW**

**Status:** ⚪ Not Started (0%)

**Time Estimate:** 2-3 weeks

---

## Overview

The `silm` CLI is the primary interface for Silmaril game development. It enables a **code-first workflow** where games are Rust workspaces, modules are crates, and all configuration is version-controlled text files. This is the foundation that makes Silmaril AI-agent friendly.

**Philosophy:**
- Games are Rust projects (git-commitable, diff-able, mergeable)
- Modules are crates (vendorable via `--copy`, upgradeable)
- Hot-reload during development (code + assets)
- Build → production binaries
- No visual editor required (editor is Phase 0.8, optional)

---

## Goals

- ✅ Scaffold new game projects (`silm new`)
- ✅ Add/vendor modules (`silm add module`, `silm module vendor`)
- ✅ Generate components/systems (`silm add component`, `silm add system`)
- ✅ Hot-reload development workflow (`silm dev`)
- ✅ Production builds (`silm build`, `silm package`)
- ✅ Test automation (`silm test`)

---

## Task Breakdown

### **CLI.1: Project Structure (3 days)**

**Create CLI crate:**
```
engine/cli/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point (clap CLI)
│   ├── lib.rs               # Public API
│   ├── commands/            # Command implementations
│   │   ├── mod.rs
│   │   ├── new.rs           # silm new
│   │   ├── add.rs           # silm add
│   │   ├── dev.rs           # silm dev
│   │   ├── build.rs         # silm build
│   │   ├── test.rs          # silm test
│   │   └── module.rs        # silm module
│   ├── templates/           # Project templates
│   │   ├── basic/
│   │   ├── mmo/
│   │   └── moba/
│   └── codegen/             # Code generation
│       ├── component.rs
│       ├── system.rs
│       └── mod.rs
└── tests/
    └── integration_tests.rs
```

**Dependencies:**
- `clap` (v4) - CLI framework
- `toml` - Config parsing
- `serde` - Serialization
- `notify` - File watching (for hot-reload)
- `tempfile` - Testing

**Deliverables:**
- [ ] CLI crate structure
- [ ] Basic `silm --version` working
- [ ] Add to workspace members
- [ ] CI includes CLI tests

---

### **CLI.2: `silm new` - Project Scaffolding (4 days)**

**Command:**
```bash
silm new my-game [--template basic|mmo|moba]
```

**Generated structure:**
```
my-game/
├── game.toml              # Game metadata
├── Cargo.toml             # Workspace
├── xtask/                 # Build automation (cargo xtask)
├── .gitignore
├── README.md
├── assets/                # Loose files (dev)
│   ├── models/
│   ├── textures/
│   └── audio/
├── config/
│   ├── server.ron
│   └── client.ron
├── shared/                # Crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── components.rs  # Re-exports
│       └── systems.rs     # Re-exports
├── server/                # Crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── lib.rs
└── client/                # Crate
    ├── Cargo.toml
    └── src/
        ├── main.rs
        └── lib.rs
```

**game.toml format:**
```toml
[game]
name = "my-game"
version = "0.1.0"
description = "My awesome game built with Silmaril"

[dependencies]
silmaril-engine = { version = "0.1.0", path = "../engine/core" }

[modules]
# Modules added via `silm add module` appear here

[features]
client = []
server = []
networking = []
```

**Implementation tasks:**
- [ ] Template system (embed templates at compile-time)
- [ ] Cargo.toml generation (workspace + crates)
- [ ] game.toml generation
- [ ] xtask setup (cargo xtask build automation)
- [ ] README.md generation
- [ ] Stub implementations (main.rs files)
- [ ] Tests (verify generated project compiles)

**Tests:**
- [ ] Generate basic template
- [ ] Generated project compiles
- [ ] Generated tests pass
- [ ] Verify workspace structure
- [ ] Verify all files present

**Deliverables:**
- [ ] `silm new` working
- [ ] 3 templates (basic, mmo, moba)
- [ ] Generated projects compile
- [ ] Documentation in CLI help

---

### **CLI.3: `silm add component` - Code Generation (3 days)**

**Command:**
```bash
silm add component Health --shared --fields "current:f32,max:f32"
```

**Generated code:**
```rust
// shared/src/components/health.rs
use silmaril_core::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(
    Component,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Result<Self, HealthError> {
        if max <= 0.0 {
            return Err(HealthError::InvalidMaxHealth { value: max });
        }
        Ok(Self { current: max, max })
    }

    pub fn take_damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }

    pub fn heal(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0.0
    }
}

define_error! {
    pub enum HealthError {
        InvalidMaxHealth { value: f32 } = ErrorCode::InvalidValue, ErrorSeverity::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_creation() {
        let health = Health::new(100.0).unwrap();
        assert_eq!(health.current, 100.0);
        assert_eq!(health.max, 100.0);
    }

    #[test]
    fn test_invalid_max_health() {
        assert!(Health::new(-10.0).is_err());
    }

    #[test]
    fn test_take_damage() {
        let mut health = Health::new(100.0).unwrap();
        health.take_damage(30.0);
        assert_eq!(health.current, 70.0);
    }

    #[test]
    fn test_damage_clamped_to_zero() {
        let mut health = Health::new(100.0).unwrap();
        health.take_damage(150.0);
        assert_eq!(health.current, 0.0);
    }
}
```

**Updates:**
- [ ] Creates `shared/src/components/health.rs`
- [ ] Updates `shared/src/components.rs` to re-export
- [ ] Creates `shared/tests/health_tests.rs`

**Flags:**
- `--shared` - Add to shared crate (default)
- `--server` - Add to server crate
- `--client` - Add to client crate
- `--minimal` - Generate bare struct only
- `--fields "name:Type,..."` - Component fields

**Implementation tasks:**
- [ ] Parse field definitions
- [ ] Generate component struct
- [ ] Generate impl block (constructors, methods)
- [ ] Generate error types
- [ ] Generate tests
- [ ] Update module exports
- [ ] Format generated code (rustfmt)

**Tests:**
- [ ] Generate simple component
- [ ] Generate component with multiple fields
- [ ] Generated code compiles
- [ ] Generated tests pass
- [ ] Verify exports updated

**Deliverables:**
- [ ] `silm add component` working
- [ ] Full-featured generation by default
- [ ] `--minimal` flag for bare structs
- [ ] Documentation

---

### **CLI.4: `silm add system` - System Generation (2 days)**

**Command:**
```bash
silm add system health_regen --shared --query "Health,RegenerationRate"
```

**Generated code:**
```rust
// shared/src/systems/health_regen.rs
use silmaril_core::prelude::*;
use tracing::debug;
use crate::components::{Health, RegenerationRate};

pub fn health_regeneration_system(
    query: Query<(&mut Health, &RegenerationRate)>,
    dt: f32,
) {
    for (mut health, regen_rate) in query.iter_mut() {
        if health.current < health.max {
            let old_health = health.current;
            health.current = (health.current + regen_rate.0 * dt).min(health.max);

            debug!(
                old_health = old_health,
                new_health = health.current,
                regen_rate = regen_rate.0,
                "Health regenerated"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use silmaril_core::ecs::World;

    #[test]
    fn test_health_regenerates() {
        let mut world = World::new();
        let entity = world.spawn();
        world.add(entity, Health { current: 50.0, max: 100.0 });
        world.add(entity, RegenerationRate(10.0));

        health_regeneration_system(&mut world, 1.0);

        let health = world.get::<Health>(entity).unwrap();
        assert_eq!(health.current, 60.0);
    }

    #[test]
    fn test_health_clamped_to_max() {
        let mut world = World::new();
        let entity = world.spawn();
        world.add(entity, Health { current: 95.0, max: 100.0 });
        world.add(entity, RegenerationRate(10.0));

        health_regeneration_system(&mut world, 1.0);

        let health = world.get::<Health>(entity).unwrap();
        assert_eq!(health.current, 100.0);
    }
}
```

**Implementation tasks:**
- [ ] Parse query parameters
- [ ] Generate system function signature
- [ ] Generate basic iteration logic
- [ ] Generate tests
- [ ] Update module exports

**Tests:**
- [ ] Generate simple system
- [ ] Generated code compiles
- [ ] Generated tests pass

**Deliverables:**
- [ ] `silm add system` working
- [ ] Documentation

---

### **CLI.5: `silm add module` - Module Management (4 days)**

**Commands:**
```bash
# Add module as dependency
silm add module combat
# Downloads from registry, adds to Cargo.toml

# Vendor module (copy source code)
silm add module combat --copy
# Clones to modules/combat/, switches to path dependency

# Update vendored module
silm module update combat --merge
# Pulls upstream changes, merges with local changes
```

**game.toml tracking:**
```toml
[modules]
combat = { source = "vendored", upstream = "https://github.com/silmaril-modules/combat", version = "0.2.0" }
inventory = { source = "git", version = "0.1.0" }
quests = { source = "local" }  # No upstream
```

**Implementation tasks:**
- [ ] Module registry client (GitHub-based initially)
- [ ] Dependency mode (add to Cargo.toml)
- [ ] Vendor mode (clone + update Cargo.toml)
- [ ] Update logic (git fetch + merge)
- [ ] Diff preview (show upstream changes)
- [ ] game.toml tracking

**Tests:**
- [ ] Add module as dependency
- [ ] Vendor module
- [ ] Update vendored module
- [ ] Detect conflicts

**Deliverables:**
- [ ] `silm add module` working
- [ ] `silm module vendor` working
- [ ] `silm module update` working
- [ ] Documentation

---

### **CLI.6: `silm dev` - Hot-Reload Development (5 days)**

**Command:**
```bash
silm dev
```

**Behavior:**
1. Watches for file changes (code + assets)
2. Rebuilds changed crates (shared/server/client)
3. Sends reload signals to running game
4. Shows build errors in terminal
5. Logs server/client output

**Architecture:**

```
┌─────────────────────────────────────────────────┐
│  silm dev (Orchestrator)                        │
│  ┌───────────────────────────────────────────┐  │
│  │ File Watcher (notify)                     │  │
│  │  - Watch: shared/, server/, client/       │  │
│  │  - Watch: assets/                         │  │
│  └───────────────────────────────────────────┘  │
│                                                  │
│  ┌───────────────────────────────────────────┐  │
│  │ Build Manager                             │  │
│  │  - cargo build (incremental)              │  │
│  │  - Detect changed crate                   │  │
│  └───────────────────────────────────────────┘  │
│                                                  │
│  ┌───────────────────────────────────────────┐  │
│  │ Process Manager                           │  │
│  │  - Run server (with hot-reload listener)  │  │
│  │  - Run client (with hot-reload listener)  │  │
│  │  - Send reload signals via TCP            │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

**Hot-Reload Protocol (TCP):**
```rust
// CLI → Game
#[derive(Serialize, Deserialize)]
enum ReloadSignal {
    Asset { path: String },
    Code { changed_crate: String },
    Config { path: String },
    RestartGame,
}

// Game listens on localhost:9999 (debug builds only)
```

**Implementation tasks:**
- [ ] File watcher (notify crate)
- [ ] Build orchestration (cargo watch)
- [ ] Process management (start/stop server/client)
- [ ] TCP communication (send reload signals)
- [ ] Log aggregation (server + client output)
- [ ] Error display (build errors, runtime errors)
- [ ] Hot-reload manager in engine (TCP listener)
- [ ] Asset reloading (reload textures, models, etc.)

**Tests:**
- [ ] Watch detects file changes
- [ ] Build triggered on change
- [ ] Reload signal sent
- [ ] Multiple changes batched

**Deliverables:**
- [ ] `silm dev` working
- [ ] Hot-reload for code changes
- [ ] Hot-reload for asset changes
- [ ] Clean error messages
- [ ] Documentation

---

### **CLI.7: `silm build` - Production Builds (3 days)**

**Commands:**
```bash
# Release builds
silm build --release

# Platform-specific
silm build --release --target x86_64-pc-windows-msvc
silm build --release --target x86_64-unknown-linux-gnu
silm build --release --target x86_64-apple-darwin

# Package for distribution
silm package --platform windows
```

**Behavior:**
1. Builds server + client in release mode
2. Packs assets (assets/ → assets.pak)
3. Optional: Embed assets in binary
4. Generates distribution archive

**Asset Packing:**
```rust
// Development: loose files
assets/
├── models/player.gltf
├── textures/player_diffuse.png
└── audio/jump.wav

// Production: packed archive
assets.pak  (12 MB, compressed)

// Optional: embedded
client.exe  (includes assets.pak at end)
```

**Implementation tasks:**
- [ ] Release build orchestration
- [ ] Asset packing (walk directory, compress, serialize)
- [ ] Asset embedding (append to binary)
- [ ] Distribution packaging (zip/tar.gz)
- [ ] Cross-compilation support
- [ ] Optimization flags (LTO, codegen-units)

**Tests:**
- [ ] Release build succeeds
- [ ] Assets packed correctly
- [ ] Game runs with packed assets
- [ ] Cross-compilation works

**Deliverables:**
- [ ] `silm build --release` working
- [ ] `silm package` working
- [ ] Asset packing working
- [ ] Documentation

---

### **CLI.8: `silm test` - Test Automation (2 days)**

**Commands:**
```bash
# Run all tests
silm test

# Run specific crate tests
silm test --shared
silm test --server
silm test --client

# Run determinism tests (verify client/server match)
silm test --determinism

# Run benchmarks
silm test --bench
```

**Determinism Test Example:**
```rust
// shared/tests/determinism_tests.rs
#[test]
fn test_movement_deterministic() {
    let mut world = World::new();
    let entity = world.spawn();
    world.add(entity, Transform::default());
    world.add(entity, Velocity(Vec3::new(1.0, 0.0, 0.0)));

    // Run movement system
    movement_system(&mut world, 1.0);

    // Verify result matches expected
    let transform = world.get::<Transform>(entity).unwrap();
    assert_eq!(transform.position.x, 1.0);
}
```

**Implementation tasks:**
- [ ] Test runner (cargo test wrapper)
- [ ] Filter by crate
- [ ] Determinism test suite
- [ ] Benchmark runner (criterion)
- [ ] Output formatting

**Tests:**
- [ ] Test command runs
- [ ] Filters work
- [ ] Determinism tests pass

**Deliverables:**
- [ ] `silm test` working
- [ ] Documentation

---

### **CLI.9: Integration & Documentation (3 days)**

**Tasks:**
- [ ] Install script (cargo install silm-cli)
- [ ] Shell completions (bash, zsh, fish)
- [ ] Error messages (helpful, actionable)
- [ ] Progress indicators (spinners, progress bars)
- [ ] Colors (pretty terminal output)
- [ ] Help text (comprehensive)
- [ ] Tutorial (docs/cli-tutorial.md)
- [ ] Video demo (optional)

**Error Message Example:**
```bash
$ silm add component Health --fields "current:f32"
✗ Error: Missing required field 'max' for Health component

Health components typically need both current and max fields.
Did you mean:
  silm add component Health --fields "current:f32,max:f32"

See: https://docs.silmaril.dev/components/health
```

**Deliverables:**
- [ ] Install instructions
- [ ] Shell completions
- [ ] Comprehensive help text
- [ ] Tutorial documentation
- [ ] CLI reference docs

---

## Success Criteria

- [x] CLI compiles and runs
- [ ] `silm new` generates working project
- [ ] `silm add component` generates compilable code
- [ ] `silm add system` generates compilable code
- [ ] `silm dev` provides hot-reload experience
- [ ] `silm build --release` produces optimized binaries
- [ ] `silm test` runs all tests
- [ ] Documentation complete
- [ ] CI tests CLI commands

---

## Performance Targets

- `silm new`: < 5s (project generation)
- `silm dev`: < 3s rebuild (incremental)
- `silm build --release`: < 5min (full rebuild)
- Hot-reload latency: < 500ms (file change → game updated)

---

## Dependencies

### Required Engine Features
- ✅ Phase 0 complete (docs, CI, profiling)
- ✅ Phase 1.1 complete (ECS core)
- ⚠️ Hot-reload listener in engine (new - part of CLI.6)

### External Crates
- `clap` - CLI framework
- `notify` - File watching
- `toml` - Config parsing
- `serde` / `serde_json` - Serialization
- `tokio` - Async runtime (for hot-reload)
- `tera` - Template engine (for code generation)
- `indicatif` - Progress bars
- `console` - Terminal colors

---

## Testing Strategy

### Unit Tests
- [ ] Template generation
- [ ] Code generation (components, systems)
- [ ] File watching
- [ ] Build orchestration

### Integration Tests
- [ ] Generate project → compiles
- [ ] Add component → compiles
- [ ] Add system → compiles
- [ ] Hot-reload → game updates

### End-to-End Tests
- [ ] Full workflow: new → add → dev → build
- [ ] Cross-platform (Windows, Linux, macOS)

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Hot-reload complexity | Start simple (restart process), iterate to smart reload |
| Cross-platform file watching | Use `notify` crate (battle-tested) |
| Code generation bugs | Comprehensive tests, format with rustfmt |
| Build orchestration | Use cargo directly, don't reinvent |

---

## Example Workflow

```bash
# 1. Create new game
silm new my-mmo --template mmo
cd my-mmo

# 2. Add components
silm add component Health --shared --fields "current:f32,max:f32"
silm add component Mana --shared --fields "current:f32,max:f32"

# 3. Add systems
silm add system health_regen --shared --query "Health,RegenerationRate"
silm add system mana_regen --shared --query "Mana,ManaRegenerationRate"

# 4. Start dev mode
silm dev
# Server starts on localhost:7777
# Client window opens

# 5. Edit code in VSCode
# Save → game hot-reloads automatically

# 6. Edit asset in Blender
# Save → asset reloads in running game

# 7. Test
silm test

# 8. Build for production
silm build --release
silm package --platform windows
```

---

## Deliverables

- [ ] `engine/cli/` crate implementation
- [ ] `silm` binary in PATH
- [ ] 8 working commands (new, add, dev, build, test, package, module)
- [ ] Hot-reload working (code + assets)
- [ ] Asset packing working
- [ ] Documentation complete
- [ ] Tutorial written
- [ ] CI includes CLI tests

---

**Time Estimate:** 2-3 weeks (15-20 working days)

**Priority:** 🔴 **CRITICAL** - This is foundational infrastructure that unblocks AI-agent workflows. Should be implemented BEFORE continuing Phase 1 rendering.

**Next Steps After Completion:**
- Phase 0.8: Editor Foundation (Tauri + Svelte + shadcn-svelte)
- Phase 1.6: Continue rendering pipeline with hot-reload support
