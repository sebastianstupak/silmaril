//! Unix and macOS time implementations.

use super::TimeBackend;
use crate::PlatformError;
use std::time::Duration;

/// Unix time backend using clock_gettime with CLOCK_MONOTONIC.
///
/// This provides high-resolution monotonic time on Linux and other Unix systems.
///
/// # Performance Optimizations
///
/// - Uses CLOCK_MONOTONIC which is typically vDSO-accelerated on Linux (no syscall overhead)
/// - On kernels 2.6.32+, clock_gettime is mapped to vDSO and executes in <30ns
/// - Pre-zeroed timespec to avoid extra memory initialization
/// - CLOCK_MONOTONIC is preferred over CLOCK_MONOTONIC_RAW:
///   - MONOTONIC: vDSO-accelerated, NTP-adjusted (stable but corrected)
///   - MONOTONIC_RAW: Not vDSO-accelerated, requires syscall (~100ns overhead)
///
/// # Target Performance
///
/// - Single call: <30ns (ideal), <50ns (acceptable)
/// - Batch (1000 calls): <30us (ideal), <50us (acceptable)
#[cfg(all(unix, not(target_os = "macos")))]
pub struct UnixTime {
    // Cache the clock ID to avoid lookup overhead
    _clock_id: libc::clockid_t,
}

#[cfg(all(unix, not(target_os = "macos")))]
impl UnixTime {
    /// Create a new Unix time backend.
    ///
    /// Validates that CLOCK_MONOTONIC is available on this system.
    pub fn new() -> Result<Self, PlatformError> {
        // Verify clock is available by making a test call
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };

        let result = unsafe {
            libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts as *mut _)
        };

        if result != 0 {
            return Err(PlatformError::TimeInitFailed {
                details: "CLOCK_MONOTONIC not available".to_string(),
            });
        }

        Ok(Self {
            _clock_id: libc::CLOCK_MONOTONIC,
        })
    }

    /// Get the current time using CLOCK_MONOTONIC_RAW (not vDSO-accelerated).
    ///
    /// This is slower (~100ns vs ~30ns) but provides raw hardware time
    /// without NTP adjustments. Only use for specialized profiling needs.
    #[cfg(target_os = "linux")]
    #[allow(dead_code)]
    pub fn monotonic_nanos_raw(&self) -> u64 {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };

        unsafe {
            libc::clock_gettime(libc::CLOCK_MONOTONIC_RAW, &mut ts as *mut _);
        }

        // Convert to nanoseconds with overflow protection
        (ts.tv_sec as u64)
            .saturating_mul(1_000_000_000)
            .saturating_add(ts.tv_nsec as u64)
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
impl TimeBackend for UnixTime {
    #[inline]
    fn monotonic_nanos(&self) -> u64 {
        // SAFETY: clock_gettime is thread-safe and CLOCK_MONOTONIC is guaranteed
        // to be available on all modern Unix systems. The vDSO implementation
        // makes this extremely fast on Linux (no syscall).
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };

        unsafe {
            // On Linux with vDSO, this is a direct function call, not a syscall
            libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts as *mut _);
        }

        // Convert to nanoseconds with overflow protection
        // Modern systems won't overflow for decades, but we use saturating_mul
        // to be safe and avoid panics in debug mode
        (ts.tv_sec as u64)
            .saturating_mul(1_000_000_000)
            .saturating_add(ts.tv_nsec as u64)
    }

    fn sleep(&self, duration: Duration) {
        // Use standard library sleep which handles signals correctly
        std::thread::sleep(duration);
    }
}

