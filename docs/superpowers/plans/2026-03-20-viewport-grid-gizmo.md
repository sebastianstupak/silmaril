# Viewport Grid & Axis Gizmo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the disconnected fixed-geometry grid with a shader-based infinite grid (toggle wired to Rust) and replace the flat SVG axis compass with a proper 3D projected cube gizmo that tracks both yaw and pitch.

**Architecture:** New GLSL shaders compute grid lines via `fwidth()` derivatives on a large flat quad — zero geometry changes for zoom/scale. The cube gizmo is pure SVG/JS using rotation matrices from JS-tracked yaw and pitch (aligned to Rust camera state). Two new Tauri commands handle grid visibility and absolute camera orientation for snap-to-axis.

**Tech Stack:** Rust (Ash/Vulkan, naga GLSL compilation), Svelte 5 (`$state`, `$effect`), TypeScript, Vitest, Tauri 2

---

## Spec

`docs/superpowers/specs/2026-03-20-viewport-grid-gizmo-design.md`

---

## File Map

| File | Role |
|------|------|
| `engine/editor/src-tauri/viewport/native_viewport.rs` | All Vulkan changes: grid_visible state, new methods, new shaders, new mesh, push constants, pipeline |
| `engine/editor/src-tauri/bridge/commands.rs` | Two new Tauri commands: `viewport_set_grid_visible`, `viewport_camera_set_orientation` |
| `engine/editor/src-tauri/lib.rs` | Register both new commands in `generate_handler!` |
| `engine/editor/src/lib/api.ts` | Two new TypeScript wrappers for the new commands |
| `engine/editor/src/lib/viewport-settings.ts` | Add `cameraYawRad` and `cameraPitchRad` optional fields |
| `engine/editor/src/lib/viewport-settings.test.ts` | Test new fields round-trip through localStorage |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | Rename state, fix sign convention, wire grid toggle, replace SVG gizmo with 3D cube |

---

## Task 1: Rust — ViewportInstance grid state + new camera methods

**Files:**
- Modify: `engine/editor/src-tauri/viewport/native_viewport.rs`

### What changes

In the `platform` mod (Windows implementation):

1. `ViewportInstance` gets `grid_visible: bool` (default `true`)
2. `NativeViewport` gets `set_grid_visible()` and `camera_set_orientation()` methods
3. `OrbitCamera::default()` yaw changes from `FRAC_PI_4` to `0.0`
4. The non-Windows stub at the bottom gets the new no-op methods

### Steps

- [ ] **Step 1: Add `grid_visible` to `ViewportInstance` and update `new()`**

In the `platform` mod, find `struct ViewportInstance` and its `new()`:

```rust
#[derive(Clone)]
struct ViewportInstance {
    bounds: ViewportBounds,
    camera: OrbitCamera,
    visible: bool,
    grid_visible: bool,   // ← add
}

impl ViewportInstance {
    fn new(bounds: ViewportBounds) -> Self {
        Self { bounds, camera: OrbitCamera::default(), visible: true, grid_visible: true }
    }
}
```

- [ ] **Step 2: Add `set_grid_visible` and `camera_set_orientation` to `NativeViewport`**

After the existing `camera_reset` method:

```rust
pub fn set_grid_visible(&self, id: &str, visible: bool) {
    if let Ok(mut instances) = self.instances.lock() {
        if let Some(inst) = instances.get_mut(id) {
            inst.grid_visible = visible;
        }
    }
}

pub fn camera_set_orientation(&self, id: &str, yaw: f32, pitch: f32) {
    if let Ok(mut instances) = self.instances.lock() {
        if let Some(inst) = instances.get_mut(id) {
            inst.camera.yaw = yaw;
            inst.camera.pitch = pitch.clamp(-1.5, 1.5);
        }
    }
}
```

- [ ] **Step 3: Change `OrbitCamera::default()` yaw to `0.0`**

```rust
impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 10.0,
            yaw: 0.0,                             // was FRAC_PI_4
            pitch: std::f32::consts::FRAC_PI_6,
            fov_y: std::f32::consts::FRAC_PI_4,
            near: 0.1,
            far: 500.0,
        }
    }
}
```

- [ ] **Step 4: Add non-Windows stubs at the bottom of the file**

Find the `#[cfg(not(windows))] impl NativeViewport` block near the bottom of the file. It ends with `pub fn destroy(&mut self) {}` followed by a closing `}`. Add the two new stubs **before that closing `}`**:

```rust
pub fn set_grid_visible(&self, _id: &str, _visible: bool) {}
pub fn camera_set_orientation(&self, _id: &str, _yaw: f32, _pitch: f32) {}
```

Note: this block has no `camera_reset` method — do not search for it here. The anchor is the end of the block.

- [ ] **Step 5: Compile**

