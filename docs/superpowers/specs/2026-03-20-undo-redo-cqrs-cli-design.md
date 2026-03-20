# Undo/Redo + CQRS + CLI Integration Design

## Overview

Implement a unified command system (CQRS) for the editor's template editing operations, with full undo/redo support accessible from both the editor UI and the `silm` CLI. All state is file-authoritative (YAML templates on disk). The `UndoStack` in `engine/ops` is extended into a complete `CommandProcessor` that becomes the single choke-point for all template mutations.

---

## Section 1: Naming & Terminology

- **Template** — a YAML file defining entities and their components. One unified type: a "scene" is just a template loaded as the root world. No separate Scene/Prefab distinction.
- **World** — the runtime ECS container (in-memory)
- **Project** — the `game.toml` root; owns zero or more templates
- **EntityId** — `u64`, already defined in `engine/ops/src/undo.rs`

**Migration of `scene.rs`**: Rename `Scene → TemplateState`, `SceneEntity → TemplateEntity`, `SceneComponent → TemplateComponent` in-place. No structural changes except the `name` field — see Section 3.

All code, IPC commands, CLI subcommands, and docs use "template" not "scene".

---

## Section 2: Undoable vs. Not-Undoable

### Undoable (go through CommandProcessor + UndoStack)

| Command | Notes |
|---|---|
| `CreateEntity` | Initial name stored in action |
| `DeleteEntity` | Stores full `EntitySnapshot` (incl. name) for restoration |
| `DuplicateEntity` | Decomposes to `Batch(CreateEntity + N AddComponent)` internally |
| `RenameEntity` | Old name stored in action |
| `SetComponent` | Old value stored in action (includes Transform for move/rotate/scale) |
| `AddComponent` | `EditorAction::AddComponent` stores the full `data: Value` for redo |
| `RemoveComponent` | Full component data stored for restoration |

### Not Undoable (bypass CommandProcessor)

| Command | Reason |
|---|---|
| Camera orbit/pan/zoom | Transient viewport state |
| Projection toggle (ortho/persp) | Viewport preference, localStorage |
| Grid/snap/tool settings | Viewport preference |
| Open/close template | Filesystem operation |
| New template | Filesystem operation (creates file) — not reversible |

---

## Section 3: Core Types

### Error Type

Existing `ErrorCode` entries for the 2000–2099 range (Template System):

```
TemplateNotFound         = 2000
TemplateAlreadyExists    = 2001
TemplateInvalidYaml      = 2002
TemplateUnknownComponent = 2003
TemplateCircularReference = 2004
TemplateIo               = 2005
TemplateSerialization    = 2006
```

New entries to add (2007–2011):

```
TemplateEntityNotFound    = 2007
TemplateComponentNotFound = 2008
TemplateUndoEmpty         = 2009
TemplateRedoEmpty         = 2010
TemplateNoTemplateOpen    = 2011
```

`OpsError` definition:

```rust
// engine/ops/src/error.rs
use silmaril_core::define_error;

define_error! {
    pub enum OpsError {
        EntityNotFound { id: EntityId }
            = ErrorCode::TemplateEntityNotFound, ErrorSeverity::Error,
        ComponentNotFound { entity: EntityId, component_type: String }
            = ErrorCode::TemplateComponentNotFound, ErrorSeverity::Error,
        WriteFailed { path: String, reason: String }
            = ErrorCode::TemplateIo, ErrorSeverity::Error,
        YamlFailed { reason: String }
            = ErrorCode::TemplateSerialization, ErrorSeverity::Error,
        UndoEmpty
            = ErrorCode::TemplateUndoEmpty, ErrorSeverity::Warning,
        RedoEmpty
            = ErrorCode::TemplateRedoEmpty, ErrorSeverity::Warning,
        NoTemplateOpen
            = ErrorCode::TemplateNoTemplateOpen, ErrorSeverity::Error,
    }
}
```

**Migration**: `undo.rs` currently uses `anyhow::Result`. Replace `bail!("Nothing to undo")` with `Err(OpsError::UndoEmpty)` and likewise for redo. Remove `anyhow` dependency from `engine/ops`.

### TemplateState

```rust
// engine/ops/src/template.rs  (renamed from scene.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateState {
    pub name: String,
    pub entities: Vec<TemplateEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateEntity {
    pub id: EntityId,
    pub name: Option<String>,   // keeps parity with existing SceneEntity
    pub components: Vec<TemplateComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateComponent {
    pub type_name: String,
    pub data: serde_json::Value,
}
```

`TemplateCommand::CreateEntity { name: Option<String> }` — empty string and `None` are both valid.

### ActionId

```rust
pub type ActionId = u64;  // monotonically incrementing per CommandProcessor instance
```

Serializes as a JSON number over IPC. Frontend uses it to label undo/redo buttons.

### EditorAction changes

`AddComponent` gains a `data` field to enable redo (re-adding the exact component):

```rust
/// Before (existing):
AddComponent { entity: EntityId, name: String }

/// After:
AddComponent { entity: EntityId, name: String, data: serde_json::Value }
```

`EntitySnapshot` gains a `name` field so `DeleteEntity` undo can restore the entity's display name:

