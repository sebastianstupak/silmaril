//! Unix and macOS time implementations.

use super::TimeBackend;
use crate::PlatformError;
use std::time::Duration;

/// Unix time backend using clock_gettime with CLOCK_MONOTONIC.
///
/// This provides high-resolution monotonic time on Linux and other Unix systems.
#[cfg(all(unix, not(target_os = "macos")))]
pub struct UnixTime;

#[cfg(all(unix, not(target_os = "macos")))]
impl UnixTime {
    /// Create a new Unix time backend.
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self)
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
impl TimeBackend for UnixTime {
    fn monotonic_nanos(&self) -> u64 {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };

        unsafe {
            libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts);
        }

        (ts.tv_sec as u64) * 1_000_000_000 + (ts.tv_nsec as u64)
    }

    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// macOS time backend using mach_absolute_time.
///
/// This provides the highest resolution monotonic time on macOS.
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
    fn monotonic_nanos(&self) -> u64 {
        let time = unsafe { mach_absolute_time() };

        // Convert to nanoseconds using timebase
        ((time as u128 * self.timebase_info.numer as u128) / self.timebase_info.denom as u128)
            as u64
    }

    fn sleep(&self, duration: Duration) {
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
}
