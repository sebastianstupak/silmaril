//! Main renderer orchestration - integrates all Phase 1.6 components
//!
//! This module ties together Window, Surface, Swapchain, RenderPass, Framebuffers,
//! Command buffers, and Synchronization to create a functioning render loop.
//!
//! # Agentic Debug Integration (Phase 1.6.R)
//!
//! The renderer integrates agentic debugging infrastructure to enable AI agents
//! to autonomously debug rendering issues:
//!
//! - Automatic snapshot capture at frame boundaries
//! - Event recording for resource lifecycle and errors
//! - Optional export to SQLite for offline analysis
//! - Configurable debug overhead (can be disabled in production)

use crate::*;
use ash::vk;
use std::marker::PhantomData;
use std::path::Path;
use tracing::{debug, error, info, instrument, warn};

/// Describes one viewport sub-rect within the swapchain surface.
///
/// Pass this to multi-viewport render APIs to specify which portion of the
/// swapchain surface a camera should render into.
pub struct ViewportDescriptor {
    /// Screen-space bounds (pixels, origin at top-left of the swapchain surface)
    pub bounds: engine_render_context::Rect,
    /// View matrix (world → camera space)
    pub view: glam::Mat4,
    /// Projection matrix (camera → clip space)
    pub proj: glam::Mat4,
}

/// Live handle to an in-progress frame.
///
/// The render pass is open for the entire lifetime of this value.  Inject
/// overlay draw calls (e.g. gizmos) directly into `command_buffer` between
/// [`Renderer::begin_frame`] and [`Renderer::end_frame`].
///
/// # Thread safety
///
/// `FrameRecorder` is intentionally `!Send` — the Vulkan command buffer must
/// not cross thread boundaries while recording.  Consume via
/// [`Renderer::end_frame`] on the same thread.
pub struct FrameRecorder {
    /// The Vulkan command buffer currently recording the frame.
    ///
    /// The render pass is already open; record additional draw calls here
    /// before passing the recorder to [`Renderer::end_frame`].
    pub command_buffer: vk::CommandBuffer,
    pub(crate) image_index: u32,
    _not_send: PhantomData<*mut ()>,
}

/// Main renderer struct that orchestrates the rendering pipeline
pub struct Renderer {
    /// Winit window, absent when created via `from_raw_handle`.
    window: Option<Window>,
    /// Fallback dimensions used when `window` is `None`.
    dimensions: (u32, u32),
    context: VulkanContext,
    _entry: ash::Entry,
    #[allow(dead_code)]
    surface: Surface,
    swapchain: Swapchain,
    render_pass: RenderPass,
    framebuffers: Vec<Framebuffer>,
    command_pool: CommandPool,
    command_buffers: Vec<CommandBuffer>,
    sync_objects: Vec<FrameSyncObjects>,
    current_frame: usize,
    clear_color: [f32; 4],
    frame_counter: u64,
    swapchain_needs_rebuild: bool,

    // Agentic debugging (Phase 1.6.R)
    debug_enabled: bool,
    debugger: Option<debug::RenderingDebugger>,
    event_recorder: Option<debug::EventRecorder>,
    debug_exporter: Option<debug::SqliteExporter>,

    // Frame capture (Phase 1.9)
    capture_manager: Option<capture::CaptureManager>,

    // Mesh rendering (Phase 1.8)
    _depth_buffer: Option<DepthBuffer>,
    mesh_pipeline: Option<GraphicsPipeline>,
    gpu_cache: std::cell::RefCell<GpuCache>,

    // Storage buffer descriptor infrastructure for per-mesh uniforms (Phase 1.8)
    // RefCell mirrors gpu_cache — render_meshes has &self but must upload data each frame
    mesh_uniform_buffers: Vec<std::cell::RefCell<crate::mesh_uniform::MeshUniformBuffer>>,
    mesh_descriptor_pool: Option<vk::DescriptorPool>,
    mesh_descriptor_sets: Vec<vk::DescriptorSet>,
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
        let window = Window::new(window_config).map_err(|e| {
            RendererError::surfacecreationfailed(format!("Window creation failed: {:?}", e))
        })?;
        let (width, height) = window.size();

        // 2. Create Vulkan context first (instance creation now includes surface extensions)
        let context = VulkanContext::new(app_name, None, None)?;

        // 3. Create Vulkan entry for surface creation
        let entry = unsafe {
            ash::Entry::load().map_err(|e| {
                RendererError::instancecreationfailed(format!("Failed to load Vulkan: {:?}", e))
            })?
        };

        // 4. Create surface using the context's instance
        let surface = Surface::new(&entry, &context.instance, &window).map_err(|e| {
            RendererError::surfacecreationfailed(format!("Surface creation failed: {:?}", e))
        })?;

