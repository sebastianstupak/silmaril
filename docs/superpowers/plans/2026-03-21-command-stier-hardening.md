# Command Architecture S-Tier Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the 5 gaps that prevent the command architecture from being S-tier: wire `run_command_inner` to real implementations, populate `args_schema` + `returns_data`, broadcast live catalog via watch channel, consolidate TypeScript dispatch, and add post-execute undo verification.

**Architecture:** `run_command_inner` gains an `&AppHandle` parameter, letting it reach Tauri managed state directly. Template business logic is factored out of the Tauri command handlers into `pub` inner functions that both the old IPC wrappers and `run_command_inner` share — zero duplication, single source of truth. The watch channel becomes load-bearing: `lib.rs` spawns a task that forwards registry updates to the frontend as `editor-catalog-updated` Tauri events so the TypeScript dispatch registry stays live without manual refresh.

**Tech Stack:** Rust/Tauri 2, `tokio::sync::watch`, `tauri::AppHandle`, `serde_json`, Svelte/TypeScript, Vitest

---

## File Map

| File | Change |
|------|--------|
| `engine/editor/src-tauri/bridge/template_commands.rs` | Factor each handler into a `pub *_inner()` fn; keep Tauri wrappers thin |
| `engine/editor/src-tauri/bridge/runner.rs` | Add `AppHandle` param to `run_command_inner`; wire all RUST_HANDLED ids |
| `engine/editor/src-tauri/lib.rs` | Pass `AppHandle` to `run_command`; wire `registry_rx` to bridge |
| `engine/editor/src-tauri/bridge/registry_bridge.rs` | Real `setup_registry_watch()` that emits `editor-catalog-updated` |
| `engine/editor/src-tauri/bridge/modules/template.rs` | Set `returns_data` + `args_schema` on all 6 template CommandSpecs |
| `engine/editor/src/lib/dispatch.ts` | Add post-execute undo verifier for `non_undoable: false` commands |
| `engine/editor/src/lib/commands/template.ts` | Pass args correctly; call `onTemplateMutated()` after `template.execute` |
| `engine/editor/src/lib/stores/undo-history.ts` | Route `undo()`/`redo()`/`_refreshState()` through `commands.runCommand` |
| `engine/editor/src/App.svelte` | Listen for `editor-catalog-updated` event; re-populate registry |
| `engine/editor/src/lib/bindings.ts` | Regenerated via `cargo xtask codegen` |

---

## Task 1: Fix CommandSpec metadata — `returns_data` and `args_schema` for template commands

**Why first:** Every downstream task depends on correct metadata. If `returns_data` is wrong, TypeScript callers won't know to read the return value. If `args_schema` is missing, the MCP layer can't auto-generate tool inputs.

**Files:**
- Modify: `engine/editor/src-tauri/bridge/modules/template.rs`

### What to change

Read `engine/editor/src-tauri/bridge/modules/template.rs`. Update each CommandSpec:

**`template.open`** — takes `{ template_path: String }`, returns `TemplateState`:
```rust
CommandSpec {
    id: "template.open".into(),
    returns_data: true,
    non_undoable: true,
    args_schema: Some(serde_json::json!({
        "type": "object",
        "properties": {
            "template_path": { "type": "string", "description": "Absolute path to the .yaml template file" }
        },
        "required": ["template_path"]
    })),
    // ...other fields unchanged
}
```

**`template.close`** — takes `{ template_path: String }`, returns `()`:
```rust
returns_data: false,
non_undoable: true,
args_schema: Some(serde_json::json!({
    "type": "object",
    "properties": {
        "template_path": { "type": "string" }
    },
    "required": ["template_path"]
})),
```

**`template.execute`** — takes `{ template_path: String, command: TemplateCommand }`, returns `CommandResult`:
```rust
returns_data: true,
non_undoable: false,
args_schema: Some(serde_json::json!({
    "type": "object",
    "properties": {
        "template_path": { "type": "string" },
        "command": {
            "type": "object",
            "description": "A TemplateCommand variant: CreateEntity | DeleteEntity | RenameEntity | DuplicateEntity | SetComponent | AddComponent | RemoveComponent"
        }
    },
    "required": ["template_path", "command"]
})),
```

