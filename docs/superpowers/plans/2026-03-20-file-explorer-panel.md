# File Explorer Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a VSCode-style file explorer panel to the Silmaril editor — indented tree, git status badges, file watcher, file operations — dockable alongside existing panels.

**Architecture:** Rust handles all FS ops (tree traversal, `notify` watcher, git status subprocess, file mutations) via Tauri commands and events. Svelte renders a lazy-expanding tree using a module-level store singleton. Follows the existing wrapper + panel + store pattern (see `ConsoleWrapper` / `ConsolePanel` / `console.ts`).

**Tech Stack:** Rust (`notify`, `trash`, `ignore` crates), Tauri 2 IPC, Svelte 5, TypeScript

**Spec:** `docs/superpowers/specs/2026-03-20-file-explorer-design.md`

---

## File Map

### New files (create)

| File | Responsibility |
|---|---|
| `engine/editor/src-tauri/src/file_explorer/mod.rs` | Module entry, exports commands |
| `engine/editor/src-tauri/src/file_explorer/tree.rs` | `TreeNode` type, `get_file_tree`, `expand_dir` |
| `engine/editor/src-tauri/src/file_explorer/git.rs` | `get_git_status` — spawns `git status --porcelain -z` |
| `engine/editor/src-tauri/src/file_explorer/watcher.rs` | `FileWatcherState`, `start_file_watch`, `stop_file_watch`, debounce |
| `engine/editor/src-tauri/src/file_explorer/ops.rs` | `open_in_editor`, `create_file`, `create_dir`, `rename_path`, `delete_path` |
| `engine/editor/src/lib/stores/file-explorer.ts` | Store singleton — tree state, git status, expansion, error |
| `engine/editor/src/lib/components/FileTreeNode.svelte` | Recursive tree node — chevron, icon, git badge, inline input, context menu |
| `engine/editor/src/lib/docking/panels/FileExplorerPanel.svelte` | Panel shell — header (refresh, toggle), scrollable tree |
| `engine/editor/src/lib/docking/panels/FileExplorerWrapper.svelte` | Mounts watcher, subscribes to store, bridges Tauri events |

### Modified files

| File | Change |
|---|---|
| `engine/editor/Cargo.toml` | Add `notify`, `trash`, `ignore` dependencies |
| `engine/editor/src-tauri/lib.rs` | Add `pub mod file_explorer;`, `.manage(FileWatcherState)`, register commands |
| `engine/editor/src/lib/docking/types.ts` | Add `file-explorer` to `panelRegistry` |
| `engine/editor/src/App.svelte` | Import `FileExplorerWrapper`, add to `panelComponents` |
| `engine/editor/src/lib/i18n/locales/en.ts` | Add `panel.file_explorer` + `explorer.*` keys |

---

## Task 1: Add Rust dependencies

**Files:**
- Modify: `engine/editor/Cargo.toml`

- [ ] **Step 1: Add crates to Cargo.toml**

Open `engine/editor/Cargo.toml` and add under `[dependencies]`:

```toml
notify = "6"
trash = "5"
ignore = "0.4"
```

- [ ] **Step 2: Verify crates resolve**

```bash
cd engine/editor && cargo fetch
```

Expected: no errors, crates downloaded.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/Cargo.toml
git commit -m "chore(editor): add notify, trash, ignore crates for file explorer"
```

---

## Task 2: TreeNode type + tree commands

**Files:**
- Create: `engine/editor/src-tauri/src/file_explorer/mod.rs`
- Create: `engine/editor/src-tauri/src/file_explorer/tree.rs`

- [ ] **Step 1: Create mod.rs**

```rust
// engine/editor/src-tauri/src/file_explorer/mod.rs
pub mod git;
pub mod ops;
pub mod tree;
pub mod watcher;