```rust
/// Before (existing):
pub struct EntitySnapshot {
    pub id: EntityId,
    pub components: Vec<(String, Value)>,
}

/// After:
pub struct EntitySnapshot {
    pub id: EntityId,
    pub name: Option<String>,
    pub components: Vec<(String, Value)>,
}
```

`UndoStack` internals: replace `Vec<EditorAction>` with `VecDeque<EditorAction>` so trimming at max depth is O(1). Add `#[derive(Serialize, Deserialize)]` to `UndoStack`, `EditorAction`, `EntitySnapshot` for `.undo.json` persistence. `EditorAction::Batch` serializes recursively — this is intentional.

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

`DuplicateEntity` does NOT appear in `EditorAction`. `execute()` decomposes it into `EditorAction::Batch { label: "Duplicate Entity", actions: [CreateEntity, AddComponent×N] }`.

### CommandResult & IPC Error Envelope

```rust
#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub action_id: ActionId,
    pub new_state: TemplateState,   // full state, MVP scope — acknowledged cost
}

#[derive(Debug, Serialize)]
pub struct ActionSummary {
    pub action_id: ActionId,
    pub description: String,   // from EditorAction::description()
}

#[derive(Debug, Serialize)]
pub struct IpcError {
    pub code: u32,
    pub message: String,
}
```

`CommandResult` returns full `TemplateState` on every mutation. This is O(entities × components) per command and is acceptable for MVP. Optimization (delta events) is deferred.

`history()` is a Rust-internal method. The IPC/CLI surface uses `Vec<ActionSummary>` (described below) so `EditorAction` does not need to be a public IPC type.

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
}

impl CommandProcessor {
    pub fn load(path: PathBuf) -> Result<Self, OpsError>;
    pub fn execute(&mut self, cmd: TemplateCommand) -> Result<CommandResult, OpsError>;
    // Returns Ok(None) when stack is empty (not an error — caller checks can_undo first)
    pub fn undo(&mut self) -> Result<Option<ActionId>, OpsError>;
    pub fn redo(&mut self) -> Result<Option<ActionId>, OpsError>;
    pub fn history_summaries(&self) -> Vec<ActionSummary>;
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
}
```

`undo()` returns `Ok(None)` when nothing to undo. `OpsError::UndoEmpty` / `RedoEmpty` are reserved for internal assertion failures only.

### Multi-Template Support in Tauri

Multiple templates can be open simultaneously (one per tab). Tauri state holds a map keyed by template path:

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

`engine/ops` is a library crate shared by both Tauri backend and `silm`. CLI adds a `template` subcommand group. All commands take `--template <path>` to specify the target YAML file explicitly (required for scripting/agent use):

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
├── world.undo.json          ← serialized UndoStack (auto-managed, never commit)
```

`*.undo.json` is added to the generated `.gitignore` by `silm new` (update `BasicTemplate::gitignore()` in `project.rs`).

---

## Section 5: Testing Strategy

### Tier 1 — Unit Tests (`engine/ops/tests/`)

- `UndoStack`: push → undo → redo cycle; new command after undo clears redo stack; max depth enforcement (100); `VecDeque` O(1) trim verified
- `CommandProcessor::execute`: each `TemplateCommand` variant produces the correct `EditorAction` variant
- Inverse correctness: `undo(execute(cmd))` → state equals pre-command state (all 7 command variants)
- `DuplicateEntity` produces `EditorAction::Batch` with correct sub-actions; single undo reverts all
- `AddComponent` undo removes component; `AddComponent` redo restores exact data
- `DeleteEntity` undo restores name + components via `EntitySnapshot`
- `undo()` returns `Ok(None)` on empty stack (not error)
- Property-based (proptest, uses `tempfile` — still single crate):
  - For any sequence of N commands: `undo` N times → `redo` N times → final state equals post-command state
  - YAML file is always valid after any command or undo/redo

### Tier 2 — Integration Tests (`engine/shared/tests/`)

- Full round-trip: execute command → assert YAML written → undo → assert YAML restored to original content
- Persistence: write `.undo.json` → create fresh `CommandProcessor::load()` → undo still works
- Error cases: `DeleteEntity` on non-existent id returns `OpsError::EntityNotFound`
- Multi-template: two `CommandProcessor` instances on different paths operate independently

### Tier 3 — E2E Tests (`scripts/e2e-tests/`)

- `silm` CLI subprocess: pre-built binary required (CI builds before running Tier 3)
  - `silm template --template x.yaml entity create "X"` → read YAML → assert entity present
  - `silm template --template x.yaml undo` → read YAML → assert entity absent
  - `silm template --template x.yaml redo` → read YAML → assert entity present

### AI-Agent Testability

`silm` CLI + JSON output + file-authoritative state: an agent can script full regression suites — no UI, no mocks, no Tauri.

---

## Constraints & Non-Goals

- **No collaborative editing**: single writer at a time
- **No network sync of undo history**: undo is local to the editor session
- **No undo of filesystem operations**: open/close/new template
- **Max undo depth**: 100 actions (configurable in `editor.toml`)
- **Full state on every command**: `CommandResult` returns complete `TemplateState` — MVP scope, optimization deferred
- **Frontend state**: `scene/state.ts` and `scene/commands.ts` replaced by IPC calls; no duplicated state on the JS side
