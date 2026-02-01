//! Prometheus metrics collection for engine observability
//!
//! This module provides Prometheus-compatible metrics for monitoring:
//! - Frame time and FPS
//! - Server tick rate and duration
//! - Entity count and ECS performance
//! - Network bandwidth and latency
//! - Memory usage
//!
//! # Examples
//!
//! ```no_run
//! use engine_observability::metrics::{MetricsRegistry, start_metrics_server};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Initialize metrics registry
//!     let registry = MetricsRegistry::new();
//!
//!     // Start HTTP server for Prometheus scraping
//!     start_metrics_server("0.0.0.0:9090").await.unwrap();
//!
//!     // Record metrics in your game loop
//!     registry.record_frame_time(16.7); // 60 FPS
//!     registry.increment_entity_count(1);
//! }
//! ```

#[cfg(feature = "metrics")]
use prometheus::{
    register_gauge, register_histogram, register_int_counter, register_int_gauge, Encoder,
    Gauge, Histogram, IntCounter, IntGauge, TextEncoder,
};

#[cfg(feature = "metrics")]
use lazy_static::lazy_static;

#[cfg(feature = "metrics")]
use std::net::SocketAddr;

#[cfg(feature = "metrics")]
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};

#[cfg(feature = "metrics")]
lazy_static! {
    // === Frame/Rendering Metrics ===

    /// Frame time in seconds (client)
    pub static ref FRAME_TIME: Histogram = register_histogram!(
        "engine_frame_time_seconds",
        "Frame rendering time in seconds"
    )
    .unwrap();

    /// Frames per second (client)
    pub static ref FPS: Gauge = register_gauge!(
        "engine_fps",
        "Current frames per second"
    )
    .unwrap();

    // === Server Tick Metrics ===

    /// Server tick duration in seconds
    pub static ref TICK_DURATION: Histogram = register_histogram!(
        "engine_tick_duration_seconds",
        "Server tick processing time in seconds"
    )
    .unwrap();

    /// Server tick rate (ticks per second)
    pub static ref TICK_RATE: Gauge = register_gauge!(
        "engine_tick_rate_tps",
        "Server tick rate in ticks per second"
    )
    .unwrap();

    /// Total server ticks processed
    pub static ref TICK_COUNT: IntCounter = register_int_counter!(
        "engine_tick_count_total",
        "Total number of server ticks processed"
    )
    .unwrap();

    // === ECS Metrics ===

    /// Current entity count
    pub static ref ENTITY_COUNT: IntGauge = register_int_gauge!(
        "engine_entity_count",
        "Current number of entities in the world"
    )
    .unwrap();

    /// Total entities spawned
    pub static ref ENTITIES_SPAWNED: IntCounter = register_int_counter!(
        "engine_entities_spawned_total",
        "Total number of entities spawned"
    )
    .unwrap();

    /// Total entities despawned
    pub static ref ENTITIES_DESPAWNED: IntCounter = register_int_counter!(
        "engine_entities_despawned_total",
        "Total number of entities despawned"
    )
    .unwrap();

    /// ECS query execution time
    pub static ref QUERY_TIME: Histogram = register_histogram!(
        "engine_query_time_seconds",
        "ECS query execution time in seconds"
    )
    .unwrap();

    // === Network Metrics ===

    /// Connected clients (server)
    pub static ref CONNECTED_CLIENTS: IntGauge = register_int_gauge!(
        "engine_connected_clients",
        "Number of currently connected clients"
    )
    .unwrap();

    /// Network bytes sent
    pub static ref BYTES_SENT: IntCounter = register_int_counter!(
        "engine_network_bytes_sent_total",
        "Total bytes sent over the network"
    )
    .unwrap();

    /// Network bytes received
    pub static ref BYTES_RECEIVED: IntCounter = register_int_counter!(
        "engine_network_bytes_received_total",
        "Total bytes received over the network"
    )
    .unwrap();

    /// Network packets sent
    pub static ref PACKETS_SENT: IntCounter = register_int_counter!(
        "engine_network_packets_sent_total",
        "Total packets sent"
    )
    .unwrap();

    /// Network packets received
    pub static ref PACKETS_RECEIVED: IntCounter = register_int_counter!(
        "engine_network_packets_received_total",
        "Total packets received"
    )
    .unwrap();

    /// Network latency (RTT)
    pub static ref NETWORK_LATENCY: Histogram = register_histogram!(
        "engine_network_latency_seconds",
        "Network round-trip time in seconds"
    )
    .unwrap();

    // === Memory Metrics ===

    /// Allocated memory in bytes
    pub static ref MEMORY_ALLOCATED: IntGauge = register_int_gauge!(
        "engine_memory_allocated_bytes",
        "Currently allocated memory in bytes"
    )
    .unwrap();

    /// Memory usage by ECS
    pub static ref ECS_MEMORY: IntGauge = register_int_gauge!(
        "engine_ecs_memory_bytes",
        "Memory used by ECS storage in bytes"
    )
    .unwrap();
}

