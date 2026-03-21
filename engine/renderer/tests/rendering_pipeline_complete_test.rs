//! Complete rendering pipeline integration test
//!
//! Tests the full Phase 1.6 rendering pipeline:
//! - Window creation
//! - Vulkan context initialization
//! - Surface creation
//! - Swapchain creation
//! - Render pass creation
//! - Framebuffer creation
//! - Command pool/buffer allocation
//! - Synchronization object creation
//! - Frame rendering (clear color)
//!
//! This validates that all Phase 1.6 modules work together correctly.

use engine_renderer::*;

#[test]
fn test_complete_rendering_pipeline() {
    // Create window
    let window_config = WindowConfig {
        title: "Phase 1.6 Complete Test".to_string(),
        width: 800,
        height: 600,
        resizable: false,
        visible: false, // Headless for CI
    };

    let window = Window::new(window_config).expect("Failed to create window");
    let (width, height) = window.size();
    assert_eq!(width, 800);
    assert_eq!(height, 600);

    // Create Vulkan entry
    let entry = unsafe { ash::Entry::load().expect("Failed to load Vulkan") };

    // Create Vulkan context (headless for testing)
    let context =
        VulkanContext::new("Phase1.6Test", None, None).expect("Failed to create Vulkan context");

    // Create surface
    let surface =
        Surface::new(&entry, &context.instance, &window).expect("Failed to create surface");

    // Create swapchain
    let swapchain =
        Swapchain::new(&context, surface.handle(), surface.loader(), width, height, None)
            .expect("Failed to create swapchain");
    assert!(swapchain.image_count >= 2, "Swapchain should have at least 2 images");
    assert_eq!(swapchain.extent.width, 800);
    assert_eq!(swapchain.extent.height, 600);

    // Create render pass
    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: swapchain.format,
            depth_format: None, // Phase 1.6 doesn't need depth
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .expect("Failed to create render pass");

    // Create framebuffers (one per swapchain image)
    let framebuffers = create_framebuffers(
        &context.device,
        render_pass.handle(),
        &swapchain.image_views,
        swapchain.extent,
    )
    .expect("Failed to create framebuffers");
    assert_eq!(
        framebuffers.len(),
        swapchain.image_count as usize,
        "Should have one framebuffer per swapchain image"
    );

    // Create command pool
    let command_pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    // Allocate command buffers (2 for frames in flight)
    let command_buffers = command_pool
        .allocate(&context.device, ash::vk::CommandBufferLevel::PRIMARY, 2)
        .expect("Failed to allocate command buffers");
    assert_eq!(command_buffers.len(), 2);

    // Create synchronization objects (2 frames in flight)
    let sync_objects =
        create_sync_objects(&context.device, 2).expect("Failed to create sync objects");
    assert_eq!(sync_objects.len(), 2);

    // Test recording a command buffer
    let cmd_buffer = CommandBuffer::from_handle(command_buffers[0]);
    cmd_buffer
        .begin(&context.device, ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
        .expect("Failed to begin command buffer");

    cmd_buffer.begin_render_pass(
        &context.device,
        render_pass.handle(),
        framebuffers[0].handle(),
        swapchain.extent,
        [0.1, 0.2, 0.3, 1.0], // Blue-ish clear color
    );

    cmd_buffer.end_render_pass(&context.device);

    cmd_buffer.end(&context.device).expect("Failed to end command buffer");

    // Wait for device idle before cleanup
    context.wait_idle().expect("Failed to wait for device idle");

    // Cleanup happens automatically via Drop implementations
}

#[test]
fn test_multiple_frame_rendering() {
    // Create minimal renderer setup
    let window_config = WindowConfig {
        title: "Multi-Frame Test".to_string(),
        width: 640,
        height: 480,
        resizable: false,
        visible: false,
    };

    let window = Window::new(window_config).expect("Failed to create window");
    let (width, height) = window.size();

    let entry = unsafe { ash::Entry::load().expect("Failed to load Vulkan") };
    let context =
        VulkanContext::new("MultiFrameTest", None, None).expect("Failed to create Vulkan context");

    let surface =
        Surface::new(&entry, &context.instance, &window).expect("Failed to create surface");

    let swapchain =
        Swapchain::new(&context, surface.handle(), surface.loader(), width, height, None)
            .expect("Failed to create swapchain");

    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: swapchain.format,
            depth_format: None,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .expect("Failed to create render pass");

    let framebuffers = create_framebuffers(
        &context.device,
        render_pass.handle(),
        &swapchain.image_views,
        swapchain.extent,
    )
    .expect("Failed to create framebuffers");

    let command_pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    let command_buffers = command_pool
        .allocate(&context.device, ash::vk::CommandBufferLevel::PRIMARY, 2)
        .expect("Failed to allocate command buffers");

    let sync_objects =
        create_sync_objects(&context.device, 2).expect("Failed to create sync objects");

    // Simulate multiple frames (without actual presentation)
    for frame in 0..10 {
        let current_frame = frame % 2;
        let sync = &sync_objects[current_frame];

        // Wait for previous frame
        sync.wait(&context.device, u64::MAX).expect("Failed to wait for fence");

        // Reset fence
        sync.reset(&context.device).expect("Failed to reset fence");

        // Record command buffer
        let cmd = CommandBuffer::from_handle(command_buffers[current_frame]);
        cmd.begin(&context.device, ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .expect("Failed to begin command buffer");

        // Use different clear colors per frame
        let clear_color = [(frame as f32 * 0.1) % 1.0, 0.2, 0.3, 1.0];

        cmd.begin_render_pass(
            &context.device,
            render_pass.handle(),
            framebuffers[current_frame % framebuffers.len()].handle(),
            swapchain.extent,
            clear_color,
        );

        cmd.end_render_pass(&context.device);

        cmd.end(&context.device).expect("Failed to end command buffer");

        // Submit command buffer
        let wait_semaphores = [sync.image_available()];
        let signal_semaphores = [sync.render_finished()];
        let wait_stages = [ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let command_buffers_to_submit = [cmd.handle()];

        let submit_info = ash::vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers_to_submit)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            context
                .device
                .queue_submit(context.graphics_queue, &[submit_info], sync.fence())
                .expect("Failed to submit queue");
        }
    }

    // Wait for all frames to complete
    context.wait_idle().expect("Failed to wait for device idle");
}

#[test]
fn test_render_pass_compatibility() {
    // Test that render pass is compatible with framebuffer
    let entry = unsafe { ash::Entry::load().expect("Failed to load Vulkan") };
    let context = VulkanContext::new("RenderPassCompatTest", None, None)
        .expect("Failed to create Vulkan context");

    // Create a dummy swapchain-like setup
    let format = ash::vk::Format::B8G8R8A8_UNORM;
    let extent = ash::vk::Extent2D { width: 1920, height: 1080 };

    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: format,
            depth_format: None,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .expect("Failed to create render pass");

    // Verify render pass handle is valid
    assert_ne!(
        render_pass.handle(),
        ash::vk::RenderPass::null(),
        "Render pass handle should not be null"
    );

    context.wait_idle().expect("Failed to wait for device idle");
}

#[test]
fn test_command_buffer_lifecycle() {
    // Test command buffer recording and reset
    let context = VulkanContext::new("CommandBufferLifecycle", None, None)
        .expect("Failed to create Vulkan context");

    let command_pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    let buffers = command_pool
        .allocate(&context.device, ash::vk::CommandBufferLevel::PRIMARY, 1)
        .expect("Failed to allocate command buffers");

    let cmd = CommandBuffer::from_handle(buffers[0]);

    // Test begin/end cycle
    cmd.begin(&context.device, ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
        .expect("Failed to begin command buffer");
    cmd.end(&context.device).expect("Failed to end command buffer");

    // Test multiple begin/end cycles (with reset)
    for _ in 0..5 {
        cmd.begin(&context.device, ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            .expect("Failed to begin command buffer");
        cmd.end(&context.device).expect("Failed to end command buffer");

        // Reset command buffer for next iteration
        unsafe {
            context
                .device
                .reset_command_buffer(
                    cmd.handle(),
                    ash::vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .expect("Failed to reset command buffer");
        }
    }

    context.wait_idle().expect("Failed to wait for device idle");
}

#[test]
fn test_sync_objects_creation() {
    // Test sync objects are created correctly
    let context =
        VulkanContext::new("SyncObjectsTest", None, None).expect("Failed to create Vulkan context");

    let sync_objects =
        create_sync_objects(&context.device, 3).expect("Failed to create sync objects");

    assert_eq!(sync_objects.len(), 3, "Should create 3 sync objects");

    for (i, sync) in sync_objects.iter().enumerate() {
        assert_ne!(
            sync.image_available(),
            ash::vk::Semaphore::null(),
            "Semaphore {} should not be null",
            i
        );
        assert_ne!(
            sync.render_finished(),
            ash::vk::Semaphore::null(),
            "Semaphore {} should not be null",
            i
        );
        assert_ne!(sync.fence(), ash::vk::Fence::null(), "Fence {} should not be null", i);
    }

    context.wait_idle().expect("Failed to wait for device idle");
}

#[test]
#[ignore] // Visual test - run manually
fn test_render_clear_color_visual() {
    // This test opens a window and renders for a few seconds
    // Run with: cargo test test_render_clear_color_visual -- --ignored --nocapture

    let window_config = WindowConfig {
        title: "Phase 1.6 Visual Test - Red Clear Color".to_string(),
        width: 800,
        height: 600,
        resizable: true,
        visible: true, // Actually visible
    };

    let mut renderer =
        Renderer::new(window_config, "Phase1.6VisualTest").expect("Failed to create renderer");

    renderer.set_clear_color(1.0, 0.0, 0.0, 1.0); // Red

    // Render for 3 seconds (180 frames at 60 FPS)
    let start = std::time::Instant::now();
    let mut frame_count = 0;

    while start.elapsed().as_secs() < 3 {
        if let Some(w) = renderer.window_mut() {
            w.poll_events();
        }

        if renderer.window().map_or(false, |w| w.should_close()) {
            break;
        }

        renderer.render_frame().expect("Failed to render frame");
        frame_count += 1;
    }

    let elapsed = start.elapsed().as_secs_f64();
    let fps = frame_count as f64 / elapsed;

    println!("Rendered {} frames in {:.2}s ({:.1} FPS)", frame_count, elapsed, fps);
    assert!(fps >= 55.0, "FPS should be at least 55, got {:.1}", fps);

    renderer.wait_idle().expect("Failed to wait idle");
}
