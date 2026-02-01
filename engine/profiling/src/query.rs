//! Query API for programmatic access to profiling data.
//!
//! This module provides a builder-style API for querying profiling metrics,
//! designed specifically for AI agent feedback loops and automated analysis.

use crate::{ProfileCategory, Profiler};
use std::ops::Range;

/// Timeline event representing a single profiling scope.
///
/// Used for detailed timeline analysis and Chrome Trace export.
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    /// Scope name
    pub name: String,

    /// Profiling category
    pub category: ProfileCategory,

    /// Frame number when this event occurred
    pub frame: u64,

    /// Start time in microseconds (relative to frame start)
    pub start_us: u64,

    /// Duration in microseconds
    pub duration_us: u64,
}

/// Aggregated metrics for a set of profiling scopes.
///
/// Provides statistical analysis including percentiles for performance analysis.
#[derive(Debug, Clone)]
pub struct AggregateMetrics {
    /// Total time across all matching scopes (in microseconds)
    pub total_time_us: u64,

    /// Number of matching scope calls
    pub call_count: u32,

    /// Average time per call (in microseconds)
    pub avg_time_us: f32,

    /// 50th percentile (median) in microseconds
    pub p50_us: u64,

    /// 95th percentile in microseconds
    pub p95_us: u64,

    /// 99th percentile in microseconds
    pub p99_us: u64,

    /// Minimum time in microseconds
    pub min_us: u64,

    /// Maximum time in microseconds
    pub max_us: u64,
}

impl Default for AggregateMetrics {
    fn default() -> Self {
        Self {
            total_time_us: 0,
            call_count: 0,
            avg_time_us: 0.0,
            p50_us: 0,
            p95_us: 0,
            p99_us: 0,
            min_us: 0,
            max_us: 0,
        }
    }
}

impl AggregateMetrics {
    /// Calculate percentile from sorted durations.
    ///
    /// # Arguments
    ///
    /// * `sorted_durations` - Sorted array of durations
    /// * `percentile` - Percentile to calculate (0.0 to 1.0)
    fn calculate_percentile(sorted_durations: &[u64], percentile: f32) -> u64 {
        if sorted_durations.is_empty() {
            return 0;
        }

        if sorted_durations.len() == 1 {
            return sorted_durations[0];
        }

        let float_index = (sorted_durations.len() - 1) as f32 * percentile;
        let lower_index = float_index.floor() as usize;
        let upper_index = float_index.ceil() as usize;

        if lower_index == upper_index {
            sorted_durations[lower_index]
        } else {
            // Linear interpolation between the two nearest values
            let lower_value = sorted_durations[lower_index] as f32;
            let upper_value = sorted_durations[upper_index] as f32;
            let fraction = float_index - lower_index as f32;
            (lower_value + (upper_value - lower_value) * fraction) as u64
        }
    }

    /// Create aggregate metrics from a list of durations.
    fn from_durations(mut durations: Vec<u64>) -> Self {
        if durations.is_empty() {
            return Self::default();
        }

        durations.sort_unstable();

        let total_time_us: u64 = durations.iter().sum();
        let call_count = durations.len() as u32;
        let avg_time_us = total_time_us as f32 / call_count as f32;

        let p50_us = Self::calculate_percentile(&durations, 0.50);
        let p95_us = Self::calculate_percentile(&durations, 0.95);
        let p99_us = Self::calculate_percentile(&durations, 0.99);
        let min_us = durations[0];
        let max_us = durations[durations.len() - 1];

        Self { total_time_us, call_count, avg_time_us, p50_us, p95_us, p99_us, min_us, max_us }
    }
}

/// Builder for querying profiling data.
///
/// Provides a fluent API for filtering and aggregating profiling metrics.
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
/// // Query physics metrics for frames 1000-2000
/// let stats = profiler.query()
///     .frames(1000..2000)
///     .category(ProfileCategory::Physics)
///     .aggregate();
///
/// println!("Physics p95: {}us", stats.p95_us);
/// # }
/// ```
pub struct QueryBuilder<'a> {
    profiler: &'a Profiler,
    frame_range: Option<Range<usize>>,
    category_filter: Option<ProfileCategory>,
    scope_filter: Option<String>,
}

