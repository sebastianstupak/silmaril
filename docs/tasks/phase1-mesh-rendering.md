# Phase 1.7: Mesh Rendering

**Status:** ⚪ Not Started
**Estimated Time:** 5-6 days
**Priority:** High (enables actual game visuals)

---

## 🎯 **Objective**

Implement mesh rendering pipeline with vertex/index buffers, shaders (GLSL → SPIR-V), graphics pipeline, and basic camera. Render a cube or triangle.

**Goal:** Display a rotating 3D mesh on screen.

---

## 📋 **Detailed Tasks**

### **1. Vertex Data Structures** (Day 1)

**File:** `engine/renderer/src/vertex.rs`

```rust
use glam::{Vec2, Vec3};

/// Vertex format
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub color: Vec3,
}

impl Vertex {
    /// Get vertex binding description
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    /// Get attribute descriptions
    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 4] {
        [
            // Position
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0)
                .build(),
            // Normal
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(std::mem::size_of::<Vec3>() as u32)
                .build(),
            // UV
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset((std::mem::size_of::<Vec3>() * 2) as u32)
                .build(),
            // Color
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(3)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset((std::mem::size_of::<Vec3>() * 2 + std::mem::size_of::<Vec2>()) as u32)
                .build(),
        ]
    }
}

/// Mesh data
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    /// Create a cube mesh
    pub fn cube() -> Self {
        let vertices = vec![
            // Front face
            Vertex { position: Vec3::new(-0.5, -0.5, 0.5), normal: Vec3::Z, uv: Vec2::new(0.0, 0.0), color: Vec3::ONE },
            Vertex { position: Vec3::new(0.5, -0.5, 0.5), normal: Vec3::Z, uv: Vec2::new(1.0, 0.0), color: Vec3::ONE },
            Vertex { position: Vec3::new(0.5, 0.5, 0.5), normal: Vec3::Z, uv: Vec2::new(1.0, 1.0), color: Vec3::ONE },
            Vertex { position: Vec3::new(-0.5, 0.5, 0.5), normal: Vec3::Z, uv: Vec2::new(0.0, 1.0), color: Vec3::ONE },
            // Back face
            Vertex { position: Vec3::new(0.5, -0.5, -0.5), normal: -Vec3::Z, uv: Vec2::new(0.0, 0.0), color: Vec3::ONE },
            Vertex { position: Vec3::new(-0.5, -0.5, -0.5), normal: -Vec3::Z, uv: Vec2::new(1.0, 0.0), color: Vec3::ONE },
            Vertex { position: Vec3::new(-0.5, 0.5, -0.5), normal: -Vec3::Z, uv: Vec2::new(1.0, 1.0), color: Vec3::ONE },
            Vertex { position: Vec3::new(0.5, 0.5, -0.5), normal: -Vec3::Z, uv: Vec2::new(0.0, 1.0), color: Vec3::ONE },
            // Top, bottom, left, right faces...
            // (Full cube vertices omitted for brevity)
        ];

        let indices = vec![
            // Front
            0, 1, 2, 2, 3, 0,
            // Back
            4, 5, 6, 6, 7, 4,
            // (All indices omitted for brevity)
        ];

        Self { vertices, indices }
    }

    /// Create a triangle mesh
    pub fn triangle() -> Self {
        let vertices = vec![
            Vertex {
                position: Vec3::new(0.0, -0.5, 0.0),
                normal: Vec3::Z,
                uv: Vec2::new(0.5, 0.0),
                color: Vec3::new(1.0, 0.0, 0.0),
            },
            Vertex {
                position: Vec3::new(0.5, 0.5, 0.0),
                normal: Vec3::Z,
                uv: Vec2::new(1.0, 1.0),
                color: Vec3::new(0.0, 1.0, 0.0),
            },
            Vertex {
                position: Vec3::new(-0.5, 0.5, 0.0),
                normal: Vec3::Z,
                uv: Vec2::new(0.0, 1.0),
                color: Vec3::new(0.0, 0.0, 1.0),
            },
        ];

        let indices = vec![0, 1, 2];

        Self { vertices, indices }
    }
}
```

---

### **2. Buffers (Vertex/Index)** (Day 1-2)

**File:** `engine/renderer/src/vulkan/buffer.rs`

