# Mesh Rendering — Design Spec

**Date:** 2026-03-22
**Phase:** 1.8
**Status:** Approved for implementation

---

## Overview

Wire mesh rendering into the editor viewport so that entities with a `MeshRenderer`
component show their geometry in the Vulkan viewport. Supports built-in primitives
(Cube, Sphere, Plane, Cylinder, Capsule) and external `.glb`/`.obj` files loaded
from the project's `assets/models/` directory.

Shading: basic diffuse (single directional light, world-space normals).
Architecture: follows Bevy's pattern — per-mesh data in a GPU storage buffer,
normal matrix pre-computed on CPU.

---

## Goals

1. Entities with `MeshRenderer` template components render visible geometry in the viewport.
2. Five built-in primitives available immediately, no files required.
3. External `.glb`/`.obj` files in `assets/models/` are auto-discovered and assignable.
4. Mesh assigned via drag-and-drop (assets panel → hierarchy) or inspector picker.
5. Correct diffuse shading under rotation and uniform scale.

## Non-Goals

- PBR materials, shadows, environment maps (Phase 4).
- Non-uniform scale normal correction (deferred — storage buffer normal matrix handles this in Phase 4 full rollout).
- Asset `.meta` sidecar files / rename-safe GUIDs (future).
- FBX, DAE, or other formats beyond `.glb`/`.obj`.

---

## Architecture

### Data Flow

```
AUTHORING
─────────
User assigns mesh (drag-drop or inspector picker)
  → assign_mesh Tauri command
      writes TemplateComponent { type_name: "MeshRenderer",
                                  data: '{"mesh_path":"assets/models/robot.glb"}' }
  → sync_mesh_renderer_to_ecs
      resolves mesh_path → mesh_id (u64 seed)
      loads file into AssetManager if not cached
      writes MeshRenderer { mesh_id } to ECS world

RENDER (every frame)
─────────────────────
render_frame reads ECS world + Arc<AssetManager>
  → builds Vec<MeshUniform> from all (Transform, MeshRenderer) entities
      MeshUniform { world_from_local, local_from_world_transpose }  ← CPU pre-computed
  → uploads Vec<MeshUniform> to storage buffer
  → binds descriptor set (binding 0 = storage buffer)
  → pushes VP matrix via push constants (64 bytes)
  → for each unique mesh: draw instanced — gl_InstanceIndex selects MeshUniform row
      GPU uploads mesh to GpuCache on first encounter
```

### Mesh Identity

Mesh references in templates are **human-readable path strings**, following the
Unreal/Godot convention. The stable backing identifier is derived from the path,
not the file content (following Bevy's `asset_server.load()` pattern — same path
always maps to the same handle).

| Template `mesh_path`         | Seed derivation                                   |
|------------------------------|---------------------------------------------------|
| `builtin://cube`             | Compile-time constant `1`                         |
| `builtin://sphere`           | Constant `2`                                      |
| `builtin://plane`            | Constant `3`                                      |
| `builtin://cylinder`         | Constant `4`                                      |
| `builtin://capsule`          | Constant `5`                                      |
| `assets/models/foo.glb`      | `blake3(b"assets/models/foo.glb")` → first 8 bytes as u64 |

**Built-in primitives** are inserted directly into the `AssetManager` under
`AssetId::from_seed_and_params(seed, b"mesh")`, so the seed IS the stable ID.

**File-based meshes** use a different identity chain:
1. `load_sync::<MeshData>` loads the file, stores it under a **content-hash** `AssetId`
   (derived from vertex + index data by `MeshData::generate_id`).
2. `sync_mesh_renderer_to_ecs` reads `handle.id()` from the returned handle and
   stores the raw `AssetId` bytes (first 8) as `mesh_id` in `MeshRenderer`.
3. `render_meshes` looks up by `AssetId::from_seed_and_params(mesh_renderer.mesh_id, b"mesh")`
   for built-ins, and by the raw `AssetId` for file assets.

> **Note:** `render_meshes` must be updated to support both lookup paths. Built-in
> lookup uses `from_seed_and_params`; file-asset lookup uses the stored `AssetId`
> directly. A wrapper enum or a two-step lookup resolves this.

**Why content-hashing for files:** updating `robot.glb` on disk changes the hash,
but `load_sync` will re-register it under the new ID and `sync_mesh_renderer_to_ecs`
will update the ECS `mesh_id` on next template load — the same re-import flow used
by Unreal and Godot when an asset is updated. The template stores the path (stable);
the ECS stores the current content ID (authoritative for the current session).

