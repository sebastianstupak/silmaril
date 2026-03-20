# Undo/Redo + CQRS + CLI Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire a complete CQRS command system into the editor backend and `silm` CLI, giving all template mutations (entity/component CRUD) a typed command choke-point with undo/redo persisted to `.undo.json`.

**Architecture:** `CommandProcessor` (new) owns a `TemplateState` (renamed from `Scene`) and an `UndoStack`. Every mutating `TemplateCommand` goes through `execute()`, which mutates state, writes YAML, and pushes an `EditorAction`. `undo()`/`redo()` apply inverses and re-write YAML. Tauri IPC and `silm` CLI both call `CommandProcessor` directly — no logic duplication.

**Tech Stack:** Rust, Tauri 2, clap 4, serde/serde_json/serde_yaml, proptest, tempfile (tests), TypeScript (api.ts)

**Spec:** `docs/superpowers/specs/2026-03-20-undo-redo-cqrs-cli-design.md`

---

## File Map

| File | Action | Purpose |
|---|---|---|
| `engine/core/src/error.rs` | Modify | Add 3 new ErrorCode variants |
| `engine/ops/src/error.rs` | **Create** | OpsError type |
| `engine/ops/src/undo.rs` | Modify | VecDeque, Serde, field renames, new fields |
| `engine/ops/src/scene.rs` | **Delete / rename** | → `template.rs` |
| `engine/ops/src/template.rs` | **Create** | TemplateState, TemplateEntity, TemplateComponent |
| `engine/ops/src/command.rs` | **Create** | TemplateCommand enum |
| `engine/ops/src/ipc.rs` | **Create** | IpcError, ActionSummary, CommandResult, From<OpsError> |
| `engine/ops/src/processor.rs` | **Create** | CommandProcessor |
| `engine/ops/src/lib.rs` | Modify | Export new modules, remove `scene` export |
| `engine/ops/Cargo.toml` | Modify | Remove anyhow, add proptest dev-dep |
| `engine/ops/tests/undo_tests.rs` | Modify | Update field names: `name` → `type_name` |
| `engine/ops/tests/template_tests.rs` | **Create** | Replaces scene_tests.rs (delete that) |
| `engine/ops/tests/processor_tests.rs` | **Create** | CommandProcessor unit + proptest |
| `engine/core/src/error.rs` | Modify | Add TemplateEntityNotFound=2007, TemplateComponentNotFound=2008, TemplateNoTemplateOpen=2011 |
| `engine/editor/src-tauri/bridge/commands.rs` | Modify | Remove `scene_command`, add `EditorState` management |
| `engine/editor/src-tauri/bridge/template_commands.rs` | **Create** | 6 Tauri template IPC handlers |
| `engine/editor/src-tauri/lib.rs` | Modify | Register EditorState, register template commands |
| `engine/cli/src/commands/template.rs` | Modify | Add Entity/Component/Undo/Redo/History subcommands |
| `engine/editor/src/lib/api.ts` | Modify | Add TypeScript IPC wrappers for template commands |
| `engine/ops/src/project.rs` | Modify | Add `*.undo.json` to gitignore |
| `scripts/e2e-tests/test-template-undo-redo.sh` | **Create** | Tier 3 E2E test |

---

## Task 1: Add ErrorCode variants to engine-core

**Files:**
- Modify: `engine/core/src/error.rs`

- [ ] **Step 1: Find the Template System block and add three variants**

In `engine/core/src/error.rs`, after `TemplateSerialization = 2006,` add:

```rust
    /// Entity not found in template during command execution.
    TemplateEntityNotFound = 2007,

    /// Component not found on entity during command execution.
    TemplateComponentNotFound = 2008,

    /// No template is currently open in the CommandProcessor.
    TemplateNoTemplateOpen = 2011,
```

Also update the `subsystem()` match arm for `2000..=2099` (it already covers the range, no change needed).

- [ ] **Step 2: Verify it compiles**

```bash
cd /d/dev/maethril/silmaril && cargo check -p engine-core 2>&1 | tail -5
```

Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add engine/core/src/error.rs
git commit -m "feat(core): add TemplateEntityNotFound, TemplateComponentNotFound, TemplateNoTemplateOpen error codes"
```

---

## Task 2: Create OpsError + update Cargo.toml

**Files:**
- Create: `engine/ops/src/error.rs`
- Modify: `engine/ops/Cargo.toml`
- Modify: `engine/ops/src/lib.rs`

- [ ] **Step 1: Create `engine/ops/src/error.rs`**

```rust
//! Error types for the ops layer — command execution, I/O, and state errors.

use engine_core::error::{ErrorCode, ErrorSeverity};
use engine_macros::define_error;

/// Entity identifier (re-exported for use in error fields).
pub use crate::undo::EntityId;

define_error! {
    pub enum OpsError {
        EntityNotFound { id: EntityId }
            = ErrorCode::TemplateEntityNotFound, ErrorSeverity::Error,
        ComponentNotFound { entity: EntityId, type_name: String }
            = ErrorCode::TemplateComponentNotFound, ErrorSeverity::Error,
        /// Covers both read and write I/O failures on template files.
        IoFailed { path: String, reason: String }
            = ErrorCode::TemplateIo, ErrorSeverity::Error,
        /// Covers YAML/JSON serialization and deserialization failures.
        SerializeFailed { reason: String }
            = ErrorCode::TemplateSerialization, ErrorSeverity::Error,
        NoTemplateOpen
            = ErrorCode::TemplateNoTemplateOpen, ErrorSeverity::Error,
    }
}
```

- [ ] **Step 2: Add proptest dev-dependency to `engine/ops/Cargo.toml`**

In `[dev-dependencies]`:
```toml
proptest = "1"
```

- [ ] **Step 3: Export the new module in `engine/ops/src/lib.rs`**

Add after the existing `pub mod undo;` line:
```rust
pub mod error;
```

- [ ] **Step 4: Verify it compiles**

```bash
cd /d/dev/maethril/silmaril && cargo check -p engine-ops 2>&1 | tail -10
```

Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add engine/ops/src/error.rs engine/ops/Cargo.toml engine/ops/src/lib.rs
git commit -m "feat(ops): add OpsError type with 5 variants, add proptest dev-dep"
```

---

## Task 3: Migrate undo.rs — field renames, new fields, VecDeque, Serde

**Files:**
- Modify: `engine/ops/src/undo.rs`
- Modify: `engine/ops/tests/undo_tests.rs`

Read the current files first — `engine/ops/src/undo.rs` and `engine/ops/tests/undo_tests.rs`.

Key changes:
- Replace `Vec` with `VecDeque` (import `std::collections::VecDeque`)
- Add `#[derive(Serialize, Deserialize)]` to `UndoStack`, `EditorAction`, `EntitySnapshot`
- Remove `anyhow` import
- In `EditorAction::SetComponent`: rename `name: String` → `type_name: String`
- In `EditorAction::AddComponent`: rename `name: String` → `type_name: String`, add `data: serde_json::Value`
- In `EditorAction::RemoveComponent`: rename `name: String` → `type_name: String`
- In `EditorAction::CreateEntity`: add `name: Option<String>`
- In `EditorAction::RenameEntity`: change `old_name: String, new_name: String` → `old_name: Option<String>, new_name: Option<String>`
- In `EntitySnapshot`: add `name: Option<String>` field
- `UndoStack` internals: `done: VecDeque<EditorAction>`, `undone: VecDeque<EditorAction>`
- `UndoStack::push`: trim with `self.done.pop_front()` (not `remove(0)`)
- Keep `undo()` and `redo()` returning `Result<EditorAction, anyhow::Error>` — replace with a plain `Result<EditorAction, ()>` (return `Err(())` when empty). The tests check `is_err()` so this still passes.

