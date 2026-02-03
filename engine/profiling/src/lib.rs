// Profiling crate uses intentional casts and string formatting for performance measurement
// These are well-tested and intentional design choices
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::format_push_string)]

//! Profiling and observability infrastructure for the Silmaril.
//!
//! This crate provides a zero-cost profiling abstraction that can be compiled away
//! completely when profiling features are disabled. When enabled, it provides:
//!
//! - **Frame-level metrics**: FPS, frame time, memory usage
//! - **Scope-level profiling**: Detailed timing for specific code sections
//! - **Category-based organization**: Group metrics by subsystem (ECS, Rendering, etc.)
//! - **Performance budgets**: Automatic warnings when operations exceed budgets
//! - **Export formats**: Chrome Tracing JSON for visualization
//! - **AI agent queries**: Programmatic access to metrics for automated development
//!
//! # Feature Flags
//!
//! - `profiling-puffin`: Enable Puffin profiler backend (50-200ns overhead per scope)
//! - `profiling-tracy`: Enable Tracy profiler backend (< 10ns overhead per scope)
//! - `metrics`: Enable lightweight metrics collection (1-2μs per frame)
//! - `config`: Enable YAML configuration file support
//! - `backtrace`: Enable backtrace capture (for debugging)
//!
//! # Quick Start
//!
//! ```rust
//! use silmaril_profiling::{Profiler, ProfilerConfig, ProfileCategory, profile_scope};
//!
//! # #[cfg(feature = "metrics")]
//! # {
//! let profiler = Profiler::new(ProfilerConfig::default());
//!
//! // Begin a frame
//! profiler.begin_frame();
//!
//! // Profile a scope
//! {
//!     let _guard = profiler.scope("game_logic", ProfileCategory::ECS);
//!     // ... game logic code ...
//! }
//!
//! // End frame and get metrics
//! let metrics = profiler.end_frame();
//! println!("Frame time: {}ms", metrics.frame_time_ms);
//! # }
//! ```
//!
//! # Zero-Cost Abstraction
//!
//! When profiling features are disabled, all profiling code compiles to nothing:
//!
//! ```rust
//! use silmaril_profiling::profile_scope;
//!
//! fn expensive_function() {
//!     profile_scope!("expensive_work");
//!     // Without profiling features, the macro expands to nothing
//!     // Zero runtime overhead
//! }
//! ```
//!
//! # Architecture
//!
//! The profiling system is organized into three tiers:
//!
//! **Tier 0**: Always-available metrics (optional via `metrics` feature)
//! - Frame time, FPS, memory usage, entity count
//! - ~1-2μs overhead per frame
//! - Zero cost if feature disabled
//!
//! **Tier 1**: Lightweight profiling (via `metrics` feature)
//! - Per-system timing (~50 scopes)
//! - <0.1ms overhead per frame
//! - High-level performance tracking
//!
//! **Tier 2**: Deep profiling (via `profiling-puffin` feature)
//! - Detailed scope instrumentation (~200-500 scopes)
//! - 50-200ns per scope overhead
//! - Chrome Tracing export
//! - Visual timeline analysis

#![warn(missing_docs)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

mod config;
mod feedback_metrics;
mod profiler;
pub mod query;

#[cfg(any(feature = "profiling-puffin", feature = "profiling-tracy"))]
pub mod backends;

#[cfg(feature = "profiling-puffin")]
pub mod export;

pub use config::{
    format_duration, parse_duration, ConfigError, ProfileFormat, ProfilerConfig, RetentionConfig,
};
pub use feedback_metrics::AgentFeedbackMetrics;
pub use profiler::{FrameMetrics, Profiler, ScopeGuard};
pub use query::{AggregateMetrics, QueryBuilder, TimelineEvent};

#[cfg(feature = "profiling-puffin")]
pub use backends::PuffinBackend;

#[cfg(feature = "profiling-tracy")]
pub use backends::TracyBackend;

/// Categories for organizing profiling data by subsystem.
///
/// This matches industry standards (Unity, Unreal) for profiling organization.
/// Categories allow filtering and aggregating metrics by engine subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ProfileCategory {
    /// Entity Component System operations
    ECS,
    /// Vulkan rendering and GPU operations
    Rendering,
    /// Physics simulation and collision detection
    Physics,
    /// Client/server networking and state synchronization
    Networking,
    /// Audio playback and processing
    Audio,
    /// Binary serialization and deserialization
    Serialization,
    /// Game logic scripts (future)
    Scripts,
    /// Uncategorized profiling data
    Unknown,
}

