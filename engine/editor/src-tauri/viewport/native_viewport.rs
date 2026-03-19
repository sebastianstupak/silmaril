//! Native child-window viewport for Vulkan rendering.
//!
//! Creates a platform-native child window parented inside the Tauri webview
//! window.  The child window is the target for a Vulkan surface; a render
//! thread draws into it at ~60 fps using the full `engine-renderer` pipeline.
//!
//! Features:
//! - 3D orbit camera (right-drag=orbit, middle-drag=pan, scroll=zoom)
//! - Infinite-style ground grid on the XZ plane via vertex shaders
//! - Push-constant MVP matrix
//! - GLSL→SPIR-V compilation at runtime via `naga`

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
        UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture},
        UI::WindowsAndMessaging::*,
    };

    /// Wrapper around HWND that is Send+Sync.
    #[derive(Clone, Copy)]
    struct SendHwnd(HWND);
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
                    None,
                    Some(hinstance),
                    None,
                )
                .map_err(|e| format!("CreateWindowExW failed: {e}"))?;

                let _ = SetWindowPos(
                    child,
                    Some(HWND_TOP),
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );

                // Store camera state pointer in the window's user data so
                // the wndproc can access it.
                let camera = Box::new(Mutex::new(OrbitCamera::default()));
                let camera_ptr = Box::into_raw(camera);
                SetWindowLongPtrW(child, GWLP_USERDATA, camera_ptr as isize);

                tracing::info!(
                    hwnd = ?child,
                    x = bounds.x, y = bounds.y,
                    w = bounds.width, h = bounds.height,
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

        pub fn start_rendering(&mut self) -> Result<(), String> {
            let should_stop = self.should_stop.clone();
            let bounds = self.bounds.clone();
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

        #[allow(dead_code)]
        pub fn hwnd(&self) -> HWND {
            self.child_hwnd.0
        }

        pub fn set_visible(&self, visible: bool) {
            unsafe {
                let cmd = if visible { SW_SHOW } else { SW_HIDE };
                let _ = ShowWindow(self.child_hwnd.0, cmd);
            }
        }

        pub fn destroy(&mut self) {
            self.should_stop.store(true, Ordering::Relaxed);
            if let Some(handle) = self.renderer_thread.take() {
                let _ = handle.join();
            }
            unsafe {
                // Free the camera state stored in GWLP_USERDATA
                let ptr = GetWindowLongPtrW(self.child_hwnd.0, GWLP_USERDATA) as *mut Mutex<OrbitCamera>;
                if !ptr.is_null() {
                    drop(Box::from_raw(ptr));
                    SetWindowLongPtrW(self.child_hwnd.0, GWLP_USERDATA, 0);
                }
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
    // 3D Orbit Camera
    // ──────────────────────────────────────────────────────────────────────

    use glam::{Mat4, Vec3};

    /// Orbit camera that rotates around a target point.
    #[derive(Clone, Debug)]
    struct OrbitCamera {
        /// Point the camera orbits around
        target: Vec3,
        /// Distance from target
        distance: f32,
        /// Horizontal angle in radians
        yaw: f32,
        /// Vertical angle in radians (clamped to avoid gimbal lock)
        pitch: f32,
        /// Vertical field of view in radians
        fov_y: f32,
        /// Near clip plane
        near: f32,
        /// Far clip plane
        far: f32,
    }

    impl Default for OrbitCamera {
        fn default() -> Self {
            Self {
                target: Vec3::ZERO,
                distance: 10.0,
                yaw: std::f32::consts::FRAC_PI_4,        // 45°
                pitch: std::f32::consts::FRAC_PI_6,       // 30°
                fov_y: std::f32::consts::FRAC_PI_4,       // 45° fov
                near: 0.1,
                far: 500.0,
            }
        }
    }

    impl OrbitCamera {
        fn eye(&self) -> Vec3 {
            let cp = self.pitch.cos();
            let sp = self.pitch.sin();
            let cy = self.yaw.cos();
            let sy = self.yaw.sin();
            self.target + self.distance * Vec3::new(cp * sy, sp, cp * cy)
        }

        fn view_matrix(&self) -> Mat4 {
            Mat4::look_at_rh(self.eye(), self.target, Vec3::Y)
        }

        fn projection_matrix(&self, aspect: f32) -> Mat4 {
            Mat4::perspective_rh(self.fov_y, aspect, self.near, self.far)
        }

        fn view_projection(&self, aspect: f32) -> Mat4 {
            self.projection_matrix(aspect) * self.view_matrix()
        }

        fn orbit(&mut self, dx: f32, dy: f32) {
            self.yaw -= dx * 0.005;
            self.pitch += dy * 0.005;
            // Clamp pitch to avoid flipping
            self.pitch = self.pitch.clamp(-1.5, 1.5);
        }

        fn pan(&mut self, dx: f32, dy: f32) {
            let view = self.view_matrix();
            let right = Vec3::new(view.col(0).x, view.col(1).x, view.col(2).x);
            let up = Vec3::new(view.col(0).y, view.col(1).y, view.col(2).y);
            let scale = self.distance * 0.002;
            self.target -= right * dx * scale;
            self.target += up * dy * scale;
        }

        fn zoom(&mut self, delta: f32) {
            self.distance *= 1.0 - delta * 0.001;
            self.distance = self.distance.clamp(0.5, 200.0);
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Shader compilation (GLSL → SPIR-V via naga)
    // ──────────────────────────────────────────────────────────────────────

    /// Compile a GLSL shader string to SPIR-V u32 words using naga.
    fn compile_glsl_to_spirv(source: &str, stage: naga::ShaderStage) -> Result<Vec<u32>, String> {
        use naga::back::spv;
        use naga::front::glsl;
        use naga::valid::{Capabilities, ValidationFlags, Validator};

        let mut frontend = glsl::Frontend::default();
        let options = glsl::Options::from(stage);

        let module = frontend
            .parse(&options, source)
            .map_err(|errs| format!("GLSL parse errors: {:?}", errs))?;

        let info = Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .map_err(|e| format!("Shader validation error: {e}"))?;

        let options = spv::Options {
            lang_version: (1, 0),
            ..Default::default()
        };

        let spirv = spv::write_vec(&module, &info, &options, None)
            .map_err(|e| format!("SPIR-V generation error: {e}"))?;

        Ok(spirv)
    }

    const GRID_VERT_GLSL: &str = r#"#version 450

layout(push_constant) uniform PushConstants {
    mat4 viewProj;
} pc;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;

layout(location = 0) out vec3 fragColor;

void main() {
    gl_Position = pc.viewProj * vec4(inPosition, 1.0);
    fragColor = inColor;
}
"#;

    const GRID_FRAG_GLSL: &str = r#"#version 450

layout(location = 0) in vec3 fragColor;
layout(location = 0) out vec4 outColor;

void main() {
    outColor = vec4(fragColor, 1.0);
}
"#;

    // ──────────────────────────────────────────────────────────────────────
    // Grid geometry generation
    // ──────────────────────────────────────────────────────────────────────

    /// Vertex with position and color.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct GridVertex {
        pos: [f32; 3],
        col: [f32; 3],
    }

    /// Generate a grid of lines on the XZ plane centered at the origin.
    fn generate_grid_vertices(half_extent: i32, spacing: f32) -> Vec<GridVertex> {
        let grid_color = [0.25, 0.25, 0.30];
        let x_axis_color = [0.7, 0.15, 0.15];
        let z_axis_color = [0.15, 0.15, 0.7];
        let y_axis_color = [0.15, 0.7, 0.15];

        let mut verts = Vec::new();
        let extent = half_extent as f32 * spacing;

        // Grid lines parallel to Z (varying X)
        for i in -half_extent..=half_extent {
            let x = i as f32 * spacing;
            let col = if i == 0 { z_axis_color } else { grid_color };
            verts.push(GridVertex { pos: [x, 0.0, -extent], col });
            verts.push(GridVertex { pos: [x, 0.0,  extent], col });
        }

        // Grid lines parallel to X (varying Z)
        for i in -half_extent..=half_extent {
            let z = i as f32 * spacing;
            let col = if i == 0 { x_axis_color } else { grid_color };
            verts.push(GridVertex { pos: [-extent, 0.0, z], col });
            verts.push(GridVertex { pos: [ extent, 0.0, z], col });
        }

        // Y axis (vertical green line at origin)
        verts.push(GridVertex { pos: [0.0, 0.0, 0.0], col: y_axis_color });
        verts.push(GridVertex { pos: [0.0, extent, 0.0], col: y_axis_color });

        verts
    }

    // ──────────────────────────────────────────────────────────────────────
    // Engine-renderer based viewport state
    // ──────────────────────────────────────────────────────────────────────

    /// Clear colour for the viewport background: dark (#1a1a2e).
    const CLEAR_COLOR: [f32; 4] = [0.078, 0.078, 0.118, 1.0];

    use ash::vk;
    use engine_renderer::{
        CommandBuffer, CommandPool, DepthBuffer, Framebuffer, GpuBuffer, RenderPass,
        RenderPassConfig, ShaderModule, Surface, Swapchain, VulkanContext,
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
        // 3D grid pipeline
        grid_pipeline: vk::Pipeline,
        grid_pipeline_layout: vk::PipelineLayout,
        grid_vertex_buffer: GpuBuffer,
        grid_vertex_count: u32,
        // Shader modules kept alive for pipeline lifetime
        _vert_shader: ShaderModule,
        _frag_shader: ShaderModule,
    }

    impl ViewportRenderer {
        fn new(hwnd: HWND, width: u32, height: u32) -> Result<Self, String> {
            let width = width.max(1);
            let height = height.max(1);
            let hwnd_raw = hwnd.0 as isize;

            let context = VulkanContext::new("SilmarilEditor", None, None)
                .map_err(|e| format!("VulkanContext creation failed: {e}"))?;

            let surface = Surface::from_raw_hwnd(&context.entry, &context.instance, hwnd_raw)
                .map_err(|e| format!("Surface creation failed: {e}"))?;

            let swapchain = Swapchain::new(
                &context, surface.handle(), surface.loader(),
                width, height, None,
            ).map_err(|e| format!("Swapchain creation failed: {e}"))?;

            let depth_buffer = DepthBuffer::new(&context.device, &context.allocator, swapchain.extent)
                .map_err(|e| format!("DepthBuffer creation failed: {e}"))?;

            let render_pass = RenderPass::new(
                &context.device,
                RenderPassConfig {
                    color_format: swapchain.format,
                    depth_format: Some(depth_buffer.format()),
                    samples: vk::SampleCountFlags::TYPE_1,
                    load_op: vk::AttachmentLoadOp::CLEAR,
                    store_op: vk::AttachmentStoreOp::STORE,
                },
            ).map_err(|e| format!("RenderPass creation failed: {e}"))?;

            let framebuffers = create_viewport_framebuffers(
                &context.device, &swapchain, &render_pass, &depth_buffer,
            )?;

            const FRAMES_IN_FLIGHT: u32 = 2;

            let command_pool = CommandPool::new(
                &context.device,
                context.queue_families.graphics,
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            ).map_err(|e| format!("CommandPool creation failed: {e}"))?;

            let command_buffers = command_pool
                .allocate(&context.device, vk::CommandBufferLevel::PRIMARY, FRAMES_IN_FLIGHT)
                .map_err(|e| format!("CommandBuffer allocation failed: {e}"))?
                .into_iter()
                .map(CommandBuffer::from_handle)
                .collect();

            let sync_objects =
                engine_renderer::create_sync_objects(&context.device, FRAMES_IN_FLIGHT)
                    .map_err(|e| format!("Sync object creation failed: {e}"))?;

            // --- Compile shaders via naga ---
            tracing::info!("Compiling grid shaders via naga");
            let vert_spirv = compile_glsl_to_spirv(GRID_VERT_GLSL, naga::ShaderStage::Vertex)?;
            let frag_spirv = compile_glsl_to_spirv(GRID_FRAG_GLSL, naga::ShaderStage::Fragment)?;

            let vert_shader = ShaderModule::from_spirv(
                &context.device, &vert_spirv, vk::ShaderStageFlags::VERTEX, "main",
            ).map_err(|e| format!("Vertex shader creation failed: {e}"))?;
            let frag_shader = ShaderModule::from_spirv(
                &context.device, &frag_spirv, vk::ShaderStageFlags::FRAGMENT, "main",
            ).map_err(|e| format!("Fragment shader creation failed: {e}"))?;

            // --- Create grid pipeline ---
            let (grid_pipeline, grid_pipeline_layout) = create_grid_pipeline(
                &context.device, &render_pass, &vert_shader, &frag_shader,
            )?;

            // --- Generate grid vertex buffer ---
            let grid_verts = generate_grid_vertices(20, 1.0);
            let grid_vertex_count = grid_verts.len() as u32;
            let buf_size = (grid_verts.len() * std::mem::size_of::<GridVertex>()) as u64;

            let mut grid_vertex_buffer = GpuBuffer::new(
                &context, buf_size,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                gpu_allocator::MemoryLocation::CpuToGpu,
            ).map_err(|e| format!("Grid vertex buffer creation failed: {e}"))?;
            grid_vertex_buffer.upload(&grid_verts)
                .map_err(|e| format!("Grid vertex upload failed: {e}"))?;

            tracing::info!(
                width, height,
                images = swapchain.image_count,
                grid_verts = grid_vertex_count,
                "Viewport renderer initialised (3D mode)"
            );

            Ok(Self {
                context, surface, swapchain, render_pass, depth_buffer,
                framebuffers, command_pool, command_buffers, sync_objects,
                current_frame: 0, width, height, needs_recreate: false,
                grid_pipeline, grid_pipeline_layout, grid_vertex_buffer,
                grid_vertex_count,
                _vert_shader: vert_shader, _frag_shader: frag_shader,
            })
        }

        fn notify_resize(&mut self, width: u32, height: u32) {
            let width = width.max(1);
            let height = height.max(1);
            if width != self.width || height != self.height {
                self.width = width;
                self.height = height;
                self.needs_recreate = true;
            }
        }

        fn render_frame(&mut self, camera: &OrbitCamera) -> Result<bool, String> {
            if self.needs_recreate {
                self.recreate_swapchain()?;
                self.needs_recreate = false;
            }

            let sync = &self.sync_objects[self.current_frame];

            unsafe {
                self.context.device
                    .wait_for_fences(&[sync.in_flight_fence], true, u64::MAX)
                    .map_err(|e| format!("wait_for_fences: {e}"))?;

                let acquire_result = self.swapchain.loader.acquire_next_image(
                    self.swapchain.swapchain, u64::MAX,
                    sync.image_available_semaphore, vk::Fence::null(),
                );

                let image_index = match acquire_result {
                    Ok((index, _)) => index,
                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                        self.needs_recreate = true;
                        return Ok(false);
                    }
                    Err(e) => return Err(format!("acquire_next_image: {e}")),
                };

                self.context.device
                    .reset_fences(&[sync.in_flight_fence])
                    .map_err(|e| format!("reset_fences: {e}"))?;

                let cmd = self.command_buffers[self.current_frame].handle();
                self.record_frame_commands(cmd, image_index as usize, camera)?;

                let wait_semaphores = [sync.image_available_semaphore];
                let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
                let signal_semaphores = [sync.render_finished_semaphore];
                let command_buffers = [cmd];

                let submit_info = vk::SubmitInfo::default()
                    .wait_semaphores(&wait_semaphores)
                    .wait_dst_stage_mask(&wait_stages)
                    .command_buffers(&command_buffers)
                    .signal_semaphores(&signal_semaphores);

                self.context.device
                    .queue_submit(self.context.graphics_queue, &[submit_info], sync.in_flight_fence)
                    .map_err(|e| format!("queue_submit: {e}"))?;

                let swapchains = [self.swapchain.swapchain];
                let image_indices = [image_index];
                let present_info = vk::PresentInfoKHR::default()
                    .wait_semaphores(&signal_semaphores)
                    .swapchains(&swapchains)
                    .image_indices(&image_indices);

                match self.swapchain.loader.queue_present(self.context.present_queue, &present_info) {
                    Ok(_) => {}
                    Err(vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR) => {
                        self.needs_recreate = true;
                    }
                    Err(e) => return Err(format!("queue_present: {e}")),
                }
            }

            self.current_frame = (self.current_frame + 1) % self.sync_objects.len();
            Ok(true)
        }

        unsafe fn record_frame_commands(
            &self,
            cmd: vk::CommandBuffer,
            image_index: usize,
            camera: &OrbitCamera,
        ) -> Result<(), String> {
            let device = &self.context.device;

            device.reset_command_buffer(cmd, vk::CommandBufferResetFlags::empty())
                .map_err(|e| format!("reset_command_buffer: {e}"))?;

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.begin_command_buffer(cmd, &begin_info)
                .map_err(|e| format!("begin_command_buffer: {e}"))?;

            let clear_values = [
                vk::ClearValue { color: vk::ClearColorValue { float32: CLEAR_COLOR } },
                vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } },
            ];

            let extent = self.swapchain.extent;
            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass.handle())
                .framebuffer(self.framebuffers[image_index].handle())
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                })
                .clear_values(&clear_values);

            device.cmd_begin_render_pass(cmd, &render_pass_info, vk::SubpassContents::INLINE);

            // Set dynamic viewport/scissor
            let viewport = vk::Viewport {
                x: 0.0, y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0, max_depth: 1.0,
            };
            device.cmd_set_viewport(cmd, 0, &[viewport]);
            device.cmd_set_scissor(cmd, 0, &[vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            }]);

            // Bind grid pipeline and draw
            let aspect = extent.width as f32 / extent.height.max(1) as f32;
            let vp = camera.view_projection(aspect);
            let vp_bytes: &[u8] = std::slice::from_raw_parts(
                vp.as_ref().as_ptr() as *const u8, 64,
            );

            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.grid_pipeline);
            device.cmd_push_constants(
                cmd, self.grid_pipeline_layout,
                vk::ShaderStageFlags::VERTEX, 0, vp_bytes,
            );
            device.cmd_bind_vertex_buffers(cmd, 0, &[self.grid_vertex_buffer.handle()], &[0]);
            device.cmd_draw(cmd, self.grid_vertex_count, 1, 0, 0);

            device.cmd_end_render_pass(cmd);
            device.end_command_buffer(cmd)
                .map_err(|e| format!("end_command_buffer: {e}"))?;

            Ok(())
        }

        fn recreate_swapchain(&mut self) -> Result<(), String> {
            self.context.wait_idle().map_err(|e| format!("device_wait_idle: {e}"))?;

            self.framebuffers.clear();

            self.swapchain.recreate(
                &self.context, self.surface.handle(), self.surface.loader(),
                self.width, self.height,
            ).map_err(|e| format!("Swapchain recreation failed: {e}"))?;

            self.depth_buffer = DepthBuffer::new(
                &self.context.device, &self.context.allocator, self.swapchain.extent,
            ).map_err(|e| format!("DepthBuffer recreation failed: {e}"))?;

            self.framebuffers = create_viewport_framebuffers(
                &self.context.device, &self.swapchain, &self.render_pass, &self.depth_buffer,
            )?;

            tracing::debug!(
                width = self.swapchain.extent.width,
                height = self.swapchain.extent.height,
                "Viewport swapchain recreated"
            );
            Ok(())
        }
    }

    impl Drop for ViewportRenderer {
        fn drop(&mut self) {
            let _ = self.context.wait_idle();
            unsafe {
                self.context.device.destroy_pipeline(self.grid_pipeline, None);
                self.context.device.destroy_pipeline_layout(self.grid_pipeline_layout, None);
            }
            self.command_buffers.clear();
            self.framebuffers.clear();
        }
    }

    /// Create the grid line-drawing pipeline.
    fn create_grid_pipeline(
        device: &ash::Device,
        render_pass: &RenderPass,
        vert: &ShaderModule,
        frag: &ShaderModule,
    ) -> Result<(vk::Pipeline, vk::PipelineLayout), String> {
        let stages = [vert.stage_create_info(), frag.stage_create_info()];

        // Vertex input: vec3 pos + vec3 color = 24 bytes
        let binding = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(24)
            .input_rate(vk::VertexInputRate::VERTEX);

        let attrs = [
            vk::VertexInputAttributeDescription::default()
                .location(0).binding(0)
                .format(vk::Format::R32G32B32_SFLOAT).offset(0),
            vk::VertexInputAttributeDescription::default()
                .location(1).binding(0)
                .format(vk::Format::R32G32B32_SFLOAT).offset(12),
        ];

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&binding))
            .vertex_attribute_descriptions(&attrs);

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::LINE_LIST);

        let viewport = vk::Viewport::default()
            .width(1.0).height(1.0).max_depth(1.0);
        let scissor = vk::Rect2D::default().extent(vk::Extent2D { width: 1, height: 1 });
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(std::slice::from_ref(&viewport))
            .scissors(std::slice::from_ref(&scissor));

        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);

        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA);

        let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(std::slice::from_ref(&color_blend_attachment));

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
            .dynamic_states(&dynamic_states);

        // Push constant: mat4 viewProj = 64 bytes
        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0).size(64);

        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .push_constant_ranges(std::slice::from_ref(&push_range));

        let layout = unsafe {
            device.create_pipeline_layout(&layout_info, None)
                .map_err(|e| format!("Pipeline layout creation failed: {e}"))?
        };

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization)
            .multisample_state(&multisample)
            .depth_stencil_state(&depth_stencil)
            .color_blend_state(&color_blend)
            .dynamic_state(&dynamic_state)
            .layout(layout)
            .render_pass(render_pass.handle())
            .subpass(0);

        let pipeline = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&pipeline_info),
                None,
            ).map_err(|(_, e)| {
                device.destroy_pipeline_layout(layout, None);
                format!("Grid pipeline creation failed: {e}")
            })?[0]
        };

        tracing::info!("Grid pipeline created");
        Ok((pipeline, layout))
    }

    fn create_viewport_framebuffers(
        device: &ash::Device,
        swapchain: &Swapchain,
        render_pass: &RenderPass,
        depth_buffer: &DepthBuffer,
    ) -> Result<Vec<Framebuffer>, String> {
        let mut framebuffers = Vec::with_capacity(swapchain.image_views.len());
        for (i, &image_view) in swapchain.image_views.iter().enumerate() {
            let attachments = [image_view, depth_buffer.image_view()];
            let info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass.handle())
                .attachments(&attachments)
                .width(swapchain.extent.width)
                .height(swapchain.extent.height)
                .layers(1);
            let fb = unsafe { device.create_framebuffer(&info, None) }
                .map_err(|e| format!("create_framebuffer[{i}]: {e}"))?;
            framebuffers.push(Framebuffer::from_raw(device, fb));
        }
        Ok(framebuffers)
    }

    // ──────────────────────────────────────────────────────────────────────
    // Render loop + mouse input
    // ──────────────────────────────────────────────────────────────────────

    /// Mouse state tracked across WM_* messages via the camera mutex.
    struct MouseState {
        dragging: bool,
        button: u32, // 0=left, 1=middle, 2=right
        last_x: i32,
        last_y: i32,
    }

    static MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState {
        dragging: false,
        button: 0,
        last_x: 0,
        last_y: 0,
    });

    fn render_loop(
        hwnd: HWND,
        should_stop: Arc<AtomicBool>,
        bounds: Arc<Mutex<ViewportBounds>>,
    ) {
        let initial_bounds = *bounds.lock().unwrap();

        tracing::info!(
            width = initial_bounds.width,
            height = initial_bounds.height,
            "Render thread: initialising ViewportRenderer (3D)"
        );

        let mut renderer = match ViewportRenderer::new(hwnd, initial_bounds.width, initial_bounds.height) {
            Ok(r) => {
                tracing::info!("ViewportRenderer initialised successfully (3D)!");
                r
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to initialise viewport renderer");
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
            // Re-assert z-order every ~60 frames
            frame_counter += 1;
            if frame_counter % 60 == 0 {
                unsafe {
                    let _ = SetWindowPos(
                        hwnd, Some(HWND_TOP),
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

            // Read camera from the window's user data
            let camera_snapshot = unsafe {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<OrbitCamera>;
                if !ptr.is_null() {
                    (*ptr).lock().unwrap().clone()
                } else {
                    OrbitCamera::default()
                }
            };

            if let Err(e) = renderer.render_frame(&camera_snapshot) {
                tracing::error!(error = %e, "Viewport render_frame failed");
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }

            // Pump Win32 messages
            unsafe {
                let mut msg = std::mem::zeroed::<MSG>();
                while PeekMessageW(&mut msg, Some(hwnd), 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        drop(renderer);
    }

    /// Window procedure — handles mouse input for 3D camera control.
    unsafe extern "system" fn viewport_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let lo = (lparam.0 & 0xFFFF) as i16 as i32;
        let hi = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

        match msg {
            WM_ERASEBKGND => LRESULT(1),

            WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_LBUTTONDOWN => {
                let button = match msg {
                    WM_LBUTTONDOWN => 0,
                    WM_MBUTTONDOWN => 1,
                    WM_RBUTTONDOWN => 2,
                    _ => 0,
                };
                let _ = SetCapture(hwnd);
                if let Ok(mut ms) = MOUSE_STATE.lock() {
                    ms.dragging = true;
                    ms.button = button;
                    ms.last_x = lo;
                    ms.last_y = hi;
                }
                LRESULT(0)
            }

            WM_RBUTTONUP | WM_MBUTTONUP | WM_LBUTTONUP => {
                let _ = ReleaseCapture();
                if let Ok(mut ms) = MOUSE_STATE.lock() {
                    ms.dragging = false;
                }
                LRESULT(0)
            }

            WM_MOUSEMOVE => {
                let (dragging, button, last_x, last_y) = {
                    let ms = MOUSE_STATE.lock().unwrap();
                    (ms.dragging, ms.button, ms.last_x, ms.last_y)
                };

                if dragging {
                    let dx = lo - last_x;
                    let dy = hi - last_y;

                    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<OrbitCamera>;
                    if !ptr.is_null() {
                        if let Ok(mut cam) = (*ptr).lock() {
                            match button {
                                2 => cam.orbit(dx as f32, dy as f32),   // Right = orbit
                                1 => cam.pan(dx as f32, dy as f32),     // Middle = pan
                                0 => cam.orbit(dx as f32, dy as f32),   // Left = orbit too (for now)
                                _ => {}
                            }
                        }
                    }

                    if let Ok(mut ms) = MOUSE_STATE.lock() {
                        ms.last_x = lo;
                        ms.last_y = hi;
                    }
                }
                LRESULT(0)
            }

            WM_MOUSEWHEEL => {
                let delta = ((wparam.0 >> 16) & 0xFFFF) as i16 as f32;
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<OrbitCamera>;
                if !ptr.is_null() {
                    if let Ok(mut cam) = (*ptr).lock() {
                        cam.zoom(delta);
                    }
                }
                LRESULT(0)
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
