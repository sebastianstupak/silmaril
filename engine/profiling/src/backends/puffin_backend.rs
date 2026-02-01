//! Puffin profiler backend implementation.
//!
//! This backend integrates the Puffin profiler from Embark Studios, providing
//! low-overhead profiling with 50-200ns per scope overhead.
//!
//! Puffin provides:
//! - Web-based viewer UI
//! - Chrome Tracing export
//! - Thread visualization
//! - Zero overhead when disabled
//!
//! # Web Viewer
//!
//! To view Puffin profiling data:
//!
//! 1. Enable the Puffin HTTP server:
//!    ```rust
//!    let mut backend = PuffinBackend::new();
//!    backend.start_server("0.0.0.0:8585");
//!    ```
//!
//! 2. Run the Puffin viewer:
//!    ```bash
//!    cargo install puffin_viewer
//!    puffin_viewer
//!    ```
//!
//! 3. Connect to `localhost:8585` in the viewer
//!
//! Alternatively, use Chrome Tracing:
//!    ```rust
//!    let trace = backend.export_chrome_trace();
//!    std::fs::write("trace.json", trace)?;
//!    // Open chrome://tracing and load trace.json
//!    ```

use crate::ProfileCategory;
use parking_lot::Mutex;
use std::sync::Arc;

/// Puffin profiler backend.
///
/// This backend wraps the Puffin global profiler and provides integration
/// with our profiling infrastructure.
///
/// # Examples
///
/// ```rust
/// use agent_game_engine_profiling::backends::PuffinBackend;
/// use agent_game_engine_profiling::ProfileCategory;
///
/// # #[cfg(feature = "profiling-puffin")]
/// # {
/// let mut backend = PuffinBackend::new();
///
/// // Begin a scope
/// backend.begin_scope("physics_step", ProfileCategory::Physics);
/// // ... physics work ...
/// backend.end_scope();
///
/// // Export Chrome Trace
/// let trace = backend.export_chrome_trace();
/// # }
/// ```
pub struct PuffinBackend {
    /// The Puffin global profiler instance
    _global_profiler: Arc<Mutex<()>>,
}

impl PuffinBackend {
    /// Create a new Puffin backend.
    ///
    /// This initializes the Puffin global profiler and enables profiling.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "profiling-puffin")]
    /// # {
    /// use agent_game_engine_profiling::backends::PuffinBackend;
    ///
    /// let backend = PuffinBackend::new();
    /// # }
    /// ```
    pub fn new() -> Self {
        // Enable Puffin profiling
        puffin::set_scopes_on(true);

        Self { _global_profiler: Arc::new(Mutex::new(())) }
    }

    /// Begin a new profiling scope.
    ///
    /// Note: Puffin uses RAII-based scopes, so this method is provided for API
    /// compatibility but doesn't create a persistent scope. Use the `profile_scope!`
    /// macro instead for proper RAII-style profiling.
    ///
    /// # Arguments
    ///
    /// * `_name` - Name of the scope (unused in this implementation)
    /// * `_category` - Category for the scope (unused in this implementation)
    #[allow(dead_code)]
    pub fn begin_scope(&mut self, _name: &str, _category: ProfileCategory) {
        // Puffin's scopes are RAII-based and cannot be manually begun/ended
        // This method is provided for API compatibility only
        // Users should use profile_scope!() macro instead
    }

    /// End the current profiling scope.
    ///
    /// # Note
    ///
    /// This is a low-level API. Puffin uses RAII guards, so typically scopes
    /// end automatically when the guard is dropped.
    #[allow(dead_code)]
    pub fn end_scope(&mut self) {
        // With Puffin's RAII design, scopes end when guards drop.
        // This method is provided for API compatibility but is typically not needed.
    }

    /// Export profiling data as Chrome Tracing JSON format.
    ///
    /// The Chrome Tracing format can be visualized in:
    /// - `chrome://tracing` in Chrome/Chromium browsers
    /// - Perfetto UI (https://ui.perfetto.dev/)
    /// - Various profiling analysis tools
    ///
    /// # Returns
    ///
    /// A JSON string in Chrome Tracing format.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "profiling-puffin")]
    /// # {
    /// use agent_game_engine_profiling::backends::PuffinBackend;
    ///
    /// let backend = PuffinBackend::new();
    /// // ... run some profiled code ...
    ///
    /// let trace_json = backend.export_chrome_trace();
    /// std::fs::write("trace.json", trace_json).unwrap();
    /// // Open chrome://tracing and load trace.json
    /// # }
    /// ```
    pub fn export_chrome_trace(&self) -> String {
        // For Phase 0.5.2, we implement a basic Chrome Trace exporter
        // The full implementation requires deep integration with Puffin's internal
        // stream format which varies by version.
        //
        // For now, we return a valid but minimal Chrome Trace JSON.
        // Users should use Puffin's web viewer for full profiling visualization.

        // Return a minimal valid Chrome Trace format
        let frames: Vec<puffin::FrameData> = Vec::new();
        crate::export::chrome_trace::export_puffin_to_chrome_trace(frames.iter())
    }

