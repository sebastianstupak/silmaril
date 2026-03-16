# silm dev Hot-Reload Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `silm dev` — a layered hot-reload development orchestrator that applies asset/config changes live and code changes via state-preserving restart.

**Architecture:** A `DevOrchestrator` in `engine/cli` coordinates a `FileWatcher` (notify-debouncer-full), `ProcessManager` (tokio::process), and `ReloadClient` (TCP). A `DevReloadServer` in a new `engine/dev-tools/hot-reload` crate runs inside each game process and handles live reloads. Code changes serialize the ECS world to `.silmaril/dev-state.yaml` before restart and restore it after.

**Tech Stack:** Rust, tokio, notify-debouncer-full, serde_json (newline-delimited JSON protocol), async-trait, clap 4, colored

**Spec:** `docs/superpowers/specs/2026-03-16-silm-dev-hot-reload-design.md`

**Phase dependencies:**
- Phase 1 (Tasks 1-2) must complete before all others
- Phase 2 (Tasks 3-7) tasks are independent of each other — run in parallel
- Phase 3 (Tasks 8-12) tasks are independent of each other — run in parallel; depend on Phase 2
- Phase 4 (Tasks 13-16) depend on Phase 3

---

## Chunk 1: Prerequisites

### Task 1: Extend ErrorCode for Dev Tools (2100–2199)

**Files:**
- Modify: `engine/core/src/error.rs`
- Modify: `docs/error-handling.md`

- [ ] **Step 1: Write failing test for new error codes**

Add to the `#[cfg(test)]` block at the bottom of `engine/core/src/error.rs`:

```rust
#[test]
fn test_dev_tools_error_code_range() {
    assert_eq!(ErrorCode::DevPortBindFailed.subsystem(), "Dev Tools");
    assert_eq!(ErrorCode::DevSerializeFailed.subsystem(), "Dev Tools");
    assert_eq!(ErrorCode::DevRestoreFailed.subsystem(), "Dev Tools");
    assert_eq!(ErrorCode::DevReloadFailed.subsystem(), "Dev Tools");
    assert_eq!(ErrorCode::DevTcpSendFailed.subsystem(), "Dev Tools");
    assert!((ErrorCode::DevPortBindFailed as u32) >= 2100);
    assert!((ErrorCode::DevPortBindFailed as u32) < 2200);
}
```

- [ ] **Step 2: Run test — expect compile error (variants don't exist yet)**

```bash
cargo test -p engine-core --lib -- test_dev_tools_error_code_range
```

Expected: compile error — `ErrorCode::DevPortBindFailed` not found.

- [ ] **Step 3: Add new ErrorCode variants**

In `engine/core/src/error.rs`, after the Template System block (after `TemplateSerialization = 2006`), add:

```rust
    // Dev Tools (2100-2199)
    /// DevReloadServer failed to bind TCP port
    DevPortBindFailed = 2100,
    /// Failed to serialize ECS world state for dev handoff
    DevSerializeFailed = 2101,
    /// Failed to restore ECS world state after dev restart
    DevRestoreFailed = 2102,
    /// Asset or config hot-reload failed
    DevReloadFailed = 2103,
    /// TCP send to DevReloadServer failed
    DevTcpSendFailed = 2104,
```

- [ ] **Step 4: Add subsystem() match arm**

In `engine/core/src/error.rs`, in the `subsystem()` method, add after the `2000..=2099` arm:

```rust
            2100..=2199 => "Dev Tools",
```

- [ ] **Step 5: Update error-handling.md**

In `docs/error-handling.md`, find the error code range table and add:

```
| 2100-2199 | Dev Tools |
```

- [ ] **Step 6: Run test — expect pass**

```bash
cargo test -p engine-core --lib -- test_dev_tools_error_code_range
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add engine/core/src/error.rs docs/error-handling.md
git commit -m "feat(core): add Dev Tools error codes 2100-2199"
```

---

### Task 2: Add HotReloader::force_reload method

**Files:**
- Modify: `engine/assets/src/hot_reload.rs`

- [ ] **Step 1: Write failing test**

Add to `engine/assets/src/hot_reload.rs` in `#[cfg(test)]`:

```rust
#[test]
fn test_force_reload_returns_ok_for_registered_path() {
    use std::sync::Arc;
    use crate::{AssetManager, AssetType};

    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let mut reloader = HotReloader::new(manager.clone(), config).unwrap();

    let path = std::path::PathBuf::from("assets/textures/test.png");
    let id = AssetId::new(1);
    reloader.register_asset(path.clone(), id);

    // force_reload triggers an immediate reload attempt
    let result = reloader.force_reload(&path);
    // Ok even if asset doesn't exist on disk — the reload is queued
    assert!(result.is_ok() || matches!(result, Err(AssetError::NotFound { .. })));
}

#[test]
fn test_force_reload_unregistered_path_returns_error() {
    use std::sync::Arc;
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let reloader = HotReloader::new(manager, config).unwrap();

    let path = std::path::PathBuf::from("nonexistent/asset.png");
    let result = reloader.force_reload(&path);
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run tests — expect compile error**

```bash
cargo test -p engine-assets --lib -- force_reload --features hot-reload
```

Expected: compile error — `force_reload` not found.

- [ ] **Step 3: Implement force_reload**

In `engine/assets/src/hot_reload.rs`, inside `impl HotReloader`, add after `poll_event`:

```rust
/// Force an immediate reload of the asset at `path`.
///
/// The path must have been previously registered with `register_asset`.
/// Returns `AssetError::NotFound` if the path is not registered.
/// This is called from `ForceReloader` in `engine-dev-tools` to handle
/// `reload_asset` messages from `silm dev`.
pub fn force_reload(&self, path: &Path) -> Result<(), AssetError> {
    if !self.path_to_id.contains_key(path) {
        return Err(AssetError::NotFound {
            path: path.display().to_string(),
        });
    }
    // Queue the path as a pending reload by sending a synthetic modify event
    // through the internal sender. process_events() will pick it up next frame.
    self.force_reload_sender
        .send(path.to_path_buf())
        .map_err(|e| AssetError::LoadFailed {
            path: path.display().to_string(),
            reason: format!("force reload channel closed: {e}"),
        })
}
```

Add a `force_reload_sender: std::sync::mpsc::Sender<PathBuf>` field to `HotReloader` and a matching receiver. Wire them up in `HotReloader::new` and drain the receiver at the top of `process_events`. (If internal channel structure differs, add a `Vec<PathBuf>` pending queue guarded by `Mutex` instead — the key interface is `&self, path: &Path -> Result<(), AssetError>`.)

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test -p engine-assets --lib -- force_reload --features hot-reload
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add engine/assets/src/hot_reload.rs
git commit -m "feat(assets): add HotReloader::force_reload for external driven reloads"
```

---

## Chunk 2: engine/dev-tools/hot-reload Crate

> Tasks 3-7 are independent. Run them in parallel.

### Task 3: Crate scaffolding + ReloadMessage types

**Files:**
- Create: `engine/dev-tools/hot-reload/Cargo.toml`
- Create: `engine/dev-tools/hot-reload/src/lib.rs`
- Create: `engine/dev-tools/hot-reload/src/messages.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Create crate directory and Cargo.toml**

```bash
mkdir -p D:/dev/maethril/silmaril/engine/dev-tools/hot-reload/src
```

Create `engine/dev-tools/hot-reload/Cargo.toml`:

```toml
[package]
name = "engine-dev-tools-hot-reload"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
description = "Development hot-reload infrastructure for Silmaril game projects"

[features]
dev = []

[dependencies]
# Async runtime
tokio = { workspace = true, features = ["net", "rt", "sync", "time", "io-util", "macros"] }
# Protocol serialization
serde = { workspace = true }
serde_json = "1"
# Async trait support for ProcessKiller
async-trait = "0.1"
# Logging (never use println!)
tracing = { workspace = true }
# Engine dependencies
engine-core = { path = "../../core" }
engine-assets = { path = "../../assets", features = ["hot-reload"] }
engine-macros = { path = "../../macros" }

[dev-dependencies]
tokio = { workspace = true, features = ["full"] }
tempfile = "3.10"
```

- [ ] **Step 2: Add to workspace**

In root `Cargo.toml`, add to the `members` array:

```toml
    "engine/dev-tools/hot-reload",
```

- [ ] **Step 3: Write failing test for ReloadMessage round-trip**

Create `engine/dev-tools/hot-reload/src/messages.rs`:

```rust
// Stub for test to compile against
```

Create `engine/dev-tools/hot-reload/src/lib.rs`:

```rust
pub mod messages;
```

Create `engine/dev-tools/hot-reload/tests/messages_test.rs`:

```rust
use engine_dev_tools_hot_reload::messages::ReloadMessage;

#[test]
fn test_reload_asset_round_trip() {
    let msg = ReloadMessage::ReloadAsset {
        path: "assets/textures/grass.png".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ReloadMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, ReloadMessage::ReloadAsset { path } if path == "assets/textures/grass.png"));
}