```rust
use gpu_allocator::MemoryLocation;

/// GPU buffer wrapper
pub struct Buffer {
    buffer: vk::Buffer,
    allocation: Allocation,
    size: u64,
}

impl Buffer {
    /// Create vertex buffer
    pub fn create_vertex_buffer(
        device: &ash::Device,
        allocator: &mut VulkanAllocator,
        vertices: &[Vertex],
    ) -> Result<Self, RendererError> {
        let size = (std::mem::size_of::<Vertex>() * vertices.len()) as u64;

        // Create staging buffer (CPU accessible)
        let (staging_buffer, staging_allocation) = allocator.allocate_buffer(
            device,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
        )?;

        // Copy data to staging buffer
        unsafe {
            let mapped = staging_allocation.mapped_ptr().unwrap().as_ptr() as *mut Vertex;
            std::ptr::copy_nonoverlapping(vertices.as_ptr(), mapped, vertices.len());
        }

        // Create device-local buffer
        let (buffer, allocation) = allocator.allocate_buffer(
            device,
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::GpuOnly,
        )?;

        // Copy staging → device buffer (requires command buffer)
        // TODO: Implement copy_buffer function

        // Cleanup staging
        allocator.free_buffer(device, staging_buffer, staging_allocation);

        Ok(Self {
            buffer,
            allocation,
            size,
        })
    }

    /// Create index buffer
    pub fn create_index_buffer(
        device: &ash::Device,
        allocator: &mut VulkanAllocator,
        indices: &[u32],
    ) -> Result<Self, RendererError> {
        let size = (std::mem::size_of::<u32>() * indices.len()) as u64;

        let (staging_buffer, staging_allocation) = allocator.allocate_buffer(
            device,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
        )?;

        unsafe {
            let mapped = staging_allocation.mapped_ptr().unwrap().as_ptr() as *mut u32;
            std::ptr::copy_nonoverlapping(indices.as_ptr(), mapped, indices.len());
        }

        let (buffer, allocation) = allocator.allocate_buffer(
            device,
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            MemoryLocation::GpuOnly,
        )?;

        // Copy staging → device buffer

        allocator.free_buffer(device, staging_buffer, staging_allocation);

        Ok(Self {
            buffer,
            allocation,
            size,
        })
    }

    pub fn handle(&self) -> vk::Buffer {
        self.buffer
    }
}
```

---

### **3. Shaders** (Day 2-3)

**File:** `engine/renderer/shaders/basic.vert` (GLSL)

```glsl
#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;
layout(location = 3) in vec3 inColor;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec3 fragNormal;
layout(location = 2) out vec2 fragUV;

layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

void main() {
    gl_Position = ubo.proj * ubo.view * ubo.model * vec4(inPosition, 1.0);
    fragColor = inColor;
    fragNormal = mat3(transpose(inverse(ubo.model))) * inNormal;
    fragUV = inUV;
}
```

**File:** `engine/renderer/shaders/basic.frag` (GLSL)

```glsl
#version 450

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragUV;

layout(location = 0) out vec4 outColor;

void main() {
    // Simple lighting
    vec3 lightDir = normalize(vec3(1.0, 1.0, 1.0));
    float diffuse = max(dot(normalize(fragNormal), lightDir), 0.0);

    vec3 color = fragColor * (0.3 + 0.7 * diffuse);
    outColor = vec4(color, 1.0);
}
```

**Shader Compilation Build Script:** `engine/renderer/build.rs`

```rust
use std::process::Command;

fn main() {
    // Compile shaders to SPIR-V
    let shaders = ["basic.vert", "basic.frag"];

    for shader in &shaders {
        println!("cargo:rerun-if-changed=shaders/{}", shader);

        let output = Command::new("glslc")
            .args(&[
                &format!("shaders/{}", shader),
                "-o",
                &format!("shaders/{}.spv", shader),
            ])
            .output()
            .expect("Failed to compile shader");

        if !output.status.success() {
            panic!(
                "Shader compilation failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
```

**File:** `engine/renderer/src/vulkan/shader.rs`