pub use tree::{expand_dir, get_file_tree};
pub use git::get_git_status;
pub use ops::{create_dir, create_file, delete_path, open_in_editor, rename_path};
pub use watcher::{start_file_watch, stop_file_watch, FileWatcherState};
```

- [ ] **Step 2: Write failing unit test for tree.rs**

```rust
// engine/editor/src-tauri/src/file_explorer/tree.rs
// (write just the test first, no implementation yet)

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_file_tree_one_level_deep() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::write(root.join("file.txt"), "").unwrap();
        fs::create_dir(root.join("subdir")).unwrap();
        fs::write(root.join("subdir").join("nested.txt"), "").unwrap();

        let nodes = read_dir_one_level(root).unwrap();

        // Should have 2 entries: file.txt and subdir
        assert_eq!(nodes.len(), 2);
        // subdir should have children = None (not expanded yet)
        let dir_node = nodes.iter().find(|n| n.kind == NodeKind::Dir).unwrap();
        assert!(dir_node.children.is_none());
        // file should have kind File
        let file_node = nodes.iter().find(|n| n.kind == NodeKind::File).unwrap();
        assert_eq!(file_node.name, "file.txt");
    }

    #[test]
    fn test_expand_dir_returns_children() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("a.rs"), "").unwrap();
        fs::write(root.join("sub").join("b.rs"), "").unwrap();

        let children = read_dir_one_level(&root.join("sub")).unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_empty_dir_returns_empty_vec() {
        let dir = TempDir::new().unwrap();
        let nodes = read_dir_one_level(dir.path()).unwrap();
        assert!(nodes.is_empty());
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd engine/editor && cargo test file_explorer::tree 2>&1 | head -20
```

Expected: compile error — `read_dir_one_level` and `NodeKind` not found.

- [ ] **Step 4: Implement tree.rs**

```rust
// engine/editor/src-tauri/src/file_explorer/tree.rs
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    File,
    Dir,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub kind: NodeKind,
    /// None = not yet expanded (dirs only). Some([]) = empty dir.
    pub children: Option<Vec<TreeNode>>,
    pub git_status: Option<String>,
    pub ignored: bool,
}

/// Read one level of a directory. Dirs get children = None (lazy).
/// Entries are sorted: dirs first, then files, both alphabetical.
pub fn read_dir_one_level(dir: &Path) -> Result<Vec<TreeNode>, String> {
    let mut entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory: {e}"))?
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        b_is_dir.cmp(&a_is_dir).then(a.file_name().cmp(&b.file_name()))
    });

    Ok(entries
        .into_iter()
        .map(|e| {
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            TreeNode {
                name: e.file_name().to_string_lossy().into_owned(),
                path: e.path().to_string_lossy().into_owned(),
                kind: if is_dir { NodeKind::Dir } else { NodeKind::File },
                children: if is_dir { None } else { Some(vec![]) },
                git_status: None,
                ignored: false,
            }
        })
        .collect())
}

#[tauri::command]
pub fn get_file_tree(root: String) -> Result<Vec<TreeNode>, String> {
    read_dir_one_level(Path::new(&root))
}

#[tauri::command]
pub fn expand_dir(path: String) -> Result<Vec<TreeNode>, String> {
    read_dir_one_level(Path::new(&path))
}

#[cfg(test)]
mod tests {
    // (paste tests from Step 2 here)
}
```

- [ ] **Step 5: Run tests — expect pass**

```bash
cd engine/editor && cargo test file_explorer::tree
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src-tauri/src/file_explorer/
git commit -m "feat(editor): file explorer tree module — get_file_tree, expand_dir"
```

---

## Task 3: Git status command

**Files:**
- Create: `engine/editor/src-tauri/src/file_explorer/git.rs`

- [ ] **Step 1: Write failing test**

```rust
// engine/editor/src-tauri/src/file_explorer/git.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_porcelain_modified() {
        let output = " M src/main.rs\0";
        let result = parse_porcelain(output, "/project");
        assert_eq!(result.get("src/main.rs").map(|s| s.as_str()), Some("modified"));
    }

    #[test]
    fn test_parse_porcelain_untracked() {
        let output = "?? new_file.rs\0";
        let result = parse_porcelain(output, "/project");
        assert_eq!(result.get("new_file.rs").map(|s| s.as_str()), Some("untracked"));
    }

    #[test]
    fn test_parse_porcelain_staged() {
        let output = "M  src/lib.rs\0";
        let result = parse_porcelain(output, "/project");
        assert_eq!(result.get("src/lib.rs").map(|s| s.as_str()), Some("staged"));
    }

    #[test]
    fn test_parse_porcelain_deleted() {
        let output = " D old.rs\0";
        let result = parse_porcelain(output, "/project");
        assert_eq!(result.get("old.rs").map(|s| s.as_str()), Some("deleted"));
    }

    #[test]
    fn test_parse_porcelain_empty() {
        let result = parse_porcelain("", "/project");
        assert!(result.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd engine/editor && cargo test file_explorer::git 2>&1 | head -10
```

Expected: compile error — `parse_porcelain` not found.

- [ ] **Step 3: Implement git.rs**

```rust
// engine/editor/src-tauri/src/file_explorer/git.rs
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Parse `git status --porcelain -z` output into a map of relative_path → status string.
pub fn parse_porcelain(output: &str, _root: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for entry in output.split('\0') {
        if entry.len() < 4 {
            continue;
        }
        let xy = &entry[..2];
        let path = entry[3..].trim().to_string();
        if path.is_empty() {
            continue;
        }
        let status = match xy {
            s if s.starts_with("??") => "untracked",
            s if s.starts_with(' ') && s.ends_with('D') => "deleted",
            s if s.starts_with(' ') && s.ends_with('M') => "modified",
            s if s.starts_with('M') || s.starts_with('A') => "staged",
            s if s.ends_with('D') => "deleted",
            _ => continue,
        };
        map.insert(path, status.to_string());
    }
    map
}

#[tauri::command]
pub fn get_git_status(root: String) -> HashMap<String, String> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "-z"])
        .current_dir(Path::new(&root))
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).into_owned();
            parse_porcelain(&text, &root)
        }
        _ => HashMap::new(), // silent failure — not a git repo, git not installed, etc.
    }
}