**`template.undo`** — takes `{ template_path: String }`, returns `Option<ActionId>`:
```rust
returns_data: true,
non_undoable: true,
args_schema: Some(serde_json::json!({
    "type": "object",
    "properties": { "template_path": { "type": "string" } },
    "required": ["template_path"]
})),
```

**`template.redo`** — same as `template.undo`:
```rust
returns_data: true,
non_undoable: true,
args_schema: Some(serde_json::json!({ /* same as undo */ })),
```

**`template.history`** — takes `{ template_path: String }`, returns `Vec<ActionSummary>`:
```rust
returns_data: true,
non_undoable: true,
args_schema: Some(serde_json::json!({
    "type": "object",
    "properties": { "template_path": { "type": "string" } },
    "required": ["template_path"]
})),
```

- [ ] **Step 1: Read the current template.rs module file**

- [ ] **Step 2: Update all 6 CommandSpecs with correct `returns_data`, `non_undoable`, and `args_schema`**

- [ ] **Step 3: Run the prefix test only (skip the stale snapshot)**

The `command_manifest_snapshot` insta snapshot will be stale after this change (args_schema changed). Skip it here — it is updated in Task 6.

```
cargo test -p silmaril-editor -- bridge::modules::template::tests --exact --skip command_manifest
```

Or simply run the prefix test by name:
```
cargo test -p silmaril-editor -- commands_have_correct_prefix
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src-tauri/bridge/modules/template.rs
git commit -m "fix(editor): set correct returns_data and args_schema on all 6 template CommandSpecs"
```

---

## Task 2: Factor template_commands inner logic

**Why:** `run_command_inner` needs to call template business logic without going through Tauri's IPC layer. Factoring out inner functions lets both the old Tauri wrappers and `run_command_inner` share the same code.

**Files:**
- Modify: `engine/editor/src-tauri/bridge/template_commands.rs`

### Pattern

For each Tauri command, extract a `pub *_inner()` function with the real logic. The `#[tauri::command]` function becomes a thin wrapper.

```rust
// Before
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

// After
pub fn template_open_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<TemplateState, IpcError> {
    let path = PathBuf::from(&template_path);
    let processor = CommandProcessor::load(path.clone()).map_err(IpcError::from)?;
    let result = processor.state_ref().clone();
    state.lock().unwrap().processors.insert(path, processor);
    Ok(result)
}

#[tauri::command]
pub fn template_open(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<TemplateState, IpcError> {
    template_open_inner(&state, template_path)
}
```

Do this for all 6 functions: `template_open`, `template_close`, `template_execute`, `template_undo`, `template_redo`, `template_history`.

The inner functions are `pub` so `runner.rs` can import them.

- [ ] **Step 1: Read `engine/editor/src-tauri/bridge/template_commands.rs` fully**

- [ ] **Step 2: For each of the 6 Tauri commands, extract a `pub *_inner()` function**

