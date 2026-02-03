//! Performance metrics for frame capture.
//!
//! Tracks timing and throughput of capture operations.

use std::time::{Duration, Instant};
use tracing::debug;

/// Capture performance metrics
#[derive(Debug, Clone)]
pub struct CaptureMetrics {
    /// Time spent copying from GPU to CPU (ms)
    pub copy_time_ms: f32,
    /// Time spent encoding image (ms)
    pub encode_time_ms: f32,
    /// Time spent saving to disk (ms)
    pub save_time_ms: f32,
    /// Total time per capture (ms)
    pub total_time_ms: f32,
    /// Number of frames captured
    pub frames_captured: u64,
}

impl Default for CaptureMetrics {
    fn default() -> Self {
        Self {
            copy_time_ms: 0.0,
            encode_time_ms: 0.0,
            save_time_ms: 0.0,
            total_time_ms: 0.0,
            frames_captured: 0,
        }
    }
}

impl CaptureMetrics {
    /// Check if capture meets performance targets
    ///
    /// Targets:
    /// - Copy: < 2ms
    /// - Encode: < 3ms
    /// - Total: < 5ms
    pub fn meets_targets(&self) -> bool {
        self.copy_time_ms < 2.0 && self.encode_time_ms < 3.0 && self.total_time_ms < 5.0
    }

    /// Get performance summary string
    pub fn summary(&self) -> String {
        format!(
            "Capture metrics: {:.2}ms total ({:.2}ms copy, {:.2}ms encode, {:.2}ms save) - {} frames",
            self.total_time_ms,
            self.copy_time_ms,
            self.encode_time_ms,
            self.save_time_ms,
            self.frames_captured
        )
    }
}

/// Metrics tracker for capture operations
pub struct MetricsTracker {
    metrics: CaptureMetrics,
    last_capture_start: Option<Instant>,
}

impl MetricsTracker {
    /// Create new metrics tracker
    pub fn new() -> Self {
        Self { metrics: CaptureMetrics::default(), last_capture_start: None }
    }

    /// Start capture timing
    pub fn start_capture(&mut self) {
        self.last_capture_start = Some(Instant::now());
    }

    /// End capture timing and update metrics
    pub fn end_capture(&mut self, copy_time: Duration, encode_time: Duration, save_time: Duration) {
        if let Some(start) = self.last_capture_start.take() {
            let total = start.elapsed();

            self.metrics.copy_time_ms = copy_time.as_secs_f32() * 1000.0;
            self.metrics.encode_time_ms = encode_time.as_secs_f32() * 1000.0;
            self.metrics.save_time_ms = save_time.as_secs_f32() * 1000.0;
            self.metrics.total_time_ms = total.as_secs_f32() * 1000.0;
            self.metrics.frames_captured += 1;

            debug!(
                copy_ms = self.metrics.copy_time_ms,
                encode_ms = self.metrics.encode_time_ms,
                save_ms = self.metrics.save_time_ms,
                total_ms = self.metrics.total_time_ms,
                "Frame capture metrics"
            );

            // Warn if exceeding performance targets
            if !self.metrics.meets_targets() {
                tracing::warn!("Capture performance below target: {}", self.metrics.summary());
            }
        }
    }

    /// Get current metrics
    pub fn metrics(&self) -> &CaptureMetrics {
        &self.metrics
    }

    /// Reset metrics
    pub fn reset(&mut self) {
        self.metrics = CaptureMetrics::default();
    }
}

impl Default for MetricsTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_default() {
        let metrics = CaptureMetrics::default();
        assert_eq!(metrics.frames_captured, 0);
        assert_eq!(metrics.total_time_ms, 0.0);
    }

    #[test]
    fn test_metrics_targets() {
        let mut metrics = CaptureMetrics::default();
        metrics.copy_time_ms = 1.5;
        metrics.encode_time_ms = 2.5;
        metrics.total_time_ms = 4.5;
        assert!(metrics.meets_targets());

        metrics.total_time_ms = 6.0;
        assert!(!metrics.meets_targets());
    }

    #[test]
    fn test_tracker_timing() {
        let mut tracker = MetricsTracker::new();
        tracker.start_capture();
        std::thread::sleep(Duration::from_millis(10));
        tracker.end_capture(
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::from_millis(1),
        );

        assert_eq!(tracker.metrics().frames_captured, 1);
        assert!(tracker.metrics().total_time_ms >= 10.0);
    }
}
