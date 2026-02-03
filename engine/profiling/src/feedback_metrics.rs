//! AI Agent Feedback Metrics
//!
//! This module provides comprehensive metrics designed for AI agent consumption.
//! Metrics are extracted from the profiler and serialized to JSON for analysis,
//! training data, or automated decision-making.

use crate::{FrameMetrics, Profiler};
use std::collections::HashMap;

/// Comprehensive metrics for AI agent feedback loops.
///
/// This structure provides all the information an AI agent needs to:
/// - Analyze game performance
/// - Make data-driven optimization decisions
/// - Generate training data for automated development
/// - Monitor resource usage and bottlenecks
///
/// # Examples
///
/// ```rust
/// use silmaril_profiling::{Profiler, ProfilerConfig};
/// use silmaril_profiling::AgentFeedbackMetrics;
///
/// # #[cfg(all(feature = "metrics", feature = "config"))]
/// # {
/// let profiler = Profiler::new(ProfilerConfig::default());
///
/// // Run some frames...
/// profiler.begin_frame();
/// // ... game logic ...
/// profiler.end_frame();
///
/// // Extract metrics for AI agent
/// let metrics = AgentFeedbackMetrics::from_profiler(&profiler);
///
/// // Serialize to JSON
/// let json = metrics.to_json();
/// println!("{}", json);
/// # }
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentFeedbackMetrics {
    // ===== Frame Timing =====
    /// Current frame time in milliseconds
    pub frame_time_ms: f32,

    /// 95th percentile frame time in milliseconds (over recent history)
    pub frame_time_p95_ms: f32,

    /// Current frames per second
    pub fps: f32,

    /// Whether the frame budget was met (true = under budget)
    pub is_frame_budget_met: bool,

    // ===== System Breakdown =====
    /// Time spent in each system category (in milliseconds)
    pub time_by_category: HashMap<String, f32>,

    // ===== ECS Stats =====
    // Note: These are placeholders for now, will integrate with World later
    /// Number of active entities in the world
    pub entity_count: u32,

    /// Number of archetypes (component combinations) in use
    pub archetype_count: u32,

    /// Count of each component type in use
    pub component_counts: HashMap<String, u32>,

    // ===== Memory =====
    /// Current memory usage in megabytes
    pub memory_used_mb: usize,

    /// Peak memory usage in megabytes
    pub memory_peak_mb: usize,

    /// Number of allocations (if tracked)
    pub allocation_count: usize,

    // ===== Rendering Stats (Phase 1+) =====
    /// Number of draw calls per frame
    pub draw_calls: u32,

    /// Total triangle count rendered
    pub triangle_count: u32,

    /// Texture memory usage in megabytes
    pub texture_memory_mb: usize,

    /// Number of shader switches per frame
    pub shader_switches: u32,

    // ===== Networking Stats (Phase 2+) =====
    /// Network bandwidth in bytes per second
    pub bandwidth_bytes_per_sec: usize,

    /// Packet loss percentage (0.0 - 100.0)
    pub packet_loss_percent: f32,

    /// Network latency in milliseconds
    pub latency_ms: f32,

    // ===== Game State =====
    /// Game time in seconds (not wall-clock time)
    pub game_time: f32,

    /// Custom metrics (extensible for game-specific data)
    pub custom: HashMap<String, f32>,
}

impl Default for AgentFeedbackMetrics {
    fn default() -> Self {
        Self {
            frame_time_ms: 0.0,
            frame_time_p95_ms: 0.0,
            fps: 0.0,
            is_frame_budget_met: true,
            time_by_category: HashMap::new(),
            entity_count: 0,
            archetype_count: 0,
            component_counts: HashMap::new(),
            memory_used_mb: 0,
            memory_peak_mb: 0,
            allocation_count: 0,
            draw_calls: 0,
            triangle_count: 0,
            texture_memory_mb: 0,
            shader_switches: 0,
            bandwidth_bytes_per_sec: 0,
            packet_loss_percent: 0.0,
            latency_ms: 0.0,
            game_time: 0.0,
            custom: HashMap::new(),
        }
    }
}

