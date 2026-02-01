//! Threading abstractions for thread priority and affinity.
//!
//! This module provides platform-independent control over thread scheduling
//! parameters, which is important for real-time game engines.

use crate::PlatformError;

/// Thread priority levels.
///
/// These map to platform-specific priority values:
/// - Windows: THREAD_PRIORITY_* constants
/// - Unix: sched_priority values
/// - macOS: mach thread policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreadPriority {
    /// Lowest priority - for background tasks
    Low,
    /// Normal priority - default
    Normal,
    /// High priority - for game logic
    High,
    /// Realtime priority - for audio/rendering (requires elevated privileges)
    Realtime,
}

/// Trait for platform-specific threading backends.
pub trait ThreadingBackend: Send + Sync {
    /// Set the priority of the current thread.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Insufficient permissions (especially for Realtime)
    /// - Platform doesn't support the requested priority
    ///
    /// # Platform Notes
    ///
    /// - **Windows**: Requires no special privileges for Low/Normal/High.
    ///   Realtime requires admin/elevated process.
    /// - **Linux**: Requires CAP_SYS_NICE capability or root for realtime priorities.
    /// - **macOS**: Realtime priority may require root.
    fn set_thread_priority(&self, priority: ThreadPriority) -> Result<(), PlatformError>;

    /// Set CPU affinity for the current thread.
    ///
    /// The `cores` parameter specifies which CPU cores this thread can run on.
    /// Core indices are 0-based (0 = first core, 1 = second core, etc.).
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Invalid core indices (>= num CPUs)
    /// - Insufficient permissions
    /// - Platform doesn't support affinity
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Pin thread to cores 0 and 1
    /// backend.set_thread_affinity(&[0, 1])?;
    /// ```
    fn set_thread_affinity(&self, cores: &[usize]) -> Result<(), PlatformError>;

    /// Get the number of logical CPU cores available.
    fn num_cpus(&self) -> usize {
        num_cpus_impl()
    }
}

// Platform-specific implementations
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

/// Create a threading backend for the current platform.
pub fn create_threading_backend() -> Result<Box<dyn ThreadingBackend>, PlatformError> {
    #[cfg(windows)]
    return Ok(Box::new(windows::WindowsThreading::new()?));

    #[cfg(all(unix, not(target_os = "macos")))]
    return Ok(Box::new(unix::UnixThreading::new()?));

    #[cfg(target_os = "macos")]
    return Ok(Box::new(unix::MacOsThreading::new()?));

    #[cfg(not(any(windows, unix)))]
    return Err(PlatformError::PlatformNotSupported {
        platform: std::env::consts::OS.to_string(),
        feature: "threading".to_string(),
    });
}

// Helper function to get number of CPUs
fn num_cpus_impl() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threading_backend_creation() {
        let backend = create_threading_backend();
        assert!(backend.is_ok());
    }

    #[test]
    fn test_set_normal_priority() {
        let backend = create_threading_backend().unwrap();
        let result = backend.set_thread_priority(ThreadPriority::Normal);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_low_priority() {
        let backend = create_threading_backend().unwrap();
        let result = backend.set_thread_priority(ThreadPriority::Low);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_high_priority() {
        let backend = create_threading_backend().unwrap();
        let result = backend.set_thread_priority(ThreadPriority::High);
        // May fail without privileges, so just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_num_cpus() {
        let backend = create_threading_backend().unwrap();
        let num = backend.num_cpus();
        assert!(num > 0);
        assert!(num <= 256); // Sanity check
    }

    #[test]
    fn test_set_affinity_single_core() {
        let backend = create_threading_backend().unwrap();
        let result = backend.set_thread_affinity(&[0]);
        // May fail on some systems, so just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_priority_ordering() {
        assert!(ThreadPriority::Low < ThreadPriority::Normal);
        assert!(ThreadPriority::Normal < ThreadPriority::High);
        assert!(ThreadPriority::High < ThreadPriority::Realtime);
    }
}
