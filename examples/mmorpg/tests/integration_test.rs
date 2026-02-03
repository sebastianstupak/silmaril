//! Integration test for multiplayer demo
//!
//! Tests the complete client-server interaction flow.

use engine_core::ecs::Entity;
use engine_networking::{TcpClient, TcpServer};
use mmorpg_shared::{ClientMessage, ServerMessage};
use std::time::Duration;
use tokio::time::timeout;

/// Test helper to send and receive messages
async fn send_client_msg(client: &TcpClient, msg: ClientMessage) -> anyhow::Result<()> {
    let data = bincode::serialize(&msg)?;
    client.send(&data).await?;
    Ok(())
}

async fn recv_server_msg(client: &TcpClient) -> anyhow::Result<ServerMessage> {
    let data = client.recv().await?;
    let msg = bincode::deserialize(&data)?;
    Ok(msg)
}

#[tokio::test]
async fn test_client_server_connection() {
    // Start server
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();

    // Spawn server accept task
    let server_task = tokio::spawn(async move {
        let conn = server.accept().await.unwrap();

        // Receive join message
        let data = conn.recv().await.unwrap();
        let msg: ClientMessage = bincode::deserialize(&data).unwrap();
        assert!(matches!(msg, ClientMessage::Join { .. }));

        // Send welcome
        let welcome = ServerMessage::Welcome {
            player_entity: Entity::new(1, 0),
            player_name: "TestPlayer".to_string(),
        };
        let data = bincode::serialize(&welcome).unwrap();
        conn.send(&data).await.unwrap();

        // Send empty snapshot
        let snapshot = ServerMessage::StateSnapshot { players: vec![] };
        let data = bincode::serialize(&snapshot).unwrap();
        conn.send(&data).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = TcpClient::connect(&server_addr.to_string()).await.unwrap();

    // Send join
    send_client_msg(&client, ClientMessage::Join { player_name: "TestPlayer".to_string() })
        .await
        .unwrap();

    // Receive welcome
    let msg = recv_server_msg(&client).await.unwrap();
    assert!(matches!(msg, ServerMessage::Welcome { .. }));

    // Receive snapshot
    let msg = recv_server_msg(&client).await.unwrap();
    assert!(matches!(msg, ServerMessage::StateSnapshot { .. }));

    // Wait for server task
    server_task.await.unwrap();
}

#[tokio::test]
async fn test_multiple_clients() {
    // Start server
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();

    // Spawn server that handles 2 clients
    let server_task = tokio::spawn(async move {
        // Accept first client
        let conn1 = server.accept().await.unwrap();
        let data = conn1.recv().await.unwrap();
        let msg: ClientMessage = bincode::deserialize(&data).unwrap();
        assert!(matches!(msg, ClientMessage::Join { .. }));

        // Send welcome to client 1
        let welcome1 = ServerMessage::Welcome {
            player_entity: Entity::new(1, 0),
            player_name: "Client1".to_string(),
        };
        let data = bincode::serialize(&welcome1).unwrap();
        conn1.send(&data).await.unwrap();

        // Send snapshot to client 1
        let snapshot = ServerMessage::StateSnapshot { players: vec![] };
        let data = bincode::serialize(&snapshot).unwrap();
        conn1.send(&data).await.unwrap();

        // Accept second client
        let conn2 = server.accept().await.unwrap();
        let data = conn2.recv().await.unwrap();
        let msg: ClientMessage = bincode::deserialize(&data).unwrap();
        assert!(matches!(msg, ClientMessage::Join { .. }));

        // Send welcome to client 2
        let welcome2 = ServerMessage::Welcome {
            player_entity: Entity::new(2, 0),
            player_name: "Client2".to_string(),
        };
        let data = bincode::serialize(&welcome2).unwrap();
        conn2.send(&data).await.unwrap();

        // Send snapshot to client 2
        let snapshot = ServerMessage::StateSnapshot { players: vec![] };
        let data = bincode::serialize(&snapshot).unwrap();
        conn2.send(&data).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect first client
    let client1 = TcpClient::connect(&server_addr.to_string()).await.unwrap();
    send_client_msg(&client1, ClientMessage::Join { player_name: "Client1".to_string() })
        .await
        .unwrap();

    // Receive welcome for client 1
    let msg = recv_server_msg(&client1).await.unwrap();
    assert!(matches!(msg, ServerMessage::Welcome { .. }));
    let msg = recv_server_msg(&client1).await.unwrap();
    assert!(matches!(msg, ServerMessage::StateSnapshot { .. }));

    // Connect second client
    let client2 = TcpClient::connect(&server_addr.to_string()).await.unwrap();
    send_client_msg(&client2, ClientMessage::Join { player_name: "Client2".to_string() })
        .await
        .unwrap();

    // Receive welcome for client 2
    let msg = recv_server_msg(&client2).await.unwrap();
    assert!(matches!(msg, ServerMessage::Welcome { .. }));
    let msg = recv_server_msg(&client2).await.unwrap();
    assert!(matches!(msg, ServerMessage::StateSnapshot { .. }));

    // Wait for server
    server_task.await.unwrap();
}

#[tokio::test]
async fn test_player_movement() {
    // Start server
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();

    // Spawn server
    let server_task = tokio::spawn(async move {
        let conn = server.accept().await.unwrap();

        // Receive join
        let data = conn.recv().await.unwrap();
        let msg: ClientMessage = bincode::deserialize(&data).unwrap();
        assert!(matches!(msg, ClientMessage::Join { .. }));

        // Send welcome
        let entity = Entity::new(1, 0);
        let welcome = ServerMessage::Welcome {
            player_entity: entity,
            player_name: "TestPlayer".to_string(),
        };
        let data = bincode::serialize(&welcome).unwrap();
        conn.send(&data).await.unwrap();

        // Send snapshot
        let snapshot = ServerMessage::StateSnapshot { players: vec![] };
        let data = bincode::serialize(&snapshot).unwrap();
        conn.send(&data).await.unwrap();

        // Receive move command
        let data = conn.recv().await.unwrap();
        let msg: ClientMessage = bincode::deserialize(&data).unwrap();
        if let ClientMessage::Move { x, y } = msg {
            assert_eq!(x, 100.0);
            assert_eq!(y, 200.0);

            // Echo back the move
            let moved = ServerMessage::PlayerMoved { entity, x, y };
            let data = bincode::serialize(&moved).unwrap();
            conn.send(&data).await.unwrap();
        } else {
            panic!("Expected Move message");
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = TcpClient::connect(&server_addr.to_string()).await.unwrap();

    // Join
    send_client_msg(&client, ClientMessage::Join { player_name: "TestPlayer".to_string() })
        .await
        .unwrap();
    recv_server_msg(&client).await.unwrap(); // Welcome
    recv_server_msg(&client).await.unwrap(); // Snapshot

    // Send move command
    send_client_msg(&client, ClientMessage::Move { x: 100.0, y: 200.0 }).await.unwrap();

    // Receive movement confirmation
    let msg = recv_server_msg(&client).await.unwrap();
    if let ServerMessage::PlayerMoved { x, y, .. } = msg {
        assert_eq!(x, 100.0);
        assert_eq!(y, 200.0);
    } else {
        panic!("Expected PlayerMoved message");
    }

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_disconnect_handling() {
    // Start server
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();

    // Spawn server
    let server_task = tokio::spawn(async move {
        let conn = server.accept().await.unwrap();

        // Receive join
        conn.recv().await.unwrap();

        // Send welcome
        let welcome = ServerMessage::Welcome {
            player_entity: Entity::new(1, 0),
            player_name: "TestPlayer".to_string(),
        };
        let data = bincode::serialize(&welcome).unwrap();
        conn.send(&data).await.unwrap();

        // Send snapshot
        let snapshot = ServerMessage::StateSnapshot { players: vec![] };
        let data = bincode::serialize(&snapshot).unwrap();
        conn.send(&data).await.unwrap();

        // Receive disconnect
        let data = conn.recv().await.unwrap();
        let msg: ClientMessage = bincode::deserialize(&data).unwrap();
        assert!(matches!(msg, ClientMessage::Disconnect));
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = TcpClient::connect(&server_addr.to_string()).await.unwrap();

    // Join
    send_client_msg(&client, ClientMessage::Join { player_name: "TestPlayer".to_string() })
        .await
        .unwrap();
    recv_server_msg(&client).await.unwrap(); // Welcome
    recv_server_msg(&client).await.unwrap(); // Snapshot

    // Disconnect
    send_client_msg(&client, ClientMessage::Disconnect).await.unwrap();

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_message_serialization() {
    // Test ClientMessage serialization
    let join = ClientMessage::Join { player_name: "Test".to_string() };
    let data = bincode::serialize(&join).unwrap();
    let decoded: ClientMessage = bincode::deserialize(&data).unwrap();
    assert_eq!(join, decoded);

    let move_msg = ClientMessage::Move { x: 1.0, y: 2.0 };
    let data = bincode::serialize(&move_msg).unwrap();
    let decoded: ClientMessage = bincode::deserialize(&data).unwrap();
    assert_eq!(move_msg, decoded);

    // Test ServerMessage serialization
    let welcome = ServerMessage::Welcome {
        player_entity: Entity::new(1, 0),
        player_name: "Test".to_string(),
    };
    let data = bincode::serialize(&welcome).unwrap();
    let decoded: ServerMessage = bincode::deserialize(&data).unwrap();
    assert_eq!(welcome, decoded);

    let moved = ServerMessage::PlayerMoved { entity: Entity::new(1, 0), x: 10.0, y: 20.0 };
    let data = bincode::serialize(&moved).unwrap();
    let decoded: ServerMessage = bincode::deserialize(&data).unwrap();
    assert_eq!(moved, decoded);
}

#[tokio::test]
async fn test_concurrent_operations() {
    // Start server
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();

    // Spawn server that handles rapid messages
    let server_task = tokio::spawn(async move {
        let conn = server.accept().await.unwrap();

        // Join
        conn.recv().await.unwrap();
        let welcome = ServerMessage::Welcome {
            player_entity: Entity::new(1, 0),
            player_name: "Test".to_string(),
        };
        conn.send(&bincode::serialize(&welcome).unwrap()).await.unwrap();
        let snapshot = ServerMessage::StateSnapshot { players: vec![] };
        conn.send(&bincode::serialize(&snapshot).unwrap()).await.unwrap();

        // Receive and echo 10 move commands
        for _ in 0..10 {
            let data = conn.recv().await.unwrap();
            let msg: ClientMessage = bincode::deserialize(&data).unwrap();
            if let ClientMessage::Move { x, y } = msg {
                let moved = ServerMessage::PlayerMoved { entity: Entity::new(1, 0), x, y };
                conn.send(&bincode::serialize(&moved).unwrap()).await.unwrap();
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let client = TcpClient::connect(&server_addr.to_string()).await.unwrap();

    // Join
    send_client_msg(&client, ClientMessage::Join { player_name: "Test".to_string() })
        .await
        .unwrap();
    recv_server_msg(&client).await.unwrap();
    recv_server_msg(&client).await.unwrap();

    // Send 10 rapid move commands
    for i in 0..10 {
        send_client_msg(&client, ClientMessage::Move { x: i as f32, y: i as f32 * 2.0 })
            .await
            .unwrap();

        // Verify response
        let msg = timeout(Duration::from_secs(1), recv_server_msg(&client)).await.unwrap().unwrap();
        if let ServerMessage::PlayerMoved { x, y, .. } = msg {
            assert_eq!(x, i as f32);
            assert_eq!(y, i as f32 * 2.0);
        } else {
            panic!("Expected PlayerMoved");
        }
    }

    server_task.await.unwrap();
}
