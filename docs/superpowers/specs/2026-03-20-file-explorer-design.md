# File Explorer Panel — Design Spec

**Date:** 2026-03-20
**Status:** Approved

**Phase:** 0.8 (Editor Foundation — EDITOR.6)

---

## Overview

A VSCode-style file explorer panel for the Silmaril editor. Shows the project directory as an indented, collapsible tree. Clicking a file opens it in the user's external editor. Integrates with git status and the Tauri file watcher to stay live as files change on disk.

---

## Layout & Interaction Model

**VSCode-style indented tree:**
- Folders show a chevron (▶/▼) to expand/collapse
- Files and folders have type icons
- Single-click a file → opens in external editor
- Expansion is lazy — `get_file_tree` returns one level deep (all dir children = `null`); clicking a chevron calls `expand_dir` to populate that dir's children
- Expansion state persists across panel remounts (module-level store singleton)
- Selected path highlighted

**Panel header:**
- Title (i18n key `panel.file_explorer`)
- Refresh button (manual re-fetch of full tree + git status)
- Toggle: show/hide gitignored files (hidden by default — frontend filters `ignored: true` nodes, no refetch needed)

**Right-click context menu (on any node):**
- New File (`explorer.new_file`)
- New Folder (`explorer.new_folder`)
- Rename (`explorer.rename`)
- Delete (`explorer.delete`) — to OS recycle bin via `trash` crate
- Copy Path (`explorer.copy_path`)
- Reveal in OS Explorer (`explorer.reveal`)

**Rename / New File / New Folder UX:**
Opens a small inline input field replacing the node's label in the tree. Pressing Enter confirms; pressing Escape cancels. This avoids a modal dialog and matches VSCode behavior.

---

## Data Model

### TreeNode (Rust → TypeScript via Tauri)

```ts
type TreeNode = {
  name: string
  path: string          // absolute path
  kind: 'file' | 'dir'
  children: TreeNode[] | null  // null = not yet expanded; [] = empty dir
  gitStatus: 'modified' | 'untracked' | 'deleted' | 'staged' | null
  ignored: boolean      // gitignored — always sent, filtered in frontend
}
```

### GitStatus (Rust enum)

```rust
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GitStatus {
    Modified,
    Untracked,
    Deleted,
    Staged,
}
```

Serialises to `"modified"` / `"untracked"` / `"deleted"` / `"staged"` over Tauri IPC.
`get_git_status` returns `HashMap<String, GitStatus>` (Rust) → `Record<string, GitStatus>` (TypeScript).

---

## Rust API

### Tauri Commands

| Command | Signature | Purpose |
|---|---|---|
| `get_file_tree` | `(root: String) -> Vec<TreeNode>` | Load one level deep from root; all dir children = null |
| `expand_dir` | `(path: String) -> Vec<TreeNode>` | Lazy-load one level of children for a directory |
| `open_in_editor` | `(path: String) -> Result<(), String>` | Open file: `$EDITOR` → `$VISUAL` → OS default (`open`/`xdg-open`/`ShellExecute`) |
| `create_file` | `(path: String) -> Result<(), String>` | Create new empty file |
| `create_dir` | `(path: String) -> Result<(), String>` | Create new folder |
| `rename_path` | `(from: String, to: String) -> Result<(), String>` | Rename file or folder |
| `delete_path` | `(path: String) -> Result<(), String>` | Move to recycle bin (`trash` crate) |
| `get_git_status` | `(root: String) -> HashMap<String, GitStatus>` | Git status for all files in repo |
| `start_file_watch` | `(root: String) -> Result<(), String>` | Start `notify` watcher |
| `stop_file_watch` | `() -> ()` | Stop watcher |

### open_in_editor Platform Fallback Chain

1. `$EDITOR` env var (all platforms)
2. `$VISUAL` env var (all platforms)
3. `xdg-open <path>` (Linux)
4. `open <path>` (macOS)
5. `ShellExecute` / `start <path>` (Windows)

If all fail: return `Err("Could not open file — configure an editor in Settings")`.

### Tauri Event (Rust → Frontend)

| Event | Payload | Trigger |
|---|---|---|
| `file-tree-changed` | `{ root: String }` | Debounced 300ms after any FS event from the `notify` watcher |

---

## Rust Implementation

**Crate additions to `engine/editor/Cargo.toml`:**
- `notify` — cross-platform file system watching
- `trash` — send files to OS recycle bin

**Module layout:**
```
src-tauri/src/
└── file_explorer/
    ├── mod.rs       — module, registers Tauri commands
    ├── tree.rs      — directory traversal, TreeNode type, gitignore detection
    ├── watcher.rs   — notify watcher, 300ms debounce, Tauri event emission
    └── git.rs       — git status subprocess (git status --porcelain -z), parser
```

**Watcher lifecycle:** Owned by `FileExplorerWrapper.svelte`. The wrapper calls `start_file_watch(root)` on mount and `stop_file_watch()` on unmount. A single global watcher is stored in Tauri managed state (`Mutex<Option<RecommendedWatcher>>`). Calling `start_file_watch` on a new root replaces the previous watcher automatically.

