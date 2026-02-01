//! Performance budget tracking and violation detection.
//!
//! This module provides runtime performance budget enforcement, allowing developers
//! to set time budgets for different scopes and receive warnings when exceeded.
//!
//! # Example
//!
//! ```rust
//! use engine_observability::budgets::{BudgetTracker, BudgetViolation};
//! use std::time::Duration;
//!
//! let mut tracker = BudgetTracker::new();
//!
//! // Set budgets for different scopes
//! tracker.set_budget("physics", Duration::from_millis(5));
//! tracker.set_budget("rendering", Duration::from_millis(8));
//!
//! // Check if actual time exceeds budget
//! let actual = Duration::from_millis(6);
//! if let Some(violation) = tracker.check("physics", actual, 42) {
//!     println!("Budget exceeded in frame {}: {:.2}ms (budget: {:.2}ms)",
//!         violation.frame,
//!         violation.actual.as_secs_f32() * 1000.0,
//!         violation.budget.as_secs_f32() * 1000.0
//!     );
//! }
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Tracks performance budgets and detects violations.
///
/// BudgetTracker allows you to set time budgets for named scopes (e.g., "physics", "rendering")
/// and checks if actual execution times exceed those budgets. All violations are recorded
/// for later analysis.
#[derive(Debug, Clone)]
pub struct BudgetTracker {
    /// Map of scope names to their time budgets
    budgets: HashMap<String, Duration>,
    /// History of all budget violations
    violations: Vec<BudgetViolation>,
}

/// A record of a single budget violation.
///
/// Contains all information needed to diagnose performance issues:
/// - Which scope exceeded its budget
/// - By how much (actual vs. budget)
/// - When it happened (frame number and timestamp)
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetViolation {
    /// The name of the scope that exceeded its budget
    pub scope: String,
    /// The actual time taken
    pub actual: Duration,
    /// The budget that was exceeded
    pub budget: Duration,
    /// The frame number when the violation occurred
    pub frame: usize,
    /// The timestamp when the violation was detected
    pub timestamp: Instant,
}

impl BudgetTracker {
    /// Creates a new BudgetTracker with no budgets set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::budgets::BudgetTracker;
    ///
    /// let tracker = BudgetTracker::new();
    /// assert_eq!(tracker.get_violations().len(), 0);
    /// ```
    pub fn new() -> Self {
        Self { budgets: HashMap::new(), violations: Vec::new() }
    }

    /// Sets a performance budget for a named scope.
    ///
    /// If a budget already exists for this scope, it will be replaced.
    ///
    /// # Arguments
    ///
    /// * `scope` - The name of the scope (e.g., "physics", "rendering")
    /// * `duration` - The maximum allowed time for this scope
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::budgets::BudgetTracker;
    /// use std::time::Duration;
    ///
    /// let mut tracker = BudgetTracker::new();
    /// tracker.set_budget("physics", Duration::from_millis(5));
    /// ```
    pub fn set_budget(&mut self, scope: &str, duration: Duration) {
        self.budgets.insert(scope.to_string(), duration);
    }

    /// Checks if the actual duration exceeds the budget for a scope.
    ///
    /// If the scope has a budget set and the actual duration exceeds it,
    /// a BudgetViolation is created, stored, and returned. If there's no
    /// budget set for this scope, or if the actual time is within budget,
    /// returns None.
    ///
    /// # Arguments
    ///
    /// * `scope` - The name of the scope to check
    /// * `duration` - The actual time taken
    /// * `frame` - The current frame number
    ///
    /// # Returns
    ///
    /// `Some(BudgetViolation)` if the budget was exceeded, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::budgets::BudgetTracker;
    /// use std::time::Duration;
    ///
    /// let mut tracker = BudgetTracker::new();
    /// tracker.set_budget("physics", Duration::from_millis(5));
    ///
    /// // Within budget - no violation
    /// let result = tracker.check("physics", Duration::from_millis(4), 1);
    /// assert!(result.is_none());
    ///
    /// // Exceeds budget - violation recorded
    /// let result = tracker.check("physics", Duration::from_millis(6), 2);
    /// assert!(result.is_some());
    /// ```
    pub fn check(
        &mut self,
        scope: &str,
        duration: Duration,
        frame: usize,
    ) -> Option<BudgetViolation> {
        // Get the budget for this scope, if one exists
        let budget = self.budgets.get(scope)?;

        // Check if we exceeded the budget
        if duration > *budget {
            let violation = BudgetViolation {
                scope: scope.to_string(),
                actual: duration,
                budget: *budget,
                frame,
                timestamp: Instant::now(),
            };

            // Store the violation for history
            self.violations.push(violation.clone());

            Some(violation)
        } else {
            None
        }
    }

