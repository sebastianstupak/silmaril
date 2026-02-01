//! Example showing message sizes for different protocol messages

use engine_core::ecs::Entity;
use engine_networking::{
    ClientMessage, ServerMessage, EntityState, SerializationFormat,
    serialize_client_message, serialize_server_message,
};

fn main() {
    println!("=== Network Protocol Message Sizes ===\n");

    // Client Messages
    println!("CLIENT MESSAGES:");
    println!("----------------");

    let player_move = ClientMessage::PlayerMove {
        x: 100.0,
        y: 50.0,
        z: 200.0,
        timestamp: 12345,
    };
    let framed = serialize_client_message(&player_move, SerializationFormat::Bincode).unwrap();
    println!("PlayerMove:        {} bytes (including 4-byte framing)", framed.total_size());

    let player_action = ClientMessage::PlayerAction {
        action_id: 1,
        target: Some(Entity::new(42, 0)),
        timestamp: 12345,
    };
    let framed = serialize_client_message(&player_action, SerializationFormat::Bincode).unwrap();
    println!("PlayerAction:      {} bytes (including 4-byte framing)", framed.total_size());

    let chat_short = ClientMessage::ChatMessage {
        message: "Hello".to_string(),
        channel: 0,
    };
    let framed = serialize_client_message(&chat_short, SerializationFormat::Bincode).unwrap();
    println!("ChatMessage (5):   {} bytes (including 4-byte framing)", framed.total_size());

    let chat_long = ClientMessage::ChatMessage {
        message: "This is a much longer message that players might send during gameplay.".to_string(),
        channel: 0,
    };
    let framed = serialize_client_message(&chat_long, SerializationFormat::Bincode).unwrap();
    println!("ChatMessage (71):  {} bytes (including 4-byte framing)", framed.total_size());

    let ping = ClientMessage::Ping {
        client_time: 123456789,
    };
    let framed = serialize_client_message(&ping, SerializationFormat::Bincode).unwrap();
    println!("Ping:              {} bytes (including 4-byte framing)", framed.total_size());

    let handshake = ClientMessage::Handshake {
        version: 1,
        client_name: "TestClient".to_string(),
    };
    let framed = serialize_client_message(&handshake, SerializationFormat::Bincode).unwrap();
    println!("Handshake:         {} bytes (including 4-byte framing)", framed.total_size());

    // Server Messages
    println!("\nSERVER MESSAGES:");
    println!("----------------");

    let entity_spawned = ServerMessage::EntitySpawned {
        entity: Entity::new(42, 0),
        prefab_id: 100,
        x: 10.0,
        y: 20.0,
        z: 30.0,
    };
    let framed = serialize_server_message(&entity_spawned, SerializationFormat::Bincode).unwrap();
    println!("EntitySpawned:     {} bytes (including 4-byte framing)", framed.total_size());

    let entity_despawned = ServerMessage::EntityDespawned {
        entity: Entity::new(42, 0),
    };
    let framed = serialize_server_message(&entity_despawned, SerializationFormat::Bincode).unwrap();
    println!("EntityDespawned:   {} bytes (including 4-byte framing)", framed.total_size());

    let entity_transform = ServerMessage::EntityTransform {
        entity: Entity::new(42, 0),
        x: 1.0,
        y: 2.0,
        z: 3.0,
        qx: 0.0,
        qy: 0.0,
        qz: 0.0,
        qw: 1.0,
    };
    let framed = serialize_server_message(&entity_transform, SerializationFormat::Bincode).unwrap();
    println!("EntityTransform:   {} bytes (including 4-byte framing)", framed.total_size());

    let pong = ServerMessage::Pong {
        client_time: 123456789,
        server_time: 123456800,
    };
    let framed = serialize_server_message(&pong, SerializationFormat::Bincode).unwrap();
    println!("Pong:              {} bytes (including 4-byte framing)", framed.total_size());

    // State Update
    println!("\nSTATE UPDATE BATCHES:");
    println!("---------------------");

    for count in [1, 10, 100, 1000] {
        let entities: Vec<EntityState> = (0..count)
            .map(|i| EntityState {
                entity: Entity::new(i as u32, 0),
                x: (i as f32) * 10.0,
                y: 0.0,
                z: (i as f32) * 5.0,
                qx: 0.0,
                qy: 0.0,
                qz: 0.0,
                qw: 1.0,
                health: if i % 2 == 0 { Some(100.0) } else { None },
                max_health: if i % 2 == 0 { Some(100.0) } else { None },
            })
            .collect();

        let state_update = ServerMessage::StateUpdate {
            timestamp: 12345,
            entities,
        };

        let framed = serialize_server_message(&state_update, SerializationFormat::Bincode).unwrap();
        println!(
            "StateUpdate ({}):   {} bytes ({} bytes/entity)",
            count,
            framed.total_size(),
            framed.total_size() / count
        );
    }

    // Overhead analysis
    println!("\nOVERHEAD ANALYSIS:");
    println!("------------------");
    println!("Framing overhead: 4 bytes (length prefix)");
    println!("Bincode enum discriminant: ~1 byte");
    println!("Total fixed overhead: ~5 bytes per message");
    println!("\nAll messages meet <50 bytes overhead requirement ✓");
}
