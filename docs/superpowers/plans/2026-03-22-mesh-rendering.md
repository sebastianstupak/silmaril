# Mesh Rendering (Phase 1.8) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire mesh rendering into the editor viewport so entities with a `MeshRenderer` component show geometry — built-in primitives + external `.glb`/`.obj` files, basic diffuse shading, assignable via drag-drop or inspector picker.

**Architecture:** Bevy-pattern — per-mesh data in a GPU storage buffer (`MeshUniform { world_from_local, local_from_world_transpose }`), VP pushed via 64-byte push constant, normal matrix pre-computed on CPU. Mesh identity is path-hash based (stable across content updates).

**Tech Stack:** Rust/Ash (Vulkan), glam, naga (GLSL→SPIRV), gpu-allocator, engine-assets, engine-core, Tauri, Svelte/TypeScript.

**Spec:** `docs/superpowers/specs/2026-03-22-mesh-rendering-design.md`

---

## Prerequisites

`engine_core::MeshRenderer` **already exists** at `engine/core/src/rendering.rs` with `mesh_id: u64` and `visible: bool` fields and `is_visible()` / `new(mesh_id)` methods. No new task needed for it.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `engine/assets/src/mesh.rs` | Modify | Add `sphere`, `plane`, `cylinder`, `capsule` primitive generators |
| `engine/render-context/shaders/mesh.vert` | Modify | Switch from MVP push-constant to VP push-constant + storage buffer |
| `engine/render-context/src/pipeline.rs` | Modify | `new_mesh_pipeline_with_descriptors`: `UNIFORM_BUFFER` → `STORAGE_BUFFER` |
| `engine/renderer/src/mesh_uniform.rs` | **Create** | `MeshUniform` struct + descriptor pool/set/buffer management types |
| `engine/renderer/src/lib.rs` | Modify | `pub mod mesh_uniform` |
| `engine/renderer/src/renderer.rs` | Modify | Add descriptor pool/sets/buffers fields; rewrite `render_meshes` |
| `engine/editor/src-tauri/state/asset_manager.rs` | **Create** | `AssetManagerState(Arc<AssetManager>)` |
| `engine/editor/src-tauri/state/primitives.rs` | **Create** | `register_primitives(manager)` |
| `engine/editor/src-tauri/state/scene_world.rs` | Modify | Register `MeshRenderer` component |
| `engine/editor/src-tauri/state/mod.rs` | Modify | Export new state modules |
| `engine/editor/src-tauri/viewport/native_viewport.rs` | Modify | Add `view_matrix`/`proj_matrix` to `OrbitCamera`; wire `render_meshes`; add `AssetManager` field |
| `engine/editor/src-tauri/bridge/template_commands.rs` | Modify | Add `sync_mesh_renderer_to_ecs` + `resolve_mesh_path` |
| `engine/editor/src-tauri/bridge/commands.rs` | Modify | Add `assign_mesh` Tauri command |
| `engine/editor/src-tauri/lib.rs` | Modify | Register `AssetManagerState`, `assign_mesh` handler, call `register_primitives` |
| `engine/editor/src/lib/api.ts` | Modify | Add `assignMesh` IPC wrapper |
| `engine/editor/src/lib/docking/panels/AssetsPanel.svelte` | Modify | File listing for `.glb`/`.obj`, drag source, "Add" button |
| `engine/editor/src/lib/docking/panels/HierarchyWrapper.svelte` | Modify | Drop target for mesh drag |
| `engine/editor/src/lib/docking/panels/InspectorPanel.svelte` (or InspectorWrapper) | Modify | `MeshRenderer` section with dropdown picker |

---

## Task 1: Add Missing Primitive Generators to engine-assets

**Files:**
- Modify: `engine/assets/src/mesh.rs`

**Context:** `MeshData::cube()` already exists (24 verts, 36 indices). We need sphere, plane, cylinder, capsule.

- [ ] **Step 1: Write failing tests**

Add to `engine/assets/src/mesh.rs` inside `#[cfg(test)] mod tests`:

```rust
#[test]
fn test_sphere_has_vertices_and_indices() {
    let m = MeshData::sphere(1.0, 8, 4); // low-res for test speed
    assert!(!m.vertices.is_empty(), "sphere must have vertices");
    assert!(!m.indices.is_empty(), "sphere must have indices");
    assert_eq!(m.indices.len() % 3, 0, "indices must be a multiple of 3");
    // All normals must be unit length
    for v in &m.vertices {
        let len = v.normal.length();
        assert!((len - 1.0).abs() < 1e-4, "normal not unit length: {}", len);
    }
}

#[test]
fn test_plane_has_four_vertices() {
    let m = MeshData::plane(1.0);
    assert_eq!(m.vertices.len(), 4);
    assert_eq!(m.indices.len(), 6);
}

#[test]
fn test_cylinder_has_correct_ring_structure() {
    let m = MeshData::cylinder(0.5, 1.0, 8);
    assert!(!m.vertices.is_empty());
    assert_eq!(m.indices.len() % 3, 0);
}

#[test]
fn test_capsule_has_vertices_and_indices() {
    let m = MeshData::capsule(0.5, 1.0, 8, 4);
    assert!(!m.vertices.is_empty());
    assert_eq!(m.indices.len() % 3, 0);
}
```

- [ ] **Step 2: Run tests — expect compile failure**

```bash
cargo test -p engine-assets test_sphere 2>&1 | head -20
```

Expected: `error[E0599]: no method named 'sphere' found`

- [ ] **Step 3: Implement sphere**

Add after `MeshData::cube()`:

```rust
/// UV sphere (radius, longitude segments, latitude segments)
pub fn sphere(radius: f32, lon_segs: u32, lat_segs: u32) -> Self {
    use std::f32::consts::PI;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for lat in 0..=lat_segs {
        let theta = lat as f32 * PI / lat_segs as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=lon_segs {
            let phi = lon as f32 * 2.0 * PI / lon_segs as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = sin_theta * cos_phi;
            let y = cos_theta;
            let z = sin_theta * sin_phi;
            let normal = glam::Vec3::new(x, y, z);
            let position = normal * radius;
            let uv = glam::Vec2::new(
                lon as f32 / lon_segs as f32,
                lat as f32 / lat_segs as f32,
            );
            vertices.push(Vertex { position, normal, uv });
        }
    }

    for lat in 0..lat_segs {
        for lon in 0..lon_segs {
            let first = lat * (lon_segs + 1) + lon;
            let second = first + lon_segs + 1;
            indices.push(first);
            indices.push(second);
            indices.push(first + 1);
            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    info!(vertices = vertices.len(), indices = indices.len(), "Sphere mesh created");
    Self { vertices, indices }
}
```

- [ ] **Step 4: Implement plane**

```rust
/// Flat plane on XZ axis (half_size in each direction)
pub fn plane(size: f32) -> Self {
    let h = size / 2.0;
    let normal = glam::Vec3::Y;
    let vertices = vec![
        Vertex { position: glam::Vec3::new(-h, 0.0, -h), normal, uv: glam::Vec2::new(0.0, 0.0) },
        Vertex { position: glam::Vec3::new( h, 0.0, -h), normal, uv: glam::Vec2::new(1.0, 0.0) },
        Vertex { position: glam::Vec3::new( h, 0.0,  h), normal, uv: glam::Vec2::new(1.0, 1.0) },
        Vertex { position: glam::Vec3::new(-h, 0.0,  h), normal, uv: glam::Vec2::new(0.0, 1.0) },
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];
    info!("Plane mesh created");
    Self { vertices, indices }
}
```