#[cfg(test)]
mod tests {
    // (paste tests from Step 1 here)
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cd engine/editor && cargo test file_explorer::git
```

Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/src/file_explorer/git.rs
git commit -m "feat(editor): file explorer git status command"
```

---

## Task 4: File watcher

**Files:**
- Create: `engine/editor/src-tauri/src/file_explorer/watcher.rs`

- [ ] **Step 1: Implement watcher.rs**

This module manages a single global `notify` watcher stored in Tauri managed state. No unit test for the watcher (requires a running Tauri app handle) — tested end-to-end in Task 13.

```rust
// engine/editor/src-tauri/src/file_explorer/watcher.rs
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub struct FileWatcherState {
    watcher: Mutex<Option<RecommendedWatcher>>,
}

impl FileWatcherState {
    pub fn new() -> Self {
        Self { watcher: Mutex::new(None) }
    }
}

#[tauri::command]
pub fn start_file_watch(
    root: String,
    app: AppHandle,
    state: tauri::State<'_, FileWatcherState>,
) -> Result<(), String> {
    let app_clone = app.clone();
    let root_clone = root.clone();

    // Debounce: only emit after 300ms of quiet
    let last_event: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    let last_event_clone = last_event.clone();

    let mut watcher = RecommendedWatcher::new(
        move |_res: notify::Result<notify::Event>| {
            let mut last = last_event_clone.lock().unwrap();
            *last = Some(Instant::now());
            let app2 = app_clone.clone();
            let root2 = root_clone.clone();
            let last2 = last_event_clone.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                let guard = last2.lock().unwrap();
                if let Some(t) = *guard {
                    if t.elapsed() >= Duration::from_millis(299) {
                        let _ = app2.emit("file-tree-changed", serde_json::json!({ "root": root2 }));
                    }
                }
            });
        },
        Config::default(),
    )
    .map_err(|e| format!("Failed to create watcher: {e}"))?;

    watcher
        .watch(Path::new(&root), RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch path: {e}"))?;

    *state.watcher.lock().unwrap() = Some(watcher);
    tracing::info!(root = %root, "File watcher started");
    Ok(())
}

#[tauri::command]
pub fn stop_file_watch(state: tauri::State<'_, FileWatcherState>) {
    *state.watcher.lock().unwrap() = None;
    tracing::info!("File watcher stopped");
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd engine/editor && cargo build 2>&1 | grep -E "error|warning.*unused" | head -20
```

Expected: no errors (warnings about unused imports are OK at this stage since commands aren't registered yet).

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src-tauri/src/file_explorer/watcher.rs
git commit -m "feat(editor): file explorer watcher — notify-based, 300ms debounce"
```

---

## Task 5: File operation commands

**Files:**
- Create: `engine/editor/src-tauri/src/file_explorer/ops.rs`

- [ ] **Step 1: Write failing tests**

```rust
// engine/editor/src-tauri/src/file_explorer/ops.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_do_create_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("new.txt").to_string_lossy().into_owned();
        do_create_file(&path).unwrap();
        assert!(std::path::Path::new(&path).exists());
    }

    #[test]
    fn test_do_create_dir() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("newdir").to_string_lossy().into_owned();
        do_create_dir(&path).unwrap();
        assert!(std::path::Path::new(&path).is_dir());
    }

    #[test]
    fn test_do_rename_path() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("old.txt").to_string_lossy().into_owned();
        let to = dir.path().join("new.txt").to_string_lossy().into_owned();
        std::fs::write(&from, "").unwrap();
        do_rename_path(&from, &to).unwrap();
        assert!(!std::path::Path::new(&from).exists());
        assert!(std::path::Path::new(&to).exists());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd engine/editor && cargo test file_explorer::ops 2>&1 | head -10
