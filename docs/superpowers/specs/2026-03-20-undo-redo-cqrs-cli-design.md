# Undo/Redo + CQRS + CLI Integration Design

## Overview

Implement a unified command system (CQRS) for the editor's template editing operations, with full undo/redo support accessible from both the editor UI and the `silm` CLI. All state is file-authoritative (YAML templates on disk). The `UndoStack` in `engine/ops` is extended into a complete `CommandProcessor` that becomes the single choke-point for all template mutations.

---

## Section 1: Naming & Terminology

The engine uses ECS terminology throughout. The correct terms are:

- **Template** — a YAML file defining entities and their components. Unified type: a "scene" is just a template loaded as the root world. No separate Scene/Prefab distinction.
- **World** — the runtime ECS container (in-memory, not serialized directly)
- **Project** — the `game.toml` root; owns zero or more templates
- **EntityId** — stable identifier for entities within a template

All code, IPC commands, CLI subcommands, and docs use "template" not "scene".

**Migration of `scene.rs`**: The existing `engine/ops/src/scene.rs` exports `Scene`, `SceneEntity`, `SceneComponent`. These are renamed in-place to `Template`, `TemplateEntity`, `TemplateComponent`. This is a mechanical rename; no structural changes.

---

## Section 2: Undoable vs. Not-Undoable

### Undoable (go through CommandProcessor + UndoStack)

| Command | Notes |
|---|---|
| `CreateEntity` | Includes initial component set |
| `DeleteEntity` | Stores full `EntitySnapshot` for restoration |
| `DuplicateEntity` | Internally produces a `Batch` action (CreateEntity + N AddComponent) |
| `RenameEntity` | Old name stored in action |
| `SetComponent` | Old value stored in action (includes Transform for move/rotate/scale) |
| `AddComponent` | Removable on undo |
| `RemoveComponent` | Full component data stored for restoration |
| `NewTemplate` | Creates empty template file; previous template is closed first |

### Not Undoable (bypass CommandProcessor)

| Command | Reason |
|---|---|
| Camera orbit/pan/zoom | Transient viewport state, not template data |
| Projection toggle (ortho/persp) | Viewport preference, persisted in localStorage |
| Grid/snap/tool settings | Viewport preference, not template data |
| Open/close template | File system operation, not a mutation |

---

## Section 3: Core Types

### Error Type

```rust
// engine/ops/src/error.rs
use silmaril_core::define_error;

define_error! {
    pub enum OpsError {
        EntityNotFound { id: EntityId } = ErrorCode::OpsEntityNotFound, ErrorSeverity::Error,
        ComponentNotFound { entity: EntityId, component_type: String } = ErrorCode::OpsComponentNotFound, ErrorSeverity::Error,
        TemplateWriteFailed { path: String, reason: String } = ErrorCode::OpsTemplateWriteFailed, ErrorSeverity::Error,
        YamlSerializeFailed { reason: String } = ErrorCode::OpsYamlSerializeFailed, ErrorSeverity::Error,
        UndoStackEmpty = ErrorCode::OpsUndoStackEmpty, ErrorSeverity::Warning,
        RedoStackEmpty = ErrorCode::OpsRedoStackEmpty, ErrorSeverity::Warning,
        NoTemplateOpen = ErrorCode::OpsNoTemplateOpen, ErrorSeverity::Error,
    }
}
```

Error codes are in the 2000–2099 range (Template/Ops System). Define these in `engine/core`'s `ErrorCode` enum.

**Note**: existing `undo.rs` uses `anyhow::Result` — this must be migrated to `OpsError` as part of this work.

### TemplateState (rename of Scene)

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
    pub name: String,
    pub components: Vec<TemplateComponent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateComponent {
    pub type_name: String,
    pub data: serde_json::Value,
}

// ComponentData is a type alias for clarity at call sites
pub type ComponentData = TemplateComponent;
```

### ActionId

```rust
// engine/ops/src/undo.rs
pub type ActionId = u64;  // monotonically incrementing per CommandProcessor instance
```

Serializes as a JSON number over IPC. The frontend uses it to label undo/redo buttons (e.g., "Undo: Set Transform").

### TemplateCommand

```rust
// engine/ops/src/command.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateCommand {
    CreateEntity { name: String },
    DeleteEntity { id: EntityId },
    RenameEntity { id: EntityId, name: String },
    DuplicateEntity { id: EntityId },
    SetComponent { id: EntityId, component: ComponentData },
    AddComponent { id: EntityId, component: ComponentData },
    RemoveComponent { id: EntityId, component_type: String },
    NewTemplate { name: String },
}
```

`DuplicateEntity` does NOT appear in `EditorAction`. It is decomposed by `CommandProcessor::execute` into a `EditorAction::Batch` containing `CreateEntity` + N `AddComponent` actions, so a single undo reverts the whole duplicate.

### CommandResult

```rust
#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub action_id: ActionId,
    pub new_state: TemplateState,
}
```

### IPC Error Envelope

Tauri commands return a serializable error envelope, not a raw `String`:

```rust
#[derive(Debug, Serialize)]
pub struct IpcError {
    pub code: u32,
    pub message: String,
}
```

Frontend can switch on `code` to handle specific error conditions programmatically.

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
    pub fn undo(&mut self) -> Result<Option<ActionId>, OpsError>;
    pub fn redo(&mut self) -> Result<Option<ActionId>, OpsError>;
    pub fn history(&self) -> &[EditorAction];  // uses EditorAction::description() for display
}
```

