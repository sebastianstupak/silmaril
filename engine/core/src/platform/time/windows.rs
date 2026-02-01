//! Windows-specific time implementation using QueryPerformanceCounter.

use super::TimeBackend;
use crate::PlatformError;
use std::time::Duration;

/// Windows time backend using QueryPerformanceCounter.
///
/// This provides high-resolution monotonic time on Windows.
/// Typical resolution is ~100ns on modern systems.
pub struct WindowsTime {
    frequency: i64,
}

impl WindowsTime {
    /// Create a new Windows time backend.
    pub fn new() -> Result<Self, PlatformError> {
        use winapi::shared::ntdef::LARGE_INTEGER;
        use winapi::um::profileapi::QueryPerformanceFrequency;

        let mut frequency: LARGE_INTEGER = unsafe { std::mem::zeroed() };
        let result = unsafe { QueryPerformanceFrequency(&mut frequency) };

        if result == 0 {
            return Err(PlatformError::timeinitfailed(
                "QueryPerformanceFrequency failed".to_string(),
            ));
        }

        // Extract the i64 value from the union
        let freq = unsafe { *frequency.QuadPart() };

        Ok(Self { frequency: freq })
    }
}

impl TimeBackend for WindowsTime {
    fn monotonic_nanos(&self) -> u64 {
        use winapi::shared::ntdef::LARGE_INTEGER;
        use winapi::um::profileapi::QueryPerformanceCounter;

        let mut counter: LARGE_INTEGER = unsafe { std::mem::zeroed() };
        unsafe {
            QueryPerformanceCounter(&mut counter);
        }

        // Extract the i64 value from the union
        let count = unsafe { *counter.QuadPart() };

        // Convert to nanoseconds
        // counter / frequency = seconds
        // (counter * 1_000_000_000) / frequency = nanoseconds
        ((count as u128 * 1_000_000_000) / self.frequency as u128) as u64
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_time_creation() {
        let time = WindowsTime::new();
        assert!(time.is_ok());
    }

    #[test]
    fn test_windows_time_monotonic() {
        let time = WindowsTime::new().unwrap();
        let t1 = time.monotonic_nanos();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = time.monotonic_nanos();

        assert!(t2 > t1);
    }

    #[test]
    fn test_windows_time_precision() {
        let time = WindowsTime::new().unwrap();

        // Measure how long it takes to call monotonic_nanos()
        let t1 = time.monotonic_nanos();
        let t2 = time.monotonic_nanos();

        // Should have sub-microsecond precision
        let diff = t2 - t1;
        assert!(diff < 10_000); // Less than 10 microseconds overhead
    }
}
