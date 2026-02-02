//! Interest Management Telemetry
//!
//! Production monitoring and observability for interest management.
//! Provides metrics, alerting, and performance tracking for AAA deployments.
//!
//! # Metrics Tracked
//!
//! - **Performance**: Visibility calculation duration, grid update time
//! - **Scale**: Total entities, total clients, average visible entities
//! - **Efficiency**: Bandwidth reduction, grid cell utilization, cache hit rate
//! - **Health**: Failed calculations, stale data warnings, performance degradation
//!
//! # Integration
//!
//! ```
//! use engine_interest::{InterestManager, InterestMetrics};
//!
//! let mut manager = InterestManager::new(50.0);
//! let mut metrics = InterestMetrics::new();
//!
//! // During game loop
//! let start = std::time::Instant::now();
//! let visible = manager.calculate_visibility(client_id);
//! metrics.record_visibility_calculation(start.elapsed(), visible.len());
//!
//! // Check health
//! if let Some(alert) = metrics.alert_if_degraded() {
//!     tracing::warn!("Performance alert: {:?}", alert);
//! }
//!
//! // Export to Prometheus
//! let prometheus = metrics.export_prometheus();
//! ```

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Performance alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    /// Warning - performance degraded but within limits
    Warning,
    /// Critical - performance significantly degraded
    Critical,
}

/// Performance alert
#[derive(Debug, Clone)]
pub struct PerformanceAlert {
    /// Alert severity
    pub severity: AlertSeverity,
    /// Human-readable alert message
    pub message: String,
    /// Metric that triggered the alert
    pub metric: String,
    /// Current value
    pub current_value: f64,
    /// Threshold that was exceeded
    pub threshold: f64,
}

/// Histogram for tracking duration distributions
#[derive(Debug, Clone)]
pub struct Histogram {
    samples: VecDeque<Duration>,
    max_samples: usize,
}

impl Histogram {
    /// Create a new histogram with max sample count
    pub fn new(max_samples: usize) -> Self {
        Self { samples: VecDeque::with_capacity(max_samples), max_samples }
    }

    /// Record a duration sample
    pub fn record(&mut self, duration: Duration) {
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(duration);
    }

    /// Get p50 (median)
    pub fn p50(&self) -> Duration {
        self.percentile(0.50)
    }

    /// Get p95
    pub fn p95(&self) -> Duration {
        self.percentile(0.95)
    }

    /// Get p99
    pub fn p99(&self) -> Duration {
        self.percentile(0.99)
    }

    /// Get p99.9
    pub fn p999(&self) -> Duration {
        self.percentile(0.999)
    }

    /// Get average
    pub fn average(&self) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let sum: Duration = self.samples.iter().sum();
        sum / self.samples.len() as u32
    }

    /// Get max
    pub fn max(&self) -> Duration {
        self.samples.iter().max().copied().unwrap_or(Duration::ZERO)
    }

    /// Get min
    pub fn min(&self) -> Duration {
        self.samples.iter().min().copied().unwrap_or(Duration::ZERO)
    }

    /// Get sample count
    pub fn count(&self) -> usize {
        self.samples.len()
    }

    /// Calculate percentile
    fn percentile(&self, p: f64) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }

        let mut sorted: Vec<Duration> = self.samples.iter().copied().collect();
        sorted.sort();

        let index = ((sorted.len() as f64 - 1.0) * p) as usize;
        sorted[index]
    }
}

/// Gauge for tracking current values
#[derive(Debug, Clone)]
pub struct Gauge {
    value: f64,
    history: VecDeque<(Instant, f64)>,
    max_history: usize,
}

impl Gauge {
    /// Create a new gauge
    pub fn new(max_history: usize) -> Self {
        Self { value: 0.0, history: VecDeque::with_capacity(max_history), max_history }
    }

    /// Set gauge value
    pub fn set(&mut self, value: f64) {
        self.value = value;

        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back((Instant::now(), value));
    }

    /// Get current value
    pub fn get(&self) -> f64 {
        self.value
    }

