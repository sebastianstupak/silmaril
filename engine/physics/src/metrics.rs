//! Enhanced profiling metrics for physics pipeline stages
//!
//! Tracks detailed performance metrics for Rapier pipeline stages:
//! - Broadphase timing (spatial partitioning)
//! - Narrowphase timing (collision detection)
//! - Solver timing (constraint resolution)
//! - Island count and body distribution
//! - Collision pair counts
//! - Solver iteration counts
//!
//! # Phase A.2 Implementation
//!
//! This module implements Phase A.2.1-A.2.5 from PHASE_ADVANCED_PHYSICS.md:
//! - A.2.1: Rapier pipeline stage timing
//! - A.2.2: Island count & body-per-island tracking
//! - A.2.3: Collision pair counting
//! - A.2.4: Solver iteration & residual tracking
//! - A.2.5: Export to FrameMetrics struct

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Frame statistics to pass to MetricsCollector::end_frame
#[derive(Debug, Clone, Copy)]
pub struct FrameStats {
    /// Number of active (non-sleeping) rigid bodies
    pub active_bodies: usize,
    /// Number of sleeping rigid bodies
    pub sleeping_bodies: usize,
    /// Number of simulation islands
    pub island_count: usize,
    /// Average number of bodies per island
    pub avg_bodies_per_island: f32,
    /// Maximum number of bodies in any single island
    pub max_bodies_in_island: usize,
    /// Number of collision pairs detected
    pub collision_pairs: usize,
    /// Number of active contact points
    pub active_contacts: usize,
    /// Number of solver iterations performed
    pub solver_iterations: usize,
    /// Number of constraints (joints + contacts)
    pub constraints: usize,
    /// Total number of colliders in the world
    pub colliders: usize,
    /// Total number of joints in the world
    pub joints: usize,
}

/// Frame-level physics metrics
///
/// Captures performance and simulation statistics for a single physics step.
/// Designed to be lightweight (<100 bytes) and zero-cost when profiling is disabled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrameMetrics {
    /// Frame number
    pub frame: u64,

    /// Total frame time (microseconds)
    pub frame_time_us: u64,

    /// === Pipeline Stage Timing (A.2.1) ===
    /// Broadphase timing (microseconds) - spatial partitioning
    pub broadphase_time_us: u64,

    /// Narrowphase timing (microseconds) - collision detection
    pub narrowphase_time_us: u64,

    /// Solver timing (microseconds) - constraint resolution
    pub solver_time_us: u64,

    /// Island building timing (microseconds)
    pub island_build_time_us: u64,

    /// CCD (continuous collision detection) timing (microseconds)
    pub ccd_time_us: u64,

    /// === Simulation Statistics (A.2.2, A.2.3) ===
    /// Number of active islands
    pub island_count: usize,

    /// Number of bodies per island (average)
    pub avg_bodies_per_island: f32,

    /// Maximum bodies in any single island
    pub max_bodies_in_island: usize,

    /// Total number of active rigid bodies
    pub active_body_count: usize,

    /// Number of sleeping bodies
    pub sleeping_body_count: usize,

    /// Number of collision pairs processed
    pub collision_pair_count: usize,

    /// Number of active contacts (touching)
    pub active_contact_count: usize,

    /// === Solver Metrics (A.2.4) ===
    /// Number of solver iterations performed
    pub solver_iterations: usize,

    /// Solver residual (final constraint error) - lower is better
    /// Note: Not exposed by Rapier 0.18 public API, will be 0.0
    pub solver_residual: f32,

    /// Number of constraints solved
    pub constraint_count: usize,

    /// === Memory Statistics ===
    /// Total colliders in simulation
    pub total_collider_count: usize,

    /// Total joints/constraints in simulation
    pub total_joint_count: usize,
}

impl FrameMetrics {
    /// Create empty metrics for a frame
    pub fn new(frame: u64) -> Self {
        Self {
            frame,
            frame_time_us: 0,
            broadphase_time_us: 0,
            narrowphase_time_us: 0,
            solver_time_us: 0,
            island_build_time_us: 0,
            ccd_time_us: 0,
            island_count: 0,
            avg_bodies_per_island: 0.0,
            max_bodies_in_island: 0,
            active_body_count: 0,
            sleeping_body_count: 0,
            collision_pair_count: 0,
            active_contact_count: 0,
            solver_iterations: 0,
            solver_residual: 0.0,
            constraint_count: 0,
            total_collider_count: 0,
            total_joint_count: 0,
        }
    }

    /// Calculate overhead percentage for a specific stage
    pub fn stage_overhead_percent(&self, stage_time_us: u64) -> f32 {
        if self.frame_time_us == 0 {
            return 0.0;
        }
        (stage_time_us as f32 / self.frame_time_us as f32) * 100.0
    }

