//! Integration tests for TCP and UDP sockets

use engine_networking::{TcpClient, TcpServer, UdpClient, UdpServer};
use std::time::Duration;

#[tokio::test]
async fn test_tcp_connect_and_send() {
    // Start echo server
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap();

    tokio::spawn(async move {
        server.run_echo_server().await.ok();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = TcpClient::connect(&addr.to_string()).await.unwrap();

    // Send message
    let message = b"Hello, World!";
    client.send(message).await.unwrap();

    // Receive echo
    let response = client.recv().await.unwrap();
    assert_eq!(response, message);

    client.close().await.unwrap();
}

#[tokio::test]
async fn test_tcp_multiple_messages() {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap();

    tokio::spawn(async move {
        server.run_echo_server().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = TcpClient::connect(&addr.to_string()).await.unwrap();

    // Send multiple messages
    for i in 0..10 {
        let message = format!("Message {}", i);
        client.send(message.as_bytes()).await.unwrap();
        let response = client.recv().await.unwrap();
        assert_eq!(response, message.as_bytes());
    }

    client.close().await.unwrap();
}

#[tokio::test]
async fn test_tcp_large_message() {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap();

    tokio::spawn(async move {
        server.run_echo_server().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = TcpClient::connect(&addr.to_string()).await.unwrap();

    // Send large message (1MB)
    let message = vec![0x42u8; 1024 * 1024];
    client.send(&message).await.unwrap();
    let response = client.recv().await.unwrap();
    assert_eq!(response.len(), message.len());
    assert_eq!(response, message);

    client.close().await.unwrap();
}

#[tokio::test]
async fn test_udp_send_recv() {
    // Start echo server
    let server = UdpServer::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr();

    tokio::spawn(async move {
        server.run_echo_server().await.ok();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = UdpClient::connect(&server_addr.to_string()).await.unwrap();

    // Send packet
    let message = b"Hello, UDP!";
    client.send(message).await.unwrap();

    // Receive echo
    let response = client.recv().await.unwrap();
    assert_eq!(response, message);
}

#[tokio::test]
async fn test_udp_multiple_packets() {
    let server = UdpServer::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr();

    tokio::spawn(async move {
        server.run_echo_server().await.ok();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = UdpClient::connect(&server_addr.to_string()).await.unwrap();

    // Send multiple packets
    for i in 0..10 {
        let message = format!("Packet {}", i);
        client.send(message.as_bytes()).await.unwrap();
        let response = client.recv().await.unwrap();
        assert_eq!(response, message.as_bytes());
    }
}