```

Expected: compile error.

- [ ] **Step 3: Implement ops.rs**

```rust
// engine/editor/src-tauri/src/file_explorer/ops.rs
use std::path::Path;

// ── Internal helpers (testable without Tauri) ──────────────────────────────

pub fn do_create_file(path: &str) -> Result<(), String> {
    std::fs::File::create(Path::new(path))
        .map(|_| ())
        .map_err(|e| format!("Cannot create file: {e}"))
}

pub fn do_create_dir(path: &str) -> Result<(), String> {
    std::fs::create_dir_all(Path::new(path))
        .map_err(|e| format!("Cannot create directory: {e}"))
}

pub fn do_rename_path(from: &str, to: &str) -> Result<(), String> {
    std::fs::rename(Path::new(from), Path::new(to))
        .map_err(|e| format!("Cannot rename: {e}"))
}

// ── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn open_in_editor(path: String) -> Result<(), String> {
    // Try $EDITOR, then $VISUAL, then OS default
    if let Ok(editor) = std::env::var("EDITOR") {
        if std::process::Command::new(&editor).arg(&path).spawn().is_ok() {
            return Ok(());
        }
    }
    if let Ok(editor) = std::env::var("VISUAL") {
        if std::process::Command::new(&editor).arg(&path).spawn().is_ok() {
            return Ok(());
        }
    }

    // OS default open
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Could not open file — configure an editor in Settings ({e})"))
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Could not open file — configure an editor in Settings ({e})"))
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Could not open file — configure an editor in Settings ({e})"))
    }
}

#[tauri::command]
pub fn create_file(path: String) -> Result<(), String> {
    do_create_file(&path)
}

#[tauri::command]
pub fn create_dir(path: String) -> Result<(), String> {
    do_create_dir(&path)
}

#[tauri::command]
pub fn rename_path(from: String, to: String) -> Result<(), String> {
    do_rename_path(&from, &to)
}

#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    trash::delete(Path::new(&path))
        .map_err(|e| format!("Cannot delete: {e}"))
}

#[cfg(test)]
mod tests {
    // (paste tests from Step 1 here)
}
```

- [ ] **Step 4: Run tests — expect pass**

```bash
cd engine/editor && cargo test file_explorer::ops
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/src/file_explorer/ops.rs
git commit -m "feat(editor): file explorer file operation commands"
```

---

## Task 6: Register commands in Tauri builder

**Files:**
- Modify: `engine/editor/src-tauri/lib.rs`

- [ ] **Step 1: Add module declaration and managed state**

In `engine/editor/src-tauri/lib.rs`, after the existing `pub mod` lines at the top, add:

```rust
pub mod file_explorer;
```

- [ ] **Step 2: Add FileWatcherState to managed state**

In `lib.rs`, in the `tauri::Builder::default()` chain, after `.manage(commands::NativeViewportState::new())`, add:

```rust
.manage(file_explorer::FileWatcherState::new())
```

- [ ] **Step 3: Register all file explorer commands**

In the `tauri::generate_handler![...]` list, add:

```rust
file_explorer::get_file_tree,
file_explorer::expand_dir,
file_explorer::get_git_status,
file_explorer::start_file_watch,
file_explorer::stop_file_watch,
file_explorer::open_in_editor,
file_explorer::create_file,
file_explorer::create_dir,
file_explorer::rename_path,
file_explorer::delete_path,
```

- [ ] **Step 4: Verify full build**

```bash
cd engine/editor && cargo build 2>&1 | grep "^error" | head -20
```

Expected: no errors.

- [ ] **Step 5: Run all Rust tests**

```bash
cd engine/editor && cargo test 2>&1 | tail -10
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src-tauri/lib.rs
git commit -m "feat(editor): register file explorer Tauri commands"
```

---

## Task 7: i18n keys

**Files:**
- Modify: `engine/editor/src/lib/i18n/locales/en.ts`

- [ ] **Step 1: Add keys to en.ts**

After the `'panel.assets': 'Assets',` line, add:

```ts
'panel.file_explorer': 'File Explorer',
```

After `'console.no_logs': 'No logs',`, add a new section:

```ts
// File explorer
'explorer.new_file': 'New File',
'explorer.new_folder': 'New Folder',
'explorer.rename': 'Rename',
'explorer.delete': 'Delete',
'explorer.copy_path': 'Copy Path',
'explorer.reveal': 'Reveal in Explorer',
'explorer.refresh': 'Refresh',
'explorer.show_ignored': 'Show Ignored Files',
'explorer.empty': 'No files',
'explorer.error': 'Could not read folder',
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd engine/editor && npm run check 2>&1 | grep "error" | head -10
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/i18n/locales/en.ts
git commit -m "feat(editor): add file explorer i18n keys"
```

---

## Task 8: File explorer store

**Files:**
- Create: `engine/editor/src/lib/stores/file-explorer.ts`

- [ ] **Step 1: Create store**

```typescript
// engine/editor/src/lib/stores/file-explorer.ts
import { invoke, listen } from '@tauri-apps/api/core';

