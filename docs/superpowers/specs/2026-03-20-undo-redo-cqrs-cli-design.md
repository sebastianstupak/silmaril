# Undo/Redo + CQRS + CLI Integration Design

## Overview

Implement a unified command system (CQRS) for the editor's template editing operations, with full undo/redo support accessible from both the editor UI and the `silm` CLI. All state is file-authoritative (YAML templates on disk). The `UndoStack` in `engine/ops` is extended into a complete `CommandProcessor` that becomes the single choke-point for all template mutations.

---

## Section 1: Naming & Terminology

- **Template** — a YAML file defining entities and their components. One unified type: a "scene" is just a template loaded as the root world. No separate Scene/Prefab distinction.
- **World** — the runtime ECS container (in-memory)
- **Project** — the `game.toml` root; owns zero or more templates
- **EntityId** — `u64`, already defined in `engine/ops/src/undo.rs`

**Migration of `scene.rs`**: Rename `Scene → TemplateState`, `SceneEntity → TemplateEntity`, `SceneComponent → TemplateComponent` in-place. No structural changes; existing `save_yaml`/`load_yaml`/`save_bincode`/`load_bincode` methods are kept and renamed accordingly. `TemplateState::name` is set to the file stem on load (`path.file_stem().unwrap_or_default()`).

All code, IPC commands, CLI subcommands, and docs use "template" not "scene".

---

## Section 2: Undoable vs. Not-Undoable

### Undoable (go through CommandProcessor + UndoStack)

| Command | Notes |
|---|---|
| `CreateEntity` | Initial name stored; `EditorAction::CreateEntity` gains `name: Option<String>` |
| `DeleteEntity` | Stores full `EntitySnapshot` (incl. name) for restoration |
| `DuplicateEntity` | Decomposes to `Batch(CreateEntity + N AddComponent)` internally |
| `RenameEntity` | Old name stored in action; both old and new are `Option<String>` |
| `SetComponent` | Old value stored; `EditorAction::SetComponent.name` renamed to `type_name` |
| `AddComponent` | `EditorAction::AddComponent` gains `data: Value`; `name` renamed to `type_name` |
| `RemoveComponent` | Full component data stored; `EditorAction::RemoveComponent.name` renamed to `type_name` |

### Not Undoable (bypass CommandProcessor)

| Command | Reason |
|---|---|
| Camera orbit/pan/zoom | Transient viewport state |
| Projection toggle (ortho/persp) | Viewport preference, localStorage |
| Grid/snap/tool settings | Viewport preference |
| Open/close template | Filesystem operation |
| New template | Filesystem operation — not reversible |

---

## Section 3: Core Types

### Error Type

Existing `ErrorCode` entries for Template System (2000–2006) stay unchanged. New entries to add at 2007–2009:

```
TemplateEntityNotFound    = 2007
TemplateComponentNotFound = 2008
TemplateNoTemplateOpen    = 2011
```

`OpsError::UndoEmpty` and `OpsError::RedoEmpty` are NOT added — empty-stack is handled at the `CommandProcessor` layer by returning `Ok(None)`, never as an error.

```rust
// engine/ops/src/error.rs
use silmaril_core::define_error;

define_error! {
    pub enum OpsError {
        EntityNotFound { id: EntityId }
            = ErrorCode::TemplateEntityNotFound, ErrorSeverity::Error,
        ComponentNotFound { entity: EntityId, type_name: String }
            = ErrorCode::TemplateComponentNotFound, ErrorSeverity::Error,
        // Covers both read and write I/O failures
        IoFailed { path: String, reason: String }
            = ErrorCode::TemplateIo, ErrorSeverity::Error,
        // Covers both serialization (YAML write) and deserialization (YAML/Bincode read) failures
        SerializeFailed { reason: String }
            = ErrorCode::TemplateSerialization, ErrorSeverity::Error,
        NoTemplateOpen
            = ErrorCode::TemplateNoTemplateOpen, ErrorSeverity::Error,
    }
}
```

**Migration**: `scene.rs` and `undo.rs` both use `anyhow::Result`. Replace all `anyhow` usage with `OpsError`. Remove `anyhow` dependency from `engine/ops/Cargo.toml`.

### IpcError and Conversion

```rust
#[derive(Debug, Serialize)]
pub struct IpcError {
    pub code: u32,
    pub message: String,
}

impl From<OpsError> for IpcError {
    fn from(e: OpsError) -> Self {
        IpcError {
            code: e.code() as u32,
            message: e.to_string(),
        }
    }
}
```

### TemplateState (renamed from Scene)

```rust
// engine/ops/src/template.rs  (renamed from scene.rs)
// Keep json_as_string mod as-is — required for Bincode compatibility

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateState {
    pub name: String,
    pub entities: Vec<TemplateEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateEntity {
    pub id: EntityId,
    pub name: Option<String>,
    pub components: Vec<TemplateComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateComponent {
    pub type_name: String,
    #[serde(with = "json_as_string")]  // keep — required for Bincode round-trip
    pub data: serde_json::Value,
}
```

