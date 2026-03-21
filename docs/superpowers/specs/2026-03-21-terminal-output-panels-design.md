# Terminal & Output Panels — Design Spec

**Date:** 2026-03-21
**Status:** Approved
**Scope:** ADV.6 (partial) — Interactive Terminal panel + Output/Build Log panel

---

## Overview

Two independent dockable panels added to the Silmaril editor:

1. **Terminal** — Interactive PTY shell (PowerShell default). Multiple tabs. Rendered via xterm.js.
2. **Output** — Read-only build log. Hardcoded buttons trigger `cargo build/test/run/clippy`. Custom ANSI renderer.

These are separate panels in the existing docking system — users place them wherever they want.

---

## Architecture

### Backend: portable-pty + std::process

- **Terminal tabs** use `portable-pty` — a real PTY (pseudoterminal) so programs detect a terminal, enabling colors, progress bars, interactive programs.
- **Output panel** uses `std::process::Command` with piped stdio + `CARGO_TERM_COLOR=always` — simpler, sufficient for read-only build output.
- Each PTY session runs in the project root directory (from `get_editor_state().project_path`).

> **Prerequisite:** `get_editor_state()` currently returns `project_path: None` (stub in `bridge/commands.rs`). Both terminal and output panels depend on a real project path being available. The wrappers guard on `project_path` and show the `placeholder.no_project` placeholder when absent. A future task must wire `open_project` → `EditorState.project_path` before these panels are fully functional.

### Frontend: xterm.js (Terminal) + custom ANSI renderer (Output)

- **xterm.js** (`@xterm/xterm` + `@xterm/addon-fit`) handles the interactive terminal — input, cursor, escape codes, scrollback buffer.
- **Custom ANSI renderer** for the Output panel — ~40-line SGR parser, append-only, renders colored `<span>` elements.

---

## File Map

### New files (create)

| File | Responsibility |
|---|---|
| `engine/editor/src-tauri/src/terminal/mod.rs` | Module entry, exports, `TerminalState` managed state |
| `engine/editor/src-tauri/src/terminal/pty.rs` | PTY session management — new/write/resize/close tab |
| `engine/editor/src-tauri/src/terminal/output.rs` | `std::process` runner for cargo commands |
| `engine/editor/src/lib/stores/terminal.ts` | Tab list, active tab, per-tab state (module singleton) |
| `engine/editor/src/lib/stores/output.ts` | Output lines, running state, exit code (module singleton) |
| `engine/editor/src/lib/docking/panels/TerminalTabs.svelte` | Tab bar — new tab button, tab labels, close buttons (co-located with TerminalPanel; not a shared component) |
| `engine/editor/src/lib/docking/panels/TerminalPanel.svelte` | xterm.js mount + tab switching |
| `engine/editor/src/lib/docking/panels/TerminalWrapper.svelte` | Lifecycle — creates first tab, bridges IPC events to xterm |
| `engine/editor/src/lib/docking/panels/OutputPanel.svelte` | Buttons + ANSI output display + status bar |
| `engine/editor/src/lib/docking/panels/OutputWrapper.svelte` | Process lifecycle — event bridge, subscribes to output store |

### Modified files

| File | Change |
|---|---|
| `engine/editor/Cargo.toml` | Add `portable-pty = "0.8"`, `which = "4"` |
| `engine/editor/src-tauri/lib.rs` | Add `pub mod terminal;`, `.manage(TerminalState::new())`, register 6 commands (`terminal_new_tab`, `terminal_write`, `terminal_resize`, `terminal_close_tab`, `output_run`, `output_cancel`) |
| `engine/editor/src-tauri/capabilities/default.json` | Add `core:event:default` to allow frontend to listen on dynamically-named events (`terminal-data:*`, `terminal-exit:*`) |
| `engine/editor/src/lib/docking/types.ts` | Add `terminal` and `output` to `panelRegistry` |
| `engine/editor/src/App.svelte` | Import `TerminalWrapper`, `OutputWrapper`; add to `panelComponents` |
| `engine/editor/src/lib/i18n/locales/en.ts` | Add `panel.terminal`, `panel.output`, `terminal.*`, `output.*` keys |

---

## Section 1: Rust Backend

### TerminalState (managed state)

