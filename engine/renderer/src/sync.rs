//! Vulkan synchronization primitives
//!
//! Provides fences and semaphores for GPU-CPU and GPU-GPU synchronization.
//! Implements the "frames in flight" pattern for efficient rendering.

use ash::vk;
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use tracing::{debug, info};

// Sync errors using define_error! macro per CLAUDE.md
define_error! {
    pub enum SyncError {
        CreationFailed { details: String } =
            ErrorCode::SyncObjectCreationFailed,
            ErrorSeverity::Error,
        WaitFailed { details: String } =
            ErrorCode::SyncObjectCreationFailed,
            ErrorSeverity::Error,
        ResetFailed { details: String } =
            ErrorCode::SyncObjectCreationFailed,
            ErrorSeverity::Error,
    }
}

/// Synchronization objects for one frame in flight
///
/// Each frame needs:
/// - Image available semaphore: Signals when swapchain image is ready
/// - Render finished semaphore: Signals when rendering is complete
/// - In-flight fence: CPU-GPU sync to prevent overwriting frames
///
/// # Synchronization Pattern
///
/// ```text
/// Frame N:
///   1. wait_for_fences([in_flight_fence])
///   2. acquire_next_image(..., image_available_semaphore)
///   3. reset_fences([in_flight_fence])
///   4. record command buffer
///   5. queue_submit(
///        wait: [image_available_semaphore],
///        signal: [render_finished_semaphore],
///        fence: in_flight_fence
///      )
///   6. queue_present(wait: [render_finished_semaphore])
/// ```
pub struct FrameSyncObjects {
    /// Semaphore signaled when swapchain image is available
    pub image_available_semaphore: vk::Semaphore,
    /// Semaphore signaled when rendering to image is finished
    pub render_finished_semaphore: vk::Semaphore,
    /// Fence for CPU-GPU synchronization
    pub in_flight_fence: vk::Fence,
    /// Device handle for cleanup
    device: ash::Device,
}

impl FrameSyncObjects {
    /// Create synchronization objects for one frame
    ///
    /// The fence is created in the signaled state so the first frame
    /// doesn't wait indefinitely.
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    ///
    /// # Example
    ///
    /// ```no_run
    /// use engine_renderer::FrameSyncObjects;
    ///
    /// # let device: ash::Device = todo!();
    /// let sync = FrameSyncObjects::new(&device)?;
    /// # Ok::<(), engine_renderer::SyncError>(())
    /// ```
    pub fn new(device: &ash::Device) -> Result<Self, SyncError> {
        debug!("Creating frame sync objects");

        // Create semaphores (no special flags needed)
        let semaphore_info = vk::SemaphoreCreateInfo::default();

        let image_available_semaphore = unsafe {
            device.create_semaphore(&semaphore_info, None)
        }
        .map_err(|e| SyncError::CreationFailed {
            details: format!("Failed to create image_available semaphore: {}", e),
        })?;

        let render_finished_semaphore = unsafe {
            device.create_semaphore(&semaphore_info, None)
        }
        .map_err(|e| {
            // Clean up first semaphore on error
            unsafe { device.destroy_semaphore(image_available_semaphore, None) };
            SyncError::CreationFailed {
                details: format!("Failed to create render_finished semaphore: {}", e),
            }
        })?;

        // Create fence in signaled state (first frame doesn't wait)
        let fence_info = vk::FenceCreateInfo::default()
            .flags(vk::FenceCreateFlags::SIGNALED);

        let in_flight_fence = unsafe {
            device.create_fence(&fence_info, None)
        }
        .map_err(|e| {
            // Clean up semaphores on error
            unsafe {
                device.destroy_semaphore(image_available_semaphore, None);
                device.destroy_semaphore(render_finished_semaphore, None);
            }
            SyncError::CreationFailed {
                details: format!("Failed to create fence: {}", e),
            }
        })?;

        debug!(
            image_available_semaphore = ?image_available_semaphore,
            render_finished_semaphore = ?render_finished_semaphore,
            in_flight_fence = ?in_flight_fence,
            "Frame sync objects created successfully"
        );

        Ok(Self {
            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence,
            device: device.clone(),
        })
    }