```bash
cd engine/editor && cargo build 2>&1 | head -50
```

Expected: no errors. Fix any before continuing.

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src-tauri/viewport/native_viewport.rs
git commit -m "feat(editor): add grid_visible + camera_set_orientation to ViewportInstance"
```

---

## Task 2: Rust — New Tauri commands + lib.rs registration

**Files:**
- Modify: `engine/editor/src-tauri/bridge/commands.rs`
- Modify: `engine/editor/src-tauri/lib.rs`

- [ ] **Step 1: Add `viewport_set_grid_visible` command to `commands.rs`**

After the `viewport_camera_reset` command:

```rust
/// Show or hide the grid for a specific viewport instance.
#[tauri::command]
pub fn viewport_set_grid_visible(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    visible: bool,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.set_grid_visible(&viewport_id, visible);
    }
    Ok(())
}
```

- [ ] **Step 2: Add `viewport_camera_set_orientation` command to `commands.rs`**

Immediately after `viewport_set_grid_visible`:

```rust
/// Set absolute camera yaw and pitch for a specific viewport instance.
/// Used for snap-to-axis from the gizmo — bypasses the pixel-delta scaling
/// of `viewport_camera_orbit`.
#[tauri::command]
pub fn viewport_camera_set_orientation(
    viewport_state: tauri::State<NativeViewportState>,
    viewport_id: String,
    yaw: f32,
    pitch: f32,
) -> Result<(), String> {
    let registry = viewport_state.0.lock().unwrap();
    if let Some(vp) = registry.get_for_id(&viewport_id) {
        vp.camera_set_orientation(&viewport_id, yaw, pitch);
    }
    Ok(())
}
```

- [ ] **Step 3: Register both commands in `lib.rs`**

In `lib.rs` at `invoke_handler`, add after `commands::viewport_camera_reset,`:

```rust
commands::viewport_set_grid_visible,
commands::viewport_camera_set_orientation,
```

- [ ] **Step 4: Compile**

```bash
cd engine/editor && cargo build 2>&1 | head -50
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src-tauri/bridge/commands.rs engine/editor/src-tauri/lib.rs
git commit -m "feat(editor): add viewport_set_grid_visible and viewport_camera_set_orientation commands"
```

---

## Task 3: TypeScript — API wrappers + viewport-settings fields

**Files:**
- Modify: `engine/editor/src/lib/api.ts`
- Modify: `engine/editor/src/lib/viewport-settings.ts`
- Modify: `engine/editor/src/lib/viewport-settings.test.ts`

- [ ] **Step 1: Add failing tests for new settings fields**

In `viewport-settings.test.ts`, add after the existing tests:

```typescript
it('round-trips cameraYawRad and cameraPitchRad', () => {
  const s: ViewportUISettings = {
    activeTool: 'select',
    gridVisible: true,
    snapToGrid: false,
    projection: 'persp',
    cameraYawRad: -0.785,
    cameraPitchRad: 0.523,
  };
  saveViewportSettings('viewport', s);
  const loaded = loadViewportSettings('viewport');
  expect(loaded?.cameraYawRad).toBeCloseTo(-0.785, 3);
  expect(loaded?.cameraPitchRad).toBeCloseTo(0.523, 3);
});

it('loads settings without cameraYawRad/cameraPitchRad gracefully', () => {
  const s: ViewportUISettings = {
    activeTool: 'select',
    gridVisible: true,
    snapToGrid: false,
    projection: 'persp',
  };
  saveViewportSettings('viewport', s);
  const loaded = loadViewportSettings('viewport');
  expect(loaded?.cameraYawRad).toBeUndefined();
  expect(loaded?.cameraPitchRad).toBeUndefined();
});
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd engine/editor && npx vitest run src/lib/viewport-settings.test.ts
```

Expected: the two new tests FAIL (TypeScript compile error — fields don't exist yet).

- [ ] **Step 3: Add fields to `ViewportUISettings` interface**

In `viewport-settings.ts`, update the interface:

```typescript
export interface ViewportUISettings {
  activeTool: string;
  gridVisible: boolean;
  snapToGrid: boolean;
  projection: string;
  cameraZoom?: number;
  cameraYawRad?: number;    // ← add
  cameraPitchRad?: number;  // ← add
}
```

- [ ] **Step 4: Run tests — all should pass**

```bash
cd engine/editor && npx vitest run src/lib/viewport-settings.test.ts
```

Expected: all 9 tests PASS.

- [ ] **Step 5: Add API wrappers in `api.ts`**

After `viewportCameraReset`:

```typescript
/** Show or hide the grid for a specific viewport instance. */
export async function viewportSetGridVisible(viewportId: string, visible: boolean): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('viewport_set_grid_visible', { viewportId, visible });
}

/** Set absolute camera yaw and pitch for a specific viewport instance.
 *  Used for snap-to-axis — does not apply the mouse-pixel scaling of orbit. */
