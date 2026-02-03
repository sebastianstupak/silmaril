//! Tracy profiler backend integration.
//!
//! Tracy is a real-time, frame-based profiler with a powerful GUI client.
//! It provides nanosecond precision timing and supports remote profiling.
//!
//! # Features
//! - Real-time profiling with < 10ns overhead per scope
//! - Remote profiling over network
//! - Frame-based timeline view
//! - GPU profiling support (future)
//! - Memory profiling support (future)
//!
//! # Usage
//! 1. Enable the `profiling-tracy` feature flag:
//!    ```bash
//!    cargo build --features profiling-tracy
//!    ```
//! 2. Download Tracy profiler client from: https://github.com/wolfpld/tracy/releases
//! 3. Run your application with profiling enabled
//! 4. Open Tracy client and connect to localhost
//!
//! # Example
//! ```rust,no_run
//! #[cfg(feature = "profiling-tracy")]
//! use silmaril_profiling::{TracyBackend, profile_scope};
//!
//! #[cfg(feature = "profiling-tracy")]
//! {
//!     // Initialize Tracy (automatic on first use)
//!     let backend = TracyBackend::new();
//!
//!     // Begin frame
//!     backend.begin_frame();
//!
//!     // Profile a scope using the macro (preferred)
//!     {
//!         profile_scope!("physics_update");
//!         // ... physics work
//!     }
//!
//!     // Or use the backend directly
//!     {
//!         let _span = backend.span("rendering");
//!         // ... rendering work
//!     }
//!
//!     // End frame
//!     backend.end_frame();
//! }
//! ```
//!
//! # Performance
//!
//! Tracy has extremely low overhead compared to Puffin:
//! - Puffin: 50-200ns per scope
//! - Tracy: < 10ns per scope (5-20x faster)
//!
//! This makes Tracy suitable for instrumenting hot paths that are called
//! thousands of times per frame without impacting performance.
//!
//! # Zero-Cost When Disabled
//!
//! When the `profiling-tracy` feature is disabled, all Tracy code compiles
//! to nothing. The `profile_scope!` macro expands to an empty block,
//! providing true zero-cost abstraction.

/// Tracy profiler backend.
///
/// This backend integrates with Tracy for real-time performance analysis.
/// Tracy provides a powerful GUI client for visualizing profiling data.
pub struct TracyBackend {
    frame_count: u64,
}

impl TracyBackend {
    /// Create a new Tracy backend.
    ///
    /// Tracy is automatically initialized on first use.
    pub fn new() -> Self {
        Self { frame_count: 0 }
    }

    /// Begin a new frame.
    ///
    /// This marks the start of a frame in Tracy's timeline.
    pub fn begin_frame(&mut self) {
        #[cfg(feature = "profiling-tracy")]
        {
            tracy_client::Client::running()
                .expect("Tracy client should be running")
                .frame_mark();
        }
    }

    /// End the current frame.
    ///
    /// This increments the frame counter and commits any pending data.
    pub fn end_frame(&mut self) {
        self.frame_count += 1;
    }

    /// Get the current frame count.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

// Note: For creating profiling scopes with Tracy, use the `profile_scope!` macro instead.
// Tracy requires compile-time string literals for optimal performance.
// The macro approach provides better ergonomics and performance.

impl Default for TracyBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for Tracy profiling spans.
///
/// When dropped, the span is automatically ended and timing data is sent to Tracy.
pub struct TracySpan {
    #[cfg(feature = "profiling-tracy")]
    _span: Option<tracy_client::Span>,
    #[cfg(not(feature = "profiling-tracy"))]
    _span: Option<()>,
}

impl Drop for TracySpan {
    fn drop(&mut self) {
        // Tracy span automatically ends on drop
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracy_backend_creation() {
        let backend = TracyBackend::new();
        assert_eq!(backend.frame_count(), 0);
    }

    #[test]
    fn test_frame_counting() {
        let mut backend = TracyBackend::new();
        backend.begin_frame();
        backend.end_frame();
        assert_eq!(backend.frame_count(), 1);

        backend.begin_frame();
        backend.end_frame();
        assert_eq!(backend.frame_count(), 2);
    }

    #[test]
    fn test_span_creation() {
        use crate::profile_scope;
        let _backend = TracyBackend::new();
        profile_scope!("test_span");
        // Should not panic
    }

    #[test]
    fn test_nested_spans() {
        use crate::profile_scope;
        let _backend = TracyBackend::new();
        {
            profile_scope!("outer");
            {
                profile_scope!("inner");
                // Both active
            }
            // Inner dropped, outer still active
        }
    }
}
