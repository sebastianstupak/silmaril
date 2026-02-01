//! Time and timing abstractions.
//!
//! This module provides cross-platform time measurement with high precision.
//! All implementations guarantee monotonic time that never goes backwards.

use crate::PlatformError;
use std::time::Duration;

/// Trait for platform-specific time backends.
///
/// Implementations must provide monotonic time that never goes backwards,
/// even across system clock adjustments.
pub trait TimeBackend: Send + Sync {
    /// Get the current monotonic time in nanoseconds.
    ///
    /// This value is guaranteed to:
    /// - Never decrease between successive calls
    /// - Have at least microsecond precision
    /// - Not be affected by system clock adjustments
    ///
    /// The epoch is unspecified and may vary between platforms.
    /// Only use this for measuring durations, not absolute timestamps.
    fn monotonic_nanos(&self) -> u64;

    /// Sleep for the specified duration.
    ///
    /// The actual sleep time may be longer than requested due to
    /// system scheduling, but will never be significantly shorter.
    fn sleep(&self, duration: Duration);

    /// Get the current time as a Duration since an unspecified epoch.
    fn now(&self) -> Duration {
        Duration::from_nanos(self.monotonic_nanos())
    }
}

// Platform-specific implementations
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

/// Create a time backend for the current platform.
pub fn create_time_backend() -> Result<Box<dyn TimeBackend>, PlatformError> {
    #[cfg(windows)]
    return Ok(Box::new(windows::WindowsTime::new()?));

    #[cfg(all(unix, not(target_os = "macos")))]
    return Ok(Box::new(unix::UnixTime::new()?));

    #[cfg(target_os = "macos")]
    return Ok(Box::new(unix::MacOsTime::new()?));

    #[cfg(not(any(windows, unix)))]
    return Err(PlatformError::PlatformNotSupported {
        platform: std::env::consts::OS.to_string(),
        feature: "time".to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_backend_creation() {
        let backend = create_time_backend();
        assert!(backend.is_ok());
    }

    #[test]
    fn test_monotonic_time() {
        let backend = create_time_backend().unwrap();
        let t1 = backend.monotonic_nanos();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = backend.monotonic_nanos();

        assert!(t2 > t1, "Time should be monotonic");
        let diff = t2 - t1;
        assert!(diff >= 10_000_000, "At least 10ms should have passed");
    }

    #[test]
    fn test_time_never_decreases() {
        let backend = create_time_backend().unwrap();
        let mut last = 0u64;

        for _ in 0..1000 {
            let now = backend.monotonic_nanos();
            assert!(now >= last, "Time should never decrease: {} -> {}", last, now);
            last = now;
        }
    }

    #[test]
    fn test_time_precision() {
        let backend = create_time_backend().unwrap();
        let t1 = backend.monotonic_nanos();
        // Do some work
        let _sum: u64 = (0..1000).sum();
        let t2 = backend.monotonic_nanos();

        // Time should have advanced (at least some nanoseconds)
        assert!(t2 >= t1);
    }

    #[test]
    fn test_sleep() {
        let backend = create_time_backend().unwrap();
        let t1 = backend.monotonic_nanos();
        backend.sleep(Duration::from_millis(50));
        let t2 = backend.monotonic_nanos();

        let elapsed = Duration::from_nanos(t2 - t1);
        // Sleep should be at least 50ms (allowing some overhead)
        assert!(elapsed >= Duration::from_millis(45));
    }

    #[test]
    fn test_now_helper() {
        let backend = create_time_backend().unwrap();
        let d1 = backend.now();
        std::thread::sleep(Duration::from_millis(10));
        let d2 = backend.now();

        assert!(d2 > d1);
    }
}