    /// Returns a reference to all recorded violations.
    ///
    /// Violations are stored in chronological order (oldest first).
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::budgets::BudgetTracker;
    /// use std::time::Duration;
    ///
    /// let mut tracker = BudgetTracker::new();
    /// tracker.set_budget("physics", Duration::from_millis(5));
    /// tracker.check("physics", Duration::from_millis(6), 1);
    ///
    /// assert_eq!(tracker.get_violations().len(), 1);
    /// ```
    pub fn get_violations(&self) -> &[BudgetViolation] {
        &self.violations
    }

    /// Clears all recorded violations.
    ///
    /// This does not affect the budget settings, only the violation history.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::budgets::BudgetTracker;
    /// use std::time::Duration;
    ///
    /// let mut tracker = BudgetTracker::new();
    /// tracker.set_budget("physics", Duration::from_millis(5));
    /// tracker.check("physics", Duration::from_millis(6), 1);
    ///
    /// assert_eq!(tracker.get_violations().len(), 1);
    ///
    /// tracker.clear_violations();
    /// assert_eq!(tracker.get_violations().len(), 0);
    /// ```
    pub fn clear_violations(&mut self) {
        self.violations.clear();
    }

    /// Gets the budget for a specific scope, if set.
    ///
    /// # Arguments
    ///
    /// * `scope` - The name of the scope
    ///
    /// # Returns
    ///
    /// `Some(Duration)` if a budget is set for this scope, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::budgets::BudgetTracker;
    /// use std::time::Duration;
    ///
    /// let mut tracker = BudgetTracker::new();
    /// tracker.set_budget("physics", Duration::from_millis(5));
    ///
    /// assert_eq!(tracker.get_budget("physics"), Some(Duration::from_millis(5)));
    /// assert_eq!(tracker.get_budget("unknown"), None);
    /// ```
    pub fn get_budget(&self, scope: &str) -> Option<Duration> {
        self.budgets.get(scope).copied()
    }

    /// Returns the total number of violations recorded.
    ///
    /// # Example
    ///
    /// ```rust
    /// use engine_observability::budgets::BudgetTracker;
    /// use std::time::Duration;
    ///
    /// let mut tracker = BudgetTracker::new();
    /// tracker.set_budget("physics", Duration::from_millis(5));
    /// tracker.check("physics", Duration::from_millis(6), 1);
    /// tracker.check("physics", Duration::from_millis(7), 2);
    ///
    /// assert_eq!(tracker.violation_count(), 2);
    /// ```
    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_new_tracker_has_no_budgets() {
        let tracker = BudgetTracker::new();
        assert_eq!(tracker.get_violations().len(), 0);
        assert_eq!(tracker.violation_count(), 0);
    }

    #[test]
    fn test_set_budget() {
        let mut tracker = BudgetTracker::new();
        tracker.set_budget("physics", Duration::from_millis(5));

        assert_eq!(tracker.get_budget("physics"), Some(Duration::from_millis(5)));
        assert_eq!(tracker.get_budget("unknown"), None);
    }

    #[test]
    fn test_budget_replacement() {
        let mut tracker = BudgetTracker::new();
        tracker.set_budget("physics", Duration::from_millis(5));
        tracker.set_budget("physics", Duration::from_millis(10));

        assert_eq!(tracker.get_budget("physics"), Some(Duration::from_millis(10)));
    }

    #[test]
    fn test_no_violation_within_budget() {
        let mut tracker = BudgetTracker::new();
        tracker.set_budget("physics", Duration::from_millis(5));

        let result = tracker.check("physics", Duration::from_millis(4), 1);
        assert!(result.is_none());
        assert_eq!(tracker.violation_count(), 0);
    }

