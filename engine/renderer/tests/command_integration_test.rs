//! Integration tests for command buffer management
//!
//! Tests command pool creation, command buffer allocation, recording, and reset.

use ash::vk;
use ash::vk::Handle;
use engine_renderer::{
    CommandBuffer, CommandPool, Framebuffer, RenderPass, RenderPassConfig, VulkanContext,
};

/// Initialize tracing for tests.
fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();
}

#[test]
fn test_command_pool_creation() {
    init_tracing();

    let context = match VulkanContext::new("CommandPoolTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create command pool with RESET_COMMAND_BUFFER flag
    let pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    );

    match pool {
        Ok(p) => {
            tracing::info!("Successfully created command pool");
            assert!(!p.handle().is_null());
        }
        Err(e) => {
            panic!("Failed to create command pool: {:?}", e);
        }
    }
}

#[test]
fn test_command_buffer_allocation() {
    init_tracing();

    let context = match VulkanContext::new("CommandBufferAllocTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    // Allocate PRIMARY command buffers
    let buffers = pool.allocate(&context.device, vk::CommandBufferLevel::PRIMARY, 3);

    match buffers {
        Ok(bufs) => {
            assert_eq!(bufs.len(), 3, "Should allocate requested number of buffers");
            tracing::info!(count = bufs.len(), "Successfully allocated command buffers");

            for (i, &buf) in bufs.iter().enumerate() {
                assert!(!buf.is_null(), "Buffer {} should not be null", i);
            }
        }
        Err(e) => {
            panic!("Failed to allocate command buffers: {:?}", e);
        }
    }
}

#[test]
fn test_command_buffer_recording() {
    init_tracing();

    let context = match VulkanContext::new("CommandRecordingTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    let buffers = pool
        .allocate(&context.device, vk::CommandBufferLevel::PRIMARY, 1)
        .expect("Failed to allocate command buffer");

    let cmd_buffer = CommandBuffer::from_handle(buffers[0]);

    // Test begin and end recording
    let begin_result =
        cmd_buffer.begin(&context.device, vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    assert!(begin_result.is_ok(), "Failed to begin command buffer: {:?}", begin_result.err());

    tracing::info!("Command buffer recording started");

    let end_result = cmd_buffer.end(&context.device);
    assert!(end_result.is_ok(), "Failed to end command buffer: {:?}", end_result.err());

    tracing::info!("Command buffer recording ended successfully");
}

#[test]
fn test_command_pool_reset() {
    init_tracing();

    let context = match VulkanContext::new("CommandPoolResetTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    let buffers = pool
        .allocate(&context.device, vk::CommandBufferLevel::PRIMARY, 2)
        .expect("Failed to allocate command buffers");

    // Record some commands
    for &buffer in &buffers {
        let cmd = CommandBuffer::from_handle(buffer);
        cmd.begin(&context.device, vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .expect("Failed to begin");
        cmd.end(&context.device).expect("Failed to end");
    }

    // Reset the pool
    let reset_result = pool.reset(&context.device);
    assert!(reset_result.is_ok(), "Failed to reset command pool: {:?}", reset_result.err());

    tracing::info!("Command pool reset successfully");

    // Buffers should be implicitly reset and can be re-recorded
    for &buffer in &buffers {
        let cmd = CommandBuffer::from_handle(buffer);
        let begin_result = cmd.begin(&context.device, vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        assert!(begin_result.is_ok(), "Failed to begin after pool reset");
        cmd.end(&context.device).expect("Failed to end");
    }

    tracing::info!("Command buffers re-recorded successfully after pool reset");
}

#[test]
fn test_render_pass_commands() {
    init_tracing();

    let context = match VulkanContext::new("RenderPassCommandsTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create render pass
    let config = RenderPassConfig {
        color_format: vk::Format::B8G8R8A8_SRGB,
        depth_format: None,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
    };

    let render_pass =
        RenderPass::new(&context.device, config).expect("Failed to create render pass");

    // Create test image and framebuffer
    let extent = vk::Extent2D { width: 800, height: 600 };

    let image_info = vk::ImageCreateInfo::default()
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

    let image = unsafe { context.device.create_image(&image_info, None).unwrap() };

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(vk::Format::B8G8R8A8_SRGB)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    let image_view = unsafe { context.device.create_image_view(&view_info, None).unwrap() };

    let framebuffer = Framebuffer::new(&context.device, render_pass.handle(), image_view, extent)
        .expect("Failed to create framebuffer");

    // Create command pool and buffer
    let pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    let buffers = pool
        .allocate(&context.device, vk::CommandBufferLevel::PRIMARY, 1)
        .expect("Failed to allocate command buffer");

    let cmd_buffer = CommandBuffer::from_handle(buffers[0]);

    // Record render pass commands
    cmd_buffer
        .begin(&context.device, vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
        .expect("Failed to begin command buffer");

    // Begin render pass
    cmd_buffer.begin_render_pass(
        &context.device,
        render_pass.handle(),
        framebuffer.handle(),
        extent,
        [0.1, 0.2, 0.3, 1.0], // Clear color
    );

    tracing::info!("Render pass begun successfully");

    // End render pass
    cmd_buffer.end_render_pass(&context.device);

    tracing::info!("Render pass ended successfully");

    cmd_buffer.end(&context.device).expect("Failed to end command buffer");

    tracing::info!("Render pass commands recorded successfully");

    // Cleanup
    unsafe {
        context.device.destroy_image_view(image_view, None);
        context.device.destroy_image(image, None);
    }
}

#[test]
fn test_multiple_command_buffers_parallel() {
    init_tracing();

    let context = match VulkanContext::new("ParallelCommandTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    // Allocate multiple buffers
    let buffers = pool
        .allocate(&context.device, vk::CommandBufferLevel::PRIMARY, 5)
        .expect("Failed to allocate command buffers");

    // Record commands in all buffers
    for (i, &buffer) in buffers.iter().enumerate() {
        let cmd = CommandBuffer::from_handle(buffer);

        cmd.begin(&context.device, vk::CommandBufferUsageFlags::SIMULTANEOUS_USE)
            .expect("Failed to begin");

        // Can add actual commands here if needed

        cmd.end(&context.device).expect("Failed to end");

        tracing::info!(index = i, "Command buffer {} recorded", i);
    }

    tracing::info!("All command buffers recorded successfully");
}

#[test]
fn test_command_buffer_reuse() {
    init_tracing();

    let context = match VulkanContext::new("CommandReuseTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    let buffers = pool
        .allocate(&context.device, vk::CommandBufferLevel::PRIMARY, 1)
        .expect("Failed to allocate command buffer");

    let cmd_buffer = CommandBuffer::from_handle(buffers[0]);

    // Record, reset pool, and record again
    for i in 0..3 {
        cmd_buffer
            .begin(&context.device, vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .expect("Failed to begin");

        cmd_buffer.end(&context.device).expect("Failed to end");

        tracing::info!(iteration = i, "Command buffer recorded iteration {}", i);

        // Reset pool for next iteration
        if i < 2 {
            pool.reset(&context.device).expect("Failed to reset pool");
        }
    }

    tracing::info!("Command buffer reused successfully");
}

#[test]
fn test_secondary_command_buffers() {
    init_tracing();

    let context = match VulkanContext::new("SecondaryCommandTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    // Allocate SECONDARY command buffers
    let buffers = pool
        .allocate(&context.device, vk::CommandBufferLevel::SECONDARY, 2)
        .expect("Failed to allocate secondary command buffers");

    assert_eq!(buffers.len(), 2, "Should allocate 2 secondary buffers");

    for (i, &buffer) in buffers.iter().enumerate() {
        assert!(!buffer.is_null(), "Secondary buffer {} should not be null", i);
        tracing::info!(index = i, "Secondary command buffer {} allocated", i);
    }

    tracing::info!("Secondary command buffers allocated successfully");
}