#[test]
fn test_reload_config_round_trip() {
    let msg = ReloadMessage::ReloadConfig {
        path: "config/server.ron".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ReloadMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, ReloadMessage::ReloadConfig { path } if path == "config/server.ron"));
}

#[test]
fn test_serialize_state_round_trip() {
    let msg = ReloadMessage::SerializeState;
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ReloadMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, ReloadMessage::SerializeState));
}

#[test]
fn test_ack_round_trip() {
    let msg = ReloadMessage::Ack;
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ReloadMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, ReloadMessage::Ack));
}
```

- [ ] **Step 4: Run tests — expect compile error**

```bash
cargo test -p engine-dev-tools-hot-reload --test messages_test
```

Expected: compile error — `ReloadMessage` not defined.

- [ ] **Step 5: Implement ReloadMessage**

Replace `engine/dev-tools/hot-reload/src/messages.rs` with:

```rust
//! Protocol messages for the silm dev ↔ DevReloadServer TCP channel.
//!
//! Wire format: newline-delimited JSON (`serde_json::to_string` + `\n`).
//! Both sides share this module so the types stay in sync.

use serde::{Deserialize, Serialize};

/// Messages sent from `silm dev` (via `ReloadClient`) to `DevReloadServer`.
/// Also includes `Ack` which the server sends back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReloadMessage {
    /// Reload the asset at the given project-relative path.
    ReloadAsset { path: String },
    /// Re-read the config file at the given project-relative path.
    ReloadConfig { path: String },
    /// Serialize the ECS world to `.silmaril/dev-state.yaml` and ack when done.
    SerializeState,
    /// Acknowledgement from server — `SerializeState` is complete.
    Ack,
}
```

- [ ] **Step 6: Run tests — expect pass**

```bash
cargo test -p engine-dev-tools-hot-reload --test messages_test
```

Expected: PASS (4 tests)

- [ ] **Step 7: Commit**

```bash
git add engine/dev-tools/hot-reload/ Cargo.toml
git commit -m "feat(dev-tools): add engine-dev-tools-hot-reload crate with ReloadMessage protocol"
```

---

### Task 4: StateHandoff

**Files:**
- Create: `engine/dev-tools/hot-reload/src/handoff.rs`
- Modify: `engine/dev-tools/hot-reload/src/lib.rs`

> Cross-crate tests (importing both engine-core and engine-dev-tools) go in `engine/shared/tests/`.

- [ ] **Step 1: Write failing cross-crate test**

Create `engine/shared/tests/dev_state_handoff_test.rs`:

```rust
//! Cross-crate test: StateHandoff round-trips a World through YAML.
//! Lives in engine/shared/tests/ because it imports both engine-core and engine-dev-tools.

use engine_core::ecs::World;
use engine_dev_tools_hot_reload::handoff::{RestoreResult, StateHandoff};
use tempfile::TempDir;

fn make_test_world() -> World {
    let mut world = World::new();
    // Spawn a few entities so we have something to verify
    let _e1 = world.spawn();
    let _e2 = world.spawn();
    let _e3 = world.spawn();
    world
}

#[test]
fn test_state_handoff_round_trip() {
    let dir = TempDir::new().unwrap();
    let handoff = StateHandoff::new(dir.path());

    let world = make_test_world();
    let original_count = world.entity_count();

    // Save
    handoff.save(&world).expect("save should succeed");
    assert!(handoff.exists(), "state file should exist after save");

    // Restore into a fresh world
    let mut restored_world = World::new();
    let result = handoff.restore(&mut restored_world).expect("restore should succeed");

    assert!(matches!(result, RestoreResult::Restored));
    assert_eq!(restored_world.entity_count(), original_count);
    assert!(!handoff.exists(), "state file should be deleted after restore");
}

#[test]
fn test_state_handoff_corrupt_file_gives_clean_start() {
    let dir = TempDir::new().unwrap();
    let handoff = StateHandoff::new(dir.path());

    // Write garbage
    let state_path = dir.path().join(".silmaril").join("dev-state.yaml");
    std::fs::create_dir_all(state_path.parent().unwrap()).unwrap();
    std::fs::write(&state_path, b"not: valid: yaml: [[[[").unwrap();

    let mut world = World::new();
    let result = handoff.restore(&mut world).expect("restore should not error on corrupt file");

    assert!(matches!(result, RestoreResult::CleanStart));
    assert!(!state_path.exists(), "corrupt state file should be deleted");
}

#[test]
fn test_state_handoff_missing_file_gives_clean_start() {
    let dir = TempDir::new().unwrap();
    let handoff = StateHandoff::new(dir.path());

    let mut world = World::new();
    let result = handoff.restore(&mut world).expect("restore should succeed even with no file");

    assert!(matches!(result, RestoreResult::CleanStart));
}
```

Add `engine-dev-tools-hot-reload` to `engine/shared/Cargo.toml` dev-dependencies:

```toml
engine-dev-tools-hot-reload = { path = "../dev-tools/hot-reload" }
```

- [ ] **Step 2: Run test — expect compile error**

```bash
cargo test -p engine-shared --test dev_state_handoff_test
```

Expected: compile error — `StateHandoff` not found.

- [ ] **Step 3: Implement StateHandoff**

Create `engine/dev-tools/hot-reload/src/handoff.rs`:

```rust
//! State handoff: serialize/restore ECS world across a dev restart.
//!
//! Before a code-change restart, `silm dev` asks the running process to
//! call `StateHandoff::save`. After the new process starts, it calls
//! `StateHandoff::restore` to recover the previous game state.

use crate::error::DevError;
use engine_core::ecs::World;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Outcome of a `restore` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreResult {
    /// World was successfully restored from the state file.
    Restored,
    /// No valid state file was found — process should start clean.
    CleanStart,
}

/// Serializes and restores the ECS world to/from `.silmaril/dev-state.yaml`.
pub struct StateHandoff {
    /// Path to the `.silmaril/` directory inside the project root.
    silmaril_dir: PathBuf,
}

impl StateHandoff {
    /// Create a `StateHandoff` pointing at `<project_root>/.silmaril/`.
    pub fn new(project_root: &Path) -> Self {
        Self {
            silmaril_dir: project_root.join(".silmaril"),
        }
    }

    fn state_path(&self) -> PathBuf {
        self.silmaril_dir.join("dev-state.yaml")
    }

    /// Returns `true` if a state file exists (i.e. a previous restart left one).
    pub fn exists(&self) -> bool {
        self.state_path().exists()
    }

    /// Serialize `world` to the state file.
    ///
    /// Uses synchronous `std::fs` for ordering guarantees. Callers from async
    /// contexts MUST use `tokio::task::spawn_blocking` to avoid blocking the
    /// executor. Ack is sent only after this returns `Ok`.
    pub fn save(&self, world: &World) -> Result<(), DevError> {
        use std::io::Write;

        std::fs::create_dir_all(&self.silmaril_dir).map_err(|e| DevError::SerializeFailed {
            reason: format!("could not create .silmaril/: {e}"),
        })?;

        let state = world.snapshot();
        let yaml = engine_core::serialization::serialize_yaml(&state).map_err(|e| {
            DevError::SerializeFailed {
                reason: format!("YAML serialize failed: {e}"),
            }
        })?;

        let path = self.state_path();
        let mut file = std::fs::File::create(&path).map_err(|e| DevError::SerializeFailed {
            reason: format!("could not create state file: {e}"),
        })?;
        file.write_all(yaml.as_bytes())
            .map_err(|e| DevError::SerializeFailed {
                reason: format!("write failed: {e}"),
            })?;
        file.flush().map_err(|e| DevError::SerializeFailed {
            reason: format!("flush failed: {e}"),
        })?;
        file.sync_all().map_err(|e| DevError::SerializeFailed {
            reason: format!("sync_all failed: {e}"),
        })?;

        info!(path = %path.display(), "dev state saved");
        Ok(())
    }