export async function viewportCameraSetOrientation(
  viewportId: string,
  yaw: number,
  pitch: number,
): Promise<void> {
  if (!isTauri) return;
  return tauriInvoke<void>('viewport_camera_set_orientation', { viewportId, yaw, pitch });
}
```

- [ ] **Step 6: Commit**

```bash
git add engine/editor/src/lib/api.ts engine/editor/src/lib/viewport-settings.ts engine/editor/src/lib/viewport-settings.test.ts
git commit -m "feat(editor): add viewportSetGridVisible and viewportCameraSetOrientation API + settings fields"
```

---

## Task 4: Rust — Infinite grid shader + mesh + pipeline rewrite

**Files:**
- Modify: `engine/editor/src-tauri/viewport/native_viewport.rs`

This is the largest single task. Make all changes in one edit so the file stays compilable.

### Overview of changes in `native_viewport.rs` (all within the `platform` mod)

1. Replace `GRID_VERT_GLSL` / `GRID_FRAG_GLSL` shader strings
2. Replace `GridVertex` struct and `generate_grid_vertices` with a quad generator
3. Add a `GridPushConstants` struct (80 bytes)
4. Update `create_grid_pipeline`: stride 12, no color attr, `TRIANGLE_LIST`, alpha blend, `depth_write_enable: false`, push range size 80 + both shader stages
5. Update `record_frame`: skip `cmd_draw` per instance when `!grid_visible`, push 80-byte `GridPushConstants` with camera eye

### Steps

- [ ] **Step 1: Replace the shader GLSL strings**

Replace both `GRID_VERT_GLSL` and `GRID_FRAG_GLSL` constants:

```rust
const GRID_VERT_GLSL: &str = r#"#version 450
layout(push_constant) uniform PushConstants {
    mat4 viewProj;
    vec3 cameraPos;
    float _pad;
} pc;
layout(location = 0) in vec3 inPosition;
layout(location = 0) out vec2 fragWorldXZ;
void main() {
    gl_Position = pc.viewProj * vec4(inPosition, 1.0);
    fragWorldXZ = inPosition.xz;
}
"#;

const GRID_FRAG_GLSL: &str = r#"#version 450
layout(push_constant) uniform PushConstants {
    mat4 viewProj;
    vec3 cameraPos;
    float _pad;
} pc;
layout(location = 0) in vec2 fragWorldXZ;
layout(location = 0) out vec4 outColor;

float gridLines(vec2 pos, float spacing) {
    vec2 p = pos / spacing;
    vec2 g = abs(fract(p - 0.5) - 0.5) / fwidth(p);
    return 1.0 - clamp(min(g.x, g.y), 0.0, 1.0);
}

void main() {
    vec2 pos = fragWorldXZ;
    float dist = length(pos - pc.cameraPos.xz);
    float fade = 1.0 - smoothstep(60.0, 80.0, dist);
    if (fade < 0.001) discard;

    float minor = gridLines(pos, 1.0);
    float major = gridLines(pos, 10.0);

    vec2 d = fwidth(pos);
    // X axis: line at world Z=0 (fragWorldXZ.y near 0) → red
    float xAxisLine = 1.0 - clamp(abs(pos.y) / max(d.y, 0.0001), 0.0, 1.0);
    // Z axis: line at world X=0 (fragWorldXZ.x near 0) → blue
    float zAxisLine = 1.0 - clamp(abs(pos.x) / max(d.x, 0.0001), 0.0, 1.0);

    vec3 col = vec3(0.0);
    float a = 0.0;

    if (minor > 0.01)          { col = vec3(0.28);               a = minor * 0.35; }
    if (major > minor)         { col = vec3(0.45);               a = max(a, major * 0.55); }
    if (xAxisLine > 0.01)      { col = vec3(0.70, 0.25, 0.25);   a = max(a, xAxisLine * 0.85); }
    if (zAxisLine > 0.01)      { col = vec3(0.25, 0.25, 0.70);   a = max(a, zAxisLine * 0.85); }

    if (a < 0.001) discard;
    outColor = vec4(col, a * fade);
}
"#;
```

- [ ] **Step 2: Replace `GridVertex` and the quad generator**

Replace the existing `GridVertex` struct and `generate_grid_vertices` function:

```rust
#[repr(C)]
#[derive(Clone, Copy)]
struct GridVertex { pos: [f32; 3] }

