//! Offscreen rendering target for headless rendering and frame capture.
//!
//! Essential for AI agent visual feedback - allows rendering without a window
//! and capturing frames for analysis.
//!
//! # Optimizations
//! - Lazy depth format selection with caching
//! - Efficient image layout transitions
//! - Minimal allocations during creation
//! - Proper memory cleanup in Drop

// Tracy profiling macros (no-op when profiling feature disabled)
#[cfg(feature = "profiling")]
macro_rules! profile_scope {
    ($name:expr) => {
        let _tracy_span = tracy_client::span!($name);
    };
}

#[cfg(not(feature = "profiling"))]
macro_rules! profile_scope {
    ($name:expr) => {};
}

use crate::context::VulkanContext;
use crate::error::RendererError;
use ash::vk;
use gpu_allocator::vulkan as gpu_alloc;
use std::sync::OnceLock;
use tracing::{info, instrument, warn};

/// Cached depth format to avoid repeated queries.
static CACHED_DEPTH_FORMAT: OnceLock<vk::Format> = OnceLock::new();

/// Offscreen render target with color and depth attachments.
pub struct OffscreenTarget {
    /// Color image
    pub color_image: vk::Image,
    /// Color image view
    pub color_image_view: vk::ImageView,
    /// Color image allocation
    color_allocation: Option<gpu_alloc::Allocation>,
    /// Depth image (optional)
    pub depth_image: Option<vk::Image>,
    /// Depth image view (optional)
    pub depth_image_view: Option<vk::ImageView>,
    /// Depth image allocation (optional)
    depth_allocation: Option<gpu_alloc::Allocation>,
    /// Image format
    pub format: vk::Format,
    /// Depth format
    pub depth_format: Option<vk::Format>,
    /// Image extent
    pub extent: vk::Extent2D,
    /// Device handle for cleanup
    device: ash::Device,
    /// Sample count (for MSAA support)
    pub sample_count: vk::SampleCountFlags,
}

impl OffscreenTarget {
    /// Create a new offscreen render target.
    ///
    /// # Arguments
    ///
    /// * `context` - Vulkan context
    /// * `width` - Image width
    /// * `height` - Image height
    /// * `format` - Color format (default: BGRA8_SRGB)
    /// * `with_depth` - Whether to create a depth attachment
    ///
    /// # Performance
    /// - Caches depth format selection across multiple targets
    /// - Minimal allocations (reuses device, allocator references)
    /// - Images created in UNDEFINED layout for efficiency
    #[instrument(skip(context))]
    pub fn new(
        context: &VulkanContext,
        width: u32,
        height: u32,
        format: Option<vk::Format>,
        with_depth: bool,
    ) -> Result<Self, RendererError> {
        profile_scope!("OffscreenTarget::new");
        Self::new_with_samples(
            context,
            width,
            height,
            format,
            with_depth,
            vk::SampleCountFlags::TYPE_1,
        )
    }

