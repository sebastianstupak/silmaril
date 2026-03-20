# Viewport Grid & Axis Gizmo — Design Spec

**Date:** 2026-03-20
**Scope:** Silmaril editor — `engine/editor`

---

## Problem Statement

Two viewport features are incomplete or insufficient:

1. **Grid toggle is disconnected.** The `gridVisible` button exists in the toolbar and is persisted to localStorage, but the flag is never sent to Rust — toggling has no visual effect. The grid itself is a fixed 20×20 line-list with no alpha blending, no distance fade, and no LOD levels.

2. **Axis gizmo only tracks yaw.** The SVG gizmo in the top-right corner rotates by `viewAngleDeg` (horizontal orbit only). Pitch is completely ignored — looking straight down and looking at 45° look identical. The gizmo gives no depth cue and does not correctly represent 3D orientation.

---

## Part 1 — Infinite Shader-Based Grid

### Approach

Replace the current vertex-buffer line-list with a large flat quad (-500..500 in XZ at Y=0). Grid lines are computed procedurally in the fragment shader using `fwidth()` derivatives for antialiased edges — the same technique used by Blender and Unity URP.

### Mesh Change

The new geometry is two triangles forming a quad in XZ at Y=0. Key Vulkan pipeline changes:

- **Topology**: `LINE_LIST` → `TRIANGLE_LIST`
- **Vertex format**: remove `inColor` attribute; new vertex has only `vec3 position` (stride 24 → 12 bytes)
- The `GridVertex` struct and `generate_grid_vertices` function are replaced entirely
- No index buffer needed — 6 explicit position vertices (two triangles)
- The existing Y-axis vertical line is intentionally dropped (not meaningful in a flat ground-plane grid)

### Shader Design

**Vertex shader** — passes world XZ position as a `vec2` to the fragment shader alongside `gl_Position`.

**Fragment shader:**
- **Minor grid**: lines every 1 world unit, dim gray
- **Major grid**: lines every 10 world units, brighter, slightly thicker
- **Axis lines**: center X line = red (`#b34040`), center Z line = blue (`#4040b3`)
- **Distance fade**: alpha tapers smoothly to 0 beyond ~80 units from camera XZ position — the grid fades into the clear color (opaque dark navy), making it appear infinite before the quad edge
- **Alpha blending**: grid fades to the clear color, not to transparent; fully correct against an opaque clear

### Push Constants (64 → 80 bytes)

Three sites in `native_viewport.rs` must all change together:

1. **GLSL shaders**: declare the new layout
2. **`create_grid_pipeline`**: update `push_range.size` from `64` to `80`
3. **`record_frame`**: push 80 bytes (camera eye position appended after the 64-byte matrix)

```glsl
layout(push_constant) uniform PushConstants {
    mat4 viewProj;    // 64 bytes — unchanged
    vec3 cameraPos;   // 12 bytes — camera world position (OrbitCamera::eye())
    float _pad;       //  4 bytes — 16-byte alignment
} pc;
```

All Vulkan implementations guarantee ≥128 bytes. The naga validator already uses `Capabilities::all()`, covering the derivative capability required by `fwidth()`.

### Pipeline Changes

- **Alpha blending**: `blend_enable: true`, `src_factor: SRC_ALPHA`, `dst_factor: ONE_MINUS_SRC_ALPHA`
- **Depth write**: `depth_write_enable: false` (keep `depth_test_enable: true`). Writing depth for a transparent quad causes faded outer edges to incorrectly occlude scene geometry. The render pass already has a depth attachment, so this change is compatible.

### Grid Visibility Wiring

- Add `grid_visible: bool` to `ViewportInstance` (default `true`)
- Add `set_grid_visible(id: &str, visible: bool)` to `NativeViewport`
- Add `viewport_set_grid_visible` Tauri command in `commands.rs`
- Add `viewport_set_grid_visible` to `generate_handler![...]` in `lib.rs`
- Add `viewportSetGridVisible(viewportId, visible)` in `api.ts`
- In `record_frame`: the per-instance loop still sets scissor/viewport and pushes constants for every instance. Only `cmd_draw` is skipped per-instance when `!instance.grid_visible`. This keeps push constant state correct for subsequent viewports.
- In `ViewportPanel.svelte`: call `viewportSetGridVisible` on toggle click and on mount (after `createNativeViewport` resolves)
- `gridVisible` is already in `ViewportUISettings` / localStorage — persistence requires no new work

---

## Part 2 — 3D Cube Axis Gizmo

### Camera Orientation State — JS/Rust Alignment

The existing JS state `viewAngleDeg` tracks yaw in degrees with the opposite sign convention from Rust:
- JS: `viewAngleDeg += orbitDx * 0.5` (right drag → positive)
- Rust: `self.yaw -= dx * 0.005` (right drag → negative radians)

Also, Rust `OrbitCamera::default()` has `yaw: FRAC_PI_4` while JS initializes `viewAngleDeg = 0` — an initial mismatch.

**Fix: replace `viewAngleDeg` with `cameraYawRad` in Rust-matching convention.**