export interface TreeNode {
  name: string;
  path: string;
  kind: 'file' | 'dir';
  children: TreeNode[] | null; // null = not expanded yet
  git_status: string | null;
  ignored: boolean;
}

export interface FileExplorerState {
  root: string | null;
  nodes: TreeNode[];
  expanded: Set<string>;
  selected: string | null;
  gitStatus: Record<string, string>;
  showIgnored: boolean;
  loading: boolean;
  error: string | null;
}

let state: FileExplorerState = {
  root: null,
  nodes: [],
  expanded: new Set(),
  selected: null,
  gitStatus: {},
  showIgnored: false,
  loading: false,
  error: null,
};

let listeners: (() => void)[] = [];

function notify() {
  listeners.forEach((fn) => fn());
}

export function getFileExplorerState(): FileExplorerState {
  return state;
}

export function subscribeFileExplorer(fn: () => void): () => void {
  listeners.push(fn);
  return () => { listeners = listeners.filter((l) => l !== fn); };
}

export async function loadTree(root: string): Promise<void> {
  state = { ...state, root, loading: true, error: null };
  notify();
  try {
    const nodes = await invoke<TreeNode[]>('get_file_tree', { root });
    const gitStatus = await invoke<Record<string, string>>('get_git_status', { root });
    state = { ...state, nodes, gitStatus, loading: false };
  } catch (e) {
    state = { ...state, loading: false, error: String(e) };
  }
  notify();
}

export async function expandDir(path: string): Promise<void> {
  try {
    const children = await invoke<TreeNode[]>('expand_dir', { path });
    state = {
      ...state,
      expanded: new Set([...state.expanded, path]),
      nodes: patchChildren(state.nodes, path, children),
    };
  } catch (e) {
    state = { ...state, error: String(e) };
  }
  notify();
}

export function collapseDir(path: string): void {
  const next = new Set(state.expanded);
  next.delete(path);
  state = { ...state, expanded: next };
  notify();
}

export function setSelected(path: string | null): void {
  state = { ...state, selected: path };
  notify();
}

export function toggleShowIgnored(): void {
  state = { ...state, showIgnored: !state.showIgnored };
  notify();
}

export async function refreshTree(): Promise<void> {
  if (!state.root) return;
  await loadTree(state.root);
}