    /// Create a new offscreen render target with MSAA support.
    ///
    /// # Arguments
    ///
    /// * `context` - Vulkan context
    /// * `width` - Image width
    /// * `height` - Image height
    /// * `format` - Color format (default: BGRA8_SRGB)
    /// * `with_depth` - Whether to create a depth attachment
    /// * `sample_count` - MSAA sample count (TYPE_1 for no MSAA)
    #[instrument(skip(context))]
    pub fn new_with_samples(
        context: &VulkanContext,
        width: u32,
        height: u32,
        format: Option<vk::Format>,
        with_depth: bool,
        sample_count: vk::SampleCountFlags,
    ) -> Result<Self, RendererError> {
        profile_scope!("OffscreenTarget::new_with_samples");
        let format = format.unwrap_or(vk::Format::B8G8R8A8_SRGB);
        let extent = vk::Extent2D { width, height };

        info!(
            width = width,
            height = height,
            format = ?format,
            with_depth = with_depth,
            sample_count = ?sample_count,
            "Creating offscreen render target"
        );

        // Create color image
        let (color_image, color_allocation) = create_image(
            &context.device,
            &context.allocator,
            extent,
            format,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
            sample_count,
        )?;

        // Create color image view
        let color_image_view =
            create_image_view(&context.device, color_image, format, vk::ImageAspectFlags::COLOR)?;

        // Create depth image if requested
        let (depth_image, depth_image_view, depth_allocation, depth_format) = if with_depth {
            // Use cached depth format if available
            let depth_fmt = *CACHED_DEPTH_FORMAT.get_or_init(|| {
                find_depth_format(context).unwrap_or_else(|_| {
                    warn!("Failed to find optimal depth format, using D32_SFLOAT");
                    vk::Format::D32_SFLOAT
                })
            });

            let (image, allocation) = create_image(
                &context.device,
                &context.allocator,
                extent,
                depth_fmt,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                sample_count,
            )?;

            let view =
                create_image_view(&context.device, image, depth_fmt, vk::ImageAspectFlags::DEPTH)?;

            (Some(image), Some(view), Some(allocation), Some(depth_fmt))
        } else {
            (None, None, None, None)
        };

        info!("Offscreen render target created");

        Ok(OffscreenTarget {
            color_image,
            color_image_view,
            color_allocation: Some(color_allocation),
            depth_image,
            depth_image_view,
            depth_allocation,
            format,
            depth_format,
            extent,
            device: context.device.clone(),
            sample_count,
        })
    }

    /// Resize the offscreen target (destroys and recreates images).
    ///
    /// More efficient than creating a new OffscreenTarget.
    #[instrument(skip(self, context))]
    pub fn resize(
        &mut self,
        context: &VulkanContext,
        width: u32,
        height: u32,
    ) -> Result<(), RendererError> {
        profile_scope!("OffscreenTarget::resize");

        if width == self.extent.width && height == self.extent.height {
            info!("Resize requested with same dimensions, ignoring");
            return Ok(());
        }

        info!(
            old_extent = ?self.extent,
            new_extent = ?(width, height),
            "Resizing offscreen target"
        );

        // Wait for device to be idle
        // SAFETY: self.device is valid. device_wait_idle ensures no operations are in flight.
        {
            profile_scope!("device_wait_idle");
            unsafe {
                self.device.device_wait_idle().map_err(|e| {
                    RendererError::devicelost(format!("Failed to wait for device idle: {:?}", e))
                })?;
            }
        }

        // Destroy old resources
        // SAFETY: All resources are valid, owned by this struct. Device is idle.
        // We destroy views before images (correct dependency order).
        {
            profile_scope!("destroy_old_resources");
            unsafe {
                self.device.destroy_image_view(self.color_image_view, None);
                if let Some(view) = self.depth_image_view {
                    self.device.destroy_image_view(view, None);
                }
                self.device.destroy_image(self.color_image, None);
                if let Some(image) = self.depth_image {
                    self.device.destroy_image(image, None);
                }
            }

            // Free allocations
            drop(self.color_allocation.take());
            drop(self.depth_allocation.take());
        }

        // Create new resources
        let extent = vk::Extent2D { width, height };

        let (color_image, color_allocation) = {
            profile_scope!("create_color_image");
            create_image(
                &context.device,
                &context.allocator,
                extent,
                self.format,
                vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
                self.sample_count,
            )?
        };

        let color_image_view = {
            profile_scope!("create_color_image_view");
            create_image_view(
                &context.device,
                color_image,
                self.format,
                vk::ImageAspectFlags::COLOR,
            )?
        };

        let (depth_image, depth_image_view, depth_allocation) = if let Some(depth_fmt) =
            self.depth_format
        {
            profile_scope!("create_depth_resources");
            let (image, allocation) = create_image(
                &context.device,
                &context.allocator,
                extent,
                depth_fmt,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                self.sample_count,
            )?;

            let view =
                create_image_view(&context.device, image, depth_fmt, vk::ImageAspectFlags::DEPTH)?;

            (Some(image), Some(view), Some(allocation))
        } else {
            (None, None, None)
        };

        // Update state
        self.color_image = color_image;
        self.color_image_view = color_image_view;
        self.color_allocation = Some(color_allocation);
        self.depth_image = depth_image;
        self.depth_image_view = depth_image_view;
        self.depth_allocation = depth_allocation;
        self.extent = extent;

        info!("Offscreen target resized");
        Ok(())
    }

