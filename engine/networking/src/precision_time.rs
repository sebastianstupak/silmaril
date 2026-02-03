//! High-Precision Timing for Esports
//!
//! Provides sub-millisecond precision timing, tick-perfect synchronization,
//! and deterministic simulation for competitive multiplayer games.

use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Nanosecond-precision timestamp (u64 nanoseconds since epoch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrecisionTimestamp(pub u64);

impl PrecisionTimestamp {
    /// Create timestamp from nanoseconds
    pub fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// Create timestamp from current time
    pub fn now() -> Self {
        use std::time::SystemTime;
        let duration = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("System time before UNIX epoch");
        Self(duration.as_nanos() as u64)
    }

    /// Get nanoseconds
    pub fn as_nanos(&self) -> u64 {
        self.0
    }

    /// Get microseconds
    pub fn as_micros(&self) -> u64 {
        self.0 / 1_000
    }

    /// Get milliseconds
    pub fn as_millis(&self) -> u64 {
        self.0 / 1_000_000
    }

    /// Duration since another timestamp
    pub fn duration_since(&self, earlier: PrecisionTimestamp) -> Duration {
        Duration::from_nanos(self.0.saturating_sub(earlier.0))
    }
}

/// High-precision timer for frame-locked updates
pub struct PrecisionTimer {
    start_time: Instant,
    tick_rate: u32,
    tick_duration_nanos: u64,
    current_tick: u64,
    accumulator_nanos: u64,
}

impl PrecisionTimer {
    /// Create a new precision timer with specified tick rate
    pub fn new(tick_rate: u32) -> Self {
        let tick_duration_nanos = 1_000_000_000 / tick_rate as u64;

        debug!(
            tick_rate,
            tick_duration_micros = tick_duration_nanos / 1_000,
            "Created precision timer"
        );

        Self {
            start_time: Instant::now(),
            tick_rate,
            tick_duration_nanos,
            current_tick: 0,
            accumulator_nanos: 0,
        }
    }

    /// Update timer and return number of ticks to process
    pub fn update(&mut self) -> u32 {
        let elapsed = self.start_time.elapsed();
        let elapsed_nanos = elapsed.as_nanos() as u64;
        let expected_nanos = self.current_tick * self.tick_duration_nanos;
        let delta_nanos = elapsed_nanos.saturating_sub(expected_nanos);

        self.accumulator_nanos += delta_nanos;

        let ticks_to_process = (self.accumulator_nanos / self.tick_duration_nanos) as u32;

        if ticks_to_process > 0 {
            self.accumulator_nanos -= ticks_to_process as u64 * self.tick_duration_nanos;
            self.current_tick += ticks_to_process as u64;

            // Warn if we're falling behind
            if ticks_to_process > 1 {
                warn!(
                    ticks_to_process,
                    accumulator_micros = self.accumulator_nanos / 1_000,
                    "Multiple ticks in single frame - falling behind"
                );
            }
        }

        ticks_to_process
    }

    /// Get current tick number
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get tick rate
    pub fn tick_rate(&self) -> u32 {
        self.tick_rate
    }

    /// Get exact time for a specific tick
    pub fn tick_time(&self, tick: u64) -> Duration {
        Duration::from_nanos(tick * self.tick_duration_nanos)
    }

    /// Get interpolation alpha for smooth rendering
    pub fn interpolation_alpha(&self) -> f32 {
        (self.accumulator_nanos as f32) / (self.tick_duration_nanos as f32)
    }

    /// Reset timer
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.current_tick = 0;
        self.accumulator_nanos = 0;
    }
}

/// Deterministic tick-based simulation clock
pub struct DeterministicClock {
    tick: u64,
    tick_duration: Duration,
    total_time: Duration,
}

impl DeterministicClock {
    /// Create a new deterministic clock
    pub fn new(tick_rate: u32) -> Self {
        let tick_duration = Duration::from_nanos(1_000_000_000 / tick_rate as u64);

        Self { tick: 0, tick_duration, total_time: Duration::ZERO }
    }

    /// Advance clock by one tick
    pub fn tick(&mut self) {
        self.tick += 1;
        self.total_time += self.tick_duration;
    }

    /// Advance clock by multiple ticks
    pub fn tick_n(&mut self, n: u32) {
        self.tick += n as u64;
        self.total_time += self.tick_duration * n;
    }

    /// Get current tick
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// Get total elapsed time
    pub fn total_time(&self) -> Duration {
        self.total_time
    }

    /// Get tick duration
    pub fn tick_duration(&self) -> Duration {
        self.tick_duration
    }

    /// Get time as seconds (f64 for physics)
    pub fn time_seconds(&self) -> f64 {
        self.total_time.as_secs_f64()
    }

    /// Get delta time as seconds (f32 for game logic)
    pub fn delta_seconds(&self) -> f32 {
        self.tick_duration.as_secs_f32()
    }
}