/// Six vertices (two triangles) forming a quad on the XZ plane, Y=0.
fn generate_grid_quad() -> Vec<GridVertex> {
    let e = 500.0f32;
    vec![
        GridVertex { pos: [-e, 0.0, -e] },
        GridVertex { pos: [ e, 0.0, -e] },
        GridVertex { pos: [ e, 0.0,  e] },
        GridVertex { pos: [-e, 0.0, -e] },
        GridVertex { pos: [ e, 0.0,  e] },
        GridVertex { pos: [-e, 0.0,  e] },
    ]
}
```

- [ ] **Step 3: Add `GridPushConstants` struct**

Add after the `GridVertex` definition:

```rust
/// Push constant layout for the grid shaders — 80 bytes.
/// Both VERTEX (viewProj) and FRAGMENT (cameraPos for fade) stages use this.
#[repr(C)]
struct GridPushConstants {
    view_proj: [f32; 16],  // 64 bytes
    camera_pos: [f32; 3],  // 12 bytes
    _pad: f32,             //  4 bytes — aligns to 80, within 128-byte guarantee
}
```

- [ ] **Step 4: Update `ViewportRenderer::new()` — replace the grid buffer upload**

In `ViewportRenderer::new()`, find the `generate_grid_vertices` call and replace:

```rust
// was: let grid_verts = generate_grid_vertices(20, 1.0);
let grid_verts = generate_grid_quad();
let grid_vertex_count = grid_verts.len() as u32;
let buf_size = (grid_verts.len() * std::mem::size_of::<GridVertex>()) as u64;
let mut grid_vertex_buffer = GpuBuffer::new(&context, buf_size,
    vk::BufferUsageFlags::VERTEX_BUFFER, gpu_allocator::MemoryLocation::CpuToGpu)
    .map_err(|e| format!("GridVB: {e}"))?;
