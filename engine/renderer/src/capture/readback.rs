//! GPU to CPU image readback for frame capture.
//!
//! Handles copying rendered frames from GPU memory to CPU-accessible memory.

use crate::{CommandPool, RendererError};
use ash::vk;
use gpu_allocator::vulkan as gpu_alloc;
use tracing::{debug, info};

/// Frame readback manager - handles GPU→CPU image copy
pub struct FrameReadback {
    readback_buffer: vk::Buffer,
    readback_allocation: Option<gpu_alloc::Allocation>,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
}

impl FrameReadback {
    /// Create readback buffer for frame capture
    ///
    /// Creates a CPU-accessible buffer for copying GPU images.
    pub fn new(
        device: &ash::Device,
        allocator: &std::sync::Arc<std::sync::Mutex<gpu_alloc::Allocator>>,
        width: u32,
        height: u32,
    ) -> Result<Self, RendererError> {
        // Size for RGBA8 image
        let size = (width * height * 4) as u64;

        info!(
            width = width,
            height = height,
            size_mb = size as f64 / (1024.0 * 1024.0),
            "Creating frame readback buffer"
        );

        // Create buffer
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let readback_buffer = unsafe {
            device
                .create_buffer(&buffer_info, None)
                .map_err(|e| RendererError::buffercreationfailed(size, format!("{:?}", e)))?
        };

        // Get memory requirements
        let mem_requirements = unsafe { device.get_buffer_memory_requirements(readback_buffer) };

        // Allocate memory (GpuToCpu for readback)
        let allocation = {
            let mut allocator_lock = allocator.lock().unwrap();
            allocator_lock
                .allocate(&gpu_alloc::AllocationCreateDesc {
                    name: "frame_readback_buffer",
                    requirements: mem_requirements,
                    location: gpu_allocator::MemoryLocation::GpuToCpu,
                    linear: true, // Buffer must be linear
                    allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
                })
                .map_err(|e| {
                    RendererError::memoryallocationfailed(mem_requirements.size, format!("{:?}", e))
                })?
        };

        // Bind memory to buffer
        unsafe {
            device
                .bind_buffer_memory(readback_buffer, allocation.memory(), allocation.offset())
                .map_err(|e| {
                    RendererError::memoryallocationfailed(
                        mem_requirements.size,
                        format!("Failed to bind buffer memory: {:?}", e),
                    )
                })?;
        }

        debug!("Frame readback buffer created successfully");

        Ok(Self { readback_buffer, readback_allocation: Some(allocation), width, height })
    }

    /// Copy swapchain image to readback buffer
    ///
    /// Uses single-time command buffer for immediate execution.
    pub fn copy_image_to_buffer(
        &self,
        device: &ash::Device,
        command_pool: &CommandPool,
        queue: vk::Queue,
        image: vk::Image,
    ) -> Result<(), RendererError> {
        // Create one-time command buffer
        let command_buffer = Self::begin_single_time_commands(device, command_pool)?;

        unsafe {
            // Transition image to TRANSFER_SRC_OPTIMAL
            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_READ);

            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            // Copy image to buffer
            let region = vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D { width: self.width, height: self.height, depth: 1 });

            device.cmd_copy_image_to_buffer(
                command_buffer,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                self.readback_buffer,
                &[region],
            );

            // Transition back to PRESENT_SRC_KHR
            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::TRANSFER_READ)
                .dst_access_mask(vk::AccessFlags::empty());

            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }

        Self::end_single_time_commands(device, command_pool, queue, command_buffer)?;

        Ok(())
    }

    /// Get image data from readback buffer
    ///
    /// Reads mapped memory to get pixel data.
    pub fn get_image_data(&self) -> Result<Vec<u8>, RendererError> {
        let size = (self.width * self.height * 4) as usize;
        let mut data = vec![0u8; size];

        let allocation = self
            .readback_allocation
            .as_ref()
            .ok_or_else(|| RendererError::memorymappingfailed("Allocation is None".to_string()))?;

        // Copy from mapped memory
        if let Some(mapped_ptr) = allocation.mapped_ptr() {
            unsafe {
                std::ptr::copy_nonoverlapping(
                    mapped_ptr.as_ptr() as *const u8,
                    data.as_mut_ptr(),
                    size,
                );
            }
        } else {
            return Err(RendererError::memorymappingfailed("Buffer memory not mapped".to_string()));
        }

        Ok(data)
    }

    /// Begin single-time command buffer
    fn begin_single_time_commands(
        device: &ash::Device,
        command_pool: &CommandPool,
    ) -> Result<vk::CommandBuffer, RendererError> {
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool.handle())
            .command_buffer_count(1);

        let command_buffer = unsafe {
            device
                .allocate_command_buffers(&allocate_info)
                .map_err(|e| RendererError::commandbufferallocationfailed(1, format!("{:?}", e)))?
                [0]
        };

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device.begin_command_buffer(command_buffer, &begin_info).map_err(|e| {
                RendererError::commandbufferallocationfailed(
                    1,
                    format!("Failed to begin command buffer: {:?}", e),
                )
            })?;
        }

        Ok(command_buffer)
    }

    /// End and submit single-time command buffer
    fn end_single_time_commands(
        device: &ash::Device,
        command_pool: &CommandPool,
        queue: vk::Queue,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), RendererError> {
        unsafe {
            device.end_command_buffer(command_buffer).map_err(|e| {
                RendererError::commandbufferallocationfailed(
                    1,
                    format!("Failed to end command buffer: {:?}", e),
                )
            })?;

            let command_buffers = [command_buffer];
            let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);

            device.queue_submit(queue, &[submit_info], vk::Fence::null()).map_err(|e| {
                RendererError::queuesubmissionfailed(format!("Failed to submit queue: {:?}", e))
            })?;

            device.queue_wait_idle(queue).map_err(|e| {
                RendererError::queuesubmissionfailed(format!("Failed to wait for queue: {:?}", e))
            })?;

            device.free_command_buffers(command_pool.handle(), &[command_buffer]);
        }

        Ok(())
    }
}

impl Drop for FrameReadback {
    fn drop(&mut self) {
        // Buffer is destroyed when allocation is dropped
        // Allocation cleanup happens automatically via RAII
        if let Some(allocation) = self.readback_allocation.take() {
            drop(allocation);
        }
    }
}