/** Recursively replace children of the node at targetPath */
function patchChildren(nodes: TreeNode[], targetPath: string, children: TreeNode[]): TreeNode[] {
  return nodes.map((node) => {
    if (node.path === targetPath) return { ...node, children };
    if (node.children) return { ...node, children: patchChildren(node.children, targetPath, children) };
    return node;
  });
}
```

- [ ] **Step 2: Verify TypeScript**

```bash
cd engine/editor && npm run check 2>&1 | grep "error" | head -10
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/stores/file-explorer.ts
git commit -m "feat(editor): file explorer store — tree state, expansion, git status"
```

---

## Task 9: FileTreeNode component

**Files:**
- Create: `engine/editor/src/lib/components/FileTreeNode.svelte`

- [ ] **Step 1: Create FileTreeNode.svelte**

```svelte
<!-- engine/editor/src/lib/components/FileTreeNode.svelte -->
<script lang="ts">
  import type { TreeNode } from '$lib/stores/file-explorer';
  import {
    expandDir,
    collapseDir,
    setSelected,
    getFileExplorerState,
  } from '$lib/stores/file-explorer';
  import { invoke } from '@tauri-apps/api/core';
  import { t } from '$lib/i18n';

  let {
    node,
    depth = 0,
    showIgnored = false,
    selected = null,
    gitStatus = {},
  }: {
    node: TreeNode;
    depth?: number;
    showIgnored?: boolean;
    selected?: string | null;
    gitStatus?: Record<string, string>;
  } = $props();

  let isExpanded = $derived(getFileExplorerState().expanded.has(node.path));
  let isSelected = $derived(selected === node.path);
  let status = $derived(gitStatus[node.path] ?? node.git_status ?? null);

  let renaming = $state(false);
  let newName = $state('');

  const GIT_COLORS: Record<string, string> = {
    modified: 'var(--color-warn)',
    untracked: 'var(--color-success)',
    deleted: 'var(--color-error)',
    staged: 'var(--color-info)',
  };

  function handleClick() {
    setSelected(node.path);
    if (node.kind === 'dir') {
      isExpanded ? collapseDir(node.path) : expandDir(node.path);
    } else {
      invoke('open_in_editor', { path: node.path }).catch((e: string) => {
        // Toast handled in wrapper
        console.error(e);
      });
    }
  }

  async function startRename() {
    newName = node.name;
    renaming = true;
  }

  async function confirmRename() {
    if (!newName.trim() || newName === node.name) { renaming = false; return; }
    const dir = node.path.substring(0, node.path.length - node.name.length);
    const to = dir + newName;
    await invoke('rename_path', { from: node.path, to }).catch(console.error);
    renaming = false;
  }

  function cancelRename() { renaming = false; }

  async function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    // Basic context menu — expanded in future iterations
    const action = await showContextMenu(node);
    if (!action) return;
    if (action === 'rename') startRename();
    if (action === 'delete') await invoke('delete_path', { path: node.path }).catch(console.error);
    if (action === 'copy_path') navigator.clipboard.writeText(node.path);
    if (action === 'new_file') {
      const name = prompt(t('explorer.new_file'));
      if (name) await invoke('create_file', { path: node.path + '/' + name }).catch(console.error);
    }
    if (action === 'new_folder') {
      const name = prompt(t('explorer.new_folder'));
      if (name) await invoke('create_dir', { path: node.path + '/' + name }).catch(console.error);
    }
  }

  // Placeholder — replaced with a proper context menu component in a follow-up
  async function showContextMenu(_node: TreeNode): Promise<string | null> {
    const options = ['rename', 'delete', 'copy_path'];
    if (_node.kind === 'dir') options.unshift('new_file', 'new_folder');
    return prompt('Action: ' + options.join(', ')) ?? null;
  }

  let visibleChildren = $derived(
    node.children
      ? node.children.filter((c) => showIgnored || !c.ignored)
      : []
  );
</script>

