//! Progress tracking for downloads and updates.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Progress information for an ongoing operation.
#[derive(Debug, Clone)]
pub struct Progress {
    /// Bytes downloaded or processed
    pub bytes_processed: u64,
    /// Total bytes to process
    pub total_bytes: u64,
    /// Current download/processing speed (bytes per second)
    pub speed: f64,
    /// Estimated time remaining
    pub eta: Option<Duration>,
}

impl Progress {
    /// Calculate the percentage complete (0.0 to 100.0).
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.bytes_processed as f64 / self.total_bytes as f64) * 100.0
        }
    }

    /// Check if the operation is complete.
    pub fn is_complete(&self) -> bool {
        self.bytes_processed >= self.total_bytes
    }

    /// Format the speed as a human-readable string.
    pub fn speed_string(&self) -> String {
        format_bytes_per_second(self.speed)
    }

    /// Format the ETA as a human-readable string.
    pub fn eta_string(&self) -> String {
        match self.eta {
            Some(duration) => format_duration(duration),
            None => "Unknown".to_string(),
        }
    }
}

/// Thread-safe progress tracker.
#[derive(Clone)]
pub struct ProgressTracker {
    bytes_processed: Arc<AtomicU64>,
    total_bytes: Arc<AtomicU64>,
    start_time: Arc<parking_lot::Mutex<Option<Instant>>>,
}

impl ProgressTracker {
    /// Create a new progress tracker.
    pub fn new(total_bytes: u64) -> Self {
        Self {
            bytes_processed: Arc::new(AtomicU64::new(0)),
            total_bytes: Arc::new(AtomicU64::new(total_bytes)),
            start_time: Arc::new(parking_lot::Mutex::new(Some(Instant::now()))),
        }
    }

    /// Update the amount of bytes processed.
    pub fn add_bytes(&self, bytes: u64) {
        self.bytes_processed.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Set the total bytes to process.
    pub fn set_total_bytes(&self, total: u64) {
        self.total_bytes.store(total, Ordering::Relaxed);
    }

    /// Reset the progress tracker.
    pub fn reset(&self) {
        self.bytes_processed.store(0, Ordering::Relaxed);
        *self.start_time.lock() = Some(Instant::now());
    }

    /// Get the current progress.
    pub fn get_progress(&self) -> Progress {
        let bytes_processed = self.bytes_processed.load(Ordering::Relaxed);
        let total_bytes = self.total_bytes.load(Ordering::Relaxed);

        let (speed, eta) = if let Some(start) = *self.start_time.lock() {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let speed = bytes_processed as f64 / elapsed;
                let remaining_bytes = total_bytes.saturating_sub(bytes_processed);
                let eta = if speed > 0.0 {
                    Some(Duration::from_secs_f64(remaining_bytes as f64 / speed))
                } else {
                    None
                };
                (speed, eta)
            } else {
                (0.0, None)
            }
        } else {
            (0.0, None)
        };

        Progress { bytes_processed, total_bytes, speed, eta }
    }
}

/// Format bytes per second as a human-readable string.
pub fn format_bytes_per_second(bytes_per_sec: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    if bytes_per_sec >= GB {
        format!("{:.2} GB/s", bytes_per_sec / GB)
    } else if bytes_per_sec >= MB {
        format!("{:.2} MB/s", bytes_per_sec / MB)
    } else if bytes_per_sec >= KB {
        format!("{:.2} KB/s", bytes_per_sec / KB)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

/// Format a duration as a human-readable string.
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Multi-file progress tracker.
pub struct MultiFileProgressTracker {
    current_file: Arc<AtomicUsize>,
    total_files: usize,
    file_tracker: ProgressTracker,
}

impl MultiFileProgressTracker {
    /// Create a new multi-file progress tracker.
    pub fn new(total_files: usize) -> Self {
        Self {
            current_file: Arc::new(AtomicUsize::new(0)),
            total_files,
            file_tracker: ProgressTracker::new(0),
        }
    }

    /// Start tracking a new file.
    pub fn start_file(&self, file_size: u64) {
        self.current_file.fetch_add(1, Ordering::Relaxed);
        self.file_tracker.set_total_bytes(file_size);
        self.file_tracker.reset();
    }

    /// Get the file-level progress tracker.
    pub fn file_tracker(&self) -> &ProgressTracker {
        &self.file_tracker
    }

    /// Get the current file index (1-based).
    pub fn current_file(&self) -> usize {
        self.current_file.load(Ordering::Relaxed)
    }

    /// Get the total number of files.
    pub fn total_files(&self) -> usize {
        self.total_files
    }

    /// Get the overall progress percentage.
    pub fn overall_percentage(&self) -> f64 {
        let current = self.current_file.load(Ordering::Relaxed);
        if self.total_files == 0 {
            0.0
        } else {
            (current as f64 / self.total_files as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_progress_percentage() {
        let progress = Progress { bytes_processed: 50, total_bytes: 100, speed: 10.0, eta: None };
        assert_eq!(progress.percentage(), 50.0);
    }

    #[test]
    fn test_progress_is_complete() {
        let mut progress =
            Progress { bytes_processed: 100, total_bytes: 100, speed: 10.0, eta: None };
        assert!(progress.is_complete());

        progress.bytes_processed = 50;
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_progress_tracker() {
        let tracker = ProgressTracker::new(1000);
        tracker.add_bytes(500);

        let progress = tracker.get_progress();
        assert_eq!(progress.bytes_processed, 500);
        assert_eq!(progress.total_bytes, 1000);
        assert_eq!(progress.percentage(), 50.0);
    }

    #[test]
    fn test_progress_tracker_speed() {
        let tracker = ProgressTracker::new(1000);
        thread::sleep(Duration::from_millis(100));
        tracker.add_bytes(100);

        let progress = tracker.get_progress();
        assert!(progress.speed > 0.0);
        assert!(progress.eta.is_some());
    }

    #[test]
    fn test_format_bytes_per_second() {
        assert_eq!(format_bytes_per_second(500.0), "500 B/s");
        assert_eq!(format_bytes_per_second(1024.0), "1.00 KB/s");
        assert_eq!(format_bytes_per_second(1024.0 * 1024.0), "1.00 MB/s");
        assert_eq!(format_bytes_per_second(1024.0 * 1024.0 * 1024.0), "1.00 GB/s");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
    }

    #[test]
    fn test_multi_file_tracker() {
        let tracker = MultiFileProgressTracker::new(3);
        assert_eq!(tracker.current_file(), 0);
        assert_eq!(tracker.overall_percentage(), 0.0);

        tracker.start_file(100);
        assert_eq!(tracker.current_file(), 1);

        tracker.start_file(200);
        assert_eq!(tracker.current_file(), 2);
        assert!(tracker.overall_percentage() > 0.0);
    }
}