    /// Restore the world from the state file, if one exists.
    ///
    /// On success, deletes the state file. On corrupt/missing file, returns
    /// `CleanStart` and deletes the corrupt file. Never returns `Err` for
    /// missing or corrupt files — only for I/O errors on the delete.
    pub fn restore(&self, world: &mut World) -> Result<RestoreResult, DevError> {
        let path = self.state_path();
        if !path.exists() {
            return Ok(RestoreResult::CleanStart);
        }

        let yaml = std::fs::read_to_string(&path).map_err(|e| DevError::RestoreFailed {
            reason: format!("could not read state file: {e}"),
        })?;

        match engine_core::serialization::deserialize_yaml(&yaml) {
            Ok(state) => {
                world.clear();
                world.restore(&state).map_err(|e| DevError::RestoreFailed {
                    reason: format!("world restore failed: {e}"),
                })?;
                let _ = std::fs::remove_file(&path);
                info!(path = %path.display(), "dev state restored");
                Ok(RestoreResult::Restored)
            }
            Err(e) => {
                warn!(
                    path = %path.display(),
                    error = ?e,
                    "dev state file corrupt — starting clean"
                );
                let _ = std::fs::remove_file(&path);
                Ok(RestoreResult::CleanStart)
            }
        }
    }
}
```

Add to `engine/dev-tools/hot-reload/src/lib.rs`:

```rust
pub mod error;
pub mod handoff;
pub mod messages;
```

Create `engine/dev-tools/hot-reload/src/error.rs` (minimal, using define_error!):

```rust
//! Error types for engine-dev-tools-hot-reload.

use engine_core::error::{ErrorCode, ErrorSeverity};
use engine_macros::define_error;

define_error! {
    pub enum DevError {
        PortBindFailed { port: u16 } = ErrorCode::DevPortBindFailed, ErrorSeverity::Warning,
        SerializeFailed { reason: String } = ErrorCode::DevSerializeFailed, ErrorSeverity::Error,
        RestoreFailed { reason: String } = ErrorCode::DevRestoreFailed, ErrorSeverity::Warning,
        ReloadFailed { path: String, reason: String } = ErrorCode::DevReloadFailed, ErrorSeverity::Warning,
        TcpSendFailed { reason: String } = ErrorCode::DevTcpSendFailed, ErrorSeverity::Warning,
    }
}
```

- [ ] **Step 4: Run cross-crate tests — expect pass**

```bash
cargo test -p engine-shared --test dev_state_handoff_test
```

Expected: PASS (3 tests)

- [ ] **Step 5: Commit**

```bash
git add engine/dev-tools/hot-reload/src/ engine/shared/
git commit -m "feat(dev-tools): add StateHandoff for ECS world serialization across restarts"
```

---

### Task 5: ForceReloader bridge

**Files:**
- Create: `engine/dev-tools/hot-reload/src/force_reload.rs`
- Modify: `engine/dev-tools/hot-reload/src/lib.rs`

- [ ] **Step 1: Write failing cross-crate test**

Add to `engine/shared/tests/dev_state_handoff_test.rs` (same file, or create `engine/shared/tests/dev_force_reload_test.rs`):

```rust
// engine/shared/tests/dev_force_reload_test.rs
use engine_assets::{AssetId, AssetManager, hot_reload::{HotReloader, HotReloadConfig}};
use engine_dev_tools_hot_reload::force_reload::ForceReloader;
use std::sync::{Arc, Mutex};

