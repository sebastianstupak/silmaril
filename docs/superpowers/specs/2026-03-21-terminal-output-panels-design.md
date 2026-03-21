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
| `engine/editor/src/lib/components/TerminalTabs.svelte` | Tab bar — new tab button, tab labels, close buttons |
| `engine/editor/src/lib/docking/panels/TerminalPanel.svelte` | xterm.js mount + tab switching |
| `engine/editor/src/lib/docking/panels/TerminalWrapper.svelte` | Lifecycle — creates first tab, bridges IPC events to xterm |
| `engine/editor/src/lib/docking/panels/OutputPanel.svelte` | Buttons + ANSI output display + status bar |
| `engine/editor/src/lib/docking/panels/OutputWrapper.svelte` | Process lifecycle — event bridge, subscribes to output store |

### Modified files

| File | Change |
|---|---|
| `engine/editor/Cargo.toml` | Add `portable-pty = "0.8"` |
| `engine/editor/src-tauri/lib.rs` | Add `pub mod terminal;`, `.manage(TerminalState::new())`, register 6 commands |
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
| `terminal_new_tab` | — | `Result<String, String>` (tab_id) | Spawns PowerShell PTY in project root. Starts background reader thread that emits `terminal-data:{tab_id}` events. |
| `terminal_write` | `tab_id: String, data: String` | `Result<(), String>` | Writes keystroke data to PTY writer. |
| `terminal_resize` | `tab_id: String, cols: u16, rows: u16` | `Result<(), String>` | Calls `pty.resize(PtySize { rows, cols, .. })`. |
| `terminal_close_tab` | `tab_id: String` | `()` | Kills child process, removes session from map. |