/// Frame timing statistics for performance monitoring
#[derive(Debug, Clone, Copy)]
pub struct FrameTimingStats {
    /// Average frame time (microseconds)
    pub avg_frame_micros: u64,
    /// Minimum frame time (microseconds)
    pub min_frame_micros: u64,
    /// Maximum frame time (microseconds)
    pub max_frame_micros: u64,
    /// Frame time standard deviation (microseconds)
    pub stddev_micros: u64,
    /// Number of frames measured
    pub frame_count: u64,
}

/// Frame timing tracker
pub struct FrameTimingTracker {
    frame_times: Vec<u64>, // Microseconds
    max_samples: usize,
    last_frame: Instant,
}

impl FrameTimingTracker {
    /// Create a new frame timing tracker
    pub fn new(max_samples: usize) -> Self {
        Self {
            frame_times: Vec::with_capacity(max_samples),
            max_samples,
            last_frame: Instant::now(),
        }
    }

    /// Record a frame
    pub fn record_frame(&mut self) {
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_frame).as_micros() as u64;
        self.last_frame = now;

        if self.frame_times.len() >= self.max_samples {
            self.frame_times.remove(0);
        }
        self.frame_times.push(frame_time);
    }

    /// Get timing statistics
    pub fn stats(&self) -> Option<FrameTimingStats> {
        if self.frame_times.is_empty() {
            return None;
        }

        let sum: u64 = self.frame_times.iter().sum();
        let avg = sum / self.frame_times.len() as u64;

        let min = *self.frame_times.iter().min().unwrap();
        let max = *self.frame_times.iter().max().unwrap();

        // Calculate standard deviation
        let variance: u64 = self
            .frame_times
            .iter()
            .map(|&t| {
                let diff = t.abs_diff(avg);
                diff * diff
            })
            .sum::<u64>()
            / self.frame_times.len() as u64;

        let stddev = (variance as f64).sqrt() as u64;

        Some(FrameTimingStats {
            avg_frame_micros: avg,
            min_frame_micros: min,
            max_frame_micros: max,
            stddev_micros: stddev,
            frame_count: self.frame_times.len() as u64,
        })
    }

    /// Check if frame timing is stable (for esports)
    pub fn is_stable(&self, max_stddev_micros: u64) -> bool {
        if let Some(stats) = self.stats() {
            stats.stddev_micros <= max_stddev_micros
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precision_timestamp() {
        let t1 = PrecisionTimestamp::now();
        std::thread::sleep(Duration::from_millis(1));
        let t2 = PrecisionTimestamp::now();

        assert!(t2 > t1);
        let duration = t2.duration_since(t1);
        assert!(duration.as_micros() >= 1000); // At least 1ms
    }

    #[test]
    fn test_precision_timer_ticks() {
        let mut timer = PrecisionTimer::new(60); // 60 TPS

        // No ticks immediately
        let ticks = timer.update();
        assert_eq!(ticks, 0);

        // Wait for one tick (~16.67ms)
        std::thread::sleep(Duration::from_millis(17));
        let ticks = timer.update();
        assert!(ticks >= 1);
    }

    #[test]
    fn test_deterministic_clock() {
        let mut clock = DeterministicClock::new(60);

        assert_eq!(clock.current_tick(), 0);
        assert_eq!(clock.total_time(), Duration::ZERO);

        clock.tick();
        assert_eq!(clock.current_tick(), 1);
        assert!(clock.total_time().as_micros() > 16000); // ~16.67ms

        clock.tick_n(59);
        assert_eq!(clock.current_tick(), 60);
        // 60 ticks at 60 TPS = ~1 second (999.999ms due to rounding)
        assert!(clock.total_time().as_millis() >= 999);
    }

    #[test]
    fn test_frame_timing_tracker() {
        let mut tracker = FrameTimingTracker::new(10);

        // Record some frames
        for _ in 0..5 {
            std::thread::sleep(Duration::from_millis(16));
            tracker.record_frame();
        }

        let stats = tracker.stats().unwrap();
        assert_eq!(stats.frame_count, 5);
        assert!(stats.avg_frame_micros >= 16000); // ~16ms
        assert!(stats.avg_frame_micros <= 17000); // Give some tolerance
    }

    #[test]
    fn test_interpolation_alpha() {
        let mut timer = PrecisionTimer::new(60);

        // Alpha should start at 0
        assert_eq!(timer.interpolation_alpha(), 0.0);

        // After half a tick, alpha should be ~0.5
        std::thread::sleep(Duration::from_micros(8333)); // Half of 16.67ms
        timer.update();
        let alpha = timer.interpolation_alpha();
        assert!(alpha >= 0.4 && alpha <= 0.6);
    }

    #[test]
    fn test_tick_time_calculation() {
        let timer = PrecisionTimer::new(60);

        let tick0_time = timer.tick_time(0);
        assert_eq!(tick0_time, Duration::ZERO);

        let tick60_time = timer.tick_time(60);
        // 60 ticks at 60 TPS = 999.999ms (due to integer division rounding)
        assert!(tick60_time.as_millis() >= 999 && tick60_time.as_millis() <= 1000);

        let tick1_time = timer.tick_time(1);
        assert!(tick1_time.as_micros() >= 16666);
        assert!(tick1_time.as_micros() <= 16667);
    }
}
