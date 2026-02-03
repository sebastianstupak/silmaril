# Phase 4.9: Silmaril Editor Advanced Features

**Priority:** 🟢 **LOW - Polish phase, after core engine works**

**Status:** ⚪ Not Started (0%)

**Time Estimate:** 3-4 weeks

---

## Overview

Advanced editor features that enhance productivity and enable AI-powered workflows. This builds on Phase 0.8 (Editor Foundation) to add drag-drop scene editing, full AI integration (code generation, debugging), visual shader editor, and more.

**Philosophy:**
- Editor writes code files (not binary assets)
- AI assistant can generate/modify code
- Visual tools generate text configs
- Everything is git-commitable

---

## Goals

- ✅ Drag-drop entity manipulation (position, parent, etc.)
- ✅ Visual scene composition (camera, lighting)
- ✅ Full AI integration (code generation, debugging, analysis)
- ✅ Asset import pipeline (GLTF, FBX, PNG, etc.)
- ✅ Material editor (visual PBR material creation)
- ✅ Animation timeline (future - deferred)
- ✅ Profiler UI (visualize performance data)

---

## Task Breakdown

### **ADV.1: Drag-Drop Entity Manipulation (4 days)**

**Features:**
- Drag entity in viewport → updates Transform component
- Drag entity in hierarchy → change parent
- Multi-select (Ctrl+click)
- Gizmos (translate, rotate, scale)
- Snap to grid
- Undo/redo

**UI (Viewport gizmos):**
```svelte
<!-- src/lib/components/Viewport.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';

  let draggedEntity = null;
  let gizmoMode = 'translate'; // translate, rotate, scale

  async function onMouseDown(e) {
    // Raycast from mouse position
    const entity = await invoke('raycast_viewport', {
      x: e.clientX,
      y: e.clientY
    });

    if (entity) {
      draggedEntity = entity;
    }
  }

  async function onMouseMove(e) {
    if (!draggedEntity) return;

    // Update entity position based on mouse delta
    await invoke('move_entity', {
      entityId: draggedEntity.id,
      delta: { x: e.movementX, y: e.movementY }
    });
  }

  async function onMouseUp() {
    draggedEntity = null;
  }
</script>

<div
  class="viewport"
  on:mousedown={onMouseDown}
  on:mousemove={onMouseMove}
  on:mouseup={onMouseUp}
>
  <!-- Vulkan viewport -->

  {#if selectedEntity}
    <Gizmo entity={selectedEntity} mode={gizmoMode} />
  {/if}
</div>
```

**Tauri commands:**
```rust
#[tauri::command]
fn raycast_viewport(x: f32, y: f32, state: State<EditorState>) -> Result<Option<EntityId>, String> {
    let world = state.world.lock();
    let camera = state.camera.lock();

    // Convert screen coords to ray
    let ray = camera.screen_to_ray(x, y);

    // Raycast against entities
    for entity in world.entities() {
        if let Some(transform) = world.get::<Transform>(entity) {
            if ray.intersects_sphere(transform.position, 1.0) {
                return Ok(Some(entity.id()));
            }
        }
    }

    Ok(None)
}

#[tauri::command]
fn move_entity(entity_id: u64, delta: Vec2, state: State<EditorState>) -> Result<(), String> {
    let mut world = state.world.lock();
    let entity = Entity::from_id(entity_id);

    let mut transform = world.get_mut::<Transform>(entity).unwrap();
    transform.position.x += delta.x * 0.01; // Scale factor
    transform.position.z += delta.y * 0.01;

    Ok(())
}
```

**Implementation tasks:**
- [ ] Viewport raycasting
- [ ] Entity selection (click)
- [ ] Drag to move (translate)
- [ ] Gizmos (visual handles)
- [ ] Multi-select (Ctrl+click)
- [ ] Undo/redo system
- [ ] Snap to grid
- [ ] Keyboard shortcuts (G=move, R=rotate, S=scale)

**Deliverables:**
- [ ] Drag-drop working
- [ ] Gizmos visible
- [ ] Undo/redo working

---

### **ADV.2: Full AI Integration (7 days)**

**Features:**
- Code generation (components, systems, modules)
- Debugging assistance (analyze errors, suggest fixes)
- Code analysis (suggest optimizations, find bugs)
- Natural language queries ("How does health work?")
- Streaming responses (live updates)

