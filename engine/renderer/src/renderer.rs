//! Main renderer orchestration - integrates all Phase 1.6 components
//!
//! This module ties together Window, Surface, Swapchain, RenderPass, Framebuffers,
//! Command buffers, and Synchronization to create a functioning render loop.

use crate::*;
use ash::vk;
use tracing::{info, instrument, warn};

/// Main renderer struct that orchestrates the rendering pipeline
pub struct Renderer {
    window: Window,
    context: VulkanContext,
    _entry: ash::Entry,
    surface: Surface,
    swapchain: Swapchain,
    render_pass: RenderPass,
    framebuffers: Vec<Framebuffer>,
    command_pool: CommandPool,
    command_buffers: Vec<CommandBuffer>,
    sync_objects: Vec<FrameSyncObjects>,
    current_frame: usize,
    clear_color: [f32; 4],
}

impl Renderer {
    /// Create a new renderer with a window
    ///
    /// # Arguments
    /// * `window_config` - Window configuration
    /// * `app_name` - Application name for Vulkan
    #[instrument(skip_all)]
    pub fn new(window_config: WindowConfig, app_name: &str) -> Result<Self, RendererError> {
        info!("Creating renderer");

        // 1. Create window
        let window = Window::new(window_config)
            .map_err(|e| RendererError::surfacecreationfailed(format!("Window creation failed: {:?}", e)))?;
        let (width, height) = window.size();

        // 2. Create Vulkan entry
        let entry = unsafe {
            ash::Entry::load()
                .map_err(|e| RendererError::instancecreationfailed(format!("Failed to load Vulkan: {:?}", e)))?
        };

        // 3. Create temporary context to get instance for surface creation
        let temp_context = VulkanContext::new(app_name, None, None)?;

        // 4. Create surface using temporary context's instance
        let surface = Surface::new(&entry, &temp_context.instance, &window)
            .map_err(|e| RendererError::surfacecreationfailed(format!("Surface creation failed: {:?}", e)))?;

        // 5. Create final Vulkan context with surface (this ensures proper device selection)
        let context = VulkanContext::new(app_name, Some(surface.handle()), Some(surface.loader()))?;

        // Temp context gets dropped here, but surface was created and is valid

        // 6. Create swapchain
        let swapchain = Swapchain::new(
            &context,
            surface.handle(),
            surface.loader(),
            width,
            height,
            None,
        )?;

        // 7. Create render pass (simple color attachment, clear to color)
        let render_pass = RenderPass::new(
            &context.device,
            RenderPassConfig {
                color_format: swapchain.format,
                depth_format: None,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
            },
        )
        .map_err(|e| RendererError::renderpasscreationfailed(format!("{:?}", e)))?;

        // 8. Create framebuffers (one per swapchain image)
        let framebuffers = create_framebuffers(
            &context.device,
            render_pass.handle(),
            &swapchain.image_views,
            swapchain.extent,
        )
        .map_err(|e| RendererError::framebuffercreationfailed(format!("{:?}", e)))?;

        // 9. Create command pool
        let command_pool = CommandPool::new(
            &context.device,
            context.queue_families.graphics,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        )
        .map_err(|e| RendererError::commandpoolcreationfailed(format!("{:?}", e)))?;

        // 10. Allocate command buffers (one per frame in flight)
        const FRAMES_IN_FLIGHT: u32 = 2;
        let command_buffers = command_pool
            .allocate(
                &context.device,
                vk::CommandBufferLevel::PRIMARY,
                FRAMES_IN_FLIGHT,
            )
            .map_err(|e| RendererError::commandbufferallocationfailed(FRAMES_IN_FLIGHT, format!("{:?}", e)))?
            .into_iter()
            .map(CommandBuffer::from_handle)
            .collect();

        // 11. Create synchronization objects
        let sync_objects = create_sync_objects(&context.device, FRAMES_IN_FLIGHT)
            .map_err(|e| RendererError::syncobjectcreationfailed("frame sync".to_string(), format!("{:?}", e)))?;

        info!(
            width = width,
            height = height,
            images = swapchain.image_count,
            "Renderer created successfully"
        );

        Ok(Self {
            window,
            context,
            _entry: entry,
            surface,
            swapchain,
            render_pass,
            framebuffers,
            command_pool,
            command_buffers,
            sync_objects,
            current_frame: 0,
            clear_color: [0.0, 0.0, 0.0, 1.0], // Black by default
        })
    }