{#if !node.ignored || showIgnored}
  <div
    class="tree-node"
    class:selected={isSelected}
    style:padding-left="{depth * 12 + 4}px"
    style:opacity={node.ignored ? 0.5 : 1}
    onclick={handleClick}
    oncontextmenu={handleContextMenu}
    role="treeitem"
    aria-selected={isSelected}
    aria-expanded={node.kind === 'dir' ? isExpanded : undefined}
  >
    <!-- Chevron for dirs -->
    <span class="chevron">
      {#if node.kind === 'dir'}
        {isExpanded ? '▼' : '▶'}
      {:else}
        &nbsp;
      {/if}
    </span>

    <!-- Icon -->
    <span class="icon">{node.kind === 'dir' ? '📁' : '📄'}</span>

    <!-- Name or inline rename input -->
    {#if renaming}
      <input
        class="rename-input"
        bind:value={newName}
        onkeydown={(e) => { if (e.key === 'Enter') confirmRename(); if (e.key === 'Escape') cancelRename(); }}
        onblur={cancelRename}
        autofocus
        onclick={(e) => e.stopPropagation()}
      />
    {:else}
      <span class="name">{node.name}</span>
    {/if}

    <!-- Git badge -->
    {#if status}
      <span class="git-badge" style:color={GIT_COLORS[status] ?? 'inherit'}>
        {status[0].toUpperCase()}
      </span>
    {/if}
  </div>

  <!-- Children (recursive) -->
  {#if node.kind === 'dir' && isExpanded && node.children !== null}
    {#each visibleChildren as child (child.path)}
      <svelte:self
        node={child}
        depth={depth + 1}
        {showIgnored}
        {selected}
        {gitStatus}
      />
    {/each}
  {/if}
{/if}

<style>
  .tree-node {
    display: flex;
    align-items: center;
    gap: 4px;
    padding-top: 2px;
    padding-bottom: 2px;
    padding-right: 8px;
    cursor: pointer;
    user-select: none;
    border-radius: 3px;
    font-size: 13px;
    color: var(--color-text, #ccc);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tree-node:hover { background: var(--color-bgHover, #2a2a2a); }
  .tree-node.selected { background: var(--color-bgSelected, #2a3f5f); color: #fff; }
  .chevron { font-size: 10px; width: 12px; flex-shrink: 0; color: var(--color-textMuted, #888); }
  .icon { font-size: 14px; flex-shrink: 0; }
  .name { overflow: hidden; text-overflow: ellipsis; flex: 1; }
  .git-badge { font-size: 11px; font-weight: 600; flex-shrink: 0; margin-left: auto; }
  .rename-input {
    flex: 1;
    background: var(--color-bgInput, #1a1a1a);
    border: 1px solid var(--color-accent, #4a9eff);
    color: var(--color-text, #ccc);
    font-size: 13px;
    padding: 0 4px;
    border-radius: 2px;
    outline: none;
  }
</style>
```

- [ ] **Step 2: Verify TypeScript**

```bash
cd engine/editor && npm run check 2>&1 | grep "error" | head -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/components/FileTreeNode.svelte
git commit -m "feat(editor): FileTreeNode — recursive tree node with git badges and inline rename"
```

---

## Task 10: FileExplorerPanel component

**Files:**
- Create: `engine/editor/src/lib/docking/panels/FileExplorerPanel.svelte`

- [ ] **Step 1: Create FileExplorerPanel.svelte**

```svelte
<!-- engine/editor/src/lib/docking/panels/FileExplorerPanel.svelte -->
<script lang="ts">
  import type { FileExplorerState } from '$lib/stores/file-explorer';
  import { toggleShowIgnored, refreshTree } from '$lib/stores/file-explorer';
  import FileTreeNode from '$lib/components/FileTreeNode.svelte';
  import { t } from '$lib/i18n';

  let { state }: { state: FileExplorerState } = $props();

  let visibleRoots = $derived(
    state.nodes.filter((n) => state.showIgnored || !n.ignored)
  );
</script>

<div class="file-explorer">
  <!-- Header -->
  <div class="panel-header">
    <span class="panel-title">{t('panel.file_explorer')}</span>
    <div class="header-actions">
      <button
        class="icon-btn"
        title={t('explorer.show_ignored')}
        class:active={state.showIgnored}
        onclick={toggleShowIgnored}
        aria-label={t('explorer.show_ignored')}
      >
        👁
      </button>
      <button
        class="icon-btn"
        title={t('explorer.refresh')}
        onclick={refreshTree}
        aria-label={t('explorer.refresh')}
        disabled={state.loading}
      >
        ↻
      </button>
    </div>
  </div>

  <!-- Error bar -->
  {#if state.error}
    <div class="error-bar" role="alert">
      {t('explorer.error')}: {state.error}
    </div>
  {/if}

  <!-- Tree -->
  <div class="tree-scroll" role="tree" aria-label={t('panel.file_explorer')}>
    {#if state.loading}
      <div class="status-msg">Loading...</div>
    {:else if !state.root}
      <div class="status-msg">{t('placeholder.no_project')}</div>
    {:else if visibleRoots.length === 0}
      <div class="status-msg">{t('explorer.empty')}</div>
    {:else}
      {#each visibleRoots as node (node.path)}
        <FileTreeNode
          {node}
          depth={0}
          showIgnored={state.showIgnored}
          selected={state.selected}
          gitStatus={state.gitStatus}
        />
      {/each}
    {/if}
  </div>
</div>

<style>
  .file-explorer {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--color-bgPanel, #1e1e1e);
  }
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    border-bottom: 1px solid var(--color-border, #333);
    flex-shrink: 0;
  }
  .panel-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-textMuted, #888);
  }
  .header-actions { display: flex; gap: 4px; }
  .icon-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    color: var(--color-textMuted, #888);
    font-size: 14px;
    line-height: 1;
  }
  .icon-btn:hover { background: var(--color-bgHover, #2a2a2a); color: var(--color-text, #ccc); }
  .icon-btn.active { color: var(--color-accent, #4a9eff); }
  .icon-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .error-bar {
    background: var(--color-error, #c0392b);
    color: #fff;
    font-size: 12px;
    padding: 4px 8px;
    flex-shrink: 0;
  }
  .tree-scroll {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px 0;
  }
  .status-msg {
    padding: 8px;
    color: var(--color-textMuted, #888);
    font-size: 12px;
  }
</style>
```

- [ ] **Step 2: Verify TypeScript**

```bash
cd engine/editor && npm run check 2>&1 | grep "error" | head -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/docking/panels/FileExplorerPanel.svelte
git commit -m "feat(editor): FileExplorerPanel — panel shell with header and tree"
```

---

## Task 11: FileExplorerWrapper component

**Files:**
- Create: `engine/editor/src/lib/docking/panels/FileExplorerWrapper.svelte`

- [ ] **Step 1: Create FileExplorerWrapper.svelte**

```svelte
<!-- engine/editor/src/lib/docking/panels/FileExplorerWrapper.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';
  import FileExplorerPanel from './FileExplorerPanel.svelte';
  import {
    getFileExplorerState,
    subscribeFileExplorer,
    loadTree,
    refreshTree,
    type FileExplorerState,
  } from '$lib/stores/file-explorer';

  const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

  let state: FileExplorerState = $state(getFileExplorerState());
  let unsubscribe: (() => void) | null = null;
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    // Subscribe to store changes
    unsubscribe = subscribeFileExplorer(() => {
      state = getFileExplorerState();
    });

    if (!isTauri) return;

    // If a project is already loaded, start the watcher and load the tree
    try {
      const editorState = await invoke<{ project_path?: string }>('get_editor_state');
      if (editorState.project_path) {
        await invoke('start_file_watch', { root: editorState.project_path });
        await loadTree(editorState.project_path);
      }
    } catch (e) {
      console.warn('FileExplorerWrapper: could not load initial project state', e);
    }

    // Listen for file system changes from Rust watcher
    unlisten = await listen<{ root: string }>('file-tree-changed', async (event) => {
      await refreshTree();
    });
  });

  onDestroy(async () => {
    unsubscribe?.();
    unlisten?.();
    if (isTauri) {
      try { await invoke('stop_file_watch'); } catch { /* ignore */ }
    }
  });
</script>

<div class="panel-opaque">
  <FileExplorerPanel {state} />
</div>

<style>
  .panel-opaque {
    width: 100%;
    height: 100%;
    background: var(--color-bgPanel, #1e1e1e);
  }
</style>
```

- [ ] **Step 2: Verify TypeScript**

```bash
cd engine/editor && npm run check 2>&1 | grep "error" | head -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/src/lib/docking/panels/FileExplorerWrapper.svelte
git commit -m "feat(editor): FileExplorerWrapper — watcher lifecycle, store subscription"
```

---

## Task 12: Register panel in types.ts and App.svelte

**Files:**
- Modify: `engine/editor/src/lib/docking/types.ts`
- Modify: `engine/editor/src/App.svelte`

- [ ] **Step 1: Add to panelRegistry in types.ts**

In `engine/editor/src/lib/docking/types.ts`, add after the `assets` entry:

```ts
{ id: 'file-explorer', titleKey: 'panel.file_explorer' },
```

- [ ] **Step 2: Import wrapper in App.svelte**

In `engine/editor/src/App.svelte`, after the `AssetsPanel` import line, add:

```ts
import FileExplorerWrapper from './lib/docking/panels/FileExplorerWrapper.svelte';
```

- [ ] **Step 3: Add to panelComponents in App.svelte**

In the `panelComponents` record (around line 72), add:

```ts
'file-explorer': FileExplorerWrapper,
```

- [ ] **Step 4: Full type check**

```bash
cd engine/editor && npm run check 2>&1 | grep "error" | head -20
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src/lib/docking/types.ts engine/editor/src/App.svelte
git commit -m "feat(editor): register file-explorer panel in docking system"
```

---

## Task 13: End-to-end verification

- [ ] **Step 1: Build Rust**

```bash
cd engine/editor && cargo build 2>&1 | grep "^error" | head -10
```

Expected: no errors.

- [ ] **Step 2: Run all Rust tests**

```bash
cd engine/editor && cargo test 2>&1 | tail -15
```

Expected: all tests pass (tree: 3, git: 5, ops: 3, plus existing tests).

- [ ] **Step 3: Run TypeScript type check**

```bash
cd engine/editor && npm run check 2>&1 | grep "error" | head -10
```

Expected: no errors.

- [ ] **Step 4: Launch the editor**

```bash
cd engine/editor && npm run tauri dev
```

- [ ] **Step 5: Add the file-explorer panel via the panel restore menu**

In the editor: open a panel slot → choose "File Explorer" from the panel restore buttons in the title bar (or drag it from the available panels list).

Expected: panel appears, shows "No project loaded" placeholder.

- [ ] **Step 6: Open a Silmaril project**

Via File → Open Project, select a folder with a `game.toml`.

Expected:
- File tree populates with project root contents
- Dirs are collapsible
- Clicking a file opens it in `$EDITOR` / OS default
- Git-tracked files show M/U/D badges

- [ ] **Step 7: Verify file watcher**

Add a new file to the project folder from Windows Explorer.

Expected: file appears in the tree within ~1 second.

- [ ] **Step 8: Final commit**

```bash
git add -A
git commit -m "feat(editor): file explorer panel complete — tree, git status, file watch, operations"
```
