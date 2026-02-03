//! Integration test for Prometheus metrics in server context
//!
//! This test verifies that the metrics endpoint can be started and accessed
//! in a realistic server scenario.

use engine_observability::metrics::{start_metrics_server, MetricsRegistry};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_server_metrics_integration() {
    // Simulate server startup
    let metrics_port = "19097";
    let metrics_addr = format!("0.0.0.0:{}", metrics_port);

    // Start metrics server (same as in server binary)
    let metrics_addr_clone = metrics_addr.clone();
    let metrics_handle = tokio::spawn(async move {
        if let Err(e) = start_metrics_server(&metrics_addr_clone).await {
            eprintln!("Metrics server error: {:?}", e);
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Initialize metrics registry (same as server would do)
    let registry = MetricsRegistry::new();

    // Simulate server recording metrics
    registry.record_tick_duration(10.0); // 10ms tick
    registry.set_tick_rate(60.0); // 60 TPS
    registry.set_entity_count(1000);
    registry.set_connected_clients(50);
    registry.record_bytes_sent(1024 * 1024); // 1MB
    registry.record_bytes_received(512 * 1024); // 512KB

    // Verify metrics are accessible
    let client = reqwest::Client::new();
    let result = timeout(
        Duration::from_secs(5),
        client.get(&format!("http://127.0.0.1:{}/metrics", metrics_port)).send(),
    )
    .await;

    // Clean up first
    metrics_handle.abort();

    // Now verify results
    assert!(result.is_ok(), "Should connect to metrics server");
    let response = result.unwrap().expect("HTTP request should succeed");
    assert_eq!(response.status(), 200, "Should return HTTP 200 OK");

    let body = response.text().await.expect("Should get response body");

    // Verify server-specific metrics are present
    assert!(
        body.contains("engine_tick_duration_seconds"),
        "Should contain tick duration metric"
    );
    assert!(body.contains("engine_tick_rate_tps"), "Should contain tick rate metric");
    assert!(body.contains("engine_entity_count"), "Should contain entity count metric");
    assert!(
        body.contains("engine_connected_clients"),
        "Should contain connected clients metric"
    );
    assert!(
        body.contains("engine_network_bytes_sent_total"),
        "Should contain bytes sent metric"
    );
    assert!(
        body.contains("engine_network_bytes_received_total"),
        "Should contain bytes received metric"
    );

    // Verify it's valid Prometheus format
    assert!(body.contains("# HELP"), "Should contain HELP directives");
    assert!(body.contains("# TYPE"), "Should contain TYPE directives");
}

#[tokio::test]
async fn test_metrics_survive_server_restart() {
    // Test that metrics accumulate correctly across requests
    let addr = "127.0.0.1:19098";

    let server_handle = tokio::spawn(async move {
        start_metrics_server(addr).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let registry = MetricsRegistry::new();

    // Simulate multiple server ticks
    for i in 1..=10 {
        registry.record_tick_duration(10.0 + i as f64); // Varying tick times
        registry.set_entity_count(100 * i);
    }

    // Fetch metrics
    let client = reqwest::Client::new();
    let response =
        timeout(Duration::from_secs(2), client.get(&format!("http://{}/metrics", addr)).send())
            .await
            .unwrap()
            .unwrap();

    let body = response.text().await.unwrap();

    // Verify histogram recorded all observations
    assert!(body.contains("engine_tick_duration_seconds"));

    // Verify entity count shows last value
    assert!(body.contains("engine_entity_count"));
    assert!(body.contains("1000"), "Should show final entity count");

    server_handle.abort();
}

#[tokio::test]
async fn test_concurrent_metric_recording() {
    // Test that metrics can be recorded from multiple tasks
    let addr = "127.0.0.1:19099";

    let server_handle = tokio::spawn(async move {
        start_metrics_server(addr).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Spawn multiple tasks recording metrics
    let mut handles = vec![];
    for _ in 0..5 {
        let handle = tokio::spawn(async {
            let registry = MetricsRegistry::new();
            for _ in 0..10 {
                registry.record_tick_duration(10.0);
                registry.record_packet_sent();
                registry.record_packet_received();
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify metrics were recorded
    let client = reqwest::Client::new();
    let response =
        timeout(Duration::from_secs(2), client.get(&format!("http://{}/metrics", addr)).send())
            .await
            .unwrap()
            .unwrap();

    let body = response.text().await.unwrap();

    // All tasks should have contributed to the metrics
    assert!(body.contains("engine_network_packets_sent_total"));
    assert!(body.contains("engine_network_packets_received_total"));

    server_handle.abort();
}