**UI (Enhanced AI Chat):**
```svelte
<!-- src/lib/components/AIChat.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { Button } from '$lib/components/ui/button';
  import { Textarea } from '$lib/components/ui/textarea';
  import { Card } from '$lib/components/ui/card';
  import { Tabs, TabsList, TabsTrigger, TabsContent } from '$lib/components/ui/tabs';

  let messages = [];
  let input = '';
  let streaming = false;
  let mode = 'generate'; // generate, debug, analyze, query

  async function sendMessage() {
    if (!input) return;

    messages = [...messages, { role: 'user', content: input }];
    streaming = true;

    // Stream AI response
    let aiMessage = { role: 'ai', content: '', actions: [] };
    messages = [...messages, aiMessage];

    await invoke('ai_chat', {
      message: input,
      mode: mode,
      onChunk: (chunk) => {
        aiMessage.content += chunk;
        messages = messages; // Trigger reactivity
      },
      onAction: (action) => {
        aiMessage.actions.push(action);
      }
    });

    streaming = false;
    input = '';
  }

  async function applyAction(action) {
    // Apply AI-generated code change
    await invoke('apply_ai_action', { action });
  }
</script>

<div class="ai-chat">
  <Tabs value={mode} onValueChange={(v) => mode = v}>
    <TabsList>
      <TabsTrigger value="generate">Generate</TabsTrigger>
      <TabsTrigger value="debug">Debug</TabsTrigger>
      <TabsTrigger value="analyze">Analyze</TabsTrigger>
      <TabsTrigger value="query">Query</TabsTrigger>
    </TabsList>
  </Tabs>

  <div class="messages">
    {#each messages as msg}
      <Card class="message {msg.role}">
        <p>{msg.content}</p>

        {#if msg.actions}
          {#each msg.actions as action}
            <div class="action">
              <p><strong>Action:</strong> {action.description}</p>
              <Button on:click={() => applyAction(action)}>Apply</Button>
            </div>
          {/each}
        {/if}
      </Card>
    {/each}
  </div>

  <Textarea
    bind:value={input}
    placeholder="Ask AI... (e.g., 'Add a health regeneration system')"
    rows={3}
  />
  <Button on:click={sendMessage} disabled={streaming}>
    {streaming ? 'Generating...' : 'Send'}
  </Button>
</div>
```

**Tauri commands:**
```rust
#[tauri::command]
async fn ai_chat(
    message: String,
    mode: String,
    state: State<EditorState>
) -> Result<AiResponse, String> {
    let project = state.project.lock();

    // Gather context (current codebase, files, errors)
    let context = gather_ai_context(&project)?;

    match mode.as_str() {
        "generate" => {
            // Generate component/system/module
            let response = ai_generate_code(&message, &context).await?;
            Ok(response)
        }
        "debug" => {
            // Analyze error, suggest fix
            let response = ai_debug_error(&message, &context).await?;
            Ok(response)
        }
        "analyze" => {
            // Code analysis (suggest optimizations, find bugs)
            let response = ai_analyze_code(&message, &context).await?;
            Ok(response)
        }
        "query" => {
            // Answer questions about codebase
            let response = ai_query_codebase(&message, &context).await?;
            Ok(response)
        }
        _ => Err("Unknown mode".into()),
    }
}

async fn ai_generate_code(prompt: &str, context: &AiContext) -> Result<AiResponse, String> {
    let system_prompt = format!(r#"
You are a Rust game development assistant for Silmaril engine.
Generate code following these rules:
- Use custom error types (define_error! macro)
- Use tracing for logging (never println!)
- Add #[derive(Component)] to components
- Include tests
- Follow existing code patterns

Current project structure:
{}

User request: {}
"#, context.project_structure, prompt);

    // Call LLM API (Claude, GPT, etc.)
    let response = call_llm_api(&system_prompt).await?;

    // Parse response (extract code, actions)
    let code_files = parse_generated_code(&response)?;

    Ok(AiResponse {
        message: response.message,
        actions: code_files.into_iter().map(|file| AiAction {
            action_type: "create_file".into(),
            description: format!("Create {}", file.path),
            data: serde_json::json!({
                "path": file.path,
                "content": file.content,
            }),
        }).collect(),
    })
}

#[tauri::command]
fn apply_ai_action(action: AiAction, state: State<EditorState>) -> Result<(), String> {
    match action.action_type.as_str() {
        "create_file" => {
            let path = action.data["path"].as_str().unwrap();
            let content = action.data["content"].as_str().unwrap();
            std::fs::write(path, content)?;
            Ok(())
        }
        "modify_file" => {
            // Apply diff/patch
            let path = action.data["path"].as_str().unwrap();
            let patch = action.data["patch"].as_str().unwrap();
            apply_patch(path, patch)?;
            Ok(())
        }
        _ => Err("Unknown action type".into()),
    }
}
```