- [ ] **Step 1: Update `engine/ops/tests/undo_tests.rs` to use new field names (tests must fail first)**

Change all occurrences of `name:` in `EditorAction::SetComponent`, `AddComponent`, `RemoveComponent` to `type_name:`. Add `data:` to `AddComponent` usages. Add `name:` to `CreateEntity` usages. Change `old_name`/`new_name` to `Option<String>` where used.

Specific changes in `undo_tests.rs`:
- Line 8: `name: "Health".into(),` → `type_name: "Health".into(),`
- Line 68: `name: "Health".into(),` → `type_name: "Health".into(),`
- Line 107: `name: "Position".into(),` → `type_name: "Position".into(),`
- Line 113: `name: "Scale".into(),` → `type_name: "Scale".into(),`
- Line 41-51 (`EntitySnapshot`): add `name: None,` field
- Line 28 (`CreateEntity { id: 42 }`): add `name: None,`
- Add `data: serde_json::json!(null),` to any `AddComponent` test usages

- [ ] **Step 2: Run tests — they should FAIL (field not found)**

```bash
cd /d/dev/maethril/silmaril && cargo test -p engine-ops --test undo_tests 2>&1 | tail -20
```

Expected: compilation errors about unknown fields

- [ ] **Step 3: Rewrite `engine/ops/src/undo.rs`**

```rust
//! Undo/redo system — command pattern with snapshot checkpoints.

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Opaque entity identifier used across the undo system.
pub type EntityId = u64;

/// Snapshot of an entity's complete state for restoration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub id: EntityId,
    pub name: Option<String>,
    pub components: Vec<(String, Value)>,
}

/// A reversible editor action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditorAction {
    SetComponent {
        entity: EntityId,
        type_name: String,
        old: Value,
        new: Value,
    },
    AddComponent {
        entity: EntityId,
        type_name: String,
        data: Value,
    },
    RemoveComponent {
        entity: EntityId,
        type_name: String,
        snapshot: Value,
    },
    CreateEntity {
        id: EntityId,
        name: Option<String>,
    },
    DeleteEntity {
        id: EntityId,
        snapshot: EntitySnapshot,
    },
    RenameEntity {
        id: EntityId,
        old_name: Option<String>,
        new_name: Option<String>,
    },
    Batch {
        label: String,
        actions: Vec<EditorAction>,
    },
}

impl EditorAction {
    pub fn description(&self) -> String {
        match self {
            EditorAction::SetComponent { entity, type_name, .. } => {
                format!("Set {type_name} on Entity {entity}")
            }
            EditorAction::AddComponent { entity, type_name, .. } => {
                format!("Add {type_name} to Entity {entity}")
            }
            EditorAction::RemoveComponent { entity, type_name, .. } => {
                format!("Remove {type_name} from Entity {entity}")
            }
            EditorAction::CreateEntity { id, name } => {
                let label = name.as_deref().unwrap_or("unnamed");
                format!("Create Entity {id} ({label})")
            }
            EditorAction::DeleteEntity { id, .. } => format!("Delete Entity {id}"),
            EditorAction::RenameEntity { id, new_name, .. } => {
                let label = new_name.as_deref().unwrap_or("unnamed");
                format!("Rename Entity {id} to {label}")
            }
            EditorAction::Batch { label, .. } => label.clone(),
        }
    }
}

/// Undo/redo stack with configurable depth.
#[derive(Debug, Serialize, Deserialize)]
pub struct UndoStack {
    done: VecDeque<EditorAction>,
    undone: VecDeque<EditorAction>,
    max_depth: usize,
}

impl UndoStack {
    pub fn new(max_depth: usize) -> Self {
        Self { done: VecDeque::new(), undone: VecDeque::new(), max_depth }
    }

    pub fn push(&mut self, action: EditorAction) {
        self.undone.clear();
        self.done.push_back(action);
        while self.done.len() > self.max_depth {
            self.done.pop_front();
        }
    }

    /// Returns the action to reverse. Err(()) when stack is empty.
    pub fn undo(&mut self) -> Result<EditorAction, ()> {
        match self.done.pop_back() {
            Some(action) => {
                self.undone.push_back(action.clone());
                Ok(action)
            }
            None => Err(()),
        }
    }

    /// Returns the action to re-apply. Err(()) when stack is empty.
    pub fn redo(&mut self) -> Result<EditorAction, ()> {
        match self.undone.pop_back() {
            Some(action) => {
                self.done.push_back(action.clone());
                Ok(action)
            }
            None => Err(()),
        }
    }

    pub fn can_undo(&self) -> bool { !self.done.is_empty() }
    pub fn can_redo(&self) -> bool { !self.undone.is_empty() }
    pub fn clear(&mut self) { self.done.clear(); self.undone.clear(); }

    pub fn undo_description(&self) -> Option<String> {
        self.done.back().map(|a| a.description())
    }
    pub fn redo_description(&self) -> Option<String> {
        self.undone.back().map(|a| a.description())
    }
}
```

- [ ] **Step 4: Run the undo tests — should PASS**

```bash
cd /d/dev/maethril/silmaril && cargo test -p engine-ops --test undo_tests 2>&1 | tail -20
```

Expected: all 8 tests pass

- [ ] **Step 5: Commit**

```bash
git add engine/ops/src/undo.rs engine/ops/tests/undo_tests.rs
git commit -m "refactor(ops): migrate UndoStack to VecDeque + Serde, rename component fields to type_name"
```

---

## Task 4: Create template.rs + template_tests.rs (rename from scene.rs)

**Files:**
- Create: `engine/ops/src/template.rs`
- Delete: `engine/ops/src/scene.rs`
- Create: `engine/ops/tests/template_tests.rs`
- Delete: `engine/ops/tests/scene_tests.rs`

Read `engine/ops/src/scene.rs` and `engine/ops/tests/scene_tests.rs` fully before starting.

- [ ] **Step 1: Write failing test in `engine/ops/tests/template_tests.rs`**

```rust
use engine_ops::template::*;
use engine_ops::undo::EntityId;
use serde_json::json;
use tempfile::tempdir;

fn sample_template() -> TemplateState {
    let mut t = TemplateState::new("test_level");
    t.add_entity(TemplateEntity {
        id: 1,
        name: Some("Player".into()),
        components: vec![TemplateComponent {
            type_name: "Transform".into(),
            data: json!({"x": 0.0, "y": 1.0, "z": 0.0}),
        }],
    });
    t
}

#[test]
fn new_template_is_empty() {
    let t = TemplateState::new("my_level");
    assert_eq!(t.name, "my_level");
    assert!(t.entities.is_empty());
}

#[test]
fn add_and_remove_entity() {
    let mut t = sample_template();
    assert_eq!(t.entities.len(), 1);
    let removed = t.remove_entity(1);
    assert!(removed.is_some());
    assert!(t.entities.is_empty());
}

#[test]
fn yaml_round_trip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("level.yaml");
    let original = sample_template();
    original.save_yaml(&path).unwrap();
    let loaded = TemplateState::load_yaml(&path).unwrap();
    assert_eq!(original, loaded);
}

#[test]
fn bincode_round_trip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("level.bin");
    let original = sample_template();
    original.save_bincode(&path).unwrap();
    let loaded = TemplateState::load_bincode(&path).unwrap();
    assert_eq!(original, loaded);
}
```