    /// Wait for the in-flight fence
    ///
    /// Blocks until the GPU has finished with this frame.
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    /// * `timeout_ns` - Timeout in nanoseconds (use u64::MAX for infinite)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::FrameSyncObjects;
    /// # let device: ash::Device = todo!();
    /// # let sync = FrameSyncObjects::new(&device)?;
    /// // Wait indefinitely
    /// sync.wait(&device, u64::MAX)?;
    /// # Ok::<(), engine_renderer::SyncError>(())
    /// ```
    pub fn wait(&self, device: &ash::Device, timeout_ns: u64) -> Result<(), SyncError> {
        unsafe {
            device.wait_for_fences(&[self.in_flight_fence], true, timeout_ns)
        }
        .map_err(|e| SyncError::WaitFailed {
            details: format!("vkWaitForFences failed: {}", e),
        })
    }

    /// Reset the in-flight fence
    ///
    /// Call this after waiting and before submitting work.
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::FrameSyncObjects;
    /// # let device: ash::Device = todo!();
    /// # let sync = FrameSyncObjects::new(&device)?;
    /// sync.wait(&device, u64::MAX)?;
    /// sync.reset(&device)?;
    /// # Ok::<(), engine_renderer::SyncError>(())
    /// ```
    pub fn reset(&self, device: &ash::Device) -> Result<(), SyncError> {
        unsafe {
            device.reset_fences(&[self.in_flight_fence])
        }
        .map_err(|e| SyncError::ResetFailed {
            details: format!("vkResetFences failed: {}", e),
        })
    }

    /// Get the image available semaphore handle
    #[inline]
    pub fn image_available(&self) -> vk::Semaphore {
        self.image_available_semaphore
    }

    /// Get the render finished semaphore handle
    #[inline]
    pub fn render_finished(&self) -> vk::Semaphore {
        self.render_finished_semaphore
    }

    /// Get the in-flight fence handle
    #[inline]
    pub fn fence(&self) -> vk::Fence {
        self.in_flight_fence
    }
}

impl Drop for FrameSyncObjects {
    fn drop(&mut self) {
        debug!("Destroying frame sync objects");
        unsafe {
            self.device.destroy_semaphore(self.image_available_semaphore, None);
            self.device.destroy_semaphore(self.render_finished_semaphore, None);
            self.device.destroy_fence(self.in_flight_fence, None);
        }
    }
}

/// Create synchronization objects for multiple frames in flight
///
/// The "frames in flight" pattern allows the CPU to work on the next frame
/// while the GPU is still rendering the current frame. Typical values are 2-3
/// frames (2 recommended for lower latency).
///
/// # Arguments
///
/// * `device` - Logical Vulkan device
/// * `frames_in_flight` - Number of frames to allow in flight (typically 2-3)
///
/// # Example
///
/// ```no_run
/// use engine_renderer::create_sync_objects;
///
/// # let device: ash::Device = todo!();
/// // Create sync objects for 2 frames in flight
/// let sync_objects = create_sync_objects(&device, 2)?;
/// # Ok::<(), engine_renderer::SyncError>(())
/// ```
pub fn create_sync_objects(
    device: &ash::Device,
    frames_in_flight: u32,
) -> Result<Vec<FrameSyncObjects>, SyncError> {
    info!(
        frames_in_flight = frames_in_flight,
        "Creating sync objects for frames in flight"
    );

    (0..frames_in_flight)
        .map(|_| FrameSyncObjects::new(device))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_error_display() {
        let err = SyncError::CreationFailed {
            details: "test error".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("CreationFailed") || msg.contains("test error"));
    }
}