impl<'a> QueryBuilder<'a> {
    /// Create a new query builder.
    ///
    /// This is called internally by `Profiler::query()`.
    pub(crate) fn new(profiler: &'a Profiler) -> Self {
        Self { profiler, frame_range: None, category_filter: None, scope_filter: None }
    }

    /// Filter to a specific frame.
    ///
    /// # Arguments
    ///
    /// * `frame` - Frame number to query
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use agent_game_engine_profiling::{Profiler, ProfilerConfig};
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// # let profiler = Profiler::new(ProfilerConfig::default());
    /// let stats = profiler.query().frame(1234).aggregate();
    /// # }
    /// ```
    pub fn frame(mut self, frame: usize) -> Self {
        self.frame_range = Some(frame..frame + 1);
        self
    }

    /// Filter to a range of frames.
    ///
    /// # Arguments
    ///
    /// * `range` - Range of frame numbers to query
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use agent_game_engine_profiling::{Profiler, ProfilerConfig};
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// # let profiler = Profiler::new(ProfilerConfig::default());
    /// let stats = profiler.query().frames(1000..2000).aggregate();
    /// # }
    /// ```
    pub fn frames(mut self, range: Range<usize>) -> Self {
        self.frame_range = Some(range);
        self
    }

    /// Filter to a specific profiling category.
    ///
    /// # Arguments
    ///
    /// * `cat` - Category to filter by
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use agent_game_engine_profiling::{Profiler, ProfilerConfig, ProfileCategory};
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// # let profiler = Profiler::new(ProfilerConfig::default());
    /// let stats = profiler.query()
    ///     .category(ProfileCategory::Physics)
    ///     .aggregate();
    /// # }
    /// ```
    pub fn category(mut self, cat: ProfileCategory) -> Self {
        self.category_filter = Some(cat);
        self
    }

    /// Filter to a specific scope name.
    ///
    /// # Arguments
    ///
    /// * `name` - Scope name to filter by
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use agent_game_engine_profiling::{Profiler, ProfilerConfig};
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// # let profiler = Profiler::new(ProfilerConfig::default());
    /// let stats = profiler.query()
    ///     .scope("physics_step")
    ///     .aggregate();
    /// # }
    /// ```
    pub fn scope(mut self, name: impl Into<String>) -> Self {
        self.scope_filter = Some(name.into());
        self
    }

    /// Compute aggregate metrics for the filtered data.
    ///
    /// Returns statistical measures including percentiles for the matching scopes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use agent_game_engine_profiling::{Profiler, ProfilerConfig, ProfileCategory};
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// # let profiler = Profiler::new(ProfilerConfig::default());
    /// let stats = profiler.query()
    ///     .frames(1000..2000)
    ///     .category(ProfileCategory::Physics)
    ///     .aggregate();
    ///
    /// assert!(stats.call_count >= 0);
    /// assert!(stats.p95_us >= stats.p50_us);
    /// # }
    /// ```
    pub fn aggregate(self) -> AggregateMetrics {
        let timeline = self.timeline();

        if timeline.is_empty() {
            return AggregateMetrics::default();
        }

        let durations: Vec<u64> = timeline.iter().map(|e| e.duration_us).collect();

        AggregateMetrics::from_durations(durations)
    }