        // 6. Create swapchain
        let swapchain =
            Swapchain::new(&context, surface.handle(), surface.loader(), width, height, None)?;

        // 7. Create depth buffer
        let depth_buffer = DepthBuffer::new(&context.device, &context.allocator, swapchain.extent)?;

        // 8. Create render pass (with depth attachment)
        let render_pass = RenderPass::new(
            &context.device,
            RenderPassConfig {
                color_format: swapchain.format,
                depth_format: Some(depth_buffer.format()),
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
            },
        )
        .map_err(|e| RendererError::renderpasscreationfailed(format!("{:?}", e)))?;

        // 9. Create framebuffers with depth attachment
        let mut framebuffers = Vec::with_capacity(swapchain.image_views.len());
        for &image_view in &swapchain.image_views {
            let attachments = [image_view, depth_buffer.image_view()];
            let framebuffer_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass.handle())
                .attachments(&attachments)
                .width(swapchain.extent.width)
                .height(swapchain.extent.height)
                .layers(1);

            let framebuffer = unsafe { context.device.create_framebuffer(&framebuffer_info, None) }
                .map_err(|e| {
                    RendererError::framebuffercreationfailed(format!(
                        "Failed to create framebuffer: {:?}",
                        e
                    ))
                })?;

            framebuffers.push(Framebuffer::from_raw(&context.device, framebuffer));
        }

        // 10. Create mesh pipeline
        let mesh_pipeline = GraphicsPipeline::new_mesh_pipeline_with_descriptors(
            &context.device,
            &render_pass,
            swapchain.extent,
            Some(depth_buffer.format()),
        )?;

        // 11. Create GPU cache
        let gpu_cache = GpuCache::new(&context)?;

        // 12. Create command pool
        let command_pool = CommandPool::new(
            &context.device,
            context.queue_families.graphics,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        )
        .map_err(|e| RendererError::commandpoolcreationfailed(format!("{:?}", e)))?;

        // 13. Allocate command buffers (one per frame in flight)
        const FRAMES_IN_FLIGHT: u32 = 2;
        let command_buffers = command_pool
            .allocate(&context.device, vk::CommandBufferLevel::PRIMARY, FRAMES_IN_FLIGHT)
            .map_err(|e| {
                RendererError::commandbufferallocationfailed(FRAMES_IN_FLIGHT, format!("{:?}", e))
            })?
            .into_iter()
            .map(CommandBuffer::from_handle)
            .collect();

        // 14. Create synchronization objects
        let sync_objects = create_sync_objects(&context.device, FRAMES_IN_FLIGHT).map_err(|e| {
            RendererError::syncobjectcreationfailed("frame sync".to_string(), format!("{:?}", e))
        })?;

        info!(
            width = width,
            height = height,
            images = swapchain.image_count,
            "Renderer created successfully"
        );

        let mut renderer = Self {
            window: Some(window),
            dimensions: (width, height),
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
            frame_counter: 0,
            swapchain_needs_rebuild: false,

            // Debug disabled by default
            debug_enabled: false,
            debugger: None,
            event_recorder: None,
            debug_exporter: None,

            // Capture disabled by default
            capture_manager: None,

            // Mesh rendering enabled
            _depth_buffer: Some(depth_buffer),
            mesh_pipeline: Some(mesh_pipeline),
            gpu_cache: std::cell::RefCell::new(gpu_cache),

            mesh_uniform_buffers: Vec::new(),
            mesh_descriptor_pool: None,
            mesh_descriptor_sets: Vec::new(),
        };

        let frames_in_flight = renderer.sync_objects.len();
        if let Err(e) = renderer.init_mesh_descriptor_resources(frames_in_flight) {
            warn!(error = ?e, "Failed to init mesh descriptor resources — mesh rendering will be unavailable");
        }
        Ok(renderer)
    }

    /// Create a renderer attached to an externally-managed Win32 window.
    ///
    /// Use this when the window (and its HWND) are managed by a host process
    /// such as Tauri — the renderer does not own a winit event loop in this
    /// case.
    ///
    /// The init sequence is identical to [`Renderer::new`] except that the
    /// Vulkan surface is created from the raw HWND rather than from a winit
    /// window.
    ///
    /// # Arguments
    ///
    /// * `hwnd`      — Raw Win32 window handle (as `isize` / `HWND`).
    /// * `width`     — Swapchain width in pixels.
    /// * `height`    — Swapchain height in pixels.
    /// * `app_name`  — Application name reported to the Vulkan driver.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::Renderer;
    /// # let hwnd: isize = 0;
    /// let renderer = Renderer::from_raw_handle(hwnd, 1280, 720, "my-app")?;
    /// # Ok::<(), engine_renderer::RendererError>(())
    /// ```
    #[cfg(windows)]
    pub fn from_raw_handle(
        hwnd: isize,
        width: u32,
        height: u32,
        app_name: &str,
    ) -> Result<Self, RendererError> {
        info!(hwnd = hwnd, width = width, height = height, "Creating renderer from raw HWND");

        // 1. Create Vulkan context (instance creation includes surface extensions)
        let context = VulkanContext::new(app_name, None, None)?;

        // 2. Create Vulkan entry for surface creation
        let entry = unsafe {
            ash::Entry::load().map_err(|e| {
                RendererError::instancecreationfailed(format!("Failed to load Vulkan: {:?}", e))
            })?
        };

        // 3. Create surface from the raw HWND (Windows-only path)
        let surface =
            Surface::from_raw_hwnd(&entry, &context.instance, hwnd).map_err(|e| {
                RendererError::surfacecreationfailed(format!(
                    "Surface creation from HWND failed: {:?}",
                    e
                ))
            })?;

        // 4. Create swapchain
        let swapchain =
            Swapchain::new(&context, surface.handle(), surface.loader(), width, height, None)?;

        // 5. Create depth buffer
        let depth_buffer =
            DepthBuffer::new(&context.device, &context.allocator, swapchain.extent)?;

        // 6. Create render pass (with depth attachment)
        let render_pass = RenderPass::new(
            &context.device,
            RenderPassConfig {
                color_format: swapchain.format,
                depth_format: Some(depth_buffer.format()),
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
            },
        )
        .map_err(|e| RendererError::renderpasscreationfailed(format!("{:?}", e)))?;

        // 7. Create framebuffers with depth attachment
        let mut framebuffers = Vec::with_capacity(swapchain.image_views.len());
        for &image_view in &swapchain.image_views {
            let attachments = [image_view, depth_buffer.image_view()];
            let framebuffer_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass.handle())
                .attachments(&attachments)
                .width(swapchain.extent.width)
                .height(swapchain.extent.height)
                .layers(1);

            let framebuffer =
                unsafe { context.device.create_framebuffer(&framebuffer_info, None) }.map_err(
                    |e| {
                        RendererError::framebuffercreationfailed(format!(
                            "Failed to create framebuffer: {:?}",
                            e
                        ))
                    },
                )?;

            framebuffers.push(Framebuffer::from_raw(&context.device, framebuffer));
        }

        // 8. Create mesh pipeline
        let mesh_pipeline = GraphicsPipeline::new_mesh_pipeline_with_descriptors(
            &context.device,
            &render_pass,
            swapchain.extent,
            Some(depth_buffer.format()),
        )?;

        // 9. Create GPU cache
        let gpu_cache = GpuCache::new(&context)?;

        // 10. Create command pool
        let command_pool = CommandPool::new(
            &context.device,
            context.queue_families.graphics,
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        )
        .map_err(|e| RendererError::commandpoolcreationfailed(format!("{:?}", e)))?;

        // 11. Allocate command buffers (one per frame in flight)
        const FRAMES_IN_FLIGHT: u32 = 2;
        let command_buffers = command_pool
            .allocate(&context.device, vk::CommandBufferLevel::PRIMARY, FRAMES_IN_FLIGHT)
            .map_err(|e| {
                RendererError::commandbufferallocationfailed(FRAMES_IN_FLIGHT, format!("{:?}", e))
            })?
            .into_iter()
            .map(CommandBuffer::from_handle)
            .collect();

        // 12. Create synchronization objects
        let sync_objects =
            create_sync_objects(&context.device, FRAMES_IN_FLIGHT).map_err(|e| {
                RendererError::syncobjectcreationfailed(
                    "frame sync".to_string(),
                    format!("{:?}", e),
                )
            })?;

        info!(
            width = width,
            height = height,
            images = swapchain.image_count,
            "Renderer (raw HWND) created successfully"
        );

        let mut renderer = Self {
            window: None,
            dimensions: (width, height),
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
            clear_color: [0.0, 0.0, 0.0, 1.0],
            frame_counter: 0,
            swapchain_needs_rebuild: false,

            debug_enabled: false,
            debugger: None,
            event_recorder: None,
            debug_exporter: None,

            capture_manager: None,

            _depth_buffer: Some(depth_buffer),
            mesh_pipeline: Some(mesh_pipeline),
            gpu_cache: std::cell::RefCell::new(gpu_cache),

            mesh_uniform_buffers: Vec::new(),
            mesh_descriptor_pool: None,
            mesh_descriptor_sets: Vec::new(),
        };

        let frames_in_flight = renderer.sync_objects.len();
        if let Err(e) = renderer.init_mesh_descriptor_resources(frames_in_flight) {
            warn!(error = ?e, "Failed to init mesh descriptor resources — mesh rendering will be unavailable");
        }
        Ok(renderer)
    }

    /// Begin a new frame and open the render pass.
    ///
    /// Returns a [`FrameRecorder`] that gives the caller access to the active
    /// Vulkan command buffer so additional draw calls (e.g. gizmos, overlays)
    /// can be injected before the frame is submitted.
    ///
    /// Returns `None` if the swapchain is out of date and needs to be rebuilt
    /// (the caller should handle resizing and retry on the next tick).
    ///
    /// # Usage
    ///
    /// ```no_run
    /// # use engine_renderer::{Renderer, WindowConfig};
    /// # let mut renderer = Renderer::new(WindowConfig::default(), "test")?;
    /// if let Some(recorder) = renderer.begin_frame() {
    ///     // inject extra draw calls into recorder.command_buffer …
    ///     renderer.end_frame(recorder);
    /// }
    /// # Ok::<(), engine_renderer::RendererError>(())
    /// ```
    /// Initialise per-frame descriptor pool, descriptor sets, and storage
    /// buffers for [`crate::mesh_uniform::MeshUniform`] data.
    ///
    /// Called once after the mesh pipeline is created.  Safe to call when
    /// no pipeline (or a pipeline built without descriptors) is present —
    /// it returns `Ok(())` immediately in both cases.
    pub(crate) fn init_mesh_descriptor_resources(
        &mut self,
        frames_in_flight: usize,
    ) -> Result<(), RendererError> {
        use crate::mesh_uniform::{MeshUniformBuffer, MESH_UNIFORM_INITIAL_CAPACITY};

        let pipeline = match &self.mesh_pipeline {
            Some(p) => p,
            None => return Ok(()), // no pipeline yet — skip
        };
        let layout = pipeline.descriptor_set_layout();
        if layout == vk::DescriptorSetLayout::null() {
            return Ok(()); // pipeline built without descriptors — skip
        }

        // Create per-frame storage buffers
        let buffers: Result<Vec<_>, _> = (0..frames_in_flight)
            .map(|_| {
                MeshUniformBuffer::new(&self.context, MESH_UNIFORM_INITIAL_CAPACITY)
                    .map(std::cell::RefCell::new)
            })
            .collect();
        self.mesh_uniform_buffers = buffers?;

        // Descriptor pool
        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(frames_in_flight as u32);

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(std::slice::from_ref(&pool_size))
            .max_sets(frames_in_flight as u32);

        let pool = unsafe {
            self.context
                .device
                .create_descriptor_pool(&pool_info, None)
                .map_err(|e| {
                    RendererError::pipelinecreationfailed(format!("descriptor pool: {:?}", e))
                })?
        };
        self.mesh_descriptor_pool = Some(pool);

        // Allocate one descriptor set per frame
        let layouts: Vec<vk::DescriptorSetLayout> = vec![layout; frames_in_flight];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(pool)
            .set_layouts(&layouts);

        self.mesh_descriptor_sets = unsafe {
            self.context
                .device
                .allocate_descriptor_sets(&alloc_info)
                .map_err(|e| {
                    RendererError::pipelinecreationfailed(format!(
                        "descriptor set alloc: {:?}",
                        e
                    ))
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

        tracing::info!(frames_in_flight, "Mesh descriptor resources initialised");
        Ok(())
    }

    /// Begin a new frame and open the render pass.
    ///
    /// Returns a [`FrameRecorder`] that gives the caller access to the active
    /// Vulkan command buffer so additional draw calls (e.g. gizmos, overlays)
    /// can be injected before the frame is submitted.
    ///
    /// Returns `None` if the swapchain is out of date and needs to be rebuilt
    /// (the caller should handle resizing and retry on the next tick).
    ///
    /// # Usage
    ///
    /// ```no_run
    /// # use engine_renderer::{Renderer, WindowConfig};
    /// # let mut renderer = Renderer::new(WindowConfig::default(), "test")?;
    /// if let Some(recorder) = renderer.begin_frame() {
    ///     // inject extra draw calls into recorder.command_buffer …
    ///     renderer.end_frame(recorder);
    /// }
    /// # Ok::<(), engine_renderer::RendererError>(())
    /// ```
    #[instrument(skip(self))]
    pub fn begin_frame(&mut self) -> Option<FrameRecorder> {
        // If the previous frame signalled that the swapchain is suboptimal,
        // signal the caller to rebuild before proceeding.
        if self.swapchain_needs_rebuild {
            self.swapchain_needs_rebuild = false;
            return None;
        }

        let sync = &self.sync_objects[self.current_frame];

        // Wait for the previous use of this frame slot to finish.
        let wait_result = unsafe {
            self.context.device.wait_for_fences(
                &[sync.in_flight_fence],
                true,
                u64::MAX,
            )
        };
        if let Err(e) = wait_result {
            error!(error = ?e, "begin_frame: failed to wait for in-flight fence");
            return None;
        }

        // Acquire the next swapchain image.
        let acquire_result = unsafe {
            self.swapchain.loader.acquire_next_image(
                self.swapchain.swapchain,
                u64::MAX,
                sync.image_available_semaphore,
                vk::Fence::null(),
            )
        };

        let image_index = match acquire_result {
            Ok((idx, suboptimal)) => {
                if suboptimal {
                    // Mark the swapchain for rebuild; the current frame can
                    // still be rendered, but the next call to begin_frame will
                    // return None so the caller can resize.
                    self.swapchain_needs_rebuild = true;
                }
                idx
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                // Swapchain must be rebuilt; signal caller by returning None.
                return None;
            }
            Err(e) => {
                error!(error = ?e, "begin_frame: swapchain acquire failed");
                return None;
            }
        };

        // Reset the fence now that we know which image we have.
        if let Err(e) =
            unsafe { self.context.device.reset_fences(&[sync.in_flight_fence]) }
        {
            error!(error = ?e, "begin_frame: failed to reset fence");
            return None;
        }

        // Begin the command buffer.
        let cmd = self.command_buffers[self.current_frame].handle();

        let begin_info = vk::CommandBufferBeginInfo::default();
        if let Err(e) =
            unsafe { self.context.device.begin_command_buffer(cmd, &begin_info) }
        {
            error!(error = ?e, "begin_frame: failed to begin command buffer");
            return None;
        }

        // Open the render pass.
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue { float32: self.clear_color },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 },
            },
        ];

        let render_pass_begin = vk::RenderPassBeginInfo::default()
            .render_pass(self.render_pass.handle())
            .framebuffer(self.framebuffers[image_index as usize].handle())
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.extent,
            })
            .clear_values(&clear_values);

        unsafe {
            self.context.device.cmd_begin_render_pass(
                cmd,
                &render_pass_begin,
                vk::SubpassContents::INLINE,
            );
        }

        Some(FrameRecorder {
            command_buffer: cmd,
            image_index,
            _not_send: PhantomData,
        })
    }

    /// Close the render pass, submit the command buffer, and present.
    ///
    /// Consumes the [`FrameRecorder`] returned by [`Renderer::begin_frame`],
    /// which ensures the render pass cannot be ended more than once.
    ///
    /// Advances the internal frame counter.
    #[instrument(skip(self, recorder))]
    pub fn end_frame(&mut self, recorder: FrameRecorder) {
        let cmd = recorder.command_buffer;
        let image_index = recorder.image_index;
        // `recorder` is consumed here; `_not_send` phantom field is dropped.
        drop(recorder);

        let sync = &self.sync_objects[self.current_frame];

        // Close the render pass and command buffer.
        unsafe {
            self.context.device.cmd_end_render_pass(cmd);
        }

        if let Err(e) = unsafe { self.context.device.end_command_buffer(cmd) } {
            error!(error = ?e, "end_frame: failed to end command buffer");
            // Advance frame counter even on error to avoid stalling.
            self.current_frame = (self.current_frame + 1) % self.sync_objects.len();
            self.frame_counter += 1;
            return;
        }

        // Submit.
        let wait_semaphores = [sync.image_available_semaphore];
        let signal_semaphores = [sync.render_finished_semaphore];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers = [cmd];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);

        if let Err(e) = unsafe {
            self.context.device.queue_submit(
                self.context.graphics_queue,
                &[submit_info],
                sync.in_flight_fence,
            )
        } {
            error!(error = ?e, "end_frame: queue submit failed");
            self.current_frame = (self.current_frame + 1) % self.sync_objects.len();
            self.frame_counter += 1;
            return;
        }

        // Present.
        let swapchains = [self.swapchain.swapchain];
        let image_indices = [image_index];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        if let Err(e) = unsafe {
            self.swapchain
                .loader
                .queue_present(self.context.present_queue, &present_info)
        } {
            match e {
                vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR => {
                    // Caller will need to rebuild swapchain on the next begin_frame.
                }
                _ => {
                    error!(error = ?e, "end_frame: present failed");
                }
            }
        }

        self.current_frame = (self.current_frame + 1) % self.sync_objects.len();
        self.frame_counter += 1;
    }

    /// Enable agentic debugging with optional database export
    ///
    /// # Arguments
    /// * `config` - Debug configuration (overdraw, entity IDs, thresholds)
    /// * `export_path` - Optional path to SQLite database for exporting snapshots
    ///
    /// # Example
    /// ```no_run
    /// # use engine_renderer::{Renderer, WindowConfig};
    /// # use engine_renderer::debug::DebugConfig;
    /// let mut renderer = Renderer::new(WindowConfig::default(), "MyApp")?;
    ///
    /// // Enable debugging with database export
    /// renderer.enable_debug(DebugConfig::default(), Some("debug.db"))?;
    ///
    /// // Render loop will now automatically capture debug data
    /// if let Some(recorder) = renderer.begin_frame() {
    ///     renderer.end_frame(recorder);
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn enable_debug(
        &mut self,
        _config: debug::DebugConfig,
        export_path: Option<&str>,
    ) -> Result<(), RendererError> {
        info!("Enabling agentic debug infrastructure");

        // Note: RenderingDebugger requires Arc<VulkanContext>, but we own context directly.
        // For Phase 1.6.R integration, we'll use event recording and snapshot export.
        // Frame capture (R.5) will be fully integrated in Phase 1.7 when mesh rendering exists.

        // Create event recorder and enable it
        let recorder = debug::EventRecorder::new();
        recorder.enable();
        self.event_recorder = Some(recorder);

        // Create exporter if path provided
        if let Some(path) = export_path {
            let exporter = debug::SqliteExporter::create(Path::new(path)).map_err(|e| {
                RendererError::renderpasscreationfailed(format!(
                    "Failed to create debug exporter: {:?}",
                    e
                ))
            })?;
            self.debug_exporter = Some(exporter);
            info!(path = path, "Debug snapshots will be exported to database");
        }

        self.debug_enabled = true;
        Ok(())
    }

    /// Disable agentic debugging
    pub fn disable_debug(&mut self) {
        self.debug_enabled = false;
        self.debugger = None;
        self.event_recorder = None;
        self.debug_exporter = None;
        info!("Agentic debug infrastructure disabled");
    }

    /// Get current frame counter (useful for correlating with debug data)
    pub fn frame_count(&self) -> u64 {
        self.frame_counter
    }

    /// Set the clear color (RGBA, 0.0-1.0)
    pub fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.clear_color = [r, g, b, a];
    }

    /// Returns the current render dimensions (width, height) in pixels.
    ///
    /// When the renderer was created with a winit window the live window size
    /// is returned; otherwise the dimensions supplied to `from_raw_handle` are
    /// used.
    pub fn dimensions(&self) -> (u32, u32) {
        if let Some(w) = &self.window {
            w.size()
        } else {
            self.dimensions
        }
    }

    /// Get window reference.
    ///
    /// Returns `None` when the renderer was created via [`Renderer::from_raw_handle`].
    pub fn window(&self) -> Option<&Window> {
        self.window.as_ref()
    }

    /// Get window mut reference.
    ///
    /// Returns `None` when the renderer was created via [`Renderer::from_raw_handle`].
    pub fn window_mut(&mut self) -> Option<&mut Window> {
        self.window.as_mut()
    }

    /// Take ownership of the event loop for manual event pumping
    ///
    /// This allows using winit 0.30's pump_app_events() for proper event handling.
    /// After calling this, the window's event loop will be None.
    ///
    /// Returns `None` when the renderer was created via [`Renderer::from_raw_handle`]
    /// (no winit event loop exists in that case).
    pub fn take_event_loop(&mut self) -> Option<winit::event_loop::EventLoop<()>> {
        self.window.as_mut().and_then(|w| w.take_event_loop())
    }

    /// Render meshes from ECS world into the active frame.
    ///
    /// Queries all entities with [`Transform`](engine_core::Transform) +
    /// [`MeshRenderer`](engine_core::MeshRenderer) components and issues draw
    /// calls directly into `recorder.command_buffer`.  Draw calls are emitted
    /// once per viewport so each viewport gets its own view-projection matrix.
    ///
    /// Pass `assets: None` when mesh rendering should be skipped for this
    /// phase (e.g. gizmo-only passes).  The method is a no-op in that case.
    ///
    /// # Arguments
    /// * `recorder`  - Active frame recorder (render pass is open).
    /// * `world`     - ECS world containing entities to render.
    /// * `assets`    - Asset manager; `None` = deferred/gizmo-only, early return.
    /// * `viewports` - One or more viewport descriptors; empty = no-op.
    ///
    /// # Example
    /// ```no_run
    /// # use engine_renderer::{Renderer, WindowConfig, ViewportDescriptor};
    /// # use engine_core::{World, Transform, MeshRenderer};
    /// # use engine_assets::AssetManager;
    /// # use engine_render_context::Rect;
    /// let mut renderer = Renderer::new(WindowConfig::default(), "MyApp")?;
    /// let world = World::new();
    /// let assets = AssetManager::new();
    ///
    /// if let Some(recorder) = renderer.begin_frame() {
    ///     let vp = ViewportDescriptor {
    ///         bounds: Rect { x: 0, y: 0, width: 1920, height: 1080 },
    ///         view: glam::Mat4::IDENTITY,
    ///         proj: glam::Mat4::IDENTITY,
    ///     };
    ///     renderer.render_meshes(&recorder, &world, Some(&assets), &[vp]);
    ///     renderer.end_frame(recorder);
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[instrument(skip(self, recorder, world, assets, viewports))]
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

        // ── 1. Build MeshUniform list + draw list ─────────────────────────────
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
                                warn!(error = ?e, mesh_id = ?mesh_id, "render_meshes: GPU upload failed, skipping entity");
                                continue;
                            }
                        }
                        None => {
                            warn!(mesh_id = ?mesh_id, "render_meshes: mesh not in AssetManager, skipping entity");
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

        // ── 2. Upload MeshUniform data via RefCell borrow_mut ─────────────────
        if let Some(buf_cell) = self.mesh_uniform_buffers.get(frame_idx) {
            let mut buf = buf_cell.borrow_mut();
            match buf.upload(&self.context, &uniforms) {
                Ok(resized) if resized => {
                    // Buffer reallocated — rebind descriptor set to new VkBuffer
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

            // Dynamic scissor (clamp to swapchain extent to avoid validation errors on
            // resize races).
            let sw = self.swapchain.extent;
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: vp.bounds.x, y: vp.bounds.y },
                extent: vk::Extent2D {
                    width: vp.bounds.width
                        .min(sw.width.saturating_sub(vp.bounds.x.max(0) as u32)),
                    height: vp.bounds.height
                        .min(sw.height.saturating_sub(vp.bounds.y.max(0) as u32)),
                },
            };

            unsafe {
                self.context.device.cmd_set_viewport(cmd, 0, &[viewport]);
                self.context.device.cmd_set_scissor(cmd, 0, &[scissor]);

                // Push VP matrix (64 bytes) — matches new shader push_constant layout
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
                            1,              // instance count = 1 per entity
                            0,              // first index
                            0,              // vertex offset
                            instance_index, // firstInstance → gl_InstanceIndex in shader
                        );
                    }
                }
            }
        }

        debug!(
            draw_count = draw_list.len(),
            viewport_count = viewports.len(),
            "render_meshes: issued draw calls"
        );
    }

    /// Wait for device to finish all operations
    pub fn wait_idle(&self) -> Result<(), RendererError> {
        self.context.wait_idle()
    }

    /// Borrow the raw Vulkan logical device handle.
    ///
    /// Useful for overlay pipelines (grid, gizmos) that need to create their
    /// own Vulkan resources against the same device.
    pub fn device(&self) -> &ash::Device {
        &self.context.device
    }

    /// The active render pass handle.
    ///
    /// Overlay pipelines need this to create compatible graphics pipelines.
    pub fn render_pass(&self) -> vk::RenderPass {
        self.render_pass.handle()
    }

    /// Current swapchain extent (width x height in pixels).
    pub fn extent(&self) -> vk::Extent2D {
        self.swapchain.extent
    }

    /// Borrow the underlying [`VulkanContext`](engine_render_context::VulkanContext).
    ///
    /// Provides access to the allocator, queues, and other low-level handles
    /// needed by overlay pipelines.
    pub fn context(&self) -> &VulkanContext {
        &self.context
    }

    /// Rebuild the swapchain, depth buffer, and framebuffers for a new size.
    ///
    /// Call this when the host window is resized.  The renderer waits for the
    /// device to become idle before tearing down old resources.
    pub fn rebuild_swapchain(&mut self, width: u32, height: u32) -> Result<(), RendererError> {
        let (width, height) = (width.max(1), height.max(1));
        self.context.wait_idle()?;

        self.framebuffers.clear();

        self.swapchain
            .recreate(
                &self.context,
                self.surface.handle(),
                self.surface.loader(),
                width,
                height,
            )
            .map_err(|e| {
                RendererError::surfacecreationfailed(format!("Swapchain recreate failed: {:?}", e))
            })?;

        let depth_buffer =
            DepthBuffer::new(&self.context.device, &self.context.allocator, self.swapchain.extent)?;

        let mut framebuffers = Vec::with_capacity(self.swapchain.image_views.len());
        for &image_view in &self.swapchain.image_views {
            let attachments = [image_view, depth_buffer.image_view()];
            let framebuffer_info = vk::FramebufferCreateInfo::default()
                .render_pass(self.render_pass.handle())
                .attachments(&attachments)
                .width(self.swapchain.extent.width)
                .height(self.swapchain.extent.height)
                .layers(1);

            let framebuffer =
                unsafe { self.context.device.create_framebuffer(&framebuffer_info, None) }.map_err(
                    |e| {
                        RendererError::framebuffercreationfailed(format!(
                            "Failed to create framebuffer: {:?}",
                            e
                        ))
                    },
                )?;

            framebuffers.push(Framebuffer::from_raw(&self.context.device, framebuffer));
        }

        self._depth_buffer = Some(depth_buffer);
        self.framebuffers = framebuffers;
        self.dimensions = (width, height);
        self.swapchain_needs_rebuild = false;

        info!(width, height, "Swapchain rebuilt");
        Ok(())
    }

    /// Enable frame capture with configuration
    ///
    /// # Example
    /// ```no_run
    /// # use engine_renderer::{Renderer, WindowConfig, CaptureConfig, CaptureFormat};
    /// # use std::path::PathBuf;
    /// let mut renderer = Renderer::new(WindowConfig::default(), "MyApp")?;
    ///
    /// let capture_config = CaptureConfig {
    ///     enabled: true,
    ///     format: CaptureFormat::Png,
    ///     output_dir: PathBuf::from("captures"),
    ///     filename_pattern: "frame_{:06}.png".to_string(),
    /// };
    ///
    /// renderer.enable_capture(capture_config)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn enable_capture(&mut self, config: capture::CaptureConfig) -> Result<(), RendererError> {
        info!("Enabling frame capture");

        let (width, height) = self.dimensions();
        let mut manager = capture::CaptureManager::new(config);
        manager.initialize(&self.context.device, &self.context.allocator, width, height)?;

        self.capture_manager = Some(manager);
        Ok(())
    }

    /// Disable frame capture
    pub fn disable_capture(&mut self) {
        self.capture_manager = None;
        info!("Frame capture disabled");
    }

    /// Capture current frame manually (independent of render loop)
    ///
    /// This captures the current swapchain image to disk.
    pub fn capture_screenshot(&mut self, _filename: &str) -> Result<(), RendererError> {
        if let Some(manager) = &mut self.capture_manager {
            let current_image = self.swapchain.images[self.current_frame];

            // Capture frame using manager
            manager.capture_frame(
                &self.context.device,
                &self.command_pool,
                self.context.graphics_queue,
                current_image,
            )?;

            info!(filename = _filename, "Screenshot saved");
            Ok(())
        } else {
            Err(RendererError::imagecreationfailed(0, 0, "Capture not enabled".to_string()))
        }
    }

    /// Get latest frame as PNG bytes (for AI agent streaming)
    ///
    /// This is useful for sending frames to AI models for analysis.
    pub fn get_frame_png(&mut self) -> Result<Vec<u8>, RendererError> {
        if let Some(manager) = &mut self.capture_manager {
            let current_image = self.swapchain.images[self.current_frame];
            manager.get_latest_frame_png(
                &self.context.device,
                &self.command_pool,
                self.context.graphics_queue,
                current_image,
            )
        } else {
            Err(RendererError::imagecreationfailed(0, 0, "Capture not enabled".to_string()))
        }
    }

    /// Get capture performance metrics
    pub fn capture_metrics(&self) -> Option<&capture::CaptureMetrics> {
        self.capture_manager.as_ref().map(|m| m.metrics())
    }
}