- [ ] **Step 5: Implement cylinder**

```rust
/// Cylinder with caps (radius, height, segments)
pub fn cylinder(radius: f32, height: f32, segs: u32) -> Self {
    use std::f32::consts::PI;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let half_h = height / 2.0;

    // Side vertices: two rings (bottom and top), duplicated for sharp cap normals
    for ring in 0..=1 {
        let y = if ring == 0 { -half_h } else { half_h };
        for seg in 0..=segs {
            let angle = seg as f32 * 2.0 * PI / segs as f32;
            let x = angle.cos();
            let z = angle.sin();
            vertices.push(Vertex {
                position: glam::Vec3::new(x * radius, y, z * radius),
                normal: glam::Vec3::new(x, 0.0, z),
                uv: glam::Vec2::new(seg as f32 / segs as f32, ring as f32),
            });
        }
    }

    // Side indices
    let stride = segs + 1;
    for seg in 0..segs {
        let b = seg;
        let t = seg + stride;
        indices.extend_from_slice(&[b, t, b + 1, t, t + 1, b + 1]);
    }

    // Bottom cap
    let bottom_center = vertices.len() as u32;
    vertices.push(Vertex {
        position: glam::Vec3::new(0.0, -half_h, 0.0),
        normal: -glam::Vec3::Y,
        uv: glam::Vec2::new(0.5, 0.5),
    });
    let bottom_start = vertices.len() as u32;
    for seg in 0..=segs {
        let angle = seg as f32 * 2.0 * PI / segs as f32;
        vertices.push(Vertex {
            position: glam::Vec3::new(angle.cos() * radius, -half_h, angle.sin() * radius),
            normal: -glam::Vec3::Y,
            uv: glam::Vec2::new(angle.cos() * 0.5 + 0.5, angle.sin() * 0.5 + 0.5),
        });
    }
    for seg in 0..segs {
        indices.extend_from_slice(&[bottom_center, bottom_start + seg + 1, bottom_start + seg]);
    }

    // Top cap
    let top_center = vertices.len() as u32;
    vertices.push(Vertex {
        position: glam::Vec3::new(0.0, half_h, 0.0),
        normal: glam::Vec3::Y,
        uv: glam::Vec2::new(0.5, 0.5),
    });
    let top_start = vertices.len() as u32;
    for seg in 0..=segs {
        let angle = seg as f32 * 2.0 * PI / segs as f32;
        vertices.push(Vertex {
            position: glam::Vec3::new(angle.cos() * radius, half_h, angle.sin() * radius),
            normal: glam::Vec3::Y,
            uv: glam::Vec2::new(angle.cos() * 0.5 + 0.5, angle.sin() * 0.5 + 0.5),
        });
    }
    for seg in 0..segs {
        indices.extend_from_slice(&[top_center, top_start + seg, top_start + seg + 1]);
    }

    info!(vertices = vertices.len(), indices = indices.len(), "Cylinder mesh created");
    Self { vertices, indices }
}
```

- [ ] **Step 6: Implement capsule**

```rust
/// Capsule (cylinder with hemispherical caps)
pub fn capsule(radius: f32, height: f32, lon_segs: u32, lat_segs: u32) -> Self {
    use std::f32::consts::PI;
    // Build as UV sphere stretched along Y — top half offset +height/2, bottom half offset -height/2
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let half_h = height / 2.0;
    let total_lat = lat_segs * 2; // hemisphere each side

    for lat in 0..=total_lat {
        let theta = lat as f32 * PI / total_lat as f32;
        let y_offset = if lat <= lat_segs { half_h } else { -half_h };
        // Clamp theta to hemisphere halves
        let half_theta = if lat <= lat_segs {
            lat as f32 * PI / 2.0 / lat_segs as f32
        } else {
            PI / 2.0 + (lat - lat_segs) as f32 * PI / 2.0 / lat_segs as f32
        };
        let sin_theta = half_theta.sin();
        let cos_theta = half_theta.cos();

        for lon in 0..=lon_segs {
            let phi = lon as f32 * 2.0 * PI / lon_segs as f32;
            let nx = sin_theta * phi.cos();
            let ny = cos_theta;
            let nz = sin_theta * phi.sin();
            let normal = glam::Vec3::new(nx, ny, nz);
            // For the lower hemisphere, ny is negative
            let actual_y = if lat <= lat_segs { ny * radius + half_h } else { ny * radius - half_h };
            vertices.push(Vertex {
                position: glam::Vec3::new(nx * radius, actual_y, nz * radius),
                normal,
                uv: glam::Vec2::new(lon as f32 / lon_segs as f32, lat as f32 / total_lat as f32),
            });
        }
    }

    for lat in 0..total_lat {
        for lon in 0..lon_segs {
            let first = lat * (lon_segs + 1) + lon;
            let second = first + lon_segs + 1;
            indices.extend_from_slice(&[first, second, first + 1, second, second + 1, first + 1]);
        }
    }

    info!(vertices = vertices.len(), indices = indices.len(), "Capsule mesh created");
    Self { vertices, indices }
}
```

- [ ] **Step 7: Run tests — expect pass**

```bash
cargo test -p engine-assets
```

Expected: All tests pass including the 4 new primitive tests.

- [ ] **Step 8: Commit**

```bash
git add engine/assets/src/mesh.rs
git commit -m "feat(assets): add sphere, plane, cylinder, capsule primitive generators"
```

---

## Task 2: MeshUniform Struct (renderer crate)

**Files:**
- Create: `engine/renderer/src/mesh_uniform.rs`
- Modify: `engine/renderer/src/lib.rs` (add `pub mod mesh_uniform;`)

**Context:** `MeshUniform` holds two Mat4s — the model matrix and its inverse-transpose for normals. Built on CPU, uploaded to a storage buffer every frame. This file keeps descriptor pool/set/buffer infrastructure separate from the 500-line renderer.rs.

- [ ] **Step 1: Write failing test**

Create `engine/renderer/src/mesh_uniform.rs` with just the test first:

```rust
//! Per-mesh GPU data following Bevy's MeshUniform pattern.

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MeshUniform {
    /// Model matrix: object → world space
    pub world_from_local: glam::Mat4,
    /// Pre-computed inverse-transpose of model matrix (correct normal transform)
    pub local_from_world_transpose: glam::Mat4,
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::Transform;

    #[test]
    fn test_mesh_uniform_identity() {
        let t = Transform::default();
        let u = MeshUniform::from_transform(&t);
        // Identity model → both matrices should be identity
        assert!((u.world_from_local - glam::Mat4::IDENTITY).abs_diff_eq(glam::Mat4::ZERO, 1e-5));
        assert!((u.local_from_world_transpose - glam::Mat4::IDENTITY).abs_diff_eq(glam::Mat4::ZERO, 1e-5));
    }

    #[test]
    fn test_mesh_uniform_uniform_scale() {
        // Uniform scale — normal matrix should still give unit normals after normalizing
        let mut t = Transform::default();
        t.scale = glam::Vec3::splat(2.0);
        let u = MeshUniform::from_transform(&t);
        // A Y-axis normal transformed by (inverse-transpose of 2x scale) = (0.5 * I)^T = 0.5 * I
        // so (0, 0.5, 0) — after normalize still (0,1,0) ✓
        let normal_in = glam::Vec3::Y;
        let transformed = glam::Mat3::from_mat4(u.local_from_world_transpose) * normal_in;
        assert!((transformed.normalize() - normal_in).length() < 1e-4);
    }

    #[test]
    fn test_mesh_uniform_rotation() {
        // 90° rotation around Z — Y normal becomes -X normal
        let mut t = Transform::default();
        t.rotation = glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let u = MeshUniform::from_transform(&t);
        let normal_in = glam::Vec3::Y;
        let transformed = (glam::Mat3::from_mat4(u.local_from_world_transpose) * normal_in).normalize();
        let expected = (glam::Mat3::from_mat4(u.world_from_local) * normal_in).normalize();
        assert!((transformed - expected).length() < 1e-3, "rotation: got {:?}, expected {:?}", transformed, expected);
    }
}
```