- [ ] **Step 2: Run — expect FAIL (module not found)**

```bash
cd /d/dev/maethril/silmaril && cargo test -p engine-ops --test template_tests 2>&1 | tail -10
```

Expected: error `could not find module template`

- [ ] **Step 3: Create `engine/ops/src/template.rs`**

This is a mechanical rename of `scene.rs`. Keep `json_as_string` mod, `save_yaml`/`load_yaml`/`save_bincode`/`load_bincode` methods renamed to operate on `TemplateState`:

```rust
//! Template save/load — YAML for development, Bincode for release.

use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::undo::EntityId;
use crate::error::OpsError;

mod json_as_string {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use serde_json::Value;

    pub fn serialize<S>(value: &Value, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let s = serde_json::to_string(value).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Value, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        serde_json::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateComponent {
    pub type_name: String,
    #[serde(with = "json_as_string")]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateEntity {
    pub id: EntityId,
    pub name: Option<String>,
    pub components: Vec<TemplateComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateState {
    pub name: String,
    pub entities: Vec<TemplateEntity>,
}

impl TemplateState {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), entities: Vec::new() }
    }

    pub fn save_yaml(&self, path: &Path) -> Result<(), OpsError> {
        let yaml = serde_yaml::to_string(self).map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })?;
        std::fs::write(path, yaml).map_err(|e| OpsError::IoFailed { path: path.display().to_string(), reason: e.to_string() })
    }

    pub fn load_yaml(path: &Path) -> Result<Self, OpsError> {
        let data = std::fs::read_to_string(path).map_err(|e| OpsError::IoFailed { path: path.display().to_string(), reason: e.to_string() })?;
        serde_yaml::from_str(&data).map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })
    }

    pub fn save_bincode(&self, path: &Path) -> Result<(), OpsError> {
        let bytes = bincode::serialize(self).map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })?;
        std::fs::write(path, bytes).map_err(|e| OpsError::IoFailed { path: path.display().to_string(), reason: e.to_string() })
    }

    pub fn load_bincode(path: &Path) -> Result<Self, OpsError> {
        let bytes = std::fs::read(path).map_err(|e| OpsError::IoFailed { path: path.display().to_string(), reason: e.to_string() })?;
        bincode::deserialize(&bytes).map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })
    }

    pub fn add_entity(&mut self, entity: TemplateEntity) {
        self.entities.push(entity);
    }

    pub fn remove_entity(&mut self, id: EntityId) -> Option<TemplateEntity> {
        if let Some(pos) = self.entities.iter().position(|e| e.id == id) {
            Some(self.entities.remove(pos))
        } else {
            None
        }
    }

    pub fn find_entity(&self, id: EntityId) -> Option<&TemplateEntity> {
        self.entities.iter().find(|e| e.id == id)
    }

    pub fn find_entity_mut(&mut self, id: EntityId) -> Option<&mut TemplateEntity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }
}
```

- [ ] **Step 4: Update `engine/ops/src/lib.rs`**

Replace `pub mod scene;` with `pub mod template;`. Keep `scene` temporarily if it's imported elsewhere — check with:
```bash
grep -rn "engine_ops::scene\|ops::scene\|use.*scene" /d/dev/maethril/silmaril/engine --include="*.rs" | grep -v target
```

If the editor imports `engine_ops::scene`, update those imports to `engine_ops::template` in the same commit.

- [ ] **Step 5: Run template tests — should PASS**

```bash
cd /d/dev/maethril/silmaril && cargo test -p engine-ops --test template_tests 2>&1 | tail -20
```

Expected: 4 tests pass

- [ ] **Step 6: Delete scene.rs and scene_tests.rs**

```bash
rm /d/dev/maethril/silmaril/engine/ops/src/scene.rs
rm /d/dev/maethril/silmaril/engine/ops/tests/scene_tests.rs
```

- [ ] **Step 7: Run all ops tests to confirm nothing broken**

```bash
cd /d/dev/maethril/silmaril && cargo test -p engine-ops 2>&1 | tail -20
```

Expected: all tests pass

- [ ] **Step 8: Commit**

```bash
git add engine/ops/src/ engine/ops/tests/ engine/ops/src/lib.rs
git commit -m "refactor(ops): rename Scene→TemplateState, SceneEntity→TemplateEntity, SceneComponent→TemplateComponent"
```

---

## Task 5: Create command.rs + ipc.rs

**Files:**
- Create: `engine/ops/src/command.rs`
- Create: `engine/ops/src/ipc.rs`

- [ ] **Step 1: Create `engine/ops/src/command.rs`**

```rust
//! Typed command enum — the single input to CommandProcessor::execute().

use serde::{Deserialize, Serialize};
use crate::undo::EntityId;

/// All mutations that go through the undo/redo system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateCommand {
    CreateEntity { name: Option<String> },
    DeleteEntity { id: EntityId },
    RenameEntity { id: EntityId, name: Option<String> },
    DuplicateEntity { id: EntityId },
    SetComponent { id: EntityId, type_name: String, data: serde_json::Value },
    AddComponent { id: EntityId, type_name: String, data: serde_json::Value },
    RemoveComponent { id: EntityId, type_name: String },
}
```

- [ ] **Step 2: Create `engine/ops/src/ipc.rs`**

```rust
//! IPC surface types: error envelope, command result, action summaries.

use serde::{Deserialize, Serialize};
use engine_core::error::SilmarilError;
use crate::error::OpsError;
use crate::template::TemplateState;

/// Monotonically increasing action identifier assigned by CommandProcessor.
pub type ActionId = u64;

/// Serializable error envelope for Tauri IPC responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub code: u32,
    pub message: String,
}

impl From<OpsError> for IpcError {
    fn from(e: OpsError) -> Self {
        IpcError { code: e.code() as u32, message: e.to_string() }
    }
}

/// Result of a successfully executed TemplateCommand.
#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub action_id: ActionId,
    pub new_state: TemplateState,
}

/// Human-readable summary of a single undo/redo entry — for history display.
#[derive(Debug, Serialize)]
pub struct ActionSummary {
    pub action_id: ActionId,
    pub description: String,
}
```

- [ ] **Step 3: Export new modules in `engine/ops/src/lib.rs`**

Add:
```rust
pub mod command;
pub mod ipc;
```

- [ ] **Step 4: Compile check**

```bash
cd /d/dev/maethril/silmaril && cargo check -p engine-ops 2>&1 | tail -10
```

Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add engine/ops/src/command.rs engine/ops/src/ipc.rs engine/ops/src/lib.rs
git commit -m "feat(ops): add TemplateCommand enum and IPC types (IpcError, CommandResult, ActionSummary)"
```

---

## Task 6: Create CommandProcessor (TDD)

**Files:**
- Create: `engine/ops/tests/processor_tests.rs`
- Create: `engine/ops/src/processor.rs`

This is the core of the feature. Write all tests first.

- [ ] **Step 1: Write `engine/ops/tests/processor_tests.rs`**

```rust
use engine_ops::command::TemplateCommand;
use engine_ops::processor::CommandProcessor;
use engine_ops::undo::{EditorAction, EntityId};
use serde_json::json;
use tempfile::tempdir;

