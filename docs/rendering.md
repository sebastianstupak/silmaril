# Rendering Architecture

> **Vulkan-based rendering system for silmaril**
>
> Low-level Vulkan renderer optimized for AI agent visual feedback loops

---

## Overview

The silmaril uses Vulkan via the Ash crate for:
- **Cross-platform rendering** - Windows, Linux, macOS (via MoltenVK)
- **Low-level control** - Direct GPU access for optimization
- **Headless rendering** - Offscreen rendering for AI agents
- **Visual feedback** - Render → analyze → iterate workflows

## Architecture

### Renderer Modules (Phase 1.6)

```
engine/renderer/src/
├── lib.rs              # Core renderer types and initialization
├── swapchain.rs        # Swapchain management for presentation
├── offscreen.rs        # Offscreen rendering for headless mode
├── window.rs           # Window abstraction (Phase 1.6)
├── surface.rs          # Vulkan surface creation (Phase 1.6)
├── renderpass.rs       # Render pass management (Phase 1.6)
├── command.rs          # Command buffer management (Phase 1.6) ✅
├── framebuffer.rs      # Framebuffer management (Phase 1.6) ✅
└── pipeline/           # Graphics pipelines (Phase 2.X)
```

**Status:** Phase 1.6 in progress (Window, Surface, RenderPass needed)

---

## Vulkan Context

### Initialization

```rust
use engine_renderer::{VulkanContext, ContextConfig};

pub fn init_renderer(window: &Window) -> Result<VulkanContext, RendererError> {
    let config = ContextConfig {
        app_name: "Silmaril",
        enable_validation: cfg!(debug_assertions),
        prefer_discrete_gpu: true,
    };

    VulkanContext::new(window, config)
}
```

### Components

```rust
pub struct VulkanContext {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub command_pool: vk::CommandPool,
}
```

**Implementation:** `engine/renderer/src/lib.rs` (partial)

---

## Window Management (Phase 1.6)

### Window Abstraction

Platform-agnostic window creation:

```rust
pub struct Window {
    inner: PlatformWindow,
    width: u32,
    height: u32,
}

pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub fullscreen: bool,
}

impl Window {
    pub fn new(config: WindowConfig) -> Result<Self, WindowError> {
        let inner = PlatformWindow::create(&config)?;
        Ok(Self {
            inner,
            width: config.width,
            height: config.height,
        })
    }

    pub fn get_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn should_close(&self) -> bool {
        self.inner.should_close()
    }

    pub fn poll_events(&mut self) {
        self.inner.poll_events();
    }
}
```

### Platform Backends

```rust
// Windows (win32)
#[cfg(windows)]
mod windows {
    pub struct Win32Window {
        hwnd: HWND,
        hinstance: HINSTANCE,
    }
}

// Linux (X11/Wayland)
#[cfg(target_os = "linux")]
mod linux {
    pub struct X11Window {
        display: *mut x11::Display,
        window: x11::Window,
    }
}

// macOS (Cocoa)
#[cfg(target_os = "macos")]
mod macos {
    pub struct CocoaWindow {
        ns_window: *mut objc::runtime::Object,
    }
}
```

**Status:** ⚪ Not implemented (Task 1.6.1)

---

## Surface Creation (Phase 1.6)

### Vulkan Surface

Create platform-specific VkSurfaceKHR:

```rust
use ash::extensions::khr::Surface as SurfaceLoader;

pub struct Surface {
    loader: SurfaceLoader,
    surface: vk::SurfaceKHR,
}

impl Surface {
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &Window,
    ) -> Result<Self, SurfaceError> {
        let loader = SurfaceLoader::new(entry, instance);

        #[cfg(windows)]
        let surface = create_win32_surface(entry, instance, window)?;

        #[cfg(target_os = "linux")]
        let surface = create_x11_surface(entry, instance, window)?;

        #[cfg(target_os = "macos")]
        let surface = create_metal_surface(entry, instance, window)?;

        Ok(Self { loader, surface })
    }

    pub fn get_capabilities(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::SurfaceCapabilitiesKHR, SurfaceError> {
        unsafe {
            self.loader
                .get_physical_device_surface_capabilities(physical_device, self.surface)
                .map_err(|e| SurfaceError::QueryFailed(e.to_string()))
        }
    }
}
```