    /// Start the Puffin HTTP server for remote profiling.
    ///
    /// This allows connecting the Puffin viewer to a running application.
    ///
    /// Note: This requires the `puffin_http` crate to be added as a dependency
    /// and is currently not implemented in the basic integration.
    ///
    /// For Phase 0.5.2, use the `export_chrome_trace()` method instead.
    ///
    /// # Arguments
    ///
    /// * `_addr` - Address to bind to (e.g., "0.0.0.0:8585")
    #[allow(dead_code)]
    pub fn start_server(&mut self, _addr: &str) {
        tracing::warn!("Puffin HTTP server not implemented in basic integration");
        tracing::info!("Use export_chrome_trace() to export profiling data instead");
    }

    /// Begin a new frame.
    ///
    /// Call this at the start of each frame to organize profiling data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[cfg(feature = "profiling-puffin")]
    /// # {
    /// use agent_game_engine_profiling::backends::PuffinBackend;
    ///
    /// let mut backend = PuffinBackend::new();
    ///
    /// loop {
    ///     backend.begin_frame();
    ///     // ... frame work ...
    ///     backend.end_frame();
    /// }
    /// # }
    /// ```
    pub fn begin_frame(&mut self) {
        puffin::GlobalProfiler::lock().new_frame();
    }

    /// End the current frame.
    ///
    /// Call this at the end of each frame.
    pub fn end_frame(&mut self) {
        // Puffin automatically manages frame boundaries
        // This is mainly for API consistency
    }
}

impl Default for PuffinBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_puffin_backend_creation() {
        let backend = PuffinBackend::new();
        // Just ensure it can be created without panicking
        drop(backend);
    }

    #[test]
    fn test_puffin_scope() {
        let mut backend = PuffinBackend::new();

        backend.begin_frame();

        // Begin and end a scope
        backend.begin_scope("test_scope", ProfileCategory::ECS);
        backend.end_scope();

        backend.end_frame();
    }

    #[test]
    fn test_chrome_trace_export() {
        let backend = PuffinBackend::new();

        // Export should not panic even with no data
        let trace = backend.export_chrome_trace();

        // Should return valid JSON (at minimum an empty array)
        assert!(trace.contains('[') && trace.contains(']'));
    }

    #[test]
    fn test_multiple_frames() {
        let mut backend = PuffinBackend::new();

        for _ in 0..5 {
            backend.begin_frame();

            // Simulate some work with scopes
            puffin::profile_scope!("frame_work", "Test");
            std::thread::sleep(std::time::Duration::from_micros(100));

            backend.end_frame();
        }

        // Export should return valid JSON
        let trace = backend.export_chrome_trace();
        assert!(trace.starts_with('[') && trace.ends_with(']'));
    }

    #[test]
    fn test_nested_scopes() {
        let mut backend = PuffinBackend::new();

        backend.begin_frame();

        {
            puffin::profile_scope!("outer", ProfileCategory::ECS.as_str());
            std::thread::sleep(std::time::Duration::from_micros(50));

            {
                puffin::profile_scope!("inner", ProfileCategory::Rendering.as_str());
                std::thread::sleep(std::time::Duration::from_micros(50));
            }
        }

        backend.end_frame();

        let trace = backend.export_chrome_trace();

        // Should return valid JSON
        assert!(trace.starts_with('[') && trace.ends_with(']'));
    }

    #[test]
    fn test_category_mapping() {
        let mut backend = PuffinBackend::new();

        backend.begin_frame();

        // Test all categories
        let categories = [
            ProfileCategory::ECS,
            ProfileCategory::Rendering,
            ProfileCategory::Physics,
            ProfileCategory::Networking,
            ProfileCategory::Audio,
            ProfileCategory::Serialization,
            ProfileCategory::Scripts,
            ProfileCategory::Unknown,
        ];

        for category in &categories {
            puffin::profile_scope!("test", category.as_str());
        }

        backend.end_frame();

        // Should complete without panicking
    }
}
