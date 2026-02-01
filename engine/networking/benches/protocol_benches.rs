//! Network Protocol Benchmarks
//!
//! Measures serialization performance, message overhead, and throughput.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use engine_core::ecs::Entity;
use engine_networking::{
    ClientMessage, ServerMessage, EntityState, FramedMessage,
    SerializationFormat, serialize_client_message, deserialize_client_message,
    serialize_server_message, deserialize_server_message,
};
use std::io::Cursor;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_player_move(timestamp: u64) -> ClientMessage {
    ClientMessage::PlayerMove {
        x: 100.0 + (timestamp as f32 * 0.1),
        y: 50.0,
        z: 200.0 + (timestamp as f32 * 0.05),
        timestamp,
    }
}

fn create_player_action(timestamp: u64) -> ClientMessage {
    ClientMessage::PlayerAction {
        action_id: 1,
        target: Some(Entity::new(42, 0)),
        timestamp,
    }
}

fn create_chat_message(msg: &str) -> ClientMessage {
    ClientMessage::ChatMessage {
        message: msg.to_string(),
        channel: 0,
    }
}

fn create_ping(timestamp: u64) -> ClientMessage {
    ClientMessage::Ping {
        client_time: timestamp,
    }
}

fn create_handshake() -> ClientMessage {
    ClientMessage::Handshake {
        version: 1,
        client_name: "BenchClient".to_string(),
    }
}

fn create_entity_spawned(entity_id: u32) -> ServerMessage {
    ServerMessage::EntitySpawned {
        entity: Entity::new(entity_id, 0),
        prefab_id: 100,
        x: 10.0,
        y: 20.0,
        z: 30.0,
    }
}

fn create_entity_despawned(entity_id: u32) -> ServerMessage {
    ServerMessage::EntityDespawned {
        entity: Entity::new(entity_id, 0),
    }
}

fn create_entity_transform(entity_id: u32) -> ServerMessage {
    ServerMessage::EntityTransform {
        entity: Entity::new(entity_id, 0),
        x: 1.0,
        y: 2.0,
        z: 3.0,
        qx: 0.0,
        qy: 0.0,
        qz: 0.0,
        qw: 1.0,
    }
}

fn create_state_update(entity_count: usize) -> ServerMessage {
    let entities: Vec<EntityState> = (0..entity_count)
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

    ServerMessage::StateUpdate {
        timestamp: 12345,
        entities,
    }
}

// ============================================================================
// Serialization Benchmarks
// ============================================================================

fn bench_client_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("client_message_serialization");

    // PlayerMove serialization
    group.bench_function("player_move_bincode", |b| {
        let msg = create_player_move(12345);
        b.iter(|| {
            black_box(serialize_client_message(&msg, SerializationFormat::Bincode).unwrap())
        });
    });

    // PlayerAction serialization
    group.bench_function("player_action_bincode", |b| {
        let msg = create_player_action(12345);
        b.iter(|| {
            black_box(serialize_client_message(&msg, SerializationFormat::Bincode).unwrap())
        });
    });

    // ChatMessage serialization (short message)
    group.bench_function("chat_short_bincode", |b| {
        let msg = create_chat_message("Hello");
        b.iter(|| {
            black_box(serialize_client_message(&msg, SerializationFormat::Bincode).unwrap())
        });
    });

    // ChatMessage serialization (long message)
    group.bench_function("chat_long_bincode", |b| {
        let long_msg = "This is a much longer chat message that might be sent by players during gameplay.";
        let msg = create_chat_message(long_msg);
        b.iter(|| {
            black_box(serialize_client_message(&msg, SerializationFormat::Bincode).unwrap())
        });
    });

    // Ping serialization
    group.bench_function("ping_bincode", |b| {
        let msg = create_ping(12345);
        b.iter(|| {
            black_box(serialize_client_message(&msg, SerializationFormat::Bincode).unwrap())
        });
    });

    // Handshake serialization
    group.bench_function("handshake_bincode", |b| {
        let msg = create_handshake();
        b.iter(|| {
            black_box(serialize_client_message(&msg, SerializationFormat::Bincode).unwrap())
        });
    });

    group.finish();
}