```rust
// engine/editor/src-tauri/src/terminal/mod.rs
pub struct TerminalState {
    sessions: Mutex<HashMap<String, PtySession>>,
}

struct PtySession {
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send>,
    // Note: pair.slave is dropped after spawn_command returns to allow
    // the master side to detect child exit via EOF on Unix.
    // On Windows (ConPTY), the slave handle can be dropped immediately.
}

impl TerminalState {
    pub fn new() -> Self {
        Self { sessions: Mutex::new(HashMap::new()) }
    }
}
```

### Tauri Commands — Terminal

| Command | Args | Returns | Description |
|---|---|---|---|
| `terminal_new_tab` | — | `Result<String, String>` (tab_id) | Spawns PowerShell PTY in project root. Starts background reader thread that emits `terminal-data:{tab_id}` events. Returns `Err` if project path is unavailable or shell not found. |
| `terminal_write` | `tab_id: String, data: String` | `Result<(), String>` | Writes keystroke data to PTY writer. Returns `Err` silently if session already closed (tab closing race). |
| `terminal_resize` | `tab_id: String, cols: u16, rows: u16` | `Result<(), String>` | Calls `pty.resize(PtySize { rows, cols, .. })`. |
| `terminal_close_tab` | `tab_id: String` | `()` | Kills child process, removes session from map. No-op if tab already closed. |

**Shell resolution** (uses `which` crate — added to `Cargo.toml`):
```rust
fn resolve_shell() -> PathBuf {
    // Try pwsh (PowerShell 7+) first, then powershell.exe (Windows built-in)
    for candidate in &["pwsh", "powershell.exe"] {
        if let Ok(path) = which::which(candidate) {
            return path;
        }
    }
    PathBuf::from("powershell.exe") // fallback — will error at spawn if not found
}
```

**PTY new tab flow:**
```rust
// In terminal_new_tab:
let tab_id = uuid_v4();
let pty_system = native_pty_system();
let pair = pty_system.openpty(PtySize { rows: 24, cols: 80, .. })?;
let cmd = CommandBuilder::new(resolve_shell());
cmd.cwd(project_root);
let child = pair.slave.spawn_command(cmd)?;
// Drop pair.slave after spawn so master detects EOF on child exit (Unix).
// On Windows/ConPTY this is also safe to drop immediately.
drop(pair.slave);
let reader = pair.master.try_clone_reader()?;

// Store session
state.sessions.lock().insert(tab_id.clone(), PtySession {
    writer: pair.master.take_writer()?,
    child,
});

// Background thread: read PTY output, emit events
let tab_id_clone = tab_id.clone();
let app_handle = app.clone();
std::thread::spawn(move || {
    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf) {
            Ok(0) | Err(_) => {
                app_handle.emit(&format!("terminal-exit:{tab_id_clone}"), ()).ok();
                break;
            }
            Ok(n) => {
                let data = String::from_utf8_lossy(&buf[..n]).into_owned();
                app_handle.emit(&format!("terminal-data:{tab_id_clone}"), data).ok();
            }
        }
    }
});
```

### OutputState (managed state)

```rust
// engine/editor/src-tauri/src/terminal/output.rs
pub struct OutputState {
    child: Mutex<Option<Child>>,  // currently running process, if any
}

impl OutputState {
    pub fn new() -> Self { Self { child: Mutex::new(None) } }
}
```

Add `.manage(OutputState::new())` alongside `TerminalState` in `lib.rs`.

### Tauri Commands — Output

| Command | Args | Returns | Description |
|---|---|---|---|
| `output_run` | `command: String, args: Vec<String>` | `Result<(), String>` | Spawns cargo command in project root with piped stdout/stderr. Returns `Err("already running")` if a process is active (frontend disables buttons while running, preventing this in normal flow). Streams merged stdout+stderr lines as `output-data` events. Stores child handle for cancellation. |
| `output_cancel` | — | `()` (unit, serializes as `null`) | Kills the running child process if any. No-op if nothing is running (does NOT emit `output-exit`). Emits `output-exit` with `{ code: null, cancelled: true }` only if a process was actually killed. |