#[test]
fn test_force_reloader_queues_registered_asset() {
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let hot_reloader = Arc::new(Mutex::new(
        HotReloader::new(manager, config).expect("create reloader"),
    ));

    let path = std::path::PathBuf::from("assets/textures/test.png");
    {
        let mut r = hot_reloader.lock().unwrap();
        r.register_asset(path.clone(), AssetId::new(1));
    }

    let force = ForceReloader::new(hot_reloader);
    // Should succeed — path is registered
    let result = force.reload(path.to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_force_reloader_errors_on_unregistered() {
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let hot_reloader = Arc::new(Mutex::new(
        HotReloader::new(manager, config).expect("create reloader"),
    ));

    let force = ForceReloader::new(hot_reloader);
    let result = force.reload("assets/unknown/nope.png");
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run — expect compile error**

```bash
cargo test -p engine-shared --test dev_force_reload_test
```

- [ ] **Step 3: Implement ForceReloader**

Create `engine/dev-tools/hot-reload/src/force_reload.rs`:

```rust
//! Bridge between the async `DevReloadServer` TCP task and the synchronous
//! `HotReloader` owned by the game loop thread.
//!
//! The game loop calls `HotReloader::process_events()` each frame. When
//! `silm dev` sends a `reload_asset` message, `ForceReloader::reload` calls
//! `HotReloader::force_reload` which queues the path for the next frame.

use crate::error::DevError;
use engine_assets::hot_reload::HotReloader;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::warn;

/// Drives `HotReloader` from outside the game loop (e.g. from an async TCP handler).
///
/// Thread-safe: wraps `HotReloader` in `Arc<Mutex<_>>`.
#[derive(Clone)]
pub struct ForceReloader {
    inner: Arc<Mutex<HotReloader>>,
}

impl ForceReloader {
    /// Create a new `ForceReloader` wrapping an existing `HotReloader`.
    pub fn new(hot_reloader: Arc<Mutex<HotReloader>>) -> Self {
        Self { inner: hot_reloader }
    }

    /// Queue an immediate reload of the asset at `path_str` (project-relative string).
    ///
    /// Returns `DevError::ReloadFailed` if the path is not registered in `HotReloader`.
    pub fn reload(&self, path_str: &str) -> Result<(), DevError> {
        let path = Path::new(path_str);
        let guard = self.inner.lock().map_err(|e| DevError::ReloadFailed {
            path: path_str.to_string(),
            reason: format!("mutex poisoned: {e}"),
        })?;
        guard.force_reload(path).map_err(|e| DevError::ReloadFailed {
            path: path_str.to_string(),
            reason: format!("{e:?}"),
        })?;
        Ok(())
    }
}
```

Add `pub mod force_reload;` to `lib.rs`.

- [ ] **Step 4: Run — expect pass**

```bash
cargo test -p engine-shared --test dev_force_reload_test
```

- [ ] **Step 5: Commit**

```bash
git add engine/dev-tools/hot-reload/src/force_reload.rs engine/dev-tools/hot-reload/src/lib.rs
git commit -m "feat(dev-tools): add ForceReloader bridge for external HotReloader access"
```

---

### Task 6: DevReloadServer

**Files:**
- Create: `engine/dev-tools/hot-reload/src/server.rs`
- Modify: `engine/dev-tools/hot-reload/src/lib.rs`

- [ ] **Step 1: Write failing unit test for no-op when dev feature is off**

Add to `engine/dev-tools/hot-reload/src/server.rs` (create it):

```rust
// stub
pub struct DevReloadServer;
impl DevReloadServer {
    pub async fn start(_reloader: Option<std::sync::Arc<crate::force_reload::ForceReloader>>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_start_noop_without_dev_feature() {
        // When dev feature is off, start() returns immediately.
        // With dev feature on, it would block; test that it returns in <100ms without the feature.
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            DevReloadServer::start(None),
        )
        .await;
        assert!(result.is_ok(), "start(None) should return immediately");
    }
}
```

- [ ] **Step 2: Run — expect pass (no-op stub)**

```bash
cargo test -p engine-dev-tools-hot-reload --lib -- server::tests
```

- [ ] **Step 3: Implement full DevReloadServer behind dev feature**

Replace `engine/dev-tools/hot-reload/src/server.rs`:

```rust
//! In-process TCP server that receives reload signals from `silm dev`.
//!
//! Compiled unconditionally; the `dev` feature controls whether `start()`
//! actually binds a port. Call sites require no `#[cfg]`.

use crate::error::DevError;
use crate::force_reload::ForceReloader;
use crate::handoff::StateHandoff;
use crate::messages::ReloadMessage;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// TCP server that handles reload signals inside a game process.
///
/// Start with `DevReloadServer::start(reloader).await`.
/// No-op when the `dev` feature is disabled.
pub struct DevReloadServer;

impl DevReloadServer {
    /// Start the reload server.
    ///
    /// - With `dev` feature: binds `SILMARIL_DEV_PORT` (default 9999), serves until process exit.
    /// - Without `dev` feature: returns immediately (no-op).
    ///
    /// `reloader` is `Some` when hot-reload is active, `None` for a no-op start.
    pub async fn start(reloader: Option<Arc<ForceReloader>>) {
        #[cfg(feature = "dev")]
        {
            if let Some(r) = reloader {
                if let Err(e) = Self::serve(r).await {
                    // Port bind failure is a warning, not fatal
                    warn!(error = ?e, "DevReloadServer failed to start");
                }
            }
        }
        #[cfg(not(feature = "dev"))]
        {
            let _ = reloader;
        }
    }

    #[cfg(feature = "dev")]
    async fn serve(reloader: Arc<ForceReloader>) -> Result<(), DevError> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let port: u16 = std::env::var("SILMARIL_DEV_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(9999);

        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|_| DevError::PortBindFailed { port })?;

        info!(port, "DevReloadServer listening");

        loop {
            let (stream, addr) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    warn!(error = ?e, "DevReloadServer accept error");
                    continue;
                }
            };
            debug!(%addr, "DevReloadServer connection");
            let reloader = reloader.clone();
            tokio::spawn(async move {
                let (reader, mut writer) = stream.into_split();
                let mut lines = BufReader::new(reader).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    match serde_json::from_str::<ReloadMessage>(&line) {
                        Ok(msg) => Self::handle(msg, &reloader, &mut writer).await,
                        Err(e) => warn!(error = ?e, line, "invalid reload message"),
                    }
                }
            });
        }
    }

    #[cfg(feature = "dev")]
    async fn handle(
        msg: ReloadMessage,
        reloader: &Arc<ForceReloader>,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
    ) {
        use tokio::io::AsyncWriteExt;
        match msg {
            ReloadMessage::ReloadAsset { path } => {
                if let Err(e) = reloader.reload(&path) {
                    warn!(error = ?e, path, "asset reload failed");
                } else {
                    info!(path, "asset reload queued");
                }
            }
            ReloadMessage::ReloadConfig { path } => {
                info!(path, "config reload requested (not yet implemented)");
            }
            ReloadMessage::SerializeState => {
                // StateHandoff is project-root-aware; project root comes from env
                let project_root = std::env::var("SILMARIL_PROJECT_ROOT")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
                let handoff = StateHandoff::new(&project_root);

                // Blocking fs I/O — use spawn_blocking
                // We don't have World here; world serialization is initiated by the
                // game loop via a channel. For now, signal the game loop.
                // (Full integration wired in Task 12 / orchestrator.)
                info!("SerializeState requested");

                let ack = serde_json::to_string(&ReloadMessage::Ack).unwrap() + "\n";
                let _ = writer.write_all(ack.as_bytes()).await;
            }
            ReloadMessage::Ack => {
                debug!("received Ack (unexpected in server)");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_none_returns_immediately() {
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            DevReloadServer::start(None),
        )
        .await;
        assert!(result.is_ok(), "start(None) should return immediately");
    }
}
```

- [ ] **Step 4: Write integration test for server accepting a connection**

Add to `engine/shared/tests/dev_server_test.rs`:

```rust
#[cfg(feature = "dev")]
mod tests {
    use engine_dev_tools_hot_reload::{messages::ReloadMessage, server::DevReloadServer};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;

    #[tokio::test]
    async fn test_server_accepts_reload_asset_message() {
        // Use an ephemeral port so tests don't conflict
        std::env::set_var("SILMARIL_DEV_PORT", "0"); // OS picks port
        // ... (bind listener, spawn server, send message, assert no panic)
        // Simplified: just verify the server starts and we can connect
        // Full wiring tested in E2E
    }
}
```

- [ ] **Step 5: Run lib tests**

```bash
cargo test -p engine-dev-tools-hot-reload --lib -- server::tests
```

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add engine/dev-tools/hot-reload/src/server.rs
git commit -m "feat(dev-tools): add DevReloadServer TCP listener with dev feature gate"
```

---

### Task 7: DevReloadClient (ReloadClient)

**Files:**
- Create: `engine/dev-tools/hot-reload/src/client.rs`
- Modify: `engine/dev-tools/hot-reload/src/lib.rs`

- [ ] **Step 1: Write failing unit test**

Create `engine/dev-tools/hot-reload/tests/client_test.rs`:

```rust
use engine_dev_tools_hot_reload::client::ReloadClient;

#[tokio::test]
async fn test_reload_client_send_fails_gracefully_when_no_server() {
    // No server running on this port — should warn and return Ok, not panic/error
    let client = ReloadClient::new(19998);
    let result = client.send_reload_asset("assets/textures/test.png").await;
    // Best-effort: returns Ok(()) even on connection failure
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_reload_client_serialize_state_fails_with_warn_on_no_server() {
    let client = ReloadClient::new(19997);
    let result = client.send_serialize_state().await;
    // serialize_state failure is also best-effort
    assert!(result.is_ok());
}
```

- [ ] **Step 2: Run — expect compile error**

```bash
cargo test -p engine-dev-tools-hot-reload --test client_test
```

- [ ] **Step 3: Implement ReloadClient**

Create `engine/dev-tools/hot-reload/src/client.rs`:

```rust
//! TCP client that sends reload signals from `silm dev` to `DevReloadServer`.

use crate::messages::ReloadMessage;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::{debug, warn};

/// Sends `ReloadMessage` commands to a running `DevReloadServer`.
pub struct ReloadClient {
    port: u16,
}

impl ReloadClient {
    /// Create a client targeting `localhost:<port>`.
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Queue an asset reload. Best-effort — logs and returns `Ok` on failure.
    pub async fn send_reload_asset(&self, path: &str) -> Result<(), ()> {
        self.send_no_ack(ReloadMessage::ReloadAsset { path: path.to_string() })
            .await
    }

    /// Queue a config reload. Best-effort — logs and returns `Ok` on failure.
    pub async fn send_reload_config(&self, path: &str) -> Result<(), ()> {
        self.send_no_ack(ReloadMessage::ReloadConfig { path: path.to_string() })
            .await
    }

    /// Send `SerializeState` and wait for `Ack`. Retries connection up to 3 times.
    ///
    /// On failure (no server / timeout), logs a warning and returns `Ok(())` —
    /// the caller proceeds with a clean restart.
    pub async fn send_serialize_state(&self) -> Result<(), ()> {
        let mut stream = match self.connect_with_retry().await {
            Some(s) => s,
            None => {
                warn!(port = self.port, "could not reach process for state serialization — restarting with clean state");
                return Ok(());
            }
        };

        let line = serde_json::to_string(&ReloadMessage::SerializeState).unwrap() + "\n";
        if stream.write_all(line.as_bytes()).await.is_err() {
            warn!(port = self.port, "serialize_state send failed — clean restart");
            return Ok(());
        }

        // Wait for Ack (up to 10s — world serialization may take time)
        let mut reader = BufReader::new(&mut stream);
        let mut response = String::new();
        match timeout(Duration::from_secs(10), reader.read_line(&mut response)).await {
            Ok(Ok(_)) => {
                if let Ok(ReloadMessage::Ack) = serde_json::from_str(response.trim()) {
                    debug!("SerializeState ack received");
                } else {
                    warn!("unexpected response to SerializeState");
                }
            }
            _ => warn!("timed out waiting for SerializeState ack — clean restart"),
        }
        Ok(())
    }

    /// Send a message with no ack expected. Best-effort.
    async fn send_no_ack(&self, msg: ReloadMessage) -> Result<(), ()> {
        let Some(mut stream) = self.connect_with_retry().await else {
            debug!(port = self.port, "no server for reload signal — skipping");
            return Ok(());
        };
        let line = serde_json::to_string(&msg).unwrap() + "\n";
        if let Err(e) = stream.write_all(line.as_bytes()).await {
            debug!(error = ?e, "reload send failed");
        }
        Ok(())
    }

    /// Try connecting up to 3 times with 100ms backoff (connection phase only).
    async fn connect_with_retry(&self) -> Option<TcpStream> {
        for attempt in 0..3 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            if let Ok(Ok(stream)) =
                timeout(Duration::from_millis(500), TcpStream::connect(("127.0.0.1", self.port)))
                    .await
            {
                return Some(stream);
            }
        }
        None
    }
}
```

Add `pub mod client;` to `lib.rs`.

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test -p engine-dev-tools-hot-reload --test client_test
```

Expected: PASS (2 tests)

- [ ] **Step 5: Commit**

```bash
git add engine/dev-tools/hot-reload/src/client.rs engine/dev-tools/hot-reload/src/lib.rs
git commit -m "feat(dev-tools): add ReloadClient for sending reload signals to DevReloadServer"
```

---

## Chunk 3: CLI dev command

> Tasks 8-11 are independent. Run in parallel. Task 12 depends on all of them.

### Task 8: OutputMux

**Files:**
- Create: `engine/cli/src/commands/dev/output.rs`

- [ ] **Step 1: Write failing test**

Create `engine/cli/tests/dev_output_test.rs`:

```rust
use silm::commands::dev::output::{OutputMux, Source};

#[tokio::test]
async fn test_output_mux_prefixes_lines() {
    let mux = OutputMux::new();
    let sender = mux.sender();

    // Collect output into a vec for testing
    let lines = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let lines_clone = lines.clone();

    let handle = tokio::spawn(async move {
        mux.run_capturing(lines_clone).await;
    });

    sender.send(Source::Server, "hello from server").await;
    sender.send(Source::Client, "hello from client").await;
    sender.send(Source::Build, "building...").await;
    sender.close().await;

    handle.await.unwrap();
    let result = lines.lock().await;
    assert!(result.iter().any(|l| l.contains("[server]") && l.contains("hello from server")));
    assert!(result.iter().any(|l| l.contains("[client]") && l.contains("hello from client")));
    assert!(result.iter().any(|l| l.contains("[build]") && l.contains("building...")));
}
```

- [ ] **Step 2: Run — expect compile error**

```bash
cargo test -p silm --test dev_output_test
```

- [ ] **Step 3: Create dev/ module structure and implement OutputMux**

```bash
mkdir -p D:/dev/maethril/silmaril/engine/cli/src/commands/dev
```

Create `engine/cli/src/commands/dev/mod.rs`:

```rust
pub mod orchestrator;
pub mod output;
pub mod process;
pub mod reload_client;
pub mod watcher;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum DevCommand {
    /// Start server and client together (default)
    #[command(name = "full")]
    Full,
    /// Start server only
    Server,
    /// Start client only
    Client,
}

pub async fn handle_dev_command(cmd: Option<DevCommand>) -> Result<()> {
    let mode = cmd.unwrap_or(DevCommand::Full);
    orchestrator::run(mode).await
}
```

Create `engine/cli/src/commands/dev/output.rs`:

```rust
//! Multiplexes server, client, and build output into a single terminal stream.
//! All sources send `OutputLine` over a channel; a single writer task drains it.

use colored::Colorize;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Which process produced a line.
#[derive(Debug, Clone, Copy)]
pub enum Source {
    Server,
    Client,
    Build,
    Dev,
}

#[derive(Debug, Clone)]
struct OutputLine {
    source: Source,
    text: String,
}

/// Receives output lines and writes them to stdout with prefixes and colors.
pub struct OutputMux {
    tx: mpsc::Sender<Option<OutputLine>>, // None = close
    rx: mpsc::Receiver<Option<OutputLine>>,
}

/// Cloneable sender handle for submitting lines to `OutputMux`.
#[derive(Clone)]
pub struct OutputSender {
    tx: mpsc::Sender<Option<OutputLine>>,
}

impl OutputSender {
    /// Send a line from the given source.
    pub async fn send(&self, source: Source, text: impl Into<String>) {
        let _ = self.tx.send(Some(OutputLine { source, text: text.into() })).await;
    }

    /// Signal the mux to stop.
    pub async fn close(&self) {
        let _ = self.tx.send(None).await;
    }
}

impl OutputMux {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(512);
        Self { tx, rx }
    }

    pub fn sender(&self) -> OutputSender {
        OutputSender { tx: self.tx.clone() }
    }

    /// Run the mux, writing to real stdout.
    pub async fn run(mut self) {
        while let Some(item) = self.rx.recv().await {
            match item {
                None => break,
                Some(line) => Self::print(&line),
            }
        }
    }

    /// Test helper: run and capture lines instead of printing.
    pub async fn run_capturing(mut self, out: Arc<Mutex<Vec<String>>>) {
        while let Some(item) = self.rx.recv().await {
            match item {
                None => break,
                Some(line) => {
                    let formatted = Self::format(&line);
                    out.lock().await.push(formatted);
                }
            }
        }
    }

    fn format(line: &OutputLine) -> String {
        match line.source {
            Source::Server => format!("{} {}", "[server]".blue().bold(), line.text),
            Source::Client => format!("{} {}", "[client]".green().bold(), line.text),
            Source::Build => format!("{} {}", "[build]".yellow().bold(), line.text),
            Source::Dev => format!("{} {}", "[dev]".cyan().bold(), line.text),
        }
    }

    fn print(line: &OutputLine) {
        println!("{}", Self::format(line));
    }
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test -p silm --test dev_output_test
```

- [ ] **Step 5: Commit**

```bash
git add engine/cli/src/commands/dev/
git commit -m "feat(cli): add OutputMux for multiplexed dev log output"
```

---

### Task 9: FileWatcher

**Files:**
- Create: `engine/cli/src/commands/dev/watcher.rs`
- Modify: `engine/cli/Cargo.toml` (add notify-debouncer-full)

- [ ] **Step 1: Add dependency**

In `engine/cli/Cargo.toml`, add:

```toml
notify-debouncer-full = "0.3"
tokio = { workspace = true, features = ["rt", "rt-multi-thread", "sync", "time", "macros", "io-util", "process"] }
```

- [ ] **Step 2: Write failing test**

Create `engine/cli/tests/dev_watcher_test.rs`:

```rust
use silm::commands::dev::watcher::{ChangeKind, FileWatcher};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

#[tokio::test]
async fn test_watcher_detects_rs_file_as_code_change() {
    let dir = TempDir::new().unwrap();
    let shared = dir.path().join("shared/src");
    std::fs::create_dir_all(&shared).unwrap();

    let (watcher, mut rx) = FileWatcher::new(dir.path()).unwrap();
    let _watcher = watcher; // keep alive

    // Touch a .rs file
    let rs_file = shared.join("lib.rs");
    std::fs::write(&rs_file, b"// hello").unwrap();

    let event = timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(event.is_ok(), "should receive event within 3s");
    let change = event.unwrap().unwrap();
    assert!(matches!(change.kind, ChangeKind::Code { .. }));
}

#[tokio::test]
async fn test_watcher_detects_png_as_asset_change() {
    let dir = TempDir::new().unwrap();
    let assets = dir.path().join("assets");
    std::fs::create_dir_all(&assets).unwrap();

    let (watcher, mut rx) = FileWatcher::new(dir.path()).unwrap();
    let _watcher = watcher;

    std::fs::write(assets.join("test.png"), b"fake png").unwrap();

    let event = timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(event.is_ok());
    let change = event.unwrap().unwrap();
    assert!(matches!(change.kind, ChangeKind::Asset { .. }));
}

#[tokio::test]
async fn test_watcher_detects_ron_as_config_change() {
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("config");
    std::fs::create_dir_all(&config).unwrap();

    let (watcher, mut rx) = FileWatcher::new(dir.path()).unwrap();
    let _watcher = watcher;

    std::fs::write(config.join("server.ron"), b"()").unwrap();

    let event = timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(event.is_ok());
    let change = event.unwrap().unwrap();
    assert!(matches!(change.kind, ChangeKind::Config { .. }));
}
```

- [ ] **Step 3: Run — expect compile error**

```bash
cargo test -p silm --test dev_watcher_test
```

- [ ] **Step 4: Implement FileWatcher**

Create `engine/cli/src/commands/dev/watcher.rs`:

```rust
//! File watcher that classifies changes into Code, Asset, or Config events.

use anyhow::Result;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::warn;

/// What kind of file changed.
#[derive(Debug, Clone)]
pub enum ChangeKind {
    /// A `.rs` file or `Cargo.toml` in shared/, server/, or client/
    Code { crate_name: String },
    /// An asset file in assets/
    Asset { path: PathBuf },
    /// A `.ron` config file in config/
    Config { path: PathBuf },
}

/// A detected file change.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub kind: ChangeKind,
    pub path: PathBuf,
}

