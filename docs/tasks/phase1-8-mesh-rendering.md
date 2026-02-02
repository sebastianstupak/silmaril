# Phase 1.8 - Mesh Rendering

**Estimated Time**: 3-4 days
**Status**: Not Started
**Dependencies**: Phase 1.7 complete (Asset System)

---

## Overview

Implement mesh rendering using the asset system created in Phase 1.7. This phase focuses on the graphics pipeline, shaders, transform components, camera system, and ECS integration for rendering meshes.

**Key Principle**: Asset data (MeshData) is loaded via asset system. This phase handles GPU upload and rendering.

---

## Prerequisites from Phase 1.7

✅ **engine-assets crate**: MeshData, Vertex, procedural primitives
✅ **engine-renderer buffer module**: GpuBuffer, GpuMesh, VertexBuffer, IndexBuffer
✅ **Shaders**: mesh.vert, mesh.frag (GLSL)
✅ **Asset Manager**: AssetHandle<MeshData>, loading strategies

**What's Already Done**:
- MeshData structure (32-byte vertices with position, normal, UV)
- GPU buffer management (GpuBuffer, VertexBuffer, IndexBuffer)
- GpuMesh::from_mesh_data() for uploading
- Basic vertex and fragment shaders

**What Remains**:
- Graphics pipeline creation
- Transform component (ECS)
- Camera system
- Rendering integration with ECS
- Depth buffer
- Testing and benchmarks

---

## Task Breakdown

### Task 1: Graphics Pipeline (1 day)

**Goal**: Create Vulkan graphics pipeline for mesh rendering with MVP matrix push constants.

**Sub-tasks**:

#### 1.1: Pipeline Layout (0.25 days)
```rust
// engine/renderer/src/pipeline.rs
pub struct PipelineLayout {
    layout: vk::PipelineLayout,
    push_constant_ranges: Vec<vk::PushConstantRange>,
}

impl PipelineLayout {
    pub fn new(context: &VulkanContext) -> Result<Self, RendererError> {
        // Push constant for MVP matrix (64 bytes = mat4)
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(64); // sizeof(mat4)

        let create_info = vk::PipelineLayoutCreateInfo::default()
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        let layout = unsafe {
            context.device.create_pipeline_layout(&create_info, None)?
        };

        Ok(Self {
            layout,
            push_constant_ranges: vec![push_constant_range],
        })
    }
}
```

#### 1.2: Vertex Input State (0.25 days)
```rust
pub fn vertex_input_state() -> (vk::PipelineVertexInputStateCreateInfo, Vec<vk::VertexInputBindingDescription>, Vec<vk::VertexInputAttributeDescription>) {
    // Binding 0: Vertex buffer (32 bytes per vertex)
    let binding = vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride(32) // sizeof(Vertex)
        .input_rate(vk::VertexInputRate::VERTEX);

    // Attribute 0: Position (vec3, offset 0)
    let pos_attr = vk::VertexInputAttributeDescription::default()
        .location(0)
        .binding(0)
        .format(vk::Format::R32G32B32_SFLOAT)
        .offset(0);

    // Attribute 1: Normal (vec3, offset 12)
    let normal_attr = vk::VertexInputAttributeDescription::default()
        .location(1)
        .binding(0)
        .format(vk::Format::R32G32B32_SFLOAT)
        .offset(12);

    // Attribute 2: UV (vec2, offset 24)
    let uv_attr = vk::VertexInputAttributeDescription::default()
        .location(2)
        .binding(0)
        .format(vk::Format::R32G32_SFLOAT)
        .offset(24);

    let bindings = vec![binding];
    let attributes = vec![pos_attr, normal_attr, uv_attr];

    let create_info = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&bindings)
        .vertex_attribute_descriptions(&attributes);

    (create_info, bindings, attributes)
}
```

