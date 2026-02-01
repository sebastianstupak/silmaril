//! Integration tests for Vulkan render pass creation

use ash::vk;
use engine_renderer::{RenderPass, RenderPassConfig, VulkanContext};

#[test]
fn test_render_pass_creation() {
    // Create Vulkan context
    let context =
        VulkanContext::new("RenderPassTest", None, None).expect("Failed to create Vulkan context");

    let config = RenderPassConfig {
        color_format: vk::Format::B8G8R8A8_SRGB,
        depth_format: None,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
    };

    let render_pass =
        RenderPass::new(&context.device, config).expect("Render pass creation should succeed");

    // Test: Handle is valid (non-null)
    assert_ne!(render_pass.handle(), vk::RenderPass::null());

    drop(render_pass);
    // If we get here, cleanup succeeded
}

#[test]
fn test_render_pass_default_config() {
    let config = RenderPassConfig::default();

    assert_eq!(config.color_format, vk::Format::B8G8R8A8_SRGB);
    assert_eq!(config.depth_format, None);
    assert_eq!(config.samples, vk::SampleCountFlags::TYPE_1);
    assert_eq!(config.load_op, vk::AttachmentLoadOp::CLEAR);
    assert_eq!(config.store_op, vk::AttachmentStoreOp::STORE);
}

#[test]
fn test_render_pass_different_formats() {
    let context =
        VulkanContext::new("RenderPassTest", None, None).expect("Failed to create Vulkan context");

    // Test various color formats
    let formats =
        vec![vk::Format::B8G8R8A8_SRGB, vk::Format::R8G8B8A8_SRGB, vk::Format::B8G8R8A8_UNORM];

    for format in formats {
        let config = RenderPassConfig {
            color_format: format,
            depth_format: None,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
        };

        let render_pass = RenderPass::new(&context.device, config)
            .unwrap_or_else(|_| panic!("Should create render pass for format {:?}", format));

        assert_ne!(render_pass.handle(), vk::RenderPass::null());
    }
}
