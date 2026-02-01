//! Swapchain management for presentation.
//!
//! Handles swapchain creation, recreation, and image acquisition with
//! optimal configuration based on surface capabilities.
//!
//! # Optimizations
//! - Efficient format selection with fallback chain
//! - Smart present mode selection based on performance needs
//! - Automatic resize handling
//! - Minimal allocations during recreation

use crate::context::VulkanContext;
use crate::error::RendererError;
use ash::vk;
use tracing::{info, instrument, warn};

/// Swapchain wrapper with all presentation resources.
pub struct Swapchain {
    /// Swapchain loader (reused across recreations)
    pub loader: ash::khr::swapchain::Device,
    /// Swapchain handle
    pub swapchain: vk::SwapchainKHR,
    /// Swapchain images (owned by swapchain, not destroyed manually)
    pub images: Vec<vk::Image>,
    /// Swapchain image views
    pub image_views: Vec<vk::ImageView>,
    /// Swapchain image format
    pub format: vk::Format,
    /// Swapchain extent
    pub extent: vk::Extent2D,
    /// Present mode
    pub present_mode: vk::PresentModeKHR,
    /// Color space
    pub color_space: vk::ColorSpaceKHR,
    /// Number of images in swapchain
    pub image_count: u32,
}

impl Swapchain {
    /// Create a new swapchain.
    ///
    /// # Arguments
    ///
    /// * `context` - Vulkan context
    /// * `surface` - Window surface
    /// * `surface_loader` - Surface loader
    /// * `width` - Window width
    /// * `height` - Window height
    /// * `old_swapchain` - Optional old swapchain to replace
    #[instrument(skip(context, surface_loader))]
    pub fn new(
        context: &VulkanContext,
        surface: vk::SurfaceKHR,
        surface_loader: &ash::khr::surface::Instance,
        width: u32,
        height: u32,
        old_swapchain: Option<vk::SwapchainKHR>,
    ) -> Result<Self, RendererError> {
        // Get surface capabilities
        // SAFETY: context.physical_device and surface are valid.
        // This query only reads surface properties, doesn't modify state.
        let capabilities = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(context.physical_device, surface)
                .map_err(|e| RendererError::SurfaceCapabilitiesQueryFailed {
                    reason: format!("{:?}", e),
                })?
        };

        // Choose surface format (prefer SRGB)
        let format = choose_surface_format(surface_loader, context.physical_device, surface)?;

        // Choose present mode (prefer MAILBOX for low latency)
        let present_mode = choose_present_mode(surface_loader, context.physical_device, surface)?;

        // Calculate image count
        let image_count = calculate_image_count(&capabilities, present_mode);

        // Calculate extent
        let extent = choose_extent(&capabilities, width, height);

        info!(
            format = ?format.format,
            color_space = ?format.color_space,
            present_mode = ?present_mode,
            image_count = image_count,
            extent = ?extent,
            "Creating swapchain"
        );

        // Create swapchain
        let swapchain_loader = ash::khr::swapchain::Device::new(&context.instance, &context.device);

        let mut create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        // Set image sharing mode
        let queue_family_indices =
            [context.queue_families.graphics, context.queue_families.present];