    /// Get broadphase overhead as percentage of frame time
    pub fn broadphase_overhead_percent(&self) -> f32 {
        self.stage_overhead_percent(self.broadphase_time_us)
    }

    /// Get narrowphase overhead as percentage of frame time
    pub fn narrowphase_overhead_percent(&self) -> f32 {
        self.stage_overhead_percent(self.narrowphase_time_us)
    }

    /// Get solver overhead as percentage of frame time
    pub fn solver_overhead_percent(&self) -> f32 {
        self.stage_overhead_percent(self.solver_time_us)
    }

    /// Check if metrics indicate performance issues
    pub fn has_performance_warning(&self) -> bool {
        // Warn if frame time exceeds 16ms (60 FPS target)
        self.frame_time_us > 16_000
    }

    /// Get human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Frame {}: {:.2}ms total | Bodies: {} active, {} sleeping | Islands: {} | Pairs: {} | Contacts: {}",
            self.frame,
            self.frame_time_us as f32 / 1000.0,
            self.active_body_count,
            self.sleeping_body_count,
            self.island_count,
            self.collision_pair_count,
            self.active_contact_count
        )
    }
}

/// Metrics collector for tracking physics performance
///
/// Lightweight wrapper around timers for measuring Rapier pipeline stages.
/// Zero overhead when profiling is disabled.
pub struct MetricsCollector {
    /// Current frame number
    frame: u64,

    /// Frame start time
    frame_start: Option<Instant>,

    /// Stage timings
    broadphase_time: Duration,
    narrowphase_time: Duration,
    solver_time: Duration,
    island_build_time: Duration,
    ccd_time: Duration,

