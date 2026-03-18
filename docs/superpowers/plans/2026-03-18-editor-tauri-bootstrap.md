# Editor Tauri Bootstrap — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Get the Silmaril Editor Tauri app opening a window with the Svelte shell UI rendering.

**Architecture:** The editor scaffold already exists at `engine/editor/`. This plan adds Tauri + Svelte dependencies, wires the config files, and connects the Rust backend to the Svelte frontend so `cargo tauri dev` opens a working window.

**Tech Stack:** Tauri 2, Svelte 5, Vite, shadcn-svelte

**Spec:** `docs/superpowers/specs/2026-03-18-editor-architecture-design.md`

---

## File Structure

### Files to modify

| File | Change |
|------|--------|
| `engine/editor/Cargo.toml` | Add `tauri`, `tauri-build` dependencies |
| `engine/editor/package.json` | Add Svelte, Vite, Tauri JS deps |
| `engine/editor/svelte.config.js` | Real Svelte config |
| `engine/editor/vite.config.ts` | Svelte + Tauri Vite plugin |
| `engine/editor/tsconfig.json` | Proper TS config for Svelte |
| `engine/editor/src-tauri/main.rs` | Real Tauri app builder |
| `engine/editor/src-tauri/lib.rs` | Tauri command registration |
| `engine/editor/src/app.html` | Svelte mount point |
| `engine/editor/src/App.svelte` | Real shell with toolbar + panel areas |

### Files to create

| File | Purpose |
|------|---------|
| `engine/editor/src-tauri/tauri.conf.json` | Tauri window config |
| `engine/editor/src-tauri/build.rs` | Tauri build script |
| `engine/editor/src-tauri/capabilities/default.json` | Tauri 2 permissions |
| `engine/editor/src/main.ts` | Svelte app entry point |
| `engine/editor/src/vite-env.d.ts` | Vite type declarations |

---

## Task 1: Install npm dependencies

**Files:**
- Modify: `engine/editor/package.json`

- [ ] **Step 1: Install Svelte + Vite + Tauri JS deps**

```bash
cd engine/editor
npm install --save-dev svelte @sveltejs/vite-plugin-svelte vite typescript
npm install @tauri-apps/api@^2
```

- [ ] **Step 2: Verify package.json has deps**

Check that `package.json` now has `svelte`, `@sveltejs/vite-plugin-svelte`, `vite`, `@tauri-apps/api` listed.

- [ ] **Step 3: Commit**

```bash
git add engine/editor/package.json engine/editor/package-lock.json engine/editor/node_modules/
# Actually, add node_modules to .gitignore (already done), so:
git add engine/editor/package.json engine/editor/package-lock.json
git commit -m "feat(editor): install Svelte + Vite + Tauri JS dependencies"
```

---

## Task 2: Configure Tauri 2

**Files:**
- Create: `engine/editor/src-tauri/tauri.conf.json`
- Create: `engine/editor/src-tauri/build.rs`
- Create: `engine/editor/src-tauri/capabilities/default.json`
- Modify: `engine/editor/Cargo.toml`

- [ ] **Step 1: Create tauri.conf.json**

```json
{
  "$schema": "https://raw.githubusercontent.com/nicosalm/tauri-docs/refs/heads/dev/tooling/cli/schema.json",
  "productName": "Silmaril Editor",
  "version": "0.1.0",
  "identifier": "com.silmaril.editor",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      {
        "title": "Silmaril Editor",
        "width": 1400,
        "height": 900,
        "resizable": true,
        "fullscreen": false,
        "decorations": true
      }
    ]
  }
}
```

- [ ] **Step 2: Create build.rs**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 3: Create capabilities/default.json**

```json
{
  "$schema": "https://raw.githubusercontent.com/nicosalm/tauri-docs/refs/heads/dev/tooling/cli/schema.json",
  "identifier": "default",
  "description": "Default capabilities for the Silmaril Editor",
  "windows": ["main"],
  "permissions": [
    "core:default"
  ]
}
```

- [ ] **Step 4: Update Cargo.toml**

Add Tauri dependencies:

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-build = { version = "2", features = [] }

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