**Git status:** Spawns `git status --porcelain -z` as a subprocess in the project root. Silent failure if git is not installed or the directory is not a git repo (returns empty map, no error propagated).

**Gitignore detection:** Use `ignore` crate (or parse `.gitignore` manually) to set `ignored: true` on nodes. Always included in `Vec<TreeNode>`; the frontend filters them based on `showIgnored`.

---

## i18n Keys

Add to `src/lib/i18n/locales/en.ts`:

```ts
// Panel title
'panel.file_explorer': 'File Explorer',

// Context menu
'explorer.new_file': 'New File',
'explorer.new_folder': 'New Folder',
'explorer.rename': 'Rename',
'explorer.delete': 'Delete',
'explorer.copy_path': 'Copy Path',
'explorer.reveal': 'Reveal in Explorer',
```

---

## Svelte Architecture

### New Files

```
src/lib/docking/panels/
├── FileExplorerWrapper.svelte   — mounts watcher, subscribes to store, passes props
└── FileExplorerPanel.svelte     — panel shell, header, scrollable tree

src/lib/stores/
└── file-explorer.ts             — tree state, git status, expansion state, error state

src/lib/components/
└── FileTreeNode.svelte          — recursive node (chevron, icon, git badge, inline input, context menu)
```

### Panel Registration

In `src/lib/docking/types.ts`, add to `panelRegistry`:

```ts
{ id: 'file-explorer', titleKey: 'panel.file_explorer' }
```

In `App.svelte` (or wherever `panelComponents` is defined), add:

```ts
'file-explorer': FileExplorerWrapper,
```

### Store Shape (`file-explorer.ts`)

```ts
type FileExplorerState = {
  root: string | null
  nodes: TreeNode[]
  expanded: Set<string>         // absolute paths currently expanded
  selected: string | null       // currently selected path
  gitStatus: Record<string, GitStatus>
  showIgnored: boolean          // default: false — filtered in frontend, no refetch
  loading: boolean
  error: string | null          // persistent error message shown in panel (e.g. root disappeared)
}
```

Module-level singleton — same pattern as `console.ts` and `editor-context.ts`. Persists across panel remounts and tab switches.

### Component Responsibilities

**`FileExplorerWrapper.svelte`:**
- Calls `start_file_watch(root)` on mount, `stop_file_watch()` on unmount
- Listens to `file-tree-changed` Tauri event → re-fetches affected subtree + git status
- Subscribes to `file-explorer` store, passes state as props to `FileExplorerPanel`

**`FileExplorerPanel.svelte`:**
- Renders panel header: title, refresh button, show-ignored toggle
- Shows inline error bar if `state.error` is set
- Renders scrollable container with `FileTreeNode` for each root node
- Filters out `ignored: true` nodes when `showIgnored` is false

**`FileTreeNode.svelte`:** Recursive. Renders:
- Chevron (▶/▼) for dirs — click calls `expand_dir`, patches store children
- Icon (folder/file type)
- Filename — replaced by inline `<input>` when renaming or creating
- Git status badge (colored M/U/D/S dot)
- Click handler: `open_in_editor(path)` for files; toggle expand for dirs
- Right-click: context menu with actions via Tauri commands

---

## Error Handling

| Scenario | Behavior |
|---|---|
| FS permission error | Set `store.error`, log to console store |
| Root path disappears | Set `store.error = "Project folder not found"`, show inline error bar |
| `open_in_editor` fails | Toast: "Could not open file — configure an editor in Settings" |
| Git not installed | Silent — no badges shown, empty git status map |
| Not a git repo | Silent — no badges shown, empty git status map |
| Git subprocess error | Silent — retain last known git status |
| `create_file` / `rename_path` fails | Toast with error message |

---

## Git Status Display

Files and folders are color-coded in the tree:

| Status | Color | Badge |
|---|---|---|
| `modified` | Yellow (`--color-warn`) | `M` |
| `untracked` | Green (`--color-success`) | `U` |
| `deleted` | Red (`--color-error`) | `D` |
| `staged` | Blue (`--color-info`) | `S` |
| Ignored | Dimmed (50% opacity) | — |

**Folder badge derivation:** A folder shows the highest-severity status of any descendant.
Severity order: `deleted > modified > staged > untracked` (deleted is most urgent; untracked is least).

---

## Future Work (Out of Scope)

- **Inline editor view** — clicking a file opens it in an editor tab within the editor
- **Cross-project search** — Ctrl+Shift+F (ADV.9)
- **Drag-and-drop** to move/copy files within the tree
- **File type icons** per extension (custom icon set)
- **Multi-select** (Ctrl+click, Shift+click)
- **Virtualised tree rendering** for projects with thousands of files

---

## Dependencies

- Rust crates: `notify`, `trash`, `ignore` (gitignore parsing)
- Follows existing patterns: wrapper + panel component, module-level store, Tauri IPC
- No new TypeScript dependencies required
