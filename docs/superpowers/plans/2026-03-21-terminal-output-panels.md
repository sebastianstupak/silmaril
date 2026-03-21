# Terminal & Output Panels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two dockable panels to the Silmaril editor — an interactive PTY terminal (PowerShell, multiple tabs, xterm.js) and a read-only Output/Build Log panel (hardcoded cargo buttons, custom ANSI renderer).

**Architecture:** Rust backend uses `portable-pty` for real PTY sessions (terminal tabs) and `std::process::Command` with piped stdio for cargo output. Frontend uses xterm.js for the interactive terminal and a custom ANSI SGR parser for the read-only output panel. Both panels follow the existing Wrapper + Panel component split.

**Tech Stack:** `portable-pty 0.8`, `which 4`, `uuid 1`, `@xterm/xterm ^5.5.0`, `@xterm/addon-fit ^0.10.0`, Svelte 5 runes, Tauri 2 IPC events

---

## File Map

### New files

| File | Responsibility |
|---|---|
| `engine/editor/src-tauri/src/terminal/mod.rs` | Module entry, `TerminalState` managed state, `PtySession` struct |
| `engine/editor/src-tauri/src/terminal/pty.rs` | Shell resolution, PTY spawn/write/resize/close commands |
| `engine/editor/src-tauri/src/terminal/output.rs` | `OutputState`, cargo process spawn/cancel commands |
| `engine/editor/src/lib/stores/terminal.ts` | Tab list, active tab, pendingData buffer (module singleton) |
| `engine/editor/src/lib/stores/terminal.test.ts` | Vitest unit tests for terminal store |
| `engine/editor/src/lib/stores/output.ts` | Output lines, ANSI parser, running state (module singleton) |
| `engine/editor/src/lib/stores/output.test.ts` | Vitest unit tests for output store + ANSI parser |
| `engine/editor/src/lib/docking/panels/TerminalTabs.svelte` | Tab bar UI (co-located with TerminalPanel) |
| `engine/editor/src/lib/docking/panels/TerminalPanel.svelte` | xterm.js mount, tab switching, resize handling |
| `engine/editor/src/lib/docking/panels/TerminalWrapper.svelte` | Lifecycle, IPC bridge, per-tab listener management |
| `engine/editor/src/lib/docking/panels/OutputPanel.svelte` | Buttons + ANSI output display + status bar |
| `engine/editor/src/lib/docking/panels/OutputWrapper.svelte` | Process lifecycle event bridge |

### Modified files

| File | Change |
|---|---|
| `engine/editor/Cargo.toml` | Add `portable-pty`, `which`, `uuid` |
| `engine/editor/src-tauri/lib.rs` | `pub mod terminal;`, `.manage(TerminalState::new())`, `.manage(OutputState::new())`, register 6 commands |
| `engine/editor/src/lib/docking/types.ts` | Add `terminal` and `output` to `panelRegistry` |
| `engine/editor/src/App.svelte` | Import + register `TerminalWrapper`, `OutputWrapper` |
| `engine/editor/src/lib/i18n/locales/en.ts` | Add `panel.terminal`, `panel.output`, `terminal.*`, `output.*` keys |

> **Note:** `capabilities/default.json` already contains `core:event:default` — no change needed.

---

## Task 1: Install Dependencies

**Files:**
- Modify: `engine/editor/Cargo.toml`
- Modify: `engine/editor/package.json`

- [ ] **Step 1: Add Rust dependencies**

In `engine/editor/Cargo.toml`, under `[dependencies]`, add after the `ignore = "0.4"` line:

```toml
portable-pty = "0.8"
which = "4"
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 2: Add npm dependencies**

```bash
cd engine/editor && npm install @xterm/xterm@^5.5.0 @xterm/addon-fit@^0.10.0
```

Both must be from the xterm 5.x series — do not upgrade one without the other.

- [ ] **Step 3: Verify Rust builds with new deps**

```bash
cd engine/editor && cargo check
```

Expected: compiles without error (warnings about unused imports are fine at this stage).

- [ ] **Step 4: Commit**

```bash
git add engine/editor/Cargo.toml engine/editor/package.json engine/editor/package-lock.json
git commit -m "chore(editor): add portable-pty, which, uuid, xterm.js deps"
```

---

## Task 2: Rust Terminal Module Scaffold

**Files:**
- Create: `engine/editor/src-tauri/src/terminal/mod.rs`

This task creates the `TerminalState` managed state and the `PtySession` struct that backs each PTY tab.

- [ ] **Step 1: Create `terminal/mod.rs`**

```rust
// engine/editor/src-tauri/src/terminal/mod.rs
pub mod output;
pub mod pty;

pub use output::OutputState;
pub use pty::TerminalState;
```

- [ ] **Step 2: Create the `TerminalState` in `pty.rs` (stub)**

Create `engine/editor/src-tauri/src/terminal/pty.rs` with just the types for now (commands come in Task 3):

```rust
// engine/editor/src-tauri/src/terminal/pty.rs
use std::collections::HashMap;
use std::io::Write;
use std::sync::Mutex;

use portable_pty::MasterPty;

/// One active PTY session per terminal tab.
pub struct PtySession {
    /// Master PTY — kept for resize calls.
    pub master: Box<dyn MasterPty + Send + Sync>,
    /// Write side — forwarded from master via `take_writer`.
    pub writer: Box<dyn Write + Send>,
    /// Shell process handle — for kill on close.
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

pub struct TerminalState {
    pub sessions: Mutex<HashMap<String, PtySession>>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self { sessions: Mutex::new(HashMap::new()) }
    }
}
```

> **Note on trait bounds:** `portable_pty::MasterPty` and `portable_pty::Child` may not have `Sync` bounds on all platforms. If the compiler rejects `+ Sync`, remove it — `Mutex` only requires `Send`. Adjust as needed to make the code compile.

- [ ] **Step 3: Create the `OutputState` stub in `output.rs`**

Create `engine/editor/src-tauri/src/terminal/output.rs`:

```rust
// engine/editor/src-tauri/src/terminal/output.rs
use std::process::Child;
use std::sync::{Arc, Mutex};

/// Shared state for the output panel process runner.
pub struct OutputState {
    /// Currently running child process, if any.
    /// Wrapped in Arc so it can be shared with the waiter thread.
    pub child: Arc<Mutex<Option<Child>>>,
}

impl OutputState {
    pub fn new() -> Self {
        Self { child: Arc::new(Mutex::new(None)) }
    }
}
```

- [ ] **Step 4: Verify it compiles**

```bash
cd engine/editor && cargo check
```

Expected: compiles (may warn about unused fields — fine for now).

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/src/terminal/
git commit -m "feat(editor/terminal): add TerminalState and OutputState scaffolds"
```

---

## Task 3: Rust — `terminal_new_tab` Command

**Files:**
- Modify: `engine/editor/src-tauri/src/terminal/pty.rs`

This is the core PTY spawn command. It resolves the shell, opens a PTY pair, spawns PowerShell, starts a background reader thread emitting `terminal-data:{tab_id}` events.

- [ ] **Step 1: Write the `resolve_shell` unit test**

Add to `pty.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_shell_returns_existing_path() {
        let path = resolve_shell();
        assert!(
            path.exists(),
            "resolve_shell() returned {path:?} which does not exist on this system"
        );
    }
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cd engine/editor && cargo test terminal::pty::tests::resolve_shell_returns_existing_path
```

