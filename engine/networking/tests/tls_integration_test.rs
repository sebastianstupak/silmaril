//! TLS integration tests
//!
//! Comprehensive tests for TLS functionality including:
//! - Connection establishment
//! - Data transmission
//! - Certificate validation
//! - Session management
//! - Error handling

use engine_networking::tls::certificates::{
    generate_and_save_self_signed_cert, CertificateManager,
};
use engine_networking::tls::{
    CertificateVerification, SelfSignedConfig, TlsClientConfigBuilder, TlsClientConnection,
    TlsServer, TlsServerConfigBuilder,
};
use std::env;
use std::path::PathBuf;
use tokio::time::{timeout, Duration};

/// Helper to create test certificates
async fn setup_test_certs() -> (PathBuf, PathBuf) {
    let temp_dir = env::temp_dir();
    let cert_path = temp_dir.join(format!("test_cert_{}.pem", std::process::id()));
    let key_path = temp_dir.join(format!("test_key_{}.pem", std::process::id()));

    let config = SelfSignedConfig::new("localhost")
        .add_san("127.0.0.1")
        .add_san("::1")
        .validity_days(1);

    generate_and_save_self_signed_cert(&config, &cert_path, &key_path)
        .expect("Failed to generate test certificate");

    (cert_path, key_path)
}

/// Cleanup test certificates
fn cleanup_test_certs(cert_path: &PathBuf, key_path: &PathBuf) {
    std::fs::remove_file(cert_path).ok();
    std::fs::remove_file(key_path).ok();
}