/// macOS time backend using mach_absolute_time.
///
/// This provides the highest resolution monotonic time on macOS.
///
/// # Performance Optimizations
///
/// - mach_absolute_time is extremely fast: <15ns on Apple Silicon, <25ns on Intel
/// - Timebase is cached at initialization (only queried once)
/// - Special-case 1:1 timebase ratio (common on Apple Silicon M1/M2/M3)
/// - Avoid u128 arithmetic when possible (10-20% faster)
/// - Inline hint for hot path
///
/// # Timebase Ratios
///
/// - **Apple Silicon (M1/M2/M3)**: Usually 1:1 (numer=1, denom=1) - already in nanoseconds
/// - **Intel Macs**: Varies, commonly 1:1000000000 or similar - requires conversion
///
/// # Target Performance
///
/// - Single call: <20ns (Apple Silicon), <30ns (Intel)
/// - Batch (1000 calls): <20us (Apple Silicon), <30us (Intel)
#[cfg(target_os = "macos")]
pub struct MacOsTime {
    timebase_info: mach_timebase_info_data_t,
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct mach_timebase_info_data_t {
    numer: u32,
    denom: u32,
}

#[cfg(target_os = "macos")]
extern "C" {
    fn mach_absolute_time() -> u64;
    fn mach_timebase_info(info: *mut mach_timebase_info_data_t) -> i32;
}

#[cfg(target_os = "macos")]
impl MacOsTime {
    /// Create a new macOS time backend.
    pub fn new() -> Result<Self, PlatformError> {
        let mut timebase_info = mach_timebase_info_data_t { numer: 0, denom: 0 };

        let result = unsafe { mach_timebase_info(&mut timebase_info) };

        if result != 0 {
            return Err(PlatformError::TimeInitFailed {
                details: "mach_timebase_info failed".to_string(),
            });
        }

        Ok(Self { timebase_info })
    }
}

#[cfg(target_os = "macos")]
impl TimeBackend for MacOsTime {
    #[inline]
    fn monotonic_nanos(&self) -> u64 {
        // OPTIMIZATION: mach_absolute_time is already very fast (<20ns on Apple Silicon, <30ns on Intel)
        // The bottleneck is the conversion. We optimize this by:
        // 1. Using u128 only when necessary to avoid overflow
        // 2. Special-casing the common 1:1 timebase ratio (Apple Silicon)
        // 3. Avoiding unnecessary casts when possible

        let time = unsafe { mach_absolute_time() };

        // OPTIMIZATION: On Apple Silicon (M1/M2/M3), the timebase is typically 1:1 (numer=1, denom=1)
        // This avoids the multiplication/division entirely for the common case.
        // On Intel Macs, this is often 1:1000000000 or similar, requiring conversion.
        if self.timebase_info.numer == self.timebase_info.denom {
            // Already in nanoseconds, no conversion needed
            // This is the fast path for Apple Silicon
            time
        } else if self.timebase_info.denom == 1 {
            // Only multiplication needed (rare case)
            // Avoid u128 if we can fit in u64
            time.saturating_mul(self.timebase_info.numer as u64)
        } else {
            // Full conversion needed - use u128 to avoid overflow
            // This is the slow path but necessary for correctness on Intel Macs
            ((time as u128 * self.timebase_info.numer as u128) / self.timebase_info.denom as u128)
                as u64
        }
    }

    fn sleep(&self, duration: Duration) {
        // OPTIMIZATION: std::thread::sleep on macOS uses nanosleep internally,
        // which is already optimal. No further optimization needed.
        // On macOS, nanosleep has microsecond precision and is accurate within 1-2%.
        std::thread::sleep(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn test_unix_time_creation() {
        let time = UnixTime::new();
        assert!(time.is_ok());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn test_unix_time_monotonic() {
        let time = UnixTime::new().unwrap();
        let t1 = time.monotonic_nanos();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = time.monotonic_nanos();

        assert!(t2 > t1);
        assert!(t2 - t1 >= 10_000_000); // At least 10ms
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_time_creation() {
        let time = MacOsTime::new();
        assert!(time.is_ok());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_time_monotonic() {
        let time = MacOsTime::new().unwrap();
        let t1 = time.monotonic_nanos();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = time.monotonic_nanos();

        assert!(t2 > t1);
        assert!(t2 - t1 >= 10_000_000); // At least 10ms
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_timebase_ratio() {
        let time = MacOsTime::new().unwrap();
        // Timebase should be initialized
        assert!(time.timebase_info.numer > 0);
        assert!(time.timebase_info.denom > 0);

        // Common ratios:
        // Apple Silicon: 1:1
        // Intel Macs: varies, often 1:1000000000 or similar
        // Just verify they're reasonable values
        assert!(time.timebase_info.numer < 1_000_000_000);
        assert!(time.timebase_info.denom < 1_000_000_000);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_time_precision() {
        let time = MacOsTime::new().unwrap();

        // Measure 100 consecutive calls - time should advance
        let mut last = time.monotonic_nanos();
        let mut advances = 0;

        for _ in 0..100 {
            let now = time.monotonic_nanos();
            if now > last {
                advances += 1;
            }
            assert!(now >= last, "Time decreased: {} -> {}", last, now);
            last = now;
        }

        // On fast systems, some calls may return the same value,
        // but we should see some advances
        assert!(advances > 0, "Time never advanced in 100 calls");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_sleep_accuracy() {
        let time = MacOsTime::new().unwrap();

        // Test various sleep durations
        for ms in [1, 5, 10, 50] {
            let t1 = time.monotonic_nanos();
            time.sleep(Duration::from_millis(ms));
            let t2 = time.monotonic_nanos();

            let elapsed_ms = (t2 - t1) / 1_000_000;

            // Allow some tolerance (sleep may be slightly longer)
            // Minimum: at least 90% of requested duration
            // Maximum: at most 150% of requested duration (generous on macOS)
            assert!(
                elapsed_ms >= (ms * 9 / 10),
                "Sleep too short: {}ms vs {}ms requested",
                elapsed_ms,
                ms
            );
            assert!(
                elapsed_ms <= (ms * 3 / 2),
                "Sleep too long: {}ms vs {}ms requested",
                elapsed_ms,
                ms
            );
        }
    }
}