**Shell resolution:**
```rust
fn resolve_shell() -> PathBuf {
    // Try pwsh (PowerShell 7+) first, then powershell.exe (Windows built-in)
    for candidate in &["pwsh", "powershell.exe"] {
        if let Ok(path) = which::which(candidate) {
            return path;
        }
    }
    PathBuf::from("powershell.exe") // fallback
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
let reader = pair.master.try_clone_reader()?;

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

### Tauri Commands — Output

| Command | Args | Returns | Description |
|---|---|---|---|
| `output_run` | `command: String, args: Vec<String>` | `Result<(), String>` | Spawns cargo command in project root with piped stdout/stderr. Streams lines as `output-data` events. Stores child handle for cancellation. |
| `output_cancel` | — | `()` | Kills the running child process if any. Emits `output-exit` with `{ "cancelled": true }`. |

**Hardcoded commands (frontend side):**
```typescript
const COMMANDS = {
  build:  { cmd: 'cargo', args: ['build'] },
  test:   { cmd: 'cargo', args: ['test'] },
  run:    { cmd: 'cargo', args: ['run'] },
  clippy: { cmd: 'cargo', args: ['clippy'] },
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
// Store child handle, then read stdout+stderr on background threads
// Emit output-data for each line, output-exit when done
```

**Tauri events emitted:**

| Event | Payload | When |
|---|---|---|
| `terminal-data:{tab_id}` | `String` (raw PTY bytes) | PTY produces output |
| `terminal-exit:{tab_id}` | — | Shell process exits |
| `output-data` | `{ line: String }` | Cargo command emits a line |
| `output-exit` | `{ code: i32 \| null, cancelled: bool }` | Cargo command finishes |

---

## Section 2: Frontend Stores

### terminal.ts

```typescript
// engine/editor/src/lib/stores/terminal.ts
export interface TerminalTab {
  id: string;
  title: string;       // "Terminal 1", "Terminal 2", etc.
  exited: boolean;
}

export interface TerminalState {
  tabs: TerminalTab[];
  activeTabId: string | null;
}

// Module-level singleton (matches console.ts pattern)
let state: TerminalState = { tabs: [], activeTabId: null };
let listeners: (() => void)[] = [];

export function getTerminalState(): TerminalState
export function subscribeTerminal(fn: () => void): () => void
export function addTab(id: string): void      // increments title counter
export function closeTab(id: string): void    // switches active if needed
export function setActiveTab(id: string): void
export function markExited(id: string): void
```

### output.ts

```typescript
// engine/editor/src/lib/stores/output.ts
export interface OutputLine {
  raw: string;                          // original with ANSI codes
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
export function appendLine(raw: string): void   // parses ANSI internally
export function setRunning(cmd: string): void
export function setFinished(code: number | null, cancelled: boolean): void
export function clearOutput(): void
```

**ANSI parser** (internal to `output.ts`):
- Handles SGR sequences: foreground colors 30–37 (standard) and 90–97 (bright)
- Bold (`1m`), reset (`0m` or `m`)
- Maps to CSS color strings from a fixed 16-color palette matching the editor dark theme
- Ignores cursor movement codes (not needed for append-only display)

---

## Section 3: Frontend Components

### TerminalTabs.svelte

Props: `tabs: TerminalTab[]`, `activeTabId: string | null`
Events (callbacks): `onNewTab`, `onCloseTab(id)`, `onSelectTab(id)`

Renders a horizontal tab bar:
- Each tab: label (`Terminal N`) + `×` close button (hidden on last tab)
- `+` button on the right → calls `invoke('terminal_new_tab')` then `addTab(id)`
- Active tab highlighted with `--color-accent` bottom border

### TerminalPanel.svelte

Receives `state: TerminalState` as prop.

Owns a `Map<tabId, Terminal>` (xterm.js instances). On mount:
```typescript
// For each tab, create an xterm.js Terminal instance
const term = new Terminal({ theme: editorTheme, fontFamily: 'monospace', fontSize: 13 });
const fitAddon = new FitAddon();
term.loadAddon(fitAddon);
term.open(containerDiv);
fitAddon.fit();

// Send keystrokes to Rust
term.onData(data => invoke('terminal_write', { tabId, data }));
```

On tab switch: show/hide via `display: none` (keeps xterm.js state alive, avoids re-mount cost).

On resize (ResizeObserver on container): `fitAddon.fit()` + `invoke('terminal_resize', { tabId, cols, rows })`.

### TerminalWrapper.svelte

`onMount`:
1. Subscribe to `terminal` store
2. If `isTauri` and project is open: `invoke('terminal_new_tab')` → `addTab(id)`
3. Listen for `terminal-data:{tabId}` → write to xterm instance
4. Listen for `terminal-exit:{tabId}` → `markExited(id)`

`onDestroy`:
1. Unsubscribe store
2. Unlisten all events
3. `invoke('terminal_close_tab', { tabId })` for each open tab

### OutputPanel.svelte

Receives `state: OutputState` as prop.

Renders:
- **Button row**: `Build` | `Test` | `Run` | `Clippy` | `Cancel` (shown only when `state.running`) | `Clear`
- **Output area**: scrollable `<div>`, one `<div class="output-line">` per line with colored `<span>` children
- **Status bar**: command name + spinner when running; exit code badge (`✓ 0` green / `✗ 1` red) when finished

Auto-scrolls to bottom on new lines (unless user has scrolled up).

### OutputWrapper.svelte

`onMount`:
1. Subscribe to `output` store
2. Listen for `output-data` → `appendLine(payload.line)`
3. Listen for `output-exit` → `setFinished(payload.code, payload.cancelled)`

`onDestroy`: unsubscribe + unlisten. Does not cancel running process on destroy (build continues in background).

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
'output.empty': 'No output yet',
```

---

## Section 5: Error Handling

- **PTY spawn failure** (PowerShell not found, project root missing): `terminal_new_tab` returns `Err(String)` → frontend calls `logError(msg)`, no tab added
- **Write to closed PTY**: `terminal_write` returns `Err`, frontend ignores (tab may already be closing)
- **Output command already running**: `output_run` returns `Err("already running")` → frontend disables buttons while running (prevents double-trigger)
- **Project root unavailable**: `get_editor_state` returns no `project_path` → wrapper skips PTY/output setup, panel shows "No project open" placeholder
- **xterm.js load failure**: wrapped in `try/catch`, `logError` on failure, panel shows error message

---

## Section 6: Testing

### Rust unit tests

- `pty.rs`: test `resolve_shell()` returns a valid path on the current platform
- `output.rs`: test `output_run` emits correct events for a simple command (`echo hello`)
- `output.rs`: test `output_cancel` stops the running process

### Frontend unit tests (Vitest)

- `terminal.ts`: tab CRUD — add, close, switch active, close-last-switches-to-previous
- `output.ts`: ANSI parser — standard colors, bright colors, bold, reset, mixed sequence
- `output.ts`: `appendLine` stores raw + parsed spans correctly

---

## Dependencies

```toml
# engine/editor/Cargo.toml
portable-pty = "0.8"
```

```json
// engine/editor/package.json
"@xterm/xterm": "^5.5.0",
"@xterm/addon-fit": "^0.10.0"
```

---

## Out of Scope

- Tasks / Run Configs panel (ADV.6 remainder — separate spec)
- Shell selector UI (always PowerShell, configurable later)
- Terminal search (`@xterm/addon-search`)
- Split panes within terminal
- Terminal history persistence across editor restarts
- xterm.js WebGL renderer addon
