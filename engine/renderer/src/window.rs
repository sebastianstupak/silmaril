//! Cross-platform window management using winit
//!
//! Provides window creation, event handling, and Vulkan surface integration
//! through the raw-window-handle abstraction layer.

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use std::ffi::c_char;
use tracing::{debug, info};
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window as WinitWindow, WindowAttributes};

#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;

/// Window configuration
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title
    pub title: String,
    /// Window width in pixels
    pub width: u32,
    /// Window height in pixels
    pub height: u32,
    /// Whether window can be resized
    pub resizable: bool,
    /// Whether window is visible (false for headless testing)
    pub visible: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Silmaril".to_string(),
            width: 1280,
            height: 720,
            resizable: true,
            visible: false, // Default to headless for testing
        }
    }
}

// Window errors using define_error! macro per CLAUDE.md
define_error! {
    pub enum WindowError {
        InvalidDimensions { width: u32, height: u32 } =
            ErrorCode::InvalidFormat,
            ErrorSeverity::Error,

        CreationFailed { details: String } =
            ErrorCode::WindowCreationFailed,
            ErrorSeverity::Error,

        EventLoopError { details: String } =
            ErrorCode::WindowCreationFailed,
            ErrorSeverity::Error,
    }
}

/// Window event
#[derive(Debug, Clone)]
pub enum WindowEventType {
    /// Window close requested
    CloseRequested,
    /// Window resized
    Resized {
        /// New width in pixels
        width: u32,
        /// New height in pixels
        height: u32,
    },
    /// Window gained focus
    Focused,
    /// Window lost focus
    Unfocused,
    /// Key pressed
    KeyPressed {
        /// Key name as string
        key: String,
    },
    /// Other events (ignored for now)
    Other,
}

/// Cross-platform window
pub struct Window {
    winit_window: WinitWindow,
    // TODO: Event loop will be used in renderer.rs for proper event handling
    #[allow(dead_code)]
    event_loop: Option<EventLoop<()>>,
    should_close: bool,
    pending_events: Vec<WindowEventType>,
}

impl std::fmt::Debug for Window {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Window")
            .field("size", &self.size())
            .field("should_close", &self.should_close)
            .field("pending_events", &self.pending_events.len())
            .finish()
    }
}

impl Window {
    /// Create a new window with the given configuration
    pub fn new(config: WindowConfig) -> Result<Self, WindowError> {
        info!(
            title = %config.title,
            width = config.width,
            height = config.height,
            "Creating window"
        );

        // Validate dimensions
        if config.width == 0 || config.height == 0 {
            return Err(WindowError::invaliddimensions(config.width, config.height));
        }

        // Create event loop with platform-specific settings
        // For Windows tests, we need any_thread() to allow creation outside main thread
        #[cfg(target_os = "windows")]
        let event_loop = EventLoop::builder()
            .with_any_thread(true)
            .build()
            .map_err(|e| WindowError::eventlooperror(e.to_string()))?;

        #[cfg(not(target_os = "windows"))]
        let event_loop =
            EventLoop::new().map_err(|e| WindowError::eventlooperror(e.to_string()))?;

        // Build window attributes
        let mut window_attrs = WindowAttributes::default()
            .with_title(config.title)
            .with_inner_size(PhysicalSize::new(config.width, config.height))
            .with_resizable(config.resizable)
            .with_visible(config.visible);

        // For testing, we want headless
        if !config.visible {
            window_attrs = window_attrs.with_visible(false);
        }

        // TODO: winit 0.30 deprecates EventLoop::create_window in favor of
        // ActiveEventLoop::create_window (only available in resumed callback).
        // This is acceptable for testing; proper event loop integration will be
        // in renderer.rs when implementing the full render loop.
        #[allow(deprecated)]
        let winit_window = event_loop
            .create_window(window_attrs)
            .map_err(|e| WindowError::creationfailed(e.to_string()))?;

        debug!("Window created successfully");

        Ok(Self {
            winit_window,
            event_loop: Some(event_loop),
            should_close: false,
            pending_events: Vec::new(),
        })
    }

    /// Get the required Vulkan extensions for this window
    pub fn required_extensions(&self) -> Vec<*const c_char> {
        // Use ash-window to get required extensions
        let display_handle = self
            .winit_window
            .display_handle()
            .expect("Failed to get display handle")
            .as_raw();

        ash_window::enumerate_required_extensions(display_handle)
            .expect("Failed to enumerate required extensions")
            .to_vec()
    }

    /// Get window size in pixels
    pub fn size(&self) -> (u32, u32) {
        let size = self.winit_window.inner_size();
        (size.width, size.height)
    }

    /// Check if window should close
    pub fn should_close(&self) -> bool {
        self.should_close
    }

    /// Poll events (returns immediately)
    pub fn poll_events(&mut self) -> Vec<WindowEventType> {
        // For now, return empty - proper event loop in renderer.rs
        // This is just for testing
        std::mem::take(&mut self.pending_events)
    }

    /// Take ownership of the event loop for manual pumping
    ///
    /// This allows the application to pump events using winit 0.30's
    /// pump_app_events() method for proper event handling.
    pub fn take_event_loop(&mut self) -> Option<EventLoop<()>> {
        self.event_loop.take()
    }

    /// Get reference to the underlying winit window
    ///
    /// Useful for calling winit-specific methods like request_redraw()
    pub fn winit_window(&self) -> &WinitWindow {
        &self.winit_window
    }

    /// Get raw window handle for Vulkan surface creation
    pub fn raw_window_handle(&self) -> RawWindowHandle {
        self.winit_window.window_handle().expect("Failed to get window handle").as_raw()
    }

    /// Get raw display handle for Vulkan surface creation
    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        self.winit_window
            .display_handle()
            .expect("Failed to get display handle")
            .as_raw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_config_builder() {
        let config = WindowConfig {
            title: "Test".to_string(),
            width: 1024,
            height: 768,
            resizable: false,
            visible: false,
        };

        assert_eq!(config.title, "Test");
        assert_eq!(config.width, 1024);
        assert_eq!(config.height, 768);
        assert!(!config.resizable);
        assert!(!config.visible);
    }

    #[test]
    fn test_window_error_display() {
        let err = WindowError::invaliddimensions(0, 0);
        // The define_error! macro generates error messages automatically
        let msg = err.to_string();
        assert!(msg.contains("InvalidDimensions") || msg.contains("Invalid"));

        let err2 = WindowError::creationfailed("test error".to_string());
        let msg2 = err2.to_string();
        assert!(msg2.contains("CreationFailed") || msg2.contains("test error"));
    }
}
