# Phase 0.8: Silmaril Editor Foundation - Tauri + Svelte + shadcn-svelte

**Priority:** 🟡 **MEDIUM - After CLI is working**

**Status:** ⚪ Not Started (0%)

**Time Estimate:** 3-4 weeks

---

## Overview

The Silmaril Editor is a **native desktop application** built with Tauri 2, Svelte 5, and shadcn-svelte. It provides a visual interface for game development, with an embedded Vulkan viewport for real-time preview and integrated AI assistant.

**Philosophy:**
- Editor is **optional** - CLI (`silm`) is primary workflow
- **Hybrid architecture** - Native Vulkan viewport + Web UI (Svelte)
- **AI-first** - Built-in chat for code generation and debugging
- **Code-centric** - Editor writes code files, not binary assets
- **VSCode-style project discovery** - Directory-based, no extra files required

---

## Goals

- ✅ Tauri app with native window
- ✅ Svelte 5 UI with shadcn-svelte components
- ✅ Native Vulkan viewport (embedded in Tauri window)
- ✅ Hierarchy panel (entity tree)
- ✅ Inspector panel (component editor)
- ✅ Assets panel (file browser)
- ✅ Console panel (logs)
- ✅ Basic AI chat panel (Phase 4 will add full features)
- ✅ Project loading (open game.toml)
- ✅ Playback controls (play/pause/stop)

---

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│  Tauri Native Window (600 KB bundle)                      │
│  ┌──────────────────────┬────────────────────────────────┐ │
│  │                      │                                │ │
│  │  Native Vulkan       │   Svelte UI (shadcn-svelte)   │ │
│  │  Viewport            │   ┌────────────────────────┐   │ │
│  │  (Game Renders Here) │   │ Hierarchy Panel        │   │ │
│  │                      │   │ ┌────────────────────┐ │   │ │
│  │  [3D Scene Preview]  │   │ │ 📦 World           │ │   │ │
│  │                      │   │ │   🎯 Player        │ │   │ │
│  │                      │   │ │   👾 Enemy (5)     │ │   │ │
│  │                      │   │ └────────────────────┘ │   │ │
│  │                      │   │                        │   │ │
│  │                      │   │ Inspector Panel        │   │ │
│  │                      │   │ ┌────────────────────┐ │   │ │
│  │                      │   │ │ Transform          │ │   │ │
│  │                      │   │ │ Health             │ │   │ │
│  │                      │   │ └────────────────────┘ │   │ │
│  │                      │   │                        │   │ │
│  │                      │   │ Assets Panel           │   │ │
│  │                      │   │ Console Panel          │   │ │
│  │                      │   │ AI Chat Panel (Basic)  │   │ │
│  └──────────────────────┴────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

---

## Project Discovery Approach (VSCode-Style Hybrid)

Following modern editor UX patterns (VSCode, Sublime Text), Silmaril uses a **directory-based** approach with optional workspace files for multi-project scenarios.

### **Primary Method: Folder-Based (No Extra Files)**

**How it works:**
1. User opens editor → sees welcome screen with "Open Folder" button
2. Clicks "Open Folder" → Tauri dialog picker appears
3. Selects project directory (e.g., `/home/user/my-game`)
4. Editor validates by checking for `game.toml` in selected folder
5. If valid, loads project and displays entities

**Why this approach:**
- ✅ **Zero friction** - Just open any Silmaril project folder
- ✅ **AI-friendly** - Agents detect projects by scanning for `game.toml`
- ✅ **Git-committable** - No binary or generated project files
- ✅ **Code-first** - Aligns perfectly with Rust workspace structure
- ✅ **Familiar** - Matches VSCode mental model developers already know

### **Recent Projects List**

**Storage:** `~/.silmaril/editor-config.toml`

```toml
# ~/.silmaril/editor-config.toml
[[recent_projects]]
name = "my-mmo"
path = "/home/user/projects/my-mmo"
last_opened = "2026-02-03T10:30:00Z"

[[recent_projects]]
name = "platformer"
path = "/home/user/projects/platformer"
last_opened = "2026-02-02T15:20:00Z"
```

**Features:**
- Shows last 10 opened projects
- Sorted by last opened date
- Quick access from welcome screen
- Persists across editor sessions

### **Optional: Multi-Project Workspaces (Phase 4.9)**

For advanced scenarios (working on game + shared modules simultaneously):

