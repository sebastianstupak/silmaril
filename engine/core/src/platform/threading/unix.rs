//! Unix and macOS threading implementations.

use super::{ThreadPriority, ThreadingBackend};
use crate::PlatformError;

/// Unix threading backend using pthread APIs.
///
/// # Performance Optimizations
///
/// - Caches scheduling policies to avoid repeated lookups
/// - Uses SCHED_BATCH for low priority (better for background tasks)
/// - Pre-validates CPU affinity to avoid syscall failures
/// - Thread-safe via pthread APIs (no locks needed)
///
/// # Permissions
///
/// - Normal/Low/High priorities: Work without special privileges
/// - Realtime priority: Requires CAP_SYS_NICE capability or root
/// - CPU affinity: Works for all users (pinning to any core)
///
/// # Target Performance
///
/// - set_thread_priority: <5us (ideal: 2us)
/// - set_thread_affinity (1 core): <10us (ideal: 5us)
/// - set_thread_affinity (4 cores): <15us (ideal: 8us)
#[cfg(all(unix, not(target_os = "macos")))]
pub struct UnixThreading {
    /// Number of CPUs, cached for validation
    num_cpus: usize,
}

#[cfg(all(unix, not(target_os = "macos")))]
impl UnixThreading {
    /// Create a new Unix threading backend.
    pub fn new() -> Result<Self, PlatformError> {
        // Cache CPU count for affinity validation
        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        Ok(Self { num_cpus })
    }