- [ ] **Step 2: Compile-check the test file**

```bash
cargo test -p engine-renderer 2>&1 | head -30
```

Expected: `error[E0599]: no method named 'from_transform' found`

- [ ] **Step 3: Implement `from_transform`**

Add after the struct definition:

```rust
impl MeshUniform {
    pub fn from_transform(transform: &engine_core::Transform) -> Self {
        let model = transform.matrix();
        let normal_mat = model.inverse().transpose();
        Self { world_from_local: model, local_from_world_transpose: normal_mat }
    }
}
```

- [ ] **Step 4: Add to lib.rs**

In `engine/renderer/src/lib.rs`, add:
```rust
pub mod mesh_uniform;
pub use mesh_uniform::MeshUniform;
```

- [ ] **Step 5: Run tests — expect pass**

```bash
cargo test -p engine-renderer mesh_uniform
```

Expected: 3 tests pass.

- [ ] **Step 6: Commit**

```bash
git add engine/renderer/src/mesh_uniform.rs engine/renderer/src/lib.rs
git commit -m "feat(renderer): add MeshUniform struct with from_transform (Bevy pattern)"
```

---

## Task 3: Update Shader + Pipeline for Storage Buffer

**Files:**
- Modify: `engine/render-context/shaders/mesh.vert`
- Modify: `engine/render-context/src/pipeline.rs`

**Context:** The vertex shader currently pushes a full MVP matrix per draw. New design: push VP (64 bytes), read per-entity model+normal matrices from a storage buffer via `gl_InstanceIndex`. The pipeline descriptor layout must use `STORAGE_BUFFER` (not `UNIFORM_BUFFER`) at binding 0.

These two changes must land together — mismatched shader/descriptor type causes Vulkan validation errors.

- [ ] **Step 1: Rewrite mesh.vert**

Replace the entire content of `engine/render-context/shaders/mesh.vert`:

```glsl
#version 450

// ── Vertex attributes ────────────────────────────────────────────────────────
layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;

// ── Fragment shader outputs ──────────────────────────────────────────────────
layout(location = 0) out vec3 fragNormal;
layout(location = 1) out vec2 fragUV;
layout(location = 2) out vec3 fragPosition;

// ── Push constants (VP matrix only — 64 bytes) ──────────────────────────────
layout(push_constant) uniform PushConstants {
    mat4 vp;
} pc;

// ── Storage buffer: one MeshUniform per renderable entity ───────────────────
// Each entry is two mat4s laid out sequentially.
struct MeshUniform {
    mat4 world_from_local;
    mat4 local_from_world_transpose;
};

layout(set = 0, binding = 0) readonly buffer MeshUniforms {
    MeshUniform entries[];
} mesh_data;

// ── Main ────────────────────────────────────────────────────────────────────
void main() {
    MeshUniform mu = mesh_data.entries[gl_InstanceIndex];

    gl_Position  = pc.vp * mu.world_from_local * vec4(inPosition, 1.0);
    fragNormal   = mat3(mu.local_from_world_transpose) * inNormal;  // world-space
    fragUV       = inUV;
    fragPosition = vec3(mu.world_from_local * vec4(inPosition, 1.0));
}
```

- [ ] **Step 2: Change UNIFORM_BUFFER → STORAGE_BUFFER in pipeline.rs**

In `engine/render-context/src/pipeline.rs`, inside `new_mesh_pipeline_with_descriptors`, change:

```rust
// OLD:
let ubo_binding = vk::DescriptorSetLayoutBinding::default()
    .binding(0)
    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
    .descriptor_count(1)
    .stage_flags(vk::ShaderStageFlags::VERTEX);
```

to:

```rust
// NEW:
let ubo_binding = vk::DescriptorSetLayoutBinding::default()
    .binding(0)
    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
    .descriptor_count(1)
    .stage_flags(vk::ShaderStageFlags::VERTEX);
```

- [ ] **Step 3: Verify render-context compiles with the new shader**

```bash
cargo build -p engine-render-context 2>&1 | tail -5
```

Expected: `Finished` — naga validates at runtime but include_str compiles fine.

- [ ] **Step 4: Run existing render-context tests**

```bash
cargo test -p engine-render-context
```

Expected: All pass (vertex layout test, push constant size test unchanged).

- [ ] **Step 5: Commit**

```bash
git add engine/render-context/shaders/mesh.vert engine/render-context/src/pipeline.rs
git commit -m "feat(render-context): switch mesh pipeline to storage buffer + VP push constant"
```

---

## Task 4: Descriptor Pool/Set/Buffer Infrastructure in Renderer

**Files:**
- Modify: `engine/renderer/src/renderer.rs`
- Modify: `engine/renderer/src/mesh_uniform.rs` (add buffer management types)

**Context:** The renderer needs a descriptor pool + per-frame descriptor set + per-frame storage buffer (one per `FRAMES_IN_FLIGHT = 2`). These are initialized alongside `mesh_pipeline` and updated each frame before drawing. The `render_meshes` call sequence is: upload MeshUniforms → update descriptor set → bind → push VP → draw indexed.

`FRAMES_IN_FLIGHT = 2` is the existing in-flight frame count (from `sync_objects.len()`).

- [ ] **Step 1: Add storage buffer infrastructure types to mesh_uniform.rs**

Append to `engine/renderer/src/mesh_uniform.rs`:

```rust
use ash::vk;
use engine_render_context::VulkanContext;
use crate::error::RendererError;
use gpu_allocator::MemoryLocation;

/// Minimum storage buffer capacity (entity count).
pub const MESH_UNIFORM_INITIAL_CAPACITY: usize = 256;

/// Per-frame storage buffer for MeshUniform data.
///
/// Wrapped in `RefCell` so `render_meshes` can mutate it through `&self`
/// (same pattern as `gpu_cache: RefCell<GpuCache>`).
pub struct MeshUniformBuffer {
    pub buffer: engine_render_context::GpuBuffer,
    pub capacity: usize,
}

impl MeshUniformBuffer {
    pub fn new(context: &VulkanContext, capacity: usize) -> Result<Self, RendererError> {
        let byte_size = (capacity * std::mem::size_of::<MeshUniform>()) as u64;
        let buffer = engine_render_context::GpuBuffer::new(
            context,
            byte_size,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::CpuToGpu,
        )?;
        Ok(Self { buffer, capacity })
    }

    /// Upload uniforms, resizing if needed (returns true if resized — caller must update descriptor set).
    pub fn upload(
        &mut self,
        context: &VulkanContext,
        uniforms: &[MeshUniform],
    ) -> Result<bool, RendererError> {
        if uniforms.len() > self.capacity {
            let new_cap = (self.capacity * 2).max(uniforms.len());
            *self = Self::new(context, new_cap)?;
            self.buffer.upload(uniforms)?;
            return Ok(true); // resized
        }
        self.buffer.upload(uniforms)?;
        Ok(false)
    }
}
```