/// Metrics registry for easy access to all metrics
#[cfg(feature = "metrics")]
pub struct MetricsRegistry;

#[cfg(feature = "metrics")]
impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new() -> Self {
        Self
    }

    // === Frame/Rendering Methods ===

    /// Record frame time in milliseconds
    pub fn record_frame_time(&self, ms: f64) {
        FRAME_TIME.observe(ms / 1000.0);
        if ms > 0.0 {
            FPS.set(1000.0 / ms);
        }
    }

    /// Set current FPS
    pub fn set_fps(&self, fps: f64) {
        FPS.set(fps);
    }

    // === Server Tick Methods ===

    /// Record server tick duration in milliseconds
    pub fn record_tick_duration(&self, ms: f64) {
        TICK_DURATION.observe(ms / 1000.0);
        TICK_COUNT.inc();
        if ms > 0.0 {
            TICK_RATE.set(1000.0 / ms);
        }
    }

    /// Set server tick rate
    pub fn set_tick_rate(&self, tps: f64) {
        TICK_RATE.set(tps);
    }

    // === ECS Methods ===

    /// Set current entity count
    pub fn set_entity_count(&self, count: i64) {
        ENTITY_COUNT.set(count);
    }

    /// Increment entity count
    pub fn increment_entity_count(&self, delta: i64) {
        ENTITY_COUNT.add(delta);
        if delta > 0 {
            ENTITIES_SPAWNED.inc_by(delta as u64);
        } else {
            ENTITIES_DESPAWNED.inc_by((-delta) as u64);
        }
    }

    /// Record ECS query execution time
    pub fn record_query_time(&self, ms: f64) {
        QUERY_TIME.observe(ms / 1000.0);
    }

    // === Network Methods ===

    /// Set connected clients count
    pub fn set_connected_clients(&self, count: i64) {
        CONNECTED_CLIENTS.set(count);
    }

    /// Record bytes sent
    pub fn record_bytes_sent(&self, bytes: u64) {
        BYTES_SENT.inc_by(bytes);
    }

    /// Record bytes received
    pub fn record_bytes_received(&self, bytes: u64) {
        BYTES_RECEIVED.inc_by(bytes);
    }

    /// Record packet sent
    pub fn record_packet_sent(&self) {
        PACKETS_SENT.inc();
    }

    /// Record packet received
    pub fn record_packet_received(&self) {
        PACKETS_RECEIVED.inc();
    }

    /// Record network latency (RTT) in milliseconds
    pub fn record_network_latency(&self, ms: f64) {
        NETWORK_LATENCY.observe(ms / 1000.0);
    }

    // === Memory Methods ===

    /// Set allocated memory in bytes
    pub fn set_memory_allocated(&self, bytes: i64) {
        MEMORY_ALLOCATED.set(bytes);
    }

    /// Set ECS memory usage in bytes
    pub fn set_ecs_memory(&self, bytes: i64) {
        ECS_MEMORY.set(bytes);
    }
}