**Implementation tasks:**
- [ ] AI context gathering (project structure, files, errors)
- [ ] LLM API integration (Claude, GPT, local model)
- [ ] Streaming responses
- [ ] Code generation (components, systems)
- [ ] Debugging assistance
- [ ] Code analysis
- [ ] Apply actions (write files, run tests)
- [ ] Show diffs before applying

**Deliverables:**
- [ ] AI chat fully functional
- [ ] Code generation working
- [ ] Debugging working
- [ ] Actions can be applied

---

### **ADV.3: Asset Import Pipeline (5 days)**

**Features:**
- Import GLTF/FBX models
- Import PNG/JPG textures
- Import WAV/OGG audio
- Import TTF fonts
- Generate .meta files (import settings)
- Texture compression (BC7, ASTC)
- Mesh optimization (simplify, LOD generation)

**UI (Asset Importer):**
```svelte
<!-- src/lib/components/AssetImporter.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { open } from '@tauri-apps/api/dialog';
  import { Dialog, DialogContent, DialogHeader } from '$lib/components/ui/dialog';
  import { Button } from '$lib/components/ui/button';
  import { Select } from '$lib/components/ui/select';

  let showImporter = false;
  let selectedFile = null;
  let importSettings = {
    textureFormat: 'BC7',
    generateLODs: true,
    optimizeMesh: true,
  };

  async function selectFile() {
    selectedFile = await open({
      filters: [
        { name: 'Models', extensions: ['gltf', 'glb', 'fbx'] },
        { name: 'Textures', extensions: ['png', 'jpg', 'jpeg'] },
        { name: 'Audio', extensions: ['wav', 'ogg', 'mp3'] },
      ]
    });
  }

  async function importAsset() {
    await invoke('import_asset', {
      path: selectedFile,
      settings: importSettings
    });

    showImporter = false;
  }
</script>

<Button on:click={() => showImporter = true}>Import Asset</Button>

<Dialog open={showImporter}>
  <DialogContent>
    <DialogHeader>Import Asset</DialogHeader>

    {#if selectedFile}
      <p>File: {selectedFile}</p>

      <div class="settings">
        <Select label="Texture Format" bind:value={importSettings.textureFormat}>
          <option value="BC7">BC7 (High Quality)</option>
          <option value="BC1">BC1 (Fast)</option>
          <option value="ASTC">ASTC (Mobile)</option>
        </Select>

        <Checkbox bind:checked={importSettings.generateLODs}>
          Generate LODs
        </Checkbox>

        <Checkbox bind:checked={importSettings.optimizeMesh}>
          Optimize Mesh
        </Checkbox>
      </div>

      <Button on:click={importAsset}>Import</Button>
    {:else}
      <Button on:click={selectFile}>Select File</Button>
    {/if}
  </DialogContent>
</Dialog>
```

**Implementation tasks:**
- [ ] GLTF parser (gltf crate)
- [ ] FBX parser (fbxcel-dom crate)
- [ ] Image loading (image crate)
- [ ] Texture compression (basis-universal)
- [ ] Mesh optimization (meshopt)
- [ ] LOD generation (simplify)
- [ ] .meta file generation
- [ ] Import settings UI

**Deliverables:**
- [ ] Asset import working
- [ ] Supported formats: GLTF, FBX, PNG, JPG, WAV
- [ ] Import settings configurable
- [ ] .meta files generated

---

### **ADV.4: Material Editor (4 days)**

**Features:**
- Visual PBR material creation
- Node-based shader graph (future)
- Material presets (metal, plastic, wood, etc.)
- Live preview (sphere, cube)
- Texture assignment (albedo, normal, metallic, roughness)
- Save as .ron file

**UI (Material Editor):**
```svelte
<!-- src/lib/components/MaterialEditor.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { Card } from '$lib/components/ui/card';
  import { Slider } from '$lib/components/ui/slider';
  import { Button } from '$lib/components/ui/button';

  let material = {
    albedo: '#ffffff',
    metallic: 0.0,
    roughness: 0.5,
    albedoTexture: null,
    normalTexture: null,
    metallicTexture: null,
    roughnessTexture: null,
  };

  async function saveMaterial() {
    await invoke('save_material', { material });
  }
</script>

<div class="material-editor">
  <div class="preview">
    <!-- Live preview (sphere with material) -->
    <MaterialPreview {material} />
  </div>

  <Card class="properties">
    <h3>Material Properties</h3>

    <Label>Albedo</Label>
    <Input type="color" bind:value={material.albedo} />

    <Label>Metallic</Label>
    <Slider min={0} max={1} step={0.01} bind:value={material.metallic} />

    <Label>Roughness</Label>
    <Slider min={0} max={1} step={0.01} bind:value={material.roughness} />

    <Label>Albedo Texture</Label>
    <Button on:click={() => selectTexture('albedo')}>Select</Button>

    <Button on:click={saveMaterial}>Save Material</Button>
  </Card>
</div>
```

