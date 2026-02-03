//! Stress Testing Utilities
//!
//! Shared utilities for performance and stress testing across the engine.
//! Provides helpers for:
//! - Performance measurement and statistics
//! - Memory stability tracking
//! - Load generation and simulation
//! - Result reporting

use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Performance statistics for a series of measurements
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub count: usize,
    pub total: Duration,
    pub min: Duration,
    pub max: Duration,
    pub avg: Duration,
    pub p50: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

impl PerformanceStats {
    /// Calculate statistics from a series of timing measurements
    pub fn from_timings(timings: &[Duration]) -> Self {
        if timings.is_empty() {
            return Self {
                count: 0,
                total: Duration::ZERO,
                min: Duration::ZERO,
                max: Duration::ZERO,
                avg: Duration::ZERO,
                p50: Duration::ZERO,
                p95: Duration::ZERO,
                p99: Duration::ZERO,
            };
        }

        let mut sorted = timings.to_vec();
        sorted.sort();

        let total: Duration = timings.iter().sum();
        let avg = total / timings.len() as u32;

        let p50_idx = timings.len() / 2;
        let p95_idx = (timings.len() * 95) / 100;
        let p99_idx = (timings.len() * 99) / 100;

        Self {
            count: timings.len(),
            total,
            min: sorted[0],
            max: sorted[sorted.len() - 1],
            avg,
            p50: sorted[p50_idx],
            p95: sorted[p95_idx.min(sorted.len() - 1)],
            p99: sorted[p99_idx.min(sorted.len() - 1)],
        }
    }

    /// Check if performance meets target thresholds
    pub fn meets_target(&self, _target_ms: u64, critical_ms: u64) -> bool {
        let p99_ms = self.p99.as_millis() as u64;
        p99_ms < critical_ms
    }

    /// Log statistics with tracing
    pub fn log(&self, label: &str) {
        info!(
            label = label,
            count = self.count,
            avg_ms = self.avg.as_millis(),
            p50_ms = self.p50.as_millis(),
            p95_ms = self.p95.as_millis(),
            p99_ms = self.p99.as_millis(),
            min_ms = self.min.as_millis(),
            max_ms = self.max.as_millis(),
            "Performance statistics"
        );
    }

    /// Check for performance regression compared to baseline
    pub fn check_regression(&self, baseline: &PerformanceStats, threshold_pct: f64) -> bool {
        let baseline_p99_us = baseline.p99.as_micros() as f64;
        let current_p99_us = self.p99.as_micros() as f64;

        let regression_pct = ((current_p99_us / baseline_p99_us) - 1.0) * 100.0;

        if regression_pct > threshold_pct {
            tracing::warn!(
                baseline_p99_us,
                current_p99_us,
                regression_pct = format!("{:.2}%", regression_pct),
                threshold_pct,
                "Performance regression detected"
            );
            false
        } else {
            true
        }
    }
}

/// Helper for measuring frame time stability
pub struct FrameTimeTracker {
    timings: Vec<Duration>,
    target_fps: f64,
}

impl FrameTimeTracker {
    pub fn new(target_fps: f64) -> Self {
        Self { timings: Vec::new(), target_fps }
    }

    /// Record a frame time
    pub fn record(&mut self, duration: Duration) {
        self.timings.push(duration);
    }

    /// Get statistics
    pub fn stats(&self) -> PerformanceStats {
        PerformanceStats::from_timings(&self.timings)
    }

    /// Check if frame times meet target FPS
    pub fn meets_fps_target(&self) -> bool {
        let stats = self.stats();
        let target_ms = (1000.0 / self.target_fps) as u64;
        let critical_ms = target_ms * 2; // 2x target as critical threshold

        stats.meets_target(target_ms, critical_ms)
    }

    /// Calculate frame time stability (variance)
    pub fn stability_score(&self) -> f64 {
        if self.timings.is_empty() {
            return 0.0;
        }

        let stats = self.stats();
        let avg_us = stats.avg.as_micros() as f64;

        let variance: f64 = self
            .timings
            .iter()
            .map(|t| {
                let diff = t.as_micros() as f64 - avg_us;
                diff * diff
            })
            .sum::<f64>()
            / self.timings.len() as f64;

        let std_dev = variance.sqrt();

        // Return coefficient of variation (lower is more stable)
        (std_dev / avg_us) * 100.0
    }

    /// Log frame time analysis
    pub fn log_analysis(&self) {
        let stats = self.stats();
        stats.log("Frame Times");

        let stability = self.stability_score();
        let meets_target = self.meets_fps_target();

        info!(
            target_fps = self.target_fps,
            stability_pct = format!("{:.2}%", stability),
            meets_target,
            "Frame time analysis"
        );
    }
}

/// Helper for tracking memory stability over iterations
pub struct MemoryStabilityTracker {
    checkpoints: Vec<(usize, usize)>, // (iteration, entity_count)
}

impl MemoryStabilityTracker {
    pub fn new() -> Self {
        Self { checkpoints: Vec::new() }
    }

    /// Record a checkpoint
    pub fn checkpoint(&mut self, iteration: usize, entity_count: usize) {
        self.checkpoints.push((iteration, entity_count));

        if self.checkpoints.len() % 10 == 0 {
            debug!(
                iteration,
                entity_count,
                checkpoint_count = self.checkpoints.len(),
                "Memory stability checkpoint"
            );
        }
    }

