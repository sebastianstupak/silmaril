# Phase 1.6: Basic Rendering Pipeline

**Status:** ⚪ Not Started
**Estimated Time:** 4-5 days
**Priority:** High (enables visual feedback loop)

---

## 🎯 **Objective**

Implement basic Vulkan rendering pipeline with render pass, framebuffers, command buffers, and synchronization. Render a simple clear color to prove the pipeline works.

**Goal:** Display a window with a clear color (no geometry yet).

---

## 📋 **Detailed Tasks**

### **1. Render Pass** (Day 1)

**File:** `engine/renderer/src/vulkan/render_pass.rs`

```rust
/// Render pass wrapper
pub struct RenderPass {
    render_pass: vk::RenderPass,
}

impl RenderPass {
    /// Create render pass
    pub fn new(device: &ash::Device, swapchain_format: vk::Format) -> Result<Self, RendererError> {
        // Color attachment (swapchain image)
        let color_attachment = vk::AttachmentDescription::builder()
            .format(swapchain_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();

        let attachments = [color_attachment];

        // Color attachment reference
        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let color_attachments = [color_attachment_ref];

        // Subpass
        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments)
            .build();

        let subpasses = [subpass];

        // Subpass dependency (synchronization)
        let dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build();

        let dependencies = [dependency];

        // Create render pass
        let create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        let render_pass = unsafe {
            device
                .create_render_pass(&create_info, None)
                .map_err(|e| RendererError::RenderPassCreationFailed {
                    details: e.to_string(),
                })?
        };

        tracing::info!("Render pass created");

        Ok(Self { render_pass })
    }

    pub fn handle(&self) -> vk::RenderPass {
        self.render_pass
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        // Cleanup handled by renderer
        tracing::debug!("Render pass dropped");
    }
}
```

---

### **2. Framebuffers** (Day 1-2)

**File:** `engine/renderer/src/vulkan/framebuffer.rs`

```rust
/// Framebuffer wrapper
pub struct Framebuffer {
    framebuffer: vk::Framebuffer,
}

impl Framebuffer {
    /// Create framebuffer
    pub fn new(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        image_view: vk::ImageView,
        extent: vk::Extent2D,
    ) -> Result<Self, RendererError> {
        let attachments = [image_view];

        let create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        let framebuffer = unsafe {
            device
                .create_framebuffer(&create_info, None)
                .map_err(|e| RendererError::FramebufferCreationFailed {
                    details: e.to_string(),
                })?
        };

        Ok(Self { framebuffer })
    }

    pub fn handle(&self) -> vk::Framebuffer {
        self.framebuffer
    }
}

/// Create framebuffers for all swapchain images
pub fn create_framebuffers(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    swapchain: &Swapchain,
) -> Result<Vec<Framebuffer>, RendererError> {
    swapchain
        .image_views()
        .iter()
        .map(|&image_view| {
            Framebuffer::new(device, render_pass, image_view, swapchain.extent())
        })
        .collect()
}
```

---

### **3. Command Pools and Buffers** (Day 2)

**File:** `engine/renderer/src/vulkan/command.rs`

```rust
/// Command pool wrapper
pub struct CommandPool {
    pool: vk::CommandPool,
}

impl CommandPool {
    /// Create command pool
    pub fn new(device: &ash::Device, queue_family: u32) -> Result<Self, RendererError> {
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let pool = unsafe {
            device
                .create_command_pool(&create_info, None)
                .map_err(|e| RendererError::CommandPoolCreationFailed {
                    details: e.to_string(),
                })?
        };

        tracing::info!("Command pool created");

        Ok(Self { pool })
    }

    /// Allocate command buffers
    pub fn allocate_command_buffers(
        &self,
        device: &ash::Device,
        count: u32,
    ) -> Result<Vec<vk::CommandBuffer>, RendererError> {
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);

        unsafe {
            device
                .allocate_command_buffers(&allocate_info)
                .map_err(|e| RendererError::CommandBufferAllocationFailed {
                    details: e.to_string(),
                })
        }
    }

    pub fn handle(&self) -> vk::CommandPool {
        self.pool
    }
}

/// Command buffer utilities
pub struct CommandBuffer {
    buffer: vk::CommandBuffer,
}

impl CommandBuffer {
    pub fn new(buffer: vk::CommandBuffer) -> Self {
        Self { buffer }
    }

    /// Begin recording
    pub fn begin(&self, device: &ash::Device) -> Result<(), RendererError> {
        let begin_info = vk::CommandBufferBeginInfo::builder();

        unsafe {
            device
                .begin_command_buffer(self.buffer, &begin_info)
                .map_err(|e| RendererError::CommandBufferRecordingFailed {
                    details: e.to_string(),
                })?;
        }

        Ok(())
    }

    /// End recording
    pub fn end(&self, device: &ash::Device) -> Result<(), RendererError> {
        unsafe {
            device
                .end_command_buffer(self.buffer)
                .map_err(|e| RendererError::CommandBufferRecordingFailed {
                    details: e.to_string(),
                })?;
        }

        Ok(())
    }

    /// Begin render pass
    pub fn begin_render_pass(
        &self,
        device: &ash::Device,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        extent: vk::Extent2D,
        clear_color: [f32; 4],
    ) {
        let clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: clear_color,
            },
        };

        let clear_values = [clear_value];

        let render_pass_info = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass)
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            })
            .clear_values(&clear_values);

        unsafe {
            device.cmd_begin_render_pass(
                self.buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );
        }
    }

    /// End render pass
    pub fn end_render_pass(&self, device: &ash::Device) {
        unsafe {
            device.cmd_end_render_pass(self.buffer);
        }
    }

    pub fn handle(&self) -> vk::CommandBuffer {
        self.buffer
    }
}
```

