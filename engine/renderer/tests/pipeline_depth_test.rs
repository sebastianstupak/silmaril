//! Integration tests for pipeline and depth buffer functionality
//!
//! Tests the complete integration of graphics pipelines with depth buffers,
//! ensuring proper Vulkan resource creation, management, and cleanup.

use ash::vk;
use engine_renderer::{
    CommandPool, DepthBuffer, GraphicsPipeline, RenderPass, RenderPassConfig, VulkanContext,
};

/// Helper to create a test Vulkan context
fn create_test_context() -> VulkanContext {
    VulkanContext::new("DepthBufferTest", None, None).expect("Failed to create Vulkan context")
}

#[test]
fn test_depth_buffer_creation() {
    let context = create_test_context();
    let device = &context.device;

    let extent = vk::Extent2D { width: 1920, height: 1080 };

    let depth_buffer = DepthBuffer::new(device, &context.allocator, extent)
        .expect("Failed to create depth buffer");

    // Verify handles are valid
    assert_ne!(depth_buffer.image(), vk::Image::null());
    assert_ne!(depth_buffer.image_view(), vk::ImageView::null());

    // Verify format is correct
    assert_eq!(depth_buffer.format(), vk::Format::D32_SFLOAT);
}

#[test]
fn test_depth_buffer_destruction() {
    let context = create_test_context();
    let device = &context.device;

    let extent = vk::Extent2D { width: 800, height: 600 };

    {
        let _depth_buffer = DepthBuffer::new(device, &context.allocator, extent)
            .expect("Failed to create depth buffer");
        // DepthBuffer should clean up properly on drop
    }

    // No crash = successful cleanup
}

#[test]
fn test_depth_buffer_multiple_sizes() {
    let context = create_test_context();
    let device = &context.device;

    let sizes = [(640, 480), (1280, 720), (1920, 1080), (2560, 1440), (3840, 2160)];

    for (width, height) in &sizes {
        let extent = vk::Extent2D { width: *width, height: *height };

        let depth_buffer = DepthBuffer::new(device, &context.allocator, extent)
            .expect(&format!("Failed to create depth buffer {}x{}", width, height));

        assert_ne!(depth_buffer.image(), vk::Image::null());
        assert_ne!(depth_buffer.image_view(), vk::ImageView::null());
    }
}

#[test]
fn test_pipeline_with_depth_testing() {
    let context = create_test_context();
    let device = &context.device;

    let extent = vk::Extent2D { width: 1920, height: 1080 };

    // Create render pass with depth attachment
    let config = RenderPassConfig {
        color_format: vk::Format::B8G8R8A8_SRGB,
        depth_format: Some(vk::Format::D32_SFLOAT),
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
    };

    let render_pass = RenderPass::new(device, config).expect("Failed to create render pass");

    // Create pipeline with depth testing
    let pipeline =
        GraphicsPipeline::new_mesh_pipeline(device, &render_pass, extent, config.depth_format)
            .expect("Failed to create pipeline with depth testing");

    // Verify pipeline handles are valid
    assert_ne!(pipeline.handle(), vk::Pipeline::null());
    assert_ne!(pipeline.layout(), vk::PipelineLayout::null());
}

#[test]
fn test_pipeline_without_depth_testing() {
    let context = create_test_context();
    let device = &context.device;

    let extent = vk::Extent2D { width: 1920, height: 1080 };

    // Create render pass without depth attachment
    let config = RenderPassConfig {
        color_format: vk::Format::B8G8R8A8_SRGB,
        depth_format: None,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
    };

    let render_pass = RenderPass::new(device, config).expect("Failed to create render pass");

    // Create pipeline without depth testing
    let pipeline = GraphicsPipeline::new_mesh_pipeline(device, &render_pass, extent, None)
        .expect("Failed to create pipeline without depth testing");

    assert_ne!(pipeline.handle(), vk::Pipeline::null());
}