Keep existing `engine-ops`, `engine-core`, `anyhow`, `tracing`, `serde`, `serde_json` deps.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/tauri.conf.json engine/editor/src-tauri/build.rs engine/editor/src-tauri/capabilities/ engine/editor/Cargo.toml
git commit -m "feat(editor): add Tauri 2 configuration and build script"
```

---

## Task 3: Configure Svelte + Vite

**Files:**
- Modify: `engine/editor/svelte.config.js`
- Modify: `engine/editor/vite.config.ts`
- Modify: `engine/editor/tsconfig.json`
- Create: `engine/editor/src/main.ts`
- Create: `engine/editor/src/vite-env.d.ts`
- Modify: `engine/editor/src/app.html`

- [ ] **Step 1: Update svelte.config.js**

```js
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
};
```

- [ ] **Step 2: Update vite.config.ts**

```typescript
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: 'ws', host, port: 5174 }
      : undefined,
  },
});
```

- [ ] **Step 3: Update tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true
  },
  "include": ["src/**/*", "src/**/*.svelte"]
}
```

- [ ] **Step 4: Create src/main.ts**

```typescript
import App from './App.svelte';
import { mount } from 'svelte';

const app = mount(App, {
  target: document.getElementById('app')!,
});

export default app;
```

- [ ] **Step 5: Create src/vite-env.d.ts**

```typescript
/// <reference types="svelte" />
/// <reference types="vite/client" />
```

- [ ] **Step 6: Update src/app.html**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Silmaril Editor</title>
    <style>
      html, body { margin: 0; padding: 0; height: 100%; overflow: hidden; }
    </style>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 7: Verify Vite dev server starts**

```bash
cd engine/editor
npm run dev
```

Expected: Vite dev server starts on localhost:5173, shows the Svelte shell.

- [ ] **Step 8: Commit**

```bash
git add engine/editor/svelte.config.js engine/editor/vite.config.ts engine/editor/tsconfig.json engine/editor/src/main.ts engine/editor/src/vite-env.d.ts engine/editor/src/app.html
git commit -m "feat(editor): configure Svelte 5 + Vite with Tauri dev server support"
```

---

## Task 4: Wire Tauri Rust backend

**Files:**
- Modify: `engine/editor/src-tauri/main.rs`
- Modify: `engine/editor/src-tauri/lib.rs`
- Modify: `engine/editor/src-tauri/bridge/commands.rs`

- [ ] **Step 1: Update main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    silmaril_editor::run();
}
```

- [ ] **Step 2: Update lib.rs**

```rust
pub mod bridge;
pub mod plugins;
pub mod state;
pub mod viewport;
pub mod world;

use bridge::commands;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_editor_state,
            commands::open_project,
        ])
        .run(tauri::generate_context!())
        .expect("error running Silmaril Editor");
}
```

- [ ] **Step 3: Add initial Tauri commands**

Update `engine/editor/src-tauri/bridge/commands.rs`:

```rust
use crate::state::EditorMode;
use serde::Serialize;

#[derive(Serialize)]
pub struct EditorStateResponse {
    pub mode: String,
    pub project_name: Option<String>,
    pub project_path: Option<String>,
}

#[tauri::command]
pub fn get_editor_state() -> EditorStateResponse {
    EditorStateResponse {
        mode: "edit".to_string(),
        project_name: None,
        project_path: None,
    }
}

#[tauri::command]
pub fn open_project(path: String) -> Result<EditorStateResponse, String> {
    let project_root = std::path::Path::new(&path);
    if !project_root.join("game.toml").exists() {
        return Err("No game.toml found in selected directory".to_string());
    }

    let game_toml = std::fs::read_to_string(project_root.join("game.toml"))
        .map_err(|e| e.to_string())?;
    let name = engine_ops::build::parse_project_name(&game_toml)
        .unwrap_or_else(|| "Unknown Project".to_string());

    Ok(EditorStateResponse {
        mode: "edit".to_string(),
        project_name: Some(name),
        project_path: Some(path),
    })
}
```

- [ ] **Step 4: Verify Tauri app launches**

```bash
cd engine/editor
cargo tauri dev
```

Expected: A window opens titled "Silmaril Editor" showing the Svelte shell UI.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/
git commit -m "feat(editor): wire Tauri backend with initial commands"
```

---

## Task 5: Update Svelte shell to call Tauri

**Files:**
- Modify: `engine/editor/src/App.svelte`
- Modify: `engine/editor/src/lib/api.ts`

- [ ] **Step 1: Update api.ts with real Tauri calls**

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface EditorState {
  mode: string;
  project_name: string | null;
  project_path: string | null;
}

export async function getEditorState(): Promise<EditorState> {
  return await invoke('get_editor_state');
}