---

### **4. Synchronization** (Day 3)

**File:** `engine/renderer/src/vulkan/sync.rs`

```rust
/// Frame synchronization objects
pub struct FrameSync {
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
}

impl FrameSync {
    /// Create synchronization objects
    pub fn new(device: &ash::Device) -> Result<Self, RendererError> {
        let semaphore_info = vk::SemaphoreCreateInfo::builder();

        let fence_info = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED); // Start signaled

        let image_available_semaphore = unsafe {
            device
                .create_semaphore(&semaphore_info, None)
                .map_err(|e| RendererError::SyncObjectCreationFailed {
                    details: e.to_string(),
                })?
        };

        let render_finished_semaphore = unsafe {
            device
                .create_semaphore(&semaphore_info, None)
                .map_err(|e| RendererError::SyncObjectCreationFailed {
                    details: e.to_string(),
                })?
        };

        let in_flight_fence = unsafe {
            device
                .create_fence(&fence_info, None)
                .map_err(|e| RendererError::SyncObjectCreationFailed {
                    details: e.to_string(),
                })?
        };

        Ok(Self {
            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence,
        })
    }

    /// Wait for fence
    pub fn wait(&self, device: &ash::Device) -> Result<(), RendererError> {
        unsafe {
            device
                .wait_for_fences(&[self.in_flight_fence], true, u64::MAX)
                .map_err(|e| RendererError::SyncWaitFailed {
                    details: e.to_string(),
                })?;
        }
        Ok(())
    }

    /// Reset fence
    pub fn reset(&self, device: &ash::Device) -> Result<(), RendererError> {
        unsafe {
            device
                .reset_fences(&[self.in_flight_fence])
                .map_err(|e| RendererError::SyncResetFailed {
                    details: e.to_string(),
                })?;
        }
        Ok(())
    }

    pub fn image_available_semaphore(&self) -> vk::Semaphore {
        self.image_available_semaphore
    }

    pub fn render_finished_semaphore(&self) -> vk::Semaphore {
        self.render_finished_semaphore
    }

    pub fn in_flight_fence(&self) -> vk::Fence {
        self.in_flight_fence
    }
}

/// Create sync objects for all frames in flight
pub fn create_frame_sync_objects(
    device: &ash::Device,
    frames_in_flight: u32,
) -> Result<Vec<FrameSync>, RendererError> {
    (0..frames_in_flight)
        .map(|_| FrameSync::new(device))
        .collect()
}
```

---

### **5. Main Render Loop** (Day 4-5)

**File:** `engine/renderer/src/renderer.rs`