Expected: FAIL with "not found in scope" or similar (function not yet implemented).

- [ ] **Step 3: Implement `resolve_shell`**

Add to `pty.rs`:

```rust
use std::path::PathBuf;

/// Resolves the PowerShell executable path.
/// Tries pwsh (PowerShell 7+) first, then powershell.exe (Windows built-in).
fn resolve_shell() -> PathBuf {
    for candidate in &["pwsh", "powershell.exe"] {
        if let Ok(path) = which::which(candidate) {
            return path;
        }
    }
    // Fallback — will error at spawn if not found
    PathBuf::from("powershell.exe")
}
```

- [ ] **Step 4: Run test to confirm it passes**

```bash
cd engine/editor && cargo test terminal::pty::tests::resolve_shell_returns_existing_path
```

Expected: PASS.

- [ ] **Step 5: Implement `terminal_new_tab`**

Add the following to `pty.rs` (add required imports at the top of the file):

```rust
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tauri::{AppHandle, Manager, State};

/// Spawns a new PowerShell PTY session. Returns the tab_id on success.
///
/// On failure (project path unavailable, shell not found): returns Err(String).
#[tauri::command]
pub fn terminal_new_tab(
    app: AppHandle,
    state: State<TerminalState>,
) -> Result<String, String> {
    // Get project root from editor state
    let project_root = {
        use crate::bridge::template_commands::EditorState;
        let editor = app
            .state::<std::sync::Mutex<EditorState>>()
            .lock()
            .map_err(|e| e.to_string())?;
        editor.project_path.clone().ok_or_else(|| "No project loaded".to_string())?
    };

    let tab_id = uuid::Uuid::new_v4().to_string();

    // Open PTY pair
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    // Spawn shell in project root
    let mut cmd = CommandBuilder::new(resolve_shell());
    cmd.cwd(&project_root);
    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    // Drop slave after spawn so master detects EOF on child exit (Unix).
    // On Windows/ConPTY this is also safe to drop immediately.
    drop(pair.slave);

    // Clone reader before taking writer (both come from master)
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    // Store session
    {
        let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
        sessions.insert(
            tab_id.clone(),
            PtySession { master: pair.master, writer, child },
        );
    }

    // Background reader thread: read PTY output, emit events
    let tab_id_clone = tab_id.clone();
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => {
                    app_handle
                        .emit(&format!("terminal-exit:{tab_id_clone}"), ())
                        .ok();
                    break;
                }
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).into_owned();
                    app_handle
                        .emit(&format!("terminal-data:{tab_id_clone}"), data)
                        .ok();
                }
            }
        }
    });

    Ok(tab_id)
}
```

Add `use std::io::Read;` to the imports at the top of `pty.rs`.

- [ ] **Step 6: Verify compilation**

```bash
cd engine/editor && cargo check
```

Expected: compiles. If `pair.master` causes a "moved" error after `try_clone_reader`/`take_writer`, use `pair.master` last (store it last in `PtySession`).

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src-tauri/src/terminal/pty.rs
git commit -m "feat(editor/terminal): implement terminal_new_tab with portable-pty"
```

---

## Task 4: Rust — `terminal_write`, `terminal_resize`, `terminal_close_tab`

**Files:**
- Modify: `engine/editor/src-tauri/src/terminal/pty.rs`

- [ ] **Step 1: Implement `terminal_write`**

```rust
/// Writes keystroke data to the PTY session.
/// Returns Err silently if session already closed (tab close race — expected).
#[tauri::command]
pub fn terminal_write(
    tab_id: String,
    data: String,
    state: State<TerminalState>,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if let Some(session) = sessions.get_mut(&tab_id) {
        session.writer.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

- [ ] **Step 2: Implement `terminal_resize`**

```rust
/// Resizes the PTY for the given tab.
#[tauri::command]
pub fn terminal_resize(
    tab_id: String,
    cols: u16,
    rows: u16,
    state: State<TerminalState>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if let Some(session) = sessions.get(&tab_id) {
        session
            .master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

- [ ] **Step 3: Implement `terminal_close_tab`**

```rust
/// Kills the shell process and removes the session from the map.
/// No-op if tab already closed.
#[tauri::command]
pub fn terminal_close_tab(
    tab_id: String,
    state: State<TerminalState>,
) -> () {
    let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(mut session) = sessions.remove(&tab_id) {
        let _ = session.child.kill();
    }
}
```

- [ ] **Step 4: Compile check**

```bash
cd engine/editor && cargo check
```

Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/src/terminal/pty.rs
git commit -m "feat(editor/terminal): implement terminal_write, terminal_resize, terminal_close_tab"
```

---

## Task 5: Rust — `output_run` and `output_cancel`

**Files:**
- Modify: `engine/editor/src-tauri/src/terminal/output.rs`

- [ ] **Step 1: Write unit test for `output_run` with a real process**

Add to `output.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a real AppHandle which is hard to construct in unit tests.
    // The actual command behavior is verified via integration testing (run the editor).
    // We test OutputState directly here.

    #[test]
    fn output_state_starts_empty() {
        let state = OutputState::new();
        let guard = state.child.lock().unwrap();
        assert!(guard.is_none());
    }

    #[test]
    fn output_state_can_store_and_take_child() {
        let state = OutputState::new();
        // Spawn a trivial process to get a real Child
        let child = std::process::Command::new("echo")
            .arg("hello")
            .spawn()
            .expect("echo must be available");

        {
            let mut guard = state.child.lock().unwrap();
            *guard = Some(child);
        }

        let taken = state.child.lock().unwrap().take();
        assert!(taken.is_some());

        let taken_again = state.child.lock().unwrap().take();
        assert!(taken_again.is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they pass (OutputState is already implemented)**

```bash
cd engine/editor && cargo test terminal::output::tests
```

Expected: PASS (both tests).

- [ ] **Step 3: Implement `output_run`**

```rust
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Manager, State};

/// Spawns a cargo command in the project root with piped stdio.
///
/// Returns Err("already running") if a process is already active.
/// Streams stdout/stderr lines as `output-data` events.
/// Emits `output-exit` when the process finishes.
#[tauri::command]
pub fn output_run(
    command: String,
    args: Vec<String>,
    app: AppHandle,
    state: State<OutputState>,
) -> Result<(), String> {
    // Check not already running
    {
        let guard = state.child.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("already running".into());
        }
    }

    // Get project root
    let project_root = {
        use crate::bridge::template_commands::EditorState;
        let editor = app
            .state::<std::sync::Mutex<EditorState>>()
            .lock()
            .map_err(|e| e.to_string())?;
        editor.project_path.clone().ok_or_else(|| "No project loaded".to_string())?
    };

    // Spawn process
    let mut child = Command::new(&command)
        .args(&args)
        .current_dir(&project_root)
        .env("CARGO_TERM_COLOR", "always")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Store child in shared Arc for waiter thread + output_cancel
    let child_arc = Arc::clone(&state.child);
    {
        let mut guard = child_arc.lock().map_err(|e| e.to_string())?;
        *guard = Some(child);
    }

    // Stdout reader thread
    let app2 = app.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    app2.emit("output-data", serde_json::json!({
                        "line": l,
                        "stream": "stdout"
                    })).ok();
                }
                Err(_) => break,
            }
        }
    });

    // Stderr reader thread
    let app3 = app.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    app3.emit("output-data", serde_json::json!({
                        "line": l,
                        "stream": "stderr"
                    })).ok();
                }
                Err(_) => break,
            }
        }
    });

    // Waiter thread: polls child exit, emits output-exit when done
    let waiter_arc = Arc::clone(&state.child);
    let app4 = app.clone();
    std::thread::spawn(move || {
        loop {
            {
                let mut guard = waiter_arc.lock().unwrap();
                if guard.is_none() {
                    // output_cancel took the child — it will emit output-exit
                    break;
                }
                if let Some(c) = guard.as_mut() {
                    match c.try_wait() {
                        Ok(Some(status)) => {
                            *guard = None;
                            drop(guard);
                            app4.emit("output-exit", serde_json::json!({
                                "code": status.code(),
                                "cancelled": false
                            })).ok();
                            break;
                        }
                        Ok(None) => {} // still running
                        Err(_) => {
                            // Process wait failed — emit exit so frontend doesn't stay stuck
                            *guard = None;
                            drop(guard);
                            app4.emit("output-exit", serde_json::json!({
                                "code": null,
                                "cancelled": false
                            })).ok();
                            return;
                        }
                    }
                }
            } // lock released here
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });

    Ok(())
}
```

- [ ] **Step 4: Implement `output_cancel`**

```rust
/// Kills the running cargo process if any.
/// No-op if nothing is running (does NOT emit output-exit).
/// Emits output-exit with cancelled=true only if a process was actually killed.
#[tauri::command]
pub fn output_cancel(
    app: AppHandle,
    state: State<OutputState>,
) -> () {
    let child_opt = {
        let mut guard = state.child.lock().unwrap_or_else(|e| e.into_inner());
        guard.take()
    };
    if let Some(mut child) = child_opt {
        let _ = child.kill();
        app.emit("output-exit", serde_json::json!({
            "code": null,
            "cancelled": true
        })).ok();
    }
}
```

- [ ] **Step 5: Compile check**

```bash
cd engine/editor && cargo check
```

Expected: compiles.

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src-tauri/src/terminal/output.rs
git commit -m "feat(editor/terminal): implement output_run and output_cancel"
```