---

## Backend Changes

### 1. `MeshUniform` + Storage Buffer (renderer)

Following Bevy's `MeshUniform` pattern:

```rust
// engine/renderer/src/mesh_uniform.rs
#[repr(C)]
pub struct MeshUniform {
    /// Model matrix (object → world space)
    pub world_from_local: glam::Mat4,
    /// Pre-computed inverse-transpose of model matrix (for correct normal transform)
    /// Equivalent to Bevy's `local_from_world_transpose`
    pub local_from_world_transpose: glam::Mat4,
}

impl MeshUniform {
    pub fn from_transform(transform: &engine_core::Transform) -> Self {
        let model = transform.matrix();
        let normal = model.inverse().transpose();
        Self {
            world_from_local: model,
            local_from_world_transpose: normal,
        }
    }
}
```

A per-frame dynamic storage buffer holds `Vec<MeshUniform>` (one entry per
renderable entity). The descriptor set layout exposes it at `binding = 0`.
VP matrix is passed via push constants (64 bytes).

**This replaces the current per-draw MVP push constant.** Today `render_meshes`
pushes a full MVP (model × view × projection) per draw call. The new design pushes
VP once and reads the model matrix from the storage buffer per instance via
`gl_InstanceIndex`. The inner draw loop in `render_meshes` is fully rewritten.

`render_meshes` updated flow:
1. Iterate ECS, build `Vec<MeshUniform>` and a matching `Vec<(AssetId, u32)>` (mesh id + instance index).
2. Upload `Vec<MeshUniform>` to the per-frame storage buffer (resize if capacity exceeded).
3. `cmd_bind_descriptor_sets` — binds the storage buffer descriptor set.
4. `cmd_push_constants` — pushes VP matrix (64 bytes).
5. Per entity: `cmd_bind_vertex_buffers` / `cmd_bind_index_buffer` / `cmd_draw_indexed` with `firstInstance = instance_index`.

**Descriptor infrastructure required (new):**
- `vk::DescriptorSetLayout` with one binding: `STORAGE_BUFFER` at `binding = 0`, `VERTEX` stage.
- `vk::DescriptorPool` sized for `MAX_FRAMES_IN_FLIGHT` sets.
- One `vk::DescriptorSet` per frame-in-flight, updated each frame with the current buffer.
- Storage buffer: host-visible + host-coherent (`MemoryLocation::CpuToGpu`), capacity starts at 256 entities and doubles on overflow.
- Pipeline layout rebuilt to include the descriptor set layout alongside the existing push constant range.

### 2. Shaders

**`mesh.vert`** — reads from storage buffer by `gl_InstanceIndex`:
```glsl
layout(push_constant) uniform PushConstants {
    mat4 vp;  // view-projection (64 bytes)
} pc;

layout(set = 0, binding = 0) readonly buffer MeshUniforms {
    mat4 world_from_local[];          // index * 2
    mat4 local_from_world_transpose[]; // index * 2 + 1
} mesh_data;
// (or as an array of MeshUniform structs — implementation detail)

void main() {
    mat4 model        = mesh_data.world_from_local[gl_InstanceIndex];
    mat4 normal_mat   = mesh_data.local_from_world_transpose[gl_InstanceIndex];

    gl_Position = pc.vp * model * vec4(inPosition, 1.0);
    fragNormal  = mat3(normal_mat) * inNormal;          // world-space normal
    fragPosition = vec3(model * vec4(inPosition, 1.0));
}
```

**`mesh.frag`** — unchanged. Already implements ambient + diffuse with a
hardcoded directional light. With world-space normals now correct, shading
will be accurate under rotation.

### 3. New Tauri Managed State

**`AssetManagerState`** — `engine/editor/src-tauri/state/asset_manager.rs`:
```rust
pub struct AssetManagerState(pub Arc<AssetManager>);
```
Registered in `lib.rs`. Cloned into the render thread alongside the ECS world.

### 4. Primitive Registration

**`engine/editor/src-tauri/state/primitives.rs`** — called at editor startup:
```rust
pub fn register_primitives(manager: &AssetManager) {
    let primitives = [
        (1u64, MeshData::cube()),
        (2,    MeshData::sphere(1.0, 32, 16)),
        (3,    MeshData::plane(1.0)),
        (4,    MeshData::cylinder(0.5, 1.0, 32)),
        (5,    MeshData::capsule(0.5, 1.0, 32, 8)),
    ];
    for (seed, mesh) in primitives {
        let id = AssetId::from_seed_and_params(seed, b"mesh");
        <MeshData as AssetLoader>::insert(manager, id, mesh).ok();
    }
}
```