export async function openProject(path: string): Promise<EditorState> {
  return await invoke('open_project', { path });
}
```

- [ ] **Step 2: Update App.svelte to show project state**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { getEditorState, type EditorState } from './lib/api';
  import PanelShell from './lib/components/PanelShell.svelte';

  let editorState: EditorState | null = $state(null);

  onMount(async () => {
    editorState = await getEditorState();
  });
</script>

<main class="editor-shell">
  <div class="toolbar">
    <span class="title">Silmaril Editor</span>
    {#if editorState?.project_name}
      <span class="project-name">— {editorState.project_name}</span>
    {/if}
    <div class="toolbar-spacer"></div>
    <span class="mode-badge">{editorState?.mode ?? 'loading...'}</span>
  </div>

  <div class="main-area">
    <div class="sidebar-left">
      <PanelShell title="Hierarchy">
        <p class="placeholder">No project loaded</p>
      </PanelShell>
    </div>

    <div class="viewport">
      <PanelShell title="Viewport">
        <div class="viewport-placeholder">
          <p>Vulkan viewport will render here</p>
        </div>
      </PanelShell>
    </div>

    <div class="sidebar-right">
      <PanelShell title="Inspector">
        <p class="placeholder">Select an entity</p>
      </PanelShell>
    </div>
  </div>

  <div class="bottom-bar">
    <PanelShell title="Console">
      <p class="placeholder">No logs yet</p>
    </PanelShell>
  </div>
</main>

<style>
  .editor-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1e1e1e;
    color: #cccccc;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 13px;
  }
  .toolbar {
    height: 40px;
    display: flex;
    align-items: center;
    padding: 0 16px;
    background: #2d2d2d;
    border-bottom: 1px solid #404040;
    gap: 8px;
  }
  .title { font-weight: 600; font-size: 14px; }
  .project-name { color: #999; font-size: 13px; }
  .toolbar-spacer { flex: 1; }
  .mode-badge {
    padding: 2px 8px;
    background: #007acc;
    border-radius: 3px;
    font-size: 11px;
    text-transform: uppercase;
    font-weight: 600;
  }
  .main-area {
    flex: 1;
    display: flex;
    overflow: hidden;
  }
  .sidebar-left { width: 250px; border-right: 1px solid #404040; }
  .sidebar-right { width: 300px; border-left: 1px solid #404040; }
  .viewport { flex: 1; }
  .bottom-bar { height: 200px; border-top: 1px solid #404040; }
  .viewport-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #666;
  }
  .placeholder { color: #666; font-style: italic; padding: 8px; }

  .sidebar-left, .sidebar-right, .viewport, .bottom-bar {
    display: flex;
    flex-direction: column;
  }
  .sidebar-left :global(.panel),
  .sidebar-right :global(.panel),
  .viewport :global(.panel),
  .bottom-bar :global(.panel) {
    flex: 1;
  }
</style>
```

- [ ] **Step 3: Test full flow**

```bash
cd engine/editor
cargo tauri dev
```

Expected: Window opens with toolbar ("Silmaril Editor", mode badge "edit"), three-column layout (Hierarchy | Viewport | Inspector), and bottom Console panel. All panels show placeholder text.

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src/
git commit -m "feat(editor): connect Svelte shell to Tauri backend with panel layout"
```

---

## Task 6: Verify and screenshot

- [ ] **Step 1: Final verification**

```bash
cd engine/editor
cargo tauri dev
```

Verify:
- Window opens at 1400x900
- Title bar shows "Silmaril Editor"
- Dark theme renders correctly
- Panel layout: Hierarchy (left) | Viewport (center) | Inspector (right) | Console (bottom)
- Mode badge shows "edit"
- No console errors in browser DevTools (F12)

- [ ] **Step 2: Commit everything**

```bash
git add -A
git commit -m "feat(editor): complete Tauri bootstrap — window opens with panel shell"
```

---

## Summary

| Task | What it does |
|------|-------------|
| 1 | Install npm deps (Svelte, Vite, Tauri JS) |
| 2 | Configure Tauri 2 (tauri.conf.json, build.rs, capabilities) |
| 3 | Configure Svelte + Vite (plugins, TS config, entry point) |
| 4 | Wire Tauri Rust backend (commands, app builder) |
| 5 | Update Svelte shell with real Tauri calls + panel layout |
| 6 | Verify window opens correctly |