#### 1.3: Pipeline Creation (0.5 days)
```rust
pub struct GraphicsPipeline {
    pipeline: vk::Pipeline,
    layout: PipelineLayout,
}

impl GraphicsPipeline {
    pub fn new(
        context: &VulkanContext,
        render_pass: &RenderPass,
        vertex_shader: &ShaderModule,
        fragment_shader: &ShaderModule,
    ) -> Result<Self, RendererError> {
        let layout = PipelineLayout::new(context)?;

        // Shader stages
        let stages = [
            vertex_shader.stage_info(vk::ShaderStageFlags::VERTEX),
            fragment_shader.stage_info(vk::ShaderStageFlags::FRAGMENT),
        ];

        // Vertex input state
        let (vertex_input, bindings, attributes) = vertex_input_state();

        // Input assembly (triangle list)
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        // Viewport and scissor (dynamic)
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        // Rasterization state (backface culling, depth test)
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);

        // Multisample (disabled for now)
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // Depth stencil (depth test enabled)
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS);

        // Color blend (opaque rendering)
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA);

        let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(std::slice::from_ref(&color_blend_attachment));

        // Dynamic state (viewport, scissor)
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&dynamic_states);

        // Create pipeline
        let create_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization)
            .multisample_state(&multisample)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blend)
            .dynamic_state(&dynamic_state)
            .layout(layout.layout)
            .render_pass(render_pass.handle())
            .subpass(0);

        let pipeline = unsafe {
            context.device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&create_info),
                None,
            ).map_err(|(_, e)| e)?[0]
        };

        Ok(Self { pipeline, layout })
    }
}
```

**Tests**:
- Unit: Pipeline creation doesn't crash
- Integration: Create pipeline → bind → draw triangle

---

### Task 2: Transform Component (0.5 days)

**Goal**: Add Transform component to engine-core ECS for object positioning.

**Implementation**:
```rust
// engine/core/src/components/transform.rs
use glam::{Mat4, Quat, Vec3};

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub fn from_position(position: Vec3) -> Self {
        Self { position, ..Self::new() }
    }

    pub fn model_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    pub fn mvp_matrix(&self, view: &Mat4, projection: &Mat4) -> Mat4 {
        *projection * *view * self.model_matrix()
    }
}
```

**Tests**:
- Unit: Transform creation
- Unit: Model matrix calculation
- Unit: MVP matrix calculation
- Integration: Add Transform to entity → query

---

### Task 3: Camera System (0.5 days)

**Goal**: Create Camera component for view and projection matrices.

**Implementation**:
```rust
// engine/core/src/components/camera.rs
#[derive(Component, Debug, Clone, Copy)]
pub struct Camera {
    pub fov: f32,           // Field of view (radians)
    pub aspect: f32,        // Aspect ratio (width / height)
    pub near: f32,          // Near plane
    pub far: f32,           // Far plane
}

impl Camera {
    pub fn new(fov: f32, aspect: f32) -> Self {
        Self {
            fov,
            aspect,
            near: 0.1,
            far: 1000.0,
        }
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
}

// System: Update camera from input
pub fn camera_controller_system(
    query: Query<(&mut Transform, &CameraController)>,
    input: &Input,
    dt: f32,
) {
    for (transform, controller) in query.iter_mut() {
        // WASD movement
        if input.key_pressed(Key::W) {
            transform.position += transform.forward() * controller.speed * dt;
        }
        // ... rotation from mouse
    }
}
```

**Tests**:
- Unit: Camera creation
- Unit: Projection matrix calculation
- Integration: Camera + Transform → view matrix

---

### Task 4: Depth Buffer (0.5 days)

**Goal**: Create depth buffer for proper Z-ordering.

**Implementation**:
```rust
// engine/renderer/src/depth.rs
pub struct DepthBuffer {
    image: vk::Image,
    view: vk::ImageView,
    allocation: Allocation,
}

impl DepthBuffer {
    pub fn new(
        context: &VulkanContext,
        width: u32,
        height: u32,
    ) -> Result<Self, RendererError> {
        // Create depth image (D32_SFLOAT)
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let image = unsafe { context.device.create_image(&image_info, None)? };

        // Allocate memory
        let requirements = unsafe { context.device.get_image_memory_requirements(image) };
        let allocation = context.allocator.lock()?.allocate(&AllocationCreateDesc {
            name: "depth_buffer",
            requirements,
            location: MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        })?;

        // Bind memory
        unsafe {
            context.device.bind_image_memory(image, allocation.memory(), allocation.offset())?;
        }

        // Create image view
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::D32_SFLOAT)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let view = unsafe { context.device.create_image_view(&view_info, None)? };

        Ok(Self { image, view, allocation })
    }
}
```

