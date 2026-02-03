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
use std::path::Path;
use tracing::{error, info, instrument, warn};

/// Main renderer struct that orchestrates the rendering pipeline
pub struct Renderer {
    window: Window,
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
    gpu_cache: GpuCache,
    // Render queue for current frame
    render_queue: Vec<MeshRenderCommand>,
}

/// A single mesh render command
#[derive(Debug, Clone)]
struct MeshRenderCommand {
    mesh_id: engine_assets::AssetId,
    mvp_matrix: glam::Mat4,
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

        // 2. Create Vulkan entry
        let entry = unsafe {
            ash::Entry::load().map_err(|e| {
                RendererError::instancecreationfailed(format!("Failed to load Vulkan: {:?}", e))
            })?
        };

        // 3. Create temporary context to get instance for surface creation
        let temp_context = VulkanContext::new(app_name, None, None)?;

        // 4. Create surface using temporary context's instance
        let surface = Surface::new(&entry, &temp_context.instance, &window).map_err(|e| {
            RendererError::surfacecreationfailed(format!("Surface creation failed: {:?}", e))
        })?;

        // 5. Create final Vulkan context with surface (this ensures proper device selection)
        let context = VulkanContext::new(app_name, Some(surface.handle()), Some(surface.loader()))?;

        // Temp context gets dropped here, but surface was created and is valid

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
        let mesh_pipeline = GraphicsPipeline::new_mesh_pipeline(
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
            frame_counter: 0,

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
            gpu_cache,
            render_queue: Vec::new(),
        })
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
    /// renderer.render_frame()?;
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

    /// Get window reference
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Get window mut reference
    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    /// Render meshes from ECS world
    ///
    /// Queries all entities with Transform + MeshRenderer components,
    /// finds active camera, calculates MVP matrices, and issues draw calls.
    ///
    /// # Arguments
    /// * `world` - ECS world containing entities to render
    ///
    /// # Example
    /// ```no_run
    /// # use engine_renderer::{Renderer, WindowConfig};
    /// # use engine_core::{World, Transform, MeshRenderer, Camera};
    /// # use engine_assets::{AssetManager, MeshData, AssetId};
    /// let mut renderer = Renderer::new(WindowConfig::default(), "MyApp")?;
    /// let mut world = World::new();
    ///
    /// // Spawn a cube
    /// let entity = world.spawn();
    /// world.add(entity, Transform::default());
    /// world.add(entity, MeshRenderer::new(1));
    ///
    /// // Spawn camera
    /// let camera_entity = world.spawn();
    /// world.add(camera_entity, Transform::default());
    /// world.add(camera_entity, Camera::default());
    ///
    /// // Load mesh asset
    /// let assets = AssetManager::new();
    /// assets.meshes().insert(AssetId::from(1u64), MeshData::cube());
    ///
    /// // Render
    /// renderer.render_meshes(&world, &assets)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[instrument(skip(self, world, assets))]
    pub fn render_meshes(
        &mut self,
        world: &engine_core::World,
        assets: &engine_assets::AssetManager,
    ) -> Result<(), RendererError> {
        use engine_core::{Camera, MeshRenderer, Transform};

        // Clear previous frame's render queue
        self.render_queue.clear();

        // Find active camera (first camera with Transform)
        let mut camera_transform: Option<&Transform> = None;
        let mut camera_comp: Option<&Camera> = None;

        for entity in world.entities() {
            if let (Some(transform), Some(camera)) =
                (world.get::<Transform>(entity), world.get::<Camera>(entity))
            {
                camera_transform = Some(transform);
                camera_comp = Some(camera);
                break;
            }
        }

        // Use default camera if none found
        let default_transform = Transform::default();
        let default_camera = Camera::default();
        let cam_transform = camera_transform.unwrap_or(&default_transform);
        let cam = if let Some(c) = camera_comp {
            c
        } else {
            warn!("No camera found in world, using default");
            &default_camera
        };

        // Calculate view-projection matrix
        let view_matrix = cam.view_matrix(cam_transform);
        let proj_matrix = cam.projection_matrix_const();
        let vp_matrix = proj_matrix * view_matrix;

        // Query all renderable entities (Transform + MeshRenderer)
        for entity in world.entities() {
            if let (Some(transform), Some(mesh_renderer)) =
                (world.get::<Transform>(entity), world.get::<MeshRenderer>(entity))
            {
                // Skip invisible meshes
                if !mesh_renderer.is_visible() {
                    continue;
                }

                // Get mesh data from asset manager (mesh_id is u64)
                // Convert to AssetId using seed method (procedural generation path)
                let mesh_id =
                    engine_assets::AssetId::from_seed_and_params(mesh_renderer.mesh_id, b"mesh");

                // Upload mesh to GPU cache if not already cached
                if !self.gpu_cache.contains(mesh_id) {
                    let mesh_data = assets.get_mesh(mesh_id).ok_or_else(|| {
                        RendererError::invalidmeshdata(format!("Mesh not found: {:?}", mesh_id))
                    })?;
                    self.gpu_cache.upload_mesh(&self.context, mesh_id, &*mesh_data)?;
                }

                // Calculate MVP matrix (Model * View * Projection)
                let model_matrix = transform.matrix();
                let mvp_matrix = vp_matrix * model_matrix;

                // Add to render queue
                self.render_queue.push(MeshRenderCommand { mesh_id, mvp_matrix });
            }
        }

        info!(draw_count = self.render_queue.len(), "Queued meshes for rendering");
        Ok(())
    }

    /// Render a frame (clears to configured color)
    #[instrument(skip(self))]
    pub fn render_frame(&mut self) -> Result<(), RendererError> {
        let frame_start_time = std::time::Instant::now();

        let sync = &self.sync_objects[self.current_frame];

        // Wait for previous frame to finish
        unsafe {
            self.context
                .device
                .wait_for_fences(&[sync.in_flight_fence], true, u64::MAX)
                .map_err(|e| {
                    RendererError::queuesubmissionfailed(format!(
                        "Failed to wait for fence: {:?}",
                        e
                    ))
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
                .map_err(|e| {
                    let err = match e {
                        vk::Result::ERROR_OUT_OF_DATE_KHR => {
                            // Debug: Record swapchain recreation event
                            if self.debug_enabled {
                                if let Some(recorder) = &self.event_recorder {
                                    let (width, height) = self.window.size();
                                    recorder.record(debug::RenderEvent::SwapchainRecreated {
                                        frame: self.frame_counter,
                                        timestamp: frame_start_time.elapsed().as_secs_f64(),
                                        reason: "out of date".to_string(),
                                        old_width: width,
                                        old_height: height,
                                        new_width: width,
                                        new_height: height,
                                    });
                                }
                            }
                            RendererError::swapchainoutofdate()
                        }
                        _ => RendererError::swapchainacquisitionfailed(format!("{:?}", e)),
                    };

                    err
                })?
                .0 as usize
        };

        // Reset fence after acquiring image
        unsafe {
            self.context.device.reset_fences(&[sync.in_flight_fence]).map_err(|e| {
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
                .queue_submit(self.context.graphics_queue, &[submit_info], sync.in_flight_fence)
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

        // Debug: Capture frame snapshot and export
        if self.debug_enabled {
            let frame_time_ms = frame_start_time.elapsed().as_secs_f64() * 1000.0;

            // Create snapshot
            let snapshot = self.create_debug_snapshot(frame_time_ms);

            // Export to database if configured
            if let (Some(exporter), Some(snapshot)) = (&mut self.debug_exporter, snapshot) {
                if let Err(e) = exporter.write_snapshot(&snapshot) {
                    error!(
                        frame = self.frame_counter,
                        error = ?e,
                        "Failed to export debug snapshot"
                    );
                }

                // Export events to database
                if let Some(recorder) = &self.event_recorder {
                    let events = recorder.drain();
                    for event in events {
                        let event_type = match &event {
                            debug::RenderEvent::TextureCreated { .. } => "TextureCreated",
                            debug::RenderEvent::TextureDestroyed { .. } => "TextureDestroyed",
                            debug::RenderEvent::BufferCreated { .. } => "BufferCreated",
                            debug::RenderEvent::BufferDestroyed { .. } => "BufferDestroyed",
                            debug::RenderEvent::PipelineCreated { .. } => "PipelineCreated",
                            debug::RenderEvent::ShaderCompilationFailed { .. } => {
                                "ShaderCompilationFailed"
                            }
                            debug::RenderEvent::DrawCallSubmitted { .. } => "DrawCallSubmitted",
                            debug::RenderEvent::DrawCallFailed { .. } => "DrawCallFailed",
                            debug::RenderEvent::FenceWaitTimeout { .. } => "FenceWaitTimeout",
                            debug::RenderEvent::SwapchainRecreated { .. } => "SwapchainRecreated",
                            debug::RenderEvent::FrameDropped { .. } => "FrameDropped",
                            debug::RenderEvent::GpuMemoryExhausted { .. } => "GpuMemoryExhausted",
                        };
                        if let Err(e) = exporter.write_event(self.frame_counter, event_type, &event)
                        {
                            error!(error = ?e, event_type = event_type, "Failed to export debug event");
                        }
                    }
                }
            }

            // Check for frame drops (> 33ms = under 30 FPS)
            if frame_time_ms > 33.0 {
                if let Some(recorder) = &self.event_recorder {
                    recorder.record(debug::RenderEvent::FrameDropped {
                        expected_frame_time_ms: 16.67, // Target 60 FPS
                        actual_frame_time_ms: frame_time_ms as f32,
                        frame: self.frame_counter,
                        timestamp: frame_start_time.elapsed().as_secs_f64(),
                    });
                }
            }
        }

        // Advance to next frame
        self.current_frame = (self.current_frame + 1) % self.sync_objects.len();
        self.frame_counter += 1;

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

            // Begin render pass with color and depth clear
            let clear_values = [
                vk::ClearValue { color: vk::ClearColorValue { float32: self.clear_color } },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 },
                },
            ];

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

            // Render queued meshes (Phase 1.8)
            if !self.render_queue.is_empty() {
                if let Some(pipeline) = &self.mesh_pipeline {
                    // Bind pipeline
                    self.context.device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline.handle(),
                    );

                    // Set viewport and scissor
                    let viewport = vk::Viewport::default()
                        .x(0.0)
                        .y(0.0)
                        .width(self.swapchain.extent.width as f32)
                        .height(self.swapchain.extent.height as f32)
                        .min_depth(0.0)
                        .max_depth(1.0);

                    self.context.device.cmd_set_viewport(command_buffer, 0, &[viewport]);

                    let scissor = vk::Rect2D::default()
                        .offset(vk::Offset2D { x: 0, y: 0 })
                        .extent(self.swapchain.extent);

                    self.context.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

                    // Draw each mesh in queue
                    for cmd in &self.render_queue {
                        // Get mesh buffers from cache
                        if let Some((vertex_buffer, index_buffer)) =
                            self.gpu_cache.get_buffers(cmd.mesh_id)
                        {
                            if let Some(mesh_info) = self.gpu_cache.get_mesh_info(cmd.mesh_id) {
                                // Push MVP matrix (convert to bytes)
                                let mvp_bytes = cmd.mvp_matrix.as_ref();
                                let mvp_slice = std::slice::from_raw_parts(
                                    mvp_bytes.as_ptr() as *const u8,
                                    std::mem::size_of::<glam::Mat4>(),
                                );
                                self.context.device.cmd_push_constants(
                                    command_buffer,
                                    pipeline.layout(),
                                    vk::ShaderStageFlags::VERTEX,
                                    0,
                                    mvp_slice,
                                );

                                // Bind vertex buffer
                                self.context.device.cmd_bind_vertex_buffers(
                                    command_buffer,
                                    0,
                                    &[vertex_buffer],
                                    &[0],
                                );

                                // Bind index buffer
                                self.context.device.cmd_bind_index_buffer(
                                    command_buffer,
                                    index_buffer,
                                    0,
                                    vk::IndexType::UINT32,
                                );

                                // Draw indexed
                                self.context.device.cmd_draw_indexed(
                                    command_buffer,
                                    mesh_info.index_count,
                                    1,
                                    0,
                                    0,
                                    0,
                                );
                            }
                        }
                    }
                }
            }

            // End render pass
            self.context.device.cmd_end_render_pass(command_buffer);

            // End command buffer
            self.context.device.end_command_buffer(command_buffer).map_err(|e| {
                RendererError::commandbufferallocationfailed(
                    1,
                    format!("Failed to end command buffer: {:?}", e),
                )
            })?;
        }

        Ok(())
    }

    /// Create debug snapshot of current render state
    fn create_debug_snapshot(&self, frame_time_ms: f64) -> Option<debug::RenderDebugSnapshot> {
        if !self.debug_enabled {
            return None;
        }

        let (width, height) = self.window.size();

        // Build snapshot using constructor
        let mut snapshot = debug::RenderDebugSnapshot::new(
            self.frame_counter,
            frame_time_ms / 1000.0, // Convert ms to seconds
        );

        // Configure viewport
        snapshot.viewport = debug::snapshot::Viewport {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };

        // Configure scissor
        snapshot.scissor = debug::snapshot::Rect2D { x: 0, y: 0, width, height };

        // No draw calls yet (Phase 1.6 only clears)
        snapshot.draw_calls = vec![];

        // Textures: swapchain images
        snapshot.textures = self
            .swapchain
            .images
            .iter()
            .enumerate()
            .map(|(i, _image)| debug::TextureInfo {
                texture_id: i as u64,
                width,
                height,
                depth: 1,
                format: format!("{:?}", self.swapchain.format),
                mip_levels: 1,
                sample_count: 1,
                memory_size: (width * height * 4) as usize, // RGBA8
                created_frame: 0,
            })
            .collect();

        // Framebuffers
        snapshot.framebuffers = self
            .framebuffers
            .iter()
            .enumerate()
            .map(|(i, _fb)| debug::FramebufferInfo {
                framebuffer_id: i as u64,
                width,
                height,
                attachment_count: 1,
            })
            .collect();

        // Render targets
        snapshot.render_targets = vec![debug::RenderTargetInfo {
            attachment_index: 0,
            texture_id: 0, // Swapchain image
            format: format!("{:?}", self.swapchain.format),
            load_op: "clear".to_string(),
            store_op: "store".to_string(),
        }];

        // Queue state (using graphics queue as primary for now)
        snapshot.queue_states = vec![debug::QueueStateInfo {
            queue_family_index: self.context.queue_families.graphics,
            queue_index: 0,
            pending_commands: 0, // Would track actual submissions in production
            last_submit_timestamp: frame_time_ms / 1000.0,
        }];

        Some(snapshot)
    }

    /// Wait for device to finish all operations
    pub fn wait_idle(&self) -> Result<(), RendererError> {
        self.context.wait_idle()
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

        let (width, height) = self.window.size();
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

// Cleanup happens automatically via Drop implementations of contained types