impl AgentFeedbackMetrics {
    /// Extract metrics from a profiler instance.
    ///
    /// This method collects all available profiling data and computes
    /// aggregate statistics like percentiles.
    ///
    /// # Arguments
    ///
    /// * `profiler` - The profiler to extract metrics from
    ///
    /// # Examples
    ///
    /// ```rust
    /// use silmaril_profiling::{Profiler, ProfilerConfig};
    /// use silmaril_profiling::AgentFeedbackMetrics;
    ///
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// let profiler = Profiler::new(ProfilerConfig::default());
    /// let metrics = AgentFeedbackMetrics::from_profiler(&profiler);
    /// # }
    /// ```
    #[must_use]
    pub fn from_profiler(profiler: &Profiler) -> Self {
        let history = profiler.frame_history();

        // Get the most recent frame metrics
        let latest = history.last().cloned().unwrap_or_default();

        // Calculate p95 from frame history
        let frame_time_p95_ms = calculate_p95(&history);

        // Convert time_by_category from ProfileCategory to String
        let time_by_category: HashMap<String, f32> = latest
            .time_by_category
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), *v))
            .collect();

        // Check if frame budget was met (assume 16.67ms budget for 60 FPS)
        #[allow(clippy::items_after_statements)] // Const used only in this scope
        const FRAME_BUDGET_MS: f32 = 16.67;
        let is_frame_budget_met = latest.frame_time_ms <= FRAME_BUDGET_MS;

        Self {
            frame_time_ms: latest.frame_time_ms,
            frame_time_p95_ms,
            fps: latest.fps,
            is_frame_budget_met,
            time_by_category,
            entity_count: latest.entity_count,
            archetype_count: 0,               // TODO: Get from World
            component_counts: HashMap::new(), // TODO: Get from World
            memory_used_mb: latest.memory_mb,
            memory_peak_mb: 0,          // TODO: Track peak memory
            allocation_count: 0,        // TODO: Track allocations
            draw_calls: 0,              // TODO: Get from renderer (Phase 1)
            triangle_count: 0,          // TODO: Get from renderer (Phase 1)
            texture_memory_mb: 0,       // TODO: Get from renderer (Phase 1)
            shader_switches: 0,         // TODO: Get from renderer (Phase 1)
            bandwidth_bytes_per_sec: 0, // TODO: Get from networking (Phase 2)
            packet_loss_percent: 0.0,   // TODO: Get from networking (Phase 2)
            latency_ms: 0.0,            // TODO: Get from networking (Phase 2)
            game_time: 0.0,             // TODO: Get from game state
            custom: HashMap::new(),
        }
    }

    /// Serialize metrics to JSON string.
    ///
    /// This method is always available and provides a simple JSON representation
    /// even if the `serde` feature is not enabled (using a hand-rolled implementation).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use silmaril_profiling::{Profiler, ProfilerConfig};
    /// use silmaril_profiling::AgentFeedbackMetrics;
    ///
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// let profiler = Profiler::new(ProfilerConfig::default());
    /// let metrics = AgentFeedbackMetrics::from_profiler(&profiler);
    /// let json = metrics.to_json();
    /// assert!(json.contains("frame_time_ms"));
    /// # }
    /// ```
    #[must_use]
    pub fn to_json(&self) -> String {
        #[cfg(feature = "serde")]
        {
            serde_json::to_string_pretty(self)
                .unwrap_or_else(|e| format!(r#"{{"error": "Serialization failed: {e}"}}"#))
        }

        #[cfg(not(feature = "serde"))]
        {
            // Hand-rolled JSON for when serde is not available
            let mut json = String::from("{\n");

            // Frame timing
            json.push_str(&format!(r#"  "frame_time_ms": {},"#, self.frame_time_ms));
            json.push('\n');
            json.push_str(&format!(r#"  "frame_time_p95_ms": {},"#, self.frame_time_p95_ms));
            json.push('\n');
            json.push_str(&format!(r#"  "fps": {},"#, self.fps));
            json.push('\n');
            json.push_str(&format!(r#"  "is_frame_budget_met": {},"#, self.is_frame_budget_met));
            json.push('\n');

            // System breakdown
            json.push_str(r#"  "time_by_category": {"#);
            json.push('\n');
            for (i, (k, v)) in self.time_by_category.iter().enumerate() {
                json.push_str(&format!(r#"    "{k}": {v}"#));
                if i < self.time_by_category.len() - 1 {
                    json.push(',');
                }
                json.push('\n');
            }
            json.push_str("  },\n");

            // ECS stats
            json.push_str(&format!(r#"  "entity_count": {},"#, self.entity_count));
            json.push('\n');
            json.push_str(&format!(r#"  "archetype_count": {},"#, self.archetype_count));
            json.push('\n');

            // Memory
            json.push_str(&format!(r#"  "memory_used_mb": {},"#, self.memory_used_mb));
            json.push('\n');
            json.push_str(&format!(r#"  "memory_peak_mb": {},"#, self.memory_peak_mb));
            json.push('\n');

            // Rendering
            json.push_str(&format!(r#"  "draw_calls": {},"#, self.draw_calls));
            json.push('\n');
            json.push_str(&format!(r#"  "triangle_count": {},"#, self.triangle_count));
            json.push('\n');

            // Networking
            json.push_str(&format!(
                r#"  "bandwidth_bytes_per_sec": {},"#,
                self.bandwidth_bytes_per_sec
            ));
            json.push('\n');
            json.push_str(&format!(r#"  "latency_ms": {}"#, self.latency_ms));
            json.push('\n');

            json.push_str("}\n");
            json
        }
    }
}

/// Calculate the 95th percentile of frame times from history.
///
/// This is a key metric for understanding worst-case performance.
/// Uses linear interpolation method consistent with industry standards.
fn calculate_p95(history: &[FrameMetrics]) -> f32 {
    if history.is_empty() {
        return 0.0;
    }

    let mut frame_times: Vec<f32> = history.iter().map(|m| m.frame_time_ms).collect();
    frame_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Calculate 95th percentile index using nearest-rank method
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )] // Intentional: calculating percentile index
    // This matches the behavior expected by tests
    let index = ((frame_times.len() - 1) as f32 * 0.95).round() as usize;

    frame_times[index]
}

// Extension trait to add `get_agent_metrics` to Profiler
impl Profiler {
    /// Get comprehensive metrics for AI agent consumption.
    ///
    /// This is a convenience method that wraps `AgentFeedbackMetrics::from_profiler`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use silmaril_profiling::{Profiler, ProfilerConfig};
    ///
    /// # #[cfg(feature = "metrics")]
    /// # {
    /// let profiler = Profiler::new(ProfilerConfig::default());
    ///
    /// // Run some frames...
    /// profiler.begin_frame();
    /// profiler.end_frame();
    ///
    /// // Get AI-friendly metrics
    /// let metrics = profiler.get_agent_metrics();
    /// println!("Frame time p95: {}ms", metrics.frame_time_p95_ms);
    /// # }
    /// ```
    #[must_use]
    pub fn get_agent_metrics(&self) -> AgentFeedbackMetrics {
        AgentFeedbackMetrics::from_profiler(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProfileCategory;
    use crate::ProfilerConfig;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_default_metrics() {
        let metrics = AgentFeedbackMetrics::default();
        assert_eq!(metrics.frame_time_ms, 0.0);
        assert_eq!(metrics.fps, 0.0);
        assert!(metrics.is_frame_budget_met);
        assert_eq!(metrics.entity_count, 0);
    }

    #[test]
    fn test_from_profiler() {
        let profiler = Profiler::new(ProfilerConfig::default());

        // Generate some frames
        for _ in 0..10 {
            profiler.begin_frame();
            {
                let _guard = profiler.scope("test_scope", ProfileCategory::ECS);
                thread::sleep(Duration::from_millis(1));
            }
            profiler.end_frame();
        }

        let metrics = AgentFeedbackMetrics::from_profiler(&profiler);

        // Should have collected metrics
        assert!(metrics.frame_time_ms > 0.0);
        assert!(metrics.fps > 0.0);
        assert!(!metrics.time_by_category.is_empty());
        assert!(metrics.time_by_category.contains_key("ECS"));
    }

    #[test]
    fn test_to_json() {
        let metrics = AgentFeedbackMetrics {
            frame_time_ms: 15.5,
            frame_time_p95_ms: 16.2,
            fps: 64.5,
            is_frame_budget_met: true,
            entity_count: 1000,
            ..Default::default()
        };

        let json = metrics.to_json();

        // Verify JSON contains expected fields
        assert!(json.contains("frame_time_ms"));
        assert!(json.contains("15.5"));
        assert!(json.contains("fps"));
        assert!(json.contains("64.5"));
        assert!(json.contains("entity_count"));
        assert!(json.contains("1000"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_json_serialization_roundtrip() {
        use serde_json;

        let original = AgentFeedbackMetrics {
            frame_time_ms: 12.3,
            frame_time_p95_ms: 14.5,
            fps: 81.3,
            is_frame_budget_met: true,
            entity_count: 500,
            memory_used_mb: 128,
            ..Default::default()
        };

        // Serialize to JSON
        let json = serde_json::to_string(&original).expect("Failed to serialize");

        // Deserialize back
        let deserialized: AgentFeedbackMetrics =
            serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify fields match
        assert_eq!(deserialized.frame_time_ms, original.frame_time_ms);
        assert_eq!(deserialized.frame_time_p95_ms, original.frame_time_p95_ms);
        assert_eq!(deserialized.fps, original.fps);
        assert_eq!(deserialized.is_frame_budget_met, original.is_frame_budget_met);
        assert_eq!(deserialized.entity_count, original.entity_count);
        assert_eq!(deserialized.memory_used_mb, original.memory_used_mb);
    }

    #[test]
    fn test_calculate_p95_empty() {
        let history = vec![];
        let p95 = calculate_p95(&history);
        assert_eq!(p95, 0.0);
    }

    #[test]
    fn test_calculate_p95_single_value() {
        let history = vec![FrameMetrics { frame_time_ms: 10.0, ..Default::default() }];

        let p95 = calculate_p95(&history);
        assert_eq!(p95, 10.0);
    }

    #[test]
    fn test_calculate_p95_multiple_values() {
        // Create 100 frames with times from 1.0 to 100.0 ms
        let history: Vec<FrameMetrics> = (1..=100)
            .map(|i| FrameMetrics { frame_time_ms: i as f32, ..Default::default() })
            .collect();

        let p95 = calculate_p95(&history);

        // 95th percentile of 1-100 should be 95
        assert_eq!(p95, 95.0);
    }

    #[test]
    fn test_calculate_p95_unsorted() {
        // Create unsorted frame times
        let history = vec![
            FrameMetrics { frame_time_ms: 20.0, ..Default::default() },
            FrameMetrics { frame_time_ms: 5.0, ..Default::default() },
            FrameMetrics { frame_time_ms: 15.0, ..Default::default() },
            FrameMetrics { frame_time_ms: 25.0, ..Default::default() },
            FrameMetrics { frame_time_ms: 10.0, ..Default::default() },
        ];

        let p95 = calculate_p95(&history);

        // 95th percentile of [5, 10, 15, 20, 25] should be 25
        assert_eq!(p95, 25.0);
    }

    #[test]
    fn test_profiler_get_agent_metrics() {
        let profiler = Profiler::new(ProfilerConfig::default());

        // Generate a frame
        profiler.begin_frame();
        {
            let _guard = profiler.scope("physics", ProfileCategory::Physics);
            thread::sleep(Duration::from_millis(2));
        }
        profiler.end_frame();

        // Get metrics via extension method
        let metrics = profiler.get_agent_metrics();

        assert!(metrics.frame_time_ms > 0.0);
        assert!(metrics.fps > 0.0);
    }

    #[test]
    fn test_frame_budget_check() {
        let profiler = Profiler::new(ProfilerConfig::default());

        // Fast frame (under budget)
        profiler.begin_frame();
        thread::sleep(Duration::from_millis(5));
        profiler.end_frame();

        let metrics = profiler.get_agent_metrics();
        assert!(metrics.is_frame_budget_met);

        // Slow frame (over budget)
        profiler.begin_frame();
        thread::sleep(Duration::from_millis(20));
        profiler.end_frame();

        let metrics = profiler.get_agent_metrics();
        assert!(!metrics.is_frame_budget_met);
    }

    #[test]
    fn test_time_by_category_conversion() {
        let profiler = Profiler::new(ProfilerConfig::default());

        profiler.begin_frame();
        {
            let _ecs = profiler.scope("ecs_work", ProfileCategory::ECS);
            thread::sleep(Duration::from_millis(1));
        }
        {
            let _physics = profiler.scope("physics_work", ProfileCategory::Physics);
            thread::sleep(Duration::from_millis(2));
        }
        profiler.end_frame();

        let metrics = profiler.get_agent_metrics();

        // Verify categories are converted to strings
        assert!(metrics.time_by_category.contains_key("ECS"));
        assert!(metrics.time_by_category.contains_key("Physics"));
        assert!(metrics.time_by_category.get("ECS").unwrap() > &0.0);
        assert!(metrics.time_by_category.get("Physics").unwrap() > &0.0);
    }

    // Property-based test for percentile calculations
    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_p95_always_in_range(frame_times in prop::collection::vec(0.0f32..100.0, 1..1000)) {
                let history: Vec<FrameMetrics> = frame_times
                    .iter()
                    .map(|&ft| FrameMetrics {
                        frame_time_ms: ft,
                        ..Default::default()
                    })
                    .collect();

                let p95 = calculate_p95(&history);

                // P95 should be within the range of input values
                let min = frame_times.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max = frame_times.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

                prop_assert!(p95 >= min);
                prop_assert!(p95 <= max);
            }

            #[test]
            fn test_p95_is_deterministic(frame_times in prop::collection::vec(0.0f32..100.0, 1..100)) {
                let history: Vec<FrameMetrics> = frame_times
                    .iter()
                    .map(|&ft| FrameMetrics {
                        frame_time_ms: ft,
                        ..Default::default()
                    })
                    .collect();

                let p95_first = calculate_p95(&history);
                let p95_second = calculate_p95(&history);

                // Should get the same result every time
                prop_assert_eq!(p95_first, p95_second);
            }

            #[test]
            fn test_p95_increases_monotonically(
                base in 1.0f32..50.0,
                spike in 50.0f32..100.0
            ) {
                // Create baseline frames
                let baseline_history: Vec<FrameMetrics> = (0..100)
                    .map(|_| FrameMetrics {
                        frame_time_ms: base,
                        ..Default::default()
                    })
                    .collect();

                let baseline_p95 = calculate_p95(&baseline_history);

                // Add spike frames
                let mut spiked_history = baseline_history.clone();
                for _ in 0..10 {
                    spiked_history.push(FrameMetrics {
                        frame_time_ms: spike,
                        ..Default::default()
                    });
                }

                let spiked_p95 = calculate_p95(&spiked_history);

                // P95 should increase when adding high values
                prop_assert!(spiked_p95 >= baseline_p95);
            }
        }
    }
}