    /// Set the clear color (RGBA, 0.0-1.0)
    pub fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.clear_color = [r, g, b, a];
    }

    /// Get window reference
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Get window mut reference
    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    /// Render a frame (clears to configured color)
    #[instrument(skip(self))]
    pub fn render_frame(&mut self) -> Result<(), RendererError> {
        let sync = &self.sync_objects[self.current_frame];

        // Wait for previous frame to finish
        unsafe {
            self.context
                .device
                .wait_for_fences(&[sync.in_flight_fence], true, u64::MAX)
                .map_err(|e| {
                    RendererError::queuesubmissionfailed(format!("Failed to wait for fence: {:?}", e))
                })?;
        }

        // Acquire next swapchain image
        let image_index = unsafe {
            self.swapchain
                .loader
                .acquire_next_image(
                    self.swapchain.swapchain,
                    u64::MAX,
                    sync.image_available_semaphore,
                    vk::Fence::null(),
                )
                .map_err(|e| match e {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => RendererError::swapchainoutofdate(),
                    _ => RendererError::swapchainacquisitionfailed(format!("{:?}", e)),
                })?
                .0 as usize
        };

        // Reset fence after acquiring image
        unsafe {
            self.context
                .device
                .reset_fences(&[sync.in_flight_fence])
                .map_err(|e| {
                    RendererError::queuesubmissionfailed(format!("Failed to reset fence: {:?}", e))
                })?;
        }

        // Record command buffer
        let cmd_buffer = self.command_buffers[self.current_frame].handle();
        self.record_command_buffer(cmd_buffer, image_index)?;

        // Submit command buffer
        let wait_semaphores = [sync.image_available_semaphore];
        let signal_semaphores = [sync.render_finished_semaphore];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [cmd_buffer];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            self.context
                .device
                .queue_submit(
                    self.context.graphics_queue,
                    &[submit_info],
                    sync.in_flight_fence,
                )
                .map_err(|e| {
                    RendererError::queuesubmissionfailed(format!("Failed to submit queue: {:?}", e))
                })?;
        }

        // Present
        let swapchains = [self.swapchain.swapchain];
        let image_indices = [image_index as u32];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            self.swapchain
                .loader
                .queue_present(self.context.present_queue, &present_info)
                .map_err(|e| match e {
                    vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR => {
                        RendererError::swapchainoutofdate()
                    }
                    _ => RendererError::presentfailed(format!("{:?}", e)),
                })?;
        }

        // Advance to next frame
        self.current_frame = (self.current_frame + 1) % self.sync_objects.len();

        Ok(())
    }

    /// Record command buffer for rendering
    fn record_command_buffer(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
    ) -> Result<(), RendererError> {
        unsafe {
            // Begin command buffer
            let begin_info = vk::CommandBufferBeginInfo::default();
            self.context
                .device
                .begin_command_buffer(command_buffer, &begin_info)
                .map_err(|e| {
                    RendererError::commandbufferallocationfailed(
                        1,
                        format!("Failed to begin command buffer: {:?}", e),
                    )
                })?;

            // Begin render pass
            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: self.clear_color,
                },
            }];

            let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass.handle())
                .framebuffer(self.framebuffers[image_index].handle())
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain.extent,
                })
                .clear_values(&clear_values);

            self.context.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            // No draw calls yet - just clear color (Phase 1.6 goal)
            // Phase 1.7 will add mesh rendering

            // End render pass
            self.context.device.cmd_end_render_pass(command_buffer);

            // End command buffer
            self.context
                .device
                .end_command_buffer(command_buffer)
                .map_err(|e| {
                    RendererError::commandbufferallocationfailed(
                        1,
                        format!("Failed to end command buffer: {:?}", e),
                    )
                })?;
        }

        Ok(())
    }

    /// Wait for device to finish all operations
    pub fn wait_idle(&self) -> Result<(), RendererError> {
        self.context.wait_idle()
    }
}

// Cleanup happens automatically via Drop implementations of contained types
