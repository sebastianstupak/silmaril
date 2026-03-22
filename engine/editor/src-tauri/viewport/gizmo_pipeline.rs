//! GizmoPipeline — procedural 3D gizmos rendered as line segments.
//!
//! Draws axis-crosshairs and move/rotate/scale handles on the selected entity.
//!
//! All geometry uses LINE_LIST topology so a single pipeline serves every
//! gizmo type.  Geometry is generated once at pipeline creation and stored
//! in per-axis vertex buffers.  Each draw call pushes 112 bytes of push
//! constants (viewProj, origin, color, scale).

// Only compiled on Windows (where Vulkan is present in the editor build).
#[cfg(windows)]
mod imp {
    use ash::vk;
    use engine_render_context::VulkanContext;
    use engine_renderer::{GpuBuffer, ShaderModule};
    use tracing::info;

    // ─────────────────────────────────────────────────────────────────────
    // Public types (re-exported from the parent mod below)
    // ─────────────────────────────────────────────────────────────────────

    /// One vertex in gizmo-local space.
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    pub struct GizmoVertex {
        pub pos: [f32; 3],
    }

    /// Which principal axis a gizmo handle is aligned to.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub enum GizmoAxis {
        X,
        Y,
        Z,
        XY,
        XZ,
        YZ,
    }

    /// Which transform manipulation mode is active.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub enum GizmoMode {
        Move,
        Rotate,
        Scale,
    }

    // ─────────────────────────────────────────────────────────────────────
    // Push constants  (112 bytes — within the 128-byte Vulkan minimum)
    // ─────────────────────────────────────────────────────────────────────

    #[repr(C)]
    struct GizmoPushConstants {
        view_proj: [[f32; 4]; 4], // 64 bytes
        origin: [f32; 3],         // 12 bytes
        _pad0: f32,               //  4 bytes
        color: [f32; 4],          // 16 bytes
        scale: f32,               //  4 bytes
        _pad1: [f32; 3],          // 12 bytes
        // Total: 112 bytes
    }

    // ─────────────────────────────────────────────────────────────────────
    // Inline GLSL sources (compiled once via naga)
    // ─────────────────────────────────────────────────────────────────────

    const GIZMO_VERT_GLSL: &str = include_str!("shaders/gizmo.vert");
    const GIZMO_FRAG_GLSL: &str = include_str!("shaders/gizmo.frag");

    static GIZMO_VERT_SPIRV: std::sync::OnceLock<Vec<u32>> = std::sync::OnceLock::new();
    static GIZMO_FRAG_SPIRV: std::sync::OnceLock<Vec<u32>> = std::sync::OnceLock::new();

    fn get_or_compile_shaders() -> Result<(&'static Vec<u32>, &'static Vec<u32>), String> {
        let vert = if let Some(v) = GIZMO_VERT_SPIRV.get() {
            v
        } else {
            info!("Compiling gizmo vertex shader (once)");
            let compiled =
                compile_glsl_to_spirv(GIZMO_VERT_GLSL, naga::ShaderStage::Vertex)?;
            let _ = GIZMO_VERT_SPIRV.set(compiled);
            GIZMO_VERT_SPIRV.get().unwrap()
        };
        let frag = if let Some(v) = GIZMO_FRAG_SPIRV.get() {
            v
        } else {
            info!("Compiling gizmo fragment shader (once)");
            let compiled =
                compile_glsl_to_spirv(GIZMO_FRAG_GLSL, naga::ShaderStage::Fragment)?;
            let _ = GIZMO_FRAG_SPIRV.set(compiled);
            GIZMO_FRAG_SPIRV.get().unwrap()
        };
        Ok((vert, frag))
    }

    fn compile_glsl_to_spirv(
        source: &str,
        stage: naga::ShaderStage,
    ) -> Result<Vec<u32>, String> {
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

    // ─────────────────────────────────────────────────────────────────────
    // Procedural geometry generators
    // ─────────────────────────────────────────────────────────────────────

    /// Three axis lines: X, Y, Z — 2 vertices each = 6 total.
    /// Each line runs from the gizmo origin to 1 unit along its axis.
    pub fn generate_crosshair_vertices() -> Vec<GizmoVertex> {
        vec![
            // X axis
            GizmoVertex { pos: [0.0, 0.0, 0.0] },
            GizmoVertex { pos: [1.0, 0.0, 0.0] },
            // Y axis
            GizmoVertex { pos: [0.0, 0.0, 0.0] },
            GizmoVertex { pos: [0.0, 1.0, 0.0] },
            // Z axis
            GizmoVertex { pos: [0.0, 0.0, 0.0] },
            GizmoVertex { pos: [0.0, 0.0, 1.0] },
        ]
    }

    /// Move-arrow: shaft from origin to 0.8 along axis, then cone outline
    /// from 0.8 to 1.0 using 8-spoke lines (LINE_LIST pairs).
    pub fn generate_move_arrow_vertices(axis: GizmoAxis) -> Vec<GizmoVertex> {
        let dir = axis_dir(axis);
        let perp1 = perpendicular(dir);
        let perp2 = dir.cross(perp1);

        let shaft_tip = dir * 0.8;
        let arrow_tip = dir * 1.0;
        const CONE_R: f32 = 0.06;
        const SPOKES: usize = 8;

        let mut verts = Vec::new();

        // Shaft: origin → shaft_tip
        verts.push(GizmoVertex { pos: [0.0, 0.0, 0.0] });
        verts.push(GizmoVertex { pos: shaft_tip.into() });

        // Cone spokes: shaft_tip ring → arrow_tip
        for i in 0..SPOKES {
            let angle = (i as f32) * std::f32::consts::TAU / (SPOKES as f32);
            let (s, c) = angle.sin_cos();
            let rim = shaft_tip + perp1 * (c * CONE_R) + perp2 * (s * CONE_R);
            // spoke from rim to tip
            verts.push(GizmoVertex { pos: rim.into() });
            verts.push(GizmoVertex { pos: arrow_tip.into() });
            // ring segment connecting adjacent rim points
            let next_angle = ((i + 1) as f32) * std::f32::consts::TAU / (SPOKES as f32);
            let (ns, nc) = next_angle.sin_cos();
            let next_rim = shaft_tip + perp1 * (nc * CONE_R) + perp2 * (ns * CONE_R);
            verts.push(GizmoVertex { pos: rim.into() });
            verts.push(GizmoVertex { pos: next_rim.into() });
        }

        verts
    }

    /// Rotation ring: a circle of N segments in the plane perpendicular to the axis.
    pub fn generate_rotate_ring_vertices(axis: GizmoAxis) -> Vec<GizmoVertex> {
        let dir = axis_dir(axis);
        let perp1 = perpendicular(dir);
        let perp2 = dir.cross(perp1);
        const SEGMENTS: usize = 32;
        const RADIUS: f32 = 1.0;

        let mut verts = Vec::new();
        for i in 0..SEGMENTS {
            let a0 = (i as f32) * std::f32::consts::TAU / (SEGMENTS as f32);
            let a1 = ((i + 1) as f32) * std::f32::consts::TAU / (SEGMENTS as f32);
            let p0 = perp1 * (a0.cos() * RADIUS) + perp2 * (a0.sin() * RADIUS);
            let p1 = perp1 * (a1.cos() * RADIUS) + perp2 * (a1.sin() * RADIUS);
            verts.push(GizmoVertex { pos: p0.into() });
            verts.push(GizmoVertex { pos: p1.into() });
        }
        verts
    }

    /// Scale handle: shaft + small box outline at the tip.
    pub fn generate_scale_handle_vertices(axis: GizmoAxis) -> Vec<GizmoVertex> {
        let dir = axis_dir(axis);
        let perp1 = perpendicular(dir);
        let perp2 = dir.cross(perp1);

        let shaft_tip = dir * 0.85;
        const HALF: f32 = 0.06;

        let mut verts = Vec::new();

        // Shaft
        verts.push(GizmoVertex { pos: [0.0, 0.0, 0.0] });
        verts.push(GizmoVertex { pos: shaft_tip.into() });

        // Box: 8 corners
        let corners: Vec<glam::Vec3> = [[-1.0f32, -1.0, -1.0], [1.0, -1.0, -1.0],
                        [1.0, 1.0, -1.0], [-1.0, 1.0, -1.0],
                        [-1.0, -1.0, 1.0], [1.0, -1.0, 1.0],
                        [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0]]
            .iter()
            .map(|o| {
                shaft_tip
                    + dir * (o[2] * HALF)
                    + perp1 * (o[0] * HALF)
                    + perp2 * (o[1] * HALF)
            })
            .collect();

        // 12 edges of the cube as LINE_LIST pairs
        let edges = [
            (0, 1), (1, 2), (2, 3), (3, 0), // bottom face
            (4, 5), (5, 6), (6, 7), (7, 4), // top face
            (0, 4), (1, 5), (2, 6), (3, 7), // vertical edges
        ];
        for (a, b) in edges {
            verts.push(GizmoVertex { pos: corners[a].into() });
            verts.push(GizmoVertex { pos: corners[b].into() });
        }

        verts
    }

    // ─────────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────────

    fn axis_dir(axis: GizmoAxis) -> glam::Vec3 {
        match axis {
            GizmoAxis::X => glam::Vec3::X,
            GizmoAxis::Y => glam::Vec3::Y,
            GizmoAxis::Z => glam::Vec3::Z,
            // Plane handles: use the "missing" axis as a diagonal
            GizmoAxis::XY => (glam::Vec3::X + glam::Vec3::Y).normalize(),
            GizmoAxis::XZ => (glam::Vec3::X + glam::Vec3::Z).normalize(),
            GizmoAxis::YZ => (glam::Vec3::Y + glam::Vec3::Z).normalize(),
        }
    }

    /// Return an arbitrary vector perpendicular to `v` (v must be non-zero).
    fn perpendicular(v: glam::Vec3) -> glam::Vec3 {
        let candidate = if v.x.abs() < 0.9 { glam::Vec3::X } else { glam::Vec3::Y };
        v.cross(candidate).normalize()
    }

    /// Returns the RGBA colour for a gizmo axis, brightened when hovered.
    fn axis_color(axis: GizmoAxis, hovered: bool) -> [f32; 4] {
        let base: [f32; 4] = match axis {
            GizmoAxis::X => [1.0, 0.2, 0.2, 1.0],
            GizmoAxis::Y => [0.2, 1.0, 0.2, 1.0],
            GizmoAxis::Z => [0.2, 0.4, 1.0, 1.0],
            _ => [0.8, 0.8, 0.8, 1.0],
        };
        if hovered {
            [
                (base[0] + 0.35).min(1.0),
                (base[1] + 0.35).min(1.0),
                (base[2] + 0.35).min(1.0),
                1.0,
            ]
        } else {
            base
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // GpuBuffer helpers
    // ─────────────────────────────────────────────────────────────────────

    fn upload_verts(
        context: &VulkanContext,
        verts: &[GizmoVertex],
    ) -> Result<GpuBuffer, String> {
        let buf_size = (verts.len() * std::mem::size_of::<GizmoVertex>()) as u64;
        // Minimum size of 1 byte to avoid zero-sized allocations.
        let buf_size = buf_size.max(1);
        let mut buf = GpuBuffer::new(
            context,
            buf_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            gpu_allocator::MemoryLocation::CpuToGpu,
        )
        .map_err(|e| format!("GizmoVB alloc: {e}"))?;
        if !verts.is_empty() {
            buf.upload(verts).map_err(|e| format!("GizmoVB upload: {e}"))?;
        }
        Ok(buf)
    }

    // ─────────────────────────────────────────────────────────────────────
    // GizmoPipeline
    // ─────────────────────────────────────────────────────────────────────

    /// Self-contained gizmo overlay pipeline.
    ///
    /// Owns one VkPipeline (LINE_LIST) and 10 vertex buffers covering
    /// crosshair + move/rotate/scale handles for each axis.
    #[allow(dead_code)]
    pub struct GizmoPipeline {
        device: ash::Device,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        _vert_shader: ShaderModule,
        _frag_shader: ShaderModule,

        crosshair_buf: GpuBuffer,
        crosshair_count: u32,

        move_x_buf: GpuBuffer,
        move_x_count: u32,
        move_y_buf: GpuBuffer,
        move_y_count: u32,
        move_z_buf: GpuBuffer,
        move_z_count: u32,

        rotate_x_buf: GpuBuffer,
        rotate_x_count: u32,
        rotate_y_buf: GpuBuffer,
        rotate_y_count: u32,
        rotate_z_buf: GpuBuffer,
        rotate_z_count: u32,

        scale_x_buf: GpuBuffer,
        scale_x_count: u32,
        scale_y_buf: GpuBuffer,
        scale_y_count: u32,
        scale_z_buf: GpuBuffer,
        scale_z_count: u32,

        /// Which gizmo axis is currently hovered (0 = none, 1..=6 = axes).
        /// Shared with the main thread via atomic for hover highlighting.
        hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
    }

    impl GizmoPipeline {
        pub fn new(
            context: &VulkanContext,
            render_pass: vk::RenderPass,
            hovered_gizmo_axis: std::sync::Arc<std::sync::atomic::AtomicU8>,
        ) -> Result<Self, String> {
            let device = &context.device;
            let (vert_spirv, frag_spirv) = get_or_compile_shaders()?;
            let vert_shader =
                ShaderModule::from_spirv(device, vert_spirv, vk::ShaderStageFlags::VERTEX, "main")
                    .map_err(|e| format!("GizmoVertShader: {e}"))?;
            let frag_shader = ShaderModule::from_spirv(
                device,
                frag_spirv,
                vk::ShaderStageFlags::FRAGMENT,
                "main",
            )
            .map_err(|e| format!("GizmoFragShader: {e}"))?;

            let (pipeline, pipeline_layout) =
                create_gizmo_pipeline(device, render_pass, &vert_shader, &frag_shader)?;

            // ── Generate and upload geometry ──────────────────────────────
            macro_rules! upload {
                ($verts:expr) => {{
                    let v = $verts;
                    let count = v.len() as u32;
                    let buf = upload_verts(context, &v)?;
                    (buf, count)
                }};
            }

            let (crosshair_buf, crosshair_count) = upload!(generate_crosshair_vertices());
            let (move_x_buf, move_x_count) =
                upload!(generate_move_arrow_vertices(GizmoAxis::X));
            let (move_y_buf, move_y_count) =
                upload!(generate_move_arrow_vertices(GizmoAxis::Y));
            let (move_z_buf, move_z_count) =
                upload!(generate_move_arrow_vertices(GizmoAxis::Z));
            let (rotate_x_buf, rotate_x_count) =
                upload!(generate_rotate_ring_vertices(GizmoAxis::X));
            let (rotate_y_buf, rotate_y_count) =
                upload!(generate_rotate_ring_vertices(GizmoAxis::Y));
            let (rotate_z_buf, rotate_z_count) =
                upload!(generate_rotate_ring_vertices(GizmoAxis::Z));
            let (scale_x_buf, scale_x_count) =
                upload!(generate_scale_handle_vertices(GizmoAxis::X));
            let (scale_y_buf, scale_y_count) =
                upload!(generate_scale_handle_vertices(GizmoAxis::Y));
            let (scale_z_buf, scale_z_count) =
                upload!(generate_scale_handle_vertices(GizmoAxis::Z));

            info!("GizmoPipeline created");
            Ok(Self {
                device: device.clone(),
                pipeline,
                pipeline_layout,
                _vert_shader: vert_shader,
                _frag_shader: frag_shader,
                crosshair_buf,
                crosshair_count,
                move_x_buf,
                move_x_count,
                move_y_buf,
                move_y_count,
                move_z_buf,
                move_z_count,
                rotate_x_buf,
                rotate_x_count,
                rotate_y_buf,
                rotate_y_count,
                rotate_z_buf,
                rotate_z_count,
                scale_x_buf,
                scale_x_count,
                scale_y_buf,
                scale_y_count,
                scale_z_buf,
                scale_z_count,
                hovered_gizmo_axis,
            })
        }

        /// Record gizmo draw commands for all entities with a Transform.
        ///
        /// Caller must have already opened a render pass.
        /// Viewport/scissor dynamic state must be set by the caller before
        /// this function is invoked (GizmoPipeline does not set them — the
        /// surrounding render loop sets them once per viewport sub-rect).
        pub unsafe fn record(
            &self,
            cmd: vk::CommandBuffer,
            world: &engine_core::World,
            selected_entity_id: Option<u64>,
            mode: GizmoMode,
            view_proj: glam::Mat4,
            camera_pos: glam::Vec3,
        ) {
            let hover_raw = self.hovered_gizmo_axis.load(std::sync::atomic::Ordering::Relaxed);

            let device = &self.device;
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);

            for entity in world.entities() {
                let Some(transform) =
                    world.get::<engine_core::Transform>(entity)
                else {
                    continue;
                };

                let origin = transform.position;
                let dist = (camera_pos - origin).length().max(0.1);
                let scale = dist * 0.15;

                // ── Mode handles (selected entity only) ──────────────────
                let is_selected = selected_entity_id.map_or(false, |id| {
                    if id > u32::MAX as u64 {
                        return false; // can't match a u32 entity id
                    }
                    entity.id() == id as u32
                });
                if is_selected {
                    // ── Crosshair (selected entity only) ─────────────────
                    // X axis — red
                    // Vertex buffer layout: X0,X1,Y0,Y1,Z0,Z1 (2 verts each).
                    // We use firstVertex to select which pair to draw:
                    // X=0, Y=2, Z=4.
                    self.draw_buf(
                        cmd,
                        device,
                        &self.crosshair_buf,
                        view_proj,
                        origin.into(),
                        { let mut c = axis_color(GizmoAxis::X, hover_raw == 1); c[3] = 0.9; c },
                        scale,
                        0,
                        2,
                    );
                    // Y axis — green
                    self.draw_buf(
                        cmd,
                        device,
                        &self.crosshair_buf,
                        view_proj,
                        origin.into(),
                        { let mut c = axis_color(GizmoAxis::Y, hover_raw == 2); c[3] = 0.9; c },
                        scale,
                        2,
                        2,
                    );
                    // Z axis — blue
                    self.draw_buf(
                        cmd,
                        device,
                        &self.crosshair_buf,
                        view_proj,
                        origin.into(),
                        { let mut c = axis_color(GizmoAxis::Z, hover_raw == 3); c[3] = 0.9; c },
                        scale,
                        4,
                        2,
                    );

                    match mode {
                        GizmoMode::Move => {
                            self.draw_buf(cmd, device, &self.move_x_buf, view_proj, origin.into(), axis_color(GizmoAxis::X, hover_raw == 1), scale, 0, self.move_x_count);
                            self.draw_buf(cmd, device, &self.move_y_buf, view_proj, origin.into(), axis_color(GizmoAxis::Y, hover_raw == 2), scale, 0, self.move_y_count);
                            self.draw_buf(cmd, device, &self.move_z_buf, view_proj, origin.into(), axis_color(GizmoAxis::Z, hover_raw == 3), scale, 0, self.move_z_count);
                        }
                        GizmoMode::Rotate => {
                            self.draw_buf(cmd, device, &self.rotate_x_buf, view_proj, origin.into(), axis_color(GizmoAxis::X, hover_raw == 1), scale, 0, self.rotate_x_count);
                            self.draw_buf(cmd, device, &self.rotate_y_buf, view_proj, origin.into(), axis_color(GizmoAxis::Y, hover_raw == 2), scale, 0, self.rotate_y_count);
                            self.draw_buf(cmd, device, &self.rotate_z_buf, view_proj, origin.into(), axis_color(GizmoAxis::Z, hover_raw == 3), scale, 0, self.rotate_z_count);
                        }
                        GizmoMode::Scale => {
                            self.draw_buf(cmd, device, &self.scale_x_buf, view_proj, origin.into(), axis_color(GizmoAxis::X, hover_raw == 1), scale, 0, self.scale_x_count);
                            self.draw_buf(cmd, device, &self.scale_y_buf, view_proj, origin.into(), axis_color(GizmoAxis::Y, hover_raw == 2), scale, 0, self.scale_y_count);
                            self.draw_buf(cmd, device, &self.scale_z_buf, view_proj, origin.into(), axis_color(GizmoAxis::Z, hover_raw == 3), scale, 0, self.scale_z_count);
                        }
                    }
                }
            }
        }

        /// Push constants and issue a single line-list draw.
        #[allow(clippy::too_many_arguments)]
        unsafe fn draw_buf(
            &self,
            cmd: vk::CommandBuffer,
            device: &ash::Device,
            buf: &GpuBuffer,
            view_proj: glam::Mat4,
            origin: [f32; 3],
            color: [f32; 4],
            scale: f32,
            first_vertex: u32,
            vertex_count: u32,
        ) {
            device.cmd_bind_vertex_buffers(cmd, 0, &[buf.handle()], &[0]);
            let pc = GizmoPushConstants {
                view_proj: view_proj.to_cols_array_2d(),
                origin,
                _pad0: 0.0,
                color,
                scale,
                _pad1: [0.0; 3],
            };
            let pc_bytes = std::slice::from_raw_parts(
                &pc as *const GizmoPushConstants as *const u8,
                std::mem::size_of::<GizmoPushConstants>(),
            );
            device.cmd_push_constants(
                cmd,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                pc_bytes,
            );
            device.cmd_draw(cmd, vertex_count, 1, first_vertex, 0);
        }
    }

    impl Drop for GizmoPipeline {
        fn drop(&mut self) {
            unsafe {
                self.device.destroy_pipeline(self.pipeline, None);
                self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // Vulkan pipeline creation
    // ─────────────────────────────────────────────────────────────────────

    fn create_gizmo_pipeline(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        vert: &ShaderModule,
        frag: &ShaderModule,
    ) -> Result<(vk::Pipeline, vk::PipelineLayout), String> {
        let stages = [vert.stage_create_info(), frag.stage_create_info()];
        let binding = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(std::mem::size_of::<GizmoVertex>() as u32) // 12 bytes
            .input_rate(vk::VertexInputRate::VERTEX);
        let attrs = [vk::VertexInputAttributeDescription::default()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)];

        // Dummy viewport/scissor — overridden by dynamic state at draw time.
        let vp =
            vk::Viewport::default().width(1.0).height(1.0).max_depth(1.0);
        let sc =
            vk::Rect2D::default().extent(vk::Extent2D { width: 1, height: 1 });

        // 112-byte push constant block shared by both stages.
        let push_range = vk::PushConstantRange::default()
            .stage_flags(
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            )
            .offset(0)
            .size(std::mem::size_of::<GizmoPushConstants>() as u32);

        let layout = unsafe {
            device
                .create_pipeline_layout(
                    &vk::PipelineLayoutCreateInfo::default()
                        .push_constant_ranges(std::slice::from_ref(&push_range)),
                    None,
                )
                .map_err(|e| format!("gizmo pipeline layout: {e}"))?
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
                                .vertex_binding_descriptions(std::slice::from_ref(
                                    &binding,
                                ))
                                .vertex_attribute_descriptions(&attrs),
                        )
                        .input_assembly_state(
                            &vk::PipelineInputAssemblyStateCreateInfo::default()
                                .topology(vk::PrimitiveTopology::LINE_LIST),
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
                            // No depth test so gizmos are always visible.
                            &vk::PipelineDepthStencilStateCreateInfo::default()
                                .depth_test_enable(false)
                                .depth_write_enable(false),
                        )
                        .color_blend_state(
                            &vk::PipelineColorBlendStateCreateInfo::default()
                                .attachments(std::slice::from_ref(&blend_attachment)),
                        )
                        .dynamic_state(
                            &vk::PipelineDynamicStateCreateInfo::default()
                                .dynamic_states(&[
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
                    format!("gizmo pipeline: {e}")
                })?[0]
        };

        info!("GizmoPipeline Vulkan pipeline created (LINE_LIST, no depth test)");
        Ok((pipeline, layout))
    }

    // ─────────────────────────────────────────────────────────────────────
    // Tests
    // ─────────────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn crosshair_generates_6_vertices() {
            let verts = generate_crosshair_vertices();
            assert_eq!(verts.len(), 6);
        }

        #[test]
        fn move_arrow_generates_nonzero_vertices() {
            let verts = generate_move_arrow_vertices(GizmoAxis::X);
            assert!(!verts.is_empty());
        }

        #[test]
        fn move_arrow_vertex_count_is_even_for_line_list() {
            for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                let verts = generate_move_arrow_vertices(axis);
                assert_eq!(
                    verts.len() % 2,
                    0,
                    "move arrow for {axis:?} must have even vertex count"
                );
            }
        }

        #[test]
        fn rotate_ring_vertex_count_is_even_for_line_list() {
            for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                let verts = generate_rotate_ring_vertices(axis);
                assert_eq!(
                    verts.len() % 2,
                    0,
                    "rotate ring for {axis:?} must have even vertex count"
                );
                assert!(!verts.is_empty());
            }
        }

        #[test]
        fn scale_handle_vertex_count_is_even_for_line_list() {
            for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                let verts = generate_scale_handle_vertices(axis);
                assert_eq!(
                    verts.len() % 2,
                    0,
                    "scale handle for {axis:?} must have even vertex count"
                );
                assert!(!verts.is_empty());
            }
        }

        #[test]
        fn push_constants_are_112_bytes() {
            assert_eq!(std::mem::size_of::<GizmoPushConstants>(), 112);
        }

        #[test]
        fn gizmo_vert_shader_compiles() {
            let result =
                compile_glsl_to_spirv(GIZMO_VERT_GLSL, naga::ShaderStage::Vertex);
            assert!(result.is_ok(), "gizmo.vert failed to compile: {:?}", result);
        }

        #[test]
        fn gizmo_frag_shader_compiles() {
            let result =
                compile_glsl_to_spirv(GIZMO_FRAG_GLSL, naga::ShaderStage::Fragment);
            assert!(result.is_ok(), "gizmo.frag failed to compile: {:?}", result);
        }

        #[test]
        fn test_axis_color_hover() {
            let normal_x = axis_color(GizmoAxis::X, false);
            let hovered_x = axis_color(GizmoAxis::X, true);
            // All RGB channels should be brighter when hovered
            assert!(hovered_x[0] >= normal_x[0]);
            assert!(hovered_x[1] >= normal_x[1]);
            assert!(hovered_x[2] >= normal_x[2]);
            // Alpha unchanged
            assert_eq!(hovered_x[3], 1.0);
            // At least one channel must actually be brighter
            let any_brighter = hovered_x[0] > normal_x[0]
                || hovered_x[1] > normal_x[1]
                || hovered_x[2] > normal_x[2];
            assert!(any_brighter);
            // No channel exceeds 1.0
            assert!(hovered_x[0] <= 1.0);
            assert!(hovered_x[1] <= 1.0);
            assert!(hovered_x[2] <= 1.0);
        }

        #[test]
        fn test_axis_color_z_channel() {
            // Z axis green channel is 0.4, NOT 0.2 — guard against future incorrect "normalisation"
            let z = axis_color(GizmoAxis::Z, false);
            assert_eq!(z[1], 0.4, "Z axis G channel must be 0.4");
        }
    }
} // mod imp