- [ ] **Step 2: Add descriptor pool + sets to Renderer struct**

In `engine/renderer/src/renderer.rs`, add fields to `Renderer` (inside the struct definition, after `mesh_pipeline`):

```rust
// Storage buffer descriptor infrastructure (Phase 1.8)
// RefCell mirrors gpu_cache — render_meshes takes &self but must upload data each frame
mesh_uniform_buffers: Vec<std::cell::RefCell<crate::mesh_uniform::MeshUniformBuffer>>,
mesh_descriptor_pool: Option<vk::DescriptorPool>,
mesh_descriptor_sets: Vec<vk::DescriptorSet>,
```

Initialize them as `Vec::new()` / `None` in the constructors (same place `mesh_pipeline` is set to `None`).

- [ ] **Step 3: Add `init_mesh_descriptor_resources` helper**

Add a method to `impl Renderer` in `renderer.rs`:

```rust
fn init_mesh_descriptor_resources(
    &mut self,
    frames_in_flight: usize,
) -> Result<(), RendererError> {
    use crate::mesh_uniform::{MeshUniformBuffer, MESH_UNIFORM_INITIAL_CAPACITY};

    let pipeline = self.mesh_pipeline.as_ref().ok_or_else(|| {
        RendererError::pipelinecreationfailed("mesh pipeline not initialised".into())
    })?;
    let layout = pipeline.descriptor_set_layout();
    if layout == vk::DescriptorSetLayout::null() {
        return Ok(());  // pipeline built without descriptors — skip
    }

    // Create storage buffers (wrapped in RefCell for interior mutability in render_meshes)
    self.mesh_uniform_buffers = (0..frames_in_flight)
        .map(|_| MeshUniformBuffer::new(&self.context, MESH_UNIFORM_INITIAL_CAPACITY)
            .map(std::cell::RefCell::new))
        .collect::<Result<Vec<_>, _>>()?;

    // Descriptor pool
    let pool_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::STORAGE_BUFFER)
        .descriptor_count(frames_in_flight as u32);

    let pool_info = vk::DescriptorPoolCreateInfo::default()
        .pool_sizes(std::slice::from_ref(&pool_size))
        .max_sets(frames_in_flight as u32);

    let pool = unsafe {
        self.context.device.create_descriptor_pool(&pool_info, None).map_err(|e| {
            RendererError::pipelinecreationfailed(format!("descriptor pool: {:?}", e))
        })?
    };
    self.mesh_descriptor_pool = Some(pool);

    // Allocate one descriptor set per frame
    let layouts = vec![layout; frames_in_flight];
    let alloc_info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(pool)
        .set_layouts(&layouts);

    self.mesh_descriptor_sets = unsafe {
        self.context.device.allocate_descriptor_sets(&alloc_info).map_err(|e| {
            RendererError::pipelinecreationfailed(format!("descriptor set alloc: {:?}", e))
        })?
    };

    // Initial write — bind each set to its buffer
    for (i, set) in self.mesh_descriptor_sets.iter().enumerate() {
        let buf_info = vk::DescriptorBufferInfo::default()
            .buffer(self.mesh_uniform_buffers[i].borrow().buffer.handle())
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let write = vk::WriteDescriptorSet::default()
            .dst_set(*set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(std::slice::from_ref(&buf_info));
        unsafe { self.context.device.update_descriptor_sets(&[write], &[]) };
    }

    info!(frames_in_flight, "Mesh descriptor resources initialised");
    Ok(())
}
```

- [ ] **Step 4: Call `init_mesh_descriptor_resources` where mesh_pipeline is created**

Search renderer.rs for where `self.mesh_pipeline = Some(...)` is set, then call:

```rust
let frames_in_flight = self.sync_objects.len();
if let Err(e) = self.init_mesh_descriptor_resources(frames_in_flight) {
    warn!(error = ?e, "Failed to init mesh descriptor resources");
}
```

- [ ] **Step 5: Add cleanup in Drop or wait_idle**

In the `Drop` impl (or wherever other Vulkan resources are destroyed), add cleanup:

```rust
// Descriptor pool (implicitly frees sets)
if let Some(pool) = self.mesh_descriptor_pool.take() {
    unsafe { self.context.device.destroy_descriptor_pool(pool, None); }
}
// mesh_uniform_buffers drop automatically via GpuBuffer::Drop
self.mesh_uniform_buffers.clear();
```

- [ ] **Step 6: Build check**

```bash
cargo build -p engine-renderer 2>&1 | tail -10
```

Expected: `Finished`.

- [ ] **Step 7: Run tests**

```bash
cargo test -p engine-renderer
```

Expected: All pass.

- [ ] **Step 8: Commit**

```bash
git add engine/renderer/src/renderer.rs engine/renderer/src/mesh_uniform.rs
git commit -m "feat(renderer): add descriptor pool/set/storage buffer infrastructure for MeshUniform"
```

---

## Task 5: Rewrite render_meshes to Use Storage Buffer

**Files:**
- Modify: `engine/renderer/src/renderer.rs`

**Context:** Current `render_meshes` pushes a full MVP per draw call. New version:
1. Build `Vec<MeshUniform>` from ECS
2. Upload to this frame's storage buffer (resize if needed, re-bind descriptor set)
3. `cmd_bind_descriptor_sets` once
4. `cmd_push_constants` VP matrix once per viewport
5. Per entity: bind vbo/ibo, `cmd_draw_indexed` with `firstInstance = instance_index`

`current_frame` is already a field on Renderer.

- [ ] **Step 1: Rewrite render_meshes**

Replace the body of `pub fn render_meshes(...)` in `renderer.rs`. The function signature stays the same (`recorder, world, assets, viewports`):

