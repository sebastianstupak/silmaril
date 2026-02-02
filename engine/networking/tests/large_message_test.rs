//! Large Message Handling Integration Tests
//!
//! Comprehensive tests for large message handling:
//! 1. Send and receive 100KB message
//! 2. Send and receive 1MB message
//! 3. Send and receive 10MB message
//! 4. Multiple large messages sequentially
//! 5. Concurrent large message transfers
//! 6. Large message timeout handling
//! 7. Large message correctness verification
//! 8. Maximum message size enforcement

use engine_networking::{TcpClient, TcpServer};
use std::time::Duration;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create test data of specific size
fn create_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

/// Start echo server
async fn start_echo_server() -> String {
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
    addr
}

// ============================================================================
// Test 1: Send and Receive 100KB Message
// ============================================================================

#[tokio::test]
async fn test_100kb_message() {
    let addr = start_echo_server().await;
    let client = TcpClient::connect(&addr).await.unwrap();

    let data = create_test_data(100 * 1024);

    client.send(&data).await.unwrap();
    let received = client.recv().await.unwrap();

    assert_eq!(received.len(), data.len());
    assert_eq!(received, data);
}

// ============================================================================
// Test 2: Send and Receive 1MB Message
// ============================================================================

#[tokio::test]
async fn test_1mb_message() {
    let addr = start_echo_server().await;
    let client = TcpClient::connect(&addr).await.unwrap();

    let data = create_test_data(1024 * 1024);

    client.send(&data).await.unwrap();
    let received = client.recv().await.unwrap();

    assert_eq!(received.len(), data.len());
    assert_eq!(received, data);
}

// ============================================================================
// Test 3: Send and Receive 10MB Message
// ============================================================================

#[tokio::test]
async fn test_10mb_message() {
    let addr = start_echo_server().await;
    let client = TcpClient::connect(&addr).await.unwrap();

    let data = create_test_data(10 * 1024 * 1024);

    client.send(&data).await.unwrap();
    let received = client.recv().await.unwrap();

    assert_eq!(received.len(), data.len());
    assert_eq!(received, data);
}

// ============================================================================
// Test 4: Multiple Large Messages Sequentially
// ============================================================================

#[tokio::test]
async fn test_multiple_large_messages_sequential() {
    let addr = start_echo_server().await;
    let client = TcpClient::connect(&addr).await.unwrap();

    // Send 10 x 1MB messages
    for i in 0..10 {
        let data = create_test_data(1024 * 1024);

        client.send(&data).await.unwrap();
        let received = client.recv().await.unwrap();

        assert_eq!(received.len(), data.len(), "Message {} failed", i);
        assert_eq!(received, data, "Message {} content mismatch", i);
    }
}

// ============================================================================
// Test 5: Concurrent Large Message Transfers
// ============================================================================