fn make_processor() -> (CommandProcessor, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.yaml");
    // Create an empty template YAML on disk so load_yaml can read it
    std::fs::write(&path, "name: test\nentities: []\n").unwrap();
    let proc = CommandProcessor::load(path).unwrap();
    (proc, dir)
}

#[test]
fn create_entity_adds_to_state() {
    let (mut proc, _dir) = make_processor();
    let result = proc.execute(TemplateCommand::CreateEntity { name: Some("Player".into()) }).unwrap();
    assert_eq!(result.new_state.entities.len(), 1);
    assert_eq!(result.new_state.entities[0].name.as_deref(), Some("Player"));
}

#[test]
fn create_entity_writes_yaml() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.yaml");
    std::fs::write(&path, "name: test\nentities: []\n").unwrap();
    let mut proc = CommandProcessor::load(path.clone()).unwrap();
    proc.execute(TemplateCommand::CreateEntity { name: Some("Hero".into()) }).unwrap();
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("Hero"), "YAML should contain entity name");
}

#[test]
fn undo_create_entity_removes_it() {
    let (mut proc, _dir) = make_processor();
    proc.execute(TemplateCommand::CreateEntity { name: Some("Temp".into()) }).unwrap();
    assert_eq!(proc.state_ref().entities.len(), 1);
    proc.undo().unwrap();
    assert_eq!(proc.state_ref().entities.len(), 0);
}

#[test]
fn undo_returns_none_when_empty() {
    let (mut proc, _dir) = make_processor();
    let result = proc.undo().unwrap();
    assert!(result.is_none(), "undo on empty stack should return Ok(None)");
}

#[test]
fn redo_returns_none_when_empty() {
    let (mut proc, _dir) = make_processor();
    let result = proc.redo().unwrap();
    assert!(result.is_none());
}

#[test]
fn undo_then_redo_restores_state() {
    let (mut proc, _dir) = make_processor();
    proc.execute(TemplateCommand::CreateEntity { name: Some("A".into()) }).unwrap();
    proc.undo().unwrap();
    assert!(proc.state_ref().entities.is_empty());
    proc.redo().unwrap();
    assert_eq!(proc.state_ref().entities.len(), 1);
}

#[test]
fn delete_entity_not_found_returns_error() {
    let (mut proc, _dir) = make_processor();
    let result = proc.execute(TemplateCommand::DeleteEntity { id: 999 });
    assert!(result.is_err());
}

#[test]
fn delete_entity_undo_restores_name_and_components() {
    let (mut proc, _dir) = make_processor();
    let create_result = proc.execute(TemplateCommand::CreateEntity { name: Some("Boss".into()) }).unwrap();
    let entity_id = create_result.new_state.entities[0].id;
    proc.execute(TemplateCommand::AddComponent {
        id: entity_id,
        type_name: "Health".into(),
        data: json!({"current": 100}),
    }).unwrap();
    proc.execute(TemplateCommand::DeleteEntity { id: entity_id }).unwrap();
    assert!(proc.state_ref().entities.is_empty());
    proc.undo().unwrap();  // undo delete
    let entity = proc.state_ref().find_entity(entity_id).unwrap();
    assert_eq!(entity.name.as_deref(), Some("Boss"));
    assert_eq!(entity.components.len(), 1);
}

#[test]
fn duplicate_entity_creates_copy() {
    let (mut proc, _dir) = make_processor();
    let result = proc.execute(TemplateCommand::CreateEntity { name: Some("Orig".into()) }).unwrap();
    let orig_id = result.new_state.entities[0].id;
    proc.execute(TemplateCommand::AddComponent {
        id: orig_id,
        type_name: "Health".into(),
        data: json!({"current": 50}),
    }).unwrap();
    proc.execute(TemplateCommand::DuplicateEntity { id: orig_id }).unwrap();
    assert_eq!(proc.state_ref().entities.len(), 2);
    let copy = &proc.state_ref().entities[1];
    assert_eq!(copy.components.len(), 1);
    assert_eq!(copy.components[0].type_name, "Health");
}

#[test]
fn duplicate_entity_undo_removes_copy_in_one_step() {
    let (mut proc, _dir) = make_processor();
    let result = proc.execute(TemplateCommand::CreateEntity { name: None }).unwrap();
    let id = result.new_state.entities[0].id;
    proc.execute(TemplateCommand::DuplicateEntity { id }).unwrap();
    assert_eq!(proc.state_ref().entities.len(), 2);
    proc.undo().unwrap();  // single undo removes the whole duplicate
    assert_eq!(proc.state_ref().entities.len(), 1);
}

#[test]
fn undo_history_persisted_and_loaded() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("level.yaml");
    std::fs::write(&path, "name: level\nentities: []\n").unwrap();
    {
        let mut proc = CommandProcessor::load(path.clone()).unwrap();
        proc.execute(TemplateCommand::CreateEntity { name: Some("Saved".into()) }).unwrap();
    } // proc dropped — .undo.json written
    let undo_path = dir.path().join("level.undo.json");
    assert!(undo_path.exists(), ".undo.json should be written");
    // Reload and undo should work
    let mut proc2 = CommandProcessor::load(path).unwrap();
    assert_eq!(proc2.state_ref().entities.len(), 1);
    proc2.undo().unwrap();
    assert_eq!(proc2.state_ref().entities.len(), 0);
}

#[test]
fn history_summaries_returns_descriptions() {
    let (mut proc, _dir) = make_processor();
    proc.execute(TemplateCommand::CreateEntity { name: Some("X".into()) }).unwrap();
    let summaries = proc.history_summaries();
    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].description.contains("Create Entity"));
}
```

- [ ] **Step 2: Run — expect FAIL (module not found)**

```bash
cd /d/dev/maethril/silmaril && cargo test -p engine-ops --test processor_tests 2>&1 | tail -10
```

Expected: `could not find module processor`

- [ ] **Step 3: Create `engine/ops/src/processor.rs`**

```rust
//! CommandProcessor — single choke-point for all template mutations.
//!
//! Owns a TemplateState + UndoStack. Every TemplateCommand flows through
//! execute(), which mutates in-memory state, writes YAML to disk, pushes
//! an EditorAction, and saves the undo history to <template>.undo.json.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use crate::command::TemplateCommand;
use crate::error::OpsError;
use crate::ipc::{ActionId, ActionSummary, CommandResult};
use crate::template::{TemplateComponent, TemplateEntity, TemplateState};
use crate::undo::{EditorAction, EntityId, EntitySnapshot, UndoStack};

pub struct CommandProcessor {
    pub(crate) state: TemplateState,
    undo_stack: UndoStack,
    // Parallel vec tracking ActionId for each entry in undo_stack.done
    done_ids: VecDeque<ActionId>,
    template_path: PathBuf,
    next_action_id: ActionId,
}

impl CommandProcessor {
    /// Load a template YAML from disk. Loads .undo.json if present.
    pub fn load(path: PathBuf) -> Result<Self, OpsError> {
        let state = TemplateState::load_yaml(&path)?;
        let undo_path = undo_path_for(&path);
        let (undo_stack, done_ids) = if undo_path.exists() {
            load_undo_history(&undo_path)?
        } else {
            (UndoStack::new(100), VecDeque::new())
        };
        let next_action_id = done_ids.back().copied().map(|id| id + 1).unwrap_or(0);
        Ok(Self { state, undo_stack, done_ids, template_path: path, next_action_id })
    }