#[cfg(feature = "metrics")]
impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Start HTTP server for Prometheus metrics scraping
///
/// # Arguments
/// * `addr` - Address to bind to (e.g., "0.0.0.0:9090")
///
/// # Examples
///
/// ```no_run
/// use engine_observability::metrics::start_metrics_server;
///
/// #[tokio::main]
/// async fn main() {
///     start_metrics_server("0.0.0.0:9090").await.unwrap();
/// }
/// ```
#[cfg(feature = "metrics")]
pub async fn start_metrics_server(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = addr.parse()?;

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, hyper::Error>(service_fn(metrics_handler))
    });

    let server = Server::bind(&addr).serve(make_svc);

    tracing::info!("Prometheus metrics server listening on http://{}", addr);

    server.await?;

    Ok(())
}

/// HTTP handler for metrics endpoint
#[cfg(feature = "metrics")]
async fn metrics_handler(_req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Ok(Response::new(Body::from(buffer)))
}

// === Stubs for when metrics feature is disabled ===

/// Stub metrics registry (metrics feature disabled)
#[cfg(not(feature = "metrics"))]
pub struct MetricsRegistry;

#[cfg(not(feature = "metrics"))]
impl MetricsRegistry {
    /// Create a new stub metrics registry (no-op)
    pub fn new() -> Self {
        Self
    }
    /// Record frame time (no-op)
    pub fn record_frame_time(&self, _ms: f64) {}
    /// Set FPS (no-op)
    pub fn set_fps(&self, _fps: f64) {}
    /// Record tick duration (no-op)
    pub fn record_tick_duration(&self, _ms: f64) {}
    /// Set tick rate (no-op)
    pub fn set_tick_rate(&self, _tps: f64) {}
    /// Set entity count (no-op)
    pub fn set_entity_count(&self, _count: i64) {}
    /// Increment entity count (no-op)
    pub fn increment_entity_count(&self, _delta: i64) {}
    /// Record query time (no-op)
    pub fn record_query_time(&self, _ms: f64) {}
    /// Set connected clients (no-op)
    pub fn set_connected_clients(&self, _count: i64) {}
    /// Record bytes sent (no-op)
    pub fn record_bytes_sent(&self, _bytes: u64) {}
    /// Record bytes received (no-op)
    pub fn record_bytes_received(&self, _bytes: u64) {}
    /// Record packet sent (no-op)
    pub fn record_packet_sent(&self) {}
    /// Record packet received (no-op)
    pub fn record_packet_received(&self) {}
    /// Record network latency (no-op)
    pub fn record_network_latency(&self, _ms: f64) {}
    /// Set allocated memory (no-op)
    pub fn set_memory_allocated(&self, _bytes: i64) {}
    /// Set ECS memory (no-op)
    pub fn set_ecs_memory(&self, _bytes: i64) {}
}

#[cfg(not(feature = "metrics"))]
impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub metrics server (metrics feature disabled)
#[cfg(not(feature = "metrics"))]
pub async fn start_metrics_server(_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    tracing::warn!("Metrics feature is disabled, server not started");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registry_creation() {
        let _registry = MetricsRegistry::new();
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_record_frame_time() {
        let registry = MetricsRegistry::new();
        registry.record_frame_time(16.7); // 60 FPS

        // Verify FPS was calculated
        let fps = FPS.get();
        assert!((fps - 59.88).abs() < 1.0); // ~60 FPS
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_entity_count() {
        let registry = MetricsRegistry::new();

        registry.set_entity_count(100);
        assert_eq!(ENTITY_COUNT.get(), 100);

        registry.increment_entity_count(10);
        assert_eq!(ENTITY_COUNT.get(), 110);
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn test_network_metrics() {
        let registry = MetricsRegistry::new();

        registry.set_connected_clients(5);
        assert_eq!(CONNECTED_CLIENTS.get(), 5);

        registry.record_bytes_sent(1024);
        registry.record_bytes_received(512);

        registry.record_packet_sent();
        registry.record_packet_received();
    }
}