fn bench_server_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("server_message_serialization");

    // EntitySpawned serialization
    group.bench_function("entity_spawned_bincode", |b| {
        let msg = create_entity_spawned(42);
        b.iter(|| {
            black_box(serialize_server_message(&msg, SerializationFormat::Bincode).unwrap())
        });
    });

    // EntityDespawned serialization
    group.bench_function("entity_despawned_bincode", |b| {
        let msg = create_entity_despawned(42);
        b.iter(|| {
            black_box(serialize_server_message(&msg, SerializationFormat::Bincode).unwrap())
        });
    });

    // EntityTransform serialization
    group.bench_function("entity_transform_bincode", |b| {
        let msg = create_entity_transform(42);
        b.iter(|| {
            black_box(serialize_server_message(&msg, SerializationFormat::Bincode).unwrap())
        });
    });

    group.finish();
}

// ============================================================================
// Deserialization Benchmarks
// ============================================================================

fn bench_client_message_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("client_message_deserialization");

    // PlayerMove deserialization
    group.bench_function("player_move_bincode", |b| {
        let msg = create_player_move(12345);
        let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();
        b.iter(|| {
            black_box(deserialize_client_message(&framed, SerializationFormat::Bincode).unwrap())
        });
    });

    // PlayerAction deserialization
    group.bench_function("player_action_bincode", |b| {
        let msg = create_player_action(12345);
        let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();
        b.iter(|| {
            black_box(deserialize_client_message(&framed, SerializationFormat::Bincode).unwrap())
        });
    });

    // ChatMessage deserialization
    group.bench_function("chat_bincode", |b| {
        let msg = create_chat_message("Hello");
        let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();
        b.iter(|| {
            black_box(deserialize_client_message(&framed, SerializationFormat::Bincode).unwrap())
        });
    });

    group.finish();
}

fn bench_server_message_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("server_message_deserialization");

    // EntitySpawned deserialization
    group.bench_function("entity_spawned_bincode", |b| {
        let msg = create_entity_spawned(42);
        let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();
        b.iter(|| {
            black_box(deserialize_server_message(&framed, SerializationFormat::Bincode).unwrap())
        });
    });

    // EntityTransform deserialization
    group.bench_function("entity_transform_bincode", |b| {
        let msg = create_entity_transform(42);
        let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();
        b.iter(|| {
            black_box(deserialize_server_message(&framed, SerializationFormat::Bincode).unwrap())
        });
    });

    group.finish();
}

// ============================================================================
// Roundtrip Benchmarks
// ============================================================================

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    // PlayerMove roundtrip (most common client message)
    group.bench_function("player_move_bincode", |b| {
        let msg = create_player_move(12345);
        b.iter(|| {
            let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();
            black_box(deserialize_client_message(&framed, SerializationFormat::Bincode).unwrap())
        });
    });

    // EntityTransform roundtrip (most common server message)
    group.bench_function("entity_transform_bincode", |b| {
        let msg = create_entity_transform(42);
        b.iter(|| {
            let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();
            black_box(deserialize_server_message(&framed, SerializationFormat::Bincode).unwrap())
        });
    });

    group.finish();
}

// ============================================================================
// Framing Overhead Benchmarks
// ============================================================================

fn bench_framing(c: &mut Criterion) {
    let mut group = c.benchmark_group("framing");

    // Write framed message
    group.bench_function("write_framed", |b| {
        let msg = create_player_move(12345);
        let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();
        b.iter(|| {
            let mut buffer = Vec::new();
            black_box(framed.write_to(&mut buffer).unwrap());
        });
    });

    // Read framed message
    group.bench_function("read_framed", |b| {
        let msg = create_player_move(12345);
        let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();
        let mut buffer = Vec::new();
        framed.write_to(&mut buffer).unwrap();

        b.iter(|| {
            let mut cursor = Cursor::new(&buffer);
            black_box(FramedMessage::read_from(&mut cursor).unwrap());
        });
    });

    group.finish();
}

// ============================================================================
// Batch Serialization Benchmarks
// ============================================================================