**Tests**:
- Integration: Create depth buffer → attach to framebuffer → render

---

### Task 5: MeshRenderer Component (0.5 days)

**Goal**: Create component that references mesh and material assets.

**Implementation**:
```rust
// engine/core/src/components/mesh_renderer.rs
#[derive(Component, Debug, Clone)]
pub struct MeshRenderer {
    pub mesh: AssetHandle<MeshData>,
    pub material: Option<AssetHandle<MaterialData>>, // Phase 1.7+
}

impl MeshRenderer {
    pub fn new(mesh: AssetHandle<MeshData>) -> Self {
        Self { mesh, material: None }
    }
}
```

**Tests**:
- Unit: Component creation
- Integration: Add to entity → query

---

### Task 6: Rendering Integration (1 day)

**Goal**: Integrate mesh rendering into Renderer with ECS query.

**Implementation**:
```rust
// engine/renderer/src/renderer.rs
impl Renderer {
    pub fn render_frame(&mut self, world: &World) -> Result<(), RendererError> {
        // Begin frame
        let (image_index, command_buffer) = self.begin_frame()?;

        // Begin render pass
        self.begin_render_pass(command_buffer, image_index)?;

        // Bind pipeline
        unsafe {
            self.context.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.mesh_pipeline.handle(),
            );
        }

        // Query all renderable entities
        for (transform, mesh_renderer) in world.query::<(&Transform, &MeshRenderer)>().iter() {
            self.draw_mesh(command_buffer, transform, mesh_renderer)?;
        }

        // End render pass
        self.end_render_pass(command_buffer)?;

        // End frame
        self.end_frame(image_index, command_buffer)?;

        Ok(())
    }

    fn draw_mesh(
        &mut self,
        command_buffer: vk::CommandBuffer,
        transform: &Transform,
        mesh_renderer: &MeshRenderer,
    ) -> Result<(), RendererError> {
        // Get or upload GPU mesh
        let gpu_mesh = self.gpu_cache.get_or_upload_mesh(&mesh_renderer.mesh)?;

        // Calculate MVP matrix
        let camera = self.get_camera(); // Get active camera
        let mvp = transform.mvp_matrix(&camera.view_matrix(), &camera.projection_matrix());

        // Push constants (MVP matrix)
        unsafe {
            self.context.device.cmd_push_constants(
                command_buffer,
                self.mesh_pipeline.layout(),
                vk::ShaderStageFlags::VERTEX,
                0,
                bytemuck::bytes_of(&mvp),
            );

            // Bind vertex buffer
            self.context.device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[gpu_mesh.vertex_buffer.handle()],
                &[0],
            );

            // Bind index buffer
            self.context.device.cmd_bind_index_buffer(
                command_buffer,
                gpu_mesh.index_buffer.handle(),
                0,
                vk::IndexType::UINT32,
            );

            // Draw
            self.context.device.cmd_draw_indexed(
                command_buffer,
                gpu_mesh.index_count(),
                1,  // instance count
                0,  // first index
                0,  // vertex offset
                0,  // first instance
            );
        }

        Ok(())
    }
}
```

**Tests**:
- Integration: Load mesh → add to entity → render → capture frame
- E2E: Rotating cube example

---

### Task 7: Examples & Testing (0.5 days)

**Goal**: Create examples and comprehensive tests.

