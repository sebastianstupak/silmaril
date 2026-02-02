//! Vulkan synchronization primitives
//!
//! Provides fences and semaphores for GPU-CPU and GPU-GPU synchronization.
//! Implements the "frames in flight" pattern for efficient rendering.
//!
//! # Frames in Flight Pattern
//!
//! We use 2 frames in flight to maximize GPU utilization:
//! - Frame 0: CPU preparing commands while GPU executes frame 1
//! - Frame 1: CPU preparing commands while GPU executes frame 0
//!
//! This ensures the GPU is never idle waiting for CPU.

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

/// Resources for a single frame in flight
///
/// Contains the synchronization primitives needed for one frame.
pub struct FrameResources {
    /// Semaphore signaled when swapchain image is available
    pub image_available: vk::Semaphore,
    /// Semaphore signaled when rendering is finished
    pub render_finished: vk::Semaphore,
    /// Fence for CPU-GPU synchronization
    pub in_flight_fence: vk::Fence,
}

/// Manages synchronization for multiple frames in flight
///
/// This struct encapsulates the "frames in flight" pattern, managing
/// all synchronization primitives and frame cycling.
///
/// # Example
///
/// ```no_run
/// use engine_renderer::FrameSync;
///
/// # let device: ash::Device = todo!();
/// // Create sync for 2 frames in flight
/// let mut sync = FrameSync::create(&device, 2)?;
///
/// // In render loop:
/// loop {
///     sync.wait_for_frame(&device)?;
///     // acquire_next_image(..., sync.current_frame_resources().image_available)
///     sync.reset_fence(&device)?;
///     // submit_commands(..., sync.current_frame_resources().in_flight_fence)
///     // present(..., sync.current_frame_resources().render_finished)
///     sync.advance_frame();
/// }
/// # Ok::<(), engine_renderer::SyncError>(())
/// ```
pub struct FrameSync {
    /// Semaphores signaled when swapchain image is available
    pub image_available_semaphores: Vec<vk::Semaphore>,

    /// Semaphores signaled when rendering is finished
    pub render_finished_semaphores: Vec<vk::Semaphore>,

    /// Fences to wait for frame completion
    pub in_flight_fences: Vec<vk::Fence>,

    /// Current frame index (0 or 1 for 2 frames in flight)
    pub current_frame: usize,

    /// Maximum frames that can be processed simultaneously
    pub frames_in_flight: usize,
}

impl FrameSync {
    /// Create synchronization primitives for N frames in flight (typically 2)
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    /// * `frames_in_flight` - Number of frames to allow in flight (typically 2)
    ///
    /// # Important
    ///
    /// Initial fences MUST be signaled, otherwise first wait_for_frame() will hang forever!
    ///
    /// # Example
    ///
    /// ```no_run
    /// use engine_renderer::FrameSync;
    ///
    /// # let device: ash::Device = todo!();
    /// let sync = FrameSync::create(&device, 2)?;
    /// # Ok::<(), engine_renderer::SyncError>(())
    /// ```
    pub fn create(device: &ash::Device, frames_in_flight: usize) -> Result<Self, SyncError> {
        info!(frames_in_flight = frames_in_flight, "Creating frame sync objects");

        let mut image_available_semaphores = Vec::with_capacity(frames_in_flight);
        let mut render_finished_semaphores = Vec::with_capacity(frames_in_flight);
        let mut in_flight_fences = Vec::with_capacity(frames_in_flight);

        // Create synchronization objects for each frame
        for frame_index in 0..frames_in_flight {
            // Create semaphores (no special flags needed)
            let semaphore_info = vk::SemaphoreCreateInfo::default();

            let image_available = unsafe { device.create_semaphore(&semaphore_info, None) }
                .map_err(|e| {
                    // Clean up any already created objects
                    Self::cleanup_partial(
                        device,
                        &image_available_semaphores,
                        &render_finished_semaphores,
                        &in_flight_fences,
                    );
                    SyncError::creationfailed(format!(
                        "Failed to create image_available semaphore for frame {}: {}",
                        frame_index, e
                    ))
                })?;

            let render_finished = unsafe { device.create_semaphore(&semaphore_info, None) }
                .map_err(|e| {
                    // Clean up newly created semaphore and any previous objects
                    unsafe { device.destroy_semaphore(image_available, None) };
                    Self::cleanup_partial(
                        device,
                        &image_available_semaphores,
                        &render_finished_semaphores,
                        &in_flight_fences,
                    );
                    SyncError::creationfailed(format!(
                        "Failed to create render_finished semaphore for frame {}: {}",
                        frame_index, e
                    ))
                })?;

            // CRITICAL: Create fence in SIGNALED state so first frame doesn't wait
            let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

            let fence = unsafe { device.create_fence(&fence_info, None) }.map_err(|e| {
                // Clean up newly created objects and any previous objects
                unsafe {
                    device.destroy_semaphore(image_available, None);
                    device.destroy_semaphore(render_finished, None);
                }
                Self::cleanup_partial(
                    device,
                    &image_available_semaphores,
                    &render_finished_semaphores,
                    &in_flight_fences,
                );
                SyncError::creationfailed(format!(
                    "Failed to create fence for frame {}: {}",
                    frame_index, e
                ))
            })?;

            image_available_semaphores.push(image_available);
            render_finished_semaphores.push(render_finished);
            in_flight_fences.push(fence);

            debug!(
                frame_index = frame_index,
                image_available = ?image_available,
                render_finished = ?render_finished,
                fence = ?fence,
                "Frame sync objects created"
            );
        }

        info!(frames_in_flight = frames_in_flight, "Frame sync creation complete");

        Ok(Self {
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            current_frame: 0,
            frames_in_flight,
        })
    }

