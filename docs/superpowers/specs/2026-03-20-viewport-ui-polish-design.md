# Viewport UI Polish Design

**Date:** 2026-03-20
**Status:** Approved

## Goal

Modernise the Vulkan viewport panel UI: replace text-char toolbar buttons with Lucide icons + bits-ui tooltips, move the axis gizmo to top-right, enforce a minimum panel size, fix the orthographic/perspective camera toggle (currently a no-op label change), and remove the meaningless zoom-% display.

---

## Scope

Six self-contained changes, all confined to the editor frontend and the Rust viewport backend:

1. **Toolbar icons + tooltips** — `ViewportPanel.svelte`
2. **Orthographic camera** — `native_viewport.rs`, `commands.rs`, `lib.rs`, `api.ts`, `ViewportPanel.svelte`
3. **Remove zoom %** — `ViewportPanel.svelte`, `viewport-settings.ts`, `viewport-settings.test.ts`
4. **Gizmo repositioning + sizing** — `ViewportPanel.svelte`
5. **Minimum panel size** — `ViewportPanel.svelte`
6. **HUD polish** — `ViewportPanel.svelte`

---

## Tech Stack

- **Icons:** `@lucide/svelte` (already installed, Svelte 5 compatible)
- **Tooltips:** `bits-ui` Tooltip (already installed)
- **Styles:** plain CSS custom properties (no Tailwind in this component — matches existing pattern)
- **Rust:** Ash/Vulkan, glam `Mat4::orthographic_rh`

---

## Section 1: Toolbar Icons + Tooltips

### Icon mapping

| Tool | Lucide icon | Import name |
|------|------------|-------------|
| Select | cursor pointer | `MousePointer2` |
| Move | four-way arrow | `Move` |
| Rotate | clockwise arrow | `RotateCw` |
| Scale | maximise arrows | `Maximize2` |
| Grid toggle | 2×2 grid | `Grid2X2` |
| Snap to grid | magnet | `Magnet` |
| Perspective (active) | video camera | `Video` |
| Orthographic (active) | scan/flat lines | `ScanLine` |
| Add entity | circle plus | `CirclePlus` |
| Reset camera | counter-clockwise | `RotateCcw` |

### Tooltip pattern (bits-ui)

Each toolbar button is wrapped in a `bits-ui` `Tooltip.Root`. The `Tooltip.Trigger` wraps the `<button>`, and `Tooltip.Content` shows the tool name and keyboard shortcut (e.g. "Move  W"). A single `Tooltip.Provider` wraps the whole toolbar.

### Projection toggle

The Perspective/Ortho toggle is a **single button** that cycles between the two modes. Its icon and label change to reflect current state:
- Perspective: `Video` icon, tooltip "Perspective  P"
- Orthographic: `ScanLine` icon, tooltip "Orthographic  P"

Shortcut `P` (no modifier) toggles projection. Add a `'p'` case to `handleKeyDown` (no modifier guard, same pattern as `'f'` for focus).

### Button sizing

- Button: 28×28 px, `border-radius: 4px`
- Icon: 14×14 px (`width="14" height="14"` on the Lucide component)
- Active state: `background: rgba(97,175,239,0.15)`, `color: #61afef`, `border: 1px solid rgba(97,175,239,0.4)`
- Hover state (inactive): `background: rgba(255,255,255,0.06)`, `color: #ccc`
- Default (inactive): `color: #777`, transparent background

---

## Section 2: Orthographic Camera Fix

### Problem

The `projection` state in `ViewportPanel.svelte` was local-only — toggling it changed a label but sent nothing to Rust. The Rust `view_projection()` always returned a perspective matrix. Additionally, the local type used `'persp'` which doesn't match the `ProjectionMode = 'ortho' | 'perspective'` type in `state.ts`.

### Rust changes (`native_viewport.rs`)

**`ViewportInstance`** gains:
```rust
is_ortho: bool,  // default: false (perspective)
```

**`OrbitCamera::view_projection`** becomes:
```rust
fn view_projection(&self, aspect: f32, is_ortho: bool) -> Mat4 {
    let view = Mat4::look_at_rh(self.eye(), self.target, Vec3::Y);
    if is_ortho {
        // Half-extent matches what perspective shows at the focus point,
        // so switching modes doesn't appear to "jump".
        let half_h = self.distance * (self.fov_y * 0.5).tan();
        let half_w = half_h * aspect;
        // near = self.near (small positive), far extended to cover objects behind camera centre.
        // Using self.near preserves depth precision; far is self.far * 2.0 to handle objects
        // well behind the orbit target when viewed from a shallow angle.
        let proj = Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, self.near, self.far * 2.0);
        proj * view
    } else {
        let proj = Mat4::perspective_rh(self.fov_y, aspect, self.near, self.far);
        proj * view
    }
}
```

**`NativeViewport`** gains a public method:
```rust
pub fn set_projection(&self, id: &str, is_ortho: bool)
```
Sets `inst.is_ortho` inside the instances mutex.

**Render loop snapshot** extends the tuple from `(ViewportBounds, OrbitCamera, bool)` to `(ViewportBounds, OrbitCamera, bool, bool)` (adding `is_ortho`). Four sites must all be updated together:
1. The `ViewportInstance` → snapshot `.map()` closure: `.map(|i| (i.bounds, i.camera.clone(), i.grid_visible, i.is_ortho))` — must include `i.is_ortho` as the fourth element.
2. The `render_frame` function parameter type: `viewports: &[(ViewportBounds, OrbitCamera, bool, bool)]`
3. The `record_frame` function parameter type: same signature update
4. The `record_frame` for-loop destructure: `for (bounds, camera, grid_visible, is_ortho) in viewports`