**Examples**:
```rust
// examples/rotating_cube.rs
use engine_core::{World, Transform};
use engine_renderer::Renderer;
use engine_assets::AssetManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup
    let mut world = World::new();
    let mut asset_manager = AssetManager::new()?;
    let mut renderer = Renderer::new("Rotating Cube", 800, 600)?;

    // Load cube mesh
    let cube_mesh = asset_manager.load_sync("assets/cube.obj")?;

    // Spawn entity with mesh
    let cube = world.spawn();
    world.add(cube, Transform::new());
    world.add(cube, MeshRenderer::new(cube_mesh));

    // Main loop
    loop {
        // Rotate cube
        let mut transform = world.get_mut::<Transform>(cube)?;
        transform.rotation *= Quat::from_rotation_y(0.01);

        // Render
        renderer.render_frame(&world)?;
    }
}
```

**Tests**:
- Unit: Pipeline creation
- Unit: Transform calculations
- Integration: Render cube → capture → verify pixels
- Benchmark: Draw 1000 cubes

---

## Integration with Asset System

### Loading Meshes
```rust
// Via asset manager
let mesh = asset_manager.load_sync("cube.obj")?;  // Blocks
let mesh = asset_manager.load_async("level.glb").await?;  // Non-blocking
let mesh = asset_manager.load_streaming("terrain.glb").await?;  // Progressive LOD
```

### GPU Upload
```rust
// Lazy upload (only when needed)
let gpu_mesh = gpu_cache.get_or_upload_mesh(&mesh_handle)?;

// Explicit upload
let gpu_mesh = GpuMesh::from_mesh_data(&context, &mesh_handle.get().unwrap())?;
```

### Memory Management
```rust
// Hard reference (never evicted)
let player_mesh = asset_manager.load_hard("player.glb")?;

// Soft reference (can be evicted by LRU)
let tree_mesh = asset_manager.load_soft("tree.glb")?;

// GPU resources cleaned up when CPU asset evicted
gpu_cache.remove_if_evicted(asset_id);
```

---

## Performance Targets

| Metric | Target | Critical |
|--------|--------|----------|
| Pipeline creation | < 10 ms | < 100 ms |
| GPU upload (1000 vertices) | < 1 ms | < 10 ms |
| Draw call overhead | < 10 µs | < 100 µs |
| Frame time (1000 cubes) | < 16 ms | < 33 ms |
| Transform calculations | < 1 µs | < 10 µs |

---

## Testing Strategy

### Unit Tests
- Pipeline creation
- Transform matrix math
- Camera projection
- Component creation

### Integration Tests
- Load mesh → upload → render → capture
- ECS query → render all entities
- Depth buffer → correct Z-ordering

### Benchmarks
- Draw call overhead
- 1000 cubes @ 60 FPS
- Transform calculations
- MVP matrix multiplications

### E2E Tests
- Rotating cube example
- Multiple meshes with different transforms
- Camera movement

---

## Dependencies

**From Phase 1.7**:
- `engine-assets` crate with MeshData
- `AssetManager` for loading
- `AssetHandle<MeshData>` for references

**New Dependencies**:
- `bytemuck` for push constants (bytes_of)
- `glam` for math (already used)

---

## Files to Create/Modify

### New Files
- `engine/renderer/src/pipeline.rs` (300 lines)
- `engine/renderer/src/depth.rs` (150 lines)
- `engine/renderer/src/gpu_cache.rs` (200 lines)
- `engine/core/src/components/transform.rs` (100 lines)
- `engine/core/src/components/camera.rs` (150 lines)
- `engine/core/src/components/mesh_renderer.rs` (50 lines)
- `examples/rotating_cube.rs` (100 lines)

### Modified Files
- `engine/renderer/src/renderer.rs` (add render_frame, draw_mesh)
- `engine/renderer/src/lib.rs` (export new types)
- `engine/core/src/lib.rs` (export new components)

---

## Success Criteria

1. ✅ Graphics pipeline compiles and doesn't crash
2. ✅ Transform component works in ECS
3. ✅ Camera system calculates correct matrices
4. ✅ Depth buffer prevents Z-fighting
5. ✅ Mesh rendering works with asset system
6. ✅ Rotating cube example runs at 60 FPS
7. ✅ All tests passing
8. ✅ Performance targets met

---

## Next Steps

After Phase 1.8 completion:
- **Phase 1.9**: Frame Capture (screenshots, video recording)
- **Phase 2+**: Advanced rendering (PBR materials, shadows, lighting)
