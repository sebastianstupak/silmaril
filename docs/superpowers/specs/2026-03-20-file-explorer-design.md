# File Explorer Panel — Design Spec

**Date:** 2026-03-20
**Status:** Approved
**Phase:** 0.8 (Editor Foundation — EDITOR.6)

---

## Overview

A VSCode-style file explorer panel for the Silmaril editor. Shows the project directory as an indented, collapsible tree. Clicking a file opens it in the user's external editor (`$EDITOR`). Integrates with git status and the Tauri file watcher to stay live as files change on disk.

---

## Layout & Interaction Model

**VSCode-style indented tree:**
- Folders show a chevron (▶/▼) to expand/collapse
- Files and folders have type icons
- Single-click a file → opens in external editor
- Expansion is lazy — children are fetched on first expand
- Expansion state persists across panel remounts (module-level store singleton)
- Selected path highlighted

**Panel header:**
- Title ("Explorer")
- Refresh button (manual re-fetch)
- Toggle: show/hide gitignored files (hidden by default)

**Right-click context menu (on any node):**
- New File
- New Folder
- Rename
- Delete (to OS recycle bin via `trash` crate — recoverable)
- Copy Path
- Reveal in OS Explorer

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
  ignored: boolean      // gitignored — sent but hidden by default
}
```

### GitStatus

```ts
type GitStatus = 'modified' | 'untracked' | 'deleted' | 'staged' | null
```

Stored as `Map<absolutePath, GitStatus>` in the frontend store. Applied to `TreeNode` on render. If no git repo or git not installed, map is empty — no badges shown, no error.

---

## Rust API

### Tauri Commands

| Command | Signature | Purpose |
|---|---|---|
| `get_file_tree` | `(root: String, show_ignored: bool) -> Vec<TreeNode>` | Load full tree from root |
| `expand_dir` | `(path: String) -> Vec<TreeNode>` | Lazy-load children of a directory |
| `open_in_editor` | `(path: String) -> ()` | Open file with `$EDITOR` or OS default |
| `create_file` | `(path: String) -> ()` | Create new file |
| `create_dir` | `(path: String) -> ()` | Create new folder |
| `rename_path` | `(from: String, to: String) -> ()` | Rename file or folder |
| `delete_path` | `(path: String) -> ()` | Move to recycle bin (`trash` crate) |
| `get_git_status` | `(root: String) -> HashMap<String, GitStatus>` | Git status for all files |
| `start_watch` | `(root: String) -> ()` | Start `notify` file watcher |
| `stop_watch` | `() -> ()` | Stop watcher |

### Tauri Event (Rust → Frontend)

| Event | Payload | Trigger |
|---|---|---|
| `file-tree-changed` | `{ root: String }` | Debounced 300ms after any FS event |

---

## Rust Implementation

**Crate additions to `src-tauri/Cargo.toml`:**
- `notify` — cross-platform file system watching
- `trash` — send files to OS recycle bin

**Module layout:**
```
src-tauri/src/
└── file_explorer/
    ├── mod.rs       — module, registers commands
    ├── tree.rs      — directory traversal, TreeNode, lazy expand
    ├── watcher.rs   — notify watcher, debounce, event emission
    └── git.rs       — git status subprocess, parse --porcelain -z output
```

**Watcher:** Single global watcher stored in Tauri managed state (`Mutex<Option<RecommendedWatcher>>`). Opening a new project replaces the previous watcher. Stopped on app exit.

**Git status:** Spawns `git status --porcelain -z` subprocess in the project root. Silent failure if git not installed or directory is not a git repo. Runs on: initial load, `file-tree-changed` event, manual refresh.

---

## Svelte Architecture

### New Files

```
src/lib/docking/panels/
├── FileExplorerWrapper.svelte   — subscribes to store, passes props to panel
└── FileExplorerPanel.svelte     — panel shell, header, scrollable tree

src/lib/stores/
└── file-explorer.ts             — tree state, git status, expansion state

src/lib/components/
└── FileTreeNode.svelte          — recursive node (chevron, icon, git badge, context menu)
```

### Store Shape (`file-explorer.ts`)

```ts
type FileExplorerState = {
  root: string | null
  nodes: TreeNode[]
  expanded: Set<string>         // absolute paths currently expanded
  selected: string | null       // currently selected path
  gitStatus: Map<string, GitStatus>
  showIgnored: boolean          // default: false
  loading: boolean
}
```

Module-level singleton — same pattern as `console.ts` and `editor-context.ts`. Persists across panel remounts and tab switches.

### Component Responsibilities

**`FileExplorerWrapper.svelte`:** Subscribes to `file-explorer` store on mount. Passes `nodes`, `expanded`, `selected`, `gitStatus`, `showIgnored` as props to `FileExplorerPanel`. Listens to `file-tree-changed` Tauri event and triggers store refresh.

**`FileExplorerPanel.svelte`:** Renders panel header (title, refresh button, show-ignored toggle) and a scrollable container with `FileTreeNode` for each root node.

**`FileTreeNode.svelte`:** Recursive. Renders:
- Chevron (▶/▼) for dirs — click calls `expand_dir`, patches store
- Icon (folder/file type)
- Filename
- Git status badge (colored M/U/D dot, dimmed if ignored)
- Click handler: `open_in_editor(path)` for files; toggle expand for dirs
- Right-click: context menu with actions routed to Tauri commands

### Panel Registration

Add `'file-explorer'` to the panel registry in `src/lib/docking/types.ts`:

```ts
{
  id: 'file-explorer',
  label: 'File Explorer',
  component: FileExplorerWrapper,
}
```

---

## Error Handling

| Scenario | Behavior |
|---|---|
| FS permission error | Log to console store + toast notification |
| Path not found | Log to console store + toast notification |
| `open_in_editor` fails (no `$EDITOR`) | Toast: "Could not open editor — set `$EDITOR` or configure in Settings" |
| Git not installed | Silent — no badges, no error shown |
| Not a git repo | Silent — no badges, no error shown |
| Git subprocess error | Silent — stale git status retained |

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

Folder badges are derived: a folder shows the "most severe" status of any child (deleted > modified > untracked > staged).

---

## Future Work (Out of Scope for This Spec)

- **Inline editor view** — clicking a file opens it in an editor tab within the editor itself (planned as a separate panel)
- **Search within files** — cross-project search (ADV.9)
- **Drag-and-drop** to move/copy files
- **File type icons** per extension (custom icon set)
- **Multi-select** (Ctrl+click, Shift+click)

---

## Dependencies

- Rust: `notify`, `trash` crates
- Follows existing patterns: wrapper + panel component, module-level store, Tauri IPC
- No new TS dependencies required