    /// Get timeline events for the filtered data.
    ///
    /// Returns a list of individual profiling scope events that match the filters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use agent_game_engine_profiling::{Profiler, ProfilerConfig, ProfileCategory};
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// # let profiler = Profiler::new(ProfilerConfig::default());
    /// let events = profiler.query()
    ///     .category(ProfileCategory::Rendering)
    ///     .timeline();
    ///
    /// for event in events {
    ///     println!("{}: {}us", event.name, event.duration_us);
    /// }
    /// # }
    /// ```
    pub fn timeline(self) -> Vec<TimelineEvent> {
        // Access internal profiler state
        let state = self.profiler.get_state_for_query();

        let mut events = Vec::new();

        // Iterate through frame history
        for (frame_idx, frame_data) in state.frame_metrics_history.iter().enumerate() {
            // Apply frame range filter
            if let Some(ref range) = self.frame_range {
                if !range.contains(&frame_idx) {
                    continue;
                }
            }

            // Get completed scopes for this frame
            for scope_data in &state.completed_scopes_by_frame {
                if scope_data.frame_number != frame_data.frame_number {
                    continue;
                }

                // Apply category filter
                if let Some(filter_cat) = self.category_filter {
                    if scope_data.category != filter_cat {
                        continue;
                    }
                }

                // Apply scope name filter
                if let Some(ref filter_name) = self.scope_filter {
                    if &scope_data.name != filter_name {
                        continue;
                    }
                }

                // Create timeline event
                let event = TimelineEvent {
                    name: scope_data.name.clone(),
                    category: scope_data.category,
                    frame: scope_data.frame_number,
                    start_us: scope_data.start_us,
                    duration_us: scope_data.duration_us,
                };

                events.push(event);
            }
        }

        events
    }

    /// Export filtered data as Chrome Trace format JSON.
    ///
    /// Generates a Chrome Tracing compatible JSON string for visualization
    /// in chrome://tracing or other compatible tools.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use agent_game_engine_profiling::{Profiler, ProfilerConfig};
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// # let profiler = Profiler::new(ProfilerConfig::default());
    /// let trace_json = profiler.query()
    ///     .frames(0..100)
    ///     .chrome_trace();
    ///
    /// // Save to file or send to analysis tool
    /// # }
    /// ```
    pub fn chrome_trace(self) -> String {
        let events = self.timeline();

        if events.is_empty() {
            return "[]".to_string();
        }

        let mut json = String::from("[\n");

        for (i, event) in events.iter().enumerate() {
            if i > 0 {
                json.push_str(",\n");
            }

            // Chrome Trace format:
            // {"name": "...", "cat": "...", "ph": "X", "pid": 1, "tid": 1, "ts": ..., "dur": ...}
            json.push_str(&format!(
                r#"  {{"name": "{}", "cat": "{}", "ph": "X", "pid": 1, "tid": 1, "ts": {}, "dur": {}}}"#,
                event.name,
                event.category.as_str(),
                event.start_us,
                event.duration_us
            ));
        }

        json.push_str("\n]");
        json
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProfileCategory, Profiler, ProfilerConfig};

    #[test]
    fn test_aggregate_metrics_empty() {
        let metrics = AggregateMetrics::from_durations(vec![]);
        assert_eq!(metrics.call_count, 0);
        assert_eq!(metrics.total_time_us, 0);
        assert_eq!(metrics.avg_time_us, 0.0);
    }

    #[test]
    fn test_aggregate_metrics_single_value() {
        let metrics = AggregateMetrics::from_durations(vec![1000]);
        assert_eq!(metrics.call_count, 1);
        assert_eq!(metrics.total_time_us, 1000);
        assert_eq!(metrics.avg_time_us, 1000.0);
        assert_eq!(metrics.p50_us, 1000);
        assert_eq!(metrics.p95_us, 1000);
        assert_eq!(metrics.p99_us, 1000);
        assert_eq!(metrics.min_us, 1000);
        assert_eq!(metrics.max_us, 1000);
    }

    #[test]
    fn test_aggregate_metrics_multiple_values() {
        let durations = vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
        let metrics = AggregateMetrics::from_durations(durations.clone());

        assert_eq!(metrics.call_count, 10);
        assert_eq!(metrics.total_time_us, 5500);
        assert_eq!(metrics.avg_time_us, 550.0);
        assert_eq!(metrics.min_us, 100);
        assert_eq!(metrics.max_us, 1000);

        // Verify percentiles (with linear interpolation)
        // p50 at index 4.5 -> interpolate between 500 and 600 = 550
        assert_eq!(metrics.p50_us, 550);
        // p95 at index 8.55 -> interpolate between 900 and 1000 = 955
        assert_eq!(metrics.p95_us, 955);
        // p99 at index 8.91 -> interpolate between 900 and 1000 = 991
        assert_eq!(metrics.p99_us, 991);
    }