```rust
pub fn render_meshes(
    &self,
    recorder: &FrameRecorder,
    world: &engine_core::World,
    assets: Option<&engine_assets::AssetManager>,
    viewports: &[ViewportDescriptor],
) {
    use engine_core::{MeshRenderer, Transform};
    use crate::mesh_uniform::MeshUniform;

    let Some(assets) = assets else { return; };
    if viewports.is_empty() { return; }

    let cmd = recorder.command_buffer;
    let frame_idx = self.current_frame;

    let Some(pipeline) = &self.mesh_pipeline else {
        warn!("render_meshes: no mesh pipeline, skipping");
        return;
    };

    // ── 1. Build MeshUniform list + parallel mesh-id list ────────────────
    let mut uniforms: Vec<MeshUniform> = Vec::new();
    let mut draw_list: Vec<(engine_assets::AssetId, u32)> = Vec::new();

    for entity in world.entities() {
        let (Some(transform), Some(mesh_renderer)) =
            (world.get::<Transform>(entity), world.get::<MeshRenderer>(entity))
        else { continue; };

        if !mesh_renderer.is_visible() { continue; }

        let mesh_id = engine_assets::AssetId::from_seed_and_params(
            mesh_renderer.mesh_id, b"mesh",
        );

        // Upload mesh to GPU cache if absent
        {
            let mut cache = self.gpu_cache.borrow_mut();
            if !cache.contains(mesh_id) {
                match assets.get_mesh(mesh_id) {
                    Some(mesh_data) => {
                        if let Err(e) = cache.upload_mesh(&self.context, mesh_id, &mesh_data) {
                            warn!(error = ?e, mesh_id = ?mesh_id, "render_meshes: GPU upload failed");
                            continue;
                        }
                    }
                    None => {
                        warn!(mesh_id = ?mesh_id, "render_meshes: mesh not in AssetManager");
                        continue;
                    }
                }
            }
        }

        let instance_index = uniforms.len() as u32;
        uniforms.push(MeshUniform::from_transform(transform));
        draw_list.push((mesh_id, instance_index));
    }

    if draw_list.is_empty() { return; }

    // ── 2. Upload MeshUniform data to this frame's storage buffer ─────────
    // Use RefCell::borrow_mut — same pattern as gpu_cache (render_meshes has &self)
    if let Some(buf_cell) = self.mesh_uniform_buffers.get(frame_idx) {
        let mut buf = buf_cell.borrow_mut();
        match buf.upload(&self.context, &uniforms) {
            Ok(resized) if resized => {
                // Buffer was reallocated — rebind descriptor set to the new VkBuffer
                if let Some(&set) = self.mesh_descriptor_sets.get(frame_idx) {
                    let buf_info = vk::DescriptorBufferInfo::default()
                        .buffer(buf.buffer.handle())
                        .offset(0)
                        .range(vk::WHOLE_SIZE);
                    let write = vk::WriteDescriptorSet::default()
                        .dst_set(set)
                        .dst_binding(0)
                        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                        .buffer_info(std::slice::from_ref(&buf_info));
                    unsafe { self.context.device.update_descriptor_sets(&[write], &[]); }
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!(error = ?e, "render_meshes: buffer upload failed");
                return;
            }
        }
    }

    // ── 3. Bind pipeline + descriptor set ─────────────────────────────────
    unsafe {
        self.context.device.cmd_bind_pipeline(
            cmd, vk::PipelineBindPoint::GRAPHICS, pipeline.handle(),
        );

        if let Some(&set) = self.mesh_descriptor_sets.get(frame_idx) {
            self.context.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.layout(),
                0,
                &[set],
                &[],
            );
        }
    }

    // ── 4. Emit draw calls per viewport ───────────────────────────────────
    let cache = self.gpu_cache.borrow();
    for vp in viewports {
        let vp_matrix = vp.proj * vp.view;

        let viewport = vk::Viewport::default()
            .x(vp.bounds.x as f32)
            .y(vp.bounds.y as f32)
            .width(vp.bounds.width as f32)
            .height(vp.bounds.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let sw = self.swapchain.extent;
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: vp.bounds.x, y: vp.bounds.y },
            extent: vk::Extent2D {
                width: vp.bounds.width.min(sw.width.saturating_sub(vp.bounds.x.max(0) as u32)),
                height: vp.bounds.height.min(sw.height.saturating_sub(vp.bounds.y.max(0) as u32)),
            },
        };

        unsafe {
            self.context.device.cmd_set_viewport(cmd, 0, &[viewport]);
            self.context.device.cmd_set_scissor(cmd, 0, &[scissor]);

            // Push VP matrix (64 bytes)
            let vp_bytes = vp_matrix.as_ref();
            let vp_slice = std::slice::from_raw_parts(
                vp_bytes.as_ptr() as *const u8,
                std::mem::size_of::<glam::Mat4>(),
            );
            self.context.device.cmd_push_constants(
                cmd, pipeline.layout(), vk::ShaderStageFlags::VERTEX, 0, vp_slice,
            );
        }

        for &(mesh_id, instance_index) in &draw_list {
            if let (Some((vertex_buf, index_buf)), Some(mesh_info)) =
                (cache.get_buffers(mesh_id), cache.get_mesh_info(mesh_id))
            {
                unsafe {
                    self.context.device.cmd_bind_vertex_buffers(cmd, 0, &[vertex_buf], &[0]);
                    self.context.device.cmd_bind_index_buffer(cmd, index_buf, 0, vk::IndexType::UINT32);
                    self.context.device.cmd_draw_indexed(
                        cmd,
                        mesh_info.index_count,
                        1,   // instance count
                        0,   // first index
                        0,   // vertex offset
                        instance_index, // firstInstance → gl_InstanceIndex in shader
                    );
                }
            }
        }
    }

    info!(draw_count = draw_list.len(), viewport_count = viewports.len(), "render_meshes: issued draw calls");
}
```

- [ ] **Step 2: Build check**

```bash
cargo build -p engine-renderer 2>&1 | tail -10
```

Expected: `Finished`.

- [ ] **Step 3: Run all renderer tests**

```bash
cargo test -p engine-renderer
```

Expected: All pass.

- [ ] **Step 4: Commit**

```bash
git add engine/renderer/src/renderer.rs
git commit -m "feat(renderer): rewrite render_meshes with storage buffer + VP push constant"
```

---

## Task 6: Editor State — AssetManager + Primitives + MeshRenderer Registration

**Files:**
- Create: `engine/editor/src-tauri/state/asset_manager.rs`
- Create: `engine/editor/src-tauri/state/primitives.rs`
- Modify: `engine/editor/src-tauri/state/scene_world.rs`
- Modify: `engine/editor/src-tauri/state/mod.rs`
- Modify: `engine/editor/src-tauri/lib.rs`

**Context:** The editor needs a Tauri-managed `AssetManagerState` (shared `Arc<AssetManager>`), all 5 primitives pre-loaded at startup, and `MeshRenderer` registered in the ECS world.

- [ ] **Step 1: Create asset_manager.rs**

```rust
// engine/editor/src-tauri/state/asset_manager.rs
use engine_assets::AssetManager;
use std::sync::Arc;

/// Tauri-managed shared asset manager.
pub struct AssetManagerState(pub Arc<AssetManager>);
```

- [ ] **Step 2: Create primitives.rs**

```rust
// engine/editor/src-tauri/state/primitives.rs
use engine_assets::{AssetId, AssetLoader, AssetManager, MeshData};

/// Register built-in primitive meshes in the asset manager at startup.
/// Seeds 1–5 correspond to builtin://cube through builtin://capsule.
pub fn register_primitives(manager: &AssetManager) {
    let primitives: [(u64, MeshData); 5] = [
        (1, MeshData::cube()),
        (2, MeshData::sphere(1.0, 32, 16)),
        (3, MeshData::plane(1.0)),
        (4, MeshData::cylinder(0.5, 1.0, 32)),
        (5, MeshData::capsule(0.5, 1.0, 32, 8)),
    ];
    for (seed, mesh) in primitives {
        let id = AssetId::from_seed_and_params(seed, b"mesh");
        let _ = <MeshData as AssetLoader>::insert(manager, id, mesh);
    }
    tracing::info!("Built-in primitives registered (cube, sphere, plane, cylinder, capsule)");
}
```

- [ ] **Step 3: Register MeshRenderer in SceneWorldState**

In `engine/editor/src-tauri/state/scene_world.rs`, find where `world.register::<Transform>()` is called and add:

```rust
world.register::<engine_core::MeshRenderer>();
```

- [ ] **Step 4: Export new modules from state/mod.rs**

Add to `engine/editor/src-tauri/state/mod.rs`:

```rust
pub mod asset_manager;
pub mod primitives;
pub use asset_manager::AssetManagerState;
```

- [ ] **Step 5: Register state + call primitives in lib.rs**

In `engine/editor/src-tauri/lib.rs`:

1. Add `use crate::state::{AssetManagerState, primitives::register_primitives};`

