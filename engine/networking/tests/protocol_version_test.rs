//! Protocol version handshake tests
//!
//! Verifies that protocol version mismatch is properly detected and rejected.

use engine_core::ecs::World;
use engine_networking::{
    deserialize_server_message, serialize_client_message, ClientMessage, SerializationFormat,
    ServerLoop, ServerMessage, TcpServer, PROTOCOL_VERSION,
};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_protocol_version_match_accepted() {
    // Create server
    let world = World::new();
    let server_loop = ServerLoop::new(world);

    let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let server_addr = tcp_server.local_addr().unwrap();

    // Start accepting connections
    server_loop.start_accepting(tcp_server).await;

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = engine_networking::TcpClient::connect(&server_addr.to_string()).await.unwrap();

    // Send handshake with correct version
    let handshake = ClientMessage::Handshake {
        version: PROTOCOL_VERSION,
        client_name: "TestClient".to_string(),
    };

    let handshake_msg = serialize_client_message(&handshake, SerializationFormat::Bincode).unwrap();
    client.send(&handshake_msg.payload).await.unwrap();

    // Wait for response
    sleep(Duration::from_millis(200)).await;

    // Receive response
    let response_data = client.recv().await.unwrap();
    let response_framed = engine_networking::FramedMessage::new(response_data).unwrap();
    let response =
        deserialize_server_message(&response_framed, SerializationFormat::Bincode).unwrap();

    // Verify handshake response
    match response {
        ServerMessage::HandshakeResponse { version, server_name, player_entity: _ } => {
            assert_eq!(version, PROTOCOL_VERSION);
            assert_eq!(server_name, "Silmaril Server");
        }
        _ => panic!("Expected HandshakeResponse, got {:?}", response),
    }

    client.close().await.unwrap();
}

#[tokio::test]
async fn test_protocol_version_mismatch_rejected() {
    // Create server
    let world = World::new();
    let server_loop = ServerLoop::new(world);

    let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let server_addr = tcp_server.local_addr().unwrap();

    // Start accepting connections
    server_loop.start_accepting(tcp_server).await;

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = engine_networking::TcpClient::connect(&server_addr.to_string()).await.unwrap();

    // Send handshake with WRONG version
    let wrong_version = PROTOCOL_VERSION + 1;
    let handshake = ClientMessage::Handshake {
        version: wrong_version,
        client_name: "OutdatedClient".to_string(),
    };

    let handshake_msg = serialize_client_message(&handshake, SerializationFormat::Bincode).unwrap();
    client.send(&handshake_msg.payload).await.unwrap();

    // Wait for server to process and disconnect
    sleep(Duration::from_millis(200)).await;

    // Try to receive response - should get error message or connection closed
    let response_result = client.recv().await;

    match response_result {
        Ok(data) => {
            // If we get a response, it should be an error message (ChatBroadcast)
            let response_framed = engine_networking::FramedMessage::new(data).unwrap();
            let response =
                deserialize_server_message(&response_framed, SerializationFormat::Bincode).unwrap();

            match response {
                ServerMessage::ChatBroadcast { sender, message, .. } => {
                    assert_eq!(sender, "Server");
                    assert!(message.contains("Protocol version mismatch"));
                    assert!(message.contains(&wrong_version.to_string()));
                    assert!(message.contains(&PROTOCOL_VERSION.to_string()));
                }
                ServerMessage::HandshakeResponse { .. } => {
                    panic!("Server should not accept mismatched version");
                }
                _ => panic!("Unexpected response: {:?}", response),
            }

            // Connection should be closed after error message
            let result = client.recv().await;
            assert!(result.is_err(), "Connection should be closed after version mismatch");
        }
        Err(_) => {
            // Connection was closed immediately - also acceptable
            // This means the server rejected before sending error message
        }
    }

    client.close().await.ok(); // May already be closed
}

#[tokio::test]
async fn test_old_client_version_rejected() {
    // Create server
    let world = World::new();
    let server_loop = ServerLoop::new(world);

    let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let server_addr = tcp_server.local_addr().unwrap();

    // Start accepting connections
    server_loop.start_accepting(tcp_server).await;

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = engine_networking::TcpClient::connect(&server_addr.to_string()).await.unwrap();

    // Send handshake with OLD version (0)
    let old_version = 0u32;
    let handshake =
        ClientMessage::Handshake { version: old_version, client_name: "VeryOldClient".to_string() };

    let handshake_msg = serialize_client_message(&handshake, SerializationFormat::Bincode).unwrap();
    client.send(&handshake_msg.payload).await.unwrap();

    // Wait for server to process and disconnect
    sleep(Duration::from_millis(200)).await;

    // Verify client is rejected
    let response_result = client.recv().await;

    match response_result {
        Ok(data) => {
            let response_framed = engine_networking::FramedMessage::new(data).unwrap();
            let response =
                deserialize_server_message(&response_framed, SerializationFormat::Bincode).unwrap();

            match response {
                ServerMessage::ChatBroadcast { message, .. } => {
                    assert!(message.contains("Protocol version mismatch"));
                }
                ServerMessage::HandshakeResponse { .. } => {
                    panic!("Server should not accept old version");
                }
                _ => {}
            }
        }
        Err(_) => {
            // Connection closed - acceptable
        }
    }

    client.close().await.ok();
}

#[test]
fn test_protocol_version_constant_defined() {
    // Verify PROTOCOL_VERSION is set to expected value
    assert_eq!(PROTOCOL_VERSION, 1, "Protocol version should be 1");
}

#[test]
fn test_handshake_message_serialization() {
    // Verify handshake message can be serialized/deserialized
    let handshake = ClientMessage::Handshake {
        version: PROTOCOL_VERSION,
        client_name: "TestClient".to_string(),
    };

    let framed = serialize_client_message(&handshake, SerializationFormat::Bincode).unwrap();

    assert!(
        framed.total_size() < 100,
        "Handshake should be compact: {} bytes",
        framed.total_size()
    );

    let deserialized =
        engine_networking::deserialize_client_message(&framed, SerializationFormat::Bincode)
            .unwrap();

    assert_eq!(handshake, deserialized);
}

#[test]
fn test_protocol_error_unsupported_version() {
    use engine_networking::ProtocolError;

    let error =
        ProtocolError::UnsupportedVersion { client_version: 2, server_version: PROTOCOL_VERSION };

    let error_string = format!("{}", error);
    assert!(error_string.contains("Unsupported protocol version"));
    assert!(error_string.contains("client=2"));
    assert!(error_string.contains(&format!("server={}", PROTOCOL_VERSION)));
}