    /// Execute a command. Mutates state, writes YAML, pushes undo action.
    pub fn execute(&mut self, cmd: TemplateCommand) -> Result<CommandResult, OpsError> {
        let action_id = self.next_action_id;
        self.next_action_id += 1;

        let action = self.apply_command(cmd)?;
        // New command: clear done_ids redo mirror (UndoStack already cleared undone)
        // But done_ids mirrors done — we push after clearing undone
        self.done_ids.push_back(action_id);
        // Trim done_ids to match UndoStack depth
        while self.done_ids.len() > 100 {
            self.done_ids.pop_front();
        }
        self.undo_stack.push(action);

        self.write_state()?;
        Ok(CommandResult { action_id, new_state: self.state.clone() })
    }

    /// Undo last action. Returns Ok(None) when nothing to undo.
    pub fn undo(&mut self) -> Result<Option<ActionId>, OpsError> {
        match self.undo_stack.undo() {
            Ok(action) => {
                let action_id = self.done_ids.pop_back();
                self.apply_inverse(&action)?;
                self.write_state()?;
                Ok(action_id)
            }
            Err(()) => Ok(None),
        }
    }

    /// Redo last undone action. Returns Ok(None) when nothing to redo.
    pub fn redo(&mut self) -> Result<Option<ActionId>, OpsError> {
        match self.undo_stack.redo() {
            Ok(action) => {
                let action_id = self.next_action_id;
                self.next_action_id += 1;
                self.done_ids.push_back(action_id);
                self.apply_action(&action)?;
                self.write_state()?;
                Ok(Some(action_id))
            }
            Err(()) => Ok(None),
        }
    }

    pub fn history_summaries(&self) -> Vec<ActionSummary> {
        // done_ids and undo_stack.done are parallel — zip them
        // UndoStack doesn't expose done directly; use description helpers
        // Instead: rebuild from undo_descriptions — simplified: return can_undo info only
        // Full impl: expose iter on UndoStack
        self.undo_stack.history_summaries(&self.done_ids)
    }

    pub fn can_undo(&self) -> bool { self.undo_stack.can_undo() }
    pub fn can_redo(&self) -> bool { self.undo_stack.can_redo() }

    /// Read-only access to current state (for tests).
    pub fn state_ref(&self) -> &TemplateState { &self.state }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn apply_command(&mut self, cmd: TemplateCommand) -> Result<EditorAction, OpsError> {
        match cmd {
            TemplateCommand::CreateEntity { name } => {
                let id = self.next_entity_id();
                self.state.add_entity(TemplateEntity { id, name: name.clone(), components: vec![] });
                Ok(EditorAction::CreateEntity { id, name })
            }
            TemplateCommand::DeleteEntity { id } => {
                let entity = self.state.find_entity(id).ok_or(OpsError::EntityNotFound { id })?.clone();
                let snapshot = EntitySnapshot {
                    id: entity.id,
                    name: entity.name.clone(),
                    components: entity.components.iter().map(|c| (c.type_name.clone(), c.data.clone())).collect(),
                };
                self.state.remove_entity(id);
                Ok(EditorAction::DeleteEntity { id, snapshot })
            }
            TemplateCommand::RenameEntity { id, name: new_name } => {
                let entity = self.state.find_entity_mut(id).ok_or(OpsError::EntityNotFound { id })?;
                let old_name = entity.name.clone();
                entity.name = new_name.clone();
                Ok(EditorAction::RenameEntity { id, old_name, new_name })
            }
            TemplateCommand::DuplicateEntity { id } => {
                let original = self.state.find_entity(id).ok_or(OpsError::EntityNotFound { id })?.clone();
                let new_id = self.next_entity_id();
                let copy = TemplateEntity {
                    id: new_id,
                    name: original.name.as_deref().map(|n| format!("{n} (copy)")),
                    components: original.components.clone(),
                };
                let sub_actions: Vec<EditorAction> = std::iter::once(EditorAction::CreateEntity {
                    id: new_id,
                    name: copy.name.clone(),
                })
                .chain(copy.components.iter().map(|c| EditorAction::AddComponent {
                    entity: new_id,
                    type_name: c.type_name.clone(),
                    data: c.data.clone(),
                }))
                .collect();
                self.state.add_entity(copy);
                Ok(EditorAction::Batch {
                    label: format!("Duplicate Entity {id}"),
                    actions: sub_actions,
                })
            }
            TemplateCommand::SetComponent { id, type_name, data } => {
                let entity = self.state.find_entity_mut(id).ok_or(OpsError::EntityNotFound { id })?;
                let old = entity.components.iter()
                    .find(|c| c.type_name == type_name)
                    .map(|c| c.data.clone())
                    .unwrap_or(serde_json::Value::Null);
                if let Some(comp) = entity.components.iter_mut().find(|c| c.type_name == type_name) {
                    comp.data = data.clone();
                } else {
                    entity.components.push(TemplateComponent { type_name: type_name.clone(), data: data.clone() });
                }
                Ok(EditorAction::SetComponent { entity: id, type_name, old, new: data })
            }
            TemplateCommand::AddComponent { id, type_name, data } => {
                let entity = self.state.find_entity_mut(id).ok_or(OpsError::EntityNotFound { id })?;
                if entity.components.iter().any(|c| c.type_name == type_name) {
                    return Err(OpsError::ComponentNotFound { entity: id, type_name }); // already exists
                }
                entity.components.push(TemplateComponent { type_name: type_name.clone(), data: data.clone() });
                Ok(EditorAction::AddComponent { entity: id, type_name, data })
            }
            TemplateCommand::RemoveComponent { id, type_name } => {
                let entity = self.state.find_entity_mut(id).ok_or(OpsError::EntityNotFound { id })?;
                let pos = entity.components.iter().position(|c| c.type_name == type_name)
                    .ok_or_else(|| OpsError::ComponentNotFound { entity: id, type_name: type_name.clone() })?;
                let snapshot = entity.components.remove(pos).data;
                Ok(EditorAction::RemoveComponent { entity: id, type_name, snapshot })
            }
        }
    }