`MeshData::sphere`, `::plane`, `::cylinder`, `::capsule` are added to
`engine-assets` (only `cube` exists today).

### 5. `NativeViewport` Wiring

`native_viewport.rs`:
- Add `Arc<AssetManager>` field; clone into render thread.
- `OrbitCamera` gains `fn view_matrix(&self) -> Mat4` and `fn proj_matrix(&self, aspect: f32, is_ortho: bool) -> Mat4` (currently only combined `view_projection` exists). The existing `view_projection` call site in the **gizmo pipeline pass** (line ~947) must be preserved — it takes the combined matrix and must remain unchanged.
- `render_frame` calls `render_meshes` with proper `ViewportDescriptor` array built from the viewport tuples, using the new separate `view_matrix` + `proj_matrix`.
- Remove the TODO comment.

### 6. `sync_mesh_renderer_to_ecs`

`template_commands.rs` — parallel to `sync_transform_to_ecs`:
```rust
pub(crate) fn sync_mesh_renderer_to_ecs(
    entity_id: u64,
    template_state: &TemplateState,
    world_state: &SceneWorldState,
    asset_manager: &AssetManager,
    project_root: &Path,
) -> Result<(), IpcError> {
    // 1. Find "MeshRenderer" component in template for entity_id
    // 2. Deserialize mesh_path from JSON
    // 3. Resolve mesh_path → mesh_id (u64 seed)
    // 4. For file paths: load into AssetManager if not cached
    // 5. Write MeshRenderer { mesh_id } to ECS world
}
```

Called from `template_execute` when a `SetComponent{MeshRenderer}` action is processed.

Path resolution helper:
```rust
fn resolve_mesh_path(path: &str, project_root: &Path, manager: &AssetManager) -> u64 {
    match path {
        "builtin://cube"     => 1,
        "builtin://sphere"   => 2,
        "builtin://plane"    => 3,
        "builtin://cylinder" => 4,
        "builtin://capsule"  => 5,
        other => {
            // Load file into AssetManager (load_sync is idempotent for cached paths).
            // Use handle.id() as the mesh_id — content-hash based, authoritative for
            // the current session. Template stores the path (stable); ECS stores the
            // current content ID.
            let full_path = project_root.join(other);
            match manager.load_sync::<MeshData>(&full_path) {
                Ok(handle) => {
                    // Extract first 8 bytes of AssetId as mesh_id seed.
                    // render_meshes must look up file assets by raw AssetId, not
                    // via from_seed_and_params (see § "Mesh Identity").
                    u64::from_le_bytes(handle.id().as_bytes()[..8].try_into().unwrap())
                }
                Err(e) => {
                    tracing::warn!(path = other, error = ?e, "Failed to load mesh asset");
                    return Err(IpcError { code: 0, message: e.to_string() });
                }
            }
        }
    }
}
```

### 7. `SceneWorldState`

Register `MeshRenderer` component alongside `Transform`:
```rust
world.register::<engine_core::MeshRenderer>();
```

### 8. `assign_mesh` Tauri Command

```rust
// assigns a mesh_path to an entity via the template CommandProcessor
async fn assign_mesh(
    entity_id: u64,
    template_path: String,
    mesh_path: String,  // "builtin://cube" or "assets/models/foo.glb"
    // ... editor_state, world_state, asset_manager, app
) -> Result<(), IpcError>
```

Writes a `SetComponent{MeshRenderer}` action to the template, then calls
`sync_mesh_renderer_to_ecs`. Undo/redo works automatically via the existing
`CommandProcessor`.

---

## Frontend Changes

### Assets Panel

- Watches `assets/models/` via existing file watcher infrastructure.
- Displays `.glb` and `.obj` files.
- "Add" button opens native OS file dialog → copies selected file(s) into
  `assets/models/` → panel refreshes.
- Empty state: "Drop .glb or .obj files here" hint.

### Drag-and-Drop Assignment

Drag a file from the assets panel onto an entity row in the hierarchy:
- Calls `assign_mesh(entity_id, templatePath, "assets/models/foo.glb")`.
- Entity gains a `MeshRenderer` component; mesh appears in viewport immediately.

### Inspector Mesh Picker

When an entity has a `MeshRenderer` component, the inspector shows:

```
Mesh Renderer
  Mesh     [ assets/models/robot.glb  ▾ ]
  Visible  [✓]
```