// Manual Drop implementation to ensure correct Vulkan cleanup order
impl Drop for Renderer {
    fn drop(&mut self) {
        info!("Destroying renderer");

        // CRITICAL: Wait for GPU to finish all work before destroying resources
        if let Err(e) = self.context.wait_idle() {
            error!(error = ?e, "Failed to wait for device idle during cleanup");
        }

        // Vulkan cleanup must happen in specific order to avoid access violations:
        // 1. Command buffers (reference framebuffers, pipelines)
        // 2. Framebuffers (reference render pass, depth buffer)
        // 3. Depth buffer (GPU memory allocation)
        // 4. Pipelines (reference render pass)
        // 5. Render pass
        // 6. Swapchain
        // 7. Surface
        // 8. Device/Context
        //
        // We explicitly drop in correct order (rest happens automatically):

        // Drop command buffers first (they reference many resources)
        drop(self.command_buffers.drain(..));

        // Drop framebuffers (they reference depth buffer and render pass)
        drop(self.framebuffers.drain(..));

        // Drop depth buffer (now safe, framebuffers are gone)
        drop(self._depth_buffer.take());

        // Drop mesh pipeline (references render pass)
        drop(self.mesh_pipeline.take());

        // Descriptor pool (implicitly frees all descriptor sets)
        if let Some(pool) = self.mesh_descriptor_pool.take() {
            unsafe { self.context.device.destroy_descriptor_pool(pool, None) };
        }
        // mesh_uniform_buffers drop automatically via GpuBuffer::Drop
        self.mesh_uniform_buffers.clear();

        // Drop GPU cache (has GPU buffer allocations)
        // Note: This has its own Drop that properly frees GPU memory
        // We don't explicitly drop it, but mentioning for documentation

        // Remaining cleanup happens automatically in reverse field order:
        // - render_pass
        // - swapchain (references surface)
        // - surface
        // - context (device, instance)
        // - window

        info!("Renderer destroyed");
    }
}