**Hardcoded commands (frontend side):**
```typescript
const COMMANDS = {
  build:  { cmd: 'cargo', args: ['build'],  label: 'output.build' },
  test:   { cmd: 'cargo', args: ['test'],   label: 'output.test' },
  run:    { cmd: 'cargo', args: ['run'],    label: 'output.run' },
  clippy: { cmd: 'cargo', args: ['clippy'], label: 'output.clippy' },
};
```

**Output run flow:**
```rust
// CARGO_TERM_COLOR=always ensures colored output even with piped stdio
let mut child = Command::new(&command)
    .args(&args)
    .current_dir(&project_root)
    .env("CARGO_TERM_COLOR", "always")
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
// Store Arc<Mutex<Option<Child>>> for cancellation.
// Two background threads: one for stdout, one for stderr.
// Both emit output-data events with stream discriminator.
// When both threads finish, emit output-exit.
```

**Tauri events emitted:**

| Event | Payload | When |
|---|---|---|
| `terminal-data:{tab_id}` | `String` (raw PTY bytes) | PTY produces output |
| `terminal-exit:{tab_id}` | — (empty) | Shell process exits |
| `output-data` | `{ line: String, stream: 'stdout' \| 'stderr' }` | Cargo command emits a line (both stdout and stderr use this event with discriminator) |
| `output-exit` | `{ code: number \| null, cancelled: boolean }` | Cargo command finishes or is cancelled |

---

## Section 2: Frontend Stores

### terminal.ts

```typescript
// engine/editor/src/lib/stores/terminal.ts
export interface TerminalTab {
  id: string;
  title: string;       // "Terminal 1", "Terminal 2", etc.
  exited: boolean;     // true after terminal-exit event; tab stays visible, dimmed
}

export interface TerminalState {
  tabs: TerminalTab[];
  activeTabId: string | null;
}

// Module-level singleton (matches console.ts pattern)
let state: TerminalState = { tabs: [], activeTabId: null };
let listeners: (() => void)[] = [];
let tabCounter = 0;  // increments per addTab call, never resets

export function getTerminalState(): TerminalState
export function subscribeTerminal(fn: () => void): () => void
export function addTab(id: string): void      // sets title to "Terminal N", sets as active
export function closeTab(id: string): void    // if active, switches to previous tab; no-op on last tab unless exited
export function setActiveTab(id: string): void
export function markExited(id: string): void  // sets exited=true; tab stays visible until user closes it
```

**Tab close rules:**
- The last tab cannot be closed unless it has `exited: true` (shell died). This prevents the user from getting a blank terminal with no way to open a new one.
- Exited tabs can always be closed, including the last one (leaves `tabs: []`, showing the no-project/no-shell placeholder).

### output.ts

```typescript
// engine/editor/src/lib/stores/output.ts
export interface OutputLine {
  raw: string;                          // original with ANSI codes
  stream: 'stdout' | 'stderr';          // for potential future styling differentiation
  spans: Array<{ text: string; color: string | null; bold: boolean }>;
}

export interface OutputState {
  lines: OutputLine[];
  running: boolean;
  exitCode: number | null;
  cancelled: boolean;
  command: string | null;               // e.g. "cargo build"
}

export function getOutputState(): OutputState
export function subscribeOutput(fn: () => void): () => void
export function appendLine(raw: string, stream: 'stdout' | 'stderr'): void   // parses ANSI internally
export function setRunning(cmd: string): void
export function setFinished(code: number | null, cancelled: boolean): void
export function clearOutput(): void    // resets lines, exitCode, cancelled, command; does NOT change running (safe to clear mid-build)
```

**ANSI parser** (internal to `output.ts`):
- Handles SGR sequences: foreground colors 30–37 (standard) and 90–97 (bright)
- Bold (`1m`), reset (`0m` or `m`)
- Maps to CSS color strings from a fixed 16-color palette matching the editor dark theme
- Ignores cursor movement codes (not needed for append-only display)

---

## Section 3: Frontend Components

### TerminalTabs.svelte

**Location:** `engine/editor/src/lib/docking/panels/TerminalTabs.svelte` (co-located with TerminalPanel — not a shared component)

Props: `tabs: TerminalTab[]`, `activeTabId: string | null`
Callbacks: `onNewTab`, `onCloseTab(id)`, `onSelectTab(id)`