The `view_projection` call becomes `camera.view_projection(aspect, is_ortho)`.

**Non-Windows stub** gets a no-op `set_projection`.

### Tauri command (`commands.rs`)

```rust
#[tauri::command]
pub fn viewport_set_projection(
    viewport_state: State<'_, ViewportState>,
    viewport_id: String,
    is_ortho: bool,
) -> Result<(), String>
```

Registered in `lib.rs` `generate_handler![]`.

### TypeScript (`api.ts`)

```typescript
export async function viewportSetProjection(
  viewportId: string,
  isOrtho: boolean,
): Promise<void>
```

### Frontend (`ViewportPanel.svelte`)

- Rename local state: `projection: ProjectionMode` stays but values become `'perspective'` / `'ortho'` (fix the `'persp'` bug)
- Toggle handler calls `viewportSetProjection(viewportId, projection === 'ortho')` and updates local state
- `P` key shortcut toggles projection
- On mount: call `viewportSetProjection` after `createNativeViewport` resolves (like grid visibility sync). Add `viewportSetProjection` to the `import { ... } from '$lib/api'` block.
- Settings load/save: fix `'persp'` → `'perspective'` in the guard clause

---

## Section 3: Remove Zoom %

### Problem

`cameraZoom` was incremented/decremented on JS scroll events and displayed as `Math.round(cameraZoom * 100)%`. It had no connection to the actual Rust orbit camera distance, drifting immediately on any zoom interaction.

### Changes

- Remove `cameraZoom` state variable from `ViewportPanel.svelte`
- Remove all assignment sites: `cameraZoom = Math.max(...)` in `handleWheel` and in the `'zoom'` drag case in `handleMouseMove`
- Remove `cameraZoom` from the `$effect` save and `onMount` restore
- Remove `cameraZoom = 1` from the reset handler (Rust reset already handles this)
- Remove the zoom % `<span>` from the HUD
- Remove `cameraZoom?: number` from `ViewportUISettings` interface in `viewport-settings.ts`
- Remove the `cameraZoom` round-trip test from `viewport-settings.test.ts`
- Update any remaining test cases that use `projection: 'persp'` → `'perspective'` (e.g. the round-trip and defaults tests in `viewport-settings.test.ts`)

---

## Section 4: Gizmo — Top-Right, Larger

### Changes

**Position:**
```css
.axis-gizmo {
  position: absolute;
  top: 8px;
  right: 8px;   /* unchanged */
}
```
(Moved from `top: 40px` — no longer needs to clear the toolbar since the toolbar is centered at top and the gizmo is right-aligned.)

**Size:** SVG grows from 60×60 → 80×80. Internal constants:
- `S = 24` (was 18 — scale factor for vertex projection)
- `CX = 40, CY = 40` (was 30, 30 — SVG centre)
- Font size: `10` (was `8`)
- `stroke-width: 0.8` (was `0.5`)

**Opacity:** default `0.85` (was `0.7`), hover `1.0` (unchanged).

---

## Section 5: Minimum Panel Size

Add to `.viewport-container` CSS:
```css
min-width: 240px;
min-height: 160px;
```

This prevents Vulkan swapchain creation with degenerate dimensions when the user drags the panel very small.

---

## Section 6: HUD Polish

**Remove:** zoom % span and its separator.

**Keep:**
- Active tool: icon (14×14 Lucide, same icon as toolbar) + name text
- Separator
- Projection mode: short text label (`Persp` / `Ortho`) — colored accent
- Separator
- Reset button: `RotateCcw` icon only (no text), 22×22 px

**Style improvements:**
- Background: `rgba(0,0,0,0.6)` with `backdrop-filter: blur(4px)`
- Border: `1px solid rgba(255,255,255,0.06)`
- Slightly larger padding: `4px 10px`
- Tool name color: `var(--color-text, #ccc)` (was `#61afef` — less intrusive)
- Projection label: `#98c379` (green — keeps it visually distinct from the blue tool name)

---

## File Map

| File | Change |
|------|--------|
| `engine/editor/src-tauri/viewport/native_viewport.rs` | Add `is_ortho` to `ViewportInstance`, update `view_projection`, add `set_projection` method + non-Windows stub |
| `engine/editor/src-tauri/bridge/commands.rs` | Add `viewport_set_projection` command |
| `engine/editor/src-tauri/lib.rs` | Register `viewport_set_projection` |
| `engine/editor/src/lib/api.ts` | Add `viewportSetProjection` wrapper |
| `engine/editor/src/lib/viewport-settings.ts` | Remove `cameraZoom` field |
| `engine/editor/src/lib/viewport-settings.test.ts` | Remove `cameraZoom` test, update `'persp'` → `'perspective'` if present |
| `engine/editor/src/lib/docking/panels/ViewportPanel.svelte` | All UI changes: icons, tooltips, gizmo, HUD, min-size, projection wiring, zoom removal |

---

## Out of Scope

- Pan behaviour in ortho mode (distance-based scaling already works correctly for ortho)
- Ortho-specific grid rendering (infinite grid shader works in both modes)
- Snap-to-grid implementation (toggle exists, backend not yet implemented)
- Any panel other than ViewportPanel
- **Known limitation:** `is_ortho` is not preserved across pop-out/dock-back operations (the `CameraState` snapshot struct captures yaw/pitch/distance but not projection mode). After a dock operation, projection resets to perspective.