### ActionId

```rust
pub type ActionId = u64;  // monotonically incrementing per CommandProcessor instance
```

Serializes as JSON number over IPC. `undo()` returns the `ActionId` of the action that was popped from the done stack (i.e., the action that was reversed). Frontend uses this to label undo/redo buttons.

### EditorAction Changes

All three changes are in `engine/ops/src/undo.rs`. **Standardize on `type_name` (not `name`) for component identifiers throughout**:

```rust
// Before → After for each variant:

SetComponent {
    entity: EntityId,
    name: String,     // RENAME → type_name: String
    old: Value,
    new: Value,
}

AddComponent {
    entity: EntityId,
    name: String,     // RENAME → type_name: String
                      // ADD   → data: Value  (for redo restoration)
}

RemoveComponent {
    entity: EntityId,
    name: String,     // RENAME → type_name: String
    snapshot: Value,
}

CreateEntity {
    id: EntityId,
                      // ADD → name: Option<String>
}

RenameEntity {
    id: EntityId,
    old_name: String, // CHANGE → old_name: Option<String>
    new_name: String, // CHANGE → new_name: Option<String>
}
// DeleteEntity, Batch — unchanged
```

### EntitySnapshot Changes

Add `name` field so `DeleteEntity` undo can restore the entity's display name:

```rust
pub struct EntitySnapshot {
    pub id: EntityId,
    pub name: Option<String>,          // ADD
    pub components: Vec<(String, Value)>,
}
```

### UndoStack Changes

1. Replace `Vec<EditorAction>` (both `done` and `undone`) with `VecDeque<EditorAction>` for O(1) front-pop when trimming at max depth. Serde serializes `VecDeque` identically to `Vec` for non-wrapped sequences — no custom serde needed.
2. Add `#[derive(Serialize, Deserialize)]` to `UndoStack`, `EditorAction`, `EntitySnapshot` for `.undo.json` persistence.
3. Keep `UndoStack::undo()`/`redo()` returning `Err` on empty stack (existing behavior, existing tests pass). Only `CommandProcessor` translates this to `Ok(None)`.

### TemplateCommand

```rust
// engine/ops/src/command.rs
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

`DuplicateEntity` does NOT appear in `EditorAction`. `execute()` decomposes it into `EditorAction::Batch { label: "Duplicate Entity <id>", actions: [CreateEntity { id: new_id, name: copied_name }, AddComponent×N] }`.

### CommandResult & ActionSummary

```rust
#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub action_id: ActionId,
    pub new_state: TemplateState,   // full state, MVP scope — acknowledged cost
}

// Used by history IPC endpoint; EditorAction is NOT exposed over IPC
#[derive(Debug, Serialize)]
pub struct ActionSummary {
    pub action_id: ActionId,
    pub description: String,   // from EditorAction::description()
}
```

Returning full `TemplateState` on every command is O(entities × components). Acceptable for MVP; delta events are deferred.

---

## Section 4: Architecture & Data Flow

```
silm CLI  ──────────────────────────┐
                                    ▼
                          TemplateCommand enum
                          (typed, versioned)
                                    │
Editor Frontend (Tauri IPC) ────────┘
                                    │
                                    ▼
                         CommandProcessor (engine/ops)
                         ├── validates command
                         ├── mutates TemplateState (in-memory)
                         ├── writes YAML → templates/<name>.yaml
                         ├── pushes EditorAction to UndoStack
                         └── returns CommandResult

                         UndoStack (engine/ops/src/undo.rs)
                         ├── undo() → inverse EditorAction → re-write YAML
                         └── redo() → re-apply EditorAction → re-write YAML

                         Persistence: templates/<name>.undo.json
```

### CommandProcessor

```rust
// engine/ops/src/processor.rs
pub struct CommandProcessor {
    state: TemplateState,
    undo_stack: UndoStack,
    template_path: PathBuf,
    next_action_id: ActionId,
}

impl CommandProcessor {
    /// Load template YAML from disk; load .undo.json if present (missing = empty stack).
    pub fn load(path: PathBuf) -> Result<Self, OpsError>;

    /// Execute a command. Returns the assigned ActionId and updated TemplateState.
    pub fn execute(&mut self, cmd: TemplateCommand) -> Result<CommandResult, OpsError>;

    /// Undo last action. Returns Ok(None) when nothing to undo.
    /// Returns Ok(Some(action_id)) where action_id is the action that was reversed.
    pub fn undo(&mut self) -> Result<Option<ActionId>, OpsError>;

    /// Redo last undone action. Returns Ok(None) when nothing to redo.
    pub fn redo(&mut self) -> Result<Option<ActionId>, OpsError>;

