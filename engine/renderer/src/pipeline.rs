//! Graphics pipeline for mesh rendering
//!
//! Provides Vulkan graphics pipeline with:
//! - Vertex input state (position, normal, UV)
//! - Push constants for MVP matrix
//! - Depth testing
//! - Backface culling

use crate::error::RendererError;
use crate::render_pass::RenderPass;
use crate::shader::ShaderModule;
use ash::vk;
use tracing::{info, instrument};

/// Graphics pipeline for mesh rendering
pub struct GraphicsPipeline {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout,
    descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    device: ash::Device,
}

impl GraphicsPipeline {
    /// Create a graphics pipeline for mesh rendering
    ///
    /// # Arguments
    /// * `device` - Vulkan logical device
    /// * `render_pass` - Render pass to use
    /// * `extent` - Viewport extent
    /// * `depth_format` - Optional depth format (enables depth testing if Some)
    #[instrument(skip(device, render_pass))]
    pub fn new_mesh_pipeline(
        device: &ash::Device,
        render_pass: &RenderPass,
        extent: vk::Extent2D,
        depth_format: Option<vk::Format>,
    ) -> Result<Self, RendererError> {
        info!("Creating mesh graphics pipeline");

        // Load compiled shaders from OUT_DIR
        let out_dir = env!("OUT_DIR");
        let vert_path_str = format!("{}/shaders/mesh.vert.spv", out_dir);
        let frag_path_str = format!("{}/shaders/mesh.frag.spv", out_dir);

        let vert_shader = ShaderModule::from_spirv_file(
            device,
            std::path::Path::new(&vert_path_str),
            vk::ShaderStageFlags::VERTEX,
            "main",
        )?;
        let frag_shader = ShaderModule::from_spirv_file(
            device,
            std::path::Path::new(&frag_path_str),
            vk::ShaderStageFlags::FRAGMENT,
            "main",
        )?;

        // Create pipeline layout with push constants
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(64); // sizeof(mat4) = 64 bytes

        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        let layout = unsafe {
            device.create_pipeline_layout(&layout_info, None).map_err(|e| {
                RendererError::pipelinecreationfailed(format!(
                    "Failed to create pipeline layout: {:?}",
                    e
                ))
            })?
        };

        // Shader stages
        let stages = [vert_shader.stage_create_info(), frag_shader.stage_create_info()];

        // Vertex input state (matches engine_assets::Vertex layout)
        let binding_description = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(32) // sizeof(Vertex) = 32 bytes (3*4 + 3*4 + 2*4)
            .input_rate(vk::VertexInputRate::VERTEX);

        let attribute_descriptions = [
            // Position (location 0, offset 0)
            vk::VertexInputAttributeDescription::default()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
            // Normal (location 1, offset 12)
            vk::VertexInputAttributeDescription::default()
                .location(1)
                .binding(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(12),
            // UV (location 2, offset 24)
            vk::VertexInputAttributeDescription::default()
                .location(2)
                .binding(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(24),
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&binding_description))
            .vertex_attribute_descriptions(&attribute_descriptions);

        // Input assembly (triangle list)
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        // Viewport and scissor (dynamic)
        let viewport = vk::Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::default().offset(vk::Offset2D { x: 0, y: 0 }).extent(extent);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(std::slice::from_ref(&viewport))
            .scissors(std::slice::from_ref(&scissor));

        // Rasterization state (backface culling)
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0);

        // Multisample (disabled for now)
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .sample_shading_enable(false);

        // Depth stencil (enabled if depth_format is provided)
        let depth_enabled = depth_format.is_some();
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(depth_enabled)
            .depth_write_enable(depth_enabled)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        // Color blend (opaque rendering)
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA);

        let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(std::slice::from_ref(&color_blend_attachment));