        if context.queue_families.graphics != context.queue_families.present {
            create_info = create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&queue_family_indices);
        } else {
            create_info = create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE);
        }

        // Set old swapchain if provided
        if let Some(old) = old_swapchain {
            create_info = create_info.old_swapchain(old);
        }

        // SAFETY: create_info is properly initialized with valid surface and device.
        // All references in create_info remain valid for the duration of this call.
        // queue_family_indices array stays in scope.
        let swapchain = unsafe {
            swapchain_loader.create_swapchain(&create_info, None).map_err(|e| {
                RendererError::SwapchainCreationFailed { reason: format!("{:?}", e) }
            })?
        };

        // Get swapchain images
        // SAFETY: swapchain is valid, just created above.
        // The returned images are owned by the swapchain and remain valid until swapchain destruction.
        let images = unsafe {
            swapchain_loader.get_swapchain_images(swapchain).map_err(|e| {
                RendererError::SwapchainImageRetrievalFailed { reason: format!("{:?}", e) }
            })?
        };

        info!(actual_image_count = images.len(), "Swapchain created");

        // Create image views
        let image_views = create_image_views(&context.device, &images, format.format)?;

        Ok(Swapchain {
            loader: swapchain_loader,
            swapchain,
            images,
            image_views,
            format: format.format,
            extent,
            present_mode,
            color_space: format.color_space,
            image_count,
        })
    }

    /// Recreate swapchain after resize or invalidation.
    ///
    /// This reuses the swapchain loader and destroys the old swapchain automatically.
    /// More efficient than creating a new Swapchain from scratch.
    #[instrument(skip(self, context, surface_loader))]
    pub fn recreate(
        &mut self,
        context: &VulkanContext,
        surface: vk::SurfaceKHR,
        surface_loader: &ash::khr::surface::Instance,
        width: u32,
        height: u32,
    ) -> Result<(), RendererError> {
        info!(
            old_extent = ?self.extent,
            new_extent = ?(width, height),
            "Recreating swapchain"
        );

        // Wait for device to be idle before destroying resources
        // SAFETY: context.device is valid. device_wait_idle ensures no operations are in flight.
        unsafe {
            context.device.device_wait_idle().map_err(|e| RendererError::DeviceLost {
                reason: format!("Failed to wait for device idle: {:?}", e),
            })?;
        }

        // Destroy old image views
        for &image_view in &self.image_views {
            // SAFETY: image_view is valid, created by this swapchain. Device is idle.
            unsafe {
                context.device.destroy_image_view(image_view, None);
            }
        }
        self.image_views.clear();

        // Store old swapchain for optimal recreation
        let old_swapchain = self.swapchain;

        // Get surface capabilities
        // SAFETY: context.physical_device and surface are valid.
        // This query only reads surface properties, doesn't modify state.
        let capabilities = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(context.physical_device, surface)
                .map_err(|e| RendererError::SurfaceCapabilitiesQueryFailed {
                    reason: format!("{:?}", e),
                })?
        };

        // Recalculate extent
        let extent = choose_extent(&capabilities, width, height);

        // Reuse existing format and present mode if still valid
        let format = vk::SurfaceFormatKHR { format: self.format, color_space: self.color_space };

        // Verify format is still supported
        // SAFETY: physical_device and surface are valid. This query only reads supported formats.
        let is_format_valid = unsafe {
            surface_loader
                .get_physical_device_surface_formats(context.physical_device, surface)
                .map(|formats| formats.contains(&format))
                .unwrap_or(false)
        };

        let format = if is_format_valid {
            format
        } else {
            warn!("Previous format no longer supported, reselecting");
            choose_surface_format(surface_loader, context.physical_device, surface)?
        };

        // Recalculate image count (may change with new extent)
        let image_count = calculate_image_count(&capabilities, self.present_mode);

        // Create new swapchain
        let mut create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(self.present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain);

        // Set image sharing mode
        let queue_family_indices =
            [context.queue_families.graphics, context.queue_families.present];

        if context.queue_families.graphics != context.queue_families.present {
            create_info = create_info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&queue_family_indices);
        } else {
            create_info = create_info.image_sharing_mode(vk::SharingMode::EXCLUSIVE);
        }

        // SAFETY: create_info is properly initialized. queue_family_indices array stays in scope.
        // old_swapchain (if not null) is valid and we're replacing it.
        let new_swapchain = unsafe {
            self.loader.create_swapchain(&create_info, None).map_err(|e| {
                RendererError::SwapchainCreationFailed { reason: format!("{:?}", e) }
            })?
        };

        // Destroy old swapchain
        // SAFETY: old_swapchain is valid. The new swapchain has been created, so it's safe to destroy the old one.
        // Device is idle (we waited above), so no operations are using the old swapchain.
        unsafe {
            self.loader.destroy_swapchain(old_swapchain, None);
        }

        // Get new swapchain images
        // SAFETY: new_swapchain is valid, just created above.
        let images = unsafe {
            self.loader.get_swapchain_images(new_swapchain).map_err(|e| {
                RendererError::SwapchainImageRetrievalFailed { reason: format!("{:?}", e) }
            })?
        };

        // Create new image views
        let image_views = create_image_views(&context.device, &images, format.format)?;

        // Update swapchain state
        self.swapchain = new_swapchain;
        self.images = images;
        self.image_views = image_views;
        self.format = format.format;
        self.color_space = format.color_space;
        self.extent = extent;
        self.image_count = image_count;

        info!(
            actual_image_count = self.images.len(),
            extent = ?self.extent,
            "Swapchain recreated"
        );

        Ok(())
    }

    /// Acquire next image from swapchain.
    ///
    /// # Returns
    /// - `Ok((image_index, suboptimal))` on success
    /// - `Err(SwapchainOutOfDate)` if swapchain needs recreation
    /// - `Err(...)` on other errors
    ///
    /// # Performance
    /// Uses u64::MAX timeout to avoid busy-waiting (VSync handles timing).
    pub fn acquire_next_image(
        &self,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> Result<(u32, bool), RendererError> {
        // SAFETY: self.swapchain is valid. Semaphore and fence must be valid (caller's responsibility).
        // u64::MAX timeout means wait indefinitely (VSync handles timing).
        unsafe {
            self.loader
                .acquire_next_image(self.swapchain, u64::MAX, semaphore, fence)
                .map_err(|e| match e {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => RendererError::SwapchainOutOfDate {},
                    vk::Result::SUBOPTIMAL_KHR => RendererError::SwapchainSuboptimal {},
                    _ => RendererError::SwapchainCreationFailed {
                        reason: format!("acquire_next_image failed: {:?}", e),
                    },
                })
        }
    }

    /// Acquire next image with custom timeout.
    ///
    /// # Arguments
    /// * `timeout` - Timeout in nanoseconds (u64::MAX for infinite)
    /// * `semaphore` - Semaphore to signal when image is ready
    /// * `fence` - Fence to signal when image is ready
    pub fn acquire_next_image_timeout(
        &self,
        timeout: u64,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> Result<(u32, bool), RendererError> {
        // SAFETY: self.swapchain is valid. Semaphore and fence must be valid (caller's responsibility).
        // timeout is just a u64 value in nanoseconds.
        unsafe {
            self.loader
                .acquire_next_image(self.swapchain, timeout, semaphore, fence)
                .map_err(|e| match e {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => RendererError::SwapchainOutOfDate {},
                    vk::Result::SUBOPTIMAL_KHR => RendererError::SwapchainSuboptimal {},
                    vk::Result::TIMEOUT => RendererError::SwapchainCreationFailed {
                        reason: "Timeout waiting for swapchain image".to_string(),
                    },
                    _ => RendererError::SwapchainCreationFailed {
                        reason: format!("acquire_next_image failed: {:?}", e),
                    },
                })
        }
    }

    /// Present image to swapchain.
    pub fn present(
        &self,
        present_queue: vk::Queue,
        image_index: u32,
        wait_semaphores: &[vk::Semaphore],
    ) -> Result<bool, RendererError> {
        let swapchains = [self.swapchain];
        let image_indices = [image_index];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        // SAFETY: present_queue must be valid (caller's responsibility).
        // present_info contains valid references to swapchains and image_indices arrays in scope.
        // wait_semaphores must be valid (caller's responsibility).
        unsafe {
            self.loader.queue_present(present_queue, &present_info).map_err(|e| match e {
                vk::Result::ERROR_OUT_OF_DATE_KHR => RendererError::SwapchainOutOfDate {},
                vk::Result::SUBOPTIMAL_KHR => RendererError::SwapchainSuboptimal {},
                _ => RendererError::PresentFailed { reason: format!("{:?}", e) },
            })
        }
    }
}

impl Swapchain {
    /// Clean up swapchain resources.
    ///
    /// # Safety
    /// - Must be called with the same device that created the swapchain
    /// - Must not be called if any command buffers are still using these resources
    /// - Should only be called once (typically in Drop or before recreation)
    pub unsafe fn destroy(&mut self, device: &ash::Device) {
        // SAFETY: Caller guarantees device is valid and matches creation device.
        // Image views are destroyed before swapchain per Vulkan object hierarchy.
        unsafe {
            // Destroy image views (owned by us, created from swapchain images)
            for &image_view in &self.image_views {
                device.destroy_image_view(image_view, None);
            }

            // Destroy swapchain (images are owned by swapchain, destroyed automatically)
            self.loader.destroy_swapchain(self.swapchain, None);
        }
    }
}

/// Choose the best surface format with comprehensive fallback chain.
///
/// # Format Preference Order
/// 1. B8G8R8A8_SRGB (most common, hardware-optimized on most platforms)
/// 2. R8G8B8A8_SRGB (alternative SRGB)
/// 3. B8G8R8A8_UNORM (non-SRGB fallback)
/// 4. R8G8B8A8_UNORM (alternative non-SRGB)
/// 5. First available format (last resort)
///
/// All with SRGB_NONLINEAR color space preferred.
#[instrument(skip(surface_loader))]
fn choose_surface_format(
    surface_loader: &ash::khr::surface::Instance,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
) -> Result<vk::SurfaceFormatKHR, RendererError> {
    // SAFETY: physical_device and surface are valid. This query only reads supported formats.
    let formats = unsafe {
        surface_loader
            .get_physical_device_surface_formats(physical_device, surface)
            .map_err(|e| RendererError::SurfaceFormatQueryFailed { reason: format!("{:?}", e) })?
    };

    if formats.is_empty() {
        return Err(RendererError::SurfaceFormatQueryFailed {
            reason: "No surface formats available".to_string(),
        });
    }

    // Comprehensive fallback chain
    const FORMAT_PREFERENCES: &[(vk::Format, vk::ColorSpaceKHR)] = &[
        // SRGB formats (preferred for correct gamma)
        (vk::Format::B8G8R8A8_SRGB, vk::ColorSpaceKHR::SRGB_NONLINEAR),
        (vk::Format::R8G8B8A8_SRGB, vk::ColorSpaceKHR::SRGB_NONLINEAR),
        // Non-SRGB fallbacks
        (vk::Format::B8G8R8A8_UNORM, vk::ColorSpaceKHR::SRGB_NONLINEAR),
        (vk::Format::R8G8B8A8_UNORM, vk::ColorSpaceKHR::SRGB_NONLINEAR),
        // Wide gamut formats (for HDR displays)
        (vk::Format::A2B10G10R10_UNORM_PACK32, vk::ColorSpaceKHR::SRGB_NONLINEAR),
        (vk::Format::A2R10G10B10_UNORM_PACK32, vk::ColorSpaceKHR::SRGB_NONLINEAR),
    ];

    // Try each preference in order
    for &(format, color_space) in FORMAT_PREFERENCES {
        if let Some(&found) =
            formats.iter().find(|f| f.format == format && f.color_space == color_space)
        {
            info!(
                format = ?found.format,
                color_space = ?found.color_space,
                "Selected surface format"
            );
            return Ok(found);
        }
    }

    // Last resort: use first available format
    warn!(
        format = ?formats[0].format,
        color_space = ?formats[0].color_space,
        "Using first available surface format (no preferred format found)"
    );
    Ok(formats[0])
}

/// Choose the best present mode with configurable performance profile.
///
/// # Present Mode Characteristics
/// - **MAILBOX**: Triple buffering, lowest latency, no tearing (best for games)
/// - **IMMEDIATE**: No buffering, lowest latency possible, may tear (competitive gaming)
/// - **FIFO_RELAXED**: Adaptive VSync, smooth when possible (balanced)
/// - **FIFO**: VSync, always available, guaranteed no tearing (power saving)
///
/// # Performance Profile
/// Based on environment variable `RENDERER_PRESENT_MODE`:
/// - `low_latency` → IMMEDIATE > MAILBOX > FIFO_RELAXED > FIFO
/// - `balanced` (default) → MAILBOX > FIFO_RELAXED > FIFO
/// - `power_save` → FIFO_RELAXED > FIFO
#[instrument(skip(surface_loader))]
fn choose_present_mode(
    surface_loader: &ash::khr::surface::Instance,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
) -> Result<vk::PresentModeKHR, RendererError> {
    // SAFETY: physical_device and surface are valid. This query only reads supported present modes.
    let modes = unsafe {
        surface_loader
            .get_physical_device_surface_present_modes(physical_device, surface)
            .map_err(|e| RendererError::PresentModeQueryFailed { reason: format!("{:?}", e) })?
    };

    // Determine performance profile from environment
    let profile = std::env::var("RENDERER_PRESENT_MODE").unwrap_or_else(|_| "balanced".to_string());

    let preference = match profile.as_str() {
        "low_latency" => {
            // Competitive gaming: minimize latency at all costs
            vec![
                vk::PresentModeKHR::IMMEDIATE,
                vk::PresentModeKHR::MAILBOX,
                vk::PresentModeKHR::FIFO_RELAXED,
                vk::PresentModeKHR::FIFO,
            ]
        }
        "power_save" => {
            // Power saving: prefer VSync modes
            vec![vk::PresentModeKHR::FIFO_RELAXED, vk::PresentModeKHR::FIFO]
        }
        _ => {
            // Balanced (default): low latency with no tearing
            vec![
                vk::PresentModeKHR::MAILBOX,
                vk::PresentModeKHR::FIFO_RELAXED,
                vk::PresentModeKHR::FIFO,
            ]
        }
    };

    // Select first available mode from preference list
    for &mode in &preference {
        if modes.contains(&mode) {
            info!(
                mode = ?mode,
                profile = %profile,
                "Selected present mode"
            );
            return Ok(mode);
        }
    }

    // FIFO is guaranteed to be available per Vulkan spec
    info!("Using fallback FIFO present mode (VSync)");
    Ok(vk::PresentModeKHR::FIFO)
}

/// Calculate optimal swapchain image count.
fn calculate_image_count(
    capabilities: &vk::SurfaceCapabilitiesKHR,
    present_mode: vk::PresentModeKHR,
) -> u32 {
    let desired = match present_mode {
        vk::PresentModeKHR::MAILBOX => {
            // MAILBOX typically uses 3-4 images
            capabilities.min_image_count.max(3)
        }
        vk::PresentModeKHR::FIFO | vk::PresentModeKHR::FIFO_RELAXED => {
            // FIFO typically uses 2-3 images (double/triple buffering)
            capabilities.min_image_count.max(2)
        }
        _ => capabilities.min_image_count,
    };

    // Clamp to max if specified (0 means no limit)
    if capabilities.max_image_count > 0 {
        desired.min(capabilities.max_image_count)
    } else {
        desired
    }
}

/// Choose swapchain extent based on window size and capabilities.
fn choose_extent(
    capabilities: &vk::SurfaceCapabilitiesKHR,
    width: u32,
    height: u32,
) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        // Window manager specifies extent
        capabilities.current_extent
    } else {
        // We can choose extent within limits
        vk::Extent2D {
            width: width
                .clamp(capabilities.min_image_extent.width, capabilities.max_image_extent.width),
            height: height
                .clamp(capabilities.min_image_extent.height, capabilities.max_image_extent.height),
        }
    }
}

