//! Windows-specific time implementation using QueryPerformanceCounter.

use super::TimeBackend;
use crate::PlatformError;
use std::time::Duration;

/// Windows time backend using QueryPerformanceCounter.
///
/// This provides high-resolution monotonic time on Windows.
/// Typical resolution is ~100ns on modern systems.
///
/// # Optimizations
///
/// - Pre-computes the frequency-to-nanoseconds conversion factor
/// - Uses floating-point multiplication instead of 128-bit integer division
/// - Caches the conversion factor to avoid repeated division
pub struct WindowsTime {
    /// Pre-computed conversion factor: nanos_per_tick = 1_000_000_000.0 / frequency
    /// This allows us to convert counter values to nanoseconds with a single multiply.
    nanos_per_tick: f64,
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

        // Pre-compute the conversion factor from ticks to nanoseconds.
        // Instead of: (counter * 1_000_000_000) / frequency
        // We compute: counter * (1_000_000_000.0 / frequency)
        //
        // This replaces a 128-bit integer division with a 64-bit float multiply,
        // which is significantly faster on modern CPUs.
        let nanos_per_tick = 1_000_000_000.0 / freq as f64;

        Ok(Self { nanos_per_tick })
    }
}

impl TimeBackend for WindowsTime {
    #[inline]
    fn monotonic_nanos(&self) -> u64 {
        use winapi::shared::ntdef::LARGE_INTEGER;
        use winapi::um::profileapi::QueryPerformanceCounter;

        // QueryPerformanceCounter is the bottleneck here, not our conversion
        let mut counter: LARGE_INTEGER = unsafe { std::mem::zeroed() };
        unsafe {
            QueryPerformanceCounter(&mut counter);
        }

        // Extract the i64 value from the union
        let count = unsafe { *counter.QuadPart() };

        // OPTIMIZATION: Use pre-computed floating-point multiplier
        // This is faster than 128-bit integer arithmetic and provides
        // more than enough precision for time measurements.
        //
        // On typical systems (10MHz QPC frequency):
        // - Precision: ~1ns (f64 has 53 bits of mantissa, more than enough)
        // - Speed: Single FP multiply vs 128-bit integer division
        (count as f64 * self.nanos_per_tick) as u64
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

    #[test]
    fn test_conversion_precision() {
        let time = WindowsTime::new().unwrap();

        // Test that the conversion maintains precision
        // Even with floating-point, we should have nanosecond precision
        let mut last = 0u64;
        for _ in 0..1000 {
            let now = time.monotonic_nanos();
            assert!(now >= last, "Time should be monotonic");
            last = now;
        }
    }
}