    #[test]
    fn test_no_violation_exact_budget() {
        let mut tracker = BudgetTracker::new();
        tracker.set_budget("physics", Duration::from_millis(5));

        let result = tracker.check("physics", Duration::from_millis(5), 1);
        assert!(result.is_none());
        assert_eq!(tracker.violation_count(), 0);
    }

    #[test]
    fn test_violation_exceeds_budget() {
        let mut tracker = BudgetTracker::new();
        tracker.set_budget("physics", Duration::from_millis(5));

        let result = tracker.check("physics", Duration::from_millis(6), 42);
        assert!(result.is_some());

        let violation = result.unwrap();
        assert_eq!(violation.scope, "physics");
        assert_eq!(violation.actual, Duration::from_millis(6));
        assert_eq!(violation.budget, Duration::from_millis(5));
        assert_eq!(violation.frame, 42);

        assert_eq!(tracker.violation_count(), 1);
    }

    #[test]
    fn test_no_violation_without_budget() {
        let mut tracker = BudgetTracker::new();

        let result = tracker.check("physics", Duration::from_millis(999), 1);
        assert!(result.is_none());
        assert_eq!(tracker.violation_count(), 0);
    }

    #[test]
    fn test_multiple_violations_recorded() {
        let mut tracker = BudgetTracker::new();
        tracker.set_budget("physics", Duration::from_millis(5));

        tracker.check("physics", Duration::from_millis(6), 1);
        tracker.check("physics", Duration::from_millis(7), 2);
        tracker.check("physics", Duration::from_millis(8), 3);

        assert_eq!(tracker.violation_count(), 3);

        let violations = tracker.get_violations();
        assert_eq!(violations[0].frame, 1);
        assert_eq!(violations[1].frame, 2);
        assert_eq!(violations[2].frame, 3);
    }

    #[test]
    fn test_clear_violations() {
        let mut tracker = BudgetTracker::new();
        tracker.set_budget("physics", Duration::from_millis(5));

        tracker.check("physics", Duration::from_millis(6), 1);
        tracker.check("physics", Duration::from_millis(7), 2);

        assert_eq!(tracker.violation_count(), 2);

        tracker.clear_violations();

        assert_eq!(tracker.violation_count(), 0);
        assert_eq!(tracker.get_violations().len(), 0);

        // Budget should still be set
        assert_eq!(tracker.get_budget("physics"), Some(Duration::from_millis(5)));
    }

    #[test]
    fn test_multiple_scopes() {
        let mut tracker = BudgetTracker::new();
        tracker.set_budget("physics", Duration::from_millis(5));
        tracker.set_budget("rendering", Duration::from_millis(8));

        tracker.check("physics", Duration::from_millis(6), 1);
        tracker.check("rendering", Duration::from_millis(9), 1);

        assert_eq!(tracker.violation_count(), 2);

        let violations = tracker.get_violations();
        assert!(violations.iter().any(|v| v.scope == "physics"));
        assert!(violations.iter().any(|v| v.scope == "rendering"));
    }

    #[test]
    fn test_violation_timestamp_ordering() {
        let mut tracker = BudgetTracker::new();
        tracker.set_budget("physics", Duration::from_millis(5));

        tracker.check("physics", Duration::from_millis(6), 1);
        thread::sleep(Duration::from_millis(10));
        tracker.check("physics", Duration::from_millis(6), 2);

        let violations = tracker.get_violations();
        assert!(violations[0].timestamp < violations[1].timestamp);
    }

    #[test]
    fn test_violation_clone() {
        let violation = BudgetViolation {
            scope: "test".to_string(),
            actual: Duration::from_millis(10),
            budget: Duration::from_millis(5),
            frame: 42,
            timestamp: Instant::now(),
        };

        let cloned = violation.clone();
        assert_eq!(cloned.scope, violation.scope);
        assert_eq!(cloned.actual, violation.actual);
        assert_eq!(cloned.budget, violation.budget);
        assert_eq!(cloned.frame, violation.frame);
    }

    #[test]
    fn test_default_tracker() {
        let tracker = BudgetTracker::default();
        assert_eq!(tracker.violation_count(), 0);
    }
}