- **Rename** JS state from `viewAngleDeg` to `cameraYawRad` (a float in radians)
- **Initialize** to `0.0`; **change** Rust `OrbitCamera::default()` yaw from `FRAC_PI_4` to `0.0`
- **Update** in orbit handler: `cameraYawRad -= orbitDx * 0.005` (matching Rust sign)
- Add `cameraPitchRad` state, initialized to `Math.PI / 6` (matching Rust `FRAC_PI_6`)
- **Update** in orbit handler: `cameraPitchRad = Math.max(-1.5, Math.min(1.5, cameraPitchRad + orbitDy * 0.005))` (matching Rust sign and clamp)

The gizmo uses `cameraYawRad` and `cameraPitchRad` for cube projection. The HUD reset button resets both to their defaults:

```ts
cameraYawRad = 0.0;
cameraPitchRad = Math.PI / 6;
viewportCameraReset(viewportId);
```

Both values are added to `ViewportUISettings` for persistence. The `$effect` save and the `onMount` restore block must both include them.

### New Tauri Command: `viewport_camera_set_orientation`

Snap-to-face cannot use `viewportCameraOrbit` because that command applies `dx * 0.005` scaling intended for raw mouse pixels — passing a pre-computed angular delta would land the camera at the wrong angle.

**Add a new command** `viewport_camera_set_orientation(viewport_id, yaw_rad, pitch_rad)` that directly sets absolute yaw and pitch on the `OrbitCamera`:

```rust
pub fn camera_set_orientation(&self, id: &str, yaw: f32, pitch: f32) {
    if let Ok(mut instances) = self.instances.lock() {
        if let Some(inst) = instances.get_mut(id) {
            inst.camera.yaw = yaw;
            inst.camera.pitch = pitch.clamp(-1.5, 1.5);
        }
    }
}
```

Register in `commands.rs`, `lib.rs` invoke_handler, and `api.ts` as `viewportCameraSetOrientation`.

### Cube Projection

Define 8 unit-cube vertices at ±1. For each render, rotate by `cameraYawRad` (around Y) and `cameraPitchRad` (around X) using 3×3 rotation matrices in JavaScript, then project orthographically to 2D within the 60×60 SVG.

Z values after rotation are used for face depth sorting (painter's algorithm). For a convex object with 6 planar faces, sorting by face-center Z is unambiguous.

### Face Layout

Top/bottom targets use `±1.5` — the Rust clamp boundary — rather than `±π/2 ≈ ±1.5708` which would overshoot the clamp and desync JS and Rust pitch permanently.

| Face | Label | Color | Snap target (yaw rad, pitch rad) |
|------|-------|-------|----------------------------------|
| +X   | X     | `#e06c75` (red)        | yaw=−π/2, pitch=0 |
| −X   | −X    | `#7a3040` (dark red)   | yaw=π/2,  pitch=0 |
| +Y   | Y     | `#98c379` (green)      | yaw=0,    pitch=−1.5 |
| −Y   | −Y    | `#3d6130` (dark green) | yaw=0,    pitch=1.5 |
| +Z   | Z     | `#61afef` (blue)       | yaw=0,    pitch=0 |
| −Z   | −Z    | `#2a4d7a` (dark blue)  | yaw=π,    pitch=0 |

**Front-facing faces** (Z > 0 after projection): full opacity.
**Back-facing faces** (Z ≤ 0): ~25% opacity — depth cue without hiding them.

### Snap Behavior

Clicking a face:
1. Sets `cameraYawRad` and `cameraPitchRad` to target values in JS
2. Calls `viewportCameraSetOrientation(viewportId, targetYaw, targetPitch)` — absolute set, no scaling

---

## Files Changed

| File | Change |
|------|--------|
| `engine/editor/src-tauri/viewport/native_viewport.rs` | New quad geometry (TRIANGLE_LIST, position-only, stride=12), new GLSL shaders (vert + frag with fwidth grid, distance fade, axis lines), alpha-blend pipeline, depth_write_enable=false, push constants 64→80 bytes (GLSL + pipeline layout + push call), `grid_visible` on `ViewportInstance`, `set_grid_visible()` method, `camera_set_orientation()` method, `OrbitCamera::default()` yaw changed from `FRAC_PI_4` to `0.0` |
| `engine/editor/src-tauri/bridge/commands.rs` | `viewport_set_grid_visible`, `viewport_camera_set_orientation` Tauri commands |
| `engine/editor/src-tauri/lib.rs` | Both new commands added to `generate_handler![...]` |
| `engine/editor/src/lib/api.ts` | `viewportSetGridVisible()`, `viewportCameraSetOrientation()` |
| `engine/editor/src/lib/viewport-settings.ts` | Add `cameraYawRad?: number` and `cameraPitchRad?: number` to `ViewportUISettings` |
| `engine/editor/src/lib/viewport-settings.test.ts` | Update tests for new fields |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | Rename `viewAngleDeg` → `cameraYawRad`, add `cameraPitchRad`, update orbit handler sign convention, update `$effect` save and `onMount` restore to include both, reset button resets both, call `viewportSetGridVisible` on toggle and mount, replace flat SVG with projected cube gizmo, add `viewportCameraSetOrientation` for snap |

---

## Non-Goals

- No Rust-side rendering for the gizmo (stays SVG/JS)
- No grid shadow or ground-plane shading
- No perspective grid (grid stays on the XZ world plane)
- No grid spacing configuration UI (hardcoded 1/10 unit levels)
- No Y-axis vertical line in the new shader-based grid
