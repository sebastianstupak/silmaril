//! Depth buffer for 3D rendering
//!
//! Provides depth attachment creation and management for rendering pipelines.
//! Depth buffers enable proper occlusion handling where closer objects obscure farther objects.
//!
//! # Format
//!
//! Uses `VK_FORMAT_D32_SFLOAT` for high precision depth values.
//!
//! # Example
//!
//! ```no_run
//! use engine_renderer::DepthBuffer;
//! use ash::vk;
//!
//! # let device: ash::Device = todo!();
//! # let allocator: std::sync::Arc<std::sync::Mutex<gpu_allocator::vulkan::Allocator>> = todo!();
//! let extent = vk::Extent2D { width: 1920, height: 1080 };
//!
//! let depth_buffer = DepthBuffer::new(&device, &allocator, extent)?;
//!
//! // Use depth_buffer.image_view() when creating framebuffers
//! # Ok::<(), engine_renderer::RendererError>(())
//! ```

use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, instrument};

// Re-use RendererError for consistency
use crate::error::RendererError;

/// Depth buffer for 3D rendering
///
/// Manages a depth image, its memory allocation, and image view.
/// The depth buffer is automatically cleaned up when dropped.
pub struct DepthBuffer {
    image: vk::Image,
    image_view: vk::ImageView,
    /// GPU memory allocation; freed explicitly via `allocator.free()` in Drop.
    allocation: Option<Allocation>,
    format: vk::Format,
    extent: vk::Extent2D,
    device: ash::Device,
    /// Needed to return the allocation to the allocator in Drop.
    allocator: Arc<Mutex<Allocator>>,
}

impl DepthBuffer {
    /// Create a new depth buffer
    ///
    /// Creates a depth image with `VK_FORMAT_D32_SFLOAT` format, allocates GPU memory,
    /// and creates an image view for use in framebuffers.
    ///
    /// # Arguments
    ///
    /// * `device` - Vulkan logical device
    /// * `allocator` - GPU memory allocator
    /// * `extent` - Depth buffer dimensions (must match framebuffer size)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use engine_renderer::DepthBuffer;
    /// use ash::vk;
    ///
    /// # let device: ash::Device = todo!();
    /// # let allocator: std::sync::Arc<std::sync::Mutex<gpu_allocator::vulkan::Allocator>> = todo!();
    /// let depth_buffer = DepthBuffer::new(
    ///     &device,
    ///     &allocator,
    ///     vk::Extent2D { width: 1920, height: 1080 }
    /// )?;
    /// # Ok::<(), engine_renderer::RendererError>(())
    /// ```
    #[instrument(skip(device, allocator))]
    pub fn new(
        device: &ash::Device,
        allocator: &Arc<Mutex<Allocator>>,
        extent: vk::Extent2D,
    ) -> Result<Self, RendererError> {
        info!(width = extent.width, height = extent.height, "Creating depth buffer");

        let format = vk::Format::D32_SFLOAT;

        // Create depth image
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D { width: extent.width, height: extent.height, depth: 1 })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = unsafe { device.create_image(&image_info, None) }.map_err(|e| {
            RendererError::imagecreationfailed(
                extent.width,
                extent.height,
                format!("Failed to create depth image: {:?}", e),
            )
        })?;

        // Allocate memory for depth image
        let requirements = unsafe { device.get_image_memory_requirements(image) };

        let allocation = {
            let mut allocator_lock = allocator.lock().unwrap();

            allocator_lock
                .allocate(&AllocationCreateDesc {
                    name: "depth_buffer",
                    requirements,
                    location: MemoryLocation::GpuOnly,
                    linear: false,
                    allocation_scheme: AllocationScheme::GpuAllocatorManaged,
                })
                .map_err(|e| {
                    unsafe { device.destroy_image(image, None) };
                    RendererError::memoryallocationfailed(
                        requirements.size,
                        format!("Failed to allocate depth image memory: {:?}", e),
                    )
                })?
        };

        // Bind memory to image
        let bind_result =
            unsafe { device.bind_image_memory(image, allocation.memory(), allocation.offset()) };

        if let Err(e) = bind_result {
            // Clean up on error
            let mut allocator_lock = allocator.lock().unwrap();
            let _ = allocator_lock.free(allocation);
            unsafe { device.destroy_image(image, None) };
            return Err(RendererError::memoryallocationfailed(
                requirements.size,
                format!("Failed to bind depth image memory: {:?}", e),
            ));
        }

        debug!(image = ?image, "Depth image created and memory bound");

        // Create image view
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let image_view_result = unsafe { device.create_image_view(&view_info, None) };

        let image_view = match image_view_result {
            Ok(view) => view,
            Err(e) => {
                // Clean up on error
                let mut allocator_lock = allocator.lock().unwrap();
                let _ = allocator_lock.free(allocation);
                unsafe { device.destroy_image(image, None) };
                return Err(RendererError::imageviewcreationfailed(format!(
                    "Failed to create depth image view: {:?}",
                    e
                )));
            }
        };

        debug!(image_view = ?image_view, "Depth image view created successfully");

        info!(
            width = extent.width,
            height = extent.height,
            format = ?format,
            "Depth buffer created successfully"
        );

        Ok(Self {
            image,
            image_view,
            allocation: Some(allocation),
            format,
            extent,
            device: device.clone(),
            allocator: allocator.clone(),
        })
    }

    /// Get the depth image handle
    #[inline]
    pub fn image(&self) -> vk::Image {
        self.image
    }

    /// Get the depth image view handle
    #[inline]
    pub fn image_view(&self) -> vk::ImageView {
        self.image_view
    }

    /// Get the depth format (always D32_SFLOAT)
    #[inline]
    pub fn format(&self) -> vk::Format {
        self.format
    }

    /// Get the depth buffer extent
    #[inline]
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }
}

impl Drop for DepthBuffer {
    fn drop(&mut self) {
        debug!("Destroying depth buffer");
        unsafe {
            self.device.destroy_image_view(self.image_view, None);
            self.device.destroy_image(self.image, None);
        }
        // Return the allocation to the allocator so it can reclaim VkDeviceMemory.
        // This must happen before the Allocator itself is dropped (see VulkanContext::Drop).
        if let Some(allocation) = self.allocation.take() {
            if let Ok(mut alloc) = self.allocator.lock() {
                let _ = alloc.free(allocation);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depth_format_constant() {
        // Verify we're using the correct depth format
        assert_eq!(vk::Format::D32_SFLOAT.as_raw(), 126);
    }

    #[test]
    fn test_depth_buffer_extent_sizes() {
        // Test that various extent sizes are representable
        let extents = [
            (1, 1),
            (640, 480),
            (1280, 720),
            (1920, 1080),
            (2560, 1440),
            (3840, 2160),
            (7680, 4320), // 8K
        ];

        for (width, height) in &extents {
            let extent = vk::Extent2D { width: *width, height: *height };
            assert_eq!(extent.width, *width);
            assert_eq!(extent.height, *height);
        }
    }

    #[test]
    fn test_image_aspect_depth() {
        // Verify depth aspect flag value
        assert_eq!(vk::ImageAspectFlags::DEPTH.as_raw(), 0x2);
    }
}
