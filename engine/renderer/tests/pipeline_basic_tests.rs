//! Basic integration tests for rendering pipeline modules
//!
//! Simplified tests that verify core functionality works correctly.

use engine_renderer::*;

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();
}

fn create_test_context() -> Option<VulkanContext> {
    VulkanContext::new("BasicPipelineTest", None, None).ok()
}

#[test]
fn test_render_pass_and_framebuffer_creation() {
    init_tracing();

    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create render pass
    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: ash::vk::Format::B8G8R8A8_SRGB,
            depth_format: None,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .expect("Failed to create render pass");

    // Create offscreen target
    let target = OffscreenTarget::new(&context, 1920, 1080, None, false)
        .expect("Failed to create offscreen target");

    // Create framebuffer
    let framebuffer = Framebuffer::new(
        &context.device,
        render_pass.handle(),
        target.color_image_view,
        ash::vk::Extent2D { width: 1920, height: 1080 },
    )
    .expect("Failed to create framebuffer");

    // Verify handles
    assert_ne!(render_pass.handle(), ash::vk::RenderPass::null());
    assert_ne!(framebuffer.handle(), ash::vk::Framebuffer::null());

    tracing::info!("Render pass and framebuffer creation test passed");
}

#[test]
fn test_batch_framebuffer_creation() {
    init_tracing();

    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: ash::vk::Format::B8G8R8A8_SRGB,
            depth_format: None,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .expect("Failed to create render pass");

    // Create multiple targets
    let targets: Vec<_> = (0..3)
        .map(|_| {
            OffscreenTarget::new(&context, 1920, 1080, None, false)
                .expect("Failed to create offscreen target")
        })
        .collect();

    let image_views: Vec<_> = targets.iter().map(|t| t.color_image_view).collect();

    // Create framebuffers
    let framebuffers = create_framebuffers(
        &context.device,
        render_pass.handle(),
        &image_views,
        ash::vk::Extent2D { width: 1920, height: 1080 },
    )
    .expect("Failed to create framebuffers");

    assert_eq!(framebuffers.len(), 3);

    tracing::info!("Batch framebuffer creation test passed");
}

#[test]
fn test_sync_objects_creation() {
    init_tracing();

    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Single sync object
    let sync = FrameSyncObjects::new(&context.device).expect("Failed to create sync objects");

    assert_ne!(sync.image_available(), ash::vk::Semaphore::null());
    assert_ne!(sync.render_finished(), ash::vk::Semaphore::null());
    assert_ne!(sync.fence(), ash::vk::Fence::null());

    // Multiple sync objects (frames in flight)
    let sync_objects =
        create_sync_objects(&context.device, 2).expect("Failed to create sync objects");

    assert_eq!(sync_objects.len(), 2);

    tracing::info!("Sync objects creation test passed");
}

#[test]
fn test_fence_wait_and_reset() {
    init_tracing();

    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let sync = FrameSyncObjects::new(&context.device).expect("Failed to create sync objects");

    // Fence starts in signaled state, so first wait succeeds immediately
    sync.wait(&context.device, u64::MAX).expect("Failed to wait for fence");

    // After reset, fence is unsignaled
    // We won't wait again since we haven't submitted GPU work to signal it
    sync.reset(&context.device).expect("Failed to reset fence");

    // Verify we can create multiple sync objects
    for _ in 0..10 {
        let _sync2 = FrameSyncObjects::new(&context.device).expect("Failed to create sync objects");
    }

    tracing::info!("Fence wait and reset test passed");
}

#[test]
fn test_command_pool_and_buffers() {
    init_tracing();

    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create command pool with reset flag
    let pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    // Allocate command buffers
    let buffers = pool
        .allocate(&context.device, ash::vk::CommandBufferLevel::PRIMARY, 3)
        .expect("Failed to allocate command buffers");

    assert_eq!(buffers.len(), 3);

    tracing::info!("Command pool and buffers test passed");
}

#[test]
fn test_full_pipeline_setup() {
    init_tracing();

    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // 1. Render pass
    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: ash::vk::Format::B8G8R8A8_SRGB,
            depth_format: None,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .expect("Failed to create render pass");

    // 2. Offscreen targets (simulating swapchain)
    let targets: Vec<_> = (0..3)
        .map(|_| {
            OffscreenTarget::new(&context, 1920, 1080, None, false)
                .expect("Failed to create offscreen target")
        })
        .collect();

    let image_views: Vec<_> = targets.iter().map(|t| t.color_image_view).collect();

    // 3. Framebuffers
    let framebuffers = create_framebuffers(
        &context.device,
        render_pass.handle(),
        &image_views,
        ash::vk::Extent2D { width: 1920, height: 1080 },
    )
    .expect("Failed to create framebuffers");

    // 4. Command pool
    let pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    // 5. Command buffers
    let cmd_buffers = pool
        .allocate(&context.device, ash::vk::CommandBufferLevel::PRIMARY, 3)
        .expect("Failed to allocate command buffers");

    // 6. Sync objects
    let sync_objects =
        create_sync_objects(&context.device, 2).expect("Failed to create sync objects");

    // Verify all components
    assert_eq!(framebuffers.len(), 3);
    assert_eq!(cmd_buffers.len(), 3);
    assert_eq!(sync_objects.len(), 2);

    tracing::info!("Full pipeline setup test passed");
}

#[test]
fn test_offscreen_target_variations() {
    init_tracing();

    let context = match create_test_context() {
        Some(ctx) => ctx,
        None => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Different resolutions
    let _target_720p = OffscreenTarget::new(&context, 1280, 720, None, false)
        .expect("Failed to create 720p target");

    let _target_1080p = OffscreenTarget::new(&context, 1920, 1080, None, false)
        .expect("Failed to create 1080p target");

    let _target_4k = OffscreenTarget::new(&context, 3840, 2160, None, false)
        .expect("Failed to create 4K target");

    // With depth
    let target_with_depth = OffscreenTarget::new(&context, 1920, 1080, None, true)
        .expect("Failed to create target with depth");

    assert!(target_with_depth.has_depth());
    assert!(target_with_depth.depth_image.is_some());
    assert!(target_with_depth.depth_image_view.is_some());

    tracing::info!("Offscreen target variations test passed");
}