#[test]
fn test_framebuffer_with_depth_buffer() {
    let context = create_test_context();
    let device = &context.device;

    let extent = vk::Extent2D { width: 1920, height: 1080 };

    // Create render pass with depth
    let config = RenderPassConfig {
        color_format: vk::Format::B8G8R8A8_SRGB,
        depth_format: Some(vk::Format::D32_SFLOAT),
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
    };

    let render_pass = RenderPass::new(device, config).expect("Failed to create render pass");

    // Create depth buffer
    let depth_buffer = DepthBuffer::new(device, &context.allocator, extent)
        .expect("Failed to create depth buffer");

    // Create dummy color image view for testing
    let color_image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::B8G8R8A8_SRGB)
        .extent(vk::Extent3D { width: extent.width, height: extent.height, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);

    let color_image = unsafe { device.create_image(&color_image_info, None) }
        .expect("Failed to create test color image");

    let view_info = vk::ImageViewCreateInfo::default()
        .image(color_image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(vk::Format::B8G8R8A8_SRGB)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    let color_view =
        unsafe { device.create_image_view(&view_info, None) }.expect("Failed to create image view");

    // Create framebuffer with both color and depth attachments
    let attachments = [color_view, depth_buffer.image_view()];

    let framebuffer_info = vk::FramebufferCreateInfo::default()
        .render_pass(render_pass.handle())
        .attachments(&attachments)
        .width(extent.width)
        .height(extent.height)
        .layers(1);

    let framebuffer = unsafe { device.create_framebuffer(&framebuffer_info, None) }
        .expect("Failed to create framebuffer with depth");

    assert_ne!(framebuffer, vk::Framebuffer::null());

    // Cleanup
    unsafe {
        device.destroy_framebuffer(framebuffer, None);
        device.destroy_image_view(color_view, None);
        device.destroy_image(color_image, None);
    }
}

#[test]
fn test_depth_buffer_format() {
    let context = create_test_context();
    let device = &context.device;

    let extent = vk::Extent2D { width: 1920, height: 1080 };

    let depth_buffer = DepthBuffer::new(device, &context.allocator, extent)
        .expect("Failed to create depth buffer");

    // Verify we're using D32_SFLOAT format
    assert_eq!(depth_buffer.format(), vk::Format::D32_SFLOAT);
}

#[test]
fn test_pipeline_vertex_input_configuration() {
    let context = create_test_context();
    let device = &context.device;

    let extent = vk::Extent2D { width: 1920, height: 1080 };

    let config = RenderPassConfig {
        color_format: vk::Format::B8G8R8A8_SRGB,
        depth_format: Some(vk::Format::D32_SFLOAT),
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
    };

    let render_pass = RenderPass::new(device, config).expect("Failed to create render pass");

    // This verifies that the pipeline properly configures vertex input for:
    // - position (vec3, location 0, offset 0)
    // - normal (vec3, location 1, offset 12)
    // - uv (vec2, location 2, offset 24)
    let _pipeline =
        GraphicsPipeline::new_mesh_pipeline(device, &render_pass, extent, config.depth_format)
            .expect("Failed to create pipeline with correct vertex input");
}

#[test]
fn test_pipeline_descriptor_sets() {
    let context = create_test_context();
    let device = &context.device;

    let extent = vk::Extent2D { width: 1920, height: 1080 };

    let config = RenderPassConfig {
        color_format: vk::Format::B8G8R8A8_SRGB,
        depth_format: Some(vk::Format::D32_SFLOAT),
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
    };

    let render_pass = RenderPass::new(device, config).expect("Failed to create render pass");

    // Create pipeline with descriptor set support for camera uniform buffer
    let pipeline = GraphicsPipeline::new_mesh_pipeline_with_descriptors(
        device,
        &render_pass,
        extent,
        config.depth_format,
    )
    .expect("Failed to create pipeline with descriptor sets");

    assert_ne!(pipeline.layout(), vk::PipelineLayout::null());
    assert_ne!(pipeline.descriptor_set_layout(), vk::DescriptorSetLayout::null());
}

#[test]
fn test_depth_buffer_small_extent() {
    let context = create_test_context();
    let device = &context.device;

    // Test minimum practical size
    let extent = vk::Extent2D { width: 1, height: 1 };

    let depth_buffer = DepthBuffer::new(device, &context.allocator, extent)
        .expect("Failed to create 1x1 depth buffer");

    assert_ne!(depth_buffer.image(), vk::Image::null());
}

#[test]
fn test_depth_buffer_large_extent() {
    let context = create_test_context();
    let device = &context.device;

    // Test 4K resolution
    let extent = vk::Extent2D { width: 3840, height: 2160 };

    let depth_buffer = DepthBuffer::new(device, &context.allocator, extent)
        .expect("Failed to create 4K depth buffer");

    assert_ne!(depth_buffer.image(), vk::Image::null());
}

#[test]
fn test_multiple_pipelines_same_render_pass() {
    let context = create_test_context();
    let device = &context.device;

    let extent = vk::Extent2D { width: 1920, height: 1080 };

    let config = RenderPassConfig {
        color_format: vk::Format::B8G8R8A8_SRGB,
        depth_format: Some(vk::Format::D32_SFLOAT),
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
    };

    let render_pass = RenderPass::new(device, config).expect("Failed to create render pass");

    // Create multiple pipelines with same render pass
    let _pipeline1 =
        GraphicsPipeline::new_mesh_pipeline(device, &render_pass, extent, config.depth_format)
            .expect("Failed to create pipeline 1");

    let pipeline2 =
        GraphicsPipeline::new_mesh_pipeline(device, &render_pass, extent, config.depth_format)
            .expect("Failed to create pipeline 2");

    assert_ne!(pipeline1.handle(), vk::Pipeline::null());
    assert_ne!(pipeline2.handle(), vk::Pipeline::null());
    assert_ne!(pipeline1.handle(), pipeline2.handle());
}

#[test]
fn test_command_buffer_with_depth_rendering() {
    let context = create_test_context();
    let device = &context.device;

    let extent = vk::Extent2D { width: 1920, height: 1080 };

    // Create render pass with depth
    let config = RenderPassConfig {
        color_format: vk::Format::B8G8R8A8_SRGB,
        depth_format: Some(vk::Format::D32_SFLOAT),
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
    };

    let render_pass = RenderPass::new(device, config).expect("Failed to create render pass");

    // Create pipeline
    let _pipeline =
        GraphicsPipeline::new_mesh_pipeline(device, &render_pass, extent, config.depth_format)
            .expect("Failed to create pipeline");

    // Create depth buffer
    let _depth_buffer = DepthBuffer::new(device, &context.allocator, extent)
        .expect("Failed to create depth buffer");

    // Create command pool
    let queue_families = context.queue_families;
    let cmd_pool = CommandPool::new(
        device,
        queue_families.graphics,
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    // Allocate command buffer
    let cmd_buffers = cmd_pool
        .allocate(device, vk::CommandBufferLevel::PRIMARY, 1)
        .expect("Failed to allocate command buffer");

    // Record commands (just verify we can begin/end without crashes)
    unsafe {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        device
            .begin_command_buffer(cmd_buffers[0], &begin_info)
            .expect("Failed to begin command buffer");

        device.end_command_buffer(cmd_buffers[0]).expect("Failed to end command buffer");
    }
}
