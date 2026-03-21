//! Vulkan command buffer management
//!
//! Provides command pools and command buffers for recording GPU work.
//! Command buffers are allocated from pools and used to record rendering commands.

use ash::vk;
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use tracing::{debug, info};

// Command errors using define_error! macro per CLAUDE.md
define_error! {
    pub enum CommandError {
        PoolCreationFailed { details: String } =
            ErrorCode::CommandPoolCreationFailed,
            ErrorSeverity::Error,
        AllocationFailed { details: String } =
            ErrorCode::CommandBufferAllocationFailed,
            ErrorSeverity::Error,
        BeginFailed { details: String } =
            ErrorCode::CommandBufferRecordingFailed,
            ErrorSeverity::Error,
        EndFailed { details: String } =
            ErrorCode::CommandBufferRecordingFailed,
            ErrorSeverity::Error,
        ResetFailed { details: String } =
            ErrorCode::CommandBufferRecordingFailed,
            ErrorSeverity::Error,
    }
}

/// Vulkan command pool
///
/// Manages memory for command buffers. Command buffers are allocated from pools
/// and automatically freed when the pool is destroyed or reset.
pub struct CommandPool {
    pool: vk::CommandPool,
    device: ash::Device,
}

impl CommandPool {
    /// Create a command pool for a specific queue family
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    /// * `queue_family_index` - Queue family to create pool for
    /// * `flags` - Pool creation flags (e.g., RESET_COMMAND_BUFFER)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use engine_renderer::CommandPool;
    /// use ash::vk;
    ///
    /// # let device: ash::Device = todo!();
    /// # let queue_family_index = 0;
    /// let pool = CommandPool::new(
    ///     &device,
    ///     queue_family_index,
    ///     vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    /// )?;
    /// # Ok::<(), engine_renderer::CommandError>(())
    /// ```
    pub fn new(
        device: &ash::Device,
        queue_family_index: u32,
        flags: vk::CommandPoolCreateFlags,
    ) -> Result<Self, CommandError> {
        debug!(queue_family = queue_family_index, ?flags, "Creating command pool");

        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(flags);

        let pool = unsafe { device.create_command_pool(&pool_info, None) }.map_err(|e| {
            CommandError::poolcreationfailed(format!("vkCreateCommandPool failed: {}", e))
        })?;

        debug!(pool = ?pool, "Command pool created successfully");

        Ok(Self { pool, device: device.clone() })
    }

    /// Allocate command buffers from this pool
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    /// * `level` - PRIMARY or SECONDARY command buffer
    /// * `count` - Number of buffers to allocate
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::CommandPool;
    /// # use ash::vk;
    /// # let device: ash::Device = todo!();
    /// # let pool: CommandPool = todo!();
    /// let buffers = pool.allocate(
    ///     &device,
    ///     vk::CommandBufferLevel::PRIMARY,
    ///     2, // Allocate 2 buffers
    /// )?;
    /// # Ok::<(), engine_renderer::CommandError>(())
    /// ```
    pub fn allocate(
        &self,
        device: &ash::Device,
        level: vk::CommandBufferLevel,
        count: u32,
    ) -> Result<Vec<vk::CommandBuffer>, CommandError> {
        debug!(count = count, ?level, "Allocating command buffers");

        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(level)
            .command_buffer_count(count);

        let buffers = unsafe { device.allocate_command_buffers(&alloc_info) }.map_err(|e| {
            CommandError::allocationfailed(format!("vkAllocateCommandBuffers failed: {}", e))
        })?;

        info!(count = buffers.len(), "Command buffers allocated successfully");

        Ok(buffers)
    }

    /// Reset the command pool
    ///
    /// This invalidates all command buffers allocated from this pool and returns
    /// their memory to the pool for reuse.
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::CommandPool;
    /// # let device: ash::Device = todo!();
    /// # let pool: CommandPool = todo!();
    /// pool.reset(&device)?;
    /// # Ok::<(), engine_renderer::CommandError>(())
    /// ```
    pub fn reset(&self, device: &ash::Device) -> Result<(), CommandError> {
        debug!("Resetting command pool");

        unsafe { device.reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty()) }
            .map_err(|e| CommandError::resetfailed(format!("vkResetCommandPool failed: {}", e)))?;

        debug!("Command pool reset successfully");

        Ok(())
    }

    /// Get the raw command pool handle
    #[inline]
    pub fn handle(&self) -> vk::CommandPool {
        self.pool
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        debug!("Destroying command pool");
        unsafe {
            self.device.destroy_command_pool(self.pool, None);
        }
    }
}

