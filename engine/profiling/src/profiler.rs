//! Core profiler implementation.
//!
//! This module provides the main `Profiler` type and related structures for
//! collecting performance metrics and timing data.

use crate::ProfileCategory;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Configuration for the profiler.
///
/// Controls profiling behavior, persistence, and performance budgets.
#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    /// Whether profiling is enabled
    pub enabled: bool,

    /// Whether to persist profiling data to disk
    pub persist_to_disk: bool,

    /// Output directory for profiling data
    pub output_dir: String,

    /// Maximum file size in MB before rotation
    pub max_file_size_mb: usize,

    /// Number of frames to keep in circular buffer
    pub circular_buffer_frames: usize,

    /// Whether to save data when budget is exceeded
    pub save_on_budget_exceeded: bool,

    /// Performance budgets for specific scopes (in milliseconds)
    pub budgets: HashMap<String, f32>,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self::default_dev()
    }
}

impl ProfilerConfig {
    /// Default configuration for development builds.
    pub fn default_dev() -> Self {
        let mut budgets = HashMap::new();
        budgets.insert("game_loop".to_string(), 16.0); // 60 FPS
        budgets.insert("physics_step".to_string(), 5.0);
        budgets.insert("rendering".to_string(), 8.0);
        budgets.insert("networking".to_string(), 2.0);

        Self {
            enabled: true,
            persist_to_disk: true,
            output_dir: "profiling_data".to_string(),
            max_file_size_mb: 100,
            circular_buffer_frames: 1000,
            save_on_budget_exceeded: true,
            budgets,
        }
    }

    /// Default configuration for release builds (everything disabled).
    pub fn default_release() -> Self {
        Self {
            enabled: false,
            persist_to_disk: false,
            output_dir: String::new(),
            max_file_size_mb: 0,
            circular_buffer_frames: 0,
            save_on_budget_exceeded: false,
            budgets: HashMap::new(),
        }
    }
}

/// Metrics collected for a single frame.
///
/// These metrics are available when the `metrics` feature is enabled.
#[derive(Debug, Clone)]
pub struct FrameMetrics {
    /// Frame number (incremented each frame)
    pub frame_number: u64,

    /// Total frame time in milliseconds
    pub frame_time_ms: f32,

    /// Frames per second (calculated from frame_time_ms)
    pub fps: f32,

    /// Memory usage in megabytes (approximate)
    pub memory_mb: usize,

    /// Number of entities in the world
    pub entity_count: u32,

    /// Time spent in each category (in milliseconds)
    pub time_by_category: HashMap<ProfileCategory, f32>,
}

impl Default for FrameMetrics {
    fn default() -> Self {
        Self {
            frame_number: 0,
            frame_time_ms: 0.0,
            fps: 0.0,
            memory_mb: 0,
            entity_count: 0,
            time_by_category: HashMap::new(),
        }
    }
}

/// Internal scope data for tracking timing information.
#[derive(Debug, Clone)]
struct ScopeData {
    name: String,
    category: ProfileCategory,
    start_time: Instant,
    duration: Option<Duration>,
}

/// Shared profiler state.
#[derive(Debug)]
pub(crate) struct ProfilerState {
    config: ProfilerConfig,
    frame_number: u64,
    frame_start_time: Option<Instant>,
    active_scopes: Vec<ScopeData>,
    completed_scopes: Vec<ScopeData>,
    frame_metrics_history: Vec<FrameMetrics>,
}

impl ProfilerState {
    fn new(config: ProfilerConfig) -> Self {
        Self {
            config,
            frame_number: 0,
            frame_start_time: None,
            active_scopes: Vec::new(),
            completed_scopes: Vec::new(),
            frame_metrics_history: Vec::new(),
        }
    }
}

/// Main profiler type.
///
/// The profiler collects timing data for frames and individual scopes.
/// It is thread-safe and can be shared across multiple threads.
///
/// # Examples
///
/// ```rust
/// use agent_game_engine_profiling::{Profiler, ProfilerConfig, ProfileCategory};
///
/// # #[cfg(feature = "metrics")]
/// # {
/// let mut profiler = Profiler::new(ProfilerConfig::default());
///
/// profiler.begin_frame();
///
/// {
///     let _guard = profiler.scope("physics", ProfileCategory::Physics);
///     // Physics code here
/// }
///
/// let metrics = profiler.end_frame();
/// println!("Frame time: {}ms", metrics.frame_time_ms);
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Profiler {
    state: Arc<Mutex<ProfilerState>>,
}

impl Profiler {
    /// Create a new profiler with the given configuration.
    pub fn new(config: ProfilerConfig) -> Self {
        Self { state: Arc::new(Mutex::new(ProfilerState::new(config))) }
    }

    /// Begin a new frame.
    ///
    /// This should be called at the start of each frame. It resets per-frame
    /// metrics and starts the frame timer.
    pub fn begin_frame(&self) {
        let mut state = self.state.lock();

        if !state.config.enabled {
            return;
        }

        state.frame_start_time = Some(Instant::now());
        state.active_scopes.clear();
        state.completed_scopes.clear();
    }