Renders a horizontal tab bar:
- Each tab: label (`Terminal N`, dimmed if `exited`) + `×` close button
  - Close button hidden on the last non-exited tab (can't close last active tab)
  - Close button always shown on exited tabs
- `+` button on the right → `onNewTab` callback
- Active tab highlighted with `--color-accent` bottom border

### TerminalPanel.svelte

Receives `state: TerminalState` as prop.

Owns a `Map<tabId, Terminal>` (xterm.js instances, local to the component). On each new tab (reactive to `state.tabs`):
```typescript
// Create xterm instance for new tab ID
const term = new Terminal({ theme: editorTheme, fontFamily: 'monospace', fontSize: 13 });
const fitAddon = new FitAddon();
term.loadAddon(fitAddon);
term.open(containerDiv);  // containerDiv is a hidden <div> per tab
fitAddon.fit();

// Send keystrokes to Rust
term.onData(data => invoke('terminal_write', { tabId, data }));
```

On tab switch: show active tab's `<div>`, hide others via `display: none` (keeps xterm.js state alive).

On resize (ResizeObserver on container): `fitAddon.fit()` then read `term.cols`/`term.rows` and call `invoke('terminal_resize', { tabId, cols, rows })`.

If `state.tabs` is empty: show `{t('placeholder.no_project')}` placeholder (reuses existing i18n key).

**xterm.js version note:** `@xterm/xterm@^5.5.0` and `@xterm/addon-fit@^0.10.0` must be from the same xterm 5.x release series. Pin both together; do not upgrade one without the other.

### TerminalWrapper.svelte

`onMount`:
1. Subscribe to `terminal` store; `state = getTerminalState()` on each notification
2. If `isTauri` and `project_path` available: `invoke('terminal_new_tab')` → on success, `addTab(id)`, then set up per-tab listeners (see below)
3. If no project: show placeholder (no tabs created)

**Per-tab listener management** — `TerminalWrapper` does not own xterm.js instances (those live in `TerminalPanel`). The bridge works via a shared `pendingOutput: Map<tabId, string[]>` in the `terminal.ts` store, or more simply: `TerminalWrapper` writes incoming PTY data into a per-tab `pendingData` field on the store, and `TerminalPanel` reactively drains it by calling `term.write()`. Unlisteners are stored in a local `Map<tabId, UnlistenFn[]>`:

```typescript
const unlisteners = new Map<string, Array<() => void>>();

async function setupTabListeners(tabId: string) {
  // Write raw PTY data into store; TerminalPanel drains it reactively
  const unlistenData = await listen(`terminal-data:${tabId}`, e =>
    appendTerminalData(tabId, e.payload as string)  // store fn
  );
  const unlistenExit = await listen(`terminal-exit:${tabId}`, () => {
    markExited(tabId);
    cleanupTabListeners(tabId);
  });
  unlisteners.set(tabId, [unlistenData, unlistenExit]);
}

function cleanupTabListeners(tabId: string) {
  unlisteners.get(tabId)?.forEach(fn => fn());
  unlisteners.delete(tabId);
}
```

`terminal.ts` adds:
- `pendingData: Map<tabId, string>` field on `TerminalState` (last unread chunk)
- `appendTerminalData(tabId, data): void` — appends to per-tab buffer, calls `notify()`
- `drainTerminalData(tabId): string` — returns and clears the buffer (called by `TerminalPanel` in `$effect`)

`onNewTab` (from TerminalTabs): calls `invoke('terminal_new_tab')` → `addTab(id)` → `setupTabListeners(id, xterm)`.

`onCloseTab` (from TerminalTabs): `cleanupTabListeners(id)` → `invoke('terminal_close_tab', { tabId: id })` → `closeTab(id)`.

`onDestroy`:
1. Unsubscribe store
2. Clean up all remaining tab listeners: `unlisteners.forEach((_, id) => cleanupTabListeners(id))`
3. `invoke('terminal_close_tab')` for each remaining open tab

### OutputPanel.svelte

Receives `state: OutputState` as prop.

Renders:
- **Button row**: `Build` | `Test` | `Run` | `Clippy` (all disabled when `state.running`) | `Cancel` (shown and enabled only when `state.running`) | `Clear`
- **Output area**: scrollable `<div role="log">`, one `<div class="output-line">` per line with colored `<span>` children. When `state.lines` is empty and not running: shows `{t('output.empty')}` placeholder.
- **Status bar**: command name + spinner when running; `✓ Finished` (green) or `✗ Failed (exit N)` (red) badge when finished; `⊘ Cancelled` when cancelled.

Auto-scrolls to bottom on new lines (unless user has manually scrolled up — detected via `scrollTop + clientHeight < scrollHeight - threshold`).

### OutputWrapper.svelte

`onMount`:
1. Subscribe to `output` store
2. `const unlistenData = await listen('output-data', e => appendLine(e.payload.line, e.payload.stream))`
3. `const unlistenExit = await listen('output-exit', e => setFinished(e.payload.code, e.payload.cancelled))`

`onDestroy`: unsubscribe + call `unlistenData()` and `unlistenExit()`. Does NOT cancel the running process on destroy (build continues in background; user can re-open the panel to see the result).

---

## Section 4: i18n Keys

```typescript
// Terminal panel
'panel.terminal': 'Terminal',
'terminal.new_tab': 'New Terminal',
'terminal.close_tab': 'Close Terminal',

// Output panel
'panel.output': 'Output',
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
'output.empty': 'No output yet',  // shown when lines=[] and not running

// Reuse existing key for no-project state in both panels:
// 'placeholder.no_project' (already in en.ts)
```

---

## Section 5: Error Handling

- **PTY spawn failure** (PowerShell not found, project root unavailable): `terminal_new_tab` returns `Err(String)` → frontend calls `logError(msg)`, no tab added
- **Write to closed PTY**: `terminal_write` returns `Err` → frontend ignores silently (tab closing race is expected)
- **Output command already running**: `output_run` returns `Err("already running")` → frontend disables buttons while `state.running`, making this unreachable in normal flow; `logError` if it somehow fires
- **`output_cancel` with nothing running**: no-op, does NOT emit `output-exit`; frontend Cancel button is only shown when `state.running` so this race is not normally reachable
- **Project root unavailable** (`get_editor_state` returns no `project_path`): wrappers skip PTY/output setup, both panels show `{t('placeholder.no_project')}` placeholder
- **xterm.js load failure**: wrapped in `try/catch`, `logError` on failure, TerminalPanel shows inline error message

---

## Section 6: Testing

### Rust unit tests

- `pty.rs`: `resolve_shell()` returns a path to an existing executable on the current platform
- `output.rs`: `output_run` spawns `echo hello`, receives the line via `output-data` event, `output-exit` with code 0
- `output.rs`: `output_cancel` kills the running process; subsequent `output-exit` has `cancelled: true`
- `output.rs`: calling `output_run` twice returns `Err("already running")` on the second call

### Frontend unit tests (Vitest)

- `terminal.ts`: add tab → sets active; close non-last tab → switches active; close last non-exited tab → no-op; close exited last tab → tabs=[]
- `terminal.ts`: `markExited` sets `exited: true` without removing tab
- `output.ts`: ANSI parser — standard colors (30–37), bright colors (90–97), bold, reset, mixed sequences
- `output.ts`: `appendLine` stores raw string, correct stream discriminator, and parsed spans
- `output.ts`: `clearOutput` resets all state

---

## Dependencies

```toml
# engine/editor/Cargo.toml — new additions
portable-pty = "0.8"
which = "4"
uuid = { version = "1", features = ["v4"] }   # for tab_id generation in terminal_new_tab
```

```json
// engine/editor/package.json — new additions
"@xterm/xterm": "^5.5.0",
"@xterm/addon-fit": "^0.10.0"
```

**Version alignment:** `@xterm/xterm` and `@xterm/addon-fit` must be from the same xterm 5.x release series. Do not upgrade one without the other.

---

## Out of Scope

- Tasks / Run Configs panel (ADV.6 remainder — separate spec)
- Shell selector UI (always PowerShell, configurable later)
- Terminal search (`@xterm/addon-search`)
- Split panes within terminal
- Terminal history persistence across editor restarts
- xterm.js WebGL renderer addon
- Wiring `open_project` → `EditorState.project_path` (prerequisite, separate task)