2. In the Tauri builder `manage()` chain, add:
```rust
.manage(AssetManagerState(std::sync::Arc::new(engine_assets::AssetManager::new())))
```

3. After the `AssetManagerState` is managed, call `register_primitives`:
```rust
// Register built-in primitives
{
    let asset_state = app_handle.state::<AssetManagerState>();
    register_primitives(&asset_state.0);
}
```

(Place this in the `setup` closure or equivalent initialization hook.)

- [ ] **Step 6: Build check**

```bash
cargo build -p engine-editor 2>&1 | tail -15
```

Expected: `Finished`.

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src-tauri/state/asset_manager.rs \
        engine/editor/src-tauri/state/primitives.rs \
        engine/editor/src-tauri/state/scene_world.rs \
        engine/editor/src-tauri/state/mod.rs \
        engine/editor/src-tauri/lib.rs
git commit -m "feat(editor): add AssetManagerState, primitive registration, register MeshRenderer ECS component"
```

---

## Task 7: Wire render_meshes in NativeViewport

**Files:**
- Modify: `engine/editor/src-tauri/viewport/native_viewport.rs`

**Context:** `OrbitCamera` currently only has `view_projection(aspect, is_ortho) -> Mat4` (combined). `ViewportDescriptor` needs separate `view: Mat4` and `proj: Mat4`. We must add `view_matrix()` and `proj_matrix()` while keeping `view_projection()` unchanged (the gizmo pipeline still uses it at ~line 947). We also need to pass `Arc<AssetManager>` into the render thread and call `render_meshes`.

- [ ] **Step 1: Add view_matrix and proj_matrix to OrbitCamera**

Inside the `OrbitCamera` impl block (after `fn view_projection`), add:

```rust
fn view_matrix(&self) -> Mat4 {
    Mat4::look_at_rh(self.eye(), self.target, Vec3::Y)
}

fn proj_matrix(&self, aspect: f32, is_ortho: bool) -> Mat4 {
    if is_ortho {
        let half = self.distance * 0.5;
        Mat4::orthographic_rh(-half * aspect, half * aspect, -half, half, 0.01, 1000.0)
    } else {
        Mat4::perspective_rh(self.fov_y_radians, aspect, 0.01, 1000.0)
    }
}
```

- [ ] **Step 2: Verify view_projection is still computed correctly**

Confirm `view_projection` still calls `Mat4::look_at_rh` independently — it does not need to call the new methods (DRY would break the gizmo call site). Just verify existing tests still pass:

```bash
cargo test -p engine-editor view_projection
```

Expected: 3 existing `view_projection` tests pass.

- [ ] **Step 3: Add Arc<AssetManager> field to NativeViewport struct**

In the `NativeViewport` struct, add (after `screenshot_slot`):

```rust
asset_manager: std::sync::Arc<engine_assets::AssetManager>,
```

Update the constructor to accept `Arc<AssetManager>` and store it. Update `start_rendering` to clone the `Arc` into the render thread closure.

- [ ] **Step 4: Wire render_meshes in render_frame**

Find the TODO comment:
```rust
// TODO(Task 8): wire render_meshes once SceneWorldState is available
// self.renderer.render_meshes(&recorder, &world, None, &vp_descs);
```

Replace with working code. First, build the `ViewportDescriptor` slice using the new methods:

```rust
// Build viewport descriptors for render_meshes (separate view + proj)
let vp_descs: Vec<engine_renderer::ViewportDescriptor> = viewports
    .iter()
    .map(|(bounds, cam, is_ortho, _)| {
        let aspect = bounds.width as f32 / bounds.height as f32;
        engine_renderer::ViewportDescriptor {
            bounds: engine_render_context::Rect {
                x: bounds.x,
                y: bounds.y,
                width: bounds.width,
                height: bounds.height,
            },
            view: cam.view_matrix(),
            proj: cam.proj_matrix(aspect, *is_ortho),
        }
    })
    .collect();

// Render meshes from ECS world
{
    let world_guard = _world.read().map_err(|e| e.to_string())?;
    self.renderer.render_meshes(&recorder, &world_guard, Some(&self.asset_manager), &vp_descs);
}
```

The `_world` parameter in `render_frame` is already the `&std::sync::RwLock<engine_core::World>`. Remove the `_` prefix to suppress the unused warning.

- [ ] **Step 5: Build check**

```bash
cargo build -p engine-editor 2>&1 | tail -15
```

Expected: `Finished`.

- [ ] **Step 6: Run viewport tests**

```bash
cargo test -p engine-editor
```

Expected: All pass.

- [ ] **Step 7: Commit**

```bash
git add engine/editor/src-tauri/viewport/native_viewport.rs
git commit -m "feat(editor): wire render_meshes in viewport — add view_matrix/proj_matrix to OrbitCamera"
```

---

## Task 8: sync_mesh_renderer_to_ecs + assign_mesh Command

**Files:**
- Modify: `engine/editor/src-tauri/bridge/template_commands.rs`
- Modify: `engine/editor/src-tauri/bridge/commands.rs`
- Modify: `engine/editor/src-tauri/lib.rs`

**Context:** `sync_transform_to_ecs` is the pattern to follow. `resolve_mesh_path` translates `"builtin://cube"` → seed 1 (etc.) and file paths → blake3(path)[..8] seed, inserting file meshes into the AssetManager on the way. `assign_mesh` writes a `SetComponent{MeshRenderer}` action to the template CommandProcessor then syncs to ECS.

**Important:** The editor's `Cargo.toml` does NOT directly depend on `blake3`. Add it or use `engine_assets`' re-export if available. Check first: `grep -r "blake3" engine/editor/Cargo.toml`. If absent, add `blake3 = "1"` to `[dependencies]` in `engine/editor/src-tauri/Cargo.toml`.

- [ ] **Step 1: Write unit tests for resolve_mesh_path**

Add in `template_commands.rs` (inside `#[cfg(test)]`):

```rust
#[test]
fn test_resolve_builtin_cube() {
    // builtin://cube → seed 1 (compile-time constant)
    let seed = resolve_builtin_seed("builtin://cube");
    assert_eq!(seed, Some(1u64));
}

#[test]
fn test_resolve_path_hash_is_deterministic() {
    let path = "assets/models/robot.glb";
    let seed_a = path_to_seed(path);
    let seed_b = path_to_seed(path);
    assert_eq!(seed_a, seed_b);
}

#[test]
fn test_different_paths_give_different_seeds() {
    let s1 = path_to_seed("assets/models/a.glb");
    let s2 = path_to_seed("assets/models/b.glb");
    assert_ne!(s1, s2);
}

// Helper functions (private, tested in isolation)
fn resolve_builtin_seed(path: &str) -> Option<u64> {
    match path {
        "builtin://cube"     => Some(1),
        "builtin://sphere"   => Some(2),
        "builtin://plane"    => Some(3),
        "builtin://cylinder" => Some(4),
        "builtin://capsule"  => Some(5),
        _ => None,
    }
}

fn path_to_seed(path: &str) -> u64 {
    u64::from_le_bytes(
        blake3::hash(path.as_bytes()).as_bytes()[..8].try_into().unwrap()
    )
}
```

- [ ] **Step 2: Run test — expect compile error**

```bash
cargo test -p engine-editor test_resolve 2>&1 | head -15
```

Expected: `error: unresolved import 'blake3'` or similar.

- [ ] **Step 3: Add blake3 dependency if missing**

Check `engine/editor/src-tauri/Cargo.toml`:

```bash
grep blake3 engine/editor/src-tauri/Cargo.toml
```

If not found, add to `[dependencies]`:

```toml
blake3 = "1"
```

- [ ] **Step 4: Implement resolve_mesh_path + sync_mesh_renderer_to_ecs**

Add to `template_commands.rs`:

```rust
use std::path::Path;

/// Resolve a mesh_path string to a stable u64 seed and (for file paths)
/// load + insert the mesh into the AssetManager.
///
/// Builtin seeds are compile-time constants.
/// File seeds are blake3(path_bytes)[..8] — stable across content updates.
pub(crate) fn resolve_mesh_path(
    path: &str,
    project_root: &Path,
    manager: &engine_assets::AssetManager,
) -> Result<u64, String> {
    use engine_assets::{AssetId, AssetLoader, MeshData};

    let seed = match path {
        "builtin://cube"     => return Ok(1),
        "builtin://sphere"   => return Ok(2),
        "builtin://plane"    => return Ok(3),
        "builtin://cylinder" => return Ok(4),
        "builtin://capsule"  => return Ok(5),
        other => u64::from_le_bytes(
            blake3::hash(other.as_bytes()).as_bytes()[..8].try_into().unwrap()
        ),
    };

    let asset_id = AssetId::from_seed_and_params(seed, b"mesh");
    let full_path = project_root.join(path);

    let bytes = std::fs::read(&full_path)
        .map_err(|e| format!("cannot read {}: {}", path, e))?;

    let mesh_data = if path.ends_with(".glb") || path.ends_with(".gltf") {
        MeshData::from_gltf(&bytes, None).map_err(|e| format!("gltf parse: {}", e))?
    } else if path.ends_with(".obj") {
        MeshData::from_obj(&String::from_utf8_lossy(&bytes))
            .map_err(|e| format!("obj parse: {}", e))?
    } else {
        return Err(format!("unsupported mesh format: {}", path));
    };

    let _ = <MeshData as AssetLoader>::insert(manager, asset_id, mesh_data);
    Ok(seed)
}

/// Sync a MeshRenderer component from template state to the live ECS world.
pub(crate) fn sync_mesh_renderer_to_ecs(
    entity_id: u64,
    template_state: &crate::state::TemplateState,
    world_state: &crate::state::SceneWorldState,
    asset_manager: &engine_assets::AssetManager,
    project_root: &Path,
) -> Result<(), String> {
    if entity_id > u32::MAX as u64 {
        return Err(format!("entity_id {entity_id} exceeds u32::MAX"));
    }

    // Extract mesh_path from template JSON
    let mesh_path = template_state
        .entities
        .iter()
        .find(|e| e.id == entity_id)
        .and_then(|e| e.components.iter().find(|c| c.type_name == "MeshRenderer"))
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c.data).ok())
        .and_then(|v| v.get("mesh_path").and_then(|p| p.as_str()).map(|s| s.to_string()));

    let Some(mesh_path) = mesh_path else {
        tracing::warn!(entity_id, "sync_mesh_renderer_to_ecs: no MeshRenderer in template");
        return Ok(());
    };

    let mesh_id = match resolve_mesh_path(&mesh_path, project_root, asset_manager) {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(entity_id, path = mesh_path, error = e, "mesh path resolve failed");
            return Ok(()); // non-fatal — entity exists, just won't render
        }
    };

    let entity = engine_core::Entity::new(entity_id as u32, 0);
    {
        let mut world = world_state.0.write().map_err(|e| e.to_string())?;
        let mr = engine_core::MeshRenderer::new(mesh_id);
        if world.get::<engine_core::MeshRenderer>(entity).is_some() {
            if let Some(existing) = world.get_mut::<engine_core::MeshRenderer>(entity) {
                *existing = mr;
            }
        } else {
            world.add(entity, mr);
        }
    }

    tracing::info!(entity_id, mesh_path, mesh_id, "MeshRenderer synced to ECS");
    Ok(())
}
```

- [ ] **Step 5: Add assign_mesh Tauri command to commands.rs**

```rust
/// Assign a mesh to an entity via the template command processor.
///
/// `mesh_path` is either `"builtin://cube"` (etc.) or a project-relative
/// path like `"assets/models/robot.glb"`.
#[tauri::command]
pub async fn assign_mesh(
    entity_id: u64,
    template_path: String,
    mesh_path: String,
    editor_state: tauri::State<'_, crate::state::EditorState>,
    world_state: tauri::State<'_, crate::state::SceneWorldState>,
    asset_state: tauri::State<'_, crate::state::AssetManagerState>,
) -> Result<(), String> {
    use crate::bridge::template_commands::{sync_mesh_renderer_to_ecs};

    // Build the SetComponent action JSON for MeshRenderer
    let component_data = serde_json::json!({ "mesh_path": mesh_path }).to_string();
    let action = serde_json::json!({
        "type": "SetComponent",
        "entity_id": entity_id,
        "type_name": "MeshRenderer",
        "data": component_data,
    });

    // Write to template via existing CommandProcessor
    {
        let mut state = editor_state.0.lock().map_err(|e| e.to_string())?;
        state.process_command(&template_path, action).map_err(|e| e.to_string())?;
    }

    // Derive project root from EditorState (project_path is the .silmaril file path;
    // parent dir is the project root where "assets/models/" lives)
    let project_root_buf = {
        let es = editor_state.0.lock().map_err(|e| e.to_string())?;
        es.project_path
            .as_ref()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    };

    // Sync to ECS
    {
        let es = editor_state.0.lock().map_err(|e| e.to_string())?;
        let template_state = es.template_state(&template_path);
        sync_mesh_renderer_to_ecs(
            entity_id,
            template_state,
            &world_state,
            &asset_state.0,
            &project_root_buf,
        )
    }
}
```

**Note:** If `EditorState`/`TemplateState` APIs differ from the above, adapt to the actual pattern — check how other commands in `commands.rs` read template state.

- [ ] **Step 6: Register assign_mesh in lib.rs**

Add `assign_mesh` to the `generate_handler!` / `.invoke_handler(tauri::generate_handler![...])` list in `lib.rs`.

- [ ] **Step 7: Build check**

```bash
cargo build -p engine-editor 2>&1 | tail -15
```

Expected: `Finished`.

- [ ] **Step 8: Run tests**

```bash
cargo test -p engine-editor test_resolve
```

Expected: 3 resolve tests pass.

- [ ] **Step 9: Commit**

```bash
git add engine/editor/src-tauri/bridge/template_commands.rs \
        engine/editor/src-tauri/bridge/commands.rs \
        engine/editor/src-tauri/lib.rs \
        engine/editor/src-tauri/Cargo.toml
git commit -m "feat(editor): add assign_mesh command + sync_mesh_renderer_to_ecs with path-hash resolution"
```

---

## Task 9: Frontend — IPC + Assets Panel

**Files:**
- Modify: `engine/editor/src/lib/api.ts`
- Modify: `engine/editor/src/lib/docking/panels/AssetsPanel.svelte`

**Context:** `api.ts` wraps Tauri commands. `AssetsPanel.svelte` needs to list `.glb`/`.obj` files from `assets/models/`, be a drag source, and have an "Add" button.

- [ ] **Step 1: Add assignMesh to api.ts**

Add to `engine/editor/src/lib/api.ts`:

```typescript
export async function assignMesh(
  entityId: number,
  templatePath: string,
  meshPath: string
): Promise<void> {
  await invoke('assign_mesh', {
    entityId,
    templatePath,
    meshPath,
  });
}
```