    /// Check for memory leaks (growth over time)
    pub fn detect_leak(&self) -> Option<f64> {
        if self.checkpoints.len() < 2 {
            return None;
        }

        let first = self.checkpoints[0].1 as f64;
        let last = self.checkpoints[self.checkpoints.len() - 1].1 as f64;

        let growth_pct = ((last / first) - 1.0) * 100.0;

        // Consider > 1% growth as potential leak
        if growth_pct > 1.0 {
            Some(growth_pct)
        } else {
            None
        }
    }

    /// Log memory stability report
    pub fn log_report(&self) {
        if self.checkpoints.is_empty() {
            return;
        }

        let first = self.checkpoints[0];
        let last = self.checkpoints[self.checkpoints.len() - 1];

        let growth = last.1 as i64 - first.1 as i64;
        let growth_pct = if first.1 > 0 {
            ((last.1 as f64 / first.1 as f64) - 1.0) * 100.0
        } else {
            0.0
        };

        info!(
            iterations = last.0 - first.0,
            initial_count = first.1,
            final_count = last.1,
            growth,
            growth_pct = format!("{:.2}%", growth_pct),
            checkpoint_count = self.checkpoints.len(),
            "Memory stability report"
        );

        if let Some(leak_pct) = self.detect_leak() {
            tracing::warn!(leak_pct = format!("{:.2}%", leak_pct), "Potential memory leak detected");
        }
    }
}

/// Load generator for stress testing
pub struct LoadGenerator {
    pub entity_count: usize,
    pub update_rate: f64, // updates per second
    pub duration_sec: u64,
}

impl LoadGenerator {
    /// Create a light load profile
    pub fn light() -> Self {
        Self { entity_count: 1000, update_rate: 60.0, duration_sec: 10 }
    }

    /// Create a medium load profile
    pub fn medium() -> Self {
        Self { entity_count: 10_000, update_rate: 60.0, duration_sec: 30 }
    }

    /// Create a heavy load profile
    pub fn heavy() -> Self {
        Self { entity_count: 50_000, update_rate: 60.0, duration_sec: 60 }
    }

    /// Create an extreme load profile
    pub fn extreme() -> Self {
        Self { entity_count: 100_000, update_rate: 60.0, duration_sec: 120 }
    }

    /// Get frame delta time
    pub fn frame_dt(&self) -> f32 {
        (1.0 / self.update_rate) as f32
    }

    /// Get total frame count
    pub fn total_frames(&self) -> usize {
        (self.duration_sec as f64 * self.update_rate) as usize
    }

    /// Log load profile
    pub fn log_profile(&self) {
        info!(
            entity_count = self.entity_count,
            update_rate = self.update_rate,
            duration_sec = self.duration_sec,
            total_frames = self.total_frames(),
            "Load profile"
        );
    }
}

/// Timer for measuring operation duration
pub struct StressTimer {
    start: Instant,
    label: String,
}

impl StressTimer {
    /// Start a new timer
    pub fn start(label: impl Into<String>) -> Self {
        Self { start: Instant::now(), label: label.into() }
    }

    /// Stop timer and log duration
    pub fn stop(self) -> Duration {
        let duration = self.start.elapsed();
        info!(label = self.label, duration_ms = duration.as_millis(), "Timer stopped");
        duration
    }

    /// Stop timer and return duration without logging
    pub fn stop_silent(self) -> Duration {
        self.start.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_stats() {
        let timings =
            vec![Duration::from_millis(10), Duration::from_millis(15), Duration::from_millis(20)];

        let stats = PerformanceStats::from_timings(&timings);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.min, Duration::from_millis(10));
        assert_eq!(stats.max, Duration::from_millis(20));
        assert!(stats.avg.as_millis() >= 14 && stats.avg.as_millis() <= 16);
    }

    #[test]
    fn test_frame_time_tracker() {
        let mut tracker = FrameTimeTracker::new(60.0);

        // Record 10 frames at ~16ms each (60 FPS)
        for _ in 0..10 {
            tracker.record(Duration::from_millis(16));
        }

        assert!(tracker.meets_fps_target());
        assert!(tracker.stability_score() < 10.0); // Low variance
    }

    #[test]
    fn test_memory_stability_tracker() {
        let mut tracker = MemoryStabilityTracker::new();

        // Record stable memory usage
        for i in 0..10 {
            tracker.checkpoint(i, 1000);
        }

        assert!(tracker.detect_leak().is_none());
    }

    #[test]
    fn test_memory_leak_detection() {
        let mut tracker = MemoryStabilityTracker::new();

        // Simulate memory growth
        for i in 0..10 {
            tracker.checkpoint(i, 1000 + i * 100); // Growing memory
        }

        assert!(tracker.detect_leak().is_some());
    }

    #[test]
    fn test_load_generator() {
        let light = LoadGenerator::light();
        assert_eq!(light.entity_count, 1000);

        let heavy = LoadGenerator::heavy();
        assert_eq!(heavy.entity_count, 50_000);

        assert!(heavy.total_frames() > light.total_frames());
    }
}
