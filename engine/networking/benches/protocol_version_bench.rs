//! Benchmarks for protocol version checking (Quick Win #4)
//!
//! Measures the overhead of version validation in the handshake process.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use engine_core::ecs::World;
use engine_networking::{
    deserialize_client_message, serialize_client_message, ClientMessage, SerializationFormat,
    PROTOCOL_VERSION,
};

fn bench_version_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("protocol_version_check");

    // Benchmark handshake message creation with version
    group.bench_function("create_handshake_with_version", |b| {
        b.iter(|| {
            ClientMessage::Handshake {
                version: black_box(PROTOCOL_VERSION),
                client_name: black_box("BenchClient".to_string()),
            }
        });
    });

    // Benchmark handshake serialization
    group.bench_function("serialize_handshake", |b| {
        let handshake = ClientMessage::Handshake {
            version: PROTOCOL_VERSION,
            client_name: "BenchClient".to_string(),
        };

        b.iter(|| {
            serialize_client_message(black_box(&handshake), SerializationFormat::Bincode)
        });
    });

    // Benchmark handshake deserialization
    group.bench_function("deserialize_handshake", |b| {
        let handshake = ClientMessage::Handshake {
            version: PROTOCOL_VERSION,
            client_name: "BenchClient".to_string(),
        };
        let serialized = serialize_client_message(&handshake, SerializationFormat::Bincode).unwrap();

        b.iter(|| {
            deserialize_client_message(black_box(&serialized), SerializationFormat::Bincode)
        });
    });

    // Benchmark version comparison (the actual check)
    group.bench_function("version_comparison", |b| {
        let client_version = PROTOCOL_VERSION;
        let server_version = PROTOCOL_VERSION;

        b.iter(|| {
            black_box(client_version) == black_box(server_version)
        });
    });

    // Benchmark version mismatch detection
    group.bench_function("version_mismatch_detection", |b| {
        let client_version = PROTOCOL_VERSION + 1;
        let server_version = PROTOCOL_VERSION;

        b.iter(|| {
            let matches = black_box(client_version) == black_box(server_version);
            if !matches {
                // Simulate error path
                black_box(format!(
                    "Version mismatch: client={}, server={}",
                    client_version, server_version
                ));
            }
        });
    });

    group.finish();
}

fn bench_handshake_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("handshake_throughput");
    group.throughput(Throughput::Elements(1));

    // Full handshake message round-trip
    group.bench_function("full_handshake_roundtrip", |b| {
        b.iter(|| {
            let handshake = ClientMessage::Handshake {
                version: black_box(PROTOCOL_VERSION),
                client_name: black_box("BenchClient".to_string()),
            };

            let serialized = serialize_client_message(&handshake, SerializationFormat::Bincode).unwrap();
            let deserialized = deserialize_client_message(&serialized, SerializationFormat::Bincode).unwrap();

            match deserialized {
                ClientMessage::Handshake { version, .. } => {
                    assert_eq!(version, PROTOCOL_VERSION);
                }
                _ => panic!("Expected handshake"),
            }
        });
    });

    // Measure overhead vs baseline (no version check)
    group.bench_function("handshake_without_version_check", |b| {
        b.iter(|| {
            let handshake = ClientMessage::Handshake {
                version: black_box(PROTOCOL_VERSION),
                client_name: black_box("BenchClient".to_string()),
            };

            let serialized = serialize_client_message(&handshake, SerializationFormat::Bincode).unwrap();
            let _deserialized = deserialize_client_message(&serialized, SerializationFormat::Bincode).unwrap();
            // Skip version check
        });
    });

    group.finish();
}

criterion_group!(benches, bench_version_check, bench_handshake_throughput);
criterion_main!(benches);