    /// Summaries for history display. Internal EditorAction not exposed.
    pub fn history_summaries(&self) -> Vec<ActionSummary>;

    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
}
```

`CommandProcessor::load()` fails with `OpsError::IoFailed` if the YAML file cannot be read, or `OpsError::SerializeFailed` if it cannot be parsed. Missing `.undo.json` is not an error — the stack starts empty.

### Multi-Template Support in Tauri

Multiple templates can be open simultaneously (one per tab):

```rust
pub struct EditorState {
    processors: HashMap<PathBuf, CommandProcessor>,
}
tauri::State<Mutex<EditorState>>
```

All IPC commands include `template_path: String`.

### Tauri IPC

```rust
#[tauri::command]
async fn template_open(state, template_path: String) -> Result<TemplateState, IpcError>;
#[tauri::command]
async fn template_close(state, template_path: String) -> Result<(), IpcError>;
#[tauri::command]
async fn template_execute(state, template_path: String, command: TemplateCommand) -> Result<CommandResult, IpcError>;
#[tauri::command]
async fn template_undo(state, template_path: String) -> Result<Option<ActionId>, IpcError>;
#[tauri::command]
async fn template_redo(state, template_path: String) -> Result<Option<ActionId>, IpcError>;
#[tauri::command]
async fn template_history(state, template_path: String) -> Result<Vec<ActionSummary>, IpcError>;
```

### silm CLI

`engine/ops` is shared between Tauri backend and `silm`. All commands take `--template <path>` (required for scripting/agent use):

```bash
silm template --template templates/world.yaml entity create "Player"
silm template --template templates/world.yaml entity delete <id>
silm template --template templates/world.yaml entity rename <id> "NewName"
silm template --template templates/world.yaml entity duplicate <id>
silm template --template templates/world.yaml component set <entity-id> transform '{"x":0,"y":1,"z":0}'
silm template --template templates/world.yaml component add <entity-id> health '{"current":100,"max":100}'
silm template --template templates/world.yaml component remove <entity-id> health
silm template --template templates/world.yaml undo
silm template --template templates/world.yaml redo
silm template --template templates/world.yaml history
```

All commands print JSON results to stdout.

### File Conventions

```
templates/
├── world.yaml               ← template file (source of truth)
├── world.undo.json          ← serialized UndoStack (never commit)
```

Add `*.undo.json` to the root-level `.gitignore` generated by `silm new` (update `BasicTemplate::gitignore()` in `project.rs`). This pattern applies at all directory depths — any `.undo.json` file anywhere in the project tree is excluded, which is the desired behavior.

---

## Section 5: Testing Strategy

### Tier 1 — Unit Tests (`engine/ops/tests/`)

All tests here are single-crate (`engine_ops` only). Filesystem tests use `tempfile` crate (dev-dependency).

- `UndoStack`: push → undo → redo cycle; new command after undo clears redo stack; max depth at 100 trims oldest; `VecDeque` O(1) trim
- `UndoStack::undo()` / `redo()` return `Err` on empty (existing tests must still pass)
- `CommandProcessor::execute`: each `TemplateCommand` variant produces correct `EditorAction` variant with correct field names (`type_name`, not `name`)
- `CommandProcessor::undo()` / `redo()` return `Ok(None)` on empty (translation layer above `UndoStack`)
- Inverse correctness: `undo(execute(cmd))` → state equals pre-command state (all 7 variants)
- `DuplicateEntity` produces `EditorAction::Batch` with correct sub-actions; single undo reverts all
- `AddComponent` undo removes component; redo restores exact `data`
- `DeleteEntity` undo restores `name` + all components via `EntitySnapshot`
- `RenameEntity` with `Option<String>` old/new names round-trips correctly
- Persistence (uses `tempfile`): write `.undo.json` → create fresh `CommandProcessor::load()` → undo still works
- Round-trip (uses `tempfile`): execute command → assert YAML written → undo → assert YAML restored to original
- Property-based (proptest, uses `tempfile`):
  - For any sequence of N commands: `undo` N times → `redo` N times → final state equals post-command state
  - YAML file is always valid after any command or undo/redo

### Tier 2 — Integration Tests (`engine/shared/tests/`)

Only if tests import from 2+ engine crates. At present, no Tier 2 tests are planned for this feature — all cross-system integration is done at Tier 3.

### Tier 3 — E2E Tests (`scripts/e2e-tests/`)

- Pre-built `silm` binary required (CI builds before running Tier 3)
- `silm template --template x.yaml entity create "X"` → read YAML → assert entity present
- `silm template --template x.yaml undo` → read YAML → assert entity absent
- `silm template --template x.yaml redo` → read YAML → assert entity present

### AI-Agent Testability

`silm` CLI + JSON output + file-authoritative YAML: an agent can script complete regression suites with no UI, no mocks, no Tauri.

---

## Constraints & Non-Goals

- **No collaborative editing**: single writer at a time
- **No network sync of undo history**: undo is local to the editor session
- **No undo of filesystem operations**: open/close/new template
- **Max undo depth**: 100 actions (configurable in `editor.toml`)
- **Full state on every command**: `CommandResult` returns complete `TemplateState` — MVP scope, delta optimization deferred
- **Frontend state**: `scene/state.ts` and `scene/commands.ts` replaced by IPC calls; no duplicated state on the JS side