---

## Task 6: Register Commands in `lib.rs`

**Files:**
- Modify: `engine/editor/src-tauri/lib.rs`

- [ ] **Step 1: Add `pub mod terminal;` and imports**

In `lib.rs`, after the existing `pub mod world;` line, add:

```rust
pub mod terminal;
```

After the existing `use file_explorer::{...}` block, add:

```rust
use terminal::{
    TerminalState, OutputState,
    pty::{terminal_new_tab, terminal_write, terminal_resize, terminal_close_tab},
    output::{output_run, output_cancel},
};
```

- [ ] **Step 2: Add `.manage()` calls**

In the `tauri::Builder::default()` chain (after the existing `.manage(ComponentSchemaState(...))` line), add:

```rust
.manage(TerminalState::new())
.manage(OutputState::new())
```

- [ ] **Step 3: Register the 6 commands in `invoke_handler!`**

In the `tauri::generate_handler![...]` list, add before the closing `]`:

```rust
terminal_new_tab,
terminal_write,
terminal_resize,
terminal_close_tab,
output_run,
output_cancel,
```

- [ ] **Step 4: Full compile check**

```bash
cd engine/editor && cargo check
```

Expected: compiles without errors.

- [ ] **Step 5: Run Rust tests**

```bash
cd engine/editor && cargo test
```

Expected: all existing tests pass + the 3 new tests from output.rs pass.

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src-tauri/lib.rs
git commit -m "feat(editor/terminal): register terminal and output commands in Tauri"
```

---

## Task 7: Frontend — `terminal.ts` Store

**Files:**
- Create: `engine/editor/src/lib/stores/terminal.ts`
- Create: `engine/editor/src/lib/stores/terminal.test.ts`

- [ ] **Step 1: Write the failing tests first**

Create `engine/editor/src/lib/stores/terminal.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import {
  getTerminalState,
  addTab,
  closeTab,
  setActiveTab,
  markExited,
  appendTerminalData,
  drainTerminalData,
} from './terminal';

// Reset module state between tests by re-importing or resetting via a helper.
// Since the store is a module singleton, we need to reset it.
// We'll add a resetTerminalState export for testing only.
import { _resetForTest } from './terminal';

beforeEach(() => _resetForTest());

describe('addTab', () => {
  it('adds a tab and sets it as active', () => {
    addTab('tab-1');
    const s = getTerminalState();
    expect(s.tabs).toHaveLength(1);
    expect(s.tabs[0].id).toBe('tab-1');
    expect(s.tabs[0].title).toBe('Terminal 1');
    expect(s.tabs[0].exited).toBe(false);
    expect(s.activeTabId).toBe('tab-1');
  });

  it('increments title counter per call', () => {
    addTab('tab-1');
    addTab('tab-2');
    const s = getTerminalState();
    expect(s.tabs[0].title).toBe('Terminal 1');
    expect(s.tabs[1].title).toBe('Terminal 2');
  });

  it('new tab becomes active', () => {
    addTab('tab-1');
    addTab('tab-2');
    expect(getTerminalState().activeTabId).toBe('tab-2');
  });
});

describe('closeTab', () => {
  it('removes tab when multiple tabs exist', () => {
    addTab('tab-1');
    addTab('tab-2');
    closeTab('tab-1');
    const s = getTerminalState();
    expect(s.tabs).toHaveLength(1);
    expect(s.tabs[0].id).toBe('tab-2');
  });

  it('switches active to previous tab when closing active', () => {
    addTab('tab-1');
    addTab('tab-2');
    closeTab('tab-2');
    expect(getTerminalState().activeTabId).toBe('tab-1');
  });

  it('does NOT close the last non-exited tab', () => {
    addTab('tab-1');
    closeTab('tab-1');
    expect(getTerminalState().tabs).toHaveLength(1);
  });

  it('CAN close the last exited tab', () => {
    addTab('tab-1');
    markExited('tab-1');
    closeTab('tab-1');
    expect(getTerminalState().tabs).toHaveLength(0);
  });
});

describe('markExited', () => {
  it('sets exited=true without removing tab', () => {
    addTab('tab-1');
    markExited('tab-1');
    const tab = getTerminalState().tabs.find(t => t.id === 'tab-1');
    expect(tab?.exited).toBe(true);
    expect(getTerminalState().tabs).toHaveLength(1);
  });
});

describe('pendingData', () => {
  it('appendTerminalData accumulates data and drainTerminalData clears it', () => {
    addTab('tab-1');
    appendTerminalData('tab-1', 'hello ');
    appendTerminalData('tab-1', 'world');
    expect(drainTerminalData('tab-1')).toBe('hello world');
    expect(drainTerminalData('tab-1')).toBe('');
  });
});
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd engine/editor && npm test -- --run terminal.test
```

Expected: FAIL (module not found).

- [ ] **Step 3: Implement `terminal.ts`**

Create `engine/editor/src/lib/stores/terminal.ts`:

```typescript
// Terminal panel store — manages tab list and pending PTY data.
// Follows the module-level singleton pattern from console.ts.