    /// End the current frame and return metrics.
    ///
    /// This should be called at the end of each frame. It calculates frame
    /// timing and aggregates scope data.
    pub fn end_frame(&self) -> FrameMetrics {
        let mut state = self.state.lock();

        if !state.config.enabled {
            return FrameMetrics::default();
        }

        let frame_time = if let Some(start) = state.frame_start_time {
            start.elapsed()
        } else {
            Duration::from_secs(0)
        };

        let frame_time_ms = frame_time.as_secs_f32() * 1000.0;
        let fps = if frame_time_ms > 0.0 { 1000.0 / frame_time_ms } else { 0.0 };

        // Aggregate time by category
        let mut time_by_category = HashMap::new();
        for scope in &state.completed_scopes {
            if let Some(duration) = scope.duration {
                let time_ms = duration.as_secs_f32() * 1000.0;
                *time_by_category.entry(scope.category).or_insert(0.0) += time_ms;
            }
        }

        // Check budgets and warn if exceeded
        for scope in &state.completed_scopes {
            if let Some(duration) = scope.duration {
                let time_ms = duration.as_secs_f32() * 1000.0;
                if let Some(&budget_ms) = state.config.budgets.get(&scope.name) {
                    if time_ms > budget_ms {
                        tracing::warn!(
                            scope = %scope.name,
                            category = %scope.category,
                            actual_ms = time_ms,
                            budget_ms = budget_ms,
                            frame = state.frame_number,
                            "Performance budget exceeded"
                        );
                    }
                }
            }
        }

        let metrics = FrameMetrics {
            frame_number: state.frame_number,
            frame_time_ms,
            fps,
            memory_mb: 0,    // TODO: Implement memory tracking
            entity_count: 0, // TODO: Get from world
            time_by_category,
        };

        // Store in history
        state.frame_metrics_history.push(metrics.clone());

        // Limit history size
        if state.frame_metrics_history.len() > state.config.circular_buffer_frames {
            state.frame_metrics_history.remove(0);
        }

        state.frame_number += 1;

        metrics
    }

    /// Create a new profiling scope.
    ///
    /// The scope will automatically end when the returned `ScopeGuard` is dropped.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use agent_game_engine_profiling::{Profiler, ProfilerConfig, ProfileCategory};
    ///
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// let profiler = Profiler::new(ProfilerConfig::default());
    ///
    /// {
    ///     let _guard = profiler.scope("expensive_work", ProfileCategory::ECS);
    ///     // Work happens here
    /// } // Scope ends here when _guard is dropped
    /// # }
    /// ```
    pub fn scope(&self, name: &str, category: ProfileCategory) -> ScopeGuard {
        let state = self.state.lock();

        if !state.config.enabled {
            drop(state);
            return ScopeGuard::disabled();
        }

        drop(state);

        let start_time = Instant::now();

        let scope_data = ScopeData { name: name.to_string(), category, start_time, duration: None };

        // Push to active scopes
        self.state.lock().active_scopes.push(scope_data);

        ScopeGuard::new(self.clone(), name.to_string())
    }

    /// Set a performance budget for a specific scope.
    ///
    /// If the scope exceeds this budget, a warning will be logged.
    ///
    /// # Arguments
    ///
    /// * `scope` - Name of the scope
    /// * `duration` - Budget duration
    pub fn set_budget(&self, scope: &str, duration: Duration) {
        let mut state = self.state.lock();
        let budget_ms = duration.as_secs_f32() * 1000.0;
        state.config.budgets.insert(scope.to_string(), budget_ms);
    }

    /// Internal method to end a scope.
    fn end_scope(&self, name: &str) {
        let mut state = self.state.lock();

        if !state.config.enabled {
            return;
        }

        // Find the scope in active scopes
        if let Some(pos) = state.active_scopes.iter().position(|s| s.name == name) {
            let mut scope = state.active_scopes.remove(pos);
            scope.duration = Some(scope.start_time.elapsed());
            state.completed_scopes.push(scope);
        }
    }

    /// Get the frame metrics history.
    ///
    /// Returns a vector of metrics for recent frames (limited by circular buffer size).
    pub fn frame_history(&self) -> Vec<FrameMetrics> {
        let state = self.state.lock();
        state.frame_metrics_history.clone()
    }

    /// Get the current performance budget for a scope (for testing).
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn get_budget(&self, scope: &str) -> Option<f32> {
        let state = self.state.lock();
        state.config.budgets.get(scope).copied()
    }
}

/// RAII guard for profiling scopes.
///
/// When this guard is dropped, the profiling scope is automatically ended
/// and timing data is recorded.
///
/// This type should not be created directly. Use `Profiler::scope()` instead.
pub struct ScopeGuard {
    profiler: Option<Profiler>,
    name: String,
}

