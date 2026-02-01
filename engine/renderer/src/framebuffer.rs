//! Vulkan framebuffer management
//!
//! Framebuffers link render passes to swapchain image views, defining the actual
//! images that will be rendered to during a render pass.

use ash::vk;
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use tracing::{debug, info};

// Framebuffer errors using define_error! macro per CLAUDE.md
define_error! {
    pub enum FramebufferError {
        CreationFailed { details: String } =
            ErrorCode::FramebufferCreationFailed,
            ErrorSeverity::Error,
    }
}

/// Vulkan framebuffer
///
/// Links a render pass to specific image views. One framebuffer is typically
/// created for each swapchain image. The framebuffer is automatically destroyed
/// when dropped.
pub struct Framebuffer {
    framebuffer: vk::Framebuffer,
    device: ash::Device,
}

impl Framebuffer {
    /// Create a framebuffer for a swapchain image view
    ///
    /// The framebuffer links the render pass to a specific image view,
    /// defining where rendering output will go.
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    /// * `render_pass` - Render pass this framebuffer is compatible with
    /// * `image_view` - Image view to render into
    /// * `extent` - Framebuffer dimensions (must match image extent)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use engine_renderer::Framebuffer;
    /// use ash::vk;
    ///
    /// # let device: ash::Device = todo!();
    /// # let render_pass = vk::RenderPass::null();
    /// # let image_view = vk::ImageView::null();
    /// let framebuffer = Framebuffer::new(
    ///     &device,
    ///     render_pass,
    ///     image_view,
    ///     vk::Extent2D { width: 1920, height: 1080 },
    /// )?;
    /// # Ok::<(), engine_renderer::FramebufferError>(())
    /// ```
    pub fn new(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        image_view: vk::ImageView,
        extent: vk::Extent2D,
    ) -> Result<Self, FramebufferError> {
        debug!(width = extent.width, height = extent.height, "Creating framebuffer");

        let attachments = [image_view];

        let framebuffer_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        let framebuffer =
            unsafe { device.create_framebuffer(&framebuffer_info, None) }.map_err(|e| {
                FramebufferError::creationfailed(format!("vkCreateFramebuffer failed: {}", e))
            })?;

        debug!(framebuffer = ?framebuffer, "Framebuffer created successfully");

        Ok(Self { framebuffer, device: device.clone() })
    }

    /// Get the raw framebuffer handle
    #[inline]
    pub fn handle(&self) -> vk::Framebuffer {
        self.framebuffer
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        debug!("Destroying framebuffer");
        unsafe {
            self.device.destroy_framebuffer(self.framebuffer, None);
        }
    }
}

/// Helper function to create framebuffers for all swapchain images
///
/// Creates one framebuffer for each image view in the swapchain.
/// This is the typical usage pattern for presentation.
///
/// # Arguments
///
/// * `device` - Logical Vulkan device
/// * `render_pass` - Render pass for the framebuffers
/// * `image_views` - Image views from swapchain
/// * `extent` - Swapchain extent
///
/// # Example
///
/// ```no_run
/// use engine_renderer::create_framebuffers;
/// use ash::vk;
///
/// # let device: ash::Device = todo!();
/// # let render_pass = vk::RenderPass::null();
/// # let image_views: Vec<vk::ImageView> = vec![];
/// # let extent = vk::Extent2D { width: 1920, height: 1080 };
/// let framebuffers = create_framebuffers(
///     &device,
///     render_pass,
///     &image_views,
///     extent,
/// )?;
/// # Ok::<(), engine_renderer::FramebufferError>(())
/// ```
pub fn create_framebuffers(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    image_views: &[vk::ImageView],
    extent: vk::Extent2D,
) -> Result<Vec<Framebuffer>, FramebufferError> {
    info!(
        count = image_views.len(),
        width = extent.width,
        height = extent.height,
        "Creating framebuffers for swapchain images"
    );

    image_views
        .iter()
        .map(|&image_view| Framebuffer::new(device, render_pass, image_view, extent))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framebuffer_error_display() {
        let err = FramebufferError::creationfailed("test error".to_string());
        let msg = err.to_string();
        assert!(msg.contains("CreationFailed") || msg.contains("test error"));
    }
}