impl ProfileCategory {
    /// Get the string name of this category.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ProfileCategory::ECS => "ECS",
            ProfileCategory::Rendering => "Rendering",
            ProfileCategory::Physics => "Physics",
            ProfileCategory::Networking => "Networking",
            ProfileCategory::Audio => "Audio",
            ProfileCategory::Serialization => "Serialization",
            ProfileCategory::Scripts => "Scripts",
            ProfileCategory::Unknown => "Unknown",
        }
    }
}

impl std::fmt::Display for ProfileCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Macro for creating a profiling scope that automatically ends when dropped.
///
/// This macro compiles to nothing when profiling features are disabled,
/// providing zero runtime overhead.
///
/// Note: When using this macro, you need to have a profiler instance in scope.
/// For best results, use `Profiler::scope()` directly in application code.
///
/// # Examples
///
/// ```rust
/// use silmaril_profiling::profile_scope;
///
/// fn game_loop() {
///     profile_scope!("game_loop");
///
///     // The scope automatically ends when the function returns
///     // or when the guard is dropped
/// }
/// ```
///
/// With explicit category:
///
/// ```rust
/// use silmaril_profiling::{profile_scope, ProfileCategory};
///
/// fn physics_update() {
///     profile_scope!("physics_step", ProfileCategory::Physics);
///     // Physics code here
/// }
/// ```
#[cfg(feature = "profiling-tracy")]
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        // When Tracy is enabled, use tracy_client directly
        // Tracy's span! macro creates a guard that auto-ends on drop
        let _tracy_span = tracy_client::span!($name);
    };
    ($name:expr, $category:expr) => {
        // Tracy requires string literals at compile time for optimal performance.
        // Categories are not directly supported, so we just use the name.
        // Users can manually prefix names with categories in the string literal if needed.
        // Example: profile_scope!("Physics::update") instead of profile_scope!("update", ProfileCategory::Physics)
        let _tracy_span = tracy_client::span!($name);
        // Silence unused variable warning
        let _ = $category;
    };
}

/// Profile scope macro for Puffin backend.
///
/// Records a profiling scope with optional category.
///
/// # Examples
///
/// ```ignore
/// profile_scope!("my_function");
/// profile_scope!("my_function", ProfileCategory::ECS);
/// ```
#[cfg(all(feature = "profiling-puffin", not(feature = "profiling-tracy")))]
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        // When Puffin is enabled, use puffin directly
        puffin::profile_scope!($name);
    };
    ($name:expr, $category:expr) => {
        // When Puffin is enabled, use puffin with category
        puffin::profile_scope!($name, $category.as_str());
    };
}

/// Zero-cost version of `profile_scope!` when profiling is disabled.
///
/// This macro expands to nothing, ensuring zero runtime overhead.
#[cfg(not(any(feature = "profiling-puffin", feature = "profiling-tracy")))]
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {};
    ($name:expr, $category:expr) => {};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_category_as_str() {
        assert_eq!(ProfileCategory::ECS.as_str(), "ECS");
        assert_eq!(ProfileCategory::Rendering.as_str(), "Rendering");
        assert_eq!(ProfileCategory::Physics.as_str(), "Physics");
        assert_eq!(ProfileCategory::Networking.as_str(), "Networking");
        assert_eq!(ProfileCategory::Audio.as_str(), "Audio");
        assert_eq!(ProfileCategory::Serialization.as_str(), "Serialization");
        assert_eq!(ProfileCategory::Scripts.as_str(), "Scripts");
        assert_eq!(ProfileCategory::Unknown.as_str(), "Unknown");
    }

    #[test]
    fn test_profile_category_display() {
        assert_eq!(format!("{}", ProfileCategory::ECS), "ECS");
        assert_eq!(format!("{}", ProfileCategory::Rendering), "Rendering");
    }

    #[test]
    fn test_profile_scope_macro_compiles() {
        // Test that the macro compiles without error
        profile_scope!("test_scope");
        profile_scope!("test_scope_with_category", ProfileCategory::ECS);
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_profiler_creation() {
        let profiler = Profiler::new(ProfilerConfig::default());
        // Just ensure it can be created
        drop(profiler);
    }
}
