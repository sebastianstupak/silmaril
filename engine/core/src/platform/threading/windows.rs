//! Windows-specific threading implementation.

use super::{ThreadPriority, ThreadingBackend};
use crate::PlatformError;
use winapi::um::processthreadsapi::{GetCurrentThread, SetThreadPriority};
use winapi::um::winbase::{
    SetThreadAffinityMask, THREAD_PRIORITY_HIGHEST, THREAD_PRIORITY_LOWEST, THREAD_PRIORITY_NORMAL,
    THREAD_PRIORITY_TIME_CRITICAL,
};

/// Windows threading backend.
///
/// # Performance Optimizations
///
/// - Caches CPU count at initialization to avoid repeated syscalls
/// - Thread-safe via Windows API guarantees (no locks needed)
///
/// # Target Performance
///
/// - set_thread_priority: <5us (ideal: 2us)
/// - set_thread_affinity (1 core): <10us (ideal: 5us)
/// - num_cpus: <1us (ideal: <100ns, cached)
pub struct WindowsThreading {
    /// Number of CPUs, cached for fast access
    num_cpus: usize,
}

impl WindowsThreading {
    /// Create a new Windows threading backend.
    pub fn new() -> Result<Self, PlatformError> {
        // Cache CPU count for fast access
        let num_cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);

        Ok(Self { num_cpus })
    }
}

impl ThreadingBackend for WindowsThreading {
    fn set_thread_priority(&self, priority: ThreadPriority) -> Result<(), PlatformError> {
        let win_priority = match priority {
            ThreadPriority::Low => THREAD_PRIORITY_LOWEST as i32,
            ThreadPriority::Normal => THREAD_PRIORITY_NORMAL as i32,
            ThreadPriority::High => THREAD_PRIORITY_HIGHEST as i32,
            ThreadPriority::Realtime => THREAD_PRIORITY_TIME_CRITICAL as i32,
        };

        let result = unsafe { SetThreadPriority(GetCurrentThread(), win_priority) };

        if result == 0 {
            return Err(PlatformError::threadingerror(
                "set_priority".to_string(),
                format!("SetThreadPriority failed for {:?}", priority),
            ));
        }

        Ok(())
    }

    fn set_thread_affinity(&self, cores: &[usize]) -> Result<(), PlatformError> {
        if cores.is_empty() {
            return Err(PlatformError::threadingerror(
                "set_affinity".to_string(),
                "Core list cannot be empty".to_string(),
            ));
        }

        // Validate core indices before building mask
        for &core in cores {
            if core >= self.num_cpus {
                return Err(PlatformError::threadingerror(
                    "set_affinity".to_string(),
                    format!("Core {} exceeds available CPUs ({})", core, self.num_cpus),
                ));
            }
        }

        // Build affinity mask
        let mut mask: usize = 0;
        for &core in cores {
            if core >= std::mem::size_of::<usize>() * 8 {
                return Err(PlatformError::threadingerror(
                    "set_affinity".to_string(),
                    format!("Core index {} is out of range", core),
                ));
            }
            mask |= 1 << core;
        }

        let result = unsafe { SetThreadAffinityMask(GetCurrentThread(), mask) };

        if result == 0 {
            return Err(PlatformError::threadingerror(
                "set_affinity".to_string(),
                "SetThreadAffinityMask failed".to_string(),
            ));
        }

        Ok(())
    }

    fn num_cpus(&self) -> usize {
        // Return cached value for fast access
        self.num_cpus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_threading_creation() {
        let threading = WindowsThreading::new();
        assert!(threading.is_ok());
    }

    #[test]
    fn test_set_priorities() {
        let threading = WindowsThreading::new().unwrap();

        assert!(threading.set_thread_priority(ThreadPriority::Low).is_ok());
        assert!(threading.set_thread_priority(ThreadPriority::Normal).is_ok());
        assert!(threading.set_thread_priority(ThreadPriority::High).is_ok());

        // Realtime may fail without admin privileges
        let _ = threading.set_thread_priority(ThreadPriority::Realtime);
    }

    #[test]
    fn test_set_affinity() {
        let threading = WindowsThreading::new().unwrap();

        // Try to set affinity to first core
        let result = threading.set_thread_affinity(&[0]);
        // May fail on some systems, but shouldn't panic
        let _ = result;
    }

    #[test]
    fn test_empty_affinity_fails() {
        let threading = WindowsThreading::new().unwrap();
        let result = threading.set_thread_affinity(&[]);
        assert!(result.is_err());
    }
}