Dropdown lists:
1. Built-in primitives: Cube, Sphere, Plane, Cylinder, Capsule
2. Separator
3. All `.glb`/`.obj` files found in `assets/models/`

Selecting an option calls `assign_mesh`. Selecting a primitive uses the
`builtin://` scheme.

---

## Error Handling

| Scenario | Behaviour |
|---|---|
| `mesh_path` file missing at sync time | Log `warn`, skip ECS write — entity exists, renders invisible |
| Unsupported format (.fbx, .dae, etc.) | `assign_mesh` returns `IpcError`, frontend shows toast |
| GPU upload fails (OOM) | Log `error`, skip draw call — other meshes still render |
| `assets/models/` absent | Assets panel shows empty state with hint |
| Corrupt `.glb` | `load_sync` returns `Err`, logged, entity renders invisible |
| Storage buffer too small | Resize buffer (double capacity) — same pattern as Vec growth |

---

## Testing

### Unit
- `MeshUniform::from_transform` — verify `local_from_world_transpose` is the
  correct inverse-transpose of a known model matrix (rotation + uniform scale).
- Path resolution — `builtin://cube` → `1`, `assets/models/foo.glb` → deterministic u64.
- Primitive registration — all 5 primitives registered, retrievable by seed.

### Integration
- Entity with `MeshRenderer` template component → `sync_mesh_renderer_to_ecs`
  → ECS world contains correct `MeshRenderer { mesh_id }`.
- `render_meshes` with ECS world containing a cube entity — no panics, no
  Vulkan validation errors.
- Undo/redo of `assign_mesh` — mesh assignment reverts correctly.

### Frontend
- Inspector mesh picker renders built-in options + files from `assets/models/`.
- Drag-drop from assets panel writes correct template component.
- Selecting "None" (future) hides mesh without removing entity.

---

## Performance Targets

| Metric | Target |
|---|---|
| Storage buffer upload (100 entities) | < 0.1 ms |
| `MeshUniform` CPU build (100 entities) | < 0.1 ms |
| `inverse().transpose()` per entity | < 1 µs |
| First mesh load from disk (.glb, ~1MB) | < 50 ms |
| Primitive mesh lookup (cached) | < 1 µs |

---

## Files Created / Modified

### New
- `engine/renderer/src/mesh_uniform.rs` — `MeshUniform` struct + storage buffer management
- `engine/editor/src-tauri/state/asset_manager.rs` — `AssetManagerState`
- `engine/editor/src-tauri/state/primitives.rs` — primitive registration

### Modified
- `engine/renderer/src/renderer.rs` — `render_meshes` updated for storage buffer
- `engine/renderer/shaders/mesh.vert` — reads from storage buffer, world-space normals
- `engine/renderer/shaders/mesh.frag` — unchanged
- `engine/assets/src/mesh.rs` — add `sphere`, `plane`, `cylinder`, `capsule` primitives
- `engine/editor/src-tauri/viewport/native_viewport.rs` — wire `render_meshes`, split OrbitCamera view/proj
- `engine/editor/src-tauri/state/scene_world.rs` — register `MeshRenderer`
- `engine/editor/src-tauri/state/mod.rs` — export new states
- `engine/editor/src-tauri/bridge/template_commands.rs` — add `sync_mesh_renderer_to_ecs`
- `engine/editor/src-tauri/bridge/commands.rs` — add `assign_mesh` command
- `engine/editor/src-tauri/lib.rs` — register new states + commands
- `engine/editor/src/lib/api.ts` — add `assignMesh` IPC wrapper
- `engine/editor/src/lib/docking/panels/AssetsPanel.svelte` — file watching + drag source
- `engine/editor/src/lib/docking/panels/HierarchyWrapper.svelte` — drop target
- `engine/editor/src/lib/docking/panels/InspectorPanel.svelte` — mesh picker

---

## Open Questions / Future Work

- **Non-uniform scale normal correction:** The storage buffer `local_from_world_transpose`
  is computed but the shader currently uses `mat3(normal_mat) * inNormal`. For
  non-uniform scale this requires the full 4x3 Bevy pattern — deferred to Phase 4.
- **Instanced draw calls:** MVP uses one draw call per entity. True GPU instancing
  (one call per unique mesh, N instances) is a Phase 4 optimisation.
- **Sidecar `.meta` files:** Rename-safe asset GUIDs deferred. Current model:
  renaming `assets/models/foo.glb` breaks template references.
- **Texture support:** `mesh.frag` uses flat white. UV coordinates are already
  passed through — texture sampling deferred to Phase 4.