#[tokio::test]
async fn test_concurrent_large_message_transfers() {
    let addr = start_echo_server().await;

    let mut handles = Vec::new();

    // 10 clients each sending a 1MB message
    for i in 0..10 {
        let addr = addr.clone();
        let handle = tokio::spawn(async move {
            let client = TcpClient::connect(&addr).await.unwrap();
            let data = create_test_data(1024 * 1024);

            client.send(&data).await.unwrap();
            let received = client.recv().await.unwrap();

            assert_eq!(received.len(), data.len(), "Client {} length mismatch", i);
            assert_eq!(received, data, "Client {} content mismatch", i);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

// ============================================================================
// Test 6: Large Message Timeout Handling
// ============================================================================

#[tokio::test]
async fn test_large_message_timeout_handling() {
    let addr = start_echo_server().await;
    let client = TcpClient::connect(&addr).await.unwrap();

    let data = create_test_data(10 * 1024 * 1024);

    // Should complete within reasonable timeout
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        client.send(&data).await.unwrap();
        client.recv().await.unwrap()
    })
    .await;

    assert!(result.is_ok(), "Large message transfer timed out");
    let received = result.unwrap();
    assert_eq!(received.len(), data.len());
}

// ============================================================================
// Test 7: Large Message Correctness Verification
// ============================================================================

#[tokio::test]
async fn test_large_message_correctness() {
    let addr = start_echo_server().await;
    let client = TcpClient::connect(&addr).await.unwrap();

    // Create data with specific pattern
    let size = 5 * 1024 * 1024; // 5MB
    let data: Vec<u8> = (0..size)
        .map(|i| {
            // Pattern: position % 256, plus some variation based on position
            ((i % 256) + (i / 256 % 256)) as u8
        })
        .collect();

    client.send(&data).await.unwrap();
    let received = client.recv().await.unwrap();

    assert_eq!(received.len(), data.len());

    // Verify every byte matches
    for (i, (sent, recv)) in data.iter().zip(received.iter()).enumerate() {
        assert_eq!(sent, recv, "Byte mismatch at position {}: sent={}, received={}", i, sent, recv);
    }
}

// ============================================================================
// Test 8: Maximum Message Size Enforcement
// ============================================================================

#[tokio::test]
async fn test_maximum_message_size_enforcement() {
    let addr = start_echo_server().await;
    let client = TcpClient::connect(&addr).await.unwrap();

    // Try to send message larger than MAX_MESSAGE_SIZE (10MB)
    let data = create_test_data(11 * 1024 * 1024); // 11MB

    let result = client.send(&data).await;

    // Should fail due to size limit
    assert!(result.is_err(), "Expected error for message exceeding size limit");
}

// ============================================================================
// Test 9: Mixed Small and Large Messages
// ============================================================================

#[tokio::test]
async fn test_mixed_small_and_large_messages() {
    let addr = start_echo_server().await;
    let client = TcpClient::connect(&addr).await.unwrap();

    // Send pattern: small, large, small, large, ...
    let test_sizes = vec![
        100,             // 100 bytes
        100 * 1024,      // 100KB
        1000,            // 1KB
        1024 * 1024,     // 1MB
        50,              // 50 bytes
        5 * 1024 * 1024, // 5MB
    ];

    for (i, size) in test_sizes.iter().enumerate() {
        let data = create_test_data(*size);

        client.send(&data).await.unwrap();
        let received = client.recv().await.unwrap();

        assert_eq!(received.len(), data.len(), "Message {} (size {}) length mismatch", i, size);
        assert_eq!(received, data, "Message {} (size {}) content mismatch", i, size);
    }
}

// ============================================================================
// Test 10: Large Message Partial Read Handling
// ============================================================================

#[tokio::test]
async fn test_large_message_streaming() {
    let addr = start_echo_server().await;
    let client = TcpClient::connect(&addr).await.unwrap();

    // Send and receive 5 x 1MB messages
    for i in 0..5 {
        let data = create_test_data(1024 * 1024);
        client.send(&data).await.unwrap();
        let received = client.recv().await.unwrap();
        assert_eq!(received.len(), 1024 * 1024, "Response {} size mismatch", i);
    }
}

// ============================================================================
// Test 11: Large Message Under Poor Network Conditions
// ============================================================================

#[tokio::test]
async fn test_large_message_robustness() {
    let addr = start_echo_server().await;

    // Multiple clients sending large messages with slight delays
    let mut handles = Vec::new();

    for i in 0..5 {
        let addr = addr.clone();
        let handle = tokio::spawn(async move {
            let client = TcpClient::connect(&addr).await.unwrap();

            // Each client sends 3 x 2MB messages
            for j in 0..3 {
                let data = create_test_data(2 * 1024 * 1024);

                client.send(&data).await.unwrap();

                // Small random delay
                tokio::time::sleep(Duration::from_millis(i * 10 + j * 5)).await;

                let received = client.recv().await.unwrap();

                assert_eq!(received.len(), data.len());
                assert_eq!(received, data);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