### Platform-Specific Surface Creation

```rust
#[cfg(windows)]
fn create_win32_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    window: &Window,
) -> Result<vk::SurfaceKHR, SurfaceError> {
    use ash::extensions::khr::Win32Surface;

    let win32_loader = Win32Surface::new(entry, instance);
    let create_info = vk::Win32SurfaceCreateInfoKHR::builder()
        .hinstance(window.get_hinstance())
        .hwnd(window.get_hwnd());

    unsafe {
        win32_loader
            .create_win32_surface(&create_info, None)
            .map_err(|e| SurfaceError::CreationFailed(e.to_string()))
    }
}
```

**Status:** ⚪ Not implemented (Task 1.6.2)

---

## Render Pass (Phase 1.6)

### Render Pass Definition

Define attachment formats and subpass dependencies:

```rust
pub struct RenderPass {
    render_pass: vk::RenderPass,
    device: ash::Device,
}

pub struct RenderPassConfig {
    pub color_format: vk::Format,
    pub depth_format: Option<vk::Format>,
    pub sample_count: vk::SampleCountFlags,
}

impl RenderPass {
    pub fn new(
        device: &ash::Device,
        config: RenderPassConfig,
    ) -> Result<Self, RenderPassError> {
        let mut attachments = vec![
            // Color attachment
            vk::AttachmentDescription::builder()
                .format(config.color_format)
                .samples(config.sample_count)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .build(),
        ];

        // Optional depth attachment
        if let Some(depth_format) = config.depth_format {
            attachments.push(
                vk::AttachmentDescription::builder()
                    .format(depth_format)
                    .samples(config.sample_count)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::DONT_CARE)
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                    .build(),
            );
        }

        let color_refs = [vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build()];

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_refs);

        let create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(std::slice::from_ref(&subpass));

        let render_pass = unsafe {
            device
                .create_render_pass(&create_info, None)
                .map_err(|e| RenderPassError::CreationFailed(e.to_string()))?
        };

        Ok(Self {
            render_pass,
            device: device.clone(),
        })
    }

    pub fn handle(&self) -> vk::RenderPass {
        self.render_pass
    }
}
```

**Status:** ⚪ Not implemented (Task 1.6.3)

---

## Command Buffers (Phase 1.6)

### Command Pool

Allocate and manage command buffers:

```rust
pub struct CommandPool {
    pool: vk::CommandPool,
    device: ash::Device,
}

impl CommandPool {
    pub fn new(
        device: &ash::Device,
        queue_family_index: u32,
    ) -> Result<Self, CommandError> {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let pool = unsafe {
            device
                .create_command_pool(&create_info, None)
                .map_err(|e| CommandError::PoolCreationFailed(e.to_string()))?
        };

        Ok(Self {
            pool,
            device: device.clone(),
        })
    }

    pub fn allocate_buffers(&self, count: u32) -> Result<Vec<vk::CommandBuffer>, CommandError> {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);

        unsafe {
            self.device
                .allocate_command_buffers(&alloc_info)
                .map_err(|e| CommandError::AllocationFailed(e.to_string()))
        }
    }
}
```

### Command Buffer Recording

```rust
pub fn record_render_commands(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    extent: vk::Extent2D,
) -> Result<(), CommandError> {
    let begin_info = vk::CommandBufferBeginInfo::builder()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    unsafe {
        device
            .begin_command_buffer(command_buffer, &begin_info)
            .map_err(|e| CommandError::RecordingFailed(e.to_string()))?;

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let render_pass_begin = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            })
            .clear_values(&clear_values);

        device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_begin,
            vk::SubpassContents::INLINE,
        );

        // Draw commands here

        device.cmd_end_render_pass(command_buffer);

        device
            .end_command_buffer(command_buffer)
            .map_err(|e| CommandError::RecordingFailed(e.to_string()))?;
    }

    Ok(())
}
```