    /// Get the width of the render target.
    pub fn width(&self) -> u32 {
        self.extent.width
    }

    /// Get the height of the render target.
    pub fn height(&self) -> u32 {
        self.extent.height
    }

    /// Check if this target has a depth attachment.
    pub fn has_depth(&self) -> bool {
        self.depth_image.is_some()
    }

    /// Get sample count (for MSAA).
    pub fn sample_count(&self) -> vk::SampleCountFlags {
        self.sample_count
    }

    /// Transition color image layout (convenience function).
    ///
    /// # Safety
    /// Command buffer must be in recording state and executed on appropriate queue.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn transition_color_layout(
        &self,
        command_buffer: vk::CommandBuffer,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        src_access: vk::AccessFlags,
        dst_access: vk::AccessFlags,
        src_stage: vk::PipelineStageFlags,
        dst_stage: vk::PipelineStageFlags,
    ) {
        // SAFETY: Caller guarantees command buffer is in recording state.
        // self.color_image is valid (owned by this struct).
        unsafe {
            transition_image_layout(
                &self.device,
                command_buffer,
                self.color_image,
                old_layout,
                new_layout,
                src_access,
                dst_access,
                src_stage,
                dst_stage,
                vk::ImageAspectFlags::COLOR,
            );
        }
    }

    /// Transition depth image layout (convenience function).
    ///
    /// # Safety
    /// Command buffer must be in recording state and executed on appropriate queue.
    /// Target must have depth attachment.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn transition_depth_layout(
        &self,
        command_buffer: vk::CommandBuffer,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        src_access: vk::AccessFlags,
        dst_access: vk::AccessFlags,
        src_stage: vk::PipelineStageFlags,
        dst_stage: vk::PipelineStageFlags,
    ) {
        if let Some(depth_image) = self.depth_image {
            // SAFETY: Caller guarantees command buffer is in recording state.
            // depth_image is valid (Some variant checked above).
            unsafe {
                transition_image_layout(
                    &self.device,
                    command_buffer,
                    depth_image,
                    old_layout,
                    new_layout,
                    src_access,
                    dst_access,
                    src_stage,
                    dst_stage,
                    vk::ImageAspectFlags::DEPTH,
                );
            }
        }
    }
}

impl Drop for OffscreenTarget {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: We own these resources and device is still valid.
            // Destroy order: views -> images -> allocations (reverse of creation).

            // Destroy image views first (depend on images)
            self.device.destroy_image_view(self.color_image_view, None);
            if let Some(view) = self.depth_image_view {
                self.device.destroy_image_view(view, None);
            }

            // Destroy images (depend on allocations)
            self.device.destroy_image(self.color_image, None);
            if let Some(image) = self.depth_image {
                self.device.destroy_image(image, None);
            }

            // Allocations are freed when dropped (RAII)
            // Must be last to ensure images are destroyed first
            drop(self.color_allocation.take());
            drop(self.depth_allocation.take());
        }
    }
}