fn bench_batch_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_serialization");

    for size in [1, 10, 100, 1000] {
        group.throughput(Throughput::Elements(size as u64));

        // Batch PlayerMove messages
        group.bench_with_input(
            BenchmarkId::new("player_moves", size),
            &size,
            |b, &size| {
                let messages: Vec<ClientMessage> = (0..size)
                    .map(|i| create_player_move(i as u64))
                    .collect();
                b.iter(|| {
                    for msg in &messages {
                        black_box(serialize_client_message(msg, SerializationFormat::Bincode).unwrap());
                    }
                });
            },
        );

        // Batch EntityTransform messages
        group.bench_with_input(
            BenchmarkId::new("entity_transforms", size),
            &size,
            |b, &size| {
                let messages: Vec<ServerMessage> = (0..size)
                    .map(|i| create_entity_transform(i as u32))
                    .collect();
                b.iter(|| {
                    for msg in &messages {
                        black_box(serialize_server_message(msg, SerializationFormat::Bincode).unwrap());
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// State Update Benchmarks
// ============================================================================

fn bench_state_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_update");

    for entity_count in [10, 100, 1000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", entity_count),
            &entity_count,
            |b, &count| {
                let msg = create_state_update(count);
                b.iter(|| {
                    black_box(serialize_server_message(&msg, SerializationFormat::Bincode).unwrap())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("deserialize", entity_count),
            &entity_count,
            |b, &count| {
                let msg = create_state_update(count);
                let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();
                b.iter(|| {
                    black_box(deserialize_server_message(&framed, SerializationFormat::Bincode).unwrap())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("roundtrip", entity_count),
            &entity_count,
            |b, &count| {
                let msg = create_state_update(count);
                b.iter(|| {
                    let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();
                    black_box(deserialize_server_message(&framed, SerializationFormat::Bincode).unwrap())
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Message Overhead Analysis
// ============================================================================

fn bench_message_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_overhead");

    // Measure bytes per message type
    group.bench_function("measure_player_move", |b| {
        let msg = create_player_move(12345);
        b.iter(|| {
            let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();
            black_box(framed.total_size());
        });
    });

    group.bench_function("measure_entity_transform", |b| {
        let msg = create_entity_transform(42);
        b.iter(|| {
            let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();
            black_box(framed.total_size());
        });
    });

    group.bench_function("measure_ping", |b| {
        let msg = create_ping(12345);
        b.iter(|| {
            let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();
            black_box(framed.total_size());
        });
    });

    group.bench_function("measure_entity_spawned", |b| {
        let msg = create_entity_spawned(42);
        b.iter(|| {
            let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();
            black_box(framed.total_size());
        });
    });

    group.finish();
}

// ============================================================================
// Throughput Benchmarks (messages/sec)
// ============================================================================

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.throughput(Throughput::Elements(1));

    // Client -> Server throughput
    group.bench_function("client_to_server_player_move", |b| {
        let msg = create_player_move(12345);
        b.iter(|| {
            let framed = serialize_client_message(&msg, SerializationFormat::Bincode).unwrap();
            black_box(deserialize_client_message(&framed, SerializationFormat::Bincode).unwrap())
        });
    });

    // Server -> Client throughput
    group.bench_function("server_to_client_entity_transform", |b| {
        let msg = create_entity_transform(42);
        b.iter(|| {
            let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();
            black_box(deserialize_server_message(&framed, SerializationFormat::Bincode).unwrap())
        });
    });

    group.finish();
}

// ============================================================================
// Protocol Version Negotiation
// ============================================================================

fn bench_protocol_negotiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("protocol_negotiation");

    // Handshake + Response roundtrip
    group.bench_function("handshake_roundtrip", |b| {
        let client_msg = create_handshake();
        let server_msg = ServerMessage::HandshakeResponse {
            version: 1,
            server_name: "TestServer".to_string(),
            player_entity: Entity::new(1, 0),
        };

        b.iter(|| {
            // Client -> Server
            let client_framed = serialize_client_message(&client_msg, SerializationFormat::Bincode).unwrap();
            let _client_decoded = deserialize_client_message(&client_framed, SerializationFormat::Bincode).unwrap();

            // Server -> Client
            let server_framed = serialize_server_message(&server_msg, SerializationFormat::Bincode).unwrap();
            black_box(deserialize_server_message(&server_framed, SerializationFormat::Bincode).unwrap())
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_client_message_serialization,
    bench_server_message_serialization,
    bench_client_message_deserialization,
    bench_server_message_deserialization,
    bench_roundtrip,
    bench_framing,
    bench_batch_serialization,
    bench_state_update,
    bench_message_overhead,
    bench_throughput,
    bench_protocol_negotiation,
);

criterion_main!(benches);