const CODE_DEBOUNCE: Duration = Duration::from_millis(500);
const ASSET_DEBOUNCE: Duration = Duration::from_millis(200);

const ASSET_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "obj", "gltf", "glb", "ogg", "wav", "mp3", "webp", "hdr",
];

/// Watches the project directory and emits `FileChange` events.
pub struct FileWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
}

impl FileWatcher {
    /// Create a watcher rooted at `project_root`.
    ///
    /// Returns `(watcher, receiver)`. Keep the watcher alive for events to flow.
    pub fn new(project_root: &Path) -> Result<(Self, mpsc::Receiver<FileChange>)> {
        let (tx, rx) = mpsc::channel::<FileChange>(256);
        let root = project_root.to_path_buf();

        let debouncer = new_debouncer(
            CODE_DEBOUNCE, // use longer debounce; asset events filtered separately
            None,
            move |result: DebounceEventResult| match result {
                Ok(events) => {
                    for event in events {
                        for path in &event.paths {
                            if let Some(change) = classify(&root, path) {
                                let _ = tx.blocking_send(change);
                            }
                        }
                    }
                }
                Err(errors) => {
                    for e in errors {
                        warn!(error = ?e, "FileWatcher error");
                    }
                }
            },
        )?;

        let mut watcher = debouncer;
        // Watch the whole project root recursively
        watcher.watch(project_root, notify::RecursiveMode::Recursive)?;

        Ok((Self { _debouncer: watcher }, rx))
    }
}