    /// Helper function to clean up partially created sync objects
    fn cleanup_partial(
        device: &ash::Device,
        image_available: &[vk::Semaphore],
        render_finished: &[vk::Semaphore],
        fences: &[vk::Fence],
    ) {
        unsafe {
            for &semaphore in image_available {
                device.destroy_semaphore(semaphore, None);
            }
            for &semaphore in render_finished {
                device.destroy_semaphore(semaphore, None);
            }
            for &fence in fences {
                device.destroy_fence(fence, None);
            }
        }
    }

    /// Wait for the current frame's fence (CPU waits for GPU)
    ///
    /// Blocks until the GPU has finished rendering the frame that was
    /// `frames_in_flight` frames ago.
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::FrameSync;
    /// # let device: ash::Device = todo!();
    /// # let sync = FrameSync::create(&device, 2)?;
    /// sync.wait_for_frame(&device)?;
    /// # Ok::<(), engine_renderer::SyncError>(())
    /// ```
    pub fn wait_for_frame(&self, device: &ash::Device) -> Result<(), SyncError> {
        let fence = self.in_flight_fences[self.current_frame];

        unsafe {
            device
                .wait_for_fences(&[fence], true, u64::MAX)
                .map_err(|e| SyncError::waitfailed(format!("Failed to wait for fence: {:?}", e)))?;
        }

        Ok(())
    }

    /// Reset the current frame's fence (prepare for next frame)
    ///
    /// Call this after waiting for the fence and before submitting work.
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::FrameSync;
    /// # let device: ash::Device = todo!();
    /// # let sync = FrameSync::create(&device, 2)?;
    /// sync.wait_for_frame(&device)?;
    /// sync.reset_fence(&device)?;
    /// # Ok::<(), engine_renderer::SyncError>(())
    /// ```
    pub fn reset_fence(&self, device: &ash::Device) -> Result<(), SyncError> {
        let fence = self.in_flight_fences[self.current_frame];

        unsafe {
            device
                .reset_fences(&[fence])
                .map_err(|e| SyncError::resetfailed(format!("Failed to reset fence: {:?}", e)))?;
        }

        Ok(())
    }

    /// Advance to the next frame
    ///
    /// Call this at the end of the frame, after present.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::FrameSync;
    /// # let device: ash::Device = todo!();
    /// # let mut sync = FrameSync::create(&device, 2)?;
    /// // After rendering and presenting
    /// sync.advance_frame();
    /// # Ok::<(), engine_renderer::SyncError>(())
    /// ```
    pub fn advance_frame(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.frames_in_flight;
    }