**Status:** ✅ Complete (Task 1.6.4)

---

## Framebuffers (Phase 1.6)

### Framebuffer Creation

Attach images to render passes:

```rust
pub struct Framebuffer {
    framebuffer: vk::Framebuffer,
    device: ash::Device,
}

impl Framebuffer {
    pub fn new(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        attachments: &[vk::ImageView],
        width: u32,
        height: u32,
    ) -> Result<Self, FramebufferError> {
        let create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(attachments)
            .width(width)
            .height(height)
            .layers(1);

        let framebuffer = unsafe {
            device
                .create_framebuffer(&create_info, None)
                .map_err(|e| FramebufferError::CreationFailed(e.to_string()))?
        };

        Ok(Self {
            framebuffer,
            device: device.clone(),
        })
    }

    pub fn handle(&self) -> vk::Framebuffer {
        self.framebuffer
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_framebuffer(self.framebuffer, None);
        }
    }
}
```

**Status:** ✅ Complete (Task 1.6.5)

---

## Swapchain

### Swapchain Management

Present images to the screen:

```rust
pub struct Swapchain {
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    format: vk::Format,
    extent: vk::Extent2D,
    loader: ash::extensions::khr::Swapchain,
}

impl Swapchain {
    pub fn new(
        device: &ash::Device,
        surface: &Surface,
        width: u32,
        height: u32,
    ) -> Result<Self, SwapchainError> {
        let capabilities = surface.get_capabilities(physical_device)?;
        let format = choose_surface_format(&surface_formats);
        let present_mode = choose_present_mode(&present_modes);
        let extent = choose_extent(&capabilities, width, height);

        let image_count = (capabilities.min_image_count + 1)
            .min(capabilities.max_image_count);

        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.handle())
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        let loader = ash::extensions::khr::Swapchain::new(instance, device);
        let swapchain = unsafe {
            loader
                .create_swapchain(&create_info, None)
                .map_err(|e| SwapchainError::CreationFailed(e.to_string()))?
        };

        let images = unsafe { loader.get_swapchain_images(swapchain)? };
        let image_views = create_image_views(device, &images, format.format)?;

        Ok(Self {
            swapchain,
            images,
            image_views,
            format: format.format,
            extent,
            loader,
        })
    }

    pub fn acquire_next_image(&self, semaphore: vk::Semaphore)
        -> Result<(u32, bool), SwapchainError>
    {
        unsafe {
            self.loader
                .acquire_next_image(
                    self.swapchain,
                    u64::MAX,
                    semaphore,
                    vk::Fence::null(),
                )
                .map_err(|e| SwapchainError::AcquireFailed(e.to_string()))
        }
    }
}
```

**Implementation:** `engine/renderer/src/swapchain.rs` (partial)

---

## Offscreen Rendering

### Headless Mode

Render without a window for AI agents:

```rust
pub struct OffscreenRenderer {
    images: Vec<vk::Image>,
    memory: Vec<vk::DeviceMemory>,
    image_views: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    width: u32,
    height: u32,
}

impl OffscreenRenderer {
    pub fn new(
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        width: u32,
        height: u32,
        format: vk::Format,
    ) -> Result<Self, RendererError> {
        // Create images
        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D { width, height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC);

        let image = unsafe {
            device.create_image(&image_create_info, None)?
        };

        // Allocate and bind memory
        let mem_requirements = unsafe {
            device.get_image_memory_requirements(image)
        };

        let memory = allocate_device_memory(device, physical_device, mem_requirements)?;

        unsafe {
            device.bind_image_memory(image, memory, 0)?;
        }

        // Create image view
        let view_create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let image_view = unsafe {
            device.create_image_view(&view_create_info, None)?
        };

        Ok(Self {
            images: vec![image],
            memory: vec![memory],
            image_views: vec![image_view],
            framebuffers: vec![],
            width,
            height,
        })
    }

    pub fn read_pixels(&self, device: &ash::Device) -> Result<Vec<u8>, RendererError> {
        // Copy image to CPU-visible buffer and read
        // ...
    }
}
```