    #[test]
    fn test_percentile_calculation() {
        let durations = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        assert_eq!(AggregateMetrics::calculate_percentile(&durations, 0.0), 1);
        // p50 at index 4.5 -> interpolate between 5 and 6 = 5.5, rounded to 5
        assert_eq!(AggregateMetrics::calculate_percentile(&durations, 0.5), 5);
        assert_eq!(AggregateMetrics::calculate_percentile(&durations, 1.0), 10);
    }

    #[test]
    fn test_percentile_calculation_unsorted() {
        let mut durations = vec![5, 2, 8, 1, 9, 3, 7, 4, 6, 10];
        durations.sort_unstable();

        // After sorting: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        // p50 at index 4.5 -> interpolate between 5 and 6 = 5.5, rounded to 5
        assert_eq!(AggregateMetrics::calculate_percentile(&durations, 0.5), 5);
        // p95 at index 8.55 -> interpolate between 9 and 10 = 9.55, rounded to 9
        assert_eq!(AggregateMetrics::calculate_percentile(&durations, 0.95), 9);
    }

    #[test]
    fn test_query_builder_frame_filter() {
        let profiler = Profiler::new(ProfilerConfig::default());
        let query = profiler.query().frame(100);

        assert!(query.frame_range.is_some());
        assert_eq!(query.frame_range.unwrap(), 100..101);
    }

    #[test]
    fn test_query_builder_frames_filter() {
        let profiler = Profiler::new(ProfilerConfig::default());
        let query = profiler.query().frames(1000..2000);

        assert!(query.frame_range.is_some());
        assert_eq!(query.frame_range.unwrap(), 1000..2000);
    }

    #[test]
    fn test_query_builder_category_filter() {
        let profiler = Profiler::new(ProfilerConfig::default());
        let query = profiler.query().category(ProfileCategory::Physics);

        assert!(query.category_filter.is_some());
        assert_eq!(query.category_filter.unwrap(), ProfileCategory::Physics);
    }

    #[test]
    fn test_query_builder_scope_filter() {
        let profiler = Profiler::new(ProfilerConfig::default());
        let query = profiler.query().scope("physics_step");

        assert!(query.scope_filter.is_some());
        assert_eq!(query.scope_filter.unwrap(), "physics_step");
    }

    #[test]
    fn test_query_builder_chaining() {
        let profiler = Profiler::new(ProfilerConfig::default());
        let query = profiler
            .query()
            .frames(1000..2000)
            .category(ProfileCategory::Physics)
            .scope("physics_step");

        assert!(query.frame_range.is_some());
        assert!(query.category_filter.is_some());
        assert!(query.scope_filter.is_some());
    }

    #[test]
    fn test_aggregate_empty_query() {
        let profiler = Profiler::new(ProfilerConfig::default());

        let stats = profiler.query().aggregate();

        assert_eq!(stats.call_count, 0);
        assert_eq!(stats.total_time_us, 0);
    }

    #[test]
    fn test_timeline_empty_query() {
        let profiler = Profiler::new(ProfilerConfig::default());

        let timeline = profiler.query().timeline();

        assert!(timeline.is_empty());
    }

    #[test]
    fn test_chrome_trace_empty() {
        let profiler = Profiler::new(ProfilerConfig::default());

        let trace = profiler.query().chrome_trace();

        assert_eq!(trace, "[]");
    }

    #[test]
    fn test_chrome_trace_format() {
        // Test that Chrome Trace format is valid JSON structure
        let profiler = Profiler::new(ProfilerConfig::default());
        let trace = profiler.query().chrome_trace();

        // Should be valid JSON array
        assert!(trace.starts_with('['));
        assert!(trace.ends_with(']'));
    }
}