export interface TerminalTab {
  id: string;
  title: string;   // "Terminal 1", "Terminal 2", etc.
  exited: boolean; // true after terminal-exit event; tab stays visible, dimmed
}

export interface TerminalState {
  tabs: TerminalTab[];
  activeTabId: string | null;
  pendingData: Map<string, string>; // per-tab unread PTY data buffer
}

let state: TerminalState = { tabs: [], activeTabId: null, pendingData: new Map() };
let listeners: (() => void)[] = [];
let tabCounter = 0; // monotonically increasing, never resets

function notify() {
  listeners.forEach(fn => fn());
}

export function getTerminalState(): TerminalState {
  return state;
}

export function subscribeTerminal(fn: () => void): () => void {
  listeners.push(fn);
  return () => { listeners = listeners.filter(l => l !== fn); };
}

export function addTab(id: string): void {
  tabCounter++;
  state.tabs = [...state.tabs, { id, title: `Terminal ${tabCounter}`, exited: false }];
  state.activeTabId = id;
  notify();
}

export function closeTab(id: string): void {
  const tab = state.tabs.find(t => t.id === id);
  if (!tab) return;

  // Block closing the last non-exited tab
  if (state.tabs.length === 1 && !tab.exited) return;

  const idx = state.tabs.indexOf(tab);
  state.tabs = state.tabs.filter(t => t.id !== id);
  state.pendingData.delete(id);

  // If we closed the active tab, switch to the nearest remaining tab
  if (state.activeTabId === id) {
    if (state.tabs.length === 0) {
      state.activeTabId = null;
    } else {
      const newIdx = Math.max(0, idx - 1);
      state.activeTabId = state.tabs[newIdx].id;
    }
  }
  notify();
}

export function setActiveTab(id: string): void {
  state.activeTabId = id;
  notify();
}

export function markExited(id: string): void {
  state.tabs = state.tabs.map(t => t.id === id ? { ...t, exited: true } : t);
  notify();
}

export function appendTerminalData(tabId: string, data: string): void {
  const existing = state.pendingData.get(tabId) ?? '';
  state.pendingData.set(tabId, existing + data);
  notify();
}

export function drainTerminalData(tabId: string): string {
  const data = state.pendingData.get(tabId) ?? '';
  state.pendingData.delete(tabId);
  return data;
}

/** For testing only — resets all module-level state. */
export function _resetForTest(): void {
  state = { tabs: [], activeTabId: null, pendingData: new Map() };
  listeners = [];
  tabCounter = 0;
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd engine/editor && npm test -- --run terminal.test
```

Expected: all 9 tests pass.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src/lib/stores/terminal.ts engine/editor/src/lib/stores/terminal.test.ts
git commit -m "feat(editor/terminal): add terminal store with tab management and pendingData"
```

---

## Task 8: Frontend — `output.ts` Store + ANSI Parser

**Files:**
- Create: `engine/editor/src/lib/stores/output.ts`
- Create: `engine/editor/src/lib/stores/output.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `engine/editor/src/lib/stores/output.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from 'vitest';
import {
  getOutputState,
  appendLine,
  setRunning,
  setFinished,
  clearOutput,
  _resetForTest,
} from './output';

beforeEach(() => _resetForTest());

describe('ANSI parser — standard colors', () => {
  it('parses red foreground', () => {
    appendLine('\x1b[31mhello\x1b[0m', 'stdout');
    const spans = getOutputState().lines[0].spans;
    expect(spans[0].text).toBe('hello');
    expect(spans[0].color).not.toBeNull();
    expect(spans[0].bold).toBe(false);
  });

  it('parses all standard foreground colors (30-37)', () => {
    for (let i = 30; i <= 37; i++) {
      _resetForTest();
      appendLine(`\x1b[${i}mtext\x1b[0m`, 'stdout');
      const span = getOutputState().lines[0].spans[0];
      expect(span.text).toBe('text');
      expect(span.color).not.toBeNull();
    }
  });

  it('parses bright foreground colors (90-97)', () => {
    for (let i = 90; i <= 97; i++) {
      _resetForTest();
      appendLine(`\x1b[${i}mtext\x1b[0m`, 'stdout');
      const span = getOutputState().lines[0].spans[0];
      expect(span.text).toBe('text');
      expect(span.color).not.toBeNull();
    }
  });

  it('parses bold', () => {
    appendLine('\x1b[1mhello\x1b[0m', 'stdout');
    expect(getOutputState().lines[0].spans[0].bold).toBe(true);
  });

  it('resets on code 0', () => {
    appendLine('\x1b[31mred\x1b[0mnormal', 'stdout');
    const spans = getOutputState().lines[0].spans;
    expect(spans[0].color).not.toBeNull();
    expect(spans[1].color).toBeNull();
    expect(spans[1].bold).toBe(false);
  });

  it('handles mixed bold + color', () => {
    appendLine('\x1b[1;32mbold green\x1b[0m', 'stdout');
    const span = getOutputState().lines[0].spans[0];
    expect(span.bold).toBe(true);
    expect(span.color).not.toBeNull();
  });

  it('plain text has null color and false bold', () => {
    appendLine('plain text', 'stdout');
    const span = getOutputState().lines[0].spans[0];
    expect(span.text).toBe('plain text');
    expect(span.color).toBeNull();
    expect(span.bold).toBe(false);
  });
});

describe('appendLine', () => {
  it('stores raw string and stream discriminator', () => {
    appendLine('\x1b[31merror\x1b[0m', 'stderr');
    const line = getOutputState().lines[0];
    expect(line.raw).toBe('\x1b[31merror\x1b[0m');
    expect(line.stream).toBe('stderr');
  });
});

describe('state transitions', () => {
  it('setRunning sets running=true and command', () => {
    setRunning('cargo build');
    const s = getOutputState();
    expect(s.running).toBe(true);
    expect(s.command).toBe('cargo build');
  });

  it('setFinished sets running=false and exitCode', () => {
    setRunning('cargo build');
    setFinished(0, false);
    const s = getOutputState();
    expect(s.running).toBe(false);
    expect(s.exitCode).toBe(0);
    expect(s.cancelled).toBe(false);
  });

  it('clearOutput resets lines, exitCode, cancelled, command but does not touch running', () => {
    setRunning('cargo build');
    appendLine('hello', 'stdout');
    clearOutput();
    const s = getOutputState();
    expect(s.lines).toHaveLength(0);
    expect(s.exitCode).toBeNull();
    expect(s.cancelled).toBe(false);
    expect(s.command).toBeNull();
    // running stays true — clearOutput is safe mid-build
    expect(s.running).toBe(true);
  });
});
```

- [ ] **Step 2: Run to confirm failure**

```bash
cd engine/editor && npm test -- --run output.test
```

Expected: FAIL (module not found).

- [ ] **Step 3: Implement `output.ts`**

Create `engine/editor/src/lib/stores/output.ts`:

```typescript
// Output panel store — manages cargo build log lines and running state.
// Follows the module-level singleton pattern from console.ts.

export interface OutputLine {
  raw: string;                           // original with ANSI codes
  stream: 'stdout' | 'stderr';
  spans: Array<{ text: string; color: string | null; bold: boolean }>;
}

export interface OutputState {
  lines: OutputLine[];
  running: boolean;
  exitCode: number | null;
  cancelled: boolean;
  command: string | null;
}

let state: OutputState = {
  lines: [],
  running: false,
  exitCode: null,
  cancelled: false,
  command: null,
};
let listeners: (() => void)[] = [];

function notify() {
  listeners.forEach(fn => fn());
}

// 16-color palette (dark theme): indexed by color code offset from base
// Colors 30-37 use indices 0-7, colors 90-97 use indices 8-15
const PALETTE: string[] = [
  '#1e1e1e', // 30 black (dark)
  '#cc3e28', // 31 red
  '#57a64a', // 32 green
  '#d7ba7d', // 33 yellow
  '#569cd6', // 34 blue
  '#c586c0', // 35 magenta
  '#9cdcfe', // 36 cyan
  '#d4d4d4', // 37 white
  '#666666', // 90 bright black (gray)
  '#f44747', // 91 bright red
  '#b5cea8', // 92 bright green
  '#dcdcaa', // 93 bright yellow
  '#4ec9b0', // 94 bright blue/cyan
  '#d670d6', // 95 bright magenta
  '#87d5f5', // 96 bright cyan
  '#ffffff', // 97 bright white
];

interface Attrs { color: string | null; bold: boolean }

/** Parses ANSI SGR escape sequences. Handles colors 30-37, 90-97, bold (1), reset (0). */
function parseAnsi(raw: string): Array<{ text: string; color: string | null; bold: boolean }> {
  const spans: Array<{ text: string; color: string | null; bold: boolean }> = [];
  // Matches SGR sequences like \x1b[1;32m or \x1b[0m
  const re = /\x1b\[([0-9;]*)m/g;
  let cur: Attrs = { color: null, bold: false };
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = re.exec(raw)) !== null) {
    // Text before this escape sequence
    if (match.index > lastIndex) {
      const text = raw.slice(lastIndex, match.index);
      if (text) spans.push({ ...cur, text });
    }
    lastIndex = re.lastIndex;

    // Parse codes (may be "1;32" or "0" or "")
    const codes = match[1].split(';').map(Number);
    const next: Attrs = { ...cur };
    for (const code of codes) {
      if (code === 0 || match[1] === '') {
        next.color = null;
        next.bold = false;
      } else if (code === 1) {
        next.bold = true;
      } else if (code >= 30 && code <= 37) {
        next.color = PALETTE[code - 30];
      } else if (code >= 90 && code <= 97) {
        next.color = PALETTE[code - 90 + 8];
      }
      // Cursor movement codes (e.g., 2J, K) — ignored (no text output)
    }
    cur = next;
  }

  // Remaining text after last escape
  if (lastIndex < raw.length) {
    const text = raw.slice(lastIndex);
    if (text) spans.push({ ...cur, text });
  }

  return spans.length > 0 ? spans : [{ text: raw, color: null, bold: false }];
}