/// Vulkan command buffer
///
/// Records GPU commands that can be submitted to a queue for execution.
/// Command buffers must be allocated from a CommandPool.
pub struct CommandBuffer {
    buffer: vk::CommandBuffer,
}

impl CommandBuffer {
    /// Create a CommandBuffer wrapper from a raw handle
    ///
    /// # Safety
    ///
    /// The buffer must be a valid command buffer allocated from a pool.
    #[inline]
    pub fn from_handle(buffer: vk::CommandBuffer) -> Self {
        Self { buffer }
    }

    /// Begin recording commands
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    /// * `flags` - Usage flags (e.g., ONE_TIME_SUBMIT, SIMULTANEOUS_USE)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::CommandBuffer;
    /// # use ash::vk;
    /// # let device: ash::Device = todo!();
    /// # let buffer = vk::CommandBuffer::null();
    /// # let cmd = CommandBuffer::from_handle(buffer);
    /// cmd.begin(&device, vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)?;
    /// // Record commands here...
    /// cmd.end(&device)?;
    /// # Ok::<(), engine_renderer::CommandError>(())
    /// ```
    pub fn begin(
        &self,
        device: &ash::Device,
        flags: vk::CommandBufferUsageFlags,
    ) -> Result<(), CommandError> {
        let begin_info = vk::CommandBufferBeginInfo::default().flags(flags);

        unsafe { device.begin_command_buffer(self.buffer, &begin_info) }.map_err(|e| {
            CommandError::beginfailed(format!("vkBeginCommandBuffer failed: {}", e))
        })?;

        Ok(())
    }

    /// End recording commands
    ///
    /// Must be called after begin() and all commands have been recorded.
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::CommandBuffer;
    /// # use ash::vk;
    /// # let device: ash::Device = todo!();
    /// # let buffer = vk::CommandBuffer::null();
    /// # let cmd = CommandBuffer::from_handle(buffer);
    /// cmd.begin(&device, vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)?;
    /// // Record commands...
    /// cmd.end(&device)?;
    /// # Ok::<(), engine_renderer::CommandError>(())
    /// ```
    pub fn end(&self, device: &ash::Device) -> Result<(), CommandError> {
        unsafe { device.end_command_buffer(self.buffer) }
            .map_err(|e| CommandError::endfailed(format!("vkEndCommandBuffer failed: {}", e)))?;

        Ok(())
    }

    /// Begin a render pass
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    /// * `render_pass` - Render pass to begin
    /// * `framebuffer` - Framebuffer to render into
    /// * `extent` - Render area extent
    /// * `clear_color` - Clear color values [r, g, b, a]
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::CommandBuffer;
    /// # use ash::vk;
    /// # let device: ash::Device = todo!();
    /// # let buffer = vk::CommandBuffer::null();
    /// # let cmd = CommandBuffer::from_handle(buffer);
    /// # let render_pass = vk::RenderPass::null();
    /// # let framebuffer = vk::Framebuffer::null();
    /// cmd.begin_render_pass(
    ///     &device,
    ///     render_pass,
    ///     framebuffer,
    ///     vk::Extent2D { width: 1920, height: 1080 },
    ///     [0.0, 0.0, 0.0, 1.0], // Black clear color
    /// );
    /// ```
    pub fn begin_render_pass(
        &self,
        device: &ash::Device,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        extent: vk::Extent2D,
        clear_color: [f32; 4],
    ) {
        let clear_value = vk::ClearValue { color: vk::ClearColorValue { float32: clear_color } };

        let render_area = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent };

        let render_pass_info = vk::RenderPassBeginInfo::default()
            .render_pass(render_pass)
            .framebuffer(framebuffer)
            .render_area(render_area)
            .clear_values(std::slice::from_ref(&clear_value));

        unsafe {
            device.cmd_begin_render_pass(
                self.buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );
        }
    }

    /// End the current render pass
    ///
    /// Must be called after begin_render_pass() and all rendering commands.
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    pub fn end_render_pass(&self, device: &ash::Device) {
        unsafe {
            device.cmd_end_render_pass(self.buffer);
        }
    }

    /// Get the raw command buffer handle
    #[inline]
    pub fn handle(&self) -> vk::CommandBuffer {
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_error_display() {
        let err = CommandError::poolcreationfailed("test error".to_string());
        let msg = err.to_string();
        assert!(msg.contains("PoolCreationFailed") || msg.contains("test error"));
    }

    #[test]
    fn test_allocation_error_display() {
        let err = CommandError::allocationfailed("allocation failed".to_string());
        let msg = err.to_string();
        assert!(msg.contains("AllocationFailed") || msg.contains("allocation failed"));
    }
}