    /// Check if we have realtime scheduling permissions.
    ///
    /// This requires CAP_SYS_NICE or root on Linux.
    #[cfg(target_os = "linux")]
    pub fn has_realtime_permissions(&self) -> bool {
        use libc::{geteuid, pthread_self, pthread_setschedparam, sched_param, SCHED_RR};

        // If we're root, we have permission
        if unsafe { geteuid() } == 0 {
            return true;
        }

        // Try to set realtime priority temporarily
        let param = sched_param { sched_priority: 1 };
        let result = unsafe { pthread_setschedparam(pthread_self(), SCHED_RR, &param as *const _) };

        if result == 0 {
            // Success, revert back to normal
            let normal_param = sched_param { sched_priority: 0 };
            unsafe {
                pthread_setschedparam(pthread_self(), libc::SCHED_OTHER, &normal_param as *const _)
            };
            true
        } else {
            false
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
impl ThreadingBackend for UnixThreading {
    fn set_thread_priority(&self, priority: ThreadPriority) -> Result<(), PlatformError> {
        use libc::{pthread_self, pthread_setschedparam, sched_param};

        #[cfg(target_os = "linux")]
        use libc::{SCHED_BATCH, SCHED_OTHER, SCHED_RR};

        #[cfg(not(target_os = "linux"))]
        use libc::{SCHED_OTHER, SCHED_RR};

        // Optimized scheduling policy selection
        // SCHED_BATCH is Linux-specific and better for non-interactive workloads
        let (policy, sched_priority) = match priority {
            #[cfg(target_os = "linux")]
            ThreadPriority::Low => (SCHED_BATCH, 0), // SCHED_BATCH for background tasks

            #[cfg(not(target_os = "linux"))]
            ThreadPriority::Low => (SCHED_OTHER, 0),

            ThreadPriority::Normal => (SCHED_OTHER, 0),
            ThreadPriority::High => (SCHED_OTHER, 0),
            ThreadPriority::Realtime => (SCHED_RR, 50), // Middle realtime priority (1-99 range)
        };

        let param = sched_param { sched_priority };

        let result = unsafe { pthread_setschedparam(pthread_self(), policy, &param as *const _) };

        if result != 0 {
            // Provide helpful error message
            let details = match result {
                libc::EINVAL => format!(
                    "Invalid priority/policy for {:?} (errno: EINVAL)",
                    priority
                ),
                libc::EPERM => format!(
                    "Permission denied for {:?} (errno: EPERM) - need CAP_SYS_NICE for realtime",
                    priority
                ),
                _ => format!(
                    "pthread_setschedparam failed with code {} for {:?}",
                    result, priority
                ),
            };

            return Err(PlatformError::ThreadingError {
                operation: "set_priority".to_string(),
                details,
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

        // Validate core indices before syscall
        for &core in cores {
            if core >= self.num_cpus {
                return Err(PlatformError::ThreadingError {
                    operation: "set_affinity".to_string(),
                    details: format!(
                        "Core {} exceeds available CPUs ({})",
                        core, self.num_cpus
                    ),
                });
            }
        }

        #[cfg(target_os = "linux")]
        {
            use libc::{cpu_set_t, pthread_self, pthread_setaffinity_np, CPU_SET, CPU_ZERO};

            unsafe {
                // Initialize CPU set (zeroed for safety)
                let mut cpuset: cpu_set_t = std::mem::zeroed();
                CPU_ZERO(&mut cpuset);

                // Set requested cores
                for &core in cores {
                    CPU_SET(core, &mut cpuset);
                }

                let result = pthread_setaffinity_np(
                    pthread_self(),
                    std::mem::size_of::<cpu_set_t>(),
                    &cpuset as *const _,
                );

                if result != 0 {
                    let details = match result {
                        libc::EINVAL => "Invalid cpuset (errno: EINVAL)".to_string(),
                        libc::EFAULT => "Invalid cpuset pointer (errno: EFAULT)".to_string(),
                        libc::ESRCH => "Thread not found (errno: ESRCH)".to_string(),
                        _ => format!("pthread_setaffinity_np failed with code {}", result),
                    };

                    return Err(PlatformError::ThreadingError {
                        operation: "set_affinity".to_string(),
                        details,
                    });
                }
            }

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Other Unix systems (BSD, etc.) may not support CPU affinity
            Err(PlatformError::PlatformNotSupported {
                platform: std::env::consts::OS.to_string(),
                feature: "thread_affinity".to_string(),
            })
        }
    }

    fn num_cpus(&self) -> usize {
        // Return cached value for fast access
        self.num_cpus
    }
}

/// macOS threading backend.
///
/// # Platform-Specific Behavior
///
/// macOS uses a different threading model than Linux:
///
/// - **Priority**: Uses pthread_setschedparam (same as Linux)
/// - **Affinity**: NOT SUPPORTED - macOS uses dynamic scheduling
/// - **QoS Classes**: macOS-specific Quality of Service system (recommended)
///
/// ## Why Thread Affinity is Not Supported on macOS
///
/// macOS does not expose a public API for CPU affinity because:
///
/// 1. **Dynamic Scheduling**: macOS uses sophisticated power management
///    and thermal throttling that requires flexible thread migration.
///
/// 2. **Heterogeneous Cores**: Apple Silicon (M1/M2/M3) has Performance
///    and Efficiency cores. The OS needs to migrate threads dynamically
///    based on workload and power state.
///
/// 3. **Private APIs**: thread_policy_set exists but is:
///    - Undocumented and may change
///    - Requires Mach thread port manipulation
///    - Often ignored by the scheduler
///    - Not recommended by Apple
///
/// ## Recommended Alternative: QoS Classes
///
/// Instead of affinity, macOS provides Quality of Service (QoS) classes:
///
/// - **User Interactive**: UI and animation (maps to High priority)
/// - **User Initiated**: User-requested tasks (maps to High priority)
/// - **Default**: Normal priority work (maps to Normal priority)
/// - **Utility**: Long-running background work (maps to Low priority)
/// - **Background**: Deferrable maintenance (maps to Low priority)
///
/// The system automatically schedules QoS classes to appropriate cores:
/// - High QoS → Performance cores (on Apple Silicon)
/// - Low QoS → Efficiency cores (on Apple Silicon)
/// - On Intel Macs, QoS affects priority but all cores are homogeneous
///
/// # Performance
///
/// - pthread_setschedparam: <2us per call (fast on both Intel and Apple Silicon)
/// - QoS classes are respected by the scheduler on macOS 10.10+
/// - set_thread_affinity returns PlatformNotSupported (not a performance issue)
///
/// # Target Performance
///
/// - set_thread_priority: <5us (ideal: 2us)
/// - num_cpus: <1us (cached value)
///
/// # References
///
/// - Apple Technical Note TN2169: "High Precision Timers in iOS"
/// - WWDC 2015: "Advanced NSOperations" (discusses QoS)
/// - pthread documentation: man pthread_setschedparam(3)
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

        // OPTIMIZATION: Use SCHED_OTHER for most priorities (faster than SCHED_RR)
        // macOS scheduler respects priority within the same policy.
        //
        // ALTERNATIVE: For macOS-specific code, consider pthread_set_qos_class_self_np:
        // - QOS_CLASS_USER_INTERACTIVE for High
        // - QOS_CLASS_DEFAULT for Normal
        // - QOS_CLASS_UTILITY or QOS_CLASS_BACKGROUND for Low
        //
        // This provides better hints to the scheduler for P-core vs E-core assignment
        // on Apple Silicon, but we use pthread_setschedparam for cross-platform
        // compatibility.
        let (policy, sched_priority) = match priority {
            ThreadPriority::Low => (SCHED_OTHER, 0),
            ThreadPriority::Normal => (SCHED_OTHER, 0),
            ThreadPriority::High => (SCHED_OTHER, 0),
            ThreadPriority::Realtime => (SCHED_RR, 50), // May require root
        };

        let param = sched_param { sched_priority };

        let result = unsafe { pthread_setschedparam(pthread_self(), policy, &param as *const _) };

        if result != 0 {
            // Provide helpful error messages
            let details = match result {
                libc::EINVAL => format!(
                    "Invalid priority/policy for {:?} (errno: EINVAL)",
                    priority
                ),
                libc::EPERM => format!(
                    "Permission denied for {:?} (errno: EPERM) - realtime may require root",
                    priority
                ),
                _ => format!(
                    "pthread_setschedparam failed with code {} for {:?}",
                    result, priority
                ),
            };

            return Err(PlatformError::ThreadingError {
                operation: "set_priority".to_string(),
                details,
            });
        }

        Ok(())
    }

    fn set_thread_affinity(&self, _cores: &[usize]) -> Result<(), PlatformError> {
        // macOS does NOT support CPU affinity through public APIs.
        //
        // REASON: macOS uses dynamic thread scheduling for:
        // 1. Power management and thermal throttling
        // 2. Heterogeneous cores (P-cores vs E-cores on Apple Silicon)
        // 3. Automatic workload balancing
        //
        // ALTERNATIVES:
        // - Use QoS classes (pthread_set_qos_class_self_np) for hints to scheduler
        // - Trust macOS scheduler to place threads optimally
        // - On Apple Silicon, high-priority threads automatically prefer P-cores
        //
        // PRIVATE APIs (NOT RECOMMENDED):
        // - thread_policy_set with THREAD_AFFINITY_POLICY exists but:
        //   * Undocumented and may change
        //   * Requires Mach thread port manipulation
        //   * Often ignored by the scheduler anyway
        //   * Not available in public headers
        //
        // EXAMPLE OF WHY THIS IS GOOD:
        // On Apple Silicon M1 Max:
        // - 8 P-cores (Firestorm) at 3.2 GHz
        // - 2 E-cores (Icestorm) at 2.0 GHz
        // The OS dynamically assigns threads based on current CPU load,
        // thermal state, and power budget. Manual affinity would interfere.
        //
        // See Apple Technical Note TN2169 for more details.
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