### Multi-Template Support in Tauri

Multiple templates can be open simultaneously (one per tab). Tauri state holds a map keyed by template path:

```rust
// Tauri app state
pub struct EditorState {
    processors: HashMap<PathBuf, CommandProcessor>,
}

tauri::State<Mutex<EditorState>>
```

All IPC commands take a `template_path: String` parameter to select the active processor.

### Tauri IPC

```rust
#[tauri::command]
async fn template_execute(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
    command: TemplateCommand,
) -> Result<CommandResult, IpcError>;

#[tauri::command]
async fn template_undo(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<Option<ActionId>, IpcError>;

#[tauri::command]
async fn template_redo(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<Option<ActionId>, IpcError>;

#[tauri::command]
async fn template_open(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<TemplateState, IpcError>;

#[tauri::command]
async fn template_close(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<(), IpcError>;
```

### silm CLI Integration

`engine/ops` is a library crate shared by both the Tauri backend and the `silm` binary. The CLI adds a `template` subcommand group:

```bash
silm template entity create "Player"
silm template entity delete <id>
silm template entity rename <id> "NewName"
silm template entity duplicate <id>
silm template component set <entity-id> transform '{"x":0,"y":1,"z":0}'
silm template component add <entity-id> health '{"current":100,"max":100}'
silm template component remove <entity-id> health
silm template undo
silm template redo
silm template history          # print undo stack summary
silm template new <name>       # create new template file, close any currently open
```

All commands print JSON results to stdout for scripting/agent consumption.

### File Conventions

```
templates/
├── world.yaml               ← template file (source of truth)
├── world.undo.json          ← serialized UndoStack (auto-managed)
├── player.yaml
└── player.undo.json
```

`.undo.json` files are added to the project's `.gitignore` by `silm new` (in the generated `.gitignore` template): `*.undo.json`. The existing `BasicTemplate::gitignore()` in `project.rs` must include this pattern.

---

## Section 5: Testing Strategy

### Tier 1 — Unit Tests (`engine/ops/tests/`)

- `UndoStack`: push → undo → redo cycle; new command after undo clears redo stack; max depth enforcement (100)
- `CommandProcessor::execute`: each `TemplateCommand` variant produces the correct `EditorAction` variant
- Inverse correctness: `undo(execute(cmd))` → state equals pre-command state (all 8 command variants)
- `DuplicateEntity` produces `EditorAction::Batch` with correct sub-actions
- `Batch` actions: undo reverts all sub-actions atomically
- Property-based (proptest, uses `tempfile` for filesystem — still Tier 1 since single crate):
  - For any sequence of N commands: `undo` N times → `redo` N times → final state equals post-command state
  - File is always valid YAML after any command or undo/redo

### Tier 2 — Integration Tests (`engine/shared/tests/`)

- Full round-trip: execute command → assert YAML written → undo → assert YAML restored to original content
- Persistence: serialize `UndoStack` to `.undo.json` → create fresh `CommandProcessor` from disk → undo still works
- Error cases: `DeleteEntity` on non-existent id returns `OpsError::EntityNotFound`

### Tier 3 — E2E Tests (`scripts/e2e-tests/`)

- `silm` CLI subprocess: build binary, spawn `silm template entity create "X"`, read YAML file, assert entity present, spawn `silm template undo`, read YAML file, assert entity absent
- Requires pre-built `silm` binary (CI must build before running Tier 3)

### AI-Agent Testability

The `silm` CLI + JSON output + file-authoritative state means an AI agent can:
1. Run `silm template entity create "X"` → read YAML to verify
2. Run `silm template undo` → read YAML to verify restoration
3. Script full regression suites with no UI, no mocks, no Tauri

---

## Constraints & Non-Goals

- **No collaborative editing**: single writer at a time, no CRDT or conflict resolution
- **No network sync of undo history**: undo is local to the editor session
- **No undo of file system operations** (open/close template, project-level changes)
- **Max undo depth**: 100 actions (configurable in `editor.toml`)
- **Frontend state**: `scene/state.ts` and `scene/commands.ts` are replaced by IPC calls; no duplicated state on the JS side