    /// Get average over history
    pub fn average(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.history.iter().map(|(_, v)| v).sum();
        sum / self.history.len() as f64
    }

    /// Get growth rate (value/hour)
    pub fn growth_rate(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }

        let first = self.history.front().unwrap();
        let last = self.history.back().unwrap();

        let value_delta = last.1 - first.1;
        let time_delta = last.0.duration_since(first.0).as_secs_f64() / 3600.0; // hours

        if time_delta == 0.0 {
            0.0
        } else {
            value_delta / time_delta
        }
    }
}

/// Counter for tracking cumulative events
#[derive(Debug, Clone, Copy)]
pub struct Counter {
    value: u64,
}

impl Counter {
    /// Create a new counter
    pub fn new() -> Self {
        Self { value: 0 }
    }

    /// Increment counter
    pub fn increment(&mut self) {
        self.value += 1;
    }

    /// Add to counter
    pub fn add(&mut self, amount: u64) {
        self.value += amount;
    }

    /// Get current value
    pub fn get(&self) -> u64 {
        self.value
    }

    /// Reset to zero
    pub fn reset(&mut self) {
        self.value = 0;
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

/// Interest Management Metrics
///
/// Comprehensive metrics for production monitoring and alerting.
pub struct InterestMetrics {
    // Performance metrics
    /// Visibility calculation duration (per-player)
    pub visibility_calculation_duration: Histogram,
    /// Grid update duration (entire spatial grid rebuild)
    pub grid_update_duration: Histogram,
    /// Entities per visibility calculation
    pub entities_per_calculation: Histogram,

    // Scale metrics
    /// Total entities in spatial grid
    pub total_entities: Gauge,
    /// Total registered clients
    pub total_clients: Gauge,
    /// Average entities visible per client
    pub average_visible_entities: Gauge,

    // Efficiency metrics
    /// Bandwidth reduction percentage (0-100)
    pub bandwidth_saved_percentage: Gauge,
    /// Grid cell utilization (entities per cell)
    pub grid_cell_utilization: Histogram,
    /// Cache hit rate estimate (0-1)
    pub cache_hit_rate: Gauge,

    // Health metrics
    /// Failed calculations (errors, timeouts)
    pub failed_calculations: Counter,
    /// Stale data warnings (data older than threshold)
    pub stale_data_warnings: Counter,
    /// Performance degradation events
    pub performance_degradation_alerts: Counter,
}

impl InterestMetrics {
    /// Create new metrics tracker
    pub fn new() -> Self {
        Self {
            visibility_calculation_duration: Histogram::new(1000),
            grid_update_duration: Histogram::new(100),
            entities_per_calculation: Histogram::new(1000),

            total_entities: Gauge::new(100),
            total_clients: Gauge::new(100),
            average_visible_entities: Gauge::new(100),

            bandwidth_saved_percentage: Gauge::new(100),
            grid_cell_utilization: Histogram::new(100),
            cache_hit_rate: Gauge::new(100),

            failed_calculations: Counter::new(),
            stale_data_warnings: Counter::new(),
            performance_degradation_alerts: Counter::new(),
        }
    }

    /// Record a visibility calculation
    ///
    /// # Arguments
    ///
    /// * `duration` - Time taken to calculate visibility
    /// * `entity_count` - Number of entities returned
    pub fn record_visibility_calculation(&mut self, duration: Duration, entity_count: usize) {
        self.visibility_calculation_duration.record(duration);
        self.entities_per_calculation.record(Duration::from_micros(entity_count as u64));
    }

    /// Record a grid update
    pub fn record_grid_update(&mut self, duration: Duration) {
        self.grid_update_duration.record(duration);
    }

    /// Update scale metrics
    pub fn update_scale_metrics(&mut self, entities: usize, clients: usize, avg_visible: f64) {
        self.total_entities.set(entities as f64);
        self.total_clients.set(clients as f64);
        self.average_visible_entities.set(avg_visible);
    }

    /// Update efficiency metrics
    pub fn update_efficiency_metrics(&mut self, bandwidth_reduction: f64, cache_hit_rate: f64) {
        self.bandwidth_saved_percentage.set(bandwidth_reduction);
        self.cache_hit_rate.set(cache_hit_rate);
    }

    /// Record a failed calculation
    pub fn record_failed_calculation(&mut self) {
        self.failed_calculations.increment();
    }

    /// Record a stale data warning
    pub fn record_stale_data_warning(&mut self) {
        self.stale_data_warnings.increment();
    }

    /// Check for performance degradation and generate alerts
    ///
    /// # Alerting Rules
    ///
    /// - **Warning**: p95 visibility >5ms, p99 >10ms
    /// - **Critical**: p99 visibility >10ms, failed calculations >0.1%
    /// - **Warning**: Memory growth >10% per hour
    /// - **Critical**: Failed calculations >0.1%
    pub fn alert_if_degraded(&mut self) -> Option<PerformanceAlert> {
        // Check p95 visibility calculation time
        let p95_micros = self.visibility_calculation_duration.p95().as_micros() as f64;
        if p95_micros > 5000.0 {
            self.performance_degradation_alerts.increment();
            return Some(PerformanceAlert {
                severity: AlertSeverity::Warning,
                message: format!(
                    "p95 visibility calculation {}µs exceeds 5ms threshold",
                    p95_micros
                ),
                metric: "visibility_calculation_duration_p95".to_string(),
                current_value: p95_micros,
                threshold: 5000.0,
            });
        }

        // Check p99 visibility calculation time
        let p99_micros = self.visibility_calculation_duration.p99().as_micros() as f64;
        if p99_micros > 10000.0 {
            self.performance_degradation_alerts.increment();
            return Some(PerformanceAlert {
                severity: AlertSeverity::Critical,
                message: format!(
                    "p99 visibility calculation {}µs exceeds 10ms threshold",
                    p99_micros
                ),
                metric: "visibility_calculation_duration_p99".to_string(),
                current_value: p99_micros,
                threshold: 10000.0,
            });
        }

        // Check memory growth rate
        let entity_growth = self.total_entities.growth_rate();
        let expected_growth = 0.0; // Stable expected
        if entity_growth > expected_growth * 1.1 {
            // 10% above expected
            self.performance_degradation_alerts.increment();
            return Some(PerformanceAlert {
                severity: AlertSeverity::Warning,
                message: format!(
                    "Entity count growing at {:.1}/hour (unexpected growth)",
                    entity_growth
                ),
                metric: "total_entities_growth_rate".to_string(),
                current_value: entity_growth,
                threshold: expected_growth * 1.1,
            });
        }

        // Check failed calculation rate
        let total_calculations = self.visibility_calculation_duration.count() as f64;
        if total_calculations > 0.0 {
            let failure_rate = self.failed_calculations.get() as f64 / total_calculations;
            if failure_rate > 0.001 {
                // >0.1%
                self.performance_degradation_alerts.increment();
                return Some(PerformanceAlert {
                    severity: AlertSeverity::Critical,
                    message: format!(
                        "Failed calculation rate {:.2}% exceeds 0.1%",
                        failure_rate * 100.0
                    ),
                    metric: "failed_calculation_rate".to_string(),
                    current_value: failure_rate * 100.0,
                    threshold: 0.1,
                });
            }
        }

        None
    }

    /// Export metrics in Prometheus format
    ///
    /// # Format
    ///
    /// ```text
    /// # HELP interest_visibility_calculation_duration_p50 p50 visibility calculation time (µs)
    /// # TYPE interest_visibility_calculation_duration_p50 gauge
    /// interest_visibility_calculation_duration_p50 456.0
    /// ```
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Visibility calculation duration
        output.push_str("# HELP interest_visibility_calculation_duration_p50 p50 visibility calculation time (µs)\n");
        output.push_str("# TYPE interest_visibility_calculation_duration_p50 gauge\n");
        output.push_str(&format!(
            "interest_visibility_calculation_duration_p50 {}\n",
            self.visibility_calculation_duration.p50().as_micros()
        ));

        output.push_str("# HELP interest_visibility_calculation_duration_p95 p95 visibility calculation time (µs)\n");
        output.push_str("# TYPE interest_visibility_calculation_duration_p95 gauge\n");
        output.push_str(&format!(
            "interest_visibility_calculation_duration_p95 {}\n",
            self.visibility_calculation_duration.p95().as_micros()
        ));

        output.push_str("# HELP interest_visibility_calculation_duration_p99 p99 visibility calculation time (µs)\n");
        output.push_str("# TYPE interest_visibility_calculation_duration_p99 gauge\n");
        output.push_str(&format!(
            "interest_visibility_calculation_duration_p99 {}\n",
            self.visibility_calculation_duration.p99().as_micros()
        ));

        output.push_str("# HELP interest_visibility_calculation_duration_avg avg visibility calculation time (µs)\n");
        output.push_str("# TYPE interest_visibility_calculation_duration_avg gauge\n");
        output.push_str(&format!(
            "interest_visibility_calculation_duration_avg {}\n",
            self.visibility_calculation_duration.average().as_micros()
        ));

        // Grid update duration
        output.push_str("# HELP interest_grid_update_duration_p95 p95 grid update time (µs)\n");
        output.push_str("# TYPE interest_grid_update_duration_p95 gauge\n");
        output.push_str(&format!(
            "interest_grid_update_duration_p95 {}\n",
            self.grid_update_duration.p95().as_micros()
        ));

        // Scale metrics
        output.push_str("# HELP interest_total_entities Total entities in spatial grid\n");
        output.push_str("# TYPE interest_total_entities gauge\n");
        output.push_str(&format!("interest_total_entities {}\n", self.total_entities.get()));

        output.push_str("# HELP interest_total_clients Total registered clients\n");
        output.push_str("# TYPE interest_total_clients gauge\n");
        output.push_str(&format!("interest_total_clients {}\n", self.total_clients.get()));

        output.push_str(
            "# HELP interest_average_visible_entities Average entities visible per client\n",
        );
        output.push_str("# TYPE interest_average_visible_entities gauge\n");
        output.push_str(&format!(
            "interest_average_visible_entities {}\n",
            self.average_visible_entities.get()
        ));

        // Efficiency metrics
        output.push_str(
            "# HELP interest_bandwidth_saved_percentage Bandwidth reduction percentage\n",
        );
        output.push_str("# TYPE interest_bandwidth_saved_percentage gauge\n");
        output.push_str(&format!(
            "interest_bandwidth_saved_percentage {}\n",
            self.bandwidth_saved_percentage.get()
        ));

        output.push_str("# HELP interest_cache_hit_rate Cache hit rate (0-1)\n");
        output.push_str("# TYPE interest_cache_hit_rate gauge\n");
        output.push_str(&format!("interest_cache_hit_rate {}\n", self.cache_hit_rate.get()));

        // Health metrics
        output.push_str("# HELP interest_failed_calculations_total Total failed calculations\n");
        output.push_str("# TYPE interest_failed_calculations_total counter\n");
        output.push_str(&format!(
            "interest_failed_calculations_total {}\n",
            self.failed_calculations.get()
        ));

        output.push_str("# HELP interest_stale_data_warnings_total Total stale data warnings\n");
        output.push_str("# TYPE interest_stale_data_warnings_total counter\n");
        output.push_str(&format!(
            "interest_stale_data_warnings_total {}\n",
            self.stale_data_warnings.get()
        ));

        output.push_str(
            "# HELP interest_performance_degradation_alerts_total Total performance alerts\n",
        );
        output.push_str("# TYPE interest_performance_degradation_alerts_total counter\n");
        output.push_str(&format!(
            "interest_performance_degradation_alerts_total {}\n",
            self.performance_degradation_alerts.get()
        ));

        output
    }

    /// Generate a human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Interest Management Metrics:\n\
             \n\
             Performance:\n\
             - Visibility calc p50: {}µs, p95: {}µs, p99: {}µs\n\
             - Grid update p95: {}µs\n\
             \n\
             Scale:\n\
             - Total entities: {:.0}\n\
             - Total clients: {:.0}\n\
             - Avg visible per client: {:.1}\n\
             \n\
             Efficiency:\n\
             - Bandwidth reduction: {:.1}%\n\
             - Cache hit rate: {:.1}%\n\
             \n\
             Health:\n\
             - Failed calculations: {}\n\
             - Stale data warnings: {}\n\
             - Performance alerts: {}\n",
            self.visibility_calculation_duration.p50().as_micros(),
            self.visibility_calculation_duration.p95().as_micros(),
            self.visibility_calculation_duration.p99().as_micros(),
            self.grid_update_duration.p95().as_micros(),
            self.total_entities.get(),
            self.total_clients.get(),
            self.average_visible_entities.get(),
            self.bandwidth_saved_percentage.get(),
            self.cache_hit_rate.get() * 100.0,
            self.failed_calculations.get(),
            self.stale_data_warnings.get(),
            self.performance_degradation_alerts.get(),
        )
    }
}

