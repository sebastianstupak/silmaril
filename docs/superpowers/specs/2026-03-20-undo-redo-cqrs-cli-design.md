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

---

## Section 2: Undoable vs. Not-Undoable

### Undoable (go through CommandProcessor + UndoStack)

| Command | Notes |
|---|---|
| `CreateEntity` | Includes initial component set |
| `DeleteEntity` | Stores full `EntitySnapshot` for restoration |
| `DuplicateEntity` | Treated as CreateEntity with copied components |
| `RenameEntity` | Old name stored in action |
| `SetComponent` | Old value stored in action (includes Transform for move/rotate/scale) |
| `AddComponent` | Removable on undo |
| `RemoveComponent` | Full component data stored for restoration |
| `NewTemplate` | Creates empty template file |

### Not Undoable (bypass CommandProcessor)

| Command | Reason |
|---|---|
| Camera orbit/pan/zoom | Transient viewport state, not template data |
| Projection toggle (ortho/persp) | Viewport preference, persisted in localStorage |
| Grid/snap/tool settings | Viewport preference, not template data |
| Open/close template | File system operation, not a mutation |

---

## Section 3: Architecture & Data Flow

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

### Core Types

```rust
// engine/ops/src/command.rs
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

pub struct CommandResult {
    pub action_id: ActionId,
    pub new_state: TemplateState,
}
```

```rust
// engine/ops/src/processor.rs
pub struct CommandProcessor {
    state: TemplateState,
    undo_stack: UndoStack,
    template_path: PathBuf,
}

impl CommandProcessor {
    pub fn execute(&mut self, cmd: TemplateCommand) -> Result<CommandResult, OpsError>;
    pub fn undo(&mut self) -> Result<Option<ActionId>, OpsError>;
    pub fn redo(&mut self) -> Result<Option<ActionId>, OpsError>;
    pub fn history(&self) -> &[EditorAction];
}
```

### File Conventions

```
templates/
├── world.yaml               ← template file (source of truth)
├── world.undo.json          ← serialized UndoStack (auto-managed)
├── player.yaml
└── player.undo.json
```

Undo history is co-located with its template. If `.undo.json` is deleted, history is lost but the template is unaffected. Both files are `.gitignore`d by convention (history is ephemeral).

### Tauri Integration

`CommandProcessor` lives in `tauri::State<Mutex<CommandProcessor>>`. The current scattered "scene" Tauri commands are replaced by two IPC endpoints:

```rust
#[tauri::command]
async fn template_execute(
    state: State<'_, Mutex<CommandProcessor>>,
    command: TemplateCommand,
) -> Result<CommandResult, String>;

#[tauri::command]
async fn template_undo(state: State<'_, Mutex<CommandProcessor>>) -> Result<Option<ActionId>, String>;

#[tauri::command]
async fn template_redo(state: State<'_, Mutex<CommandProcessor>>) -> Result<Option<ActionId>, String>;
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
silm template new <name>       # create new template file
```

Short forms when a project context is active:
```bash
silm undo
silm redo
```

All commands print JSON results to stdout for scripting/agent consumption.

---

## Section 4: Testing Strategy

### Tier 1 — Unit Tests (`engine/ops/tests/`)

- `UndoStack`: push → undo → redo cycle; new command after undo clears redo stack; max depth enforcement (100)
- `CommandProcessor::execute`: each `TemplateCommand` variant produces the correct `EditorAction` variant
- Inverse correctness: `undo(execute(cmd))` → state equals pre-command state (for all 8 command variants)
- `Batch` actions: N grouped commands → single undo step

### Tier 2 — Integration Tests (`engine/shared/tests/`)

- Full round-trip: execute command → assert YAML written → undo → assert YAML restored to original content
- Persistence: serialize `UndoStack` to `.undo.json` → deserialize fresh `CommandProcessor` → undo still works
- `silm` CLI integration: spawn subprocess, run `silm template entity create`, assert YAML file, run `silm template undo`, assert YAML reverted

### Tier 3 — Property-Based Tests (`engine/ops/tests/`, proptest)

- For any sequence of N commands: `undo` N times → `redo` N times → final state equals post-command state
- File is always valid YAML after any command or undo/redo operation
- Undo stack never exceeds max depth regardless of command sequence length

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
- **Frontend state**: `scene/state.ts` and `scene/commands.ts` are replaced by IPC calls to `CommandProcessor`; no duplicated state on the JS side