/// Create image views for swapchain images.
#[instrument(skip(device, images))]
fn create_image_views(
    device: &ash::Device,
    images: &[vk::Image],
    format: vk::Format,
) -> Result<Vec<vk::ImageView>, RendererError> {
    images
        .iter()
        .map(|&image| {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::IDENTITY,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            // SAFETY: device is valid, image is valid (from swapchain), create_info is properly initialized.
            unsafe {
                device.create_image_view(&create_info, None).map_err(|e| {
                    RendererError::ImageViewCreationFailed { reason: format!("{:?}", e) }
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_image_count_mailbox() {
        let capabilities = vk::SurfaceCapabilitiesKHR {
            min_image_count: 2,
            max_image_count: 8,
            ..Default::default()
        };

        let count = calculate_image_count(&capabilities, vk::PresentModeKHR::MAILBOX);
        assert!(count >= 3);
        assert!(count <= 8);
    }

    #[test]
    fn test_calculate_image_count_fifo() {
        let capabilities = vk::SurfaceCapabilitiesKHR {
            min_image_count: 2,
            max_image_count: 4,
            ..Default::default()
        };

        let count = calculate_image_count(&capabilities, vk::PresentModeKHR::FIFO);
        assert!(count >= 2);
        assert!(count <= 4);
    }

    #[test]
    fn test_choose_extent_fixed() {
        let capabilities = vk::SurfaceCapabilitiesKHR {
            current_extent: vk::Extent2D { width: 1920, height: 1080 },
            ..Default::default()
        };

        let extent = choose_extent(&capabilities, 800, 600);
        assert_eq!(extent.width, 1920);
        assert_eq!(extent.height, 1080);
    }

    #[test]
    fn test_choose_extent_variable() {
        let capabilities = vk::SurfaceCapabilitiesKHR {
            current_extent: vk::Extent2D { width: u32::MAX, height: u32::MAX },
            min_image_extent: vk::Extent2D { width: 640, height: 480 },
            max_image_extent: vk::Extent2D { width: 3840, height: 2160 },
            ..Default::default()
        };

        let extent = choose_extent(&capabilities, 1920, 1080);
        assert_eq!(extent.width, 1920);
        assert_eq!(extent.height, 1080);
    }

    #[test]
    fn test_choose_extent_clamping() {
        let capabilities = vk::SurfaceCapabilitiesKHR {
            current_extent: vk::Extent2D { width: u32::MAX, height: u32::MAX },
            min_image_extent: vk::Extent2D { width: 640, height: 480 },
            max_image_extent: vk::Extent2D { width: 1920, height: 1080 },
            ..Default::default()
        };

        // Test clamping to max
        let extent = choose_extent(&capabilities, 3840, 2160);
        assert_eq!(extent.width, 1920);
        assert_eq!(extent.height, 1080);

        // Test clamping to min
        let extent = choose_extent(&capabilities, 320, 240);
        assert_eq!(extent.width, 640);
        assert_eq!(extent.height, 480);
    }
}