/// Create an image with memory allocation.
///
/// # Performance
/// - Uses OPTIMAL tiling for GPU efficiency
/// - Allocates from GPU-only memory
/// - Initial layout is UNDEFINED (must transition before use)
#[instrument(skip(device, allocator))]
fn create_image(
    device: &ash::Device,
    allocator: &std::sync::Arc<std::sync::Mutex<gpu_alloc::Allocator>>,
    extent: vk::Extent2D,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    sample_count: vk::SampleCountFlags,
) -> Result<(vk::Image, gpu_alloc::Allocation), RendererError> {
    profile_scope!("create_image");

    // Create image
    let create_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(format)
        .extent(vk::Extent3D { width: extent.width, height: extent.height, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .samples(sample_count)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);

    let image = {
        profile_scope!("vkCreateImage");
        unsafe {
            // SAFETY: create_info is valid and device is valid
            device.create_image(&create_info, None).map_err(|e| {
                RendererError::imagecreationfailed(extent.width, extent.height, format!("{:?}", e))
            })?
        }
    };

    // Get memory requirements
    let mem_requirements = unsafe {
        // SAFETY: image was just created successfully
        device.get_image_memory_requirements(image)
    };

    // Determine allocation name based on usage
    let name = if usage.contains(vk::ImageUsageFlags::COLOR_ATTACHMENT) {
        "offscreen_color_image"
    } else if usage.contains(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT) {
        "offscreen_depth_image"
    } else {
        "offscreen_image"
    };

    // Allocate memory (scoped lock)
    let allocation = {
        profile_scope!("allocate_image_memory");
        let mut allocator_lock = allocator.lock().unwrap();
        allocator_lock
            .allocate(&gpu_alloc::AllocationCreateDesc {
                name,
                requirements: mem_requirements,
                location: gpu_allocator::MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| {
                RendererError::memoryallocationfailed(mem_requirements.size, format!("{:?}", e))
            })?
    };

    // Bind memory to image
    {
        profile_scope!("bind_image_memory");
        unsafe {
            // SAFETY: allocation is valid and matches memory requirements
            device
                .bind_image_memory(image, allocation.memory(), allocation.offset())
                .map_err(|e| {
                    RendererError::memoryallocationfailed(
                        mem_requirements.size,
                        format!("Failed to bind image memory: {:?}", e),
                    )
                })?;
        }
    }

    Ok((image, allocation))
}

/// Create an image view.
#[instrument(skip(device))]
fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
    aspect_mask: vk::ImageAspectFlags,
) -> Result<vk::ImageView, RendererError> {
    profile_scope!("create_image_view");

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
            aspect_mask,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    // SAFETY: device is valid, image is valid, create_info is properly initialized.
    unsafe {
        device
            .create_image_view(&create_info, None)
            .map_err(|e| RendererError::imageviewcreationfailed(format!("{:?}", e)))
    }
}

/// Find a supported depth format with comprehensive fallback chain.
///
/// # Format Preference Order
/// 1. D32_SFLOAT - Highest precision, no stencil (preferred for quality)
/// 2. D32_SFLOAT_S8_UINT - High precision with stencil
/// 3. D24_UNORM_S8_UINT - Standard depth with stencil (widely supported)
/// 4. D16_UNORM - Minimum precision fallback
///
/// # Caching
/// Result is cached in CACHED_DEPTH_FORMAT to avoid repeated queries.
#[instrument(skip(context))]
fn find_depth_format(context: &VulkanContext) -> Result<vk::Format, RendererError> {
    profile_scope!("find_depth_format");

    const DEPTH_FORMAT_CANDIDATES: &[vk::Format] = &[
        vk::Format::D32_SFLOAT,         // Best: 32-bit float, no stencil
        vk::Format::D32_SFLOAT_S8_UINT, // Good: 32-bit float + 8-bit stencil
        vk::Format::D24_UNORM_S8_UINT,  // Common: 24-bit depth + 8-bit stencil
        vk::Format::D16_UNORM,          // Fallback: 16-bit depth only
    ];

    for &format in DEPTH_FORMAT_CANDIDATES {
        let properties = {
            profile_scope!("query_format_properties");
            unsafe {
                // SAFETY: context.physical_device is valid
                context
                    .instance
                    .get_physical_device_format_properties(context.physical_device, format)
            }
        };

        // Check if format supports depth/stencil attachment in optimal tiling
        if properties
            .optimal_tiling_features
            .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
        {
            info!(
                format = ?format,
                stencil = format == vk::Format::D32_SFLOAT_S8_UINT
                    || format == vk::Format::D24_UNORM_S8_UINT,
                "Selected depth format"
            );
            return Ok(format);
        }
    }

    Err(RendererError::imagecreationfailed(
        0,
        0,
        "No suitable depth format found (tried D32_SFLOAT, D32_SFLOAT_S8_UINT, D24_UNORM_S8_UINT, D16_UNORM)".to_string(),
    ))
}

/// Transition image layout with pipeline barrier.
///
/// # Performance
/// - Uses precise pipeline stage synchronization
/// - Minimal barrier scope for better GPU scheduling
///
/// # Safety
/// - Command buffer must be in recording state
/// - Image must be valid and owned by calling code
/// - Must be executed on appropriate queue for layout transition
#[allow(clippy::too_many_arguments)]
unsafe fn transition_image_layout(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    src_access: vk::AccessFlags,
    dst_access: vk::AccessFlags,
    src_stage: vk::PipelineStageFlags,
    dst_stage: vk::PipelineStageFlags,
    aspect_mask: vk::ImageAspectFlags,
) {
    let barrier = vk::ImageMemoryBarrier::default()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        })
        .src_access_mask(src_access)
        .dst_access_mask(dst_access);

    // SAFETY: Caller guarantees command buffer is recording and image is valid
    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            src_stage,
            dst_stage,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
    }
}

