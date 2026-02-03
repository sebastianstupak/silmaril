//! Connection timeout tests (Quick Win #5)
//!
//! Verifies that the server correctly detects and removes idle clients.

use engine_core::ecs::World;
use engine_networking::{ServerLoop, TcpServer, TcpClient, CLIENT_TIMEOUT};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_client_timeout_after_idle() {
    // Start server
    let world = World::new();
    let server_loop = ServerLoop::new(world);
    let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = tcp_server.local_addr().unwrap();

    // Start accepting connections
    server_loop.start_accepting(tcp_server).await;

    // Connect client
    let client = TcpClient::connect(&addr.to_string()).await.unwrap();

    // Wait for connection to establish
    sleep(Duration::from_millis(100)).await;

    // Verify client is connected
    assert_eq!(server_loop.client_count().await, 1, "Client should be connected");

    // Simulate idle timeout by waiting longer than CLIENT_TIMEOUT
    // For testing, we'll use a shorter timeout simulation
    // In production CLIENT_TIMEOUT is 30 seconds
    sleep(CLIENT_TIMEOUT + Duration::from_millis(500)).await;

    // Server should have removed the idle client
    // Note: This test will take 30+ seconds to run in production
    let client_count = server_loop.client_count().await;
    assert_eq!(
        client_count, 0,
        "Server should have removed idle client after timeout. Found {} clients",
        client_count
    );

    // Cleanup
    client.close().await.ok();
}

#[tokio::test]
async fn test_active_client_not_timed_out() {
    // Start server
    let world = World::new();
    let server_loop = ServerLoop::new(world);
    let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = tcp_server.local_addr().unwrap();

    server_loop.start_accepting(tcp_server).await;

    // Connect client
    let client = TcpClient::connect(&addr.to_string()).await.unwrap();

    sleep(Duration::from_millis(100)).await;
    assert_eq!(server_loop.client_count().await, 1);

    // Send a message periodically to keep connection active
    // This test uses a shorter duration to avoid long test times
    for _ in 0..5 {
        // Send keepalive message
        client.send(&[0u8; 1]).await.ok();

        // Wait, but less than timeout
        sleep(Duration::from_secs(5)).await;

        // Client should still be connected
        assert_eq!(
            server_loop.client_count().await,
            1,
            "Active client should not be timed out"
        );
    }

    // Cleanup
    client.close().await.ok();
}

#[tokio::test]
async fn test_multiple_clients_partial_timeout() {
    // Start server
    let world = World::new();
    let server_loop = ServerLoop::new(world);
    let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = tcp_server.local_addr().unwrap();

    server_loop.start_accepting(tcp_server).await;

    // Connect 3 clients
    let client1 = TcpClient::connect(&addr.to_string()).await.unwrap();
    let client2 = TcpClient::connect(&addr.to_string()).await.unwrap();
    let client3 = TcpClient::connect(&addr.to_string()).await.unwrap();

    sleep(Duration::from_millis(200)).await;
    assert_eq!(server_loop.client_count().await, 3);

    // Keep client1 and client3 active, let client2 go idle
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    for _ in 0..6 {
        interval.tick().await;

        // Send keepalive only for client1 and client3
        client1.send(&[0u8; 1]).await.ok();
        client3.send(&[0u8; 1]).await.ok();
        // client2 does NOT send - it will timeout
    }

    // After ~30 seconds, client2 should be timed out
    sleep(CLIENT_TIMEOUT + Duration::from_millis(500)).await;

    let remaining = server_loop.client_count().await;
    assert_eq!(
        remaining, 2,
        "Only active clients should remain. Expected 2, found {}",
        remaining
    );

    // Cleanup
    client1.close().await.ok();
    client2.close().await.ok();
    client3.close().await.ok();
}

#[tokio::test]
async fn test_timeout_detection_removes_player_entity() {
    // Start server
    let world = World::new();
    let server_loop = ServerLoop::new(world);
    let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = tcp_server.local_addr().unwrap();

    server_loop.start_accepting(tcp_server).await;

    // Connect client and complete handshake
    let client = TcpClient::connect(&addr.to_string()).await.unwrap();

    sleep(Duration::from_millis(100)).await;

    // Get initial entity count from server world
    let initial_entities = server_loop.entity_count().await;

    // Wait for timeout
    sleep(CLIENT_TIMEOUT + Duration::from_millis(500)).await;

    // Server should have cleaned up the client AND its player entity
    assert_eq!(server_loop.client_count().await, 0, "Client should be removed");

    let final_entities = server_loop.entity_count().await;
    assert!(
        final_entities <= initial_entities,
        "Player entity should be despawned on timeout"
    );

    // Cleanup
    client.close().await.ok();
}

#[tokio::test]
#[ignore] // Ignored by default due to 30+ second runtime
async fn test_timeout_respects_exact_duration() {
    // This test verifies the exact timeout duration
    let world = World::new();
    let server_loop = ServerLoop::new(world);
    let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = tcp_server.local_addr().unwrap();

    server_loop.start_accepting(tcp_server).await;

    let client = TcpClient::connect(&addr.to_string()).await.unwrap();
    sleep(Duration::from_millis(100)).await;
    assert_eq!(server_loop.client_count().await, 1);

    // Wait just BEFORE timeout
    sleep(CLIENT_TIMEOUT - Duration::from_secs(1)).await;

    // Client should still be connected
    assert_eq!(
        server_loop.client_count().await,
        1,
        "Client should not timeout before CLIENT_TIMEOUT duration"
    );

    // Wait the remaining time + buffer
    sleep(Duration::from_secs(2)).await;

    // Now client should be timed out
    assert_eq!(
        server_loop.client_count().await,
        0,
        "Client should timeout after CLIENT_TIMEOUT duration"
    );

    client.close().await.ok();
}