impl ScopeGuard {
    fn new(profiler: Profiler, name: String) -> Self {
        Self { profiler: Some(profiler), name }
    }

    fn disabled() -> Self {
        Self { profiler: None, name: String::new() }
    }
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        if let Some(profiler) = &self.profiler {
            profiler.end_scope(&self.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_profiler_config_default() {
        let config = ProfilerConfig::default();
        assert!(config.enabled);
        assert!(config.persist_to_disk);
        assert!(config.budgets.contains_key("game_loop"));
    }

    #[test]
    fn test_profiler_config_release() {
        let config = ProfilerConfig::default_release();
        assert!(!config.enabled);
        assert!(!config.persist_to_disk);
        assert!(config.budgets.is_empty());
    }

    #[test]
    fn test_profiler_creation() {
        let profiler = Profiler::new(ProfilerConfig::default());
        assert!(profiler.state.lock().config.enabled);
    }

    #[test]
    fn test_begin_end_frame() {
        let profiler = Profiler::new(ProfilerConfig::default());

        profiler.begin_frame();
        thread::sleep(Duration::from_millis(1));
        let metrics = profiler.end_frame();

        assert_eq!(metrics.frame_number, 0);
        assert!(metrics.frame_time_ms > 0.0);
        assert!(metrics.fps > 0.0);
    }

    #[test]
    fn test_scope_timing() {
        let profiler = Profiler::new(ProfilerConfig::default());

        profiler.begin_frame();

        {
            let _guard = profiler.scope("test_scope", ProfileCategory::ECS);
            thread::sleep(Duration::from_millis(2));
        }

        let metrics = profiler.end_frame();

        assert!(metrics.time_by_category.contains_key(&ProfileCategory::ECS));
        if let Some(&time_ms) = metrics.time_by_category.get(&ProfileCategory::ECS) {
            assert!(time_ms >= 1.0); // At least 1ms
        }
    }

    #[test]
    fn test_nested_scopes() {
        let profiler = Profiler::new(ProfilerConfig::default());

        profiler.begin_frame();

        {
            let _outer = profiler.scope("outer", ProfileCategory::ECS);
            thread::sleep(Duration::from_millis(1));

            {
                let _inner = profiler.scope("inner", ProfileCategory::Rendering);
                thread::sleep(Duration::from_millis(1));
            }
        }

        let metrics = profiler.end_frame();

        assert!(metrics.time_by_category.contains_key(&ProfileCategory::ECS));
        assert!(metrics.time_by_category.contains_key(&ProfileCategory::Rendering));
    }

    #[test]
    fn test_budget_setting() {
        let profiler = Profiler::new(ProfilerConfig::default());

        profiler.set_budget("test_scope", Duration::from_millis(5));

        let state = profiler.state.lock();
        assert!(state.config.budgets.contains_key("test_scope"));
        assert_eq!(state.config.budgets.get("test_scope"), Some(&5.0));
    }

    #[test]
    fn test_frame_history() {
        let profiler = Profiler::new(ProfilerConfig::default());

        // Generate a few frames
        for _ in 0..3 {
            profiler.begin_frame();
            thread::sleep(Duration::from_millis(1));
            profiler.end_frame();
        }

        let history = profiler.frame_history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].frame_number, 0);
        assert_eq!(history[1].frame_number, 1);
        assert_eq!(history[2].frame_number, 2);
    }

    #[test]
    fn test_circular_buffer() {
        let mut config = ProfilerConfig::default();
        config.circular_buffer_frames = 2;

        let profiler = Profiler::new(config);

        // Generate 5 frames
        for _ in 0..5 {
            profiler.begin_frame();
            profiler.end_frame();
        }

        let history = profiler.frame_history();
        assert_eq!(history.len(), 2); // Only last 2 frames kept
        assert_eq!(history[0].frame_number, 3);
        assert_eq!(history[1].frame_number, 4);
    }

    #[test]
    fn test_disabled_profiler() {
        let profiler = Profiler::new(ProfilerConfig::default_release());

        profiler.begin_frame();

        {
            let _guard = profiler.scope("test", ProfileCategory::ECS);
            thread::sleep(Duration::from_millis(1));
        }

        let metrics = profiler.end_frame();

        // Metrics should be default when disabled
        assert_eq!(metrics.frame_number, 0);
        assert_eq!(metrics.frame_time_ms, 0.0);
        assert!(metrics.time_by_category.is_empty());
    }

    #[test]
    fn test_scope_guard_drop() {
        let profiler = Profiler::new(ProfilerConfig::default());

        profiler.begin_frame();

        // Create and immediately drop a scope
        {
            let guard = profiler.scope("test", ProfileCategory::ECS);
            drop(guard);
        }

        let metrics = profiler.end_frame();

        // Should have recorded the scope
        assert!(metrics.time_by_category.contains_key(&ProfileCategory::ECS));
    }
}
