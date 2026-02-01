//! Engine Observability
//!
//! Provides performance monitoring:
//! - Metrics collection (CPU, GPU, memory, network)
//! - Frame profiling
//! - External profiler integration (Tracy, Puffin)
//! - Performance validation

#![warn(missing_docs)]

pub mod profiler;
pub mod metrics;
pub mod overlay;

// Re-export commonly used types
pub use profiler::Profiler;
pub use metrics::Metrics;