**Implementation tasks:**
- [ ] Material editor UI
- [ ] Live preview (render sphere)
- [ ] Texture assignment
- [ ] Material presets
- [ ] Save as .ron file
- [ ] Load material in engine

**Deliverables:**
- [ ] Material editor working
- [ ] Live preview
- [ ] Materials saved as .ron

---

### **ADV.5: Profiler UI (5 days)**

**Features:**
- Visualize profiling data from engine
- Timeline view (systems, frames)
- Flamegraph view
- Frame time graph
- GPU profiling
- Memory profiling
- Export to Chrome Tracing

**UI (Profiler):**
```svelte
<!-- src/lib/components/Profiler.svelte -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/tauri';
  import { onMount } from 'svelte';
  import { Timeline } from '$lib/components/ui/timeline';
  import { Flamegraph } from '$lib/components/ui/flamegraph';

  let profilingData = [];
  let viewMode = 'timeline'; // timeline, flamegraph

  async function loadProfilingData() {
    profilingData = await invoke('get_profiling_data');
  }

  onMount(() => {
    // Poll for profiling data
    setInterval(loadProfilingData, 1000);
  });
</script>

<div class="profiler">
  <Tabs value={viewMode}>
    <TabsList>
      <TabsTrigger value="timeline">Timeline</TabsTrigger>
      <TabsTrigger value="flamegraph">Flamegraph</TabsTrigger>
    </TabsList>
  </Tabs>

  {#if viewMode === 'timeline'}
    <Timeline data={profilingData} />
  {:else}
    <Flamegraph data={profilingData} />
  {/if}
</div>
```

**Implementation tasks:**
- [ ] Profiling data API (query from engine)
- [ ] Timeline visualization
- [ ] Flamegraph visualization
- [ ] Frame time graph
- [ ] GPU timeline
- [ ] Memory graph
- [ ] Export to Chrome Tracing

**Deliverables:**
- [ ] Profiler UI working
- [ ] Timeline view
- [ ] Flamegraph view
- [ ] Real-time updates

---

### **ADV.6: Integration & Polish (5 days)**

**Tasks:**
- [ ] Save/load editor layouts
- [ ] Keyboard shortcuts (comprehensive)
- [ ] Context menus (right-click)
- [ ] Drag-drop between panels
- [ ] Recent projects list
- [ ] Search (global, assets, entities)
- [ ] Command palette (Ctrl+P)
- [ ] Themes (light mode optional)
- [ ] Localization (i18n - future)

**Deliverables:**
- [ ] Polished UX
- [ ] All features integrated
- [ ] Documentation updated

---

## Success Criteria

- [ ] Drag-drop working smoothly
- [ ] AI generates working code
- [ ] Asset import functional
- [ ] Material editor creates materials
- [ ] Profiler visualizes data
- [ ] UX feels polished
- [ ] No major bugs

---

## Performance Targets

- Viewport: 60 FPS (even while dragging)
- AI response: < 5s (first token)
- Asset import: < 10s (1MB model)
- Profiler update: < 100ms

---

## Dependencies

### Required Engine Features
- ✅ Phase 0.7 complete (CLI)
- ✅ Phase 0.8 complete (Editor Foundation)
- ✅ Phase 1 complete (Rendering)
- ✅ Phase 0.5 complete (Profiling)

### External Crates
- `gltf` - GLTF parser
- `fbxcel-dom` - FBX parser
- `image` - Image loading
- `meshopt` - Mesh optimization
- `basis-universal` - Texture compression

---

## Testing Strategy

### Unit Tests
- [ ] Asset import
- [ ] Material serialization
- [ ] AI action application

### Integration Tests
- [ ] Full AI workflow (prompt → code → test)
- [ ] Asset import → render in game
- [ ] Drag entity → updates world

### Manual Tests
- [ ] UX feels good
- [ ] No UI glitches
- [ ] Performance acceptable

---

## Deliverables

- [ ] Drag-drop entity manipulation
- [ ] Full AI integration (generate, debug, analyze)
- [ ] Asset import pipeline
- [ ] Material editor
- [ ] Profiler UI
- [ ] Polished UX
- [ ] Documentation updated

---

**Time Estimate:** 3-4 weeks (20-25 working days)

**Priority:** 🟢 **LOW** - Polish phase. Nice-to-have features that enhance productivity but aren't blocking.

**Next Steps After Completion:**
- Ship Silmaril v1.0! 🚀