export function getOutputState(): OutputState {
  return state;
}

export function subscribeOutput(fn: () => void): () => void {
  listeners.push(fn);
  return () => { listeners = listeners.filter(l => l !== fn); };
}

export function appendLine(raw: string, stream: 'stdout' | 'stderr'): void {
  const spans = parseAnsi(raw);
  state.lines = [...state.lines, { raw, stream, spans }];
  notify();
}

export function setRunning(cmd: string): void {
  state.running = true;
  state.command = cmd;
  state.exitCode = null;
  state.cancelled = false;
  notify();
}

export function setFinished(code: number | null, cancelled: boolean): void {
  state.running = false;
  state.exitCode = code;
  state.cancelled = cancelled;
  notify();
}

export function clearOutput(): void {
  state.lines = [];
  state.exitCode = null;
  state.cancelled = false;
  state.command = null;
  // Note: does NOT change `running` — safe to call mid-build
  notify();
}

/** For testing only. */
export function _resetForTest(): void {
  state = { lines: [], running: false, exitCode: null, cancelled: false, command: null };
  listeners = [];
}
```

- [ ] **Step 4: Run tests to confirm they pass**

```bash
cd engine/editor && npm test -- --run output.test
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src/lib/stores/output.ts engine/editor/src/lib/stores/output.test.ts
git commit -m "feat(editor/terminal): add output store with ANSI SGR parser"
```

---

## Task 9: `TerminalTabs.svelte`

**Files:**
- Create: `engine/editor/src/lib/docking/panels/TerminalTabs.svelte`

Co-located with TerminalPanel — not a shared component.

- [ ] **Step 1: Create the component**

```svelte
<!-- engine/editor/src/lib/docking/panels/TerminalTabs.svelte -->
<script lang="ts">
  import { t } from '$lib/i18n';
  import type { TerminalTab } from '$lib/stores/terminal';

  let {
    tabs,
    activeTabId,
    onNewTab,
    onCloseTab,
    onSelectTab,
  }: {
    tabs: TerminalTab[];
    activeTabId: string | null;
    onNewTab: () => void;
    onCloseTab: (id: string) => void;
    onSelectTab: (id: string) => void;
  } = $props();

  /** Last non-exited tab cannot be closed */
  function canClose(tab: TerminalTab): boolean {
    if (tab.exited) return true;
    const liveCount = tabs.filter(t => !t.exited).length;
    return liveCount > 1;
  }
</script>

<div class="tab-bar" role="tablist">
  {#each tabs as tab (tab.id)}
    <button
      class="tab"
      class:active={tab.id === activeTabId}
      class:exited={tab.exited}
      role="tab"
      aria-selected={tab.id === activeTabId}
      onclick={() => onSelectTab(tab.id)}
    >
      <span class="tab-label">{tab.title}</span>
      {#if canClose(tab)}
        <span
          class="tab-close"
          role="button"
          aria-label={t('terminal.close_tab')}
          tabindex="0"
          onclick={e => { e.stopPropagation(); onCloseTab(tab.id); }}
          onkeydown={e => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); onCloseTab(tab.id); } }}
        >×</span>
      {/if}
    </button>
  {/each}
  <button class="tab-new" aria-label={t('terminal.new_tab')} onclick={onNewTab}>+</button>
</div>