    /// Whether metrics collection is enabled
    enabled: bool,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            frame: 0,
            frame_start: None,
            broadphase_time: Duration::ZERO,
            narrowphase_time: Duration::ZERO,
            solver_time: Duration::ZERO,
            island_build_time: Duration::ZERO,
            ccd_time: Duration::ZERO,
            enabled: false,
        }
    }

    /// Enable metrics collection
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable metrics collection
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Start a new frame
    pub fn begin_frame(&mut self, frame: u64) {
        if !self.enabled {
            return;
        }
        self.frame = frame;
        self.frame_start = Some(Instant::now());
        self.broadphase_time = Duration::ZERO;
        self.narrowphase_time = Duration::ZERO;
        self.solver_time = Duration::ZERO;
        self.island_build_time = Duration::ZERO;
        self.ccd_time = Duration::ZERO;
    }

    /// Record broadphase timing
    pub fn record_broadphase(&mut self, duration: Duration) {
        if self.enabled {
            self.broadphase_time = duration;
        }
    }

    /// Record narrowphase timing
    pub fn record_narrowphase(&mut self, duration: Duration) {
        if self.enabled {
            self.narrowphase_time = duration;
        }
    }

    /// Record solver timing
    pub fn record_solver(&mut self, duration: Duration) {
        if self.enabled {
            self.solver_time = duration;
        }
    }

    /// Record island build timing
    pub fn record_island_build(&mut self, duration: Duration) {
        if self.enabled {
            self.island_build_time = duration;
        }
    }

    /// Record CCD timing
    pub fn record_ccd(&mut self, duration: Duration) {
        if self.enabled {
            self.ccd_time = duration;
        }
    }

    /// Finish frame and generate metrics
    pub fn end_frame(&mut self, stats: FrameStats) -> FrameMetrics {
        let frame_time = if let Some(start) = self.frame_start {
            start.elapsed()
        } else {
            Duration::ZERO
        };

        FrameMetrics {
            frame: self.frame,
            frame_time_us: frame_time.as_micros() as u64,
            broadphase_time_us: self.broadphase_time.as_micros() as u64,
            narrowphase_time_us: self.narrowphase_time.as_micros() as u64,
            solver_time_us: self.solver_time.as_micros() as u64,
            island_build_time_us: self.island_build_time.as_micros() as u64,
            ccd_time_us: self.ccd_time.as_micros() as u64,
            island_count: stats.island_count,
            avg_bodies_per_island: stats.avg_bodies_per_island,
            max_bodies_in_island: stats.max_bodies_in_island,
            active_body_count: stats.active_bodies,
            sleeping_body_count: stats.sleeping_bodies,
            collision_pair_count: stats.collision_pairs,
            active_contact_count: stats.active_contacts,
            solver_iterations: stats.solver_iterations,
            solver_residual: 0.0, // Not exposed by Rapier 0.18
            constraint_count: stats.constraints,
            total_collider_count: stats.colliders,
            total_joint_count: stats.joints,
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_metrics_creation() {
        let metrics = FrameMetrics::new(42);
        assert_eq!(metrics.frame, 42);
        assert_eq!(metrics.frame_time_us, 0);
        assert_eq!(metrics.island_count, 0);
    }

    #[test]
    fn test_overhead_calculation() {
        let mut metrics = FrameMetrics::new(0);
        metrics.frame_time_us = 10_000; // 10ms
        metrics.broadphase_time_us = 1_000; // 1ms
        metrics.narrowphase_time_us = 3_000; // 3ms
        metrics.solver_time_us = 5_000; // 5ms

        assert!((metrics.broadphase_overhead_percent() - 10.0).abs() < 0.01);
        assert!((metrics.narrowphase_overhead_percent() - 30.0).abs() < 0.01);
        assert!((metrics.solver_overhead_percent() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_performance_warning() {
        let mut metrics = FrameMetrics::new(0);
        metrics.frame_time_us = 15_000; // 15ms - under 16ms
        assert!(!metrics.has_performance_warning());

        metrics.frame_time_us = 17_000; // 17ms - over 16ms
        assert!(metrics.has_performance_warning());
    }

    #[test]
    fn test_metrics_collector_disabled() {
        let mut collector = MetricsCollector::new();
        assert!(!collector.is_enabled());

        collector.begin_frame(1);
        collector.record_broadphase(Duration::from_millis(5));
        let stats = FrameStats {
            active_bodies: 10,
            sleeping_bodies: 5,
            island_count: 2,
            avg_bodies_per_island: 7.5,
            max_bodies_in_island: 10,
            collision_pairs: 20,
            active_contacts: 15,
            solver_iterations: 4,
            constraints: 5,
            colliders: 30,
            joints: 8,
        };
        let metrics = collector.end_frame(stats);

        // Should have zero timings when disabled
        assert_eq!(metrics.broadphase_time_us, 0);
    }

    #[test]
    fn test_metrics_collector_enabled() {
        let mut collector = MetricsCollector::new();
        collector.enable();
        assert!(collector.is_enabled());

        collector.begin_frame(1);
        collector.record_broadphase(Duration::from_micros(100));
        collector.record_narrowphase(Duration::from_micros(200));
        collector.record_solver(Duration::from_micros(300));

        let stats = FrameStats {
            active_bodies: 10,
            sleeping_bodies: 5,
            island_count: 2,
            avg_bodies_per_island: 7.5,
            max_bodies_in_island: 10,
            collision_pairs: 20,
            active_contacts: 15,
            solver_iterations: 4,
            constraints: 5,
            colliders: 30,
            joints: 8,
        };
        let metrics = collector.end_frame(stats);

        assert_eq!(metrics.frame, 1);
        assert_eq!(metrics.broadphase_time_us, 100);
        assert_eq!(metrics.narrowphase_time_us, 200);
        assert_eq!(metrics.solver_time_us, 300);
        assert_eq!(metrics.active_body_count, 10);
        assert_eq!(metrics.sleeping_body_count, 5);
        assert_eq!(metrics.island_count, 2);
        assert_eq!(metrics.avg_bodies_per_island, 7.5);
        assert_eq!(metrics.max_bodies_in_island, 10);
        assert_eq!(metrics.collision_pair_count, 20);
        assert_eq!(metrics.active_contact_count, 15);
        assert_eq!(metrics.solver_iterations, 4);
        assert_eq!(metrics.constraint_count, 5);
        assert_eq!(metrics.total_collider_count, 30);
        assert_eq!(metrics.total_joint_count, 8);
    }

    #[test]
    fn test_summary_format() {
        let mut metrics = FrameMetrics::new(42);
        metrics.frame_time_us = 8_500; // 8.5ms
        metrics.active_body_count = 100;
        metrics.sleeping_body_count = 50;
        metrics.island_count = 5;
        metrics.collision_pair_count = 250;
        metrics.active_contact_count = 180;

        let summary = metrics.summary();
        assert!(summary.contains("Frame 42"));
        assert!(summary.contains("8.50ms"));
        assert!(summary.contains("100 active"));
        assert!(summary.contains("50 sleeping"));
        assert!(summary.contains("Islands: 5"));
        assert!(summary.contains("Pairs: 250"));
        assert!(summary.contains("Contacts: 180"));
    }

    #[test]
    fn test_metrics_serialization() {
        let metrics = FrameMetrics {
            frame: 100,
            frame_time_us: 12_000,
            broadphase_time_us: 1_000,
            narrowphase_time_us: 3_000,
            solver_time_us: 7_000,
            island_build_time_us: 500,
            ccd_time_us: 500,
            island_count: 3,
            avg_bodies_per_island: 10.5,
            max_bodies_in_island: 15,
            active_body_count: 30,
            sleeping_body_count: 20,
            collision_pair_count: 75,
            active_contact_count: 45,
            solver_iterations: 4,
            solver_residual: 0.001,
            constraint_count: 10,
            total_collider_count: 50,
            total_joint_count: 12,
        };

        // Test JSON serialization
        let json = serde_json::to_string(&metrics).unwrap();
        let deserialized: FrameMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(metrics, deserialized);
    }
}