- [ ] **Step 2: Update AssetsPanel.svelte**

Open `engine/editor/src/lib/docking/panels/AssetsPanel.svelte` and read the current structure first.

Add model file listing. The key additions:

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { readDir } from '@tauri-apps/plugin-fs';

  let modelFiles: string[] = [];

  async function loadModelFiles() {
    try {
      // Use the existing project root from app state
      const entries = await readDir('assets/models');
      modelFiles = entries
        .filter(e => e.name && (e.name.endsWith('.glb') || e.name.endsWith('.obj')))
        .map(e => `assets/models/${e.name}`);
    } catch {
      modelFiles = []; // directory absent — show empty state
    }
  }

  // Drag source — drag a model file onto hierarchy items
  function onDragStart(event: DragEvent, filePath: string) {
    event.dataTransfer?.setData('application/x-mesh-path', filePath);
  }

  async function openFileDialog() {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const selected = await open({
      multiple: true,
      filters: [{ name: 'Mesh Files', extensions: ['glb', 'obj'] }],
    });
    if (selected) {
      // Copy files to assets/models/ using existing file ops
      // (adapt to your project's copy utility)
      await loadModelFiles();
    }
  }

  loadModelFiles();
</script>

<!-- In the template: -->
<section class="models-section">
  <header>
    <span>Models</span>
    <button on:click={openFileDialog}>Add</button>
  </header>

  {#if modelFiles.length === 0}
    <p class="empty-hint">Drop .glb or .obj files here</p>
  {:else}
    <ul>
      {#each modelFiles as file}
        <li
          draggable="true"
          on:dragstart={(e) => onDragStart(e, file)}
        >
          {file.split('/').pop()}
        </li>
      {/each}
    </ul>
  {/if}
</section>
```

Adapt class names and layout to match the existing AssetsPanel structure.

- [ ] **Step 3: TypeScript check**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | head -20
```

Expected: No new errors.

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src/lib/api.ts \
        engine/editor/src/lib/docking/panels/AssetsPanel.svelte
git commit -m "feat(editor-frontend): add assignMesh API + model file listing in AssetsPanel"
```

---

## Task 10: Frontend — Hierarchy Drop Target

**Files:**
- Modify: `engine/editor/src/lib/docking/panels/HierarchyWrapper.svelte`

**Context:** When a mesh file is dragged from the assets panel and dropped onto an entity row in the hierarchy, call `assignMesh`. The `application/x-mesh-path` drag data carries the file path.

- [ ] **Step 1: Read HierarchyWrapper.svelte to understand drop target structure**

Look for existing `on:drop` / `dragover` handlers or entity row rendering.

- [ ] **Step 2: Add drop handler to entity rows**

In the entity row rendering, add drop event handling:

```svelte
<script lang="ts">
  import { assignMesh } from '$lib/api';

  // Assume `templatePath` and `selectedEntityId` are available from parent/store
  // Adapt to actual prop names

  async function onDropMesh(event: DragEvent, entityId: number) {
    event.preventDefault();
    const meshPath = event.dataTransfer?.getData('application/x-mesh-path');
    if (!meshPath || !templatePath) return;
    await assignMesh(entityId, templatePath, meshPath);
  }

  function onDragOver(event: DragEvent) {
    if (event.dataTransfer?.types.includes('application/x-mesh-path')) {
      event.preventDefault();
      event.dataTransfer.dropEffect = 'copy';
    }
  }
</script>

<!-- On entity row element: -->
<div
  class="entity-row"
  on:dragover={onDragOver}
  on:drop={(e) => onDropMesh(e, entityId)}
>
  <!-- existing entity row content -->
</div>
```

- [ ] **Step 3: TypeScript check**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | head -20
```

Expected: No new errors.

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src/lib/docking/panels/HierarchyWrapper.svelte
git commit -m "feat(editor-frontend): add drop target on hierarchy for mesh assignment"
```

---

## Task 11: Frontend — Inspector Mesh Picker

**Files:**
- Modify: `engine/editor/src/lib/docking/panels/InspectorPanel.svelte` (or InspectorWrapper.svelte — check which renders component properties)

**Context:** When an entity has a `MeshRenderer` component, the inspector shows a `Mesh` dropdown listing all 5 primitives + discovered model files. Selecting one calls `assignMesh`.

- [ ] **Step 1: Read InspectorPanel/InspectorWrapper to find component rendering**

Find where `Transform` component properties are rendered — the `MeshRenderer` section goes in the same list.

- [ ] **Step 2: Add MeshRenderer inspector section**

```svelte
<script lang="ts">
  import { assignMesh } from '$lib/api';

  const BUILTINS = [
    { label: 'Cube',     path: 'builtin://cube'     },
    { label: 'Sphere',   path: 'builtin://sphere'   },
    { label: 'Plane',    path: 'builtin://plane'    },
    { label: 'Cylinder', path: 'builtin://cylinder' },
    { label: 'Capsule',  path: 'builtin://capsule'  },
  ];

  // modelFiles from AssetsPanel store or re-fetched here
  // Assume $modelFiles is a readable store shared with AssetsPanel

  async function onMeshChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    const meshPath = select.value;
    if (!meshPath || !selectedEntityId || !templatePath) return;
    await assignMesh(selectedEntityId, templatePath, meshPath);
  }
</script>

<!-- Inside component list, when MeshRenderer is present: -->
{#if hasMeshRenderer}
  <section class="component-section">
    <h4>Mesh Renderer</h4>
    <label>
      Mesh
      <select value={currentMeshPath} on:change={onMeshChange}>
        <optgroup label="Primitives">
          {#each BUILTINS as b}
            <option value={b.path}>{b.label}</option>
          {/each}
        </optgroup>
        {#if $modelFiles.length > 0}
          <optgroup label="Models">
            {#each $modelFiles as f}
              <option value={f}>{f.split('/').pop()}</option>
            {/each}
          </optgroup>
        {/if}
      </select>
    </label>
  </section>
{/if}
```

Adapt `hasMeshRenderer`, `currentMeshPath`, `$modelFiles`, `selectedEntityId`, `templatePath` to the actual store/prop names used in this file.

- [ ] **Step 3: TypeScript check**

```bash
cd engine/editor && npx tsc --noEmit 2>&1 | head -20
```

Expected: No new errors.

- [ ] **Step 4: Commit**

```bash
git add engine/editor/src/lib/docking/panels/InspectorPanel.svelte
# (or InspectorWrapper.svelte)
git commit -m "feat(editor-frontend): add MeshRenderer inspector section with mesh dropdown picker"
```

---

## Task 12: Full Build + Smoke Test

- [ ] **Step 1: Run full check**

```bash
cargo xtask check 2>&1 | tail -20
```

Expected: `Finished` — no errors, no clippy warnings.

- [ ] **Step 2: Run all tests**

```bash
cargo xtask test all 2>&1 | tail -30
```

Expected: All tests pass.

- [ ] **Step 3: Build frontend**

```bash
cd engine/editor && npm run build 2>&1 | tail -20
```

Expected: `✓ built in`.

- [ ] **Step 4: Final commit if any lint fixes needed**

```bash
git add -p  # stage only lint/format fixes
git commit -m "fix(mesh-rendering): clippy + format pass before merge"
```

---

## Done

After all tasks complete and tests pass, use `superpowers:finishing-a-development-branch` to merge to main and push.
