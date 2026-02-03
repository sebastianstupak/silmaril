//! E2E connectivity tests
//!
//! Tests basic client-server connectivity using the full networking stack.

use crate::e2e::helpers::{spawn_test_client, spawn_test_client_with_name, spawn_test_server};
use std::time::Duration;

#[tokio::test]
async fn test_single_client_connects() {
    // Spawn server on random port
    let server = spawn_test_server("127.0.0.1:0").await.unwrap();

    // Verify server is running
    assert_eq!(server.client_count().await, 0);

    // Spawn client and connect
    let client = spawn_test_client(server.address()).await.unwrap();

    // Give some time for connection to stabilize
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify client is connected
    assert!(client.is_connected().await, "Client should be connected");
    assert!(
        client.player_entity().await.is_some(),
        "Client should have player entity after handshake"
    );

    // Verify server sees the client
    assert_eq!(server.client_count().await, 1, "Server should see 1 connected client");

    // Disconnect client
    client.disconnect().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify server sees disconnection
    assert_eq!(server.client_count().await, 0, "Server should see 0 clients after disconnect");

    // Cleanup
    server.shutdown();
}

#[tokio::test]
async fn test_multiple_clients_connect() {
    let server = spawn_test_server("127.0.0.1:0").await.unwrap();

    // Connect 3 clients
    let client1 = spawn_test_client_with_name(server.address(), "Client1").await.unwrap();
    let client2 = spawn_test_client_with_name(server.address(), "Client2").await.unwrap();
    let client3 = spawn_test_client_with_name(server.address(), "Client3").await.unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify all clients connected
    assert!(client1.is_connected().await);
    assert!(client2.is_connected().await);
    assert!(client3.is_connected().await);

    // Verify server sees all clients
    assert_eq!(server.client_count().await, 3);

    // Disconnect one client
    client2.disconnect().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify server sees 2 clients
    assert_eq!(server.client_count().await, 2);

    // Cleanup
    client1.disconnect().await;
    client3.disconnect().await;
    server.shutdown();
}

#[tokio::test]
async fn test_client_reconnect() {
    let server = spawn_test_server("127.0.0.1:0").await.unwrap();

    // First connection
    let client1 = spawn_test_client(server.address()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.client_count().await, 1);

    // Disconnect
    client1.disconnect().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.client_count().await, 0);

    // Reconnect
    let client2 = spawn_test_client(server.address()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.client_count().await, 1);
    assert!(client2.is_connected().await);

    // Cleanup
    client2.disconnect().await;
    server.shutdown();
}

#[tokio::test]
async fn test_server_tick_runs_with_clients() {
    let server = spawn_test_server("127.0.0.1:0").await.unwrap();

    let initial_tick = server.tick().await;

    // Connect client
    let client = spawn_test_client(server.address()).await.unwrap();

    // Wait for some ticks
    tokio::time::sleep(Duration::from_millis(200)).await;

    let final_tick = server.tick().await;

    // Server should have processed ticks while client was connected
    assert!(
        final_tick > initial_tick + 10,
        "Server should have processed at least 10 ticks (200ms at 60 TPS)"
    );

    // Cleanup
    client.disconnect().await;
    server.shutdown();
}

#[tokio::test]
async fn test_client_receives_handshake_response() {
    let server = spawn_test_server("127.0.0.1:0").await.unwrap();

    // Client should automatically complete handshake
    let client = spawn_test_client(server.address()).await.unwrap();

    // Verify handshake completed
    assert!(client.is_connected().await);

    let player_entity = client.player_entity().await;
    assert!(
        player_entity.is_some(),
        "Client should receive player entity in handshake response"
    );

    // Cleanup
    client.disconnect().await;
    server.shutdown();
}

#[tokio::test]
async fn test_concurrent_connections() {
    let server = spawn_test_server("127.0.0.1:0").await.unwrap();

    // Spawn multiple clients concurrently
    let mut handles = Vec::new();
    for i in 0..5 {
        let addr = server.address().to_string();
        let name = format!("Client{}", i);
        handles.push(tokio::spawn(async move { spawn_test_client_with_name(&addr, &name).await }));
    }

    // Wait for all clients to connect
    let mut clients = Vec::new();
    for handle in handles {
        let client = handle.await.unwrap().unwrap();
        clients.push(client);
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify all clients connected
    assert_eq!(server.client_count().await, 5);

    for client in &clients {
        assert!(client.is_connected().await);
    }

    // Cleanup
    for client in clients {
        client.disconnect().await;
    }
    server.shutdown();
}

#[tokio::test]
async fn test_client_disconnect_cleans_up_entity() {
    let server = spawn_test_server("127.0.0.1:0").await.unwrap();

    // Connect client and get player entity
    let client = spawn_test_client(server.address()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let player_entity = client.player_entity().await;
    assert!(player_entity.is_some());

    // Disconnect
    client.disconnect().await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Server should have despawned the player entity
    // (We can't directly verify this without accessing server world,
    // but we verify client count drops to 0)
    assert_eq!(server.client_count().await, 0);

    server.shutdown();
}