    /// Get current frame's semaphores and fence
    ///
    /// Returns a `FrameResources` struct containing the synchronization
    /// primitives for the current frame.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::FrameSync;
    /// # let device: ash::Device = todo!();
    /// # let sync = FrameSync::create(&device, 2)?;
    /// let resources = sync.current_frame_resources();
    /// // Use resources.image_available with acquire_next_image
    /// // Use resources.render_finished with queue_present
    /// // Use resources.in_flight_fence with queue_submit
    /// # Ok::<(), engine_renderer::SyncError>(())
    /// ```
    pub fn current_frame_resources(&self) -> FrameResources {
        FrameResources {
            image_available: self.image_available_semaphores[self.current_frame],
            render_finished: self.render_finished_semaphores[self.current_frame],
            in_flight_fence: self.in_flight_fences[self.current_frame],
        }
    }

    /// Destroy all synchronization objects
    ///
    /// # Safety
    ///
    /// Must ensure no GPU operations are in flight before calling this.
    /// Typically called after device.device_wait_idle().
    ///
    /// # Arguments
    ///
    /// * `device` - Logical Vulkan device
    pub fn destroy(&self, device: &ash::Device) {
        debug!("Destroying frame sync objects");
        unsafe {
            for &semaphore in &self.image_available_semaphores {
                device.destroy_semaphore(semaphore, None);
            }
            for &semaphore in &self.render_finished_semaphores {
                device.destroy_semaphore(semaphore, None);
            }
            for &fence in &self.in_flight_fences {
                device.destroy_fence(fence, None);
            }
        }
    }
}

// ============================================================================
// Legacy API - kept for backward compatibility
// ============================================================================

/// Synchronization objects for one frame in flight (LEGACY)
///
/// **NOTE**: This is the legacy API. New code should use `FrameSync` instead.
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

        let image_available_semaphore = unsafe { device.create_semaphore(&semaphore_info, None) }
            .map_err(|e| {
            SyncError::creationfailed(format!("Failed to create image_available semaphore: {}", e))
        })?;

        let render_finished_semaphore = unsafe { device.create_semaphore(&semaphore_info, None) }
            .map_err(|e| {
            // Clean up first semaphore on error
            unsafe { device.destroy_semaphore(image_available_semaphore, None) };
            SyncError::creationfailed(format!("Failed to create render_finished semaphore: {}", e))
        })?;

        // Create fence in signaled state (first frame doesn't wait)
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        let in_flight_fence = unsafe { device.create_fence(&fence_info, None) }.map_err(|e| {
            // Clean up semaphores on error
            unsafe {
                device.destroy_semaphore(image_available_semaphore, None);
                device.destroy_semaphore(render_finished_semaphore, None);
            }
            SyncError::creationfailed(format!("Failed to create fence: {}", e))
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
        unsafe { device.wait_for_fences(&[self.in_flight_fence], true, timeout_ns) }
            .map_err(|e| SyncError::waitfailed(format!("vkWaitForFences failed: {}", e)))
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
        unsafe { device.reset_fences(&[self.in_flight_fence]) }
            .map_err(|e| SyncError::resetfailed(format!("vkResetFences failed: {}", e)))
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

/// Create synchronization objects for multiple frames in flight (LEGACY)
///
/// **NOTE**: This is the legacy API. New code should use `FrameSync::create()` instead.
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
        "Creating sync objects for frames in flight (legacy API)"
    );

    (0..frames_in_flight).map(|_| FrameSyncObjects::new(device)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_error_display() {
        let err = SyncError::creationfailed("test error".to_string());
        let msg = err.to_string();
        assert!(msg.contains("CreationFailed") || msg.contains("test error"));
    }

    #[test]
    fn test_frame_resources_creation() {
        // Just test that the struct can be created
        let resources = FrameResources {
            image_available: vk::Semaphore::null(),
            render_finished: vk::Semaphore::null(),
            in_flight_fence: vk::Fence::null(),
        };
        assert_eq!(resources.image_available, vk::Semaphore::null());
    }
}