```rust
/// Main renderer
pub struct Renderer {
    _instance: VulkanInstance,
    surface: vk::SurfaceKHR,
    _surface_loader: ash::extensions::khr::Surface,
    device: VulkanDevice,
    swapchain: Swapchain,
    render_pass: RenderPass,
    framebuffers: Vec<Framebuffer>,
    command_pool: CommandPool,
    command_buffers: Vec<CommandBuffer>,
    sync_objects: Vec<FrameSync>,
    current_frame: usize,
}

const MAX_FRAMES_IN_FLIGHT: u32 = 2;

impl Renderer {
    /// Create renderer
    pub fn new(window: &dyn WindowBackend) -> Result<Self, RendererError> {
        // Create instance
        let required_extensions = window.required_vulkan_extensions();
        let instance = VulkanInstance::new("Agent Game Engine", cfg!(debug_assertions), &required_extensions)?;

        // Create surface
        let surface_loader = ash::extensions::khr::Surface::new(instance.entry(), instance.instance());
        let surface = window.create_vulkan_surface(instance.entry(), instance.instance())?;

        // Select physical device
        let physical_device_info = PhysicalDeviceInfo::select(
            instance.instance(),
            surface,
            &surface_loader,
            &DeviceRequirements::default(),
        )?;

        // Create logical device
        let device = VulkanDevice::new(instance.instance(), &physical_device_info)?;

        // Create swapchain
        let (width, height) = window.size();
        let swapchain = Swapchain::new(
            instance.instance(),
            device.device(),
            physical_device_info.device,
            surface,
            &surface_loader,
            width,
            height,
        )?;

        // Create render pass
        let render_pass = RenderPass::new(device.device(), swapchain.format())?;

        // Create framebuffers
        let framebuffers = create_framebuffers(device.device(), render_pass.handle(), &swapchain)?;

        // Create command pool
        let command_pool = CommandPool::new(device.device(), device.graphics_queue_family())?;

        // Allocate command buffers
        let command_buffer_handles =
            command_pool.allocate_command_buffers(device.device(), MAX_FRAMES_IN_FLIGHT)?;
        let command_buffers = command_buffer_handles
            .into_iter()
            .map(CommandBuffer::new)
            .collect();

        // Create sync objects
        let sync_objects = create_frame_sync_objects(device.device(), MAX_FRAMES_IN_FLIGHT)?;

        tracing::info!("Renderer created successfully");

        Ok(Self {
            _instance: instance,
            surface,
            _surface_loader: surface_loader,
            device,
            swapchain,
            render_pass,
            framebuffers,
            command_pool,
            command_buffers,
            sync_objects,
            current_frame: 0,
        })
    }

    /// Render a frame
    pub fn render_frame(&mut self, clear_color: [f32; 4]) -> Result<(), RendererError> {
        let sync = &self.sync_objects[self.current_frame];

        // Wait for previous frame
        sync.wait(self.device.device())?;

        // Acquire next image
        let (image_index, _is_suboptimal) = unsafe {
            self.swapchain
                .loader
                .acquire_next_image(
                    self.swapchain.swapchain,
                    u64::MAX,
                    sync.image_available_semaphore(),
                    vk::Fence::null(),
                )
                .map_err(|e| RendererError::ImageAcquisitionFailed {
                    details: e.to_string(),
                })?
        };

        sync.reset(self.device.device())?;

        // Record command buffer
        let command_buffer = &self.command_buffers[self.current_frame];
        self.record_command_buffer(command_buffer, image_index as usize, clear_color)?;

        // Submit command buffer
        let wait_semaphores = [sync.image_available_semaphore()];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [sync.render_finished_semaphore()];
        let command_buffers = [command_buffer.handle()];

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            self.device
                .device()
                .queue_submit(
                    self.device.graphics_queue(),
                    &[submit_info.build()],
                    sync.in_flight_fence(),
                )
                .map_err(|e| RendererError::QueueSubmitFailed {
                    details: e.to_string(),
                })?;
        }

        // Present
        let swapchains = [self.swapchain.swapchain];
        let image_indices = [image_index];

        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            self.swapchain
                .loader
                .queue_present(self.device.present_queue(), &present_info)
                .map_err(|e| RendererError::PresentFailed {
                    details: e.to_string(),
                })?;
        }

        // Advance frame
        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT as usize;

        Ok(())
    }

    /// Record command buffer
    fn record_command_buffer(
        &self,
        command_buffer: &CommandBuffer,
        image_index: usize,
        clear_color: [f32; 4],
    ) -> Result<(), RendererError> {
        command_buffer.begin(self.device.device())?;

        command_buffer.begin_render_pass(
            self.device.device(),
            self.render_pass.handle(),
            self.framebuffers[image_index].handle(),
            self.swapchain.extent(),
            clear_color,
        );

        // No draw commands yet - just clear

        command_buffer.end_render_pass(self.device.device());

        command_buffer.end(self.device.device())?;

        Ok(())
    }

    /// Wait for device idle (cleanup)
    pub fn wait_idle(&self) {
        unsafe {
            self.device.device().device_wait_idle().unwrap();
        }
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Window opens with clear color
- [ ] Render loop runs at 60 FPS
- [ ] Synchronization works (no tearing)
- [ ] Resize handling works
- [ ] Validation layers show no errors
- [ ] All resources cleaned up properly
- [ ] Works on all platforms

---

## 🧪 **Test**

```rust
#[test]
fn test_renderer_creation() {
    let window = Platform::create_window();
    let renderer = Renderer::new(&*window).unwrap();
    // Should not crash
}
```

---

**Dependencies:** [phase1-vulkan-context.md](phase1-vulkan-context.md)
**Next:** [phase1-mesh-rendering.md](phase1-mesh-rendering.md)
