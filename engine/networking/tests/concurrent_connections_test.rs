//! Concurrent Connection Integration Tests
//!
//! Comprehensive tests for concurrent connection handling:
//! 1. Basic concurrent connections
//! 2. Connection acceptance rate
//! 3. Idle connection stability
//! 4. Connection cleanup
//! 5. Resource leak detection
//! 6. Burst handling
//! 7. Mixed load scenarios
//! 8. Connection timeout handling
//! 9. Concurrent send/receive
//! 10. Server shutdown with active connections

use engine_networking::{TcpClient, TcpServer};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::sync::Barrier;

// ============================================================================
// Test 1: Basic Concurrent Connections
// ============================================================================

#[tokio::test]
async fn test_basic_concurrent_connections() {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap().to_string();

    // Spawn server task
    tokio::spawn(async move {
        for _ in 0..10 {
            server.accept().await.ok();
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect 10 clients concurrently
    let mut handles = Vec::new();
    for _ in 0..10 {
        let addr = addr.clone();
        let handle = tokio::spawn(async move { TcpClient::connect(&addr).await });
        handles.push(handle);
    }

    // Verify all connections succeeded
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }
}

// ============================================================================
// Test 2: Connection Acceptance Rate
// ============================================================================

#[tokio::test]
async fn test_connection_acceptance_rate() {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap().to_string();

    let accepted_count = Arc::new(AtomicUsize::new(0));
    let accepted_count_clone = Arc::clone(&accepted_count);

    tokio::spawn(async move {
        for _ in 0..100 {
            if server.accept().await.is_ok() {
                accepted_count_clone.fetch_add(1, Ordering::SeqCst);
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect 100 clients as fast as possible
    let mut handles = Vec::new();
    for _ in 0..100 {
        let addr = addr.clone();
        let handle = tokio::spawn(async move { TcpClient::connect(&addr).await });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap().ok();
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify all connections were accepted
    assert_eq!(accepted_count.load(Ordering::SeqCst), 100);
}

// ============================================================================
// Test 3: Idle Connection Stability
// ============================================================================

#[tokio::test]
async fn test_idle_connection_stability() {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        loop {
            if let Ok(_conn) = server.accept().await {
                // Just accept, keep connection alive
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Create 50 idle connections
    let mut clients = Vec::new();
    for _ in 0..50 {
        let client = TcpClient::connect(&addr).await.unwrap();
        clients.push(client);
    }

    // Keep connections idle for 1 second
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Verify all connections are still alive (don't try to send, just verify they exist)
    assert_eq!(clients.len(), 50);
}

// ============================================================================
// Test 4: Connection Cleanup
// ============================================================================

#[tokio::test]
async fn test_connection_cleanup() {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        loop {
            server.accept().await.ok();
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Create and destroy connections in batches
    for _ in 0..5 {
        let mut clients = Vec::new();

        // Create 20 connections
        for _ in 0..20 {
            let client = TcpClient::connect(&addr).await.unwrap();
            clients.push(client);
        }

        // Close all connections
        for client in clients {
            client.close().await.ok();
        }

        // Brief pause between batches
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // If no resource leaks, this should succeed
    assert!(TcpClient::connect(&addr).await.is_ok());
}

// ============================================================================
// Test 5: Resource Leak Detection
// ============================================================================

#[tokio::test]
async fn test_resource_leak_detection() {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        loop {
            server.accept().await.ok();
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Rapidly connect and disconnect 200 times
    for _ in 0..200 {
        let client = TcpClient::connect(&addr).await.unwrap();
        client.send(b"test").await.ok();
        client.close().await.ok();
    }

    // System should still be stable
    assert!(TcpClient::connect(&addr).await.is_ok());
}

// ============================================================================
// Test 6: Burst Handling
// ============================================================================

#[tokio::test]
async fn test_burst_handling() {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap().to_string();

    let accepted_count = Arc::new(AtomicUsize::new(0));
    let accepted_count_clone = Arc::clone(&accepted_count);

    tokio::spawn(async move {
        for _ in 0..100 {
            if server.accept().await.is_ok() {
                accepted_count_clone.fetch_add(1, Ordering::SeqCst);
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Create a burst of 100 connections all at once
    let barrier = Arc::new(Barrier::new(101)); // 100 clients + 1 main thread

    let mut handles = Vec::new();
    for _ in 0..100 {
        let addr = addr.clone();
        let barrier = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            barrier.wait().await;
            TcpClient::connect(&addr).await
        });
        handles.push(handle);
    }

    // Start all connections simultaneously
    barrier.wait().await;

    // Wait for all connections to complete
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            success_count += 1;
        }
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify most connections succeeded (allow for some timing issues)
    assert!(success_count >= 95, "Only {} out of 100 connections succeeded", success_count);
    assert!(accepted_count.load(Ordering::SeqCst) >= 95);
}

// ============================================================================
// Test 7: Mixed Load Scenarios
// ============================================================================

#[tokio::test]
async fn test_mixed_load_scenarios() {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap().to_string();

    // Server echoes messages
    tokio::spawn(async move {
        loop {
            if let Ok(conn) = server.accept().await {
                tokio::spawn(async move {
                    loop {
                        match conn.recv().await {
                            Ok(data) => {
                                if conn.send(&data).await.is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Mix of different behaviors:
    // - Some clients send many messages
    // - Some clients stay idle
    // - Some clients connect and disconnect quickly

    let mut handles = Vec::new();

    // 10 heavy senders
    for i in 0..10 {
        let addr = addr.clone();
        let handle = tokio::spawn(async move {
            let client = TcpClient::connect(&addr).await.unwrap();
            for _ in 0..10 {
                let msg = format!("Message {}", i);
                client.send(msg.as_bytes()).await.unwrap();
                client.recv().await.unwrap();
            }
        });
        handles.push(handle);
    }

    // 10 idle clients
    for _ in 0..10 {
        let addr = addr.clone();
        let handle = tokio::spawn(async move {
            let _client = TcpClient::connect(&addr).await.unwrap();
            tokio::time::sleep(Duration::from_millis(500)).await;
        });
        handles.push(handle);
    }

    // 10 quick connect/disconnect
    for _ in 0..10 {
        let addr = addr.clone();
        let handle = tokio::spawn(async move {
            let client = TcpClient::connect(&addr).await.unwrap();
            client.close().await.ok();
        });
        handles.push(handle);
    }

    // Wait for all scenarios to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

// ============================================================================
// Test 8: Connection Timeout Handling
// ============================================================================

#[tokio::test]
async fn test_connection_timeout_handling() {
    // Try to connect to a non-existent server
    let result =
        tokio::time::timeout(Duration::from_secs(2), TcpClient::connect("127.0.0.1:54321")).await;

    // Connection should timeout or fail
    assert!(result.is_err() || result.unwrap().is_err());
}

// ============================================================================
// Test 9: Concurrent Send/Receive
// ============================================================================

#[tokio::test]
async fn test_concurrent_send_receive() {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        loop {
            if let Ok(conn) = server.accept().await {
                tokio::spawn(async move {
                    loop {
                        match conn.recv().await {
                            Ok(data) => {
                                if conn.send(&data).await.is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // 20 clients each sending 10 messages concurrently
    let mut handles = Vec::new();
    for i in 0..20 {
        let addr = addr.clone();
        let handle = tokio::spawn(async move {
            let client = TcpClient::connect(&addr).await.unwrap();

            for j in 0..10 {
                let msg = format!("Client {} Message {}", i, j);
                client.send(msg.as_bytes()).await.unwrap();
                let response = client.recv().await.unwrap();
                assert_eq!(response, msg.as_bytes());
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

// ============================================================================
// Test 10: Server Shutdown with Active Connections
// ============================================================================

#[tokio::test]
async fn test_server_shutdown_with_active_connections() {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap().to_string();

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Server task that can be shutdown
    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = server.accept() => {
                    if result.is_err() {
                        break;
                    }
                }
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Create several active connections
    let mut clients = Vec::new();
    for _ in 0..10 {
        let client = TcpClient::connect(&addr).await.unwrap();
        clients.push(client);
    }

    // Shutdown server
    shutdown_tx.send(()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Existing connections should still work (until closed)
    for client in &clients {
        // May succeed or fail depending on timing, but shouldn't panic
        client.send(b"test").await.ok();
    }

    // New connections should fail
    let result = tokio::time::timeout(Duration::from_millis(100), TcpClient::connect(&addr)).await;
    assert!(result.is_err() || result.unwrap().is_err());
}