    fn apply_inverse(&mut self, action: &EditorAction) -> Result<(), OpsError> {
        match action {
            EditorAction::CreateEntity { id, .. } => {
                self.state.remove_entity(*id);
            }
            EditorAction::DeleteEntity { snapshot, .. } => {
                let entity = TemplateEntity {
                    id: snapshot.id,
                    name: snapshot.name.clone(),
                    components: snapshot.components.iter().map(|(t, d)| TemplateComponent {
                        type_name: t.clone(), data: d.clone(),
                    }).collect(),
                };
                self.state.add_entity(entity);
            }
            EditorAction::RenameEntity { id, old_name, .. } => {
                if let Some(e) = self.state.find_entity_mut(*id) { e.name = old_name.clone(); }
            }
            EditorAction::SetComponent { entity, type_name, old, .. } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    if old.is_null() {
                        e.components.retain(|c| &c.type_name != type_name);
                    } else if let Some(c) = e.components.iter_mut().find(|c| &c.type_name == type_name) {
                        c.data = old.clone();
                    }
                }
            }
            EditorAction::AddComponent { entity, type_name, .. } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    e.components.retain(|c| &c.type_name != type_name);
                }
            }
            EditorAction::RemoveComponent { entity, type_name, snapshot } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    e.components.push(TemplateComponent { type_name: type_name.clone(), data: snapshot.clone() });
                }
            }
            EditorAction::Batch { actions, .. } => {
                for a in actions.iter().rev() { self.apply_inverse(a)?; }
            }
        }
        Ok(())
    }

    fn apply_action(&mut self, action: &EditorAction) -> Result<(), OpsError> {
        match action {
            EditorAction::CreateEntity { id, name } => {
                self.state.add_entity(TemplateEntity { id: *id, name: name.clone(), components: vec![] });
            }
            EditorAction::DeleteEntity { id, .. } => { self.state.remove_entity(*id); }
            EditorAction::RenameEntity { id, new_name, .. } => {
                if let Some(e) = self.state.find_entity_mut(*id) { e.name = new_name.clone(); }
            }
            EditorAction::SetComponent { entity, type_name, new, .. } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    if let Some(c) = e.components.iter_mut().find(|c| &c.type_name == type_name) {
                        c.data = new.clone();
                    } else {
                        e.components.push(TemplateComponent { type_name: type_name.clone(), data: new.clone() });
                    }
                }
            }
            EditorAction::AddComponent { entity, type_name, data } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    e.components.push(TemplateComponent { type_name: type_name.clone(), data: data.clone() });
                }
            }
            EditorAction::RemoveComponent { entity, type_name, .. } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    e.components.retain(|c| &c.type_name != type_name);
                }
            }
            EditorAction::Batch { actions, .. } => {
                for a in actions { self.apply_action(a)?; }
            }
        }
        Ok(())
    }

    fn next_entity_id(&self) -> EntityId {
        self.state.entities.iter().map(|e| e.id).max().map(|m| m + 1).unwrap_or(1)
    }

    fn write_state(&self) -> Result<(), OpsError> {
        self.state.save_yaml(&self.template_path)?;
        self.save_undo_history()
    }

    fn save_undo_history(&self) -> Result<(), OpsError> {
        let undo_path = undo_path_for(&self.template_path);
        let data = serde_json::to_string(&(&self.undo_stack, &self.done_ids))
            .map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })?;
        std::fs::write(&undo_path, data)
            .map_err(|e| OpsError::IoFailed { path: undo_path.display().to_string(), reason: e.to_string() })
    }
}

fn undo_path_for(template_path: &Path) -> PathBuf {
    template_path.with_extension("undo.json")
}

fn load_undo_history(path: &Path) -> Result<(UndoStack, VecDeque<ActionId>), OpsError> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| OpsError::IoFailed { path: path.display().to_string(), reason: e.to_string() })?;
    serde_json::from_str(&data)
        .map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })
}
```

Also add `history_summaries` to `UndoStack` in `undo.rs`:

```rust
/// Returns descriptions paired with external action IDs.
pub fn history_summaries(&self, ids: &VecDeque<crate::ipc::ActionId>) -> Vec<crate::ipc::ActionSummary> {
    self.done.iter().zip(ids.iter()).map(|(action, &id)| {
        crate::ipc::ActionSummary { action_id: id, description: action.description() }
    }).collect()
}
```

- [ ] **Step 4: Export processor in `engine/ops/src/lib.rs`**

Add: `pub mod processor;`

- [ ] **Step 5: Run processor tests — should PASS**

```bash
cd /d/dev/maethril/silmaril && cargo test -p engine-ops --test processor_tests 2>&1 | tail -30
```

Expected: 12 tests pass

- [ ] **Step 6: Run all ops tests**

```bash
cd /d/dev/maethril/silmaril && cargo test -p engine-ops 2>&1 | tail -20
```

Expected: all tests pass

- [ ] **Step 7: Commit**

```bash
git add engine/ops/src/processor.rs engine/ops/src/undo.rs engine/ops/src/lib.rs engine/ops/tests/processor_tests.rs
git commit -m "feat(ops): add CommandProcessor with execute/undo/redo and .undo.json persistence"
```

---

## Task 7: Remove anyhow from engine-ops

**Files:**
- Modify: `engine/ops/Cargo.toml`
- Modify: `engine/ops/src/lib.rs` (check for remaining anyhow usage)
- Modify: `engine/cli/src/main.rs` (it uses `anyhow::Result` — keep it there, anyhow stays in CLI)

Note: `engine/cli` can keep `anyhow` — it's a separate crate. Only `engine-ops` removes it.

- [ ] **Step 1: Check for remaining anyhow usage in engine-ops**

```bash
grep -rn "anyhow" /d/dev/maethril/silmaril/engine/ops/src/ | grep -v target
```

Expected: none (all replaced in Tasks 3 and 4)

- [ ] **Step 2: Remove from Cargo.toml**

In `engine/ops/Cargo.toml`, remove:
```toml
anyhow = "1.0"
```

- [ ] **Step 3: Compile check**

```bash
cd /d/dev/maethril/silmaril && cargo check -p engine-ops 2>&1 | tail -10
```

Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add engine/ops/Cargo.toml
git commit -m "chore(ops): remove anyhow dependency, all errors use OpsError"
```

---

## Task 8: Add template Tauri IPC commands

**Files:**
- Create: `engine/editor/src-tauri/bridge/template_commands.rs`
- Modify: `engine/editor/src-tauri/bridge/commands.rs` (remove `scene_command`, add `EditorState`)
- Modify: `engine/editor/src-tauri/bridge/mod.rs` (export `template_commands`)
- Modify: `engine/editor/src-tauri/lib.rs` (manage EditorState, register template commands)

- [ ] **Step 1: Create `engine/editor/src-tauri/bridge/template_commands.rs`**

```rust
//! Tauri IPC handlers for template CRUD and undo/redo.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use engine_ops::command::TemplateCommand;
use engine_ops::ipc::{ActionId, ActionSummary, CommandResult, IpcError};
use engine_ops::processor::CommandProcessor;
use engine_ops::template::TemplateState;
use tauri::State;

/// Global map of open template files → CommandProcessor.
pub struct EditorState {
    pub processors: HashMap<PathBuf, CommandProcessor>,
}

impl EditorState {
    pub fn new() -> Self {
        Self { processors: HashMap::new() }
    }
}

fn get_processor<'a>(
    map: &'a mut HashMap<PathBuf, CommandProcessor>,
    template_path: &str,
) -> Result<&'a mut CommandProcessor, IpcError> {
    let path = PathBuf::from(template_path);
    map.get_mut(&path).ok_or_else(|| IpcError {
        code: engine_core::error::ErrorCode::TemplateNoTemplateOpen as u32,
        message: format!("Template not open: {template_path}"),
    })
}

#[tauri::command]
pub fn template_open(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<TemplateState, IpcError> {
    let path = PathBuf::from(&template_path);
    let processor = CommandProcessor::load(path.clone()).map_err(IpcError::from)?;
    let result = processor.state_ref().clone();
    state.lock().unwrap().processors.insert(path, processor);
    Ok(result)
}

#[tauri::command]
pub fn template_close(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<(), IpcError> {
    let path = PathBuf::from(&template_path);
    state.lock().unwrap().processors.remove(&path);
    Ok(())
}

#[tauri::command]
pub fn template_execute(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
    command: TemplateCommand,
) -> Result<CommandResult, IpcError> {
    let mut guard = state.lock().unwrap();
    let proc = get_processor(&mut guard.processors, &template_path)?;
    proc.execute(command).map_err(IpcError::from)
}

#[tauri::command]
pub fn template_undo(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<Option<ActionId>, IpcError> {
    let mut guard = state.lock().unwrap();
    let proc = get_processor(&mut guard.processors, &template_path)?;
    proc.undo().map_err(IpcError::from)
}

#[tauri::command]
pub fn template_redo(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<Option<ActionId>, IpcError> {
    let mut guard = state.lock().unwrap();
    let proc = get_processor(&mut guard.processors, &template_path)?;
    proc.redo().map_err(IpcError::from)
}

#[tauri::command]
pub fn template_history(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<Vec<ActionSummary>, IpcError> {
    let guard = state.lock().unwrap();
    let path = PathBuf::from(&template_path);
    let proc = guard.processors.get(&path).ok_or_else(|| IpcError {
        code: engine_core::error::ErrorCode::TemplateNoTemplateOpen as u32,
        message: format!("Template not open: {template_path}"),
    })?;
    Ok(proc.history_summaries())
}
```

