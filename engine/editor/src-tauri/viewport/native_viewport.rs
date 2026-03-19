//! Native child-window viewport for Vulkan rendering.
//!
//! Creates a platform-native child window parented inside the Tauri webview
//! window.  The child window is the target for a Vulkan surface; a render
//! thread draws into it at ~60 fps using the full `engine-renderer` pipeline.
//!
//! The render thread creates a Vulkan context, surface, swapchain, render pass,
//! depth buffer, framebuffers, command buffers, and sync objects from
//! `engine-renderer` types, then clears to the editor background colour each
//! frame with a grid overlay.  Swapchain is automatically recreated on resize.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Viewport bounds in physical (device) pixels.
#[derive(Clone, Copy, Debug)]
pub struct ViewportBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// ──────────────────────────────────────────────────────────────────────────────
// Windows implementation
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(windows)]
mod platform {
    use super::*;

    use windows::Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::HBRUSH,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    };

    /// Wrapper around HWND that is Send+Sync.
    ///
    /// HWND itself contains a raw pointer.  We only pass it to Win32 APIs
    /// that are safe to call from any thread (SetWindowPos, InvalidateRect,
    /// DestroyWindow, message pumping) so the Send impl is sound.
    #[derive(Clone, Copy)]
    struct SendHwnd(HWND);

    // SAFETY: We restrict usage to thread-safe Win32 calls.
    unsafe impl Send for SendHwnd {}
    unsafe impl Sync for SendHwnd {}

    /// State of the native Vulkan viewport (Windows).
    pub struct NativeViewport {
        child_hwnd: SendHwnd,
        renderer_thread: Option<std::thread::JoinHandle<()>>,
        should_stop: Arc<AtomicBool>,
        bounds: Arc<Mutex<ViewportBounds>>,
    }

    impl NativeViewport {
        /// Create a new child window parented to `parent_hwnd`.
        ///
        /// `parent_hwnd` is the HWND of the Tauri main window, obtained via
        /// `tauri::WebviewWindow::hwnd()`.
        pub fn new(parent_hwnd: HWND, bounds: ViewportBounds) -> Result<Self, String> {
            unsafe {
                let class_name = windows::core::w!("SilmarilViewport");
                let hinstance: HINSTANCE = GetModuleHandleW(None)
                    .map_err(|e| format!("GetModuleHandleW failed: {e}"))?
                    .into();

                let wc = WNDCLASSEXW {
                    cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                    style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
                    lpfnWndProc: Some(viewport_wnd_proc),
                    hInstance: hinstance,
                    lpszClassName: class_name,
                    hbrBackground: HBRUSH(std::ptr::null_mut()),
                    ..Default::default()
                };

                // RegisterClassExW returns 0 on failure *unless* the class
                // already exists (in which case the previous registration is
                // reused).  We ignore the return value intentionally.
                RegisterClassExW(&wc);

                let child = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    class_name,
                    windows::core::w!(""),
                    WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
                    bounds.x,
                    bounds.y,
                    bounds.width as i32,
                    bounds.height as i32,
                    Some(parent_hwnd),
                    None, // no menu
                    Some(hinstance),
                    None, // no extra param
                )
                .map_err(|e| format!("CreateWindowExW failed: {e}"))?;

                // Explicitly place our Vulkan child on top of sibling windows
                // (including WebView2's internal Chrome renderer).  WebView2
                // uses DirectComposition which can override default z-order,
                // so we must be explicit here.
                let _ = SetWindowPos(
                    child,
                    Some(HWND_TOP),
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                tracing::info!("Vulkan child window created and set to HWND_TOP");

                tracing::info!(
                    hwnd = ?child,
                    x = bounds.x,
                    y = bounds.y,
                    w = bounds.width,
                    h = bounds.height,
                    "Native viewport child window created"
                );

                Ok(Self {
                    child_hwnd: SendHwnd(child),
                    renderer_thread: None,
                    should_stop: Arc::new(AtomicBool::new(false)),
                    bounds: Arc::new(Mutex::new(bounds)),
                })
            }
        }

        /// Start the Vulkan render loop on a background thread.
        ///
        /// Initialises the engine-renderer pipeline on the child window and
        /// clears it to the editor background colour each frame (~60 fps).
        pub fn start_rendering(&mut self) -> Result<(), String> {
            let should_stop = self.should_stop.clone();
            let bounds = self.bounds.clone();
            // Extract the raw pointer as an integer so we can send it across
            // threads without triggering the `Send` check on `*mut c_void`.
            let hwnd_raw = self.child_hwnd.0 .0 as isize;

            let handle = std::thread::Builder::new()
                .name("viewport-render".into())
                .spawn(move || {
                    let hwnd = HWND(hwnd_raw as *mut _);
                    tracing::info!("Viewport render thread started");
                    render_loop(hwnd, should_stop, bounds);
                    tracing::info!("Viewport render thread stopped");
                })
                .map_err(|e| format!("Failed to spawn render thread: {e}"))?;

            self.renderer_thread = Some(handle);
            Ok(())
        }

        /// Reposition and resize the child window (called when the Svelte
        /// container's bounds change).
        pub fn set_bounds(&self, new_bounds: ViewportBounds) {
            *self.bounds.lock().unwrap() = new_bounds;

            unsafe {
                let _ = SetWindowPos(
                    self.child_hwnd.0,
                    None,
                    new_bounds.x,
                    new_bounds.y,
                    new_bounds.width as i32,
                    new_bounds.height as i32,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }

        /// Get the child HWND (for future Vulkan surface creation).
        #[allow(dead_code)]
        pub fn hwnd(&self) -> HWND {
            self.child_hwnd.0
        }

        /// Show or hide the child window. Used during drag operations
        /// to let the webview drop zone overlay be visible.
        pub fn set_visible(&self, visible: bool) {
            unsafe {
                let cmd = if visible { SW_SHOW } else { SW_HIDE };
                let _ = ShowWindow(self.child_hwnd.0, cmd);
            }
        }

        /// Stop the render thread and destroy the child window.
        pub fn destroy(&mut self) {
            self.should_stop.store(true, Ordering::Relaxed);
            if let Some(handle) = self.renderer_thread.take() {
                let _ = handle.join();
            }
            unsafe {
                let _ = DestroyWindow(self.child_hwnd.0);
            }
            tracing::info!("Native viewport destroyed");
        }
    }

    impl Drop for NativeViewport {
        fn drop(&mut self) {
            self.destroy();
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Engine-renderer based viewport state
    // ──────────────────────────────────────────────────────────────────────

    /// Clear colour for the viewport background: dark (#1a1a2e).
    const CLEAR_COLOR: [f32; 4] = [0.078, 0.078, 0.118, 1.0];

    /// Grid line colour: subtle (#2a2a3e).
    const GRID_COLOR: [f32; 4] = [0.133, 0.133, 0.200, 1.0];

    /// X-axis colour: red (#8b2020).
    const X_AXIS_COLOR: [f32; 4] = [0.545, 0.125, 0.125, 1.0];

    /// Y-axis colour: green (#208b20).
    const Y_AXIS_COLOR: [f32; 4] = [0.125, 0.545, 0.125, 1.0];

    /// Grid spacing in pixels.
    const GRID_SPACING: u32 = 50;

    /// Axis line thickness in pixels.
    const AXIS_THICKNESS: u32 = 2;

    use ash::vk;
    use engine_renderer::{
        CommandBuffer, CommandPool, DepthBuffer, Framebuffer, RenderPass, RenderPassConfig,
        Surface, Swapchain, VulkanContext,
    };

    /// Viewport renderer state backed by `engine-renderer` types.
    struct ViewportRenderer {
        context: VulkanContext,
        surface: Surface,
        swapchain: Swapchain,
        render_pass: RenderPass,
        depth_buffer: DepthBuffer,
        framebuffers: Vec<Framebuffer>,
        #[allow(dead_code)]
        command_pool: CommandPool,
        command_buffers: Vec<CommandBuffer>,
        sync_objects: Vec<engine_renderer::FrameSyncObjects>,
        current_frame: usize,
        width: u32,
        height: u32,
        needs_recreate: bool,
    }

    impl ViewportRenderer {
        /// Create a new viewport renderer from a raw HWND.
        fn new(hwnd: HWND, width: u32, height: u32) -> Result<Self, String> {
            let width = width.max(1);
            let height = height.max(1);
            let hwnd_raw = hwnd.0 as isize;

            // 1. Create Vulkan context (headless - no surface yet)
            let context = VulkanContext::new("SilmarilEditor", None, None)
                .map_err(|e| format!("VulkanContext creation failed: {e}"))?;

            // 2. Create surface from raw HWND using engine-renderer's Surface
            let surface = Surface::from_raw_hwnd(&context.entry, &context.instance, hwnd_raw)
                .map_err(|e| format!("Surface creation failed: {e}"))?;

            // 3. Create swapchain
            let swapchain = Swapchain::new(
                &context,
                surface.handle(),
                surface.loader(),
                width,
                height,
                None,
            )
            .map_err(|e| format!("Swapchain creation failed: {e}"))?;

            // 4. Create depth buffer
            let depth_buffer =
                DepthBuffer::new(&context.device, &context.allocator, swapchain.extent)
                    .map_err(|e| format!("DepthBuffer creation failed: {e}"))?;

            // 5. Create render pass (with depth)
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
            .map_err(|e| format!("RenderPass creation failed: {e}"))?;

            // 6. Create framebuffers (color + depth)
            let framebuffers = create_viewport_framebuffers(
                &context.device,
                &swapchain,
                &render_pass,
                &depth_buffer,
            )?;

            // 7. Create command pool + buffers
            const FRAMES_IN_FLIGHT: u32 = 2;

            let command_pool = CommandPool::new(
                &context.device,
                context.queue_families.graphics,
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            )
            .map_err(|e| format!("CommandPool creation failed: {e}"))?;

            let command_buffers = command_pool
                .allocate(
                    &context.device,
                    vk::CommandBufferLevel::PRIMARY,
                    FRAMES_IN_FLIGHT,
                )
                .map_err(|e| format!("CommandBuffer allocation failed: {e}"))?
                .into_iter()
                .map(CommandBuffer::from_handle)
                .collect();

            // 8. Create sync objects
            let sync_objects =
                engine_renderer::create_sync_objects(&context.device, FRAMES_IN_FLIGHT)
                    .map_err(|e| format!("Sync object creation failed: {e}"))?;

            tracing::info!(
                width,
                height,
                images = swapchain.image_count,
                "Viewport renderer initialised with engine-renderer"
            );

            Ok(Self {
                context,
                surface,
                swapchain,
                render_pass,
                depth_buffer,
                framebuffers,
                command_pool,
                command_buffers,
                sync_objects,
                current_frame: 0,
                width,
                height,
                needs_recreate: false,
            })
        }

        /// Notify the renderer that the viewport has been resized.
        fn notify_resize(&mut self, width: u32, height: u32) {
            let width = width.max(1);
            let height = height.max(1);
            if width != self.width || height != self.height {
                self.width = width;
                self.height = height;
                self.needs_recreate = true;
            }
        }

        /// Render a single frame (background clear + grid overlay).
        fn render_frame(&mut self) -> Result<bool, String> {
            if self.needs_recreate {
                self.recreate_swapchain()?;
                self.needs_recreate = false;
            }

            let sync = &self.sync_objects[self.current_frame];

            unsafe {
                // Wait for previous frame
                self.context
                    .device
                    .wait_for_fences(&[sync.in_flight_fence], true, u64::MAX)
                    .map_err(|e| format!("wait_for_fences: {e}"))?;

                // Acquire next image
                let acquire_result = self.swapchain.loader.acquire_next_image(
                    self.swapchain.swapchain,
                    u64::MAX,
                    sync.image_available_semaphore,
                    vk::Fence::null(),
                );

                let image_index = match acquire_result {
                    Ok((index, _suboptimal)) => index,
                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                        self.needs_recreate = true;
                        return Ok(false);
                    }
                    Err(e) => return Err(format!("acquire_next_image: {e}")),
                };

                self.context
                    .device
                    .reset_fences(&[sync.in_flight_fence])
                    .map_err(|e| format!("reset_fences: {e}"))?;

                // Record command buffer
                let cmd = self.command_buffers[self.current_frame].handle();
                self.record_frame_commands(cmd, image_index as usize)?;

                // Submit
                let wait_semaphores = [sync.image_available_semaphore];
                let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
                let signal_semaphores = [sync.render_finished_semaphore];
                let command_buffers = [cmd];

                let submit_info = vk::SubmitInfo::default()
                    .wait_semaphores(&wait_semaphores)
                    .wait_dst_stage_mask(&wait_stages)
                    .command_buffers(&command_buffers)
                    .signal_semaphores(&signal_semaphores);

                self.context
                    .device
                    .queue_submit(
                        self.context.graphics_queue,
                        &[submit_info],
                        sync.in_flight_fence,
                    )
                    .map_err(|e| format!("queue_submit: {e}"))?;

                // Present
                let swapchains = [self.swapchain.swapchain];
                let image_indices = [image_index];
                let present_info = vk::PresentInfoKHR::default()
                    .wait_semaphores(&signal_semaphores)
                    .swapchains(&swapchains)
                    .image_indices(&image_indices);

                match self
                    .swapchain
                    .loader
                    .queue_present(self.context.present_queue, &present_info)
                {
                    Ok(_suboptimal) => {}
                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR) => {
                        self.needs_recreate = true;
                    }
                    Err(e) => return Err(format!("queue_present: {e}")),
                }
            }

            self.current_frame = (self.current_frame + 1) % self.sync_objects.len();
            Ok(true)
        }

        /// Record frame commands: begin render pass, draw grid, end render pass.
        unsafe fn record_frame_commands(
            &self,
            cmd: vk::CommandBuffer,
            image_index: usize,
        ) -> Result<(), String> {
            self.context
                .device
                .reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
                .map_err(|e| format!("reset_command_buffer: {e}"))?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.context
                .device
                .begin_command_buffer(cmd, &begin_info)
                .map_err(|e| format!("begin_command_buffer: {e}"))?;

            // Begin render pass with color + depth clear
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: CLEAR_COLOR,
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass.handle())
                .framebuffer(self.framebuffers[image_index].handle())
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain.extent,
                })
                .clear_values(&clear_values);

            self.context.device.cmd_begin_render_pass(
                cmd,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );

            // Draw grid lines using ClearAttachments
            draw_grid(&self.context.device, cmd, self.swapchain.extent.width, self.swapchain.extent.height);

            self.context.device.cmd_end_render_pass(cmd);

            self.context
                .device
                .end_command_buffer(cmd)
                .map_err(|e| format!("end_command_buffer: {e}"))?;

            Ok(())
        }

        /// Recreate the swapchain and dependent resources after a resize.
        fn recreate_swapchain(&mut self) -> Result<(), String> {
            self.context
                .wait_idle()
                .map_err(|e| format!("device_wait_idle: {e}"))?;

            // Drop old framebuffers (they reference old image views + depth buffer)
            self.framebuffers.clear();

            // Recreate swapchain (destroys old image views internally)
            self.swapchain
                .recreate(
                    &self.context,
                    self.surface.handle(),
                    self.surface.loader(),
                    self.width,
                    self.height,
                )
                .map_err(|e| format!("Swapchain recreation failed: {e}"))?;

            // Recreate depth buffer for new extent
            self.depth_buffer =
                DepthBuffer::new(&self.context.device, &self.context.allocator, self.swapchain.extent)
                    .map_err(|e| format!("DepthBuffer recreation failed: {e}"))?;

            // Recreate framebuffers
            self.framebuffers = create_viewport_framebuffers(
                &self.context.device,
                &self.swapchain,
                &self.render_pass,
                &self.depth_buffer,
            )?;

            tracing::debug!(
                width = self.swapchain.extent.width,
                height = self.swapchain.extent.height,
                images = self.swapchain.images.len(),
                "Viewport swapchain recreated"
            );

            Ok(())
        }
    }

    impl Drop for ViewportRenderer {
        fn drop(&mut self) {
            // Wait for GPU to finish before cleanup
            let _ = self.context.wait_idle();

            // Drop in correct order: command buffers, framebuffers, depth buffer,
            // render pass, swapchain, surface, context
            self.command_buffers.clear();
            self.framebuffers.clear();

            // Remaining fields are dropped in reverse declaration order:
            // sync_objects, command_pool, depth_buffer, render_pass,
            // swapchain, surface, context
        }
    }

    /// Create framebuffers with color + depth attachments using engine-renderer types.
    fn create_viewport_framebuffers(
        device: &ash::Device,
        swapchain: &Swapchain,
        render_pass: &RenderPass,
        depth_buffer: &DepthBuffer,
    ) -> Result<Vec<Framebuffer>, String> {
        let mut framebuffers = Vec::with_capacity(swapchain.image_views.len());
        for (i, &image_view) in swapchain.image_views.iter().enumerate() {
            let attachments = [image_view, depth_buffer.image_view()];
            let framebuffer_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass.handle())
                .attachments(&attachments)
                .width(swapchain.extent.width)
                .height(swapchain.extent.height)
                .layers(1);

            let framebuffer = unsafe { device.create_framebuffer(&framebuffer_info, None) }
                .map_err(|e| format!("create_framebuffer[{i}]: {e}"))?;

            framebuffers.push(Framebuffer::from_raw(device, framebuffer));
        }
        Ok(framebuffers)
    }

    /// Draw grid lines and centre axes using `vkCmdClearAttachments`.
    ///
    /// This approach requires no shaders, pipelines, or vertex buffers --
    /// each grid line is a 1-pixel-wide clear rect, and each axis line
    /// is a 2-pixel-wide clear rect in a distinct colour.
    unsafe fn draw_grid(device: &ash::Device, cmd: vk::CommandBuffer, width: u32, height: u32) {
        // -- Minor grid lines (every GRID_SPACING pixels) --
        let grid_attachment = vk::ClearAttachment {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            color_attachment: 0,
            clear_value: vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: GRID_COLOR,
                },
            },
        };

        let mut grid_rects = Vec::new();

        // Vertical grid lines
        let mut x = 0u32;
        while x < width {
            grid_rects.push(vk::ClearRect {
                rect: vk::Rect2D {
                    offset: vk::Offset2D { x: x as i32, y: 0 },
                    extent: vk::Extent2D { width: 1, height },
                },
                base_array_layer: 0,
                layer_count: 1,
            });
            x += GRID_SPACING;
        }

        // Horizontal grid lines
        let mut y = 0u32;
        while y < height {
            grid_rects.push(vk::ClearRect {
                rect: vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: y as i32 },
                    extent: vk::Extent2D { width, height: 1 },
                },
                base_array_layer: 0,
                layer_count: 1,
            });
            y += GRID_SPACING;
        }

        if !grid_rects.is_empty() {
            device.cmd_clear_attachments(cmd, &[grid_attachment], &grid_rects);
        }

        // -- Centre X axis (horizontal line at height/2, red-tinted) --
        let center_y = height / 2;
        let x_axis_attachment = vk::ClearAttachment {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            color_attachment: 0,
            clear_value: vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: X_AXIS_COLOR,
                },
            },
        };
        let x_axis_h = AXIS_THICKNESS.min(height.saturating_sub(center_y));
        if x_axis_h > 0 {
            device.cmd_clear_attachments(
                cmd,
                &[x_axis_attachment],
                &[vk::ClearRect {
                    rect: vk::Rect2D {
                        offset: vk::Offset2D {
                            x: 0,
                            y: center_y as i32,
                        },
                        extent: vk::Extent2D {
                            width,
                            height: x_axis_h,
                        },
                    },
                    base_array_layer: 0,
                    layer_count: 1,
                }],
            );
        }

        // -- Centre Y axis (vertical line at width/2, green-tinted) --
        let center_x = width / 2;
        let y_axis_attachment = vk::ClearAttachment {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            color_attachment: 0,
            clear_value: vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: Y_AXIS_COLOR,
                },
            },
        };
        let y_axis_w = AXIS_THICKNESS.min(width.saturating_sub(center_x));
        if y_axis_w > 0 {
            device.cmd_clear_attachments(
                cmd,
                &[y_axis_attachment],
                &[vk::ClearRect {
                    rect: vk::Rect2D {
                        offset: vk::Offset2D {
                            x: center_x as i32,
                            y: 0,
                        },
                        extent: vk::Extent2D {
                            width: y_axis_w,
                            height,
                        },
                    },
                    base_array_layer: 0,
                    layer_count: 1,
                }],
            );
        }
    }

    /// Render loop: initialises the engine-renderer pipeline on the child HWND,
    /// then renders each frame.  Falls back to a no-op idle loop if
    /// initialisation fails.
    fn render_loop(
        hwnd: HWND,
        should_stop: Arc<AtomicBool>,
        bounds: Arc<Mutex<ViewportBounds>>,
    ) {
        let initial_bounds = *bounds.lock().unwrap();

        tracing::info!(
            width = initial_bounds.width,
            height = initial_bounds.height,
            "Render thread: initialising ViewportRenderer"
        );

        let mut renderer = match ViewportRenderer::new(
            hwnd,
            initial_bounds.width,
            initial_bounds.height,
        ) {
            Ok(r) => {
                tracing::info!("ViewportRenderer initialised successfully!");
                r
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to initialise engine-renderer for viewport; falling back to idle loop");
                // Fall back: just pump messages so the window stays alive
                while !should_stop.load(Ordering::Relaxed) {
                    unsafe {
                        let mut msg = std::mem::zeroed::<MSG>();
                        while PeekMessageW(&mut msg, Some(hwnd), 0, 0, PM_REMOVE).as_bool() {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(16));
                }
                return;
            }
        };

        let mut last_width = initial_bounds.width;
        let mut last_height = initial_bounds.height;
        let mut frame_counter: u64 = 0;

        while !should_stop.load(Ordering::Relaxed) {
            // Re-assert z-order every ~60 frames (~1s) to stay on top of
            // WebView2's DirectComposition layer.
            frame_counter += 1;
            if frame_counter % 60 == 0 {
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        Some(HWND_TOP),
                        0, 0, 0, 0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
            }
            // Check for resize
            {
                let b = bounds.lock().unwrap();
                if b.width != last_width || b.height != last_height {
                    last_width = b.width;
                    last_height = b.height;
                    renderer.notify_resize(last_width, last_height);
                }
            }

            // Render frame
            if let Err(e) = renderer.render_frame() {
                tracing::error!(error = %e, "Viewport render_frame failed");
                // Don't spin -- sleep before retrying
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }

            // Pump Win32 messages (non-blocking)
            unsafe {
                let mut msg = std::mem::zeroed::<MSG>();
                while PeekMessageW(&mut msg, Some(hwnd), 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            // ~60 fps
            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        // Explicit drop to ensure Vulkan cleanup before window destruction
        drop(renderer);
    }

    /// Window procedure for the child viewport window.
    ///
    /// Vulkan owns the rendering; the wndproc just handles WM_ERASEBKGND
    /// to prevent flicker and forwards everything else to DefWindowProcW.
    unsafe extern "system" fn viewport_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_ERASEBKGND => {
                // Prevent flicker -- Vulkan owns the surface
                LRESULT(1)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Platform-agnostic re-exports
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(windows)]
pub use platform::NativeViewport;

// Stub for non-Windows platforms (not yet implemented)
#[cfg(not(windows))]
pub struct NativeViewport;

#[cfg(not(windows))]
impl NativeViewport {
    pub fn new(_parent: isize, _bounds: ViewportBounds) -> Result<Self, String> {
        Err("Native viewport not yet implemented for this platform".into())
    }

    pub fn start_rendering(&mut self) -> Result<(), String> {
        Err("Native viewport not yet implemented for this platform".into())
    }

    pub fn set_bounds(&self, _bounds: ViewportBounds) {}

    pub fn set_visible(&self, _visible: bool) {}

    pub fn destroy(&mut self) {}
}