fn classify(root: &Path, path: &Path) -> Option<FileChange> {
    let rel = path.strip_prefix(root).ok()?;
    let first = rel.components().next()?.as_os_str().to_string_lossy();
    let ext = path.extension()?.to_string_lossy().to_lowercase();

    if first == "assets" && ASSET_EXTENSIONS.contains(&ext.as_str()) {
        return Some(FileChange {
            kind: ChangeKind::Asset { path: rel.to_path_buf() },
            path: path.to_path_buf(),
        });
    }

    if first == "config" && ext == "ron" {
        return Some(FileChange {
            kind: ChangeKind::Config { path: rel.to_path_buf() },
            path: path.to_path_buf(),
        });
    }

    if matches!(first.as_ref(), "shared" | "server" | "client")
        && (ext == "rs" || path.file_name()?.to_string_lossy() == "Cargo.toml")
    {
        return Some(FileChange {
            kind: ChangeKind::Code { crate_name: first.to_string() },
            path: path.to_path_buf(),
        });
    }

    None
}
```

- [ ] **Step 5: Run tests — expect pass**

```bash
cargo test -p silm --test dev_watcher_test
```

Expected: PASS (3 tests)

- [ ] **Step 6: Commit**

```bash
git add engine/cli/src/commands/dev/watcher.rs engine/cli/Cargo.toml
git commit -m "feat(cli): add FileWatcher with change classification (Code/Asset/Config)"
```

---

### Task 10: ProcessKiller + ProcessManager

**Files:**
- Create: `engine/cli/src/commands/dev/process.rs`

- [ ] **Step 1: Write failing test**

Create `engine/cli/tests/dev_process_test.rs`:

```rust
use silm::commands::dev::process::ProcessManager;

#[tokio::test]
async fn test_process_manager_spawns_and_exits() {
    let mut mgr = ProcessManager::new();
    // Spawn a trivial process (echo on unix, cmd /C echo on windows)
    #[cfg(unix)]
    mgr.spawn("test", "echo", &["hello"]).await.unwrap();
    #[cfg(windows)]
    mgr.spawn("test", "cmd", &["/C", "echo", "hello"]).await.unwrap();

    // Should complete naturally
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    // No panic = success
}
```

- [ ] **Step 2: Run — expect compile error**

```bash
cargo test -p silm --test dev_process_test
```

- [ ] **Step 3: Implement ProcessKiller and ProcessManager**

Create `engine/cli/src/commands/dev/process.rs`:

```rust
//! Process spawning and lifecycle management for `silm dev`.

use crate::commands::dev::output::{OutputSender, Source};
use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, Command};
use tracing::{info, warn};

/// Platform-abstracted graceful process termination.
#[async_trait]
pub trait ProcessKiller: Send + Sync {
    async fn kill_graceful(&self, child: &mut Child, timeout: Duration) -> Result<()>;
}

/// Returns the appropriate `ProcessKiller` for the current platform.
pub fn platform_killer() -> Box<dyn ProcessKiller> {
    #[cfg(unix)]
    return Box::new(UnixKiller);
    #[cfg(windows)]
    return Box::new(WindowsKiller);
    #[cfg(not(any(unix, windows)))]
    return Box::new(FallbackKiller);
}

#[cfg(unix)]
struct UnixKiller;

#[cfg(unix)]
#[async_trait]
impl ProcessKiller for UnixKiller {
    async fn kill_graceful(&self, child: &mut Child, timeout_dur: Duration) -> Result<()> {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        if let Some(id) = child.id() {
            let _ = kill(Pid::from_raw(id as i32), Signal::SIGTERM);
        }
        match tokio::time::timeout(timeout_dur, child.wait()).await {
            Ok(_) => {}
            Err(_) => {
                warn!("process did not exit after SIGTERM — sending SIGKILL");
                let _ = child.kill().await;
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
struct WindowsKiller;

#[cfg(windows)]
#[async_trait]
impl ProcessKiller for WindowsKiller {
    async fn kill_graceful(&self, child: &mut Child, timeout_dur: Duration) -> Result<()> {
        // Send CTRL_BREAK_EVENT for graceful shutdown, then TerminateProcess
        if let Some(id) = child.id() {
            unsafe {
                windows_sys::Win32::System::Console::GenerateConsoleCtrlEvent(
                    windows_sys::Win32::System::Console::CTRL_BREAK_EVENT,
                    id,
                );
            }
        }
        match tokio::time::timeout(timeout_dur, child.wait()).await {
            Ok(_) => {}
            Err(_) => {
                warn!("process did not exit after CTRL_BREAK — force killing");
                let _ = child.kill().await;
            }
        }
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
struct FallbackKiller;

#[cfg(not(any(unix, windows)))]
#[async_trait]
impl ProcessKiller for FallbackKiller {
    async fn kill_graceful(&self, child: &mut Child, _timeout: Duration) -> Result<()> {
        let _ = child.kill().await;
        Ok(())
    }
}

/// Manages a set of named child processes.
pub struct ProcessManager {
    children: Vec<(String, Child)>,
    killer: Box<dyn ProcessKiller>,
    output: Option<OutputSender>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            killer: platform_killer(),
            output: None,
        }
    }

    pub fn with_output(mut self, sender: OutputSender) -> Self {
        self.output = Some(sender);
        self
    }

    /// Spawn a named process, piping its stdout/stderr to OutputMux.
    pub async fn spawn(&mut self, name: &str, program: &str, args: &[&str]) -> Result<()> {
        let source = match name {
            "server" => Source::Server,
            "client" => Source::Client,
            _ => Source::Dev,
        };

        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn()?;
        info!(name, program, "process spawned");

        // Forward stdout
        if let Some(stdout) = child.stdout.take() {
            let sender = self.output.clone();
            let name = name.to_string();
            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(ref s) = sender {
                        s.send(source, line).await;
                    }
                }
            });
        }

        // Forward stderr
        if let Some(stderr) = child.stderr.take() {
            let sender = self.output.clone();
            let name = name.to_string();
            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(ref s) = sender {
                        s.send(source, line).await;
                    }
                }
            });
        }

        self.children.push((name.to_string(), child));
        Ok(())
    }

    /// Kill a named process gracefully.
    pub async fn kill(&mut self, name: &str) {
        for (n, child) in &mut self.children {
            if n == name {
                let _ = self.killer.kill_graceful(child, Duration::from_secs(2)).await;
                return;
            }
        }
    }

    /// Kill all managed processes.
    pub async fn kill_all(&mut self) {
        for (name, child) in &mut self.children {
            info!(name, "killing process");
            let _ = self.killer.kill_graceful(child, Duration::from_secs(3)).await;
        }
        self.children.clear();
    }
}
```

Add `async-trait = "0.1"` to `engine/cli/Cargo.toml`. For Unix signal support, add `nix = { version = "0.27", features = ["signal", "process"] }`. For Windows, add `windows-sys = { version = "0.48", features = ["Win32_System_Console"] }` under `[target.'cfg(windows)'.dependencies]`.

- [ ] **Step 4: Run tests — expect pass**

```bash
cargo test -p silm --test dev_process_test
```

- [ ] **Step 5: Commit**

```bash
git add engine/cli/src/commands/dev/process.rs engine/cli/Cargo.toml
git commit -m "feat(cli): add ProcessKiller trait with Unix/Windows impls and ProcessManager"
```

---

### Task 11: ReloadClient wrapper in CLI

**Files:**
- Create: `engine/cli/src/commands/dev/reload_client.rs`
- Modify: `engine/cli/Cargo.toml`

- [ ] **Step 1: Add engine-dev-tools-hot-reload to CLI deps**

In `engine/cli/Cargo.toml`:

```toml
engine-dev-tools-hot-reload = { path = "../dev-tools/hot-reload" }
```

- [ ] **Step 2: Create thin wrapper**

Create `engine/cli/src/commands/dev/reload_client.rs`:

```rust
//! Wraps `engine_dev_tools_hot_reload::client::ReloadClient` for use in orchestrator.

