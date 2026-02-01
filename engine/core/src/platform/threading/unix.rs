//! Unix and macOS threading implementations.

use super::{ThreadPriority, ThreadingBackend};
use crate::PlatformError;

/// Unix threading backend using pthread APIs.
#[cfg(all(unix, not(target_os = "macos")))]
pub struct UnixThreading;

#[cfg(all(unix, not(target_os = "macos")))]
impl UnixThreading {
    /// Create a new Unix threading backend.
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self)
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
impl ThreadingBackend for UnixThreading {
    fn set_thread_priority(&self, priority: ThreadPriority) -> Result<(), PlatformError> {
        use libc::{pthread_self, pthread_setschedparam, sched_param, SCHED_OTHER, SCHED_RR};

        let (policy, sched_priority) = match priority {
            ThreadPriority::Low => (SCHED_OTHER, 0),
            ThreadPriority::Normal => (SCHED_OTHER, 0),
            ThreadPriority::High => (SCHED_OTHER, 0),
            ThreadPriority::Realtime => (SCHED_RR, 50), // Middle realtime priority
        };

        let param = sched_param { sched_priority };

        let result = unsafe { pthread_setschedparam(pthread_self(), policy, &param as *const _) };

        if result != 0 {
            return Err(PlatformError::ThreadingError {
                operation: "set_priority".to_string(),
                details: format!(
                    "pthread_setschedparam failed with code {} for {:?}",
                    result, priority
                ),
            });
        }

        Ok(())
    }

    fn set_thread_affinity(&self, cores: &[usize]) -> Result<(), PlatformError> {
        if cores.is_empty() {
            return Err(PlatformError::ThreadingError {
                operation: "set_affinity".to_string(),
                details: "Core list cannot be empty".to_string(),
            });
        }

        #[cfg(target_os = "linux")]
        {
            use libc::{cpu_set_t, pthread_self, pthread_setaffinity_np, CPU_SET, CPU_ZERO};

            unsafe {
                let mut cpuset: cpu_set_t = std::mem::zeroed();
                CPU_ZERO(&mut cpuset);

                for &core in cores {
                    CPU_SET(core, &mut cpuset);
                }

                let result = pthread_setaffinity_np(
                    pthread_self(),
                    std::mem::size_of::<cpu_set_t>(),
                    &cpuset as *const _,
                );

                if result != 0 {
                    return Err(PlatformError::ThreadingError {
                        operation: "set_affinity".to_string(),
                        details: format!("pthread_setaffinity_np failed with code {}", result),
                    });
                }
            }

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Other Unix systems may not support CPU affinity
            Err(PlatformError::PlatformNotSupported {
                platform: std::env::consts::OS.to_string(),
                feature: "thread_affinity".to_string(),
            })
        }
    }
}

/// macOS threading backend.
#[cfg(target_os = "macos")]
pub struct MacOsThreading;

#[cfg(target_os = "macos")]
impl MacOsThreading {
    /// Create a new macOS threading backend.
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self)
    }
}

#[cfg(target_os = "macos")]
impl ThreadingBackend for MacOsThreading {
    fn set_thread_priority(&self, priority: ThreadPriority) -> Result<(), PlatformError> {
        use libc::{pthread_self, pthread_setschedparam, sched_param, SCHED_OTHER, SCHED_RR};

        let (policy, sched_priority) = match priority {
            ThreadPriority::Low => (SCHED_OTHER, 0),
            ThreadPriority::Normal => (SCHED_OTHER, 0),
            ThreadPriority::High => (SCHED_OTHER, 0),
            ThreadPriority::Realtime => (SCHED_RR, 50),
        };

        let param = sched_param { sched_priority };

        let result = unsafe { pthread_setschedparam(pthread_self(), policy, &param as *const _) };

        if result != 0 {
            return Err(PlatformError::ThreadingError {
                operation: "set_priority".to_string(),
                details: format!(
                    "pthread_setschedparam failed with code {} for {:?}",
                    result, priority
                ),
            });
        }

        Ok(())
    }

    fn set_thread_affinity(&self, _cores: &[usize]) -> Result<(), PlatformError> {
        // macOS doesn't support CPU affinity in the same way as Linux
        // thread_policy_set can be used but it's complex and not always reliable
        Err(PlatformError::PlatformNotSupported {
            platform: "macos".to_string(),
            feature: "thread_affinity".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn test_unix_threading_creation() {
        let threading = UnixThreading::new();
        assert!(threading.is_ok());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn test_unix_set_priority() {
        let threading = UnixThreading::new().unwrap();

        // These should work without special privileges
        assert!(threading.set_thread_priority(ThreadPriority::Normal).is_ok());

        // Realtime may require CAP_SYS_NICE or root
        let _ = threading.set_thread_priority(ThreadPriority::Realtime);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_threading_creation() {
        let threading = MacOsThreading::new();
        assert!(threading.is_ok());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_set_priority() {
        let threading = MacOsThreading::new().unwrap();
        assert!(threading.set_thread_priority(ThreadPriority::Normal).is_ok());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_affinity_not_supported() {
        let threading = MacOsThreading::new().unwrap();
        let result = threading.set_thread_affinity(&[0]);
        assert!(result.is_err());
    }
}
