//! Vulkan render pass management
//!
//! Provides render pass creation and configuration for the rendering pipeline.
//! Based on Vulkan Tutorial: https://vulkan-tutorial.com/Drawing_a_triangle/Graphics_pipeline_basics/Render_passes

use ash::vk;
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use tracing::{debug, info};

// Render pass errors using define_error! macro per CLAUDE.md
define_error! {
    pub enum RenderPassError {
        CreationFailed { details: String } =
            ErrorCode::RenderPassCreationFailed,
            ErrorSeverity::Error,
    }
}

/// Render pass configuration
///
/// Defines how rendering operations are structured, including attachment formats,
/// load/store operations, and layout transitions.
#[derive(Debug, Clone, Copy)]
pub struct RenderPassConfig {
    /// Color attachment format (typically swapchain format)
    pub color_format: vk::Format,
    /// Optional depth attachment format (None for 2D rendering)
    pub depth_format: Option<vk::Format>,
    /// MSAA sample count
    pub samples: vk::SampleCountFlags,
    /// Load operation for color attachment (CLEAR for fresh frame)
    pub load_op: vk::AttachmentLoadOp,
    /// Store operation for color attachment (STORE to present)
    pub store_op: vk::AttachmentStoreOp,
}

impl Default for RenderPassConfig {
    fn default() -> Self {
        Self {
            color_format: vk::Format::B8G8R8A8_SRGB,
            depth_format: None,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
        }
    }
}

/// Vulkan render pass
///
/// Describes the structure of rendering operations including attachments,
/// subpasses, and dependencies. The render pass is automatically destroyed when dropped.
pub struct RenderPass {
    render_pass: vk::RenderPass,
    device: ash::Device,
}

impl RenderPass {
    /// Create a new render pass for swapchain rendering
    ///
    /// Creates a render pass with a single subpass and one color attachment.
    /// The render pass is configured for:
    /// - CLEAR on load (for fresh frames)
    /// - STORE on store (to present to screen)
    /// - Transition from UNDEFINED to PRESENT_SRC_KHR layout
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    /// * `config` - Render pass configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use engine_renderer::{RenderPass, RenderPassConfig};
    /// use ash::vk;
    ///
    /// # let device: ash::Device = todo!();
    /// let config = RenderPassConfig {
    ///     color_format: vk::Format::B8G8R8A8_SRGB,
    ///     depth_format: None,
    ///     samples: vk::SampleCountFlags::TYPE_1,
    ///     load_op: vk::AttachmentLoadOp::CLEAR,
    ///     store_op: vk::AttachmentStoreOp::STORE,
    /// };
    ///
    /// let render_pass = RenderPass::new(&device, config)?;
    /// # Ok::<(), engine_renderer::RenderPassError>(())
    /// ```
    pub fn new(device: &ash::Device, config: RenderPassConfig) -> Result<Self, RenderPassError> {
        info!(
            format = ?config.color_format,
            depth_format = ?config.depth_format,
            samples = ?config.samples,
            "Creating render pass"
        );

        // Define color attachment
        let color_attachment = vk::AttachmentDescription::default()
            .format(config.color_format)
            .samples(config.samples)
            .load_op(config.load_op)
            .store_op(config.store_op)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        // Reference to color attachment in subpass
        let color_attachment_ref = vk::AttachmentReference::default()
            .attachment(0) // Index in attachment descriptions array
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        // Optional depth attachment
        let mut attachments_vec = vec![color_attachment];
        let depth_attachment_ref;
        let has_depth = config.depth_format.is_some();

        if let Some(depth_format) = config.depth_format {
            let depth_attachment = vk::AttachmentDescription::default()
                .format(depth_format)
                .samples(config.samples)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::DONT_CARE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

            depth_attachment_ref = vk::AttachmentReference::default()
                .attachment(1) // Second attachment
                .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

            attachments_vec.push(depth_attachment);
        } else {
            depth_attachment_ref = vk::AttachmentReference::default();
        }

        // Define subpass with optional depth
        let color_attachments = [color_attachment_ref];
        let mut subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments);

        if has_depth {
            subpass = subpass.depth_stencil_attachment(&depth_attachment_ref);
        }

        // Subpass dependency for layout transitions
        let mut src_stage = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
        let mut dst_stage = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
        let mut dst_access = vk::AccessFlags::COLOR_ATTACHMENT_WRITE;

        if has_depth {
            src_stage |= vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS;
            dst_stage |= vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS;
            dst_access |= vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE;
        }

        let dependency = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(src_stage)
            .dst_stage_mask(dst_stage)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(dst_access);

        // Create render pass
        let subpasses = [subpass];
        let dependencies = [dependency];

        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments_vec)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        let render_pass =
            unsafe { device.create_render_pass(&render_pass_info, None) }.map_err(|e| {
                RenderPassError::creationfailed(format!("vkCreateRenderPass failed: {}", e))
            })?;

        debug!(render_pass = ?render_pass, "Render pass created successfully");

        Ok(Self { render_pass, device: device.clone() })
    }

    /// Get the raw render pass handle
    #[inline]
    pub fn handle(&self) -> vk::RenderPass {
        self.render_pass
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        debug!("Destroying render pass");
        unsafe {
            self.device.destroy_render_pass(self.render_pass, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_pass_config_default() {
        let config = RenderPassConfig::default();

        assert_eq!(config.color_format, vk::Format::B8G8R8A8_SRGB);
        assert_eq!(config.depth_format, None);
        assert_eq!(config.samples, vk::SampleCountFlags::TYPE_1);
        assert_eq!(config.load_op, vk::AttachmentLoadOp::CLEAR);
        assert_eq!(config.store_op, vk::AttachmentStoreOp::STORE);
    }

    #[test]
    fn test_render_pass_error_display() {
        let err = RenderPassError::creationfailed("test error".to_string());
        let msg = err.to_string();
        assert!(msg.contains("CreationFailed") || msg.contains("test error"));
    }
}
