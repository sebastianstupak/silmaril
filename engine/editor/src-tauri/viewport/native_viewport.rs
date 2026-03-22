//! Native viewport for Vulkan rendering — parent-HWND surface approach.
//!
//! One `NativeViewport` is created per OS window (HWND).  It owns the single
//! Vulkan surface + swapchain for that window and renders **all** active
//! viewport panel instances in one frame (each with its own scissor rect and
//! orbit camera).  This lets you have duplicate viewport panels in the same
//! window and pop-out viewports in separate windows without Vulkan surface
//! conflicts.
//!
//! Architecture (bottom → top in DWM composition):
//!   1. Parent HWND — Vulkan DXGI swapchain covers the full window.
//!   2. WRY_WEBVIEW child — WebView2 UI on top.
//!      Opaque panels (Hierarchy/Inspector/Console) hide Vulkan below.
//!      Transparent `.viewport-container` CSS lets Vulkan show through.
//!      Dropdowns, HUD, overlays all render in this layer — always on top.
//!
//! Requirements on the parent HWND (set in lib.rs):
//!   - `WS_CLIPCHILDREN` must be REMOVED.
//!   - `transparent: true` in tauri.conf.json.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Viewport bounds in physical (device) pixels — position within the parent
/// window client area, used as the Vulkan scissor/viewport rect.
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
        Foundation::{HWND, RECT},
        UI::WindowsAndMessaging::GetClientRect,
    };

    /// Wrapper around HWND that is Send+Sync.
    #[derive(Clone, Copy)]
    struct SendHwnd(HWND);
    unsafe impl Send for SendHwnd {}
    unsafe impl Sync for SendHwnd {}

    // ──────────────────────────────────────────────────────────────────────
    // Per-instance state (one per viewport panel)
    // ──────────────────────────────────────────────────────────────────────

    #[derive(Clone)]
    struct ViewportInstance {
        bounds: ViewportBounds,
        camera: OrbitCamera,
        visible: bool,
        grid_visible: bool,
        is_ortho: bool,
    }

    impl ViewportInstance {
        fn new(bounds: ViewportBounds) -> Self {
            Self {
                bounds,
                camera: OrbitCamera::default(),
                visible: true,
                grid_visible: true,
                is_ortho: false,
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // NativeViewport — one per OS window
    // ──────────────────────────────────────────────────────────────────────

    /// Manages the Vulkan surface for one OS window, shared by all viewport
    /// panel instances within that window.
    pub struct NativeViewport {
        parent_hwnd: SendHwnd,
        /// All active viewport panel instances in this window.
        instances: Arc<Mutex<HashMap<String, ViewportInstance>>>,
        renderer_thread: Option<std::thread::JoinHandle<()>>,
        should_stop: Arc<AtomicBool>,
        render_active: Arc<AtomicBool>,
        /// Shared ECS world for reading entity transforms during rendering.
        world: Arc<std::sync::RwLock<engine_core::World>>,
        /// Screenshot request slot — main thread places a reply sender here;
        /// the render thread takes it, captures the frame, and sends back PNG bytes.
        screenshot_slot: Arc<Mutex<Option<std::sync::mpsc::SyncSender<Result<Vec<u8>, String>>>>>,
        /// Currently selected entity ID, shared with the render thread.
        selected_entity_id: std::sync::Arc<std::sync::Mutex<Option<u64>>>,
        /// Current gizmo mode (0=Move, 1=Rotate, 2=Scale), shared with the render thread.
        gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
        /// Which gizmo axis is currently hovered (0 = none, 1..=6 = axes), shared with the render thread.
        hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
        /// Asset manager shared with the render thread for mesh GPU upload.
        asset_manager: Arc<engine_assets::AssetManager>,
    }

    impl NativeViewport {
        pub fn new(
            parent_hwnd: HWND,
            world: Arc<std::sync::RwLock<engine_core::World>>,
            selected_entity_id: std::sync::Arc<std::sync::Mutex<Option<u64>>>,
            gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
            hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
            asset_manager: Arc<engine_assets::AssetManager>,
        ) -> Result<Self, String> {
            tracing::info!(hwnd = ?parent_hwnd, "NativeViewport created for window");
            Ok(Self {
                parent_hwnd: SendHwnd(parent_hwnd),
                instances: Arc::new(Mutex::new(HashMap::new())),
                renderer_thread: None,
                should_stop: Arc::new(AtomicBool::new(false)),
                render_active: Arc::new(AtomicBool::new(true)),
                world,
                screenshot_slot: Arc::new(Mutex::new(None)),
                selected_entity_id,
                gizmo_mode,
                hovered_gizmo_axis,
                asset_manager,
            })
        }

        pub fn start_rendering(&mut self) -> Result<(), String> {
            let should_stop = self.should_stop.clone();
            let render_active = self.render_active.clone();
            let instances = self.instances.clone();
            let hwnd_raw = self.parent_hwnd.0 .0 as isize;
            let world = self.world.clone();
            let screenshot_slot = self.screenshot_slot.clone();
            let selected_entity_id = self.selected_entity_id.clone();
            let gizmo_mode = self.gizmo_mode.clone();
            let hovered_gizmo_axis = self.hovered_gizmo_axis.clone();
            let asset_manager = self.asset_manager.clone();

            let handle = std::thread::Builder::new()
                .name("viewport-render".into())
                .spawn(move || {
                    let hwnd = HWND(hwnd_raw as *mut _);
                    tracing::info!("Viewport render thread started");
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        render_loop(hwnd, should_stop, render_active, instances, world, screenshot_slot, selected_entity_id, gizmo_mode, hovered_gizmo_axis, asset_manager);
                    })) {
                        Ok(()) => tracing::info!("Viewport render thread stopped"),
                        Err(e) => {
                            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                                s.to_string()
                            } else if let Some(s) = e.downcast_ref::<String>() {
                                s.clone()
                            } else {
                                "unknown panic".into()
                            };
                            tracing::error!(error = %msg, "Viewport render thread PANICKED");
                        }
                    }
                })
                .map_err(|e| format!("Failed to spawn render thread: {e}"))?;

            self.renderer_thread = Some(handle);
            Ok(())
        }

        /// Add a new viewport instance, or update bounds if it already exists.
        /// Also re-activates a previously hidden instance (panel drag / tab switch).
        pub fn upsert_instance(&self, id: String, bounds: ViewportBounds) {
            let mut instances = self.instances.lock().unwrap();
            instances
                .entry(id)
                .and_modify(|i| {
                    i.bounds = bounds;
                    i.visible = true;
                })
                .or_insert_with(|| ViewportInstance::new(bounds));
        }

        /// Remove a viewport instance. Returns `true` if no instances remain.
        pub fn remove_instance(&self, id: &str) -> bool {
            let mut instances = self.instances.lock().unwrap();
            instances.remove(id);
            instances.is_empty()
        }

        pub fn set_instance_bounds(&self, id: &str, bounds: ViewportBounds) {
            if let Ok(mut instances) = self.instances.lock() {
                if let Some(inst) = instances.get_mut(id) {
                    inst.bounds = bounds;
                }
            }
        }

        pub fn set_instance_visible(&self, id: &str, visible: bool) {
            if let Ok(mut instances) = self.instances.lock() {
                if let Some(inst) = instances.get_mut(id) {
                    inst.visible = visible;
                }
            }
        }

        pub fn camera_orbit(&self, id: &str, dx: f32, dy: f32) {
            if let Ok(mut instances) = self.instances.lock() {
                if let Some(inst) = instances.get_mut(id) {
                    inst.camera.orbit(dx, dy);
                }
            }
        }

        pub fn camera_pan(&self, id: &str, dx: f32, dy: f32) {
            if let Ok(mut instances) = self.instances.lock() {
                if let Some(inst) = instances.get_mut(id) {
                    inst.camera.pan(dx, dy);
                }
            }
        }

        pub fn camera_zoom(&self, id: &str, delta: f32) {
            if let Ok(mut instances) = self.instances.lock() {
                if let Some(inst) = instances.get_mut(id) {
                    inst.camera.zoom(delta);
                }
            }
        }

        pub fn camera_reset(&self, id: &str) {
            if let Ok(mut instances) = self.instances.lock() {
                if let Some(inst) = instances.get_mut(id) {
                    inst.camera = OrbitCamera::default();
                }
            }
        }

        pub fn set_grid_visible(&self, id: &str, visible: bool) {
            if let Ok(mut instances) = self.instances.lock() {
                if let Some(inst) = instances.get_mut(id) {
                    inst.grid_visible = visible;
                }
            }
        }

        pub fn set_projection(&self, id: &str, is_ortho: bool) {
            if let Ok(mut instances) = self.instances.lock() {
                if let Some(inst) = instances.get_mut(id) {
                    inst.is_ortho = is_ortho;
                }
            }
        }

        pub fn camera_set_orientation(&self, id: &str, yaw: f32, pitch: f32) {
            if let Ok(mut instances) = self.instances.lock() {
                if let Some(inst) = instances.get_mut(id) {
                    inst.camera.yaw = yaw;
                    inst.camera.pitch = pitch.clamp(-1.5, 1.5);
                }
            }
        }

        /// Move the camera target to the given world-space position, keeping
        /// the current yaw/pitch/distance so the user's orientation is preserved.
        pub fn camera_focus(&self, id: &str, target: [f32; 3]) {
            if let Ok(mut instances) = self.instances.lock() {
                if let Some(inst) = instances.get_mut(id) {
                    inst.camera.target = glam::Vec3::from_array(target);
                }
            }
        }

        /// Snapshot the camera state for a viewport instance.
        /// Used to preserve state when the panel moves to a different OS window.
        pub fn get_instance_camera(&self, id: &str) -> Option<CameraState> {
            let instances = self.instances.lock().ok()?;
            let cam = &instances.get(id)?.camera;
            Some(CameraState {
                target: cam.target.to_array(),
                distance: cam.distance,
                yaw: cam.yaw,
                pitch: cam.pitch,
            })
        }

        /// Restore a previously saved camera state onto a viewport instance.
        pub fn set_instance_camera(&self, id: &str, state: CameraState) {
            if let Ok(mut instances) = self.instances.lock() {
                if let Some(inst) = instances.get_mut(id) {
                    inst.camera.target = glam::Vec3::from_array(state.target);
                    inst.camera.distance = state.distance;
                    inst.camera.yaw = state.yaw;
                    inst.camera.pitch = state.pitch;
                }
            }
        }

        /// Return the camera data needed for ray-casting (gizmo hit-testing).
        ///
        /// Returns `(view_matrix, proj_matrix, eye_position, bounds)` or `None`
        /// if the instance does not exist.
        pub fn get_instance_ray_data(
            &self,
            id: &str,
        ) -> Option<(Mat4, Mat4, Vec3, ViewportBounds)> {
            let instances = self.instances.lock().ok()?;
            let inst = instances.get(id)?;
            let cam = &inst.camera;
            let eye = cam.eye();
            let view = Mat4::look_at_rh(eye, cam.target, Vec3::Y);
            let aspect = if inst.bounds.height > 0 {
                inst.bounds.width as f32 / inst.bounds.height as f32
            } else {
                1.0
            };
            let proj = if inst.is_ortho {
                let half_h = cam.distance * (cam.fov_y * 0.5).tan();
                let half_w = half_h * aspect;
                Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, cam.near, cam.far * 2.0)
            } else {
                Mat4::perspective_rh(cam.fov_y, aspect, cam.near, cam.far)
            };
            Some((view, proj, eye, inst.bounds))
        }

        /// Capture a PNG screenshot of the current viewport frame.
        ///
        /// Posts a request to the render thread and blocks (up to 1 second) for the result.
        /// The render thread fulfils the request on its next iteration after `render_frame`
        /// completes, so the returned bytes reflect the last fully rendered frame.
        pub fn capture_png_bytes(&self) -> Result<Vec<u8>, String> {
            let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
            *self.screenshot_slot.lock().unwrap_or_else(|p| p.into_inner()) = Some(reply_tx);
            let result = reply_rx
                .recv_timeout(std::time::Duration::from_secs(1))
                .map_err(|e| match e {
                    std::sync::mpsc::RecvTimeoutError::Timeout => {
                        "Screenshot timed out".to_string()
                    }
                    std::sync::mpsc::RecvTimeoutError::Disconnected => {
                        "Screenshot request displaced by concurrent call".to_string()
                    }
                })
                .and_then(|r| r);
            match &result {
                Ok(bytes) => tracing::debug!(bytes = bytes.len(), "Screenshot captured"),
                Err(e) => tracing::warn!(error = %e, "Screenshot capture failed"),
            }
            result
        }

        pub fn destroy(&mut self) {
            self.render_active.store(false, Ordering::SeqCst);
            self.should_stop.store(true, Ordering::SeqCst);
            if let Some(handle) = self.renderer_thread.take() {
                let _ = handle.join();
            }
            tracing::info!("NativeViewport destroyed");
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

    #[derive(Clone, Debug)]
    struct OrbitCamera {
        target: Vec3,
        distance: f32,
        yaw: f32,
        pitch: f32,
        fov_y: f32,
        near: f32,
        far: f32,
    }

    impl Default for OrbitCamera {
        fn default() -> Self {
            Self {
                target: Vec3::ZERO,
                distance: 10.0,
                yaw: 0.0,
                pitch: std::f32::consts::FRAC_PI_6,
                fov_y: std::f32::consts::FRAC_PI_4,
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

        fn view_projection(&self, aspect: f32, is_ortho: bool) -> Mat4 {
            let view = Mat4::look_at_rh(self.eye(), self.target, Vec3::Y);
            if is_ortho {
                // half-extent matches what perspective renders at the focus point —
                // so toggling persp↔ortho doesn't cause a visual jump.
                let half_h = self.distance * (self.fov_y * 0.5).tan();
                let half_w = half_h * aspect;
                let proj = Mat4::orthographic_rh(
                    -half_w,
                    half_w,
                    -half_h,
                    half_h,
                    self.near,
                    self.far * 2.0,
                );
                proj * view
            } else {
                let proj = Mat4::perspective_rh(self.fov_y, aspect, self.near, self.far);
                proj * view
            }
        }

        /// View matrix (world → camera space). Used to construct separate ViewportDescriptor.
        fn view_matrix(&self) -> Mat4 {
            Mat4::look_at_rh(self.eye(), self.target, Vec3::Y)
        }

        /// Projection matrix. Used to construct separate ViewportDescriptor.
        fn proj_matrix(&self, aspect: f32, is_ortho: bool) -> Mat4 {
            if is_ortho {
                let half_h = self.distance * (self.fov_y * 0.5).tan();
                let half_w = half_h * aspect;
                Mat4::orthographic_rh(-half_w, half_w, -half_h, half_h, self.near, self.far * 2.0)
            } else {
                Mat4::perspective_rh(self.fov_y, aspect, self.near, self.far)
            }
        }

        fn orbit(&mut self, dx: f32, dy: f32) {
            self.yaw -= dx * 0.005;
            self.pitch = (self.pitch + dy * 0.005).clamp(-1.5, 1.5);
        }

        fn pan(&mut self, dx: f32, dy: f32) {
            let view = Mat4::look_at_rh(self.eye(), self.target, Vec3::Y);
            let right = Vec3::new(view.col(0).x, view.col(1).x, view.col(2).x);
            let up = Vec3::new(view.col(0).y, view.col(1).y, view.col(2).y);
            let scale = self.distance * 0.002;
            self.target -= right * dx * scale;
            self.target += up * dy * scale;
        }

        fn zoom(&mut self, delta: f32) {
            self.distance = (self.distance * (1.0 - delta * 0.001)).clamp(0.5, 200.0);
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Shader SPIR-V (compiled once per process, cached)
    // ──────────────────────────────────────────────────────────────────────

    static GRID_VERT_SPIRV: std::sync::OnceLock<Vec<u32>> = std::sync::OnceLock::new();
    static GRID_FRAG_SPIRV: std::sync::OnceLock<Vec<u32>> = std::sync::OnceLock::new();

    fn get_or_compile_shaders() -> Result<(&'static Vec<u32>, &'static Vec<u32>), String> {
        let vert = if let Some(v) = GRID_VERT_SPIRV.get() {
            v
        } else {
            tracing::info!("Compiling grid vertex shader (once)");
            let compiled = compile_glsl_to_spirv(GRID_VERT_GLSL, naga::ShaderStage::Vertex)?;
            let _ = GRID_VERT_SPIRV.set(compiled);
            GRID_VERT_SPIRV.get().unwrap()
        };
        let frag = if let Some(v) = GRID_FRAG_SPIRV.get() {
            v
        } else {
            tracing::info!("Compiling grid fragment shader (once)");
            let compiled = compile_glsl_to_spirv(GRID_FRAG_GLSL, naga::ShaderStage::Fragment)?;
            let _ = GRID_FRAG_SPIRV.set(compiled);
            GRID_FRAG_SPIRV.get().unwrap()
        };
        Ok((vert, frag))
    }

    fn compile_glsl_to_spirv(source: &str, stage: naga::ShaderStage) -> Result<Vec<u32>, String> {
        use naga::back::spv;
        use naga::front::glsl;
        use naga::valid::{Capabilities, ValidationFlags, Validator};

        let mut frontend = glsl::Frontend::default();
        let module = frontend
            .parse(&glsl::Options::from(stage), source)
            .map_err(|e| format!("GLSL parse: {:?}", e))?;
        let info = Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .map_err(|e| format!("Shader validation: {e}"))?;
        spv::write_vec(
            &module,
            &info,
            &spv::Options { lang_version: (1, 0), ..Default::default() },
            None,
        )
        .map_err(|e| format!("SPIR-V gen: {e}"))
    }

    const GRID_VERT_GLSL: &str = r#"#version 450
layout(push_constant) uniform PushConstants {
    mat4 viewProj;
    vec3 cameraPos;
    float _pad;
} pc;
layout(location = 0) in vec3 inPosition;
layout(location = 0) out vec2 fragWorldXZ;
void main() {
    gl_Position = pc.viewProj * vec4(inPosition, 1.0);
    fragWorldXZ = inPosition.xz;
}
"#;

    const GRID_FRAG_GLSL: &str = r#"#version 450
layout(push_constant) uniform PushConstants {
    mat4 viewProj;
    vec3 cameraPos;
    float _pad;
} pc;
layout(location = 0) in vec2 fragWorldXZ;
layout(location = 0) out vec4 outColor;

float gridLines(vec2 pos, float spacing) {
    vec2 p = pos / spacing;
    vec2 g = abs(fract(p - 0.5) - 0.5) / fwidth(p);
    return 1.0 - clamp(min(g.x, g.y), 0.0, 1.0);
}

void main() {
    vec2 pos = fragWorldXZ;
    float dist = length(pos - pc.cameraPos.xz);
    float fade = 1.0 - smoothstep(60.0, 80.0, dist);
    if (fade < 0.001) discard;

    float minor = gridLines(pos, 1.0);
    float major = gridLines(pos, 10.0);

    vec2 d = fwidth(pos);
    // X axis: line at world Z=0 (fragWorldXZ.y near 0) -> red
    float xAxisLine = 1.0 - clamp(abs(pos.y) / max(d.y, 0.0001), 0.0, 1.0);
    // Z axis: line at world X=0 (fragWorldXZ.x near 0) -> blue
    float zAxisLine = 1.0 - clamp(abs(pos.x) / max(d.x, 0.0001), 0.0, 1.0);

    vec3 col = vec3(0.0);
    float a = 0.0;

    if (minor > 0.01)          { col = vec3(0.28);               a = minor * 0.35; }
    if (major > minor)         { col = vec3(0.45);               a = max(a, major * 0.55); }
    if (xAxisLine > 0.01)      { col = vec3(0.70, 0.25, 0.25);   a = max(a, xAxisLine * 0.85); }
    if (zAxisLine > 0.01)      { col = vec3(0.25, 0.25, 0.70);   a = max(a, zAxisLine * 0.85); }

    if (a < 0.001) discard;
    outColor = vec4(col, a * fade);
}
"#;

    // ──────────────────────────────────────────────────────────────────────
    // Grid geometry
    // ──────────────────────────────────────────────────────────────────────

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct GridVertex {
        pos: [f32; 3],
    }

    /// Six vertices (two triangles) forming a quad on the XZ plane, Y=0.
    fn generate_grid_quad() -> Vec<GridVertex> {
        let e = 500.0f32;
        vec![
            GridVertex { pos: [-e, 0.0, -e] },
            GridVertex { pos: [e, 0.0, -e] },
            GridVertex { pos: [e, 0.0, e] },
            GridVertex { pos: [-e, 0.0, -e] },
            GridVertex { pos: [e, 0.0, e] },
            GridVertex { pos: [-e, 0.0, e] },
        ]
    }

    /// Push constant layout for the grid shaders — 80 bytes.
    /// Both VERTEX (viewProj) and FRAGMENT (cameraPos for fade) stages use this.
    #[repr(C)]
    struct GridPushConstants {
        view_proj: [f32; 16], // 64 bytes
        camera_pos: [f32; 3], // 12 bytes
        _pad: f32,            //  4 bytes — aligns to 80, within 128-byte guarantee
    }

    // ──────────────────────────────────────────────────────────────────────
    // Vulkan renderer
    // ──────────────────────────────────────────────────────────────────────

    const CLEAR_COLOR: [f32; 4] = [0.078, 0.078, 0.118, 1.0];

    use ash::vk;
    use engine_renderer::{GpuBuffer, ShaderModule};

    // ──────────────────────────────────────────────────────────────────────
    // GridPipeline — self-contained grid overlay
    // ──────────────────────────────────────────────────────────────────────

    struct GridPipeline {
        device: ash::Device,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        vertex_buffer: GpuBuffer,
        vertex_count: u32,
        _vert_shader: ShaderModule,
        _frag_shader: ShaderModule,
    }

    impl GridPipeline {
        fn new(
            device: &ash::Device,
            render_pass: vk::RenderPass,
            context: &engine_render_context::VulkanContext,
        ) -> Result<Self, String> {
            let (vert_spirv, frag_spirv) = get_or_compile_shaders()?;
            let vert_shader = ShaderModule::from_spirv(
                device,
                vert_spirv,
                vk::ShaderStageFlags::VERTEX,
                "main",
            )
            .map_err(|e| format!("VertShader: {e}"))?;
            let frag_shader = ShaderModule::from_spirv(
                device,
                frag_spirv,
                vk::ShaderStageFlags::FRAGMENT,
                "main",
            )
            .map_err(|e| format!("FragShader: {e}"))?;

            let (pipeline, pipeline_layout) =
                create_grid_pipeline(device, render_pass, &vert_shader, &frag_shader)?;

            let grid_verts = generate_grid_quad();
            let vertex_count = grid_verts.len() as u32;
            let buf_size = (grid_verts.len() * std::mem::size_of::<GridVertex>()) as u64;
            let mut vertex_buffer = GpuBuffer::new(
                context,
                buf_size,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                gpu_allocator::MemoryLocation::CpuToGpu,
            )
            .map_err(|e| format!("GridVB: {e}"))?;
            vertex_buffer
                .upload(&grid_verts)
                .map_err(|e| format!("GridVB upload: {e}"))?;

            tracing::info!("GridPipeline created");
            Ok(Self {
                device: device.clone(),
                pipeline,
                pipeline_layout,
                vertex_buffer,
                vertex_count,
                _vert_shader: vert_shader,
                _frag_shader: frag_shader,
            })
        }

        /// Record grid draw commands into an open command buffer.
        ///
        /// Sets viewport/scissor, push constants, and issues a draw for each
        /// visible viewport sub-rect.
        unsafe fn record(
            &self,
            cmd: vk::CommandBuffer,
            extent: vk::Extent2D,
            viewports: &[(ViewportBounds, OrbitCamera, bool, bool)],
        ) {
            let device = &self.device;
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            device.cmd_bind_vertex_buffers(cmd, 0, &[self.vertex_buffer.handle()], &[0]);

            for (bounds, camera, grid_visible, is_ortho) in viewports {
                let sx = bounds.x.max(0) as u32;
                let sy = bounds.y.max(0) as u32;
                let sw = bounds.width.min(extent.width.saturating_sub(sx)).max(1);
                let sh = bounds.height.min(extent.height.saturating_sub(sy)).max(1);

                device.cmd_set_viewport(
                    cmd,
                    0,
                    &[vk::Viewport {
                        x: sx as f32,
                        y: sy as f32,
                        width: sw as f32,
                        height: sh as f32,
                        min_depth: 0.0,
                        max_depth: 1.0,
                    }],
                );
                device.cmd_set_scissor(
                    cmd,
                    0,
                    &[vk::Rect2D {
                        offset: vk::Offset2D { x: sx as i32, y: sy as i32 },
                        extent: vk::Extent2D { width: sw, height: sh },
                    }],
                );

                let aspect = sw as f32 / sh as f32;
                let eye = camera.eye();
                let pc = GridPushConstants {
                    view_proj: camera.view_projection(aspect, *is_ortho).to_cols_array(),
                    camera_pos: eye.to_array(),
                    _pad: 0.0,
                };
                let pc_bytes = std::slice::from_raw_parts(
                    &pc as *const GridPushConstants as *const u8,
                    std::mem::size_of::<GridPushConstants>(),
                );
                device.cmd_push_constants(
                    cmd,
                    self.pipeline_layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    pc_bytes,
                );

                if *grid_visible {
                    device.cmd_draw(cmd, self.vertex_count, 1, 0, 0);
                }
            }
        }
    }

    impl Drop for GridPipeline {
        fn drop(&mut self) {
            unsafe {
                self.device.destroy_pipeline(self.pipeline, None);
                self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            }
        }
    }

    fn create_grid_pipeline(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        vert: &ShaderModule,
        frag: &ShaderModule,
    ) -> Result<(vk::Pipeline, vk::PipelineLayout), String> {
        let stages = [vert.stage_create_info(), frag.stage_create_info()];
        let binding = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(12)
            .input_rate(vk::VertexInputRate::VERTEX);
        let attrs = [vk::VertexInputAttributeDescription::default()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)];
        let vp = vk::Viewport::default().width(1.0).height(1.0).max_depth(1.0);
        let sc = vk::Rect2D::default().extent(vk::Extent2D { width: 1, height: 1 });
        let push_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .offset(0)
            .size(80);
        let layout = unsafe {
            device
                .create_pipeline_layout(
                    &vk::PipelineLayoutCreateInfo::default()
                        .push_constant_ranges(std::slice::from_ref(&push_range)),
                    None,
                )
                .map_err(|e| format!("pipeline layout: {e}"))?
        };

        let blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(vk::ColorComponentFlags::RGBA);

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[vk::GraphicsPipelineCreateInfo::default()
                        .stages(&stages)
                        .vertex_input_state(
                            &vk::PipelineVertexInputStateCreateInfo::default()
                                .vertex_binding_descriptions(std::slice::from_ref(&binding))
                                .vertex_attribute_descriptions(&attrs),
                        )
                        .input_assembly_state(
                            &vk::PipelineInputAssemblyStateCreateInfo::default()
                                .topology(vk::PrimitiveTopology::TRIANGLE_LIST),
                        )
                        .viewport_state(
                            &vk::PipelineViewportStateCreateInfo::default()
                                .viewports(std::slice::from_ref(&vp))
                                .scissors(std::slice::from_ref(&sc)),
                        )
                        .rasterization_state(
                            &vk::PipelineRasterizationStateCreateInfo::default()
                                .polygon_mode(vk::PolygonMode::FILL)
                                .cull_mode(vk::CullModeFlags::NONE)
                                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                                .line_width(1.0),
                        )
                        .multisample_state(
                            &vk::PipelineMultisampleStateCreateInfo::default()
                                .rasterization_samples(vk::SampleCountFlags::TYPE_1),
                        )
                        .depth_stencil_state(
                            &vk::PipelineDepthStencilStateCreateInfo::default()
                                .depth_test_enable(true)
                                .depth_write_enable(false)
                                .depth_compare_op(vk::CompareOp::LESS),
                        )
                        .color_blend_state(
                            &vk::PipelineColorBlendStateCreateInfo::default()
                                .attachments(std::slice::from_ref(&blend_attachment)),
                        )
                        .dynamic_state(
                            &vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&[
                                vk::DynamicState::VIEWPORT,
                                vk::DynamicState::SCISSOR,
                            ]),
                        )
                        .layout(layout)
                        .render_pass(render_pass)
                        .subpass(0)],
                    None,
                )
                .map_err(|(_, e)| {
                    device.destroy_pipeline_layout(layout, None);
                    format!("pipeline: {e}")
                })?[0]
        };
        tracing::info!("Grid pipeline created (infinite shader-based)");
        Ok((pipeline, layout))
    }

    // ──────────────────────────────────────────────────────────────────────
    // ViewportRenderer — wraps engine_renderer::Renderer + GridPipeline
    // ──────────────────────────────────────────────────────────────────────

    struct ViewportRenderer {
        // IMPORTANT: field declaration order matters for Drop order.
        // `renderer` must come before `grid_pipeline` / `gizmo_pipeline` so that
        // `Renderer::drop` (which calls device.wait_idle) runs before the
        // pipeline `Drop` impls attempt to destroy their Vulkan objects.
        renderer: engine_renderer::Renderer,
        grid_pipeline: GridPipeline,
        gizmo_pipeline: crate::viewport::gizmo_pipeline::GizmoPipeline,
        width: u32,
        height: u32,
        needs_recreate: bool,
        /// Retained so GizmoSolidPipeline (Task 6) can share it without re-cloning.
        #[allow(dead_code)]
        hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
    }

    impl ViewportRenderer {
        fn new(hwnd: HWND, width: u32, height: u32, hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>) -> Result<Self, String> {
            let (width, height) = (width.max(1), height.max(1));
            let hwnd_raw = hwnd.0 as isize;

            let mut renderer =
                engine_renderer::Renderer::from_raw_handle(hwnd_raw, width, height, "silmaril-editor")
                    .map_err(|e| format!("Renderer: {e}"))?;

            renderer.set_clear_color(CLEAR_COLOR[0], CLEAR_COLOR[1], CLEAR_COLOR[2], CLEAR_COLOR[3]);

            let grid_pipeline = GridPipeline::new(
                renderer.device(),
                renderer.render_pass(),
                renderer.context(),
            )?;

            let gizmo_pipeline = crate::viewport::gizmo_pipeline::GizmoPipeline::new(
                renderer.context(),
                renderer.render_pass(),
                std::sync::Arc::clone(&hovered_gizmo_axis),
            )?;

            tracing::info!(width, height, "ViewportRenderer initialised");
            Ok(Self { renderer, grid_pipeline, gizmo_pipeline, width, height, needs_recreate: false, hovered_gizmo_axis })
        }

        fn notify_resize(&mut self, w: u32, h: u32) {
            let (w, h) = (w.max(1), h.max(1));
            if w != self.width || h != self.height {
                self.width = w;
                self.height = h;
                self.needs_recreate = true;
            }
        }

        /// Render all visible viewport instances in one frame.
        fn render_frame(
            &mut self,
            viewports: &[(ViewportBounds, OrbitCamera, bool, bool)],
            world: &std::sync::RwLock<engine_core::World>,
            selected_entity_id: Option<u64>,
            gizmo_mode: crate::viewport::gizmo_pipeline::GizmoMode,
            asset_manager: &engine_assets::AssetManager,
        ) -> Result<(), String> {
            if self.needs_recreate {
                self.renderer
                    .rebuild_swapchain(self.width, self.height)
                    .map_err(|e| format!("rebuild_swapchain: {e}"))?;
                self.needs_recreate = false;
            }

            let Some(recorder) = self.renderer.begin_frame() else {
                // Swapchain out-of-date — mark for rebuild and retry next tick
                self.needs_recreate = true;
                return Ok(());
            };

            let cmd = recorder.command_buffer;
            let extent = self.renderer.extent();

            // Record grid overlay draw commands
            unsafe {
                self.grid_pipeline.record(cmd, extent, viewports);
            }

            // Build ViewportDescriptors with separate view/proj for render_meshes
            let vp_descs: Vec<engine_renderer::ViewportDescriptor> = viewports
                .iter()
                .map(|(bounds, cam, _grid_visible, is_ortho)| {
                    let aspect = if bounds.height > 0 {
                        bounds.width as f32 / bounds.height as f32
                    } else {
                        1.0
                    };
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

            // Record gizmo draw commands — one call per viewport sub-rect so
            // each sub-rect gets its own view/projection matrix.
            // The grid pipeline's record already set viewport/scissor to the
            // last sub-rect; we re-set them here for correctness.
            if let Ok(world_guard) = world.read() {
                unsafe {
                    for (bounds, camera, _grid_visible, is_ortho) in viewports {
                        let sx = bounds.x.max(0) as u32;
                        let sy = bounds.y.max(0) as u32;
                        let sw = bounds.width.min(extent.width.saturating_sub(sx)).max(1);
                        let sh = bounds.height.min(extent.height.saturating_sub(sy)).max(1);

                        self.renderer.device().cmd_set_viewport(
                            cmd,
                            0,
                            &[vk::Viewport {
                                x: sx as f32,
                                y: sy as f32,
                                width: sw as f32,
                                height: sh as f32,
                                min_depth: 0.0,
                                max_depth: 1.0,
                            }],
                        );
                        self.renderer.device().cmd_set_scissor(
                            cmd,
                            0,
                            &[vk::Rect2D {
                                offset: vk::Offset2D { x: sx as i32, y: sy as i32 },
                                extent: vk::Extent2D { width: sw, height: sh },
                            }],
                        );

                        let aspect = sw as f32 / sh as f32;
                        let view_proj = camera.view_projection(aspect, *is_ortho);
                        let camera_pos = camera.eye();

                        self.gizmo_pipeline.record(
                            cmd,
                            &world_guard,
                            selected_entity_id,
                            gizmo_mode,
                            view_proj,
                            camera_pos,
                        );
                    }
                }

                // Render meshes from ECS world (Phase 1.8)
                self.renderer.render_meshes(&recorder, &world_guard, Some(asset_manager), &vp_descs);
            }

            self.renderer.end_frame(recorder);
            Ok(())
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // Render loop
    // ──────────────────────────────────────────────────────────────────────

    fn render_loop(
        hwnd: HWND,
        should_stop: Arc<AtomicBool>,
        render_active: Arc<AtomicBool>,
        instances: Arc<Mutex<HashMap<String, ViewportInstance>>>,
        world: Arc<std::sync::RwLock<engine_core::World>>,
        screenshot_slot: Arc<Mutex<Option<std::sync::mpsc::SyncSender<Result<Vec<u8>, String>>>>>,
        selected_entity_id: std::sync::Arc<std::sync::Mutex<Option<u64>>>,
        gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
        hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
        asset_manager: Arc<engine_assets::AssetManager>,
    ) {
        let (init_w, init_h) = client_size(hwnd);
        let mut renderer = match ViewportRenderer::new(hwnd, init_w, init_h, hovered_gizmo_axis) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, "ViewportRenderer init failed");
                while !should_stop.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                return;
            }
        };

        while !should_stop.load(Ordering::SeqCst) {
            if !render_active.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(32));
                continue;
            }

            let (win_w, win_h) = client_size(hwnd);
            renderer.notify_resize(win_w, win_h);

            // Snapshot visible instances for this frame
            let viewports: Vec<(ViewportBounds, OrbitCamera, bool, bool)> = {
                let lock = instances.lock().unwrap();
                lock.values()
                    .filter(|i| i.visible)
                    .map(|i| (i.bounds, i.camera.clone(), i.grid_visible, i.is_ortho))
                    .collect()
            };

            if viewports.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(32));
                continue;
            }

            let selected_id = selected_entity_id.lock().ok().and_then(|g| *g);
            let gizmo_mode_val = match gizmo_mode.load(std::sync::atomic::Ordering::Relaxed) {
                1 => crate::viewport::gizmo_pipeline::GizmoMode::Rotate,
                2 => crate::viewport::gizmo_pipeline::GizmoMode::Scale,
                _ => crate::viewport::gizmo_pipeline::GizmoMode::Move,
            };
            if let Err(e) = renderer.render_frame(&viewports, &world, selected_id, gizmo_mode_val, &asset_manager) {
                tracing::error!(error = %e, "render_frame failed");
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            // Screenshot capture — if a request arrived, capture and reply.
            if let Some(reply_tx) = screenshot_slot.lock().unwrap_or_else(|p| p.into_inner()).take() {
                let result = renderer.renderer.get_frame_png().map_err(|e| e.to_string());
                let _ = reply_tx.send(result);
            }

            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        drop(renderer);
    }

    fn client_size(hwnd: HWND) -> (u32, u32) {
        unsafe {
            let mut r: RECT = std::mem::zeroed();
            let _ = GetClientRect(hwnd, &mut r);
            (r.right.max(1) as u32, r.bottom.max(1) as u32)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::f32::consts::FRAC_PI_6;

        // ── OrbitCamera ──────────────────────────────────────────────────────

        #[test]
        fn orbit_camera_default_values() {
            let cam = OrbitCamera::default();
            assert_eq!(cam.yaw, 0.0);
            assert!((cam.pitch - FRAC_PI_6).abs() < 1e-6, "pitch should be PI/6");
            assert_eq!(cam.distance, 10.0);
            assert_eq!(cam.target, Vec3::ZERO);
        }

        #[test]
        fn orbit_camera_orbit_updates_yaw_and_pitch() {
            let orbit_scale = 0.005_f32; // same constant as OrbitCamera::orbit()
            let mut cam = OrbitCamera::default();
            cam.orbit(100.0, 0.0);
            let expected_yaw = -100.0 * orbit_scale;
            assert!((cam.yaw - expected_yaw).abs() < 1e-5, "yaw should decrease by dx*0.005");
            assert!((cam.pitch - FRAC_PI_6).abs() < 1e-6, "pitch unchanged");

            let mut cam2 = OrbitCamera::default();
            cam2.orbit(0.0, 100.0);
            assert_eq!(cam2.yaw, 0.0, "yaw unchanged");
            let expected_pitch = (FRAC_PI_6 + 100.0 * orbit_scale).min(1.5);
            assert!((cam2.pitch - expected_pitch).abs() < 1e-5);
        }

        #[test]
        fn orbit_camera_pitch_clamps_to_bounds() {
            let mut cam = OrbitCamera::default();
            cam.orbit(0.0, 10_000.0); // huge positive dy
            assert!(cam.pitch <= 1.5, "pitch must not exceed 1.5");

            let mut cam2 = OrbitCamera::default();
            cam2.orbit(0.0, -10_000.0); // huge negative dy
            assert!(cam2.pitch >= -1.5, "pitch must not go below -1.5");
        }

        #[test]
        fn orbit_camera_zoom_clamps() {
            let mut cam = OrbitCamera::default();
            cam.zoom(100_000.0); // zoom in maximally
            assert!(cam.distance >= 0.5, "distance must not go below 0.5");

            let mut cam2 = OrbitCamera::default();
            cam2.zoom(-100_000.0); // zoom out maximally
            assert!(cam2.distance <= 200.0, "distance must not exceed 200.0");
        }

        #[test]
        fn orbit_camera_eye_is_offset_from_target() {
            let cam = OrbitCamera::default(); // yaw=0, pitch=PI/6, distance=10
            let eye = cam.eye();
            // Eye should be above and behind the target, not at origin
            assert!(eye.y > 0.0, "eye should be above Y=0 at default pitch");
            assert!(eye.length() > 0.1, "eye should not be at origin");
        }

        // ── camera_set_orientation ───────────────────────────────────────────

        #[test]
        fn orbit_camera_direct_yaw_assignment() {
            let mut cam = OrbitCamera::default();
            cam.yaw = -std::f32::consts::FRAC_PI_2;
            assert!((cam.yaw - (-std::f32::consts::FRAC_PI_2)).abs() < 1e-6);
        }

        #[test]
        fn orbit_camera_pitch_clamp_at_boundary() {
            // Exercise the actual orbit() clamp — large dy drives pitch to max/min
            let orbit_scale = 0.005_f32;
            let mut cam_high = OrbitCamera::default();
            cam_high.orbit(0.0, 99.0 / orbit_scale); // huge positive dy
            assert!(
                (cam_high.pitch - 1.5).abs() < 1e-5,
                "large positive dy should saturate pitch at 1.5, got {}",
                cam_high.pitch
            );

            let mut cam_low = OrbitCamera::default();
            cam_low.orbit(0.0, -99.0 / orbit_scale); // huge negative dy
            assert!(
                (cam_low.pitch - (-1.5)).abs() < 1e-5,
                "large negative dy should saturate pitch at -1.5, got {}",
                cam_low.pitch
            );

            // Mid-range value passes through unchanged
            let mut cam_mid = OrbitCamera { pitch: 0.0, ..OrbitCamera::default() };
            cam_mid.orbit(0.0, 0.5 / orbit_scale); // dy such that pitch moves exactly 0.5
            assert!(
                (cam_mid.pitch - 0.5).abs() < 1e-5,
                "pitch should be 0.5 after small dy, got {}",
                cam_mid.pitch
            );
        }

        // ── generate_grid_quad ───────────────────────────────────────────────

        #[test]
        fn generate_grid_quad_returns_six_vertices() {
            let verts = generate_grid_quad();
            assert_eq!(verts.len(), 6, "two triangles = 6 vertices");
        }

        #[test]
        fn generate_grid_quad_all_y_zero() {
            for v in generate_grid_quad() {
                assert_eq!(v.pos[1], 0.0, "all vertices on XZ plane (Y=0)");
            }
        }

        #[test]
        fn generate_grid_quad_covers_500_unit_extent() {
            for v in generate_grid_quad() {
                assert!(v.pos[0].abs() <= 500.0, "X within ±500");
                assert!(v.pos[2].abs() <= 500.0, "Z within ±500");
            }
            // Should actually have ±500 corners — verify extreme values present
            let verts = generate_grid_quad();
            let has_neg = verts.iter().any(|v| v.pos[0] < -499.0);
            let has_pos = verts.iter().any(|v| v.pos[0] > 499.0);
            assert!(has_neg && has_pos, "should span from -500 to +500");
        }

        // ── GridPushConstants size ───────────────────────────────────────────

        #[test]
        fn grid_push_constants_is_80_bytes() {
            assert_eq!(
                std::mem::size_of::<GridPushConstants>(),
                80,
                "GridPushConstants must be 80 bytes: 64 (mat4) + 12 (vec3) + 4 (pad)"
            );
        }

        // ── render_loop Arc read pattern ─────────────────────────────────────

        #[test]
        fn render_loop_reads_selected_entity_from_arc() {
            use std::sync::{Arc, Mutex};
            let selected = Arc::new(Mutex::new(Some(7u64)));
            let val = selected.lock().ok().and_then(|g| *g);
            assert_eq!(val, Some(7));
            *selected.lock().unwrap() = None;
            let val = selected.lock().ok().and_then(|g| *g);
            assert_eq!(val, None);
        }

        // ── ViewportInstance ─────────────────────────────────────────────────

        #[test]
        fn viewport_instance_defaults_to_visible_and_grid_on() {
            let inst =
                ViewportInstance::new(ViewportBounds { x: 0, y: 0, width: 800, height: 600 });
            assert!(inst.visible, "new instance should be visible");
            assert!(inst.grid_visible, "new instance should show grid");
        }

        #[test]
        fn viewport_instance_camera_default_yaw_zero() {
            let inst =
                ViewportInstance::new(ViewportBounds { x: 0, y: 0, width: 800, height: 600 });
            assert_eq!(inst.camera.yaw, 0.0, "camera yaw should start at 0 to match JS convention");
        }

        // ── Orthographic projection ──────────────────────────────────────────

        #[test]
        fn view_projection_perspective_returns_finite_values() {
            let cam = OrbitCamera::default();
            let mat = cam.view_projection(16.0 / 9.0, false);
            assert!(mat.to_cols_array().iter().all(|v| v.is_finite()));
        }

        #[test]
        fn view_projection_ortho_returns_finite_values() {
            let cam = OrbitCamera::default();
            let mat = cam.view_projection(16.0 / 9.0, true);
            assert!(mat.to_cols_array().iter().all(|v| v.is_finite()));
        }

        #[test]
        fn view_projection_ortho_differs_from_perspective() {
            let cam = OrbitCamera::default();
            let persp = cam.view_projection(1.0, false).to_cols_array();
            let ortho = cam.view_projection(1.0, true).to_cols_array();
            // Matrices must be different — ortho has no perspective divide
            assert!(persp != ortho, "ortho and perspective matrices must differ");
        }

        #[test]
        fn viewport_instance_defaults_to_perspective() {
            let inst =
                ViewportInstance::new(ViewportBounds { x: 0, y: 0, width: 800, height: 600 });
            assert!(!inst.is_ortho, "new instance should default to perspective");
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Platform-agnostic types
// ──────────────────────────────────────────────────────────────────────────────

/// Serialisable snapshot of an orbit camera — used to preserve state when a
/// viewport panel moves across OS windows (e.g. pop-out / dock-back).
#[derive(Clone, Debug, Default)]
pub struct CameraState {
    pub target: [f32; 3],
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
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
    pub fn new(
        _parent_hwnd: isize,
        _world: std::sync::Arc<std::sync::RwLock<engine_core::World>>,
        _selected_entity_id: std::sync::Arc<std::sync::Mutex<Option<u64>>>,
        _gizmo_mode: std::sync::Arc<std::sync::atomic::AtomicU8>,
    ) -> Result<Self, String> {
        Err("Native viewport not yet implemented for this platform".into())
    }
    pub fn start_rendering(&mut self) -> Result<(), String> {
        Err("Not implemented".into())
    }
    pub fn upsert_instance(&self, _id: String, _bounds: ViewportBounds) {}
    pub fn remove_instance(&self, _id: &str) -> bool {
        true
    }
    pub fn set_instance_bounds(&self, _id: &str, _bounds: ViewportBounds) {}
    pub fn set_instance_visible(&self, _id: &str, _visible: bool) {}
    pub fn camera_orbit(&self, _id: &str, _dx: f32, _dy: f32) {}
    pub fn camera_pan(&self, _id: &str, _dx: f32, _dy: f32) {}
    pub fn camera_zoom(&self, _id: &str, _delta: f32) {}
    pub fn camera_reset(&self, _id: &str) {}
    pub fn get_instance_camera(&self, _id: &str) -> Option<CameraState> {
        None
    }
    pub fn set_instance_camera(&self, _id: &str, _state: CameraState) {}
    pub fn set_grid_visible(&self, _id: &str, _visible: bool) {}
    pub fn camera_set_orientation(&self, _id: &str, _yaw: f32, _pitch: f32) {}
    pub fn camera_focus(&self, _id: &str, _target: [f32; 3]) {}
    pub fn set_projection(&self, _id: &str, _is_ortho: bool) {}
    pub fn capture_png_bytes(&self) -> Result<Vec<u8>, String> {
        Err("Screenshot capture is only supported on Windows".into())
    }
    pub fn destroy(&mut self) {}
}