        // Dynamic state (viewport and scissor can be set dynamically)
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        // Create graphics pipeline
        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input_info)
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
            device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_info),
                    None,
                )
                .map_err(|(_, e)| {
                    device.destroy_pipeline_layout(layout, None);
                    RendererError::pipelinecreationfailed(format!(
                        "Failed to create graphics pipeline: {:?}",
                        e
                    ))
                })?[0]
        };

        info!("Mesh graphics pipeline created successfully");

        Ok(Self { pipeline, layout, descriptor_set_layout: None, device: device.clone() })
    }

    /// Create a graphics pipeline with descriptor set support
    ///
    /// This version includes a descriptor set layout for camera uniform buffers.
    ///
    /// # Arguments
    /// * `device` - Vulkan logical device
    /// * `render_pass` - Render pass to use
    /// * `extent` - Viewport extent
    /// * `depth_format` - Optional depth format (enables depth testing if Some)
    #[instrument(skip(device, render_pass))]
    pub fn new_mesh_pipeline_with_descriptors(
        device: &ash::Device,
        render_pass: &RenderPass,
        extent: vk::Extent2D,
        depth_format: Option<vk::Format>,
    ) -> Result<Self, RendererError> {
        info!("Creating mesh graphics pipeline with descriptor sets");

        // Load compiled shaders from OUT_DIR
        let out_dir = env!("OUT_DIR");
        let vert_path_str = format!("{}/shaders/mesh.vert.spv", out_dir);
        let frag_path_str = format!("{}/shaders/mesh.frag.spv", out_dir);

        let vert_shader = ShaderModule::from_spirv_file(
            device,
            std::path::Path::new(&vert_path_str),
            vk::ShaderStageFlags::VERTEX,
            "main",
        )?;
        let frag_shader = ShaderModule::from_spirv_file(
            device,
            std::path::Path::new(&frag_path_str),
            vk::ShaderStageFlags::FRAGMENT,
            "main",
        )?;

        // Create descriptor set layout for camera uniform buffer
        let ubo_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);

        let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(std::slice::from_ref(&ubo_binding));

        let descriptor_set_layout = unsafe {
            device
                .create_descriptor_set_layout(&descriptor_set_layout_info, None)
                .map_err(|e| {
                    RendererError::pipelinecreationfailed(format!(
                        "Failed to create descriptor set layout: {:?}",
                        e
                    ))
                })?
        };

        // Create pipeline layout with descriptor sets and push constants
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(64); // sizeof(mat4) = 64 bytes

        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(std::slice::from_ref(&descriptor_set_layout))
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        let layout = unsafe {
            device.create_pipeline_layout(&layout_info, None).map_err(|e| {
                device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                RendererError::pipelinecreationfailed(format!(
                    "Failed to create pipeline layout: {:?}",
                    e
                ))
            })?
        };

        // Shader stages
        let stages = [vert_shader.stage_create_info(), frag_shader.stage_create_info()];

        // Vertex input state (matches engine_assets::Vertex layout)
        let binding_description = vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(32) // sizeof(Vertex) = 32 bytes (3*4 + 3*4 + 2*4)
            .input_rate(vk::VertexInputRate::VERTEX);

        let attribute_descriptions = [
            // Position (location 0, offset 0)
            vk::VertexInputAttributeDescription::default()
                .location(0)
                .binding(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0),
            // Normal (location 1, offset 12)
            vk::VertexInputAttributeDescription::default()
                .location(1)
                .binding(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(12),
            // UV (location 2, offset 24)
            vk::VertexInputAttributeDescription::default()
                .location(2)
                .binding(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(24),
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&binding_description))
            .vertex_attribute_descriptions(&attribute_descriptions);

        // Input assembly (triangle list)
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        // Viewport and scissor (dynamic)
        let viewport = vk::Viewport::default()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::default().offset(vk::Offset2D { x: 0, y: 0 }).extent(extent);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(std::slice::from_ref(&viewport))
            .scissors(std::slice::from_ref(&scissor));

        // Rasterization state (backface culling)
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0);

        // Multisample (disabled for now)
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .sample_shading_enable(false);

        // Depth stencil (enabled if depth_format is provided)
        let depth_enabled = depth_format.is_some();
        let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(depth_enabled)
            .depth_write_enable(depth_enabled)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);

        // Color blend (opaque rendering)
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(false)
            .color_write_mask(vk::ColorComponentFlags::RGBA);

        let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(std::slice::from_ref(&color_blend_attachment));

        // Dynamic state (viewport and scissor can be set dynamically)
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        // Create graphics pipeline
        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input_info)
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
            device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_info),
                    None,
                )
                .map_err(|(_, e)| {
                    device.destroy_pipeline_layout(layout, None);
                    device.destroy_descriptor_set_layout(descriptor_set_layout, None);
                    RendererError::pipelinecreationfailed(format!(
                        "Failed to create graphics pipeline: {:?}",
                        e
                    ))
                })?[0]
        };

        info!("Mesh graphics pipeline with descriptor sets created successfully");

        Ok(Self {
            pipeline,
            layout,
            descriptor_set_layout: Some(descriptor_set_layout),
            device: device.clone(),
        })
    }

    /// Get the pipeline handle
    #[inline]
    pub fn handle(&self) -> vk::Pipeline {
        self.pipeline
    }

    /// Get the pipeline layout handle
    #[inline]
    pub fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }

    /// Get the descriptor set layout handle (if created with descriptors)
    #[inline]
    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout.unwrap_or(vk::DescriptorSetLayout::null())
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.layout, None);
            if let Some(layout) = self.descriptor_set_layout {
                self.device.destroy_descriptor_set_layout(layout, None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use engine_assets::Vertex;

    #[test]
    fn test_vertex_layout() {
        // Verify Vertex structure layout matches shader expectations

        assert_eq!(std::mem::size_of::<Vertex>(), 32, "Vertex size must be 32 bytes");

        // Verify offsets match shader layout
        let vertex =
            Vertex { position: glam::Vec3::ZERO, normal: glam::Vec3::Z, uv: glam::Vec2::ZERO };

        let vertex_ptr = &vertex as *const Vertex as *const u8;
        let pos_offset =
            &vertex.position as *const glam::Vec3 as *const u8 as usize - vertex_ptr as usize;
        let normal_offset =
            &vertex.normal as *const glam::Vec3 as *const u8 as usize - vertex_ptr as usize;
        let uv_offset = &vertex.uv as *const glam::Vec2 as *const u8 as usize - vertex_ptr as usize;

        assert_eq!(pos_offset, 0, "Position offset must be 0");
        assert_eq!(normal_offset, 12, "Normal offset must be 12");
        assert_eq!(uv_offset, 24, "UV offset must be 24");
    }

    #[test]
    fn test_push_constant_size() {
        // MVP matrix must be exactly 64 bytes
        assert_eq!(std::mem::size_of::<glam::Mat4>(), 64, "Mat4 must be 64 bytes");
    }
}