grid_vertex_buffer.upload(&grid_verts).map_err(|e| format!("GridVB upload: {e}"))?;
```

- [ ] **Step 5: Update `create_grid_pipeline`**

Replace the entire function body with the following. Key changes: stride 12 (pos only), no inColor attr, `TRIANGLE_LIST`, alpha blend, `depth_write_enable: false`, push range 80 bytes with `VERTEX | FRAGMENT` stage flags:

```rust
fn create_grid_pipeline(
    device: &ash::Device,
    render_pass: &RenderPass,
    vert: &ShaderModule,
    frag: &ShaderModule,
) -> Result<(vk::Pipeline, vk::PipelineLayout), String> {
    let stages = [vert.stage_create_info(), frag.stage_create_info()];
    let binding = vk::VertexInputBindingDescription::default()
        .binding(0).stride(12).input_rate(vk::VertexInputRate::VERTEX); // stride=12: vec3 pos only
    let attrs = [
        vk::VertexInputAttributeDescription::default()
            .location(0).binding(0).format(vk::Format::R32G32B32_SFLOAT).offset(0),
        // No inColor attribute — color computed in fragment shader
    ];
    let vp = vk::Viewport::default().width(1.0).height(1.0).max_depth(1.0);
    let sc = vk::Rect2D::default().extent(vk::Extent2D { width: 1, height: 1 });
    // Push constants used by both vertex (viewProj) and fragment (cameraPos) shaders
    let push_range = vk::PushConstantRange::default()
        .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
        .offset(0)
        .size(80); // GridPushConstants: 64 (mat4) + 12 (vec3) + 4 (pad) = 80
    let layout = unsafe {
        device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::default()
                .push_constant_ranges(std::slice::from_ref(&push_range)), None)
            .map_err(|e| format!("pipeline layout: {e}"))?
    };

    // Standard src-alpha / one-minus-src-alpha blending for the grid fade
    let blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA);

    let pipeline = unsafe {
        device.create_graphics_pipelines(vk::PipelineCache::null(), &[
            vk::GraphicsPipelineCreateInfo::default()
                .stages(&stages)
                .vertex_input_state(&vk::PipelineVertexInputStateCreateInfo::default()
                    .vertex_binding_descriptions(std::slice::from_ref(&binding))
                    .vertex_attribute_descriptions(&attrs))
                .input_assembly_state(&vk::PipelineInputAssemblyStateCreateInfo::default()
                    .topology(vk::PrimitiveTopology::TRIANGLE_LIST)) // quad = 2 triangles
                .viewport_state(&vk::PipelineViewportStateCreateInfo::default()
                    .viewports(std::slice::from_ref(&vp))
                    .scissors(std::slice::from_ref(&sc)))
                .rasterization_state(&vk::PipelineRasterizationStateCreateInfo::default()
                    .polygon_mode(vk::PolygonMode::FILL)
                    .cull_mode(vk::CullModeFlags::NONE)
                    .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                    .line_width(1.0))
                .multisample_state(&vk::PipelineMultisampleStateCreateInfo::default()
                    .rasterization_samples(vk::SampleCountFlags::TYPE_1))
                .depth_stencil_state(&vk::PipelineDepthStencilStateCreateInfo::default()
                    .depth_test_enable(true)
                    .depth_write_enable(false) // false: don't write depth for transparent grid
                    .depth_compare_op(vk::CompareOp::LESS))
                .color_blend_state(&vk::PipelineColorBlendStateCreateInfo::default()
                    .attachments(std::slice::from_ref(&blend_attachment)))
                .dynamic_state(&vk::PipelineDynamicStateCreateInfo::default()
                    .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]))
                .layout(layout)
                .render_pass(render_pass.handle())
                .subpass(0)
        ], None).map_err(|(_, e)| { unsafe { device.destroy_pipeline_layout(layout, None); }
            format!("pipeline: {e}") })?[0]
    };
    tracing::info!("Grid pipeline created (infinite shader-based)");
    Ok((pipeline, layout))
}
```

- [ ] **Step 6: Update `record_frame` — push 80-byte constants and skip draw per grid_visible**

In `record_frame`, the signature currently takes `viewports: &[(ViewportBounds, OrbitCamera)]`. Update the render loop section to include grid_visible and push the new constants.

First, update the type in `render_loop` where instances are snapshot:

```rust
// In render_loop, replace the existing snapshot:
let viewports: Vec<(ViewportBounds, OrbitCamera, bool)> = {
    let lock = instances.lock().unwrap();
    lock.values()
        .filter(|i| i.visible)
        .map(|i| (i.bounds, i.camera.clone(), i.grid_visible))
        .collect()
};
```

Update `render_frame` signature:

```rust
fn render_frame(&mut self, viewports: &[(ViewportBounds, OrbitCamera, bool)]) -> Result<bool, String> {
```

The internal forwarding call `self.record_frame(cmd, image_index, viewports)?` inside `render_frame` requires no textual change — only the type signature needs updating.

Update `record_frame` signature:

```rust
unsafe fn record_frame(
    &self,
    cmd: vk::CommandBuffer,
    image_index: usize,
    viewports: &[(ViewportBounds, OrbitCamera, bool)],
) -> Result<(), String> {
```

In the per-instance loop inside `record_frame`, replace the existing push_constants + draw calls:

```rust
for (bounds, camera, grid_visible) in viewports {
    let sx = bounds.x.max(0) as u32;
    let sy = bounds.y.max(0) as u32;
    let sw = bounds.width.min(extent.width.saturating_sub(sx)).max(1);
    let sh = bounds.height.min(extent.height.saturating_sub(sy)).max(1);

    device.cmd_set_viewport(cmd, 0, &[vk::Viewport {
        x: sx as f32, y: sy as f32,
        width: sw as f32, height: sh as f32,
        min_depth: 0.0, max_depth: 1.0,
    }]);
    device.cmd_set_scissor(cmd, 0, &[vk::Rect2D {
        offset: vk::Offset2D { x: sx as i32, y: sy as i32 },
        extent: vk::Extent2D { width: sw, height: sh },
    }]);

    let aspect = sw as f32 / sh as f32;
    let eye = camera.eye();
    let pc = GridPushConstants {
        view_proj: camera.view_projection(aspect).to_cols_array(),
        camera_pos: eye.to_array(),
        _pad: 0.0,
    };
    let pc_bytes = std::slice::from_raw_parts(
        &pc as *const GridPushConstants as *const u8,
        std::mem::size_of::<GridPushConstants>(),
    );
    device.cmd_push_constants(cmd, self.grid_pipeline_layout,
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT, 0, pc_bytes);

    // Skip draw if grid hidden — push constants still set for next viewport correctness
    if *grid_visible {
        device.cmd_draw(cmd, self.grid_vertex_count, 1, 0, 0);
    }
}
```

- [ ] **Step 7: Compile**

```bash
cd engine/editor && cargo build 2>&1 | head -80
```

Expected: no errors. Common issues to watch for:
- `to_cols_array()` is on `glam::Mat4` — it exists, but double-check if `to_array()` is the correct method name (it may be `.to_cols_array()` or `.as_ref()`; check the glam version in use)
- `eye.to_array()` on `glam::Vec3` — exists in glam 0.24+

- [ ] **Step 8: Visual verify**

Run the editor (`cargo tauri dev` in `engine/editor/`). The grid should now:
- Show a large infinite-looking grid fading at the edges
- Have a red X-axis line and blue Z-axis line at the origin
- Have subtle minor lines (1 unit) and brighter major lines (10 units)

If the grid appears all-white or invisible, check that alpha blending is working (most likely the clear color issue).

- [ ] **Step 9: Commit**

```bash
git add engine/editor/src-tauri/viewport/native_viewport.rs
git commit -m "feat(editor): infinite shader-based grid with fwidth AA, distance fade, alpha blend"
```

---

## Task 5: Svelte — Wire grid toggle to Rust

**Files:**
- Modify: `engine/editor/src/lib/docking/panels/ViewportPanel.svelte`

- [ ] **Step 1: Import `viewportSetGridVisible`**

At the top of the `<script>` block, add only `viewportSetGridVisible` to the existing import from `$lib/api` (do NOT add `viewportCameraSetOrientation` yet — it is added in Task 7 to avoid an unused-import lint error):

```typescript
import {
  createNativeViewport,
  destroyNativeViewport,
  viewportCameraOrbit,
  viewportCameraPan,
  viewportCameraZoom,
  viewportCameraReset,
  viewportSetGridVisible,          // ← add
} from '$lib/api';
```

- [ ] **Step 2: Wire grid toggle button to call Rust**

Find the grid toggle button `onclick` handler:

```typescript
onclick={(e: MouseEvent) => { e.stopPropagation(); gridVisible = !gridVisible; }}
```

Replace with:

```typescript
onclick={(e: MouseEvent) => {
  e.stopPropagation();
  gridVisible = !gridVisible;
  viewportSetGridVisible(viewportId, gridVisible);
}}
```

- [ ] **Step 3: Call `viewportSetGridVisible` on mount**

In `onMount`, find the `.then()` block after `createNativeViewport`:

```typescript
createNativeViewport(viewportId, bounds.x, bounds.y, bounds.width, bounds.height).then(() => {
  nativeViewportCreated = true;
  loading = false;
  console.log('[viewport] Viewport instance ready:', viewportId);
}).catch(...)
```

Update the `.then()` body:

```typescript
createNativeViewport(viewportId, bounds.x, bounds.y, bounds.width, bounds.height).then(() => {
  nativeViewportCreated = true;
  loading = false;
  // Sync grid visibility to Rust on mount — restores persisted state
  viewportSetGridVisible(viewportId, gridVisible);
  console.log('[viewport] Viewport instance ready:', viewportId);
}).catch(...)
```

Note: `gridVisible` is already at its restored value here because `loadViewportSettings` runs synchronously before `createNativeViewport` is called.

- [ ] **Step 4: Visual verify**

Run `cargo tauri dev`. Click the `#` grid button — the grid should appear and disappear. Reload the editor — the grid state should persist correctly.

- [ ] **Step 5: Commit**

```bash
git add engine/editor/src/lib/docking/panels/ViewportPanel.svelte
git commit -m "feat(editor): wire grid toggle to Vulkan renderer, sync on mount"
```

---

## Task 6: Svelte — Camera state rename + sign convention fix + persistence

**Files:**
- Modify: `engine/editor/src/lib/docking/panels/ViewportPanel.svelte`

### Context

`viewAngleDeg` currently tracks yaw in degrees with `+= orbitDx * 0.5` (right = positive). Rust does `yaw -= dx * 0.005` (right = negative radians). The signs are opposite and the units differ. We rename, fix the sign, and add pitch.

- [ ] **Step 1: Rename `viewAngleDeg` to `cameraYawRad` and add `cameraPitchRad`**

Replace the state declarations:

```typescript
// was:
let viewAngleDeg = $state(0);

// replace with:
let cameraYawRad = $state(0.0);         // radians, matches Rust OrbitCamera::yaw
let cameraPitchRad = $state(Math.PI / 6); // radians, matches Rust OrbitCamera::pitch default
```

- [ ] **Step 2: Fix orbit handler sign convention**

In `handleMouseMove`, `case 'orbit'`:

```typescript
case 'orbit': {
  const orbitDx = event.clientX - dragStartX;
  const orbitDy = event.clientY - dragStartY;
  dragStartX = event.clientX;
  dragStartY = event.clientY;
  // Match Rust sign convention: yaw -= dx * 0.005, pitch += dy * 0.005 (clamped)
  cameraYawRad -= orbitDx * 0.005;
  cameraPitchRad = Math.max(-1.5, Math.min(1.5, cameraPitchRad + orbitDy * 0.005));
  viewportCameraOrbit(viewportId, orbitDx, orbitDy);
  break;
}
```

- [ ] **Step 3: Update the `$effect` save to include both new fields**

```typescript
$effect(() => {
  if (!settingsLoaded) return;
  saveViewportSettings(viewportId, {
    activeTool,
    gridVisible,
    snapToGrid,
    projection,
    cameraZoom,
    cameraYawRad,      // ← add
    cameraPitchRad,    // ← add
  });
});
```

- [ ] **Step 4: Update `onMount` restore to read back both fields**

In the restore block (after `loadViewportSettings`):

```typescript
const saved = loadViewportSettings(viewportId);
if (saved) {
  if (saved.activeTool) activeTool = saved.activeTool as SceneTool;
  gridVisible = saved.gridVisible;
  snapToGrid = saved.snapToGrid;
  if (saved.projection === 'persp' || saved.projection === 'ortho') {
    projection = saved.projection;
  }
  if (saved.cameraZoom != null && saved.cameraZoom > 0) {
    cameraZoom = saved.cameraZoom;
  }
  if (saved.cameraYawRad != null) {      // ← add
    cameraYawRad = saved.cameraYawRad;
  }
  if (saved.cameraPitchRad != null) {    // ← add
    cameraPitchRad = saved.cameraPitchRad;
  }
  cursor = cursorForTool(activeTool);
}
```

- [ ] **Step 5: Fix the reset button**

Find the HUD reset button `onclick`:

```typescript
onclick={(e: MouseEvent) => { e.stopPropagation(); cameraZoom = 1; viewAngleDeg = 0; viewportCameraReset(viewportId); }}
```

Replace with:

```typescript
onclick={(e: MouseEvent) => {
  e.stopPropagation();
  cameraZoom = 1;
  cameraYawRad = 0.0;
  cameraPitchRad = Math.PI / 6;
  viewportCameraReset(viewportId);
}}
```

- [ ] **Step 6: Check for any remaining `viewAngleDeg` references**

The old SVG gizmo uses `viewAngleDeg` in `rotate({-viewAngleDeg}, 30, 30)`. Since the gizmo is replaced in Task 7, leave the old SVG for now — it will be removed. Just make sure there are no other references outside the SVG block.

```bash
grep -n "viewAngleDeg" engine/editor/src/lib/docking/panels/ViewportPanel.svelte
```

Expected: only references inside the axis-gizmo `<div>` block that will be replaced in Task 7.

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src/lib/docking/panels/ViewportPanel.svelte
git commit -m "feat(editor): align JS camera yaw/pitch tracking with Rust sign convention, persist both"
```

---

## Task 7: Svelte — 3D cube axis gizmo

**Files:**
- Modify: `engine/editor/src/lib/docking/panels/ViewportPanel.svelte`

Replace the flat SVG compass with a projected 3D cube.

### How the projection works

Each cube vertex V = (vx, vy, vz) is rotated by the inverse camera orientation:
1. Rotate around Y by `-cameraYawRad` (so when camera orbits right, gizmo shows right side of world)
2. Rotate around X by `-cameraPitchRad`

Project orthographically: SVG x = center + projected.x * scale, SVG y = center - projected.y * scale (flip Y for SVG).

Faces are sorted back-to-front by their center's projected Z value (painter's algorithm). Faces with Z > 0 (facing viewer) get full opacity; faces with Z ≤ 0 get 25% opacity.

- [ ] **Step 1: Replace the axis-gizmo SVG block**

Find and remove the entire `<!-- Axis gizmo ... -->` section (the `<div class="axis-gizmo">` block with its old SVG). Replace it with:

```svelte
<!-- Axis gizmo — 3D projected cube showing camera orientation -->
<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="axis-gizmo" role="group" aria-label="Camera orientation gizmo">
  {@const S = 18}
  {@const CX = 30}
  {@const CY = 30}
  {@const cy_rot = Math.cos(-cameraYawRad)}
  {@const sy_rot = Math.sin(-cameraYawRad)}
  {@const cp_rot = Math.cos(-cameraPitchRad)}
  {@const sp_rot = Math.sin(-cameraPitchRad)}
  {@const project = (vx: number, vy: number, vz: number) => {
    // Rotate around Y (-yaw)
    const rx = cy_rot * vx + sy_rot * vz;
    const ry = vy;
    const rz = -sy_rot * vx + cy_rot * vz;
    // Rotate around X (-pitch)
    const px = rx;
    const py = cp_rot * ry - sp_rot * rz;
    const pz = sp_rot * ry + cp_rot * rz;
    return { x: CX + px * S, y: CY - py * S, z: pz };
  }}
  {@const VERTS: [number,number,number][] = [
    [-1,-1,-1],[1,-1,-1],[1,1,-1],[-1,1,-1],
    [-1,-1, 1],[1,-1, 1],[1,1, 1],[-1,1, 1],
  ]}
  {@const FACES: { vi: number[]; label: string; color: string; snapYaw: number; snapPitch: number }[] = [
    { vi:[1,2,6,5], label:'X',  color:'#e06c75', snapYaw:-Math.PI/2, snapPitch:0      },
    { vi:[0,4,7,3], label:'-X', color:'#7a3040', snapYaw: Math.PI/2, snapPitch:0      },
    { vi:[3,2,6,7], label:'Y',  color:'#98c379', snapYaw:0,          snapPitch:-1.5   },
    { vi:[0,1,5,4], label:'-Y', color:'#3d6130', snapYaw:0,          snapPitch: 1.5   },
    { vi:[4,5,6,7], label:'Z',  color:'#61afef', snapYaw:0,          snapPitch:0      },
    { vi:[0,1,2,3], label:'-Z', color:'#2a4d7a', snapYaw:Math.PI,    snapPitch:0      },
  ]}
  {@const projected = VERTS.map(([x,y,z]) => project(x,y,z))}
  {@const sortedFaces = FACES.map(f => {
    const pts = f.vi.map(i => projected[i]);
    const centerZ = pts.reduce((s,p) => s + p.z, 0) / pts.length;
    const points = pts.map(p => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(' ');
    // Centroid for label
    const lx = pts.reduce((s,p) => s + p.x, 0) / pts.length;
    const ly = pts.reduce((s,p) => s + p.y, 0) / pts.length;
    return { ...f, centerZ, points, lx, ly };
  }).sort((a,b) => a.centerZ - b.centerZ)}
  <svg width="60" height="60" viewBox="0 0 60 60">
    {#each sortedFaces as face}
      {@const opacity = face.centerZ > 0 ? 1.0 : 0.25}
      {@const snapYaw = face.snapYaw}
      {@const snapPitch = face.snapPitch}
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <polygon
        points={face.points}
        fill={face.color}
        fill-opacity={opacity * 0.85}
        stroke={face.color}
        stroke-width="0.5"
        stroke-opacity={opacity}
        style="cursor: pointer;"
        onclick={(e: MouseEvent) => {
          e.stopPropagation();
          cameraYawRad = snapYaw;
          cameraPitchRad = snapPitch;
          viewportCameraSetOrientation(viewportId, snapYaw, snapPitch);
        }}
      />
      <text
        x={face.lx}
        y={face.ly + 3.5}
        text-anchor="middle"
        fill="white"
        fill-opacity={opacity}
        font-size="8"
        font-family="sans-serif"
        font-weight="600"
        style="pointer-events: none; user-select: none;"
      >{face.label}</text>
    {/each}
  </svg>
</div>
```

- [ ] **Step 2: Add `viewportCameraSetOrientation` to the import**

Add it to the existing `$lib/api` import block (the gizmo snap uses it):

```typescript
import {
  createNativeViewport,
  destroyNativeViewport,
  viewportCameraOrbit,
  viewportCameraPan,
  viewportCameraZoom,
  viewportCameraReset,
  viewportSetGridVisible,
  viewportCameraSetOrientation,    // ← add here
} from '$lib/api';
```

- [ ] **Step 3: Verify no remaining `viewAngleDeg` references**

```bash
grep -n "viewAngleDeg" engine/editor/src/lib/docking/panels/ViewportPanel.svelte
```

Expected: 0 results.

- [ ] **Step 4: Visual verify — cube orientation**

Run `cargo tauri dev`. The gizmo should:
- Show a 3D cube in the top-right corner
- Rotate correctly as you orbit (right-drag): camera orbiting right should rotate the cube to show the X/+X face
- Pitch up/down should tilt the cube showing Y/−Y faces
- Click faces to snap to that axis view
- Back-facing faces should appear at ~25% opacity

If the cube appears mirrored horizontally, negate `sy_rot` in the Y-rotation step. If pitch appears inverted, negate `sp_rot` in the X-rotation step.

- [ ] **Step 5: Visual verify — grid toggle + persistence**

1. Toggle the grid off → grid disappears
2. Close and reopen editor → grid remains off
3. Orbit to a non-zero yaw/pitch → close → reopen → gizmo shows same orientation

- [ ] **Step 6: Run full TypeScript test suite**

```bash
cd engine/editor && npx vitest run
```

Expected: all tests pass.

- [ ] **Step 7: Final compile check**

```bash
cd engine/editor && cargo build
```

Expected: no errors.

- [ ] **Step 8: Commit**

```bash
git add engine/editor/src/lib/docking/panels/ViewportPanel.svelte
git commit -m "feat(editor): 3D cube axis gizmo with pitch tracking and snap-to-axis"
```

---

## Notes for Implementers

**Grid visual troubleshooting:**
- Grid all white / no fade → alpha blending not enabled or push constants not being sent to FRAGMENT stage
- Grid flickers / z-fighting → ensure `depth_write_enable: false` is set
- Grid invisible entirely → check that `grid_visible` defaults to `true` in `ViewportInstance::new()`
- `fwidth()` compile error in naga → confirm fragment stage is used (naga only allows derivatives in fragment shaders)

**Gizmo visual troubleshooting:**
- Cube appears flat/not rotating with pitch → check `cameraPitchRad` is updating in the orbit handler
- Cube orientation mirrored → negate `sy_rot` or `sp_rot` in the projection function
- Snap puts camera in wrong position → verify `viewportCameraSetOrientation` sets absolute yaw/pitch (not delta)
- After snap, further orbit drifts from expected position → confirm JS yaw/pitch state is updated on snap

**glam API:**
- `Mat4::to_cols_array()` returns `[f32; 16]` in column-major order — correct for Vulkan
- `Vec3::to_array()` returns `[f32; 3]`