**Implementation:** `engine/renderer/src/offscreen.rs` (partial)

---

## Graphics Pipeline (Phase 2.X)

### Pipeline Creation

```rust
pub struct GraphicsPipeline {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
}

pub struct PipelineConfig {
    pub vertex_shader: Vec<u32>,   // SPIR-V
    pub fragment_shader: Vec<u32>, // SPIR-V
    pub vertex_input: VertexInputDescription,
    pub render_pass: vk::RenderPass,
}

impl GraphicsPipeline {
    pub fn new(
        device: &ash::Device,
        config: PipelineConfig,
    ) -> Result<Self, PipelineError> {
        // Create shader modules
        let vert_module = create_shader_module(device, &config.vertex_shader)?;
        let frag_module = create_shader_module(device, &config.fragment_shader)?;

        // Pipeline stages
        let stages = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_module)
                .name(c"main")
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_module)
                .name(c"main")
                .build(),
        ];

        // Vertex input state
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&config.vertex_input.bindings)
            .vertex_attribute_descriptions(&config.vertex_input.attributes);

        // ... (rasterization, multisample, depth/stencil, color blend states)

        let create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&stages)
            .vertex_input_state(&vertex_input_state)
            // ... other states
            .render_pass(config.render_pass);

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[*create_info], None)
                .map_err(|(_, e)| PipelineError::CreationFailed(e.to_string()))?[0]
        };

        Ok(Self { pipeline, layout })
    }
}
```

**Status:** ⚪ Not implemented (Phase 2.X)

---

## Performance Targets

| Metric | Target | Critical |
|--------|--------|----------|
| Frame time (1080p) | < 16.67ms (60 FPS) | < 33ms (30 FPS) |
| Frame time (4K) | < 33ms (30 FPS) | < 50ms (20 FPS) |
| GPU memory usage | < 2GB | < 4GB |
| Draw call overhead | < 50μs per call | < 100μs |
| Offscreen render + read | < 20ms | < 50ms |

---

## Testing

### Integration Tests

```rust
#[test]
fn test_command_buffer_lifecycle() {
    let pool = CommandPool::new(&device, queue_family_index).unwrap();
    let buffers = pool.allocate_buffers(1).unwrap();
    assert_eq!(buffers.len(), 1);
}

#[test]
fn test_framebuffer_creation() {
    let framebuffer = Framebuffer::new(
        &device,
        render_pass,
        &[image_view],
        800,
        600,
    ).unwrap();
    assert_ne!(framebuffer.handle(), vk::Framebuffer::null());
}
```

**Implementation:** `engine/renderer/tests/`

---

## Benchmarks

```rust
#[bench]
fn bench_command_buffer_allocation(b: &mut Bencher) {
    let pool = CommandPool::new(&device, 0).unwrap();
    b.iter(|| {
        pool.allocate_buffers(1).unwrap()
    });
}
```

**Implementation:** `engine/renderer/benches/vulkan_context_bench.rs`

---

## References

- **Implementation:** `engine/renderer/src/`
- **Tests:** `engine/renderer/tests/`
- **Benchmarks:** `engine/renderer/benches/`
- **Vulkan Spec:** https://registry.khronos.org/vulkan/specs/1.3/html/

**Related Documentation:**
- [Platform Abstraction](platform-abstraction.md)
- [Error Handling](error-handling.md)
- [Performance Targets](performance-targets.md)
