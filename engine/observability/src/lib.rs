//! Engine Observability
//!
//! Provides performance monitoring and profiling infrastructure:
//! - Budget tracking and violation detection
//! - Metrics collection (CPU, GPU, memory, network)
//! - Frame profiling
//! - External profiler integration (Tracy, Puffin)
//! - Performance validation
//! - Prometheus metrics HTTP endpoint
//!
//! # Performance Budget Example
//!
//! ```rust
//! use engine_observability::{Profiler, ProfilerConfig};
//! use std::time::Duration;
//!
//! let mut profiler = Profiler::new(ProfilerConfig::default());
//!
//! // Set performance budgets
//! profiler.set_budget("physics", Duration::from_millis(5));
//! profiler.set_budget("rendering", Duration::from_millis(8));
//!
//! // Begin frame
//! profiler.begin_frame();
//!
//! // Profile scopes
//! {
//!     let _guard = profiler.scope("physics");
//!     // Physics simulation here
//! }
//!
//! // End frame - warnings logged if budgets exceeded
//! profiler.end_frame();
//! ```
//!
//! # Prometheus Metrics Example
//!
//! ```no_run
//! use engine_observability::metrics::{MetricsRegistry, start_metrics_server};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Start Prometheus metrics HTTP server
//!     let metrics_handle = tokio::spawn(async {
//!         start_metrics_server("0.0.0.0:9090").await.unwrap();
//!     });
//!
//!     // Record metrics in your game loop
//!     let registry = MetricsRegistry::new();
//!     registry.record_frame_time(16.7); // 60 FPS
//!     registry.set_entity_count(1000);
//!     registry.set_connected_clients(50);
//!
//!     // Metrics are now available at http://0.0.0.0:9090/metrics
//!     // Can be scraped by Prometheus, Grafana, etc.
//! }
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod admin;
pub mod budgets;
pub mod metrics;

// Re-export commonly used types
pub use budgets::{BudgetTracker, BudgetViolation};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use tracing::warn;

/// Configuration for the profiler.
#[derive(Debug, Clone)]
pub struct ProfilerConfig {
    /// Whether profiling is enabled
    pub enabled: bool,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self { enabled: cfg!(debug_assertions) }
    }
}

/// Main profiler for tracking performance metrics and budgets.
///
/// The profiler provides:
/// - Scope-based timing
/// - Performance budget tracking
/// - Automatic warnings when budgets are exceeded
/// - Frame-level metrics collection
///
/// # Example
///
/// ```rust
/// use engine_observability::{Profiler, ProfilerConfig};
/// use std::time::Duration;
///
/// let mut profiler = Profiler::new(ProfilerConfig::default());
/// profiler.set_budget("game_loop", Duration::from_millis(16));
///
/// profiler.begin_frame();
/// {
///     let _guard = profiler.scope("game_loop");
///     // Game logic here
/// }
/// profiler.end_frame();
/// ```
pub struct Profiler {
    config: ProfilerConfig,
    inner: Rc<RefCell<ProfilerInner>>,
}

struct ProfilerInner {
    budget_tracker: BudgetTracker,
    current_frame: usize,
    frame_start: Option<Instant>,
}

impl Profiler {
    /// Creates a new profiler with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the profiler
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::{Profiler, ProfilerConfig};
    ///
    /// let profiler = Profiler::new(ProfilerConfig::default());
    /// ```
    pub fn new(config: ProfilerConfig) -> Self {
        Self {
            config,
            inner: Rc::new(RefCell::new(ProfilerInner {
                budget_tracker: BudgetTracker::new(),
                current_frame: 0,
                frame_start: None,
            })),
        }
    }

    /// Marks the beginning of a new frame.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::{Profiler, ProfilerConfig};
    ///
    /// let mut profiler = Profiler::new(ProfilerConfig::default());
    /// profiler.begin_frame();
    /// ```
    pub fn begin_frame(&mut self) {
        self.inner.borrow_mut().frame_start = Some(Instant::now());
    }