- [ ] **Step 2: Export in `engine/editor/src-tauri/bridge/mod.rs`**

Add: `pub mod template_commands;`

- [ ] **Step 3: Update `engine/editor/src-tauri/lib.rs`**

Add `.manage(Mutex::new(template_commands::EditorState::new()))`.

Add to `generate_handler!` list:
```rust
commands::template_commands::template_open,
commands::template_commands::template_close,
commands::template_commands::template_execute,
commands::template_commands::template_undo,
commands::template_commands::template_redo,
commands::template_commands::template_history,
```

Remove `commands::scene_command` from the handler list.

- [ ] **Step 4: Remove `scene_command` from `commands.rs`**

Delete the `scene_command` function and its doc comment (lines 150–213).

- [ ] **Step 5: Compile check**

```bash
cd /d/dev/maethril/silmaril/engine/editor && cargo build --no-default-features 2>&1 | grep "^error" | head -20
```

Expected: no errors

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src-tauri/bridge/ engine/editor/src-tauri/lib.rs
git commit -m "feat(editor): add template Tauri IPC commands (open/close/execute/undo/redo/history), replace scene_command"
```

---

## Task 9: Extend silm CLI with template edit commands

**Files:**
- Modify: `engine/cli/src/commands/template.rs`
- Modify: `engine/cli/src/main.rs`

The existing `template` subcommand has Add/Validate/Compile/List variants for a different purpose (template file management). We add new variants alongside them.

- [ ] **Step 1: Add new variants to `TemplateCommand` in `engine/cli/src/commands/template.rs`**

After the existing variants, add:

```rust
    /// Edit entity data in a template file (CQRS, supports undo/redo)
    Entity {
        /// Path to the template YAML file
        #[arg(long)]
        template: PathBuf,

        #[command(subcommand)]
        command: EntitySubcommand,
    },

    /// Edit component data in a template file
    Component {
        #[arg(long)]
        template: PathBuf,

        #[command(subcommand)]
        command: ComponentSubcommand,
    },

    /// Undo last template edit
    Undo {
        #[arg(long)]
        template: PathBuf,
    },

    /// Redo last undone template edit
    Redo {
        #[arg(long)]
        template: PathBuf,
    },

    /// Show template edit history
    History {
        #[arg(long)]
        template: PathBuf,
    },
```

Add the subcommand enums:

```rust
#[derive(Subcommand)]
pub enum EntitySubcommand {
    Create {
        #[arg(short, long)]
        name: Option<String>,
    },
    Delete { id: u64 },
    Rename { id: u64, name: String },
    Duplicate { id: u64 },
}

#[derive(Subcommand)]
pub enum ComponentSubcommand {
    Set { entity_id: u64, type_name: String, data: String },
    Add { entity_id: u64, type_name: String, data: String },
    Remove { entity_id: u64, type_name: String },
}
```

- [ ] **Step 2: Add handlers in `handle_template_command` function**

```rust
TemplateCommand::Entity { template, command } => {
    let cmd = match command {
        EntitySubcommand::Create { name } => TemplateCommand::CreateEntity { name },
        EntitySubcommand::Delete { id } => TemplateCommand::DeleteEntity { id },
        EntitySubcommand::Rename { id, name } => TemplateCommand::RenameEntity { id, name: Some(name) },
        EntitySubcommand::Duplicate { id } => TemplateCommand::DuplicateEntity { id },
    };
    let mut proc = CommandProcessor::load(template)?;
    let result = proc.execute(cmd)?;
    println!("{}", serde_json::to_string_pretty(&result.new_state).unwrap());
}
TemplateCommand::Component { template, command } => {
    let cmd = match command {
        ComponentSubcommand::Set { entity_id, type_name, data } => {
            let data = serde_json::from_str(&data).expect("data must be valid JSON");
            TemplateCommand::SetComponent { id: entity_id, type_name, data }
        }
        ComponentSubcommand::Add { entity_id, type_name, data } => {
            let data = serde_json::from_str(&data).expect("data must be valid JSON");
            TemplateCommand::AddComponent { id: entity_id, type_name, data }
        }
        ComponentSubcommand::Remove { entity_id, type_name } => {
            TemplateCommand::RemoveComponent { id: entity_id, type_name }
        }
    };
    let mut proc = CommandProcessor::load(template)?;
    let result = proc.execute(cmd)?;
    println!("{}", serde_json::to_string_pretty(&result.new_state).unwrap());
}
TemplateCommand::Undo { template } => {
    let mut proc = CommandProcessor::load(template)?;
    match proc.undo()? {
        Some(id) => println!("{{\"ok\":true,\"undone_action_id\":{id}}}"),
        None => println!("{{\"ok\":true,\"nothing_to_undo\":true}}"),
    }
}
TemplateCommand::Redo { template } => {
    let mut proc = CommandProcessor::load(template)?;
    match proc.redo()? {
        Some(id) => println!("{{\"ok\":true,\"redone_action_id\":{id}}}"),
        None => println!("{{\"ok\":true,\"nothing_to_redo\":true}}"),
    }
}
TemplateCommand::History { template } => {
    let proc = CommandProcessor::load(template)?;
    let summaries = proc.history_summaries();
    println!("{}", serde_json::to_string_pretty(&summaries).unwrap());
}
```

Add required imports at the top of the file:
```rust
use engine_ops::command::TemplateCommand as OpsTemplateCommand;
use engine_ops::processor::CommandProcessor;
```

Note: rename the local `TemplateCommand` variant to avoid collision with `engine_ops::command::TemplateCommand`. The clap enum stays as `TemplateCommand`; use `OpsTemplateCommand` alias for ops.

- [ ] **Step 3: Compile check**

```bash
cd /d/dev/maethril/silmaril && cargo check -p silm 2>&1 | grep "^error" | head -20
```

Expected: no errors

- [ ] **Step 4: Smoke test**

```bash
cd /d/dev/maethril/silmaril
# Create a temp template file
echo "name: test\nentities: []" > /tmp/test_template.yaml
cargo run --bin silm -- template entity --template /tmp/test_template.yaml create --name Hero 2>&1 | tail -10
```

Expected: JSON output with `entities` containing `Hero`

- [ ] **Step 5: Commit**

```bash
git add engine/cli/src/commands/template.rs
git commit -m "feat(cli): add silm template entity/component/undo/redo/history subcommands"
```

---

## Task 10: Update api.ts with TypeScript IPC wrappers

**Files:**
- Modify: `engine/editor/src/lib/api.ts`

Read the current `api.ts` to find where to add. Look for the `scene_command` call to replace.

- [ ] **Step 1: Read `engine/editor/src/lib/api.ts`**

```bash
grep -n "scene_command\|invoke\|export" /d/dev/maethril/silmaril/engine/editor/src/lib/api.ts | head -40
```

- [ ] **Step 2: Add TypeScript types and wrappers**

Add to `api.ts`:

```typescript
// ── Template CQRS types ────────────────────────────────────────────────────