// ─────────────────────────────────────────────────────────────────────────────
// Re-exports (available on all platforms so bridge/gizmo_commands.rs can
// import GizmoAxis / GizmoMode without cfg(windows) guards everywhere).
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(windows)]
pub use imp::{
    generate_crosshair_vertices, generate_move_arrow_vertices,
    generate_rotate_ring_vertices, generate_scale_handle_vertices, GizmoAxis,
    GizmoMode, GizmoPipeline, GizmoVertex,
};

// On non-Windows platforms expose only the pure-Rust types (no Vulkan needed).
#[cfg(not(windows))]
mod portable {
    use glam::Vec3;

    /// Which principal axis a gizmo handle is aligned to.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub enum GizmoAxis {
        X,
        Y,
        Z,
        XY,
        XZ,
        YZ,
    }

    /// Which transform manipulation mode is active.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub enum GizmoMode {
        Move,
        Rotate,
        Scale,
    }

    /// One vertex in gizmo-local space.
    #[repr(C)]
    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    pub struct GizmoVertex {
        pub pos: [f32; 3],
    }

    fn axis_dir(axis: GizmoAxis) -> Vec3 {
        match axis {
            GizmoAxis::X => Vec3::X,
            GizmoAxis::Y => Vec3::Y,
            GizmoAxis::Z => Vec3::Z,
            GizmoAxis::XY => (Vec3::X + Vec3::Y).normalize(),
            GizmoAxis::XZ => (Vec3::X + Vec3::Z).normalize(),
            GizmoAxis::YZ => (Vec3::Y + Vec3::Z).normalize(),
        }
    }

    fn perpendicular(v: Vec3) -> Vec3 {
        let candidate = if v.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        v.cross(candidate).normalize()
    }

    /// Returns the RGBA colour for a gizmo axis, brightened when hovered.
    fn axis_color(axis: GizmoAxis, hovered: bool) -> [f32; 4] {
        let base: [f32; 4] = match axis {
            GizmoAxis::X => [1.0, 0.2, 0.2, 1.0],
            GizmoAxis::Y => [0.2, 1.0, 0.2, 1.0],
            GizmoAxis::Z => [0.2, 0.4, 1.0, 1.0],
            _ => [0.8, 0.8, 0.8, 1.0],
        };
        if hovered {
            [
                (base[0] + 0.35).min(1.0),
                (base[1] + 0.35).min(1.0),
                (base[2] + 0.35).min(1.0),
                1.0,
            ]
        } else {
            base
        }
    }

    pub fn generate_crosshair_vertices() -> Vec<GizmoVertex> {
        vec![
            GizmoVertex { pos: [0.0, 0.0, 0.0] },
            GizmoVertex { pos: [1.0, 0.0, 0.0] },
            GizmoVertex { pos: [0.0, 0.0, 0.0] },
            GizmoVertex { pos: [0.0, 1.0, 0.0] },
            GizmoVertex { pos: [0.0, 0.0, 0.0] },
            GizmoVertex { pos: [0.0, 0.0, 1.0] },
        ]
    }

    pub fn generate_move_arrow_vertices(axis: GizmoAxis) -> Vec<GizmoVertex> {
        let dir = axis_dir(axis);
        let perp1 = perpendicular(dir);
        let perp2 = dir.cross(perp1);
        let shaft_tip = dir * 0.8;
        let arrow_tip = dir * 1.0;
        const CONE_R: f32 = 0.06;
        const SPOKES: usize = 8;
        let mut verts = Vec::new();
        verts.push(GizmoVertex { pos: [0.0, 0.0, 0.0] });
        verts.push(GizmoVertex { pos: shaft_tip.into() });
        for i in 0..SPOKES {
            let angle = (i as f32) * std::f32::consts::TAU / (SPOKES as f32);
            let (s, c) = angle.sin_cos();
            let rim = shaft_tip + perp1 * (c * CONE_R) + perp2 * (s * CONE_R);
            verts.push(GizmoVertex { pos: rim.into() });
            verts.push(GizmoVertex { pos: arrow_tip.into() });
            let next_angle =
                ((i + 1) as f32) * std::f32::consts::TAU / (SPOKES as f32);
            let (ns, nc) = next_angle.sin_cos();
            let next_rim =
                shaft_tip + perp1 * (nc * CONE_R) + perp2 * (ns * CONE_R);
            verts.push(GizmoVertex { pos: rim.into() });
            verts.push(GizmoVertex { pos: next_rim.into() });
        }
        verts
    }

    pub fn generate_rotate_ring_vertices(axis: GizmoAxis) -> Vec<GizmoVertex> {
        let dir = axis_dir(axis);
        let perp1 = perpendicular(dir);
        let perp2 = dir.cross(perp1);
        const SEGMENTS: usize = 32;
        const RADIUS: f32 = 1.0;
        let mut verts = Vec::new();
        for i in 0..SEGMENTS {
            let a0 = (i as f32) * std::f32::consts::TAU / (SEGMENTS as f32);
            let a1 = ((i + 1) as f32) * std::f32::consts::TAU / (SEGMENTS as f32);
            let p0 = perp1 * (a0.cos() * RADIUS) + perp2 * (a0.sin() * RADIUS);
            let p1 = perp1 * (a1.cos() * RADIUS) + perp2 * (a1.sin() * RADIUS);
            verts.push(GizmoVertex { pos: p0.into() });
            verts.push(GizmoVertex { pos: p1.into() });
        }
        verts
    }

    pub fn generate_scale_handle_vertices(axis: GizmoAxis) -> Vec<GizmoVertex> {
        let dir = axis_dir(axis);
        let perp1 = perpendicular(dir);
        let perp2 = dir.cross(perp1);
        let shaft_tip = dir * 0.85;
        const HALF: f32 = 0.06;
        let mut verts = Vec::new();
        verts.push(GizmoVertex { pos: [0.0, 0.0, 0.0] });
        verts.push(GizmoVertex { pos: shaft_tip.into() });
        let corners: Vec<Vec3> = [[-1.0f32, -1.0, -1.0], [1.0, -1.0, -1.0],
                        [1.0, 1.0, -1.0], [-1.0, 1.0, -1.0],
                        [-1.0, -1.0, 1.0], [1.0, -1.0, 1.0],
                        [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0]]
            .iter()
            .map(|o| {
                shaft_tip
                    + dir * (o[2] * HALF)
                    + perp1 * (o[0] * HALF)
                    + perp2 * (o[1] * HALF)
            })
            .collect();
        let edges = [
            (0, 1), (1, 2), (2, 3), (3, 0),
            (4, 5), (5, 6), (6, 7), (7, 4),
            (0, 4), (1, 5), (2, 6), (3, 7),
        ];
        for (a, b) in edges {
            verts.push(GizmoVertex { pos: corners[a].into() });
            verts.push(GizmoVertex { pos: corners[b].into() });
        }
        verts
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn crosshair_generates_6_vertices() {
            let verts = generate_crosshair_vertices();
            assert_eq!(verts.len(), 6);
        }

        #[test]
        fn move_arrow_generates_nonzero_vertices() {
            let verts = generate_move_arrow_vertices(GizmoAxis::X);
            assert!(!verts.is_empty());
        }

        #[test]
        fn crosshair_vertices_are_even_count() {
            assert_eq!(generate_crosshair_vertices().len() % 2, 0);
        }

        #[test]
        fn move_arrow_vertices_are_even_count() {
            for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                assert_eq!(generate_move_arrow_vertices(axis).len() % 2, 0,
                    "move arrow vertices must be even for LINE_LIST on axis {:?}", axis);
            }
        }

        #[test]
        fn rotate_ring_vertices_are_even_count() {
            for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                assert_eq!(generate_rotate_ring_vertices(axis).len() % 2, 0,
                    "rotate ring vertices must be even for LINE_LIST on axis {:?}", axis);
            }
        }

        #[test]
        fn scale_handle_vertices_are_even_count() {
            for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
                assert_eq!(generate_scale_handle_vertices(axis).len() % 2, 0,
                    "scale handle vertices must be even for LINE_LIST on axis {:?}", axis);
            }
        }

        #[test]
        fn test_axis_color_hover() {
            let normal_x = axis_color(GizmoAxis::X, false);
            let hovered_x = axis_color(GizmoAxis::X, true);
            // All RGB channels should be brighter when hovered
            assert!(hovered_x[0] >= normal_x[0]);
            assert!(hovered_x[1] >= normal_x[1]);
            assert!(hovered_x[2] >= normal_x[2]);
            // Alpha unchanged
            assert_eq!(hovered_x[3], 1.0);
            // At least one channel must actually be brighter
            let any_brighter = hovered_x[0] > normal_x[0]
                || hovered_x[1] > normal_x[1]
                || hovered_x[2] > normal_x[2];
            assert!(any_brighter);
            // No channel exceeds 1.0
            assert!(hovered_x[0] <= 1.0);
            assert!(hovered_x[1] <= 1.0);
            assert!(hovered_x[2] <= 1.0);
        }

        #[test]
        fn test_axis_color_z_channel() {
            // Z axis green channel is 0.4, NOT 0.2 — guard against future incorrect "normalisation"
            let z = axis_color(GizmoAxis::Z, false);
            assert_eq!(z[1], 0.4, "Z axis G channel must be 0.4");
        }
    }
}

#[cfg(not(windows))]
pub use portable::{
    generate_crosshair_vertices, generate_move_arrow_vertices,
    generate_rotate_ring_vertices, generate_scale_handle_vertices, GizmoAxis,
    GizmoMode, GizmoVertex,
};