#[tokio::test]
async fn test_tls_basic_connection() {
    let (cert_path, key_path) = setup_test_certs().await;

    // Create server config
    let server_config = TlsServerConfigBuilder::new()
        .certificate(&cert_path, &key_path)
        .build()
        .expect("Failed to build server config");

    // Start TLS server
    let server = TlsServer::bind("127.0.0.1:0", server_config)
        .await
        .expect("Failed to bind server");
    let server_addr = server.local_addr().expect("Failed to get server address");

    // Spawn server task
    let server_handle = tokio::spawn(async move {
        let mut conn = server.accept().await.expect("Failed to accept connection");
        let msg = conn.recv().await.expect("Failed to receive message");
        conn.send(&msg).await.expect("Failed to send echo");
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create client config (disable verification for self-signed cert)
    let client_config = TlsClientConfigBuilder::new()
        .verification(CertificateVerification::Disabled)
        .build()
        .expect("Failed to build client config");

    // Connect client
    let mut client =
        TlsClientConnection::connect(server_addr.to_string(), "localhost", client_config)
            .await
            .expect("Failed to connect client");

    // Send message
    let message = b"Hello, TLS!";
    client.send(message).await.expect("Failed to send message");

    // Receive echo
    let response = client.recv().await.expect("Failed to receive response");
    assert_eq!(response, message);

    // Wait for server to finish
    timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("Server task timeout")
        .expect("Server task failed");

    cleanup_test_certs(&cert_path, &key_path);
}

#[tokio::test]
async fn test_tls_multiple_messages() {
    let (cert_path, key_path) = setup_test_certs().await;

    let server_config = TlsServerConfigBuilder::new()
        .certificate(&cert_path, &key_path)
        .build()
        .expect("Failed to build server config");

    let server = TlsServer::bind("127.0.0.1:0", server_config)
        .await
        .expect("Failed to bind server");
    let server_addr = server.local_addr().expect("Failed to get server address");

    tokio::spawn(async move {
        let mut conn = server.accept().await.expect("Failed to accept connection");
        for _ in 0..10 {
            let msg = conn.recv().await.expect("Failed to receive message");
            conn.send(&msg).await.expect("Failed to send echo");
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client_config = TlsClientConfigBuilder::new()
        .verification(CertificateVerification::Disabled)
        .build()
        .expect("Failed to build client config");

    let mut client =
        TlsClientConnection::connect(server_addr.to_string(), "localhost", client_config)
            .await
            .expect("Failed to connect client");

    // Send multiple messages
    for i in 0..10 {
        let message = format!("Message {}", i);
        client.send(message.as_bytes()).await.expect("Failed to send message");
        let response = client.recv().await.expect("Failed to receive response");
        assert_eq!(response, message.as_bytes());
    }

    cleanup_test_certs(&cert_path, &key_path);
}

#[tokio::test]
async fn test_tls_large_message() {
    let (cert_path, key_path) = setup_test_certs().await;

    let server_config = TlsServerConfigBuilder::new()
        .certificate(&cert_path, &key_path)
        .build()
        .expect("Failed to build server config");

    let server = TlsServer::bind("127.0.0.1:0", server_config)
        .await
        .expect("Failed to bind server");
    let server_addr = server.local_addr().expect("Failed to get server address");

    tokio::spawn(async move {
        let mut conn = server.accept().await.expect("Failed to accept connection");
        let msg = conn.recv().await.expect("Failed to receive message");
        conn.send(&msg).await.expect("Failed to send echo");
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client_config = TlsClientConfigBuilder::new()
        .verification(CertificateVerification::Disabled)
        .build()
        .expect("Failed to build client config");

    let mut client =
        TlsClientConnection::connect(server_addr.to_string(), "localhost", client_config)
            .await
            .expect("Failed to connect client");

    // Send large message (1MB)
    let message = vec![0x42u8; 1024 * 1024];
    client.send(&message).await.expect("Failed to send message");
    let response = client.recv().await.expect("Failed to receive response");
    assert_eq!(response.len(), message.len());
    assert_eq!(response, message);

    cleanup_test_certs(&cert_path, &key_path);
}

#[tokio::test]
async fn test_tls_concurrent_connections() {
    let (cert_path, key_path) = setup_test_certs().await;

    let server_config = TlsServerConfigBuilder::new()
        .certificate(&cert_path, &key_path)
        .build()
        .expect("Failed to build server config");

    let server = TlsServer::bind("127.0.0.1:0", server_config)
        .await
        .expect("Failed to bind server");
    let server_addr = server.local_addr().expect("Failed to get server address");

    // Spawn server task handling multiple connections
    let server_handle = tokio::spawn(async move {
        for _ in 0..5 {
            let mut conn = server.accept().await.expect("Failed to accept connection");
            tokio::spawn(async move {
                let msg = conn.recv().await.expect("Failed to receive message");
                conn.send(&msg).await.expect("Failed to send echo");
            });
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create multiple clients concurrently
    let mut handles = vec![];
    for i in 0..5 {
        let addr = server_addr.to_string();
        let handle = tokio::spawn(async move {
            let client_config = TlsClientConfigBuilder::new()
                .verification(CertificateVerification::Disabled)
                .build()
                .expect("Failed to build client config");

            let mut client = TlsClientConnection::connect(addr, "localhost", client_config)
                .await
                .expect("Failed to connect client");

            let message = format!("Client {}", i);
            client.send(message.as_bytes()).await.expect("Failed to send message");
            let response = client.recv().await.expect("Failed to receive response");
            assert_eq!(response, message.as_bytes());
        });
        handles.push(handle);
    }

    // Wait for all clients to finish
    for handle in handles {
        timeout(Duration::from_secs(10), handle)
            .await
            .expect("Client task timeout")
            .expect("Client task failed");
    }

    // Wait for server to finish
    timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("Server task timeout")
        .expect("Server task failed");

    cleanup_test_certs(&cert_path, &key_path);
}

#[tokio::test]
async fn test_certificate_manager() {
    let (cert_path, key_path) = setup_test_certs().await;
    let temp_dir = env::temp_dir().join(format!("cert_manager_{}", std::process::id()));

    let manager = CertificateManager::new(&temp_dir).expect("Failed to create certificate manager");

    // Load certificate
    let info = manager
        .load_certificate("test.local", &cert_path)
        .expect("Failed to load certificate");

    assert_eq!(info.subject, "test.local");
    assert!(info.is_valid());

    // Check certificate can be retrieved
    let retrieved = manager.get_certificate_info("test.local");
    assert!(retrieved.is_some());

    // Clean up
    cleanup_test_certs(&cert_path, &key_path);
    std::fs::remove_dir_all(temp_dir).ok();
}

#[tokio::test]
async fn test_connection_refused() {
    let client_config = TlsClientConfigBuilder::new()
        .verification(CertificateVerification::Disabled)
        .build()
        .expect("Failed to build client config");

    // Try to connect to non-existent server
    let result = TlsClientConnection::connect("127.0.0.1:9999", "localhost", client_config).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_message_too_large() {
    let (cert_path, key_path) = setup_test_certs().await;

    let server_config = TlsServerConfigBuilder::new()
        .certificate(&cert_path, &key_path)
        .build()
        .expect("Failed to build server config");

    let server = TlsServer::bind("127.0.0.1:0", server_config)
        .await
        .expect("Failed to bind server");
    let server_addr = server.local_addr().expect("Failed to get server address");

    tokio::spawn(async move {
        server.accept().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client_config = TlsClientConfigBuilder::new()
        .verification(CertificateVerification::Disabled)
        .build()
        .expect("Failed to build client config");

    let mut client =
        TlsClientConnection::connect(server_addr.to_string(), "localhost", client_config)
            .await
            .expect("Failed to connect client");

    // Try to send message larger than MAX_MESSAGE_SIZE (10MB)
    let message = vec![0u8; 11 * 1024 * 1024];
    let result = client.send(&message).await;
    assert!(result.is_err());

    cleanup_test_certs(&cert_path, &key_path);
}
