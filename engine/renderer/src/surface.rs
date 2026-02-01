//! Vulkan surface creation and management
//!
//! Provides cross-platform Vulkan surface creation from winit windows
//! using the ash-window helper library.

use crate::window::Window;
use ash::vk;
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use tracing::{debug, info};

// Surface errors using define_error! macro per CLAUDE.md
define_error! {
    pub enum SurfaceError {
        CreationFailed { details: String } =
            ErrorCode::SurfaceCreationFailedRenderer,
            ErrorSeverity::Error,

        QueryFailed { details: String } =
            ErrorCode::SurfaceCapabilitiesQueryFailed,
            ErrorSeverity::Error,
    }
}

/// Vulkan surface for window presentation
///
/// Wraps VkSurfaceKHR and provides safe access to surface capabilities.
/// The surface is automatically destroyed when dropped.
pub struct Surface {
    surface: vk::SurfaceKHR,
    surface_loader: ash::khr::surface::Instance,
}

impl Surface {
    /// Create a Vulkan surface from a window
    ///
    /// Uses ash-window to create platform-specific surface from raw window handles.
    ///
    /// # Arguments
    ///
    /// * `entry` - Vulkan entry point
    /// * `instance` - Vulkan instance
    /// * `window` - Window to create surface for
    ///
    /// # Example
    ///
    /// ```no_run
    /// use engine_renderer::{Surface, Window, WindowConfig};
    /// use ash::Entry;
    ///
    /// let entry = Entry::linked();
    /// // ... create instance ...
    /// # let instance = todo!();
    /// let window = Window::new(WindowConfig::default())?;
    /// let surface = Surface::new(&entry, &instance, &window)?;
    /// # Ok::<(), engine_renderer::SurfaceError>(())
    /// ```
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &Window,
    ) -> Result<Self, SurfaceError> {
        info!("Creating Vulkan surface");

        // Get window handles for surface creation
        let display_handle = window.raw_display_handle();
        let window_handle = window.raw_window_handle();

        // Create surface using ash-window helper
        let surface = unsafe {
            ash_window::create_surface(
                entry,
                instance,
                display_handle,
                window_handle,
                None, // No custom allocator
            )
        }
        .map_err(|e| SurfaceError::CreationFailed {
            details: format!("ash_window::create_surface failed: {}", e),
        })?;

        // Create surface loader for queries
        let surface_loader = ash::khr::surface::Instance::new(entry, instance);

        debug!(
            surface = ?surface,
            "Surface created successfully"
        );

        Ok(Self { surface, surface_loader })
    }

    /// Get the raw surface handle
    #[inline]
    pub fn handle(&self) -> vk::SurfaceKHR {
        self.surface
    }

    /// Get the surface loader for capability queries
    #[inline]
    pub fn loader(&self) -> &ash::khr::surface::Instance {
        &self.surface_loader
    }

    /// Check if a physical device supports presentation on this surface
    ///
    /// # Arguments
    ///
    /// * `physical_device` - Physical device to check
    /// * `queue_family_index` - Queue family index to check
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use engine_renderer::Surface;
    /// # let surface: Surface = todo!();
    /// # let physical_device = todo!();
    /// let supported = surface.is_supported(physical_device, 0)?;
    /// if supported {
    ///     // Device supports presentation
    /// }
    /// # Ok::<(), engine_renderer::SurfaceError>(())
    /// ```
    pub fn is_supported(
        &self,
        physical_device: vk::PhysicalDevice,
        queue_family_index: u32,
    ) -> Result<bool, SurfaceError> {
        unsafe {
            self.surface_loader.get_physical_device_surface_support(
                physical_device,
                queue_family_index,
                self.surface,
            )
        }
        .map_err(|e| SurfaceError::QueryFailed {
            details: format!("Failed to query surface support: {}", e),
        })
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        debug!("Destroying Vulkan surface");
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_error_display() {
        let err = SurfaceError::CreationFailed { details: "test error".to_string() };
        let msg = err.to_string();
        assert!(msg.contains("CreationFailed") || msg.contains("test error"));

        let err2 = SurfaceError::QueryFailed { details: "query failed".to_string() };
        let msg2 = err2.to_string();
        assert!(msg2.contains("QueryFailed") || msg2.contains("query failed"));
    }
}