```rust
/// Shader module wrapper
pub struct ShaderModule {
    module: vk::ShaderModule,
}

impl ShaderModule {
    /// Create shader module from SPIR-V bytes
    pub fn new(device: &ash::Device, code: &[u8]) -> Result<Self, RendererError> {
        let code_aligned = Self::align_spirv(code);

        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(&code_aligned);

        let module = unsafe {
            device
                .create_shader_module(&create_info, None)
                .map_err(|e| RendererError::ShaderCreationFailed {
                    details: e.to_string(),
                })?
        };

        Ok(Self { module })
    }

    /// Align SPIR-V bytes to u32
    fn align_spirv(code: &[u8]) -> Vec<u32> {
        code.chunks(4)
            .map(|chunk| {
                let mut bytes = [0u8; 4];
                bytes[..chunk.len()].copy_from_slice(chunk);
                u32::from_le_bytes(bytes)
            })
            .collect()
    }

    pub fn handle(&self) -> vk::ShaderModule {
        self.module
    }
}

/// Load shader from file
pub fn load_shader(device: &ash::Device, path: &str) -> Result<ShaderModule, RendererError> {
    let code = std::fs::read(path).map_err(|e| RendererError::ShaderLoadFailed {
        details: format!("Failed to read {}: {}", path, e),
    })?;

    ShaderModule::new(device, &code)
}
```

---

### **4. Graphics Pipeline** (Day 3-4)

**File:** `engine/renderer/src/vulkan/pipeline.rs`

```rust
/// Graphics pipeline wrapper
pub struct GraphicsPipeline {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

impl GraphicsPipeline {
    /// Create graphics pipeline
    pub fn new(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        extent: vk::Extent2D,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<Self, RendererError> {
        // Load shaders
        let vert_shader = load_shader(device, "shaders/basic.vert.spv")?;
        let frag_shader = load_shader(device, "shaders/basic.frag.spv")?;

        // Shader stages
        let entry_point = CString::new("main").unwrap();

        let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_shader.handle())
            .name(&entry_point)
            .build();

        let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_shader.handle())
            .name(&entry_point)
            .build();

        let stages = [vert_stage, frag_stage];

        // Vertex input
        let binding_description = Vertex::binding_description();
        let attribute_descriptions = Vertex::attribute_descriptions();

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&[binding_description])
            .vertex_attribute_descriptions(&attribute_descriptions);

        // Input assembly
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        // Viewport
        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .build();

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(extent)
            .build();

        let viewports = [viewport];
        let scissors = [scissor];

        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        // Rasterization
        let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        // Multisampling
        let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // Color blending
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false)
            .build();

        let color_blend_attachments = [color_blend_attachment];

        let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);

        // Pipeline layout
        let set_layouts = [descriptor_set_layout];

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&set_layouts);

        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| RendererError::PipelineCreationFailed {
                    details: e.to_string(),
                })?
        };

        // Create pipeline
        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0)
            .build();

        let pipelines = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|e| RendererError::PipelineCreationFailed {
                    details: e.1.to_string(),
                })?
        };

        tracing::info!("Graphics pipeline created");

        Ok(Self {
            pipeline: pipelines[0],
            pipeline_layout,
        })
    }

    pub fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }

    pub fn layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }
}
```

---

### **5. Uniform Buffers & Descriptors** (Day 4-5)

**File:** `engine/renderer/src/uniform.rs`

```rust
use glam::{Mat4, Vec3};

/// Uniform buffer object
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UniformBufferObject {
    pub model: Mat4,
    pub view: Mat4,
    pub proj: Mat4,
}

impl UniformBufferObject {
    pub fn new() -> Self {
        Self {
            model: Mat4::IDENTITY,
            view: Mat4::look_at_rh(
                Vec3::new(2.0, 2.0, 2.0),
                Vec3::ZERO,
                Vec3::Y,
            ),
            proj: Mat4::perspective_rh(
                45.0_f32.to_radians(),
                16.0 / 9.0,
                0.1,
                100.0,
            ),
        }
    }
}
```

---

### **6. Camera** (Day 5-6)

**File:** `engine/core/src/camera.rs`

```rust
use glam::{Mat4, Vec3};

/// Camera component
#[derive(Debug, Clone, Component)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            position: Vec3::new(2.0, 2.0, 2.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: 45.0,
            aspect,
            near: 0.1,
            far: 100.0,
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), self.aspect, self.near, self.far)
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Mesh rendered on screen
- [ ] Shaders compiled to SPIR-V
- [ ] Vertex/index buffers working
- [ ] Graphics pipeline created
- [ ] Uniform buffers updated per frame
- [ ] Camera controls work
- [ ] Lighting visible
- [ ] 60 FPS maintained

---

**Dependencies:** [phase1-basic-rendering.md](phase1-basic-rendering.md)
**Next:** [phase1-frame-capture.md](phase1-frame-capture.md)