    /// Marks the end of the current frame.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::{Profiler, ProfilerConfig};
    ///
    /// let mut profiler = Profiler::new(ProfilerConfig::default());
    /// profiler.begin_frame();
    /// profiler.end_frame();
    /// ```
    pub fn end_frame(&mut self) {
        let mut inner = self.inner.borrow_mut();
        inner.current_frame += 1;
        inner.frame_start = None;
    }

    /// Creates a scope guard that measures the duration of a scope.
    ///
    /// When the guard is dropped, it records the timing and checks if the duration
    /// exceeds the budget for this scope. If so, it logs a warning.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the scope
    ///
    /// # Returns
    ///
    /// A scope guard that will measure timing when dropped.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::{Profiler, ProfilerConfig};
    /// use std::time::Duration;
    ///
    /// let mut profiler = Profiler::new(ProfilerConfig::default());
    /// profiler.set_budget("physics", Duration::from_millis(5));
    ///
    /// {
    ///     let _guard = profiler.scope("physics");
    ///     // Physics code here
    /// } // Guard dropped here, timing measured and budget checked
    /// ```
    pub fn scope(&mut self, name: &str) -> ScopeGuard {
        let frame = self.inner.borrow().current_frame;
        ScopeGuard::new(name.to_string(), frame, self.config.enabled, Rc::clone(&self.inner))
    }

    /// Sets a performance budget for a named scope.
    ///
    /// When a scope exceeds its budget, a warning is logged with structured data.
    ///
    /// # Arguments
    ///
    /// * `scope` - The name of the scope
    /// * `duration` - The maximum allowed time for this scope
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::{Profiler, ProfilerConfig};
    /// use std::time::Duration;
    ///
    /// let mut profiler = Profiler::new(ProfilerConfig::default());
    /// profiler.set_budget("physics", Duration::from_millis(5));
    /// profiler.set_budget("rendering", Duration::from_millis(8));
    /// ```
    pub fn set_budget(&mut self, scope: &str, duration: Duration) {
        self.inner.borrow_mut().budget_tracker.set_budget(scope, duration);
    }

    /// Gets all recorded budget violations.
    ///
    /// # Returns
    ///
    /// A vector of all budget violations that have been recorded.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::{Profiler, ProfilerConfig};
    ///
    /// let profiler = Profiler::new(ProfilerConfig::default());
    /// let violations = profiler.get_violations();
    /// assert_eq!(violations.len(), 0);
    /// ```
    pub fn get_violations(&self) -> Vec<BudgetViolation> {
        self.inner.borrow().budget_tracker.get_violations().to_vec()
    }

    /// Clears all recorded budget violations.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::{Profiler, ProfilerConfig};
    ///
    /// let mut profiler = Profiler::new(ProfilerConfig::default());
    /// profiler.clear_violations();
    /// ```
    pub fn clear_violations(&mut self) {
        self.inner.borrow_mut().budget_tracker.clear_violations();
    }

    /// Gets the current frame number.
    ///
    /// # Returns
    ///
    /// The current frame number (0-indexed).
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::{Profiler, ProfilerConfig};
    ///
    /// let mut profiler = Profiler::new(ProfilerConfig::default());
    /// assert_eq!(profiler.current_frame(), 0);
    ///
    /// profiler.begin_frame();
    /// profiler.end_frame();
    /// assert_eq!(profiler.current_frame(), 1);
    /// ```
    pub fn current_frame(&self) -> usize {
        self.inner.borrow().current_frame
    }
}

/// RAII guard that measures the duration of a scope.
///
/// When dropped, the guard checks if the duration exceeds the budget and logs a warning if so.
/// Uses interior mutability to allow nested scopes.
pub struct ScopeGuard {
    name: String,
    start: Instant,
    frame: usize,
    enabled: bool,
    inner: Rc<RefCell<ProfilerInner>>,
}

impl ScopeGuard {
    fn new(name: String, frame: usize, enabled: bool, inner: Rc<RefCell<ProfilerInner>>) -> Self {
        Self { name, start: Instant::now(), frame, enabled, inner }
    }

    /// Get the elapsed time for this scope.
    ///
    /// This is useful for manually checking timing without dropping the guard.
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Get the scope name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the frame number.
    pub fn frame(&self) -> usize {
        self.frame
    }
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        if !self.enabled {
            return;
        }

        let duration = self.start.elapsed();