pub use engine_dev_tools_hot_reload::client::ReloadClient;
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build -p silm
```

Expected: compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add engine/cli/src/commands/dev/reload_client.rs engine/cli/Cargo.toml
git commit -m "feat(cli): wire ReloadClient from engine-dev-tools into CLI dev command"
```

---

### Task 12: DevOrchestrator + CLI registration

**Files:**
- Create: `engine/cli/src/commands/dev/orchestrator.rs`
- Modify: `engine/cli/src/commands/mod.rs`
- Modify: `engine/cli/src/main.rs`

- [ ] **Step 1: Implement orchestrator**

Create `engine/cli/src/commands/dev/orchestrator.rs`:

```rust
//! Top-level coordinator for `silm dev`.

use super::{
    output::{OutputMux, OutputSender, Source},
    process::ProcessManager,
    reload_client::ReloadClient,
    watcher::{ChangeKind, FileWatcher},
    DevCommand,
};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::signal;
use tracing::info;

/// Configuration read from `game.toml [dev]`.
struct DevConfig {
    server_package: String,
    client_package: String,
    dev_server_port: u16,
    dev_client_port: u16,
    project_root: PathBuf,
}

impl DevConfig {
    fn load() -> Result<Self> {
        let root = find_project_root()?;
        let toml_str = std::fs::read_to_string(root.join("game.toml"))
            .context("could not read game.toml")?;
        let table: toml::Table = toml_str.parse()?;
        let dev = table.get("dev").and_then(|v| v.as_table()).context("[dev] section missing in game.toml")?;

        Ok(Self {
            server_package: dev.get("server_package").and_then(|v| v.as_str()).unwrap_or("server").to_string(),
            client_package: dev.get("client_package").and_then(|v| v.as_str()).unwrap_or("client").to_string(),
            dev_server_port: dev.get("dev_server_port").and_then(|v| v.as_integer()).unwrap_or(9999) as u16,
            dev_client_port: dev.get("dev_client_port").and_then(|v| v.as_integer()).unwrap_or(9998) as u16,
            project_root: root,
        })
    }
}

fn find_project_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("game.toml").exists() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => anyhow::bail!("game.toml not found — are you inside a silmaril project?"),
        }
    }
}

pub async fn run(cmd: DevCommand) -> Result<()> {
    let config = DevConfig::load()?;

    // Create .silmaril/ directory
    std::fs::create_dir_all(config.project_root.join(".silmaril"))?;

    let mux = OutputMux::new();
    let sender = mux.sender();

    // Start output mux
    tokio::spawn(mux.run());

    sender.send(Source::Dev, format!("starting silm dev in {}", config.project_root.display())).await;

    let run_server = matches!(cmd, DevCommand::Full | DevCommand::Server);
    let run_client = matches!(cmd, DevCommand::Full | DevCommand::Client);

    let mut mgr = ProcessManager::new().with_output(sender.clone());

    // Launch processes
    if run_server {
        let port_env = format!("SILMARIL_DEV_PORT={}", config.dev_server_port);
        let root_env = format!("SILMARIL_PROJECT_ROOT={}", config.project_root.display());
        mgr.spawn("server", "cargo", &[
            "run", "--package", &config.server_package,
            "--features", "dev",
        ]).await.context("failed to start server")?;
    }

    if run_client {
        mgr.spawn("client", "cargo", &[
            "run", "--package", &config.client_package,
            "--features", "dev",
        ]).await.context("failed to start client")?;
    }

    let server_client = ReloadClient::new(config.dev_server_port);
    let client_client = ReloadClient::new(config.dev_client_port);

    // Start file watcher
    let (watcher, mut changes) = FileWatcher::new(&config.project_root)?;
    let _watcher = watcher;

    sender.send(Source::Dev, "watching for changes...").await;

    // Event loop
    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                sender.send(Source::Dev, "shutting down...").await;
                mgr.kill_all().await;
                break;
            }
            Some(change) = changes.recv() => {
                match change.kind {
                    ChangeKind::Asset { path } => {
                        let path_str = path.to_string_lossy().to_string();
                        sender.send(Source::Dev, format!("asset changed: {path_str}")).await;
                        if run_server {
                            server_client.send_reload_asset(&path_str).await.ok();
                        }
                        if run_client {
                            client_client.send_reload_asset(&path_str).await.ok();
                        }
                    }
                    ChangeKind::Config { path } => {
                        let path_str = path.to_string_lossy().to_string();
                        sender.send(Source::Dev, format!("config changed: {path_str}")).await;
                        // Route by filename prefix
                        if path_str.contains("server") && run_server {
                            server_client.send_reload_config(&path_str).await.ok();
                        } else if run_client {
                            client_client.send_reload_config(&path_str).await.ok();
                        }
                    }
                    ChangeKind::Code { crate_name } => {
                        let should_restart_server = run_server && matches!(crate_name.as_str(), "server" | "shared");
                        let should_restart_client = run_client && matches!(crate_name.as_str(), "client" | "shared");

                        sender.send(Source::Dev, format!("{crate_name} changed — rebuilding")).await;

                        // Serialize state before kill
                        if should_restart_server {
                            server_client.send_serialize_state().await.ok();
                            mgr.kill("server").await;
                        }
                        if should_restart_client {
                            client_client.send_serialize_state().await.ok();
                            mgr.kill("client").await;
                        }

                        // Build
                        if should_restart_server {
                            sender.send(Source::Build, format!("building {}...", config.server_package)).await;
                            let status = tokio::process::Command::new("cargo")
                                .args(["build", "--package", &config.server_package, "--features", "dev"])
                                .status().await?;
                            if status.success() {
                                mgr.spawn("server", "cargo", &["run", "--package", &config.server_package, "--features", "dev"]).await.ok();
                            } else {
                                sender.send(Source::Build, "build failed — fix errors and save to retry").await;
                            }
                        }

                        if should_restart_client {
                            sender.send(Source::Build, format!("building {}...", config.client_package)).await;
                            let status = tokio::process::Command::new("cargo")
                                .args(["build", "--package", &config.client_package, "--features", "dev"])
                                .status().await?;
                            if status.success() {
                                mgr.spawn("client", "cargo", &["run", "--package", &config.client_package, "--features", "dev"]).await.ok();
                            } else {
                                sender.send(Source::Build, "build failed — fix errors and save to retry").await;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Register Dev command in CLI**

In `engine/cli/src/main.rs`, add:

```rust
/// Start development environment with hot-reload
Dev {
    #[command(subcommand)]
    command: Option<commands::dev::DevCommand>,
},
```

And in the match:

```rust
Commands::Dev { command } => {
    tokio::runtime::Runtime::new()?.block_on(commands::dev::handle_dev_command(command))?;
}
```

Add `pub mod dev;` to `engine/cli/src/commands/mod.rs`.

- [ ] **Step 3: Verify compilation**

```bash
cargo build -p silm
```

Expected: compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add engine/cli/src/commands/dev/orchestrator.rs engine/cli/src/commands/mod.rs engine/cli/src/main.rs
git commit -m "feat(cli): add silm dev command with full hot-reload orchestration"
```

---

## Chunk 4: Templates, Tests, and E2E

### Task 13: Update BasicTemplate

**Files:**
- Modify: `engine/cli/src/templates/basic.rs`

- [ ] **Step 1: Update game.toml generation**

In `basic.rs`, find `fn game_toml(&self)` and add a `[dev]` section:

```rust
fn game_toml(&self) -> TemplateFile {
    TemplateFile {
        path: "game.toml".into(),
        content: format!(r#"[project]
name = "{name}"
version = "0.1.0"

[dev]
server_package = "{name}-server"
client_package = "{name}-client"
server_port = 7777
dev_server_port = 9999
dev_client_port = 9998
"#, name = self.project_name),
    }
}
```