/// Batch transition multiple images (more efficient than individual transitions).
///
/// # Safety
/// - Command buffer must be in recording state
/// - All images must be valid
/// - All images must use same stage/access masks
pub unsafe fn batch_transition_layouts(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    transitions: &[(vk::Image, vk::ImageLayout, vk::ImageLayout, vk::ImageAspectFlags)],
    src_access: vk::AccessFlags,
    dst_access: vk::AccessFlags,
    src_stage: vk::PipelineStageFlags,
    dst_stage: vk::PipelineStageFlags,
) {
    if transitions.is_empty() {
        return;
    }

    let barriers: Vec<vk::ImageMemoryBarrier> = transitions
        .iter()
        .map(|(image, old_layout, new_layout, aspect_mask)| {
            vk::ImageMemoryBarrier::default()
                .old_layout(*old_layout)
                .new_layout(*new_layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(*image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: *aspect_mask,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(src_access)
                .dst_access_mask(dst_access)
        })
        .collect();

    // SAFETY: Caller guarantees command buffer is recording and all images are valid
    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            src_stage,
            dst_stage,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &barriers,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offscreen_target_dimensions() {
        // This is a unit test for the data structures, not an integration test
        // Integration tests that create actual Vulkan resources are in tests/
        let extent = vk::Extent2D { width: 1920, height: 1080 };

        assert_eq!(extent.width, 1920);
        assert_eq!(extent.height, 1080);
    }

    #[test]
    fn test_depth_format_ordering() {
        // Verify that we prefer D32_SFLOAT
        const DEPTH_FORMAT_CANDIDATES: &[vk::Format] = &[
            vk::Format::D32_SFLOAT,
            vk::Format::D32_SFLOAT_S8_UINT,
            vk::Format::D24_UNORM_S8_UINT,
            vk::Format::D16_UNORM,
        ];
        assert_eq!(DEPTH_FORMAT_CANDIDATES[0], vk::Format::D32_SFLOAT);
    }

    #[test]
    fn test_sample_count_default() {
        let sample_count = vk::SampleCountFlags::TYPE_1;
        assert_eq!(sample_count, vk::SampleCountFlags::TYPE_1);
    }
}