        // Check if we exceeded the budget
        let mut inner = self.inner.borrow_mut();
        if let Some(violation) = inner.budget_tracker.check(&self.name, duration, self.frame) {
            warn!(
                scope = %violation.scope,
                actual_ms = violation.actual.as_secs_f32() * 1000.0,
                budget_ms = violation.budget.as_secs_f32() * 1000.0,
                frame = violation.frame,
                "Performance budget exceeded"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_profiler_creation() {
        let profiler = Profiler::new(ProfilerConfig::default());
        assert_eq!(profiler.current_frame(), 0);
    }

    #[test]
    fn test_frame_counting() {
        let mut profiler = Profiler::new(ProfilerConfig::default());

        assert_eq!(profiler.current_frame(), 0);

        profiler.begin_frame();
        profiler.end_frame();
        assert_eq!(profiler.current_frame(), 1);

        profiler.begin_frame();
        profiler.end_frame();
        assert_eq!(profiler.current_frame(), 2);
    }

    #[test]
    fn test_set_budget() {
        let mut profiler = Profiler::new(ProfilerConfig::default());
        profiler.set_budget("physics", Duration::from_millis(5));

        // Budget is set (verified implicitly by scope checks)
        assert_eq!(profiler.get_violations().len(), 0);
    }

    #[test]
    fn test_scope_within_budget() {
        let mut profiler = Profiler::new(ProfilerConfig::default());
        profiler.set_budget("test", Duration::from_millis(100));

        {
            let _guard = profiler.scope("test");
            // Very short operation
        }

        assert_eq!(profiler.get_violations().len(), 0);
    }

    #[test]
    fn test_scope_exceeds_budget() {
        let mut profiler = Profiler::new(ProfilerConfig::default());
        profiler.set_budget("test", Duration::from_millis(1));

        {
            let _guard = profiler.scope("test");
            thread::sleep(Duration::from_millis(5));
        }

        // Should have recorded a violation
        assert_eq!(profiler.get_violations().len(), 1);

        let violation = &profiler.get_violations()[0];
        assert_eq!(violation.scope, "test");
        assert!(violation.actual > Duration::from_millis(1));
    }

    #[test]
    fn test_multiple_scopes() {
        let mut profiler = Profiler::new(ProfilerConfig::default());
        profiler.set_budget("fast", Duration::from_millis(100));
        profiler.set_budget("slow", Duration::from_millis(1));

        {
            let _guard1 = profiler.scope("fast");
            // Fast operation
        }

        {
            let _guard2 = profiler.scope("slow");
            thread::sleep(Duration::from_millis(5));
        }

        // Only the slow scope should have violated
        assert_eq!(profiler.get_violations().len(), 1);
        assert_eq!(profiler.get_violations()[0].scope, "slow");
    }

    #[test]
    fn test_clear_violations() {
        let mut profiler = Profiler::new(ProfilerConfig::default());
        profiler.set_budget("test", Duration::from_millis(1));

        {
            let _guard = profiler.scope("test");
            thread::sleep(Duration::from_millis(5));
        }

        assert_eq!(profiler.get_violations().len(), 1);

        profiler.clear_violations();
        assert_eq!(profiler.get_violations().len(), 0);
    }

    #[test]
    fn test_disabled_profiler() {
        let mut profiler = Profiler::new(ProfilerConfig { enabled: false });
        profiler.set_budget("test", Duration::from_millis(1));

        {
            let _guard = profiler.scope("test");
            thread::sleep(Duration::from_millis(5));
        }

        // No violations should be recorded when disabled
        assert_eq!(profiler.get_violations().len(), 0);
    }

    #[test]
    fn test_nested_scopes() {
        let mut profiler = Profiler::new(ProfilerConfig::default());
        profiler.set_budget("outer", Duration::from_millis(1));
        profiler.set_budget("inner", Duration::from_millis(1));

        {
            let _outer = profiler.scope("outer");
            thread::sleep(Duration::from_millis(2));
            {
                let _inner = profiler.scope("inner");
                thread::sleep(Duration::from_millis(2));
            }
        }

        // Both should have violations
        assert_eq!(profiler.get_violations().len(), 2);
    }
}