**`.silmaril-workspace.json`** (similar to VSCode's `.code-workspace`):
```json
{
  "version": "1.0",
  "name": "My Game Dev Workspace",
  "projects": [
    {
      "name": "Main Game",
      "path": "./my-game",
      "primary": true
    },
    {
      "name": "Combat Module",
      "path": "../combat-module",
      "type": "module"
    }
  ],
  "settings": {
    "viewport": {
      "split_view": true
    }
  }
}
```

**Features (Phase 4.9):**
- Double-click `.silmaril-workspace` to open editor with all projects
- File association with OS
- Tabbed interface for multiple projects
- Shared settings across projects

### **Project Discovery Flow**

```
┌─────────────────────────────────────────┐
│  Editor Startup                         │
├─────────────────────────────────────────┤
│  1. Show welcome screen with:           │
│     - Recent Projects list              │
│     - "Open Folder" button              │
│     - "Open Workspace" button (4.9)     │
└─────────────────────────────────────────┘
              │
              ▼
    ┌──────────────────────┐
    │  User clicks "Open    │
    │  Folder"             │
    └──────────────────────┘
              │
              ▼
    ┌──────────────────────┐
    │  Tauri Dialog Picker │
    │  (Directory mode)    │
    └──────────────────────┘
              │
              ▼
    ┌──────────────────────┐
    │  Validate:           │
    │  - game.toml exists? │
    │  - Cargo.toml exists?│
    │  - shared/ dir?      │
    └──────────────────────┘
              │
         ┌────┴────┐
         │         │
      Valid    Invalid
         │         │
         ▼         ▼
    ┌────────┐  ┌──────────┐
    │ Load   │  │ Show     │
    │Project │  │ Error    │
    └────────┘  │ Toast    │
         │      └──────────┘
         ▼
    ┌──────────────────────┐
    │  Load project:       │
    │  - Parse game.toml   │
    │  - Parse Cargo.toml  │
    │  - Load world state  │
    │  - Update recent list│
    │  - Display entities  │
    └──────────────────────┘
```

### **Why Not .uproject or .silmaril Files?**

**Considered alternatives:**

1. **Unreal-style .silmaril project files**
   - ❌ Duplicates info from game.toml and Cargo.toml
   - ❌ Another file to maintain
   - ❌ Against minimalist code-first philosophy
   - ❌ Not common in Rust ecosystem

2. **.NET-style solution files**
   - ❌ Overly complex for single-project scenarios
   - ❌ Duplicates workspace concept
   - ❌ Not used by any major game engine (Unity/Godot/Unreal)

**Our approach wins because:**
- Uses existing `game.toml` as natural project marker
- Scales gracefully (single project → multi-project workspace)
- Matches modern editor UX (VSCode, Sublime, Zed)
- Zero overhead for simple cases
- Optional complexity for advanced cases

---

## Task Breakdown

### **EDITOR.1: Tauri Project Setup (3 days)**

**Create editor crate:**
```
engine/editor/
├── Cargo.toml
├── src-tauri/                # Rust backend
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs           # Tauri entry point
│   │   ├── lib.rs            # Editor logic
│   │   ├── commands.rs       # Tauri commands
│   │   └── state.rs          # App state
│   └── build.rs
├── src/                      # Svelte frontend
│   ├── App.svelte            # Root component
│   ├── main.ts               # Entry point
│   ├── lib/
│   │   ├── components/       # Svelte components
│   │   │   ├── Hierarchy.svelte
│   │   │   ├── Inspector.svelte
│   │   │   ├── Assets.svelte
│   │   │   ├── Console.svelte
│   │   │   └── AIChat.svelte
│   │   └── stores.ts         # Svelte stores (state)
│   └── styles/
│       └── app.css
├── package.json
├── vite.config.ts
├── tsconfig.json
└── tauri.conf.json
```

**Dependencies (Rust):**
- `tauri` (v2) - Native app framework
- `silmaril-core` - Engine integration
- `silmaril-renderer` - Vulkan renderer
- `serde` / `serde_json` - Serialization
- `tokio` - Async runtime
- `tracing` - Logging

**Dependencies (JS):**
- `@tauri-apps/api` - Tauri API
- `svelte` (v5) - UI framework
- `shadcn-svelte` - Component library
- `vite` - Build tool
- `typescript` - Type safety

**Setup tasks:**
- [ ] Create Tauri project (`npm create tauri-app`)
- [ ] Configure for Svelte 5
- [ ] Install shadcn-svelte
- [ ] Configure Rust workspace
- [ ] Add to workspace members
- [ ] CI includes editor build

**Deliverables:**
- [ ] Editor crate structure
- [ ] Tauri app opens (hello world)
- [ ] Svelte UI renders
- [ ] shadcn-svelte components working

---

### **EDITOR.2: Native Vulkan Viewport (5 days)**

**Goal:** Embed native Vulkan rendering inside Tauri window.

**Architecture:**
```rust
// src-tauri/src/main.rs
use tauri::{Manager, Window};
use silmaril_renderer::VulkanRenderer;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_window("main").unwrap();

            // Get native window handle (platform-specific)
            let handle = window.hwnd().unwrap(); // Windows
            // let handle = window.ns_window().unwrap(); // macOS
            // let handle = window.gtk_window().unwrap(); // Linux

            // Create Vulkan surface from native handle
            let renderer = VulkanRenderer::from_raw_window_handle(handle)?;

            // Store in app state
            app.manage(EditorState {
                renderer,
                world: World::new(),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            render_frame,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn render_frame(state: State<EditorState>) -> Result<(), String> {
    let mut renderer = state.renderer.lock();
    let world = state.world.lock();

    // Render game world
    renderer.render(&world)?;

    Ok(())
}
```

**Tauri window layout:**
```html
<!-- src/App.svelte -->
<div class="editor-layout">
  <!-- Native Vulkan viewport (Tauri manages this area) -->
  <div id="viewport" class="viewport">
    <!-- Vulkan renders directly here -->
  </div>

  <!-- Svelte UI panels (web) -->
  <div class="panels">
    <Hierarchy />
    <Inspector />
  </div>
</div>

<style>
  .viewport {
    width: 60%;
    height: 100vh;
    background: #1a1a1a;
  }

  .panels {
    width: 40%;
    height: 100vh;
  }
</style>
```

**Implementation tasks:**
- [ ] Get native window handle from Tauri
- [ ] Create Vulkan surface (platform-specific)
- [ ] Integrate VulkanRenderer
- [ ] Render loop (60 FPS)
- [ ] Handle window resize
- [ ] Mouse input (camera controls)
- [ ] Keyboard input (editor shortcuts)

**Tests:**
- [ ] Viewport renders clear color
- [ ] Viewport resizes correctly
- [ ] Mouse input works
- [ ] Keyboard input works

**Deliverables:**
- [ ] Native Vulkan viewport working
- [ ] Renders at 60 FPS
- [ ] Resizes correctly
- [ ] Input working

---

### **EDITOR.3: Project Discovery & Loading (VSCode-Style Hybrid) (4 days)**

**Goal:** Implement directory-based project discovery with optional workspace files (inspired by VSCode)

**Philosophy:**
- **Primary: Directory-based** - Just open any Silmaril project folder (no extra files required)
- **Validation: game.toml** - Presence of `game.toml` confirms it's a Silmaril project
- **Recent projects list** - Stored in `~/.silmaril/editor-config.toml` for quick access
- **Optional: Workspace files** - `.silmaril-workspace.json` for multi-project scenarios (Phase 4.9)
- **AI-friendly** - Agents can detect projects by scanning for `game.toml`
- **Git-committable** - No binary or generated project files

---

#### **3.1: Welcome Screen with Recent Projects (Day 1)**

**UI Layout:**
```svelte
<!-- src/lib/components/WelcomeScreen.svelte -->
<script lang="ts">
  import { Card, Button } from '$lib/components/ui';
  import { invoke } from '@tauri-apps/api/tauri';
  import { open } from '@tauri-apps/plugin-dialog';

  interface RecentProject {
    name: string;
    path: string;
    last_opened: string;
  }

  let recentProjects: RecentProject[] = [];

  onMount(async () => {
    recentProjects = await invoke('get_recent_projects');
  });

  async function openFolder() {
    const selected = await open({
      directory: true,
      multiple: false
    });

    if (selected) {
      await invoke('open_project_folder', { path: selected });
    }
  }

  async function openRecent(project: RecentProject) {
    await invoke('open_project_folder', { path: project.path });
  }
</script>

<div class="welcome-screen">
  <div class="hero">
    <h1>Welcome to Silmaril Editor</h1>
    <p>Code-first game development with AI assistance</p>
  </div>

  <div class="actions">
    <Button on:click={openFolder} size="lg">
      📁 Open Folder
    </Button>
    <Button on:click={openWorkspace} size="lg" variant="outline">
      📋 Open Workspace
    </Button>
  </div>

  {#if recentProjects.length > 0}
    <div class="recent-projects">
      <h2>Recent Projects</h2>
      {#each recentProjects as project}
        <Card class="project-card" on:click={() => openRecent(project)}>
          <h3>{project.name}</h3>
          <p class="path">{project.path}</p>
          <p class="date">{formatDate(project.last_opened)}</p>
        </Card>
      {/each}
    </div>
  {/if}
</div>
```

**Tauri Backend (Recent Projects):**
```rust
// src-tauri/src/config.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub recent_projects: Vec<RecentProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub name: String,
    pub path: PathBuf,
    pub last_opened: DateTime<Utc>,
}

impl EditorConfig {
    pub fn load() -> Result<Self, EditorError> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: EditorConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), EditorError> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf, EditorError> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))?;
        Ok(PathBuf::from(home).join(".silmaril/editor-config.toml"))
    }

    pub fn add_recent_project(&mut self, name: String, path: PathBuf) {
        // Remove if already exists
        self.recent_projects.retain(|p| p.path != path);

        // Add to front
        self.recent_projects.insert(0, RecentProject {
            name,
            path,
            last_opened: Utc::now(),
        });

        // Keep only last 10
        self.recent_projects.truncate(10);
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            recent_projects: Vec::new(),
        }
    }
}
```

**Tauri Command:**
```rust
// src-tauri/src/commands.rs
#[tauri::command]
async fn get_recent_projects() -> Result<Vec<RecentProject>, String> {
    let config = EditorConfig::load()
        .map_err(|e| format!("Failed to load config: {}", e))?;

    Ok(config.recent_projects)
}
```

**Implementation tasks:**
- [ ] Create WelcomeScreen.svelte component
- [ ] Implement EditorConfig struct with TOML serialization
- [ ] Implement get_recent_projects command
- [ ] Create "Open Folder" button with Tauri dialog
- [ ] Display recent projects list
- [ ] Handle project click events

**Tests:**
- [ ] EditorConfig loads from ~/.silmaril/editor-config.toml
- [ ] EditorConfig saves correctly
- [ ] Recent projects list shows last 10 projects
- [ ] Opening folder updates recent projects

---

#### **3.2: Folder Picker & Project Validation (Day 2)**

**Tauri Dialog Integration:**
```rust
// src-tauri/src/commands.rs
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
async fn open_project_folder(
    app: tauri::AppHandle,
    path: String,
) -> Result<ProjectInfo, String> {
    let project_path = PathBuf::from(&path);

    // 1. Validate: Check for game.toml
    let game_toml_path = project_path.join("game.toml");
    if !game_toml_path.exists() {
        return Err(format!(
            "Not a valid Silmaril project: game.toml not found in {}",
            path
        ));
    }

    // 2. Parse game.toml
    let game_toml_content = std::fs::read_to_string(&game_toml_path)
        .map_err(|e| format!("Failed to read game.toml: {}", e))?;

    let game_config: GameToml = toml::from_str(&game_toml_content)
        .map_err(|e| format!("Failed to parse game.toml: {}", e))?;

    // 3. Validate Cargo.toml exists (workspace)
    let cargo_toml_path = project_path.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Err(format!(
            "Invalid Silmaril project: Cargo.toml not found in {}",
            path
        ));
    }

    // 4. Check for expected directories
    let required_dirs = ["shared", "server", "client"];
    for dir in &required_dirs {
        let dir_path = project_path.join(dir);
        if !dir_path.exists() {
            tracing::warn!("Expected directory missing: {}", dir);
        }
    }

    // 5. Update recent projects
    let mut config = EditorConfig::load()
        .map_err(|e| format!("Failed to load config: {}", e))?;
    config.add_recent_project(
        game_config.game.name.clone(),
        project_path.clone()
    );
    config.save()
        .map_err(|e| format!("Failed to save config: {}", e))?;

    // 6. Return project info
    Ok(ProjectInfo {
        name: game_config.game.name,
        version: game_config.game.version,
        description: game_config.game.description,
        path: project_path.to_string_lossy().to_string(),
    })
}
```

**Error Handling UI:**
```svelte
<!-- src/lib/components/WelcomeScreen.svelte -->
<script lang="ts">
  import { toast } from 'svelte-sonner';

  async function openFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Open Silmaril Project Folder"
      });

      if (selected) {
        const project = await invoke('open_project_folder', { path: selected });
        // Navigate to editor view
        goto('/editor');
      }
    } catch (error) {
      // Show friendly error message
      toast.error('Failed to open project', {
        description: error.toString()
      });
    }
  }
</script>
```

**Implementation tasks:**
- [ ] Implement Tauri dialog folder picker
- [ ] Validate game.toml exists
- [ ] Parse game.toml and Cargo.toml
- [ ] Check for required directories (shared, server, client)
- [ ] Update recent projects list
- [ ] Show error toast for invalid projects
- [ ] Navigate to editor on success

**Tests:**
- [ ] Valid project opens successfully
- [ ] Missing game.toml shows error
- [ ] Invalid game.toml shows parse error
- [ ] Recent projects updates on open
- [ ] Error messages are user-friendly

---

#### **3.3: Load World State & Display Entities (Day 3)**

**Load World Command:**
```rust
#[tauri::command]
async fn load_world(
    state: State<EditorState>,
    project_path: String,
) -> Result<Vec<EntityInfo>, String> {
    let mut world = state.world.lock();

    // 1. Check for saved world state
    let world_state_path = PathBuf::from(&project_path)
        .join("saves/editor_state.ron");

    if world_state_path.exists() {
        // Load from saved state
        let content = std::fs::read_to_string(&world_state_path)
            .map_err(|e| format!("Failed to read world state: {}", e))?;

        let world_state: WorldState = ron::from_str(&content)
            .map_err(|e| format!("Failed to parse world state: {}", e))?;

        *world = world_state.to_world();

        tracing::info!(
            "Loaded world state from {}",
            world_state_path.display()
        );
    } else {
        // Create empty world
        *world = World::new();

        tracing::info!("Created new empty world");
    }

    // 2. Extract entity info for UI
    let entities: Vec<EntityInfo> = world.entities()
        .map(|entity| {
            let name = world.get::<Name>(entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| format!("Entity {}", entity.id()));

            let components: Vec<String> = world.components(entity)
                .map(|c| c.name())
                .collect();

            EntityInfo {
                id: entity.id().into(),
                name,
                components,
            }
        })
        .collect();

    Ok(entities)
}
```

**Svelte Store for Project State:**
```typescript
// src/lib/stores/project.ts
import { writable } from 'svelte/store';

export interface ProjectInfo {
  name: string;
  version: string;
  description: string;
  path: string;
}

export interface EntityInfo {
  id: number;
  name: string;
  components: string[];
}

export const currentProject = writable<ProjectInfo | null>(null);
export const worldEntities = writable<EntityInfo[]>([]);

export async function loadProject(path: string) {
  const project = await invoke('open_project_folder', { path });
  currentProject.set(project);

  const entities = await invoke('load_world', {
    projectPath: project.path
  });
  worldEntities.set(entities);
}
```

**Implementation tasks:**
- [ ] Implement load_world command
- [ ] Check for saves/editor_state.ron
- [ ] Load WorldState from RON format
- [ ] Create empty world if no saved state
- [ ] Extract entity info for UI
- [ ] Create Svelte store for project state
- [ ] Update Hierarchy panel on load

**Tests:**
- [ ] Empty world loads correctly
- [ ] Saved world state loads correctly
- [ ] Entity info extracted correctly
- [ ] Svelte store updates on load

---

#### **3.4: Optional Workspace Files (Future - Phase 4.9)**

**Workspace File Format:**
```json
// .silmaril-workspace.json
{
  "version": "1.0",
  "name": "My Game Dev Workspace",
  "projects": [
    {
      "name": "Main Game",
      "path": "./my-game",
      "primary": true
    },
    {
      "name": "Combat Module",
      "path": "../combat-module",
      "type": "module"
    }
  ],
  "settings": {
    "viewport": {
      "split_view": true,
      "layout": "side-by-side"
    },
    "theme": "dark",
    "font_size": 14
  }
}
```

**Implementation (Phase 4.9):**
- [ ] Define workspace file schema
- [ ] Implement workspace loader
- [ ] Support multi-project tabs
- [ ] File association for .silmaril-workspace
- [ ] CLI: `silm workspace create`
- [ ] Double-click to open workspace

---

#### **Summary - EDITOR.3 Deliverables:**

**Day 1:**
- [ ] Welcome screen with recent projects list
- [ ] EditorConfig with TOML persistence (~/.silmaril/editor-config.toml)
- [ ] get_recent_projects Tauri command

**Day 2:**
- [ ] "Open Folder" with Tauri dialog picker
- [ ] Project validation (game.toml check)
- [ ] Parse game.toml and Cargo.toml
- [ ] Update recent projects on open
- [ ] Error handling with user-friendly messages

**Day 3:**
- [ ] Load world state (saves/editor_state.ron)
- [ ] Extract entity info for Hierarchy panel
- [ ] Svelte stores for project and world state
- [ ] Navigate to editor view on successful load

**Day 4:**
- [ ] Polish UI/UX
- [ ] Test all error cases
- [ ] Document project discovery flow
- [ ] Integration testing with CLI-generated projects

**Overall tests:**
- [ ] Valid project folder opens successfully
- [ ] Invalid folder shows error
- [ ] Recent projects list updates correctly
- [ ] World state loads from saves/
- [ ] Empty world created if no saved state
- [ ] All file validation works correctly
- [ ] Error handling (invalid project)

**Tests:**
- [ ] Open valid project
- [ ] Handle invalid project
- [ ] Load empty world
- [ ] Load world with entities

**Deliverables:**
- [ ] Project loading working
- [ ] Error handling robust
- [ ] UI updates correctly

---

### **EDITOR.4: Hierarchy Panel (4 days)**

**Goal:** Display entity tree, select entities

**UI Design (shadcn-svelte):**
```svelte
<!-- src/lib/components/Hierarchy.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { Button } from '$lib/components/ui/button';
  import { Tree } from '$lib/components/ui/tree';
  import { selectedEntity } from '$lib/stores';

  let entities = [];

  async function loadEntities() {
    entities = await invoke('get_entities');
  }

  async function spawnEntity() {
    const name = prompt('Entity name:');
    await invoke('spawn_entity', { name });
    await loadEntities();
  }

  function selectEntity(entity) {
    $selectedEntity = entity;
  }

  onMount(loadEntities);
</script>

<div class="hierarchy-panel">
  <div class="toolbar">
    <Button variant="outline" size="sm" on:click={spawnEntity}>
      ➕ Spawn
    </Button>
  </div>

  <Tree items={entities} let:item>
    <div
      class="entity-item"
      class:selected={$selectedEntity?.id === item.id}
      on:click={() => selectEntity(item)}
    >
      📦 {item.name}
    </div>
  </Tree>
</div>

<style>
  .hierarchy-panel {
    padding: 1rem;
    background: #1e1e1e;
    height: 100%;
  }

  .entity-item {
    padding: 0.5rem;
    cursor: pointer;
  }

  .entity-item.selected {
    background: #2d5a9e;
  }
</style>
```

**Tauri commands:**
```rust
#[tauri::command]
fn get_entities(state: State<EditorState>) -> Result<Vec<EntityInfo>, String> {
    let world = state.world.lock();

    let entities: Vec<EntityInfo> = world.entities()
        .map(|entity| EntityInfo {
            id: entity.id(),
            name: world.get::<Name>(entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| format!("Entity {}", entity.id())),
        })
        .collect();

    Ok(entities)
}

#[tauri::command]
fn spawn_entity(name: String, state: State<EditorState>) -> Result<u64, String> {
    let mut world = state.world.lock();
    let entity = world.spawn();
    world.add(entity, Name(name));
    world.add(entity, Transform::default());

    Ok(entity.id())
}

#[tauri::command]
fn delete_entity(entity_id: u64, state: State<EditorState>) -> Result<(), String> {
    let mut world = state.world.lock();
    let entity = Entity::from_id(entity_id);
    world.despawn(entity);

    Ok(())
}
```

**Implementation tasks:**
- [ ] Entity list display
- [ ] Entity selection (click)
- [ ] Spawn entity button
- [ ] Delete entity button
- [ ] Entity rename (double-click)
- [ ] Real-time updates (reactive)

**Tests:**
- [ ] Display entities
- [ ] Select entity
- [ ] Spawn entity
- [ ] Delete entity
- [ ] Rename entity

**Deliverables:**
- [ ] Hierarchy panel working
- [ ] shadcn-svelte components used
- [ ] Real-time updates

---

### **EDITOR.5: Inspector Panel (5 days)**

**Goal:** Display and edit component data

**UI Design:**
```svelte
<!-- src/lib/components/Inspector.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Button } from '$lib/components/ui/button';
  import { Card } from '$lib/components/ui/card';
  import { selectedEntity } from '$lib/stores';

  let components = [];

  $: if ($selectedEntity) {
    loadComponents($selectedEntity.id);
  }

  async function loadComponents(entityId) {
    components = await invoke('get_components', { entityId });
  }

  async function updateComponent(componentType, field, value) {
    await invoke('update_component', {
      entityId: $selectedEntity.id,
      componentType,
      field,
      value
    });
  }
</script>

<div class="inspector-panel">
  {#if $selectedEntity}
    <h2>{$selectedEntity.name}</h2>

    {#each components as component}
      <Card>
        <h3>{component.type}</h3>

        {#each Object.entries(component.fields) as [field, value]}
          <div class="field">
            <Label>{field}</Label>
            <Input
              type="number"
              value={value}
              on:change={(e) => updateComponent(component.type, field, e.target.value)}
            />
          </div>
        {/each}
      </Card>
    {/each}

    <Button on:click={addComponent}>+ Add Component</Button>
  {:else}
    <p>No entity selected</p>
  {/if}
</div>

<style>
  .inspector-panel {
    padding: 1rem;
    background: #1e1e1e;
    height: 100%;
  }

  .field {
    margin-bottom: 0.5rem;
  }
</style>
```

**Tauri commands:**
```rust
#[tauri::command]
fn get_components(entity_id: u64, state: State<EditorState>) -> Result<Vec<ComponentInfo>, String> {
    let world = state.world.lock();
    let entity = Entity::from_id(entity_id);

    let mut components = Vec::new();

    // Transform
    if let Some(transform) = world.get::<Transform>(entity) {
        components.push(ComponentInfo {
            component_type: "Transform".into(),
            fields: serde_json::json!({
                "position": transform.position,
                "rotation": transform.rotation,
                "scale": transform.scale,
            }),
        });
    }

    // Health
    if let Some(health) = world.get::<Health>(entity) {
        components.push(ComponentInfo {
            component_type: "Health".into(),
            fields: serde_json::json!({
                "current": health.current,
                "max": health.max,
            }),
        });
    }

    // ... other components

    Ok(components)
}

#[tauri::command]
fn update_component(
    entity_id: u64,
    component_type: String,
    field: String,
    value: serde_json::Value,
    state: State<EditorState>
) -> Result<(), String> {
    let mut world = state.world.lock();
    let entity = Entity::from_id(entity_id);

    match component_type.as_str() {
        "Transform" => {
            let mut transform = world.get_mut::<Transform>(entity).unwrap();
            match field.as_str() {
                "position" => transform.position = serde_json::from_value(value).unwrap(),
                "rotation" => transform.rotation = serde_json::from_value(value).unwrap(),
                "scale" => transform.scale = serde_json::from_value(value).unwrap(),
                _ => {}
            }
        }
        "Health" => {
            let mut health = world.get_mut::<Health>(entity).unwrap();
            match field.as_str() {
                "current" => health.current = serde_json::from_value(value).unwrap(),
                "max" => health.max = serde_json::from_value(value).unwrap(),
                _ => {}
            }
        }
        _ => {}
    }

    Ok(())
}
```

**Implementation tasks:**
- [ ] Component list display
- [ ] Field editors (number, vector, string)
- [ ] Real-time updates (edit → world updates)
- [ ] Add component dialog
- [ ] Remove component button
- [ ] Type-specific editors (Vec3, Quat, Color)

**Tests:**
- [ ] Display components
- [ ] Edit component field
- [ ] Add component
- [ ] Remove component
- [ ] Type validation

**Deliverables:**
- [ ] Inspector panel working
- [ ] Component editing working
- [ ] Real-time updates

---

### **EDITOR.6: Assets Panel + Console + AI Chat (4 days)**

**Assets Panel:**
```svelte
<!-- src/lib/components/Assets.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { FileTree } from '$lib/components/ui/file-tree';

  let assets = [];

  async function loadAssets() {
    assets = await invoke('get_assets');
  }

  onMount(loadAssets);
</script>

<div class="assets-panel">
  <h2>Assets</h2>
  <FileTree items={assets} />
</div>
```

**Console Panel:**
```svelte
<!-- src/lib/components/Console.svelte -->
<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';

  let logs = [];

  onMount(() => {
    listen('log', (event) => {
      logs = [...logs, event.payload];
    });
  });
</script>

<div class="console-panel">
  <h2>Console</h2>
  {#each logs as log}
    <div class="log-entry {log.level}">
      [{log.level}] {log.message}
    </div>
  {/each}
</div>
```

**AI Chat Panel (Basic):**
```svelte
<!-- src/lib/components/AIChat.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { Input } from '$lib/components/ui/input';
  import { Button } from '$lib/components/ui/button';

  let messages = [];
  let input = '';

  async function sendMessage() {
    if (!input) return;

    messages = [...messages, { role: 'user', content: input }];
    const response = await invoke('ai_chat_basic', { message: input });
    messages = [...messages, { role: 'ai', content: response }];
    input = '';
  }
</script>

<div class="ai-chat-panel">
  <h2>🤖 AI Assistant</h2>

  <div class="messages">
    {#each messages as msg}
      <div class="message {msg.role}">
        {msg.content}
      </div>
    {/each}
  </div>

  <div class="input-area">
    <Input
      bind:value={input}
      placeholder="Ask AI..."
      on:keypress={(e) => e.key === 'Enter' && sendMessage()}
    />
    <Button on:click={sendMessage}>Send</Button>
  </div>
</div>
```

**Note:** Full AI features (code generation, debugging) in Phase 4. This is just basic chat UI.

**Implementation tasks:**
- [ ] Assets panel (file tree)
- [ ] Console panel (log streaming)
- [ ] AI chat panel (basic UI only)
- [ ] Event system (logs → console)

**Deliverables:**
- [ ] Assets panel working
- [ ] Console panel working
- [ ] AI chat UI (no functionality yet)

---

### **EDITOR.7: Playback Controls (3 days)**

**Goal:** Play/pause/stop game in editor

**UI:**
```svelte
<!-- src/lib/components/Toolbar.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { Button } from '$lib/components/ui/button';

  let playing = false;

  async function play() {
    await invoke('play_game');
    playing = true;
  }

  async function pause() {
    await invoke('pause_game');
    playing = false;
  }

  async function stop() {
    await invoke('stop_game');
    playing = false;
  }
</script>

<div class="toolbar">
  {#if !playing}
    <Button on:click={play}>▶️ Play</Button>
  {:else}
    <Button on:click={pause}>⏸️ Pause</Button>
  {/if}
  <Button on:click={stop}>⏹️ Stop</Button>
</div>
```

**Tauri commands:**
```rust
#[tauri::command]
fn play_game(state: State<EditorState>) -> Result<(), String> {
    let mut editor = state.editor.lock();
    editor.play();
    Ok(())
}

#[tauri::command]
fn pause_game(state: State<EditorState>) -> Result<(), String> {
    let mut editor = state.editor.lock();
    editor.pause();
    Ok(())
}

#[tauri::command]
fn stop_game(state: State<EditorState>) -> Result<(), String> {
    let mut editor = state.editor.lock();
    editor.stop();
    Ok(())
}
```

**Implementation tasks:**
- [ ] Playback state management
- [ ] Play button (start game loop)
- [ ] Pause button (freeze time)
- [ ] Stop button (reset to initial state)
- [ ] Visual feedback (button states)

**Tests:**
- [ ] Play starts game loop
- [ ] Pause freezes game
- [ ] Stop resets state
- [ ] Button states correct

**Deliverables:**
- [ ] Playback controls working
- [ ] Visual feedback

---

### **EDITOR.8: Integration & Polish (5 days)**

**Tasks:**
- [ ] Layout system (resizable panels)
- [ ] Dark theme (consistent with shadcn-svelte)
- [ ] Keyboard shortcuts (Ctrl+S save, Ctrl+P play, etc.)
- [ ] Menu bar (File, Edit, View, Help)
- [ ] Settings dialog
- [ ] About dialog
- [ ] Error dialogs (user-friendly)
- [ ] Loading indicators
- [ ] Tooltips
- [ ] Status bar (FPS, entity count, etc.)

**shadcn-svelte components used:**
- Button, Input, Label
- Card, Sheet, Dialog
- Select, Checkbox, Switch
- Tabs, Separator
- ScrollArea, ResizablePanelGroup
- Menubar, DropdownMenu
- Toast (notifications)

**Deliverables:**
- [ ] Polished UI
- [ ] Consistent theme
- [ ] Keyboard shortcuts
- [ ] Menu bar
- [ ] Settings working

---

## Success Criteria

- [ ] Editor opens project
- [ ] Hierarchy displays entities
- [ ] Inspector edits components
- [ ] Viewport renders game (60 FPS)
- [ ] Playback controls work
- [ ] Assets panel shows files
- [ ] Console shows logs
- [ ] AI chat UI present (basic)
- [ ] shadcn-svelte components used throughout
- [ ] Dark theme looks good
- [ ] No crashes or major bugs

---

## Performance Targets

- Editor startup: < 3s
- Viewport FPS: 60+ (with game running)
- UI responsiveness: < 16ms per frame
- Memory usage: < 500 MB (editor overhead)

---

## Dependencies

### Required Engine Features
- ✅ Phase 0.7 complete (CLI tool working)
- ✅ Phase 1.5 complete (Vulkan context)
- ✅ Phase 1.1 complete (ECS core)
- ⚠️ Phase 1.6 partial (rendering pipeline)

### External Crates (Rust)
- `tauri` (v2)
- `raw-window-handle`
- `ash-window`

### External Packages (JS)
- `@tauri-apps/api`
- `svelte` (v5)
- `shadcn-svelte`
- `vite`
- `typescript`

---

## Testing Strategy

### Unit Tests
- [ ] Tauri commands work
- [ ] State management correct
- [ ] Svelte stores reactive

### Integration Tests
- [ ] Open project → displays entities
- [ ] Edit component → world updates
- [ ] Play game → viewport animates
- [ ] Add entity → appears in hierarchy

### Manual Tests
- [ ] UI looks good
- [ ] No visual glitches
- [ ] Responsive on all platforms

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Tauri + Vulkan integration complex | Start simple, iterate |
| Platform-specific window handle issues | Use raw-window-handle crate (battle-tested) |
| Svelte 5 + shadcn-svelte compatibility | Boilerplate exists (tauri2-svelte5-shadcn) |
| Performance overhead | Profile early, optimize hot paths |

---

## Deliverables

- [ ] `engine/editor/` crate implementation
- [ ] Editor binary (`silmaril-editor` or `silm editor`)
- [ ] Hierarchy, Inspector, Assets, Console, AI Chat panels
- [ ] Native Vulkan viewport (60 FPS)
- [ ] Playback controls (play/pause/stop)
- [ ] shadcn-svelte components used throughout
- [ ] Dark theme polished
- [ ] Documentation (editor-guide.md)
- [ ] CI includes editor build

---

**Time Estimate:** 3-4 weeks (20-25 working days)

**Priority:** 🟡 **MEDIUM** - After CLI is working, this provides visual workflow. Not blocking for game development (CLI is sufficient).

**Next Steps After Completion:**
- Phase 1.6: Continue rendering pipeline (editor can preview)
- Phase 4.9: Editor Advanced Features (drag-drop, full AI integration)

---

## References

- [Tauri 2 + Svelte 5 + shadcn Boilerplate](https://github.com/alysonhower/tauri2-svelte5-shadcn)
- [shadcn-svelte Documentation](https://www.shadcn-svelte.com/)
- [Tauri + Vulkan Discussion](https://github.com/huntabyte/shadcn-svelte/discussions/1636)
- [Tauri API Reference](https://v2.tauri.app/)