<style>
  .tab-bar {
    display: flex;
    align-items: stretch;
    background: var(--color-bgPanel, #1e1e1e);
    border-bottom: 1px solid var(--color-border, #333);
    overflow-x: auto;
    height: 32px;
    flex-shrink: 0;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 10px;
    font-size: 12px;
    color: var(--color-text, #ccc);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .tab:hover { background: var(--color-bgHover, #2a2a2a); }
  .tab.active { border-bottom-color: var(--color-accent, #569cd6); color: #fff; }
  .tab.exited { opacity: 0.5; }
  .tab-close {
    opacity: 0.6;
    font-size: 14px;
    line-height: 1;
    padding: 0 2px;
    border-radius: 2px;
    cursor: pointer;
  }
  .tab-close:hover { opacity: 1; background: var(--color-bgHover, #2a2a2a); }
  .tab-new {
    padding: 0 10px;
    font-size: 16px;
    color: var(--color-text, #ccc);
    background: transparent;
    border: none;
    cursor: pointer;
    align-self: center;
  }
  .tab-new:hover { color: #fff; }
</style>
```

- [ ] **Step 2: Verify it compiles (no Svelte errors)**

```bash
cd engine/editor && npm run build 2>&1 | head -20
```

Expected: no errors related to TerminalTabs.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/docking/panels/TerminalTabs.svelte
git commit -m "feat(editor/terminal): add TerminalTabs component"
```

---

## Task 10: `TerminalPanel.svelte`

**Files:**
- Create: `engine/editor/src/lib/docking/panels/TerminalPanel.svelte`

Owns the Map of xterm.js instances. Drains pendingData from the store and writes to the correct terminal.

- [ ] **Step 1: Create the component**

```svelte
<!-- engine/editor/src/lib/docking/panels/TerminalPanel.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { t } from '$lib/i18n';
  import type { TerminalState } from '$lib/stores/terminal';
  import { drainTerminalData, setActiveTab } from '$lib/stores/terminal';
  import TerminalTabs from './TerminalTabs.svelte';

  let { state, onNewTab, onCloseTab }: {
    state: TerminalState;
    onNewTab: () => void;
    onCloseTab: (id: string) => void;
  } = $props();

  // Map of tabId → { terminal, fitAddon, containerEl }
  let xtermInstances = new Map<string, { term: any; fit: any; el: HTMLDivElement }>();
  let containerRef: HTMLDivElement;
  let resizeObserver: ResizeObserver | null = null;
  let loadError: string | null = $state(null);

  // xterm.js is imported dynamically to avoid SSR issues
  let XTermModule: { Terminal: any; FitAddon: any } | null = null;

  const XTERM_THEME = {
    background: '#1e1e1e',
    foreground: '#d4d4d4',
    cursor: '#d4d4d4',
    black: '#1e1e1e', red: '#cc3e28', green: '#57a64a', yellow: '#d7ba7d',
    blue: '#569cd6', magenta: '#c586c0', cyan: '#9cdcfe', white: '#d4d4d4',
    brightBlack: '#666666', brightRed: '#f44747', brightGreen: '#b5cea8',
    brightYellow: '#dcdcaa', brightBlue: '#4ec9b0', brightMagenta: '#d670d6',
    brightCyan: '#87d5f5', brightWhite: '#ffffff',
  };

  onMount(async () => {
    try {
      const [xtermPkg, fitPkg] = await Promise.all([
        import('@xterm/xterm'),
        import('@xterm/addon-fit'),
      ]);
      XTermModule = { Terminal: xtermPkg.Terminal, FitAddon: fitPkg.FitAddon };
    } catch (e) {
      loadError = `Failed to load xterm.js: ${e}`;
      return;
    }

    // Create terminals for any tabs that already exist (e.g., restored session)
    for (const tab of state.tabs) {
      if (!xtermInstances.has(tab.id)) {
        createTerminal(tab.id);
      }
    }

    resizeObserver = new ResizeObserver(() => fitAll());
    if (containerRef) resizeObserver.observe(containerRef);
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    for (const [, inst] of xtermInstances) {
      inst.term.dispose();
    }
    xtermInstances.clear();
  });

  function createTerminal(tabId: string): void {
    if (!XTermModule || !containerRef) return;

    const el = document.createElement('div');
    el.style.cssText = 'position:absolute;inset:0;display:none;';
    containerRef.appendChild(el);

    const term = new XTermModule.Terminal({
      theme: XTERM_THEME,
      fontFamily: 'Consolas, "Courier New", monospace',
      fontSize: 13,
      cursorBlink: true,
    });
    const fit = new XTermModule.FitAddon();
    term.loadAddon(fit);
    term.open(el);
    fit.fit();

    term.onData((data: string) => {
      invoke('terminal_write', { tabId, data }).catch(() => {});
    });

    xtermInstances.set(tabId, { term, fit, el });
    updateVisibility();
  }

  function updateVisibility(): void {
    for (const [id, inst] of xtermInstances) {
      inst.el.style.display = id === state.activeTabId ? 'block' : 'none';
    }
  }

  function fitAll(): void {
    for (const [tabId, inst] of xtermInstances) {
      try {
        inst.fit.fit();
        invoke('terminal_resize', {
          tabId,
          cols: inst.term.cols,
          rows: inst.term.rows,
        }).catch(() => {});
      } catch { /* ignore fit errors */ }
    }
  }

  // Reactive: when state.tabs changes, create terminals for new tabs
  $effect(() => {
    if (!XTermModule) return;
    for (const tab of state.tabs) {
      if (!xtermInstances.has(tab.id)) {
        createTerminal(tab.id);
      }
    }
    updateVisibility();
  });

  // Reactive: drain pending data and write to appropriate terminal
  $effect(() => {
    for (const tab of state.tabs) {
      const data = drainTerminalData(tab.id);
      if (data) {
        xtermInstances.get(tab.id)?.term.write(data);
      }
    }
  });

  // Reactive: update visibility when active tab changes
  $effect(() => {
    void state.activeTabId; // track dependency
    updateVisibility();
  });
</script>

<div class="terminal-panel">
  {#if state.tabs.length === 0}
    <div class="placeholder">{t('placeholder.no_project')}</div>
  {:else}
    <TerminalTabs
      tabs={state.tabs}
      activeTabId={state.activeTabId}
      {onNewTab}
      onCloseTab={onCloseTab}
      onSelectTab={id => setActiveTab(id)}
    />
    {#if loadError}
      <div class="load-error">{loadError}</div>
    {:else}
      <div class="xterm-container" bind:this={containerRef}></div>
    {/if}
  {/if}
</div>

<style>
  .terminal-panel {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background: #1e1e1e;
  }
  .xterm-container {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
  .placeholder, .load-error {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-textMuted, #666);
    font-size: 13px;
  }
  .load-error { color: var(--color-error, #f44747); }
</style>
```

- [ ] **Step 2: Compile check**

```bash
cd engine/editor && npm run build 2>&1 | head -30
```

Expected: no TypeScript/Svelte errors for TerminalPanel.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/docking/panels/TerminalPanel.svelte
git commit -m "feat(editor/terminal): add TerminalPanel with xterm.js integration"
```

---

## Task 11: `TerminalWrapper.svelte`

**Files:**
- Create: `engine/editor/src/lib/docking/panels/TerminalWrapper.svelte`

Handles lifecycle, opens the first PTY tab on mount, manages per-tab IPC listeners.

- [ ] **Step 1: Create the component**

```svelte
<!-- engine/editor/src/lib/docking/panels/TerminalWrapper.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import TerminalPanel from './TerminalPanel.svelte';
  import {
    getTerminalState,
    subscribeTerminal,
    addTab,
    closeTab,
    setActiveTab,
    markExited,
    appendTerminalData,
    type TerminalState,
  } from '$lib/stores/terminal';
  import { logError } from '$lib/stores/console';

  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  let state: TerminalState = $state(getTerminalState());
  let unsubscribe: (() => void) | null = null;
  // Per-tab unlisten functions
  const unlisteners = new Map<string, Array<() => void>>();

  async function setupTabListeners(tabId: string): Promise<void> {
    const unlistenData = await listen<string>(`terminal-data:${tabId}`, e => {
      appendTerminalData(tabId, e.payload);
    });
    const unlistenExit = await listen(`terminal-exit:${tabId}`, () => {
      markExited(tabId);
      cleanupTabListeners(tabId);
    });
    unlisteners.set(tabId, [unlistenData, unlistenExit]);
  }

  function cleanupTabListeners(tabId: string): void {
    unlisteners.get(tabId)?.forEach(fn => fn());
    unlisteners.delete(tabId);
  }

  async function openNewTab(): Promise<void> {
    try {
      const tabId = await invoke<string>('terminal_new_tab');
      addTab(tabId);
      await setupTabListeners(tabId);
    } catch (e) {
      logError(`Terminal: failed to open new tab — ${e}`);
    }
  }

  async function handleCloseTab(id: string): Promise<void> {
    cleanupTabListeners(id);
    try { await invoke('terminal_close_tab', { tabId: id }); } catch { /* ignore */ }
    closeTab(id);
  }

  onMount(async () => {
    unsubscribe = subscribeTerminal(() => {
      state = getTerminalState();
    });

    if (!isTauri) return;

    // Check if a project is loaded before trying to open a PTY
    try {
      const editorState = await invoke<{ project_path?: string }>('get_editor_state');
      if (editorState.project_path) {
        await openNewTab();
      }
    } catch (e) {
      logError(`TerminalWrapper: could not get editor state — ${e}`);
    }
  });

  onDestroy(async () => {
    unsubscribe?.();
    // Clean up all listeners and close all open tabs
    const tabIds = [...unlisteners.keys()];
    for (const tabId of tabIds) {
      cleanupTabListeners(tabId);
      if (isTauri) {
        try { await invoke('terminal_close_tab', { tabId }); } catch { /* ignore */ }
      }
    }
  });
</script>

<div class="panel-opaque">
  <TerminalPanel
    {state}
    onNewTab={openNewTab}
    onCloseTab={handleCloseTab}
  />
</div>

<style>
  .panel-opaque {
    width: 100%;
    height: 100%;
    background: var(--color-bgPanel, #1e1e1e);
  }
</style>
```

- [ ] **Step 2: Compile check**

```bash
cd engine/editor && npm run build 2>&1 | head -30
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/docking/panels/TerminalWrapper.svelte
git commit -m "feat(editor/terminal): add TerminalWrapper IPC bridge"
```

---

## Task 12: `OutputPanel.svelte`

**Files:**
- Create: `engine/editor/src/lib/docking/panels/OutputPanel.svelte`

Renders cargo command buttons, ANSI output lines, and a status bar. Auto-scrolls to bottom.

- [ ] **Step 1: Create the component**

```svelte
<!-- engine/editor/src/lib/docking/panels/OutputPanel.svelte -->
<script lang="ts">
  import { t } from '$lib/i18n';
  import type { OutputState } from '$lib/stores/output';

  let { state, onRun, onCancel, onClear }: {
    state: OutputState;
    onRun: (command: string, args: string[]) => void;
    onCancel: () => void;
    onClear: () => void;
  } = $props();

  const COMMANDS = [
    { key: 'build',  cmd: 'cargo', args: ['build'],  labelKey: 'output.build'  },
    { key: 'test',   cmd: 'cargo', args: ['test'],   labelKey: 'output.test'   },
    { key: 'run',    cmd: 'cargo', args: ['run'],    labelKey: 'output.run'    },
    { key: 'clippy', cmd: 'cargo', args: ['clippy'], labelKey: 'output.clippy' },
  ] as const;

  let outputEl: HTMLDivElement;
  let userScrolledUp = false;

  function onScroll() {
    if (!outputEl) return;
    const { scrollTop, scrollHeight, clientHeight } = outputEl;
    userScrolledUp = scrollTop + clientHeight < scrollHeight - 20;
  }

  $effect(() => {
    // Scroll to bottom on new lines unless user scrolled up
    void state.lines.length;
    if (!userScrolledUp && outputEl) {
      outputEl.scrollTop = outputEl.scrollHeight;
    }
  });
</script>

<div class="output-panel">
  <!-- Button row -->
  <div class="toolbar">
    {#each COMMANDS as c}
      <button
        class="btn"
        disabled={state.running}
        onclick={() => onRun(c.cmd, [...c.args])}
      >{t(c.labelKey)}</button>
    {/each}
    {#if state.running}
      <button class="btn btn-cancel" onclick={onCancel}>{t('output.cancel')}</button>
    {/if}
    <button class="btn btn-clear" onclick={onClear}>{t('output.clear')}</button>
  </div>

  <!-- Output area -->
  <div
    class="output-area"
    role="log"
    aria-live="polite"
    bind:this={outputEl}
    onscroll={onScroll}
  >
    {#if state.lines.length === 0 && !state.running}
      <div class="placeholder">{t('output.empty')}</div>
    {:else}
      {#each state.lines as line, i (i)}
        <div class="output-line">
          {#each line.spans as span}
            <span
              style={[
                span.color ? `color:${span.color}` : '',
                span.bold ? 'font-weight:bold' : '',
              ].filter(Boolean).join(';')}
            >{span.text}</span>
          {/each}
        </div>
      {/each}
    {/if}
  </div>

  <!-- Status bar -->
  <div class="status-bar">
    {#if state.running}
      <span class="status-running">⟳ {state.command} {t('output.running')}</span>
    {:else if state.cancelled}
      <span class="status-cancelled">⊘ {t('output.cancelled')}</span>
    {:else if state.exitCode === 0 && state.command}
      <span class="status-ok">✓ {t('output.exit_ok')}</span>
    {:else if state.exitCode !== null && state.exitCode !== 0}
      <span class="status-err">✗ {t('output.exit_err')} (exit {state.exitCode})</span>
    {:else}
      <span class="status-idle">{state.command ?? ''}</span>
    {/if}
  </div>
</div>

<style>
  .output-panel {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background: var(--color-bgPanel, #1e1e1e);
  }
  .toolbar {
    display: flex;
    gap: 4px;
    padding: 4px 8px;
    border-bottom: 1px solid var(--color-border, #333);
    flex-shrink: 0;
  }
  .btn {
    padding: 3px 10px;
    font-size: 12px;
    background: var(--color-bgHover, #2a2a2a);
    color: var(--color-text, #ccc);
    border: 1px solid var(--color-border, #333);
    border-radius: 3px;
    cursor: pointer;
  }
  .btn:hover:not(:disabled) { background: var(--color-accent, #569cd6); color: #fff; }
  .btn:disabled { opacity: 0.4; cursor: default; }
  .btn-cancel { border-color: #cc3e28; }
  .btn-clear { margin-left: auto; }
  .output-area {
    flex: 1;
    overflow-y: auto;
    padding: 6px 8px;
    font-family: Consolas, 'Courier New', monospace;
    font-size: 12px;
    line-height: 1.5;
  }
  .output-line { white-space: pre-wrap; word-break: break-all; }
  .placeholder { color: var(--color-textMuted, #666); font-style: italic; }
  .status-bar {
    padding: 3px 8px;
    font-size: 11px;
    border-top: 1px solid var(--color-border, #333);
    flex-shrink: 0;
    min-height: 22px;
  }
  .status-ok { color: #57a64a; }
  .status-err { color: #cc3e28; }
  .status-cancelled { color: #d7ba7d; }
  .status-running { color: var(--color-text, #ccc); }
  .status-idle { color: var(--color-textMuted, #666); }
</style>
```

- [ ] **Step 2: Compile check**

```bash
cd engine/editor && npm run build 2>&1 | head -30
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/docking/panels/OutputPanel.svelte
git commit -m "feat(editor/terminal): add OutputPanel with cargo buttons and ANSI display"
```

---

## Task 13: `OutputWrapper.svelte`

**Files:**
- Create: `engine/editor/src/lib/docking/panels/OutputWrapper.svelte`

Bridges IPC output events to the output store. Does NOT cancel the process on destroy.

- [ ] **Step 1: Create the component**

```svelte
<!-- engine/editor/src/lib/docking/panels/OutputWrapper.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import { t } from '$lib/i18n';
  import OutputPanel from './OutputPanel.svelte';
  import {
    getOutputState,
    subscribeOutput,
    appendLine,
    setRunning,
    setFinished,
    clearOutput,
    type OutputState,
  } from '$lib/stores/output';
  import { logError } from '$lib/stores/console';

  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  let state: OutputState = $state(getOutputState());
  let hasProject = $state(false);
  let unsubscribe: (() => void) | null = null;
  let unlistenData: (() => void) | null = null;
  let unlistenExit: (() => void) | null = null;

  async function handleRun(command: string, args: string[]): Promise<void> {
    const label = `${command} ${args.join(' ')}`;
    setRunning(label);
    try {
      await invoke('output_run', { command, args });
    } catch (e) {
      logError(`Output: failed to start command — ${e}`);
      setFinished(null, false);
    }
  }

  async function handleCancel(): Promise<void> {
    try {
      await invoke('output_cancel');
    } catch (e) {
      logError(`Output: failed to cancel — ${e}`);
    }
  }

  onMount(async () => {
    unsubscribe = subscribeOutput(() => {
      state = getOutputState();
    });

    if (!isTauri) return;

    // Check project is loaded (guard matches TerminalWrapper pattern)
    try {
      const editorState = await invoke<{ project_path?: string }>('get_editor_state');
      hasProject = !!editorState.project_path;
    } catch { /* leave hasProject false */ }

    unlistenData = await listen<{ line: string; stream: 'stdout' | 'stderr' }>(
      'output-data',
      e => appendLine(e.payload.line, e.payload.stream)
    );

    unlistenExit = await listen<{ code: number | null; cancelled: boolean }>(
      'output-exit',
      e => setFinished(e.payload.code, e.payload.cancelled)
    );
  });

  onDestroy(() => {
    unsubscribe?.();
    unlistenData?.();
    unlistenExit?.();
    // Note: does NOT cancel running process — build continues in background.
    // User can re-open the panel to see the result.
  });
</script>

<div class="panel-opaque">
  {#if !hasProject}
    <div class="placeholder">{t('placeholder.no_project')}</div>
  {:else}
    <OutputPanel
      {state}
      onRun={handleRun}
      onCancel={handleCancel}
      onClear={clearOutput}
    />
  {/if}
</div>

<style>
  .panel-opaque {
    width: 100%;
    height: 100%;
    background: var(--color-bgPanel, #1e1e1e);
    display: flex;
    flex-direction: column;
  }
  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-textMuted, #666);
    font-size: 13px;
  }
</style>
```

- [ ] **Step 2: Compile check**

```bash
cd engine/editor && npm run build 2>&1 | head -30
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/docking/panels/OutputWrapper.svelte
git commit -m "feat(editor/terminal): add OutputWrapper IPC event bridge"
```

---

## Task 14: Wire Panel Registry, App.svelte, and i18n

**Files:**
- Modify: `engine/editor/src/lib/docking/types.ts`
- Modify: `engine/editor/src/App.svelte`
- Modify: `engine/editor/src/lib/i18n/locales/en.ts`

- [ ] **Step 1: Add i18n keys to `en.ts`**

In `engine/editor/src/lib/i18n/locales/en.ts`, after the `'panel.file_explorer': 'File Explorer',` line, add:

```typescript
'panel.terminal': 'Terminal',
'panel.output': 'Output',
```

After the `'console.no_logs': 'No logs',` section, add a new section:

```typescript
// Terminal panel
'terminal.new_tab': 'New Terminal',
'terminal.close_tab': 'Close Terminal',

// Output panel
'output.build': 'Build',
'output.test': 'Test',
'output.run': 'Run',
'output.clippy': 'Clippy',
'output.cancel': 'Cancel',
'output.clear': 'Clear',
'output.running': 'Running...',
'output.exit_ok': 'Finished',
'output.exit_err': 'Failed',
'output.cancelled': 'Cancelled',
'output.empty': 'No output yet',
```

- [ ] **Step 2: Add panels to `panelRegistry` in `types.ts`**

In `engine/editor/src/lib/docking/types.ts`, in the `panelRegistry` array, after `{ id: 'file-explorer', titleKey: 'panel.file_explorer' }`, add:

```typescript
{ id: 'terminal', titleKey: 'panel.terminal' },
{ id: 'output', titleKey: 'panel.output' },
```

- [ ] **Step 3: Register components in `App.svelte`**

In `engine/editor/src/App.svelte`, after the `import FileExplorerWrapper from './lib/docking/panels/FileExplorerWrapper.svelte';` line, add:

```typescript
import TerminalWrapper from './lib/docking/panels/TerminalWrapper.svelte';
import OutputWrapper from './lib/docking/panels/OutputWrapper.svelte';
```

In the `panelComponents` object (around line 81), add:

```typescript
terminal: TerminalWrapper,
output: OutputWrapper,
```

- [ ] **Step 4: Run all tests**

```bash
cd engine/editor && npm test
```

Expected: all frontend tests pass (including the new terminal.test.ts and output.test.ts).

```bash
cd engine/editor && cargo test
```

Expected: all Rust tests pass.

- [ ] **Step 5: Full build check**

```bash
cd engine/editor && npm run build
```

Expected: clean build, no errors.

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src/lib/i18n/locales/en.ts \
        engine/editor/src/lib/docking/types.ts \
        engine/editor/src/App.svelte
git commit -m "feat(editor/terminal): wire terminal and output panels into docking registry"
```

---

## Task 15: End-to-End Verification

- [ ] **Step 1: Run the editor**

```bash
cd engine/editor && cargo tauri dev
```

- [ ] **Step 2: Verify Terminal panel**
  - Open a project (File → Open Project…)
  - Add Terminal panel from View menu or docking
  - Verify: PowerShell prompt appears in xterm.js
  - Type `ls` — verify output renders with colors
  - Click `+` — verify second tab opens
  - Close first tab — verify second stays active
  - Close last tab (should be blocked unless exited)

- [ ] **Step 3: Verify Output panel**
  - Add Output panel from View menu or docking
  - Click `Build` — verify cargo output streams in with colors
  - Verify status bar shows "cargo build Running..."
  - Verify status changes to "✓ Finished" or "✗ Failed (exit N)" on completion
  - Click `Clear` — verify output clears
  - Click `Build` again, then `Cancel` — verify "⊘ Cancelled"

- [ ] **Step 4: Verify no project state**
  - Restart editor without opening a project
  - Open Terminal panel — verify `placeholder.no_project` message shown (no crash)
  - Open Output panel — verify buttons do nothing harmful

- [ ] **Step 5: Commit final state**

```bash
git add -A
git commit -m "feat(editor): complete Terminal and Output panel implementation (ADV.6 partial)"
```