impl Default for InterestMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_percentiles() {
        let mut hist = Histogram::new(100);

        for i in 1..=100 {
            hist.record(Duration::from_micros(i));
        }

        assert_eq!(hist.count(), 100);
        assert!(hist.p50().as_micros() >= 49 && hist.p50().as_micros() <= 51);
        assert!(hist.p95().as_micros() >= 94 && hist.p95().as_micros() <= 96);
        assert!(hist.p99().as_micros() >= 98 && hist.p99().as_micros() <= 100);
    }

    #[test]
    fn test_gauge_growth_rate() {
        let mut gauge = Gauge::new(100);

        // Simulate linear growth over time
        let start = Instant::now();
        gauge.set(100.0);

        std::thread::sleep(Duration::from_millis(100));
        gauge.set(200.0);

        let growth = gauge.growth_rate();
        assert!(growth > 0.0, "Should show positive growth");
    }

    #[test]
    fn test_counter_operations() {
        let mut counter = Counter::new();

        assert_eq!(counter.get(), 0);

        counter.increment();
        assert_eq!(counter.get(), 1);

        counter.add(10);
        assert_eq!(counter.get(), 11);

        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_metrics_alerting() {
        let mut metrics = InterestMetrics::new();

        // Record fast calculations - no alert
        for _ in 0..100 {
            metrics.record_visibility_calculation(Duration::from_micros(100), 10);
        }

        assert!(metrics.alert_if_degraded().is_none());

        // Record slow calculations - should alert
        for _ in 0..100 {
            metrics.record_visibility_calculation(Duration::from_millis(15), 10);
        }

        let alert = metrics.alert_if_degraded();
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().severity, AlertSeverity::Critical);
    }

    #[test]
    fn test_prometheus_export() {
        let mut metrics = InterestMetrics::new();

        metrics.record_visibility_calculation(Duration::from_micros(500), 10);
        metrics.update_scale_metrics(1000, 100, 25.5);
        metrics.update_efficiency_metrics(95.5, 0.92);

        let prometheus = metrics.export_prometheus();

        assert!(prometheus.contains("interest_visibility_calculation_duration"));
        assert!(prometheus.contains("interest_total_entities 1000"));
        assert!(prometheus.contains("interest_total_clients 100"));
        assert!(prometheus.contains("interest_bandwidth_saved_percentage 95.5"));
    }

    #[test]
    fn test_metrics_summary() {
        let mut metrics = InterestMetrics::new();

        metrics.record_visibility_calculation(Duration::from_micros(500), 10);
        metrics.update_scale_metrics(1000, 100, 25.5);

        let summary = metrics.summary();

        assert!(summary.contains("Total entities: 1000"));
        assert!(summary.contains("Total clients: 100"));
        assert!(summary.contains("Avg visible per client: 25.5"));
    }
}
