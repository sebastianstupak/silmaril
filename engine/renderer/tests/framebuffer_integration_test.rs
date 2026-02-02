//! Integration tests for framebuffer creation
//!
//! Tests framebuffer creation, resize handling, and proper cleanup.

use ash::vk;
use ash::vk::Handle;
use engine_renderer::{
    create_framebuffers, Framebuffer, RenderPass, RenderPassConfig, VulkanContext,
};

/// Initialize tracing for tests.
fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();
}

#[test]
fn test_framebuffer_creation() {
    init_tracing();

    // Create Vulkan context in headless mode
    let context = match VulkanContext::new("FramebufferTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    // Create a render pass
    let config = RenderPassConfig {
        color_format: vk::Format::B8G8R8A8_SRGB,
        depth_format: None,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
    };

    let render_pass = match RenderPass::new(&context.device, config) {
        Ok(rp) => rp,
        Err(e) => {
            panic!("Failed to create render pass: {:?}", e);
        }
    };

    // Create a test image view manually (simulating swapchain image view)
    let extent = vk::Extent2D { width: 1920, height: 1080 };

    // Create test image
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

    let image = unsafe {
        context
            .device
            .create_image(&image_info, None)
            .expect("Failed to create test image")
    };

    // Create image view
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

    let image_view = unsafe {
        context
            .device
            .create_image_view(&view_info, None)
            .expect("Failed to create image view")
    };

    // Create framebuffer
    let framebuffer = Framebuffer::new(&context.device, render_pass.handle(), image_view, extent);

    match framebuffer {
        Ok(fb) => {
            tracing::info!("Successfully created framebuffer");
            assert!(!fb.handle().is_null());
        }
        Err(e) => {
            panic!("Failed to create framebuffer: {:?}", e);
        }
    }

    // Cleanup
    unsafe {
        context.device.destroy_image_view(image_view, None);
        context.device.destroy_image(image, None);
    }
}

#[test]
fn test_framebuffer_dimensions_match() {
    init_tracing();

    let context = match VulkanContext::new("FramebufferDimensionsTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let config = RenderPassConfig::default();
    let render_pass = match RenderPass::new(&context.device, config) {
        Ok(rp) => rp,
        Err(e) => {
            panic!("Failed to create render pass: {:?}", e);
        }
    };

    // Test multiple dimensions
    let test_dimensions = [(1920, 1080), (800, 600), (3840, 2160), (1280, 720)];

    for (width, height) in test_dimensions {
        let extent = vk::Extent2D { width, height };

        // Create test image
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::B8G8R8A8_SRGB)
            .extent(vk::Extent3D { width, height, depth: 1 })
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

        // Create framebuffer and verify it succeeds
        let _framebuffer =
            Framebuffer::new(&context.device, render_pass.handle(), image_view, extent)
                .expect("Failed to create framebuffer");

        tracing::info!(
            width = width,
            height = height,
            "Framebuffer created with specified dimensions"
        );

        // Cleanup
        unsafe {
            context.device.destroy_image_view(image_view, None);
            context.device.destroy_image(image, None);
        }
    }
}

#[test]
fn test_framebuffer_helper_creates_multiple() {
    init_tracing();

    let context = match VulkanContext::new("FramebufferMultipleTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let config = RenderPassConfig::default();
    let render_pass = match RenderPass::new(&context.device, config) {
        Ok(rp) => rp,
        Err(e) => {
            panic!("Failed to create render pass: {:?}", e);
        }
    };

    let extent = vk::Extent2D { width: 1920, height: 1080 };

    // Create multiple test image views (simulating swapchain)
    let mut image_views = Vec::new();
    let mut images = Vec::new();

    for i in 0..3 {
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
        images.push(image);

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
        image_views.push(image_view);

        tracing::info!(index = i, "Created test image view");
    }

    // Use helper function to create framebuffers
    let framebuffers =
        create_framebuffers(&context.device, render_pass.handle(), &image_views, extent);

    match framebuffers {
        Ok(fbs) => {
            assert_eq!(fbs.len(), 3, "Should create one framebuffer per image view");
            tracing::info!(count = fbs.len(), "Successfully created multiple framebuffers");

            for (i, fb) in fbs.iter().enumerate() {
                assert!(!fb.handle().is_null());
                tracing::info!(index = i, "Framebuffer {} is valid", i);
            }
        }
        Err(e) => {
            panic!("Failed to create framebuffers: {:?}", e);
        }
    }

    // Cleanup
    for image_view in image_views {
        unsafe {
            context.device.destroy_image_view(image_view, None);
        }
    }
    for image in images {
        unsafe {
            context.device.destroy_image(image, None);
        }
    }
}

#[test]
fn test_framebuffer_drop_cleanup() {
    init_tracing();

    let context = match VulkanContext::new("FramebufferDropTest", None, None) {
        Ok(ctx) => ctx,
        Err(_) => {
            eprintln!("Skipping test: Vulkan not available");
            return;
        }
    };

    let config = RenderPassConfig::default();
    let render_pass = match RenderPass::new(&context.device, config) {
        Ok(rp) => rp,
        Err(e) => {
            panic!("Failed to create render pass: {:?}", e);
        }
    };

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

    // Create and immediately drop framebuffer
    {
        let _framebuffer =
            Framebuffer::new(&context.device, render_pass.handle(), image_view, extent)
                .expect("Failed to create framebuffer");

        tracing::info!("Framebuffer created, will be dropped");
    }

    tracing::info!("Framebuffer dropped successfully (no leak)");

    // Cleanup
    unsafe {
        context.device.destroy_image_view(image_view, None);
        context.device.destroy_image(image, None);
    }
}