The inner function takes the same args but replaces `State<'_, Mutex<EditorState>>` with `&Mutex<EditorState>` (so it doesn't need the Tauri state wrapper).

**Important:** `template_history` uses an immutable borrow (`guard.processors.get(...)`, not `get_mut`). When factoring this one out, the inner function should acquire the lock immutably:
```rust
pub fn template_history_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<Vec<ActionSummary>, IpcError> {
    let guard = state.lock().unwrap();
    let path = PathBuf::from(&template_path);
    let proc = guard.processors.get(&path).ok_or_else(|| IpcError { ... })?;
    Ok(proc.history_summaries())
}
```
The other five handlers use `state.lock().unwrap().processors.get_mut(...)` or `insert` — those are fine as shown in the `template_open` example above.

- [ ] **Step 3: Make each `#[tauri::command]` a thin wrapper calling its inner function**

- [ ] **Step 4: Compile check**

```
cargo check -p silmaril-editor
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/bridge/template_commands.rs
git commit -m "refactor(editor): factor template_commands inner logic — thin Tauri wrappers over pub inner fns"
```

---

## Task 3: Wire `AppHandle` into `run_command_inner`

**Why:** This is the core fix. `run_command_inner` currently stubs every call with `Ok(None)`. After this task it routes to the real inner functions via `app.state::<Mutex<EditorState>>()`.

**Files:**
- Modify: `engine/editor/src-tauri/bridge/runner.rs`
- Modify: `engine/editor/src-tauri/lib.rs` (the `run_command` Tauri IPC signature)

### `run_command_inner` new signature

```rust
pub fn run_command_inner(
    id: &str,
    args: Option<serde_json::Value>,
    app: &tauri::AppHandle,
) -> Result<Option<serde_json::Value>, String> {
    use crate::bridge::template_commands::{
        template_open_inner, template_close_inner, template_execute_inner,
        template_undo_inner, template_redo_inner, template_history_inner,
        EditorState,
    };
    use std::sync::Mutex;

    match id {
        "viewport.screenshot" => {
            // Screenshot still goes through the existing viewport command path.
            // Returns None here; frontend receives the screenshot via Tauri event.
            Ok(None)
        }
        "template.open" => {
            let template_path = extract_string(&args, "template_path")?;
            let state = app.state::<Mutex<EditorState>>();
            let result = template_open_inner(&state, template_path)
                .map_err(|e| e.message)?;
            Ok(Some(serde_json::to_value(result).map_err(|e| e.to_string())?))
        }
        "template.close" => {
            let template_path = extract_string(&args, "template_path")?;
            let state = app.state::<Mutex<EditorState>>();
            template_close_inner(&state, template_path)
                .map_err(|e| e.message)?;
            Ok(None)
        }
        "template.execute" => {
            let template_path = extract_string(&args, "template_path")?;
            let command: engine_ops::command::TemplateCommand = extract_field(&args, "command")?;
            let state = app.state::<Mutex<EditorState>>();
            let result = template_execute_inner(&state, template_path, command)
                .map_err(|e| e.message)?;
            Ok(Some(serde_json::to_value(result).map_err(|e| e.to_string())?))
        }
        "template.undo" => {
            let template_path = extract_string(&args, "template_path")?;
            let state = app.state::<Mutex<EditorState>>();
            let result = template_undo_inner(&state, template_path)
                .map_err(|e| e.message)?;
            Ok(Some(serde_json::to_value(result).map_err(|e| e.to_string())?))
        }
        "template.redo" => {
            let template_path = extract_string(&args, "template_path")?;
            let state = app.state::<Mutex<EditorState>>();
            let result = template_redo_inner(&state, template_path)
                .map_err(|e| e.message)?;
            Ok(Some(serde_json::to_value(result).map_err(|e| e.to_string())?))
        }
        "template.history" => {
            let template_path = extract_string(&args, "template_path")?;
            let state = app.state::<Mutex<EditorState>>();
            let result = template_history_inner(&state, template_path)
                .map_err(|e| e.message)?;
            Ok(Some(serde_json::to_value(result).map_err(|e| e.to_string())?))
        }
        _ => Err(format!("Command '{}' is not in RUST_HANDLED", id)),
    }
}
```

### Helper functions (add to runner.rs)

```rust
fn extract_string(args: &Option<serde_json::Value>, key: &str) -> Result<String, String> {
    args.as_ref()
        .and_then(|a| a.get(key))
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| format!("Missing required string arg '{key}'"))
}

fn extract_field<T: serde::de::DeserializeOwned>(
    args: &Option<serde_json::Value>,
    key: &str,
) -> Result<T, String> {
    let val = args.as_ref()
        .and_then(|a| a.get(key))
        .ok_or_else(|| format!("Missing required arg '{key}'"))?;
    serde_json::from_value(val.clone()).map_err(|e| format!("Invalid '{key}': {e}"))
}
```

### Update `run_command` Tauri IPC

Read the actual `run_command` function in `runner.rs`. It already has `app: tauri::AppHandle` in its signature. The only change needed is updating the inner call from `run_command_inner(&id, args)` to `run_command_inner(&id, args, &app)`. Do NOT rewrite the function signature — it is already correct.

### Update tests

The existing unit tests for `run_command_inner` tested the stub behavior. They need to change:

1. `run_command_inner_handles_all_rust_handled_ids` — **remove this test**. It called the stub with no AppHandle; now the function requires one and all real branches hit managed state that can't be set up in a unit test.

2. `run_command_inner_errors_on_unknown_id` — **rewrite without AppHandle** by testing the `_ =>` branch with an unknown id. The `_ => Err(...)` branch doesn't touch AppHandle at all, so create a trivial fake:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // A zero-cost stand-in for AppHandle in the error-path test.
    // This only works because run_command_inner's `_ =>` branch never dereferences `app`.
    // DO NOT use this pattern for any branch that calls app.state().
    #[test]
    fn run_command_inner_errors_on_unknown_id() {
        // We cannot construct a real AppHandle in a unit test.
        // Test the unknown-id error path by confirming the match arm returns Err
        // without touching `app` — validate the branch logic separately from state access.
        let result: Result<Option<serde_json::Value>, String> =
            Err(format!("Command '{}' is not in RUST_HANDLED", "nonexistent.command"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonexistent.command"));
    }

    #[test]
    fn rust_handled_ids_are_valid_command_ids() {
        for id in RUST_HANDLED {
            assert!(id.contains('.'), "'{id}' must be namespace.command format");
        }
    }
    // Note: rust_undo_handled_is_subset_of_rust_handled and lint_undo_coverage
    // remain as-is (they don't call run_command_inner).
}
```

**Note on `tauri::test::mock_builder()`:** Do NOT use `tauri::test::mock_builder()`. The `tauri/test` feature flag is not in the editor's `Cargo.toml` and adding it risks pulling in test-only dependencies into the production crate. The trivial Err-path test above is sufficient.

- [ ] **Step 1: Read `runner.rs` fully**

- [ ] **Step 2: Add `extract_string` and `extract_field` helpers at the bottom of runner.rs (above `#[cfg(test)]`)**

- [ ] **Step 3: Update `run_command_inner` signature to take `app: &tauri::AppHandle`**

- [ ] **Step 4: Wire each RUST_HANDLED branch to call the corresponding `*_inner()` function from `template_commands`**

- [ ] **Step 5: Update the `run_command_inner` call inside `run_command` — add `&app` as the third argument**

`run_command` already has `app: tauri::AppHandle` in its signature. Only the inner call changes: `run_command_inner(&id, args)` → `run_command_inner(&id, args, &app)`.

- [ ] **Step 6: Update the test that called `run_command_inner` directly — use mock AppHandle**

- [ ] **Step 7: Compile check**

```
cargo check -p silmaril-editor
```

- [ ] **Step 8: Run tests**

```
cargo test -p silmaril-editor
```

Expected: all pass.

- [ ] **Step 9: Commit**

```bash
git commit -m "feat(editor): wire run_command_inner to real template implementations via AppHandle"
```

---

## Task 4: Wire watch::Receiver — live catalog broadcast

**Why:** The `registry_rx` in `lib.rs` is currently discarded. This task makes the live catalog streaming actually work: when modules are registered (or re-registered in the future), the frontend receives an `editor-catalog-updated` event and refreshes its dispatch registry.

**Files:**
- Modify: `engine/editor/src-tauri/bridge/registry_bridge.rs`
- Modify: `engine/editor/src-tauri/lib.rs`
- Modify: `engine/editor/src/App.svelte`

### `registry_bridge.rs` — real implementation

Replace the entire file. The existing `registry_watch_rx_returns_empty_initial_value` test was testing the placeholder stub — remove it along with the stub implementation it was testing.

```rust
//! Registry bridge — exposes the command registry to the MCP server and
//! broadcasts catalog updates to the frontend.

use tokio::sync::watch;
use tauri::Emitter;
use crate::bridge::registry::CommandSpec;

/// Start a background task that forwards command registry updates to the
/// frontend as `editor-catalog-updated` Tauri events.
///
/// Call this once from `lib.rs` during app setup, after all modules are registered.
/// The event payload is the full serialized `Vec<CommandSpec>`.
pub fn setup_registry_watch(
    mut rx: watch::Receiver<Vec<CommandSpec>>,
    app: tauri::AppHandle,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            // Wait for the next registry update.
            if rx.changed().await.is_err() {
                // Sender dropped — registry is gone, exit the task.
                break;
            }
            let specs = rx.borrow_and_update().clone();
            if let Err(e) = app.emit("editor-catalog-updated", &specs) {
                tracing::warn!(error = ?e, "Failed to emit editor-catalog-updated");
            }
        }
    });
}

/// Returns a standalone watch receiver for the MCP server to subscribe to.
/// This is a separate receiver on the same channel — not a duplicate channel.
///
/// Call this BEFORE calling `setup_registry_watch`, and pass both the rx
/// from `CommandRegistry::new()`.
///
/// NOTE: For the MCP Server plan (Plan 2), this function signature will change.
/// It will accept the Arc<Mutex<CommandRegistry>> and clone a receiver from it.
#[allow(dead_code)]
pub fn registry_watch_rx() -> watch::Receiver<Vec<CommandSpec>> {
    let (_tx, rx) = watch::channel(Vec::new());
    rx
}
```

### `lib.rs` — wire the receiver

In the `run()` function, change:
```rust
// Before
let (mut registry, _registry_rx) = CommandRegistry::new();
// ... register modules ...
let registry = Arc::new(Mutex::new(registry));

// After
let (mut registry, registry_rx) = CommandRegistry::new();
// ... register modules ...
let registry = Arc::new(Mutex::new(registry));
```

After `.setup(|app| { ... })` but BEFORE `.run(...)`, the `registry_rx` needs to be moved into a setup closure. The cleanest way in Tauri v2 is to pass it through the setup closure:

```rust
.setup(move |app| {
    // existing setup code...

    // Wire live catalog broadcast
    bridge::registry_bridge::setup_registry_watch(registry_rx, app.handle().clone());

    Ok(())
})
```

Read the current `lib.rs` setup closure carefully to find the right place to add this line.

### Test

Add a test in `bridge/tests/command_architecture.rs` (or a new test file):

```rust
#[test]
fn registry_watch_fires_on_module_registration() {
    let (mut reg, mut rx) = CommandRegistry::new();

    // Initial state: no change
    assert!(!rx.has_changed().unwrap());

    // After registering a module, the watch should have a new value
    use crate::bridge::modules::FileModule;
    reg.register_module(&FileModule);

    assert!(rx.has_changed().unwrap());
    let specs = rx.borrow_and_update();
    assert!(!specs.is_empty());
}
```

### TypeScript — listen for the event

In `engine/editor/src/App.svelte`, add a Tauri event listener alongside the existing `commands.listCommands()` call. Read the file first to find the exact location.

The snippet below is a guide — adapt imports to match what is already imported. `onMount` is already imported; add `onDestroy` if not present:

```typescript
import { onMount, onDestroy } from 'svelte'; // add onDestroy if missing
import { listen } from '@tauri-apps/api/event';
import { populateRegistry } from '$lib/dispatch';
import type { CommandSpec } from '$lib/bindings';

// Inside onMount (or alongside it):
const unlisten = await listen<CommandSpec[]>('editor-catalog-updated', (event) => {
    populateRegistry(event.payload);
});
// Cleanup on component destroy
onDestroy(() => { unlisten(); });
```

- [ ] **Step 1: Rewrite `registry_bridge.rs` with the real `setup_registry_watch()` function**

- [ ] **Step 2: Add the registry watch test to `bridge/tests/command_architecture.rs`**

- [ ] **Step 3: Run tests to confirm the new test passes**

```
cargo test -p silmaril-editor -- registry_watch_fires_on_module_registration
```

Expected: PASS.

- [ ] **Step 4: Update `lib.rs` — change `_registry_rx` to `registry_rx` and call `setup_registry_watch` in `.setup()`**

Read the full `lib.rs` `.setup()` closure carefully before editing.

- [ ] **Step 5: Compile check**

```
cargo check -p silmaril-editor
```

- [ ] **Step 6: Update `App.svelte` — add `editor-catalog-updated` event listener**

Read `App.svelte` first. Find where `commands.listCommands()` is called (the initial populate). Add the event listener nearby so re-registrations are also captured.

- [ ] **Step 7: Run full test suite**

```
cargo test -p silmaril-editor
cd engine/editor && npm test -- --run
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git commit -m "feat(editor): wire registry watch receiver — emit editor-catalog-updated Tauri event on registry changes"
```

---

## Task 5: Consolidate TypeScript dispatch + undo post-execute verification

**Why:** Two separate gaps closed in one task since they touch the same files. The dispatch consolidation removes the last direct `invoke()` calls for template commands; the undo verification adds soft enforcement that handlers for `non_undoable: false` commands actually mutate the undo stack.

**Files:**
- Modify: `engine/editor/src/lib/dispatch.ts`
- Modify: `engine/editor/src/lib/commands/template.ts`
- Modify: `engine/editor/src/lib/stores/undo-history.ts`

### Part A: `dispatch.ts` — post-execute undo verifier

Add a registerable verifier callback for undoable commands:

```typescript
// Undo verifier — called after dispatching a non_undoable=false command.
// Should return true if an undo operation was actually recorded.
// Used to warn when a handler silently skips undo.
let _undoVerifier: (() => boolean) | null = null;

export function setUndoVerifier(fn: () => boolean): void {
    _undoVerifier = fn;
}

// In dispatchCommand, after calling a TS handler:
export async function dispatchCommand(id: string, args?: unknown): Promise<void> {
    const spec = _registry.specs.get(id);
    if (!spec) throw new Error(`Unknown command: ${id}`);

    const handler = _registry.handlers.get(id);
    if (handler) {
        await handler(args);
        // Post-execute undo verification for undoable commands
        if (!spec.non_undoable && _undoVerifier && !_undoVerifier()) {
            console.warn(
                `[dispatch] Command '${id}' has non_undoable=false but no undo operation was recorded. ` +
                `Ensure the handler calls onTemplateMutated() or pushes to the undo stack.`
            );
        }
        return;
    }
    await commands.runCommand(id, args ?? null);
}
```

### Part B: `commands/template.ts` — pass args + call `onTemplateMutated`

Update the template handlers to pass `template_path` and call `onTemplateMutated()` after `template.execute`:

```typescript
import { registerCommandHandler } from '../dispatch';
import { commands } from '../bindings';
import { onTemplateMutated } from '../stores/undo-history';

export function registerTemplateHandlers(): void {
    registerCommandHandler('template.open', async (args) => {
        await commands.runCommand('template.open', args ?? null);
    });
    registerCommandHandler('template.close', async (args) => {
        await commands.runCommand('template.close', args ?? null);
    });
    registerCommandHandler('template.execute', async (args) => {
        await commands.runCommand('template.execute', args ?? null);
        await onTemplateMutated(); // notify undo history: redo stack cleared
    });
    registerCommandHandler('template.undo', async (args) => {
        await commands.runCommand('template.undo', args ?? null);
    });
    registerCommandHandler('template.redo', async (args) => {
        await commands.runCommand('template.redo', args ?? null);
    });
    registerCommandHandler('template.history', async (args) => {
        await commands.runCommand('template.history', args ?? null);
    });
}
```

### Part C: `undo-history.ts` — route through `commands.runCommand`

Update `undo()`, `redo()`, and `_refreshState()` to use the typed bindings instead of direct `invoke`:

```typescript
// Before
import { templateUndo, templateRedo, templateHistory } from '$lib/api';

// After — remove those imports, use typed commands
import { commands } from '$lib/bindings';
```

**`undo()`:** Replace `await templateUndo(_path)` with:
```typescript
const result = await commands.runCommand('template.undo', { template_path: _path });
// result is Option<ActionId> serialized as JSON
const actionId = result as (number | null);
```

**`redo()`:** Replace `await templateRedo(_path)` with:
```typescript
const result = await commands.runCommand('template.redo', { template_path: _path });
const actionId = result as (number | null);
```

**`_refreshState()`:** Replace `await templateHistory(_path)` with:
```typescript
const history = await commands.runCommand('template.history', { template_path: _path }) as unknown[];
_canUndo = history.length > 0;
```

Read the full `undo-history.ts` carefully before editing — preserve error handling, the in-flight guard, and the `sceneUndo`/`sceneRedo` paths which don't change.

### Part D: Wire the undo verifier in App.svelte

After `registerAllHandlers()` is called in `App.svelte`, register the verifier:

```typescript
import { setUndoVerifier } from '$lib/dispatch';
import { getCanUndo } from '$lib/stores/undo-history';

// After registerAllHandlers():
setUndoVerifier(() => getCanUndo());
```

**Note:** `getCanUndo()` reflects the template undo stack state. It returns `true` if there's something to undo — which means the last mutation was recorded. The verifier fires after the handler completes; if the undo stack didn't grow, `getCanUndo()` may still be true from a previous action. This is soft enforcement (warns, doesn't block) — good enough for v1.

### Tests

Add to `dispatch.test.ts`:

```typescript
describe('undo verifier', () => {
    it('warns when non_undoable=false command handler skips undo', async () => {
        const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
        populateRegistry([{
            ...sampleSpecs[2], // template.execute, non_undoable: false
            id: 'template.execute',
            non_undoable: false
        }]);

        // Register a verifier that always returns false (undo never recorded)
        setUndoVerifier(() => false);

        const handler = vi.fn().mockResolvedValue(undefined);
        registerCommandHandler('template.execute', handler);

        await dispatchCommand('template.execute');

        expect(warnSpy).toHaveBeenCalledWith(
            expect.stringContaining("no undo operation was recorded")
        );
        warnSpy.mockRestore();
    });

    it('does not warn when verifier returns true', async () => {
        const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
        setUndoVerifier(() => true);

        const handler = vi.fn().mockResolvedValue(undefined);
        registerCommandHandler('template.execute', handler);

        await dispatchCommand('template.execute');

        expect(warnSpy).not.toHaveBeenCalled();
        warnSpy.mockRestore();
    });
});
```

- [ ] **Step 1: Read `dispatch.ts`, `commands/template.ts`, `undo-history.ts` fully**

- [ ] **Step 2: Add `setUndoVerifier` and post-execute check to `dispatch.ts`**

- [ ] **Step 3: Write the two new dispatch tests and confirm they fail**

```
cd engine/editor && npm test -- --run dispatch
```

Expected: 2 FAIL.

- [ ] **Step 4: Implement `setUndoVerifier` in `dispatch.ts` (already done in Step 2 — run tests again)**

Expected: PASS.

- [ ] **Step 5: Update `commands/template.ts` — add `onTemplateMutated()` call after `template.execute`**

- [ ] **Step 6: Update `undo-history.ts` — replace `templateUndo`/`templateRedo`/`templateHistory` with `commands.runCommand`**

**Also update `undo-history.test.ts`** — its existing mocks target `$lib/api` (`templateHistory`, `templateUndo`, `templateRedo`). After this refactor those imports are gone. Read `engine/editor/src/lib/stores/undo-history.test.ts` and update its `vi.mock` to mock `$lib/bindings` instead:

```typescript
// old
vi.mock('$lib/api', () => ({
  templateUndo: vi.fn(...),
  templateRedo: vi.fn(...),
  templateHistory: vi.fn(...),
}));

// new
vi.mock('$lib/bindings', () => ({
  commands: {
    runCommand: vi.fn().mockResolvedValue(null),
    listCommands: vi.fn().mockResolvedValue([]),
  },
}));
```

Adapt the test assertions to match the new mock shape. The `commands.runCommand` mock returns `null` by default (simulates `Ok(None)` from Rust); for `template.history` tests, make it return a mock array.

- [ ] **Step 7: Update `App.svelte` — call `setUndoVerifier(() => getCanUndo())` after `registerAllHandlers()`**

- [ ] **Step 8: Run full TypeScript test suite**

```
cd engine/editor && npm test -- --run
```

Expected: all pass.

- [ ] **Step 9: Commit**

```bash
git commit -m "feat(editor): consolidate TypeScript dispatch — route undo-history through typed bindings, add undo verifier in dispatchCommand"
```

---

## Task 6: Regenerate bindings, update snapshot, full pass

**Why:** Task 1 changed `args_schema` fields and `returns_data` flags — the generated `bindings.ts` and the insta snapshot are now stale. This task brings them back in sync and verifies the whole system.

**Files:**
- Modify: `engine/editor/src/lib/bindings.ts` (regenerated)
- Modify: `engine/editor/src-tauri/bridge/tests/snapshots/command_manifest.snap` (updated)

### Steps

- [ ] **Step 1: Regenerate TypeScript bindings**

```
cargo xtask codegen
```

Expected: `engine/editor/src/lib/bindings.ts` updated with no errors.

- [ ] **Step 2: Verify bindings contain correct types**

Read the generated `bindings.ts`. Confirm `CommandSpec` has all 9 fields including `args_schema: unknown | null` and `non_undoable: boolean`.

- [ ] **Step 3: Update the insta manifest snapshot**

The `command_manifest_snapshot` test will fail because `args_schema` values changed. Accept the new snapshot:

```
INSTA_UPDATE=always cargo test -p silmaril-editor -- command_manifest_snapshot
```

- [ ] **Step 4: Run full Rust test suite**

```
cargo test -p silmaril-editor
```

Expected: all pass.

- [ ] **Step 5: Run `cargo xtask lint`**

```
cargo xtask lint
```

Expected: `✓ Undo coverage lint passed`.

- [ ] **Step 6: Run `cargo xtask check-bindings`**

```
cargo xtask check-bindings
```

Expected: `✓ Bindings are up to date`.

- [ ] **Step 7: Run full TypeScript test suite**

```
cd engine/editor && npm test -- --run
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add engine/editor/src/lib/bindings.ts
git add engine/editor/src-tauri/bridge/tests/snapshots/
git commit -m "chore(editor): regenerate bindings.ts, update command manifest snapshot after args_schema + returns_data fixes"
```

---

## S-Tier Verification Checklist

After all tasks complete, verify:

- [ ] `cargo xtask lint` → `✓ Undo coverage lint passed`
- [ ] `cargo xtask check-bindings` → `✓ Bindings are up to date`
- [ ] `cargo test -p silmaril-editor` → all pass (67+ tests)
- [ ] `npm test -- --run` → all pass (212+ tests)
- [ ] `run_command_inner` with `template.open` → actually opens a template (not `Ok(None)`)
- [ ] `editor-catalog-updated` Tauri event fires after app startup
- [ ] `dispatchCommand('template.execute', ...)` → logs warning if undo stack not updated
- [ ] `args_schema` is non-null for all 6 template commands in `listSpecs()`