export interface TemplateComponent {
  type_name: string;
  data: unknown;
}

export interface TemplateEntity {
  id: number;
  name: string | null;
  components: TemplateComponent[];
}

export interface TemplateState {
  name: string;
  entities: TemplateEntity[];
}

export interface CommandResult {
  action_id: number;
  new_state: TemplateState;
}

export interface ActionSummary {
  action_id: number;
  description: string;
}

export type TemplateCommand =
  | { CreateEntity: { name: string | null } }
  | { DeleteEntity: { id: number } }
  | { RenameEntity: { id: number; name: string | null } }
  | { DuplicateEntity: { id: number } }
  | { SetComponent: { id: number; type_name: string; data: unknown } }
  | { AddComponent: { id: number; type_name: string; data: unknown } }
  | { RemoveComponent: { id: number; type_name: string } };

// ── Template IPC calls ─────────────────────────────────────────────────────

export async function templateOpen(templatePath: string): Promise<TemplateState> {
  return invoke('template_open', { templatePath });
}

export async function templateClose(templatePath: string): Promise<void> {
  return invoke('template_close', { templatePath });
}

export async function templateExecute(templatePath: string, command: TemplateCommand): Promise<CommandResult> {
  return invoke('template_execute', { templatePath, command });
}

export async function templateUndo(templatePath: string): Promise<number | null> {
  return invoke('template_undo', { templatePath });
}

export async function templateRedo(templatePath: string): Promise<number | null> {
  return invoke('template_redo', { templatePath });
}

export async function templateHistory(templatePath: string): Promise<ActionSummary[]> {
  return invoke('template_history', { templatePath });
}
```

Remove or comment the old `sceneCommand` call if present.

- [ ] **Step 3: Build frontend type check**

```bash
cd /d/dev/maethril/silmaril/engine/editor && npx tsc --noEmit 2>&1 | head -20
```

Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src/lib/api.ts
git commit -m "feat(editor): add TypeScript IPC wrappers for template CQRS commands"
```

---

## Task 11: Update gitignore + add proptest integration tests

**Files:**
- Modify: `engine/ops/src/project.rs`
- Modify: `engine/ops/tests/processor_tests.rs` (add proptest tests)

- [ ] **Step 1: Add `*.undo.json` to the gitignore template**

In `engine/ops/src/project.rs`, in the `gitignore()` method, in the `# Editor runtime state` block add:

```
# Undo history (ephemeral, not committed)
*.undo.json
```

- [ ] **Step 2: Add proptest tests to `processor_tests.rs`**

Add at the end of the file:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn undo_redo_restores_state(names in proptest::collection::vec("[a-zA-Z]{1,10}", 0..5)) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("prop.yaml");
        std::fs::write(&path, "name: prop\nentities: []\n").unwrap();
        let mut proc = CommandProcessor::load(path.clone()).unwrap();

        // Execute N creates
        let n = names.len();
        for name in &names {
            proc.execute(TemplateCommand::CreateEntity { name: Some(name.clone()) }).unwrap();
        }
        let state_after = proc.state_ref().clone();

        // Undo all
        for _ in 0..n {
            proc.undo().unwrap();
        }
        assert!(proc.state_ref().entities.is_empty(), "after undoing all creates, state should be empty");

        // Redo all
        for _ in 0..n {
            proc.redo().unwrap();
        }
        assert_eq!(proc.state_ref().entities.len(), state_after.entities.len(),
            "after redoing all, entity count should match");

        // YAML must be valid
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: Result<serde_yaml::Value, _> = serde_yaml::from_str(&content);
        assert!(parsed.is_ok(), "YAML must be valid after undo/redo cycle");
    }
}
```

- [ ] **Step 3: Run proptest**

```bash
cd /d/dev/maethril/silmaril && cargo test -p engine-ops --test processor_tests proptest 2>&1 | tail -20
```

Expected: proptest generates cases, all pass

- [ ] **Step 4: Commit**

```bash
git add engine/ops/src/project.rs engine/ops/tests/processor_tests.rs
git commit -m "feat(ops/cli): add *.undo.json to gitignore template, add proptest undo/redo invariant tests"
```

---

## Task 12: E2E test script

**Files:**
- Create: `scripts/e2e-tests/test-template-undo-redo.sh`

- [ ] **Step 1: Build silm binary first**

```bash
cd /d/dev/maethril/silmaril && cargo build --bin silm 2>&1 | tail -5
```

Expected: binary at `target/debug/silm`

- [ ] **Step 2: Create the E2E script**

```bash
#!/usr/bin/env bash
# E2E test: silm template entity undo/redo round-trip
# Requires: pre-built silm at target/debug/silm (run cargo build --bin silm first)
set -euo pipefail

SILM="./target/debug/silm"
TMPDIR=$(mktemp -d)
TEMPLATE="$TMPDIR/world.yaml"

echo "name: world" > "$TEMPLATE"
echo "entities: []" >> "$TEMPLATE"

echo "=== Creating entity ==="
$SILM template entity --template "$TEMPLATE" create --name "Hero"

grep -q "Hero" "$TEMPLATE" || { echo "FAIL: Hero not in YAML after create"; exit 1; }
echo "PASS: entity present after create"

echo "=== Undoing create ==="
$SILM template undo --template "$TEMPLATE"

grep -q "Hero" "$TEMPLATE" && { echo "FAIL: Hero still in YAML after undo"; exit 1; } || true
echo "PASS: entity absent after undo"

echo "=== Redoing create ==="
$SILM template redo --template "$TEMPLATE"

grep -q "Hero" "$TEMPLATE" || { echo "FAIL: Hero not in YAML after redo"; exit 1; }
echo "PASS: entity present after redo"

echo "=== All E2E tests passed ==="
rm -rf "$TMPDIR"
```

- [ ] **Step 3: Make executable and run**

```bash
chmod +x /d/dev/maethril/silmaril/scripts/e2e-tests/test-template-undo-redo.sh
cd /d/dev/maethril/silmaril && bash scripts/e2e-tests/test-template-undo-redo.sh
```

Expected:
```
PASS: entity present after create
PASS: entity absent after undo
PASS: entity present after redo
All E2E tests passed
```

- [ ] **Step 4: Commit**

```bash
git add scripts/e2e-tests/test-template-undo-redo.sh
git commit -m "test(e2e): add template undo/redo round-trip E2E test script"
```

---

## Final Verification

- [ ] Run all engine-ops tests:
  ```bash
  cd /d/dev/maethril/silmaril && cargo test -p engine-ops 2>&1 | tail -20
  ```
- [ ] Run editor build:
  ```bash
  cd /d/dev/maethril/silmaril/engine/editor && cargo build 2>&1 | grep "^error" | head -10
  ```
- [ ] Run silm build:
  ```bash
  cd /d/dev/maethril/silmaril && cargo build --bin silm 2>&1 | grep "^error" | head -10
  ```
- [ ] Run E2E test:
  ```bash
  bash scripts/e2e-tests/test-template-undo-redo.sh
  ```