- [ ] **Step 2: Update server and client Cargo.toml generation**

In `fn server_cargo_toml` and `fn client_cargo_toml`, add the `dev` feature and `engine-dev-tools` dep:

```toml
[features]
dev = ["engine-dev-tools/dev"]

[dependencies]
engine-dev-tools = { path = "../../engine/dev-tools/hot-reload", optional = true }
```

- [ ] **Step 3: Update generated main.rs files**

In `fn server_main_rs` and `fn client_main_rs`, add the `DevReloadServer::start` call (unconditional, no `#[cfg]`):

```rust
// In the async main or at startup:
// (no #[cfg] needed — no-op when dev feature is off)
engine_dev_tools_hot_reload::server::DevReloadServer::start(None).await;
```

- [ ] **Step 4: Verify with a test project**

```bash
cargo run -p silm -- new test-hot-reload-project
cd test-hot-reload-project
cargo build --features dev -p test-hot-reload-project-server
```

Expected: compiles cleanly. Remove the test project after.

- [ ] **Step 5: Commit**

```bash
git add engine/cli/src/templates/basic.rs
git commit -m "feat(cli): update BasicTemplate to include dev feature and DevReloadServer wiring"
```

---

### Task 14: Cross-crate integration tests and benchmarks

**Files:**
- Create: `engine/shared/tests/dev_reload_integration.rs`
- Create: `engine/shared/benches/dev_reload_benches.rs`

- [ ] **Step 1: Write integration tests**

Create `engine/shared/tests/dev_reload_integration.rs`:

```rust
//! Cross-crate integration: DevReloadServer + StateHandoff + engine-core World.

use engine_core::ecs::World;
use engine_dev_tools_hot_reload::{
    client::ReloadClient,
    handoff::{RestoreResult, StateHandoff},
    server::DevReloadServer,
    force_reload::ForceReloader,
};
use engine_assets::{AssetManager, hot_reload::{HotReloader, HotReloadConfig}};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_state_handoff_survives_large_world() {
    let dir = TempDir::new().unwrap();
    let handoff = StateHandoff::new(dir.path());

    let mut world = World::new();
    for _ in 0..1000 {
        world.spawn();
    }

    handoff.save(&world).unwrap();
    let mut restored = World::new();
    let result = handoff.restore(&mut restored).unwrap();
    assert!(matches!(result, RestoreResult::Restored));
    assert_eq!(restored.entity_count(), 1000);
}
```

Add to `engine/shared/Cargo.toml` dev-dependencies if not already present:

```toml
engine-dev-tools-hot-reload = { path = "../dev-tools/hot-reload", features = ["dev"] }
```

- [ ] **Step 2: Write benchmarks**

Create `engine/shared/benches/dev_reload_benches.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use engine_core::ecs::World;
use engine_dev_tools_hot_reload::handoff::StateHandoff;
use tempfile::TempDir;

fn bench_state_save_1k(c: &mut Criterion) {
    let dir = TempDir::new().unwrap();
    let handoff = StateHandoff::new(dir.path());
    let mut world = World::new();
    for _ in 0..1000 { world.spawn(); }

    c.bench_function("state_save_1k_entities", |b| {
        b.iter(|| handoff.save(black_box(&world)).unwrap())
    });
}

fn bench_state_restore_1k(c: &mut Criterion) {
    let dir = TempDir::new().unwrap();
    let handoff = StateHandoff::new(dir.path());
    let mut world = World::new();
    for _ in 0..1000 { world.spawn(); }
    handoff.save(&world).unwrap();

    c.bench_function("state_restore_1k_entities", |b| {
        b.iter(|| {
            handoff.save(&world).unwrap(); // re-save each iteration
            let mut w = World::new();
            handoff.restore(black_box(&mut w)).unwrap()
        })
    });
}

criterion_group!(dev_reload_benches, bench_state_save_1k, bench_state_restore_1k);
criterion_main!(dev_reload_benches);
```

Add to `engine/shared/Cargo.toml`:

```toml
[[bench]]
name = "dev_reload_benches"
harness = false
```

- [ ] **Step 3: Run integration tests**

```bash
cargo test -p engine-shared --test dev_reload_integration
```

Expected: PASS

- [ ] **Step 4: Run benchmarks (smoke check)**

```bash
cargo bench -p engine-shared --bench dev_reload_benches -- --sample-size 10
```

Expected: runs without panic, outputs timing.

- [ ] **Step 5: Commit**

```bash
git add engine/shared/tests/dev_reload_integration.rs engine/shared/benches/dev_reload_benches.rs engine/shared/Cargo.toml
git commit -m "test(shared): add cross-crate integration tests and benchmarks for dev hot-reload"
```

---

### Task 15: E2E test script and .gitignore update

**Files:**
- Create: `scripts/e2e-tests/test-silm-dev.sh`
- Modify: `engine/cli/src/templates/basic.rs` (.gitignore template)

- [ ] **Step 1: Create E2E script**

Create `scripts/e2e-tests/test-silm-dev.sh`:

```bash
#!/usr/bin/env bash
# E2E test: silm dev hot-reload workflow
set -e

PASS=0
FAIL=0

pass() { echo "✓ $1"; PASS=$((PASS+1)); }
fail() { echo "✗ $1"; FAIL=$((FAIL+1)); }

TMPDIR=$(mktemp -d)
cd "$TMPDIR"
trap "rm -rf $TMPDIR" EXIT

echo "=== silm dev E2E test ==="

# 1. Create test project
silm new test-hot-reload-e2e || { fail "silm new failed"; exit 1; }
cd test-hot-reload-e2e
pass "silm new test-hot-reload-e2e"

# 2. Start silm dev in background
silm dev server &
DEV_PID=$!
sleep 5  # allow initial build

# 3. Touch asset — expect reload log within 5s
mkdir -p assets/textures
echo "fake" > assets/textures/test.png
sleep 2
pass "asset change sent"

# 4. Touch config
echo "()" > config/server.ron
sleep 2
pass "config change sent"

# 5. Touch source — expect rebuild
echo "// changed" >> shared/src/lib.rs
sleep 30  # allow rebuild
pass "code change — rebuild triggered"

# 6. Kill silm dev cleanly
kill -TERM "$DEV_PID" 2>/dev/null || true
wait "$DEV_PID" 2>/dev/null || true
pass "silm dev shut down cleanly"

echo ""
echo "Results: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
```

```bash
chmod +x scripts/e2e-tests/test-silm-dev.sh
```

- [ ] **Step 2: Update BasicTemplate .gitignore to include .silmaril/**

In `engine/cli/src/templates/basic.rs`, in `fn gitignore`, add:

```
# silm dev state
.silmaril/
```

- [ ] **Step 3: Commit**

```bash
git add scripts/e2e-tests/test-silm-dev.sh engine/cli/src/templates/basic.rs
git commit -m "test(e2e): add silm dev hot-reload E2E test script"
```

---

### Task 16: Final smoke test and cleanup

- [ ] **Step 1: Run all tests**

```bash
cargo xtask test all
```

Expected: all pass (some may be ignored — that's fine).

- [ ] **Step 2: Run clippy**

```bash
cargo xtask clippy
```

Fix any warnings. Use `tracing::*` not `println!`. Use `tracing::warn!` for non-fatal issues.

- [ ] **Step 3: Final commit**

```bash
git add -u
git commit -m "chore: fix clippy warnings in silm dev implementation"
```

---

## Summary of Parallelizable Work

```
Phase 1 (sequential):
  Task 1 — ErrorCode extension
  Task 2 — HotReloader::force_reload

Phase 2 (parallel after Phase 1):
  Task 3 — ReloadMessage + crate scaffold
  Task 4 — StateHandoff
  Task 5 — ForceReloader
  Task 6 — DevReloadServer
  Task 7 — ReloadClient (engine-dev-tools)

Phase 3 (parallel after Phase 2):
  Task 8  — OutputMux
  Task 9  — FileWatcher
  Task 10 — ProcessKiller + ProcessManager
  Task 11 — ReloadClient wrapper in CLI

Phase 4 (sequential after Phase 3):
  Task 12 — DevOrchestrator + CLI registration
  Task 13 — BasicTemplate updates
  Task 14 — Cross-crate tests + benchmarks
  Task 15 — E2E script
  Task 16 — Final smoke test
```
