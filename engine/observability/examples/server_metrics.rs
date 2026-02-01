//! Example: Integrating Prometheus metrics into game server
//!
//! This example demonstrates how to:
//! 1. Initialize the metrics registry
//! 2. Start the Prometheus HTTP endpoint
//! 3. Record metrics during game loop
//! 4. Query metrics via Prometheus/Grafana
//!
//! Run with:
//! ```bash
//! cargo run --example server_metrics --features metrics
//! ```
//!
//! Then visit:
//! - http://localhost:9090/metrics - Raw Prometheus metrics
//! - http://localhost:9090 - Prometheus UI
//! - http://localhost:3000 - Grafana dashboard (if running via Docker)

#[cfg(feature = "metrics")]
use engine_observability::metrics::{start_metrics_server, MetricsRegistry};

use std::time::{Duration, Instant};
use tracing::info;

#[cfg(feature = "metrics")]
#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create metrics registry
    let metrics = MetricsRegistry::new();

    // Start Prometheus HTTP server in background
    tokio::spawn(async {
        if let Err(e) = start_metrics_server("0.0.0.0:9090").await {
            eprintln!("Failed to start metrics server: {}", e);
        }
    });

    info!("Metrics server started on http://localhost:9090/metrics");
    info!("Press Ctrl+C to stop");

    // Simulate game server loop
    let mut tick_count = 0;
    let mut entity_count = 0;
    let target_tps = 60.0;
    let tick_duration = Duration::from_secs_f64(1.0 / target_tps);

    loop {
        let tick_start = Instant::now();

        // === Simulate Game Tick ===

        // 1. Process network events
        simulate_network(&metrics);

        // 2. Update entities
        entity_count = simulate_ecs(&metrics, entity_count);

        // 3. Run physics
        simulate_physics();

        // === Record Tick Metrics ===
        let elapsed = tick_start.elapsed();
        metrics.record_tick_duration(elapsed.as_secs_f64() * 1000.0);

        tick_count += 1;
        if tick_count % 60 == 0 {
            info!(
                "Tick {}: {} entities, {:.2}ms tick time",
                tick_count,
                entity_count,
                elapsed.as_secs_f64() * 1000.0
            );
        }

        // Sleep to maintain target tick rate
        if elapsed < tick_duration {
            tokio::time::sleep(tick_duration - elapsed).await;
        }
    }
}

#[cfg(feature = "metrics")]
fn simulate_network(metrics: &MetricsRegistry) {
    // Simulate network activity
    let client_count = 5 + (rand::random::<u8>() % 10) as i64;
    metrics.set_connected_clients(client_count);

    // Simulate packet traffic
    for _ in 0..client_count {
        metrics.record_bytes_sent(128);
        metrics.record_packet_sent();

        metrics.record_bytes_received(64);
        metrics.record_packet_received();

        // Simulate network latency
        let latency_ms = 10.0 + rand::random::<f64>() * 50.0; // 10-60ms
        metrics.record_network_latency(latency_ms);
    }
}

#[cfg(feature = "metrics")]
fn simulate_ecs(metrics: &MetricsRegistry, mut entity_count: i64) -> i64 {
    // Simulate entity spawning/despawning
    if rand::random::<f32>() < 0.1 {
        // Spawn entity
        entity_count += 1;
        metrics.increment_entity_count(1);
    } else if entity_count > 10 && rand::random::<f32>() < 0.05 {
        // Despawn entity
        entity_count -= 1;
        metrics.increment_entity_count(-1);
    }

    // Simulate ECS query
    let query_time_us = 50.0 + rand::random::<f64>() * 200.0; // 50-250 microseconds
    metrics.record_query_time(query_time_us / 1000.0);

    // Update ECS memory usage (estimate)
    let ecs_memory = entity_count * 256; // 256 bytes per entity
    metrics.set_ecs_memory(ecs_memory);

    entity_count
}

#[cfg(feature = "metrics")]
fn simulate_physics() {
    // Simulate physics work
    std::thread::sleep(Duration::from_micros(500 + rand::random::<u64>() % 1000));
}

#[cfg(not(feature = "metrics"))]
fn main() {
    println!("This example requires the 'metrics' feature");
    println!("Run with: cargo run --example server_metrics --features metrics");
}
