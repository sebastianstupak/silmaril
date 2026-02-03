use engine_observability::metrics::{start_metrics_server, MetricsRegistry};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_metrics_server_starts() {
    // Use a random port to avoid conflicts
    let addr = "127.0.0.1:19090";

    // Start metrics server in background
    let server_handle = tokio::spawn(async move {
        start_metrics_server(addr).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try to connect to the server
    let client = reqwest::Client::new();
    let result =
        timeout(Duration::from_secs(2), client.get(&format!("http://{}/metrics", addr)).send())
            .await;

    // Clean up
    server_handle.abort();

    // Verify connection succeeded
    assert!(result.is_ok(), "Failed to connect to metrics server");
    let response = result.unwrap();
    assert!(response.is_ok(), "HTTP request failed");
    let response = response.unwrap();
    assert_eq!(response.status(), 200, "Expected HTTP 200 OK");
}

#[tokio::test]
async fn test_metrics_endpoint_format() {
    // Use a different port
    let addr = "127.0.0.1:19091";

    // Start metrics server in background
    let server_handle = tokio::spawn(async move {
        start_metrics_server(addr).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Record some metrics
    let registry = MetricsRegistry::new();
    registry.record_frame_time(16.7); // 60 FPS
    registry.set_entity_count(100);
    registry.set_connected_clients(5);

    // Fetch metrics
    let client = reqwest::Client::new();
    let response =
        timeout(Duration::from_secs(2), client.get(&format!("http://{}/metrics", addr)).send())
            .await
            .unwrap()
            .unwrap();

    // Clean up
    server_handle.abort();

    // Verify response
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();

    // Verify Prometheus format (contains metric names and values)
    assert!(body.contains("engine_frame_time_seconds"), "Missing frame time metric");
    assert!(body.contains("engine_entity_count"), "Missing entity count metric");
    assert!(body.contains("engine_connected_clients"), "Missing connected clients metric");

    // Verify HELP and TYPE directives (Prometheus format requirement)
    assert!(body.contains("# HELP"), "Missing HELP directives");
    assert!(body.contains("# TYPE"), "Missing TYPE directives");
}

#[tokio::test]
async fn test_metrics_update_in_real_time() {
    // Use a different port
    let addr = "127.0.0.1:19092";

    // Start metrics server in background
    let server_handle = tokio::spawn(async move {
        start_metrics_server(addr).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    let registry = MetricsRegistry::new();

    // Initial state
    registry.set_entity_count(50);

    // Fetch metrics
    let client = reqwest::Client::new();
    let response =
        timeout(Duration::from_secs(2), client.get(&format!("http://{}/metrics", addr)).send())
            .await
            .unwrap()
            .unwrap();

    let body1 = response.text().await.unwrap();
    assert!(body1.contains("engine_entity_count"), "Should contain entity count metric");
    assert!(body1.contains("50"), "Should contain value 50");

    // Update metrics
    registry.set_entity_count(100);

    // Fetch again
    let response =
        timeout(Duration::from_secs(2), client.get(&format!("http://{}/metrics", addr)).send())
            .await
            .unwrap()
            .unwrap();

    let body2 = response.text().await.unwrap();
    assert!(body2.contains("engine_entity_count"), "Should contain entity count metric");
    assert!(body2.contains("100"), "Should contain value 100");

    // Clean up
    server_handle.abort();
}

#[tokio::test]
async fn test_concurrent_metrics_requests() {
    // Use a different port
    let addr = "127.0.0.1:19093";

    // Start metrics server in background
    let server_handle = tokio::spawn(async move {
        start_metrics_server(addr).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Make multiple concurrent requests
    let client = reqwest::Client::new();
    let mut handles = vec![];

    for _ in 0..10 {
        let client = client.clone();
        let addr = addr.to_string();
        let handle = tokio::spawn(async move {
            let response = client.get(&format!("http://{}/metrics", addr)).send().await.unwrap();
            assert_eq!(response.status(), 200);
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        timeout(Duration::from_secs(2), handle).await.unwrap().unwrap();
    }

    // Clean up
    server_handle.abort();
}

#[tokio::test]
async fn test_all_metrics_are_exposed() {
    // Use a different port
    let addr = "127.0.0.1:19094";

    // Start metrics server in background
    let server_handle = tokio::spawn(async move {
        start_metrics_server(addr).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Record metrics for all categories
    let registry = MetricsRegistry::new();

    // Frame/Rendering
    registry.record_frame_time(16.7);
    registry.set_fps(60.0);

    // Server tick
    registry.record_tick_duration(10.0);
    registry.set_tick_rate(60.0);

    // ECS
    registry.set_entity_count(1000);
    registry.increment_entity_count(10);
    registry.record_query_time(0.5);

    // Network
    registry.set_connected_clients(50);
    registry.record_bytes_sent(1024);
    registry.record_bytes_received(2048);
    registry.record_packet_sent();
    registry.record_packet_received();
    registry.record_network_latency(25.0);

    // Memory
    registry.set_memory_allocated(1024 * 1024 * 100); // 100 MB
    registry.set_ecs_memory(1024 * 1024 * 10); // 10 MB

    // Fetch metrics
    let client = reqwest::Client::new();
    let response =
        timeout(Duration::from_secs(2), client.get(&format!("http://{}/metrics", addr)).send())
            .await
            .unwrap()
            .unwrap();

    let body = response.text().await.unwrap();

    // Verify all metric categories are present
    let expected_metrics = [
        // Frame/Rendering
        "engine_frame_time_seconds",
        "engine_fps",
        // Server tick
        "engine_tick_duration_seconds",
        "engine_tick_rate_tps",
        "engine_tick_count_total",
        // ECS
        "engine_entity_count",
        "engine_entities_spawned_total",
        "engine_query_time_seconds",
        // Network
        "engine_connected_clients",
        "engine_network_bytes_sent_total",
        "engine_network_bytes_received_total",
        "engine_network_packets_sent_total",
        "engine_network_packets_received_total",
        "engine_network_latency_seconds",
        // Memory
        "engine_memory_allocated_bytes",
        "engine_ecs_memory_bytes",
    ];

    for metric in &expected_metrics {
        assert!(body.contains(metric), "Missing metric: {}. Body:\n{}", metric, body);
    }

    // Clean up
    server_handle.abort();
}

#[tokio::test]
async fn test_metrics_registry_default() {
    let registry = MetricsRegistry::default();

    // Should be able to record metrics without panicking
    registry.record_frame_time(16.7);
    registry.set_entity_count(100);
    registry.record_bytes_sent(1024);
}

#[tokio::test]
async fn test_histogram_metrics() {
    // Use a different port
    let addr = "127.0.0.1:19095";

    // Start metrics server in background
    let server_handle = tokio::spawn(async move {
        start_metrics_server(addr).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    let registry = MetricsRegistry::new();

    // Record multiple observations for histogram
    for _ in 0..10 {
        registry.record_frame_time(16.7);
        registry.record_tick_duration(10.0);
        registry.record_query_time(0.5);
        registry.record_network_latency(25.0);
    }

    // Fetch metrics
    let client = reqwest::Client::new();
    let response =
        timeout(Duration::from_secs(2), client.get(&format!("http://{}/metrics", addr)).send())
            .await
            .unwrap()
            .unwrap();

    let body = response.text().await.unwrap();

    // Verify histogram buckets are present
    assert!(body.contains("_bucket"), "Missing histogram buckets in Prometheus format");
    assert!(body.contains("_sum"), "Missing histogram sum");
    assert!(body.contains("_count"), "Missing histogram count");

    // Clean up
    server_handle.abort();
}

#[tokio::test]
async fn test_counter_metrics() {
    // Use a different port
    let addr = "127.0.0.1:19096";

    // Start metrics server in background
    let server_handle = tokio::spawn(async move {
        start_metrics_server(addr).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    let registry = MetricsRegistry::new();

    // Increment counters
    for _ in 0..5 {
        registry.record_packet_sent();
        registry.record_packet_received();
    }

    registry.record_bytes_sent(1024);
    registry.record_bytes_received(2048);

    // Fetch metrics
    let client = reqwest::Client::new();
    let response =
        timeout(Duration::from_secs(2), client.get(&format!("http://{}/metrics", addr)).send())
            .await
            .unwrap()
            .unwrap();

    let body = response.text().await.unwrap();

    // Verify counters have _total suffix (Prometheus convention)
    assert!(body.contains("_total"), "Should contain _total suffix");

    // Verify counter metrics exist
    assert!(
        body.contains("engine_network_packets_sent_total"),
        "Should contain packets sent metric"
    );
    assert!(
        body.contains("engine_network_packets_received_total"),
        "Should contain packets received metric"
    );

    // Verify we recorded some packets (the value should be in the output somewhere)
    assert!(body.contains("5") || body.contains("5.0"), "Should contain the value 5");

    // Clean up
    server_handle.abort();
}
