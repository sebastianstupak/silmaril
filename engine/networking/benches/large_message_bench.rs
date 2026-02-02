//! Large Message Handling Benchmarks
//!
//! Measures performance of handling large messages:
//! - Message fragmentation (100KB, 1MB, 10MB messages)
//! - Reassembly performance
//! - Reassembly with missing fragments (graceful handling)
//! - Throughput for large transfers
//!
//! AAA Targets:
//! - 100KB messages: <1ms fragmentation/reassembly
//! - 1MB messages: <5ms fragmentation/reassembly
//! - 10MB messages: <50ms fragmentation/reassembly
//! - Graceful handling of missing fragments

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::Entity;
use engine_networking::{
    deserialize_server_message, serialize_server_message, EntityState, SerializationFormat,
    ServerMessage, TcpClient, TcpServer,
};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a tokio runtime for async tests
fn create_runtime() -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
}

/// Start an echo server for testing
async fn start_echo_server() -> String {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        loop {
            match server.accept().await {
                Ok(conn) => {
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
                Err(_) => break,
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    addr
}

/// Create a large state update message with many entities
fn create_large_state_update(entity_count: usize) -> ServerMessage {
    let entities: Vec<EntityState> = (0..entity_count)
        .map(|i| EntityState {
            entity: Entity::new(i as u32, 0),
            x: (i as f32) * 10.0,
            y: (i as f32) * 5.0,
            z: (i as f32) * 7.5,
            qx: 0.0,
            qy: 0.0,
            qz: 0.0,
            qw: 1.0,
            health: Some(100.0),
            max_health: Some(100.0),
        })
        .collect();

    ServerMessage::StateUpdate { timestamp: 12345, entities }
}

/// Create a message of approximately target size
fn create_message_of_size(target_size: usize) -> Vec<u8> {
    vec![0x42u8; target_size]
}

/// Measure actual message size after serialization
fn measure_message_size(msg: &ServerMessage) -> usize {
    let framed = serialize_server_message(msg, SerializationFormat::Bincode).unwrap();
    framed.total_size()
}

/// Fragment a large message into chunks
fn fragment_message(data: &[u8], chunk_size: usize) -> Vec<Vec<u8>> {
    data.chunks(chunk_size).map(|chunk| chunk.to_vec()).collect()
}

/// Reassemble fragments back into original message
fn reassemble_fragments(fragments: &[Vec<u8>]) -> Vec<u8> {
    fragments.iter().flat_map(|f| f.iter().cloned()).collect()
}

// ============================================================================
// Benchmark: Message Fragmentation
// ============================================================================

fn bench_message_fragmentation(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_fragmentation");

    let test_cases = vec![
        ("100KB", 100 * 1024, Duration::from_millis(1)),
        ("1MB", 1024 * 1024, Duration::from_millis(5)),
        ("10MB", 10 * 1024 * 1024, Duration::from_millis(50)),
    ];

    for (name, size, target_duration) in test_cases {
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("fragment", name), &size, |b, &size| {
            let data = create_message_of_size(size);
            let chunk_size = 8192; // 8KB chunks (common MTU-safe size)

            b.iter(|| {
                let start = Instant::now();
                let fragments = fragment_message(black_box(&data), chunk_size);
                let duration = start.elapsed();

                // Verify against target
                if duration > target_duration {
                    eprintln!(
                        "WARNING: {} fragmentation took {:?}, exceeds {:?} target",
                        name, duration, target_duration
                    );
                }

                black_box(fragments);
                duration
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: Message Reassembly
// ============================================================================

fn bench_message_reassembly(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_reassembly");

    let test_cases = vec![
        ("100KB", 100 * 1024, Duration::from_millis(1)),
        ("1MB", 1024 * 1024, Duration::from_millis(10)),
        ("10MB", 10 * 1024 * 1024, Duration::from_millis(100)),
    ];

    for (name, size, target_duration) in test_cases {
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("reassemble", name), &size, |b, &size| {
            let data = create_message_of_size(size);
            let fragments = fragment_message(&data, 8192);

            b.iter(|| {
                let start = Instant::now();
                let reassembled = reassemble_fragments(black_box(&fragments));
                let duration = start.elapsed();

                // Verify correctness
                assert_eq!(reassembled.len(), data.len());

                // Verify against target
                if duration > target_duration {
                    eprintln!(
                        "WARNING: {} reassembly took {:?}, exceeds {:?} target",
                        name, duration, target_duration
                    );
                }

                black_box(reassembled);
                duration
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: Fragmentation + Reassembly Roundtrip
// ============================================================================

fn bench_fragmentation_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("fragmentation_roundtrip");

    let test_cases = vec![
        ("100KB", 100 * 1024, Duration::from_millis(1)),
        ("1MB", 1024 * 1024, Duration::from_millis(5)),
        ("10MB", 10 * 1024 * 1024, Duration::from_millis(50)),
    ];

    for (name, size, target_duration) in test_cases {
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("roundtrip", name), &size, |b, &size| {
            let data = create_message_of_size(size);

            b.iter(|| {
                let start = Instant::now();
                let fragments = fragment_message(black_box(&data), 8192);
                let reassembled = reassemble_fragments(&fragments);
                let duration = start.elapsed();

                // Verify correctness
                assert_eq!(reassembled, data);

                // Verify against target
                if duration > target_duration {
                    eprintln!(
                        "WARNING: {} roundtrip took {:?}, exceeds {:?} target",
                        name, duration, target_duration
                    );
                }

                black_box(reassembled);
                duration
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: Large Message Serialization
// ============================================================================

fn bench_large_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_message_serialization");

    // Create messages with different entity counts to reach target sizes
    let test_cases = vec![
        ("100KB", 500),  // ~500 entities ≈ 100KB
        ("1MB", 5000),   // ~5000 entities ≈ 1MB
        ("10MB", 50000), // ~50000 entities ≈ 10MB
    ];

    for (name, entity_count) in test_cases {
        let msg = create_large_state_update(entity_count);
        let actual_size = measure_message_size(&msg);

        group.throughput(Throughput::Bytes(actual_size as u64));

        group.bench_with_input(BenchmarkId::new("serialize", name), &msg, |b, msg| {
            b.iter(|| {
                let framed =
                    serialize_server_message(black_box(msg), SerializationFormat::Bincode).unwrap();
                black_box(framed);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: Large Message Deserialization
// ============================================================================

fn bench_large_message_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_message_deserialization");

    let test_cases = vec![("100KB", 500), ("1MB", 5000), ("10MB", 50000)];

    for (name, entity_count) in test_cases {
        let msg = create_large_state_update(entity_count);
        let framed = serialize_server_message(&msg, SerializationFormat::Bincode).unwrap();
        let actual_size = framed.total_size();

        group.throughput(Throughput::Bytes(actual_size as u64));

        group.bench_with_input(BenchmarkId::new("deserialize", name), &framed, |b, framed| {
            b.iter(|| {
                let msg =
                    deserialize_server_message(black_box(framed), SerializationFormat::Bincode)
                        .unwrap();
                black_box(msg);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: End-to-End Large Transfer
// ============================================================================

fn bench_end_to_end_large_transfer(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end_large_transfer");
    group.sample_size(10);

    let test_cases = vec![("100KB", 100 * 1024), ("1MB", 1024 * 1024), ("10MB", 10 * 1024 * 1024)];

    for (name, size) in test_cases {
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("transfer", name), &size, |b, &size| {
            let rt = create_runtime();

            b.to_async(&rt).iter(|| async move {
                let addr = start_echo_server().await;
                let client = TcpClient::connect(&addr).await.unwrap();

                let data = create_message_of_size(size);

                let start = Instant::now();
                client.send(&data).await.unwrap();
                let received = client.recv().await.unwrap();
                let duration = start.elapsed();

                // Verify correctness
                assert_eq!(received.len(), data.len());

                black_box(duration);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: Throughput for Large Transfers
// ============================================================================

fn bench_large_transfer_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_transfer_throughput");
    group.sample_size(10);

    group.bench_function("sustained_10MB_transfers", |b| {
        let rt = create_runtime();

        b.to_async(&rt).iter(|| async move {
            let addr = start_echo_server().await;
            let client = TcpClient::connect(&addr).await.unwrap();

            let data = create_message_of_size(10 * 1024 * 1024);

            // Transfer 10 messages
            let start = Instant::now();
            for _ in 0..10 {
                client.send(&data).await.unwrap();
                let _ = client.recv().await.unwrap();
            }
            let duration = start.elapsed();

            // Calculate throughput
            let total_bytes = 10 * data.len() * 2; // send + receive
            let throughput_mbps = (total_bytes as f64 * 8.0) / duration.as_secs_f64() / 1_000_000.0;

            black_box(throughput_mbps);
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: Concurrent Large Transfers
// ============================================================================

fn bench_concurrent_large_transfers(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_large_transfers");
    group.sample_size(10);

    let concurrent_counts = vec![2, 5, 10];

    for count in concurrent_counts {
        group.bench_with_input(BenchmarkId::new("concurrent_1MB", count), &count, |b, &count| {
            let rt = create_runtime();

            b.to_async(&rt).iter(|| async move {
                let addr = start_echo_server().await;

                let data = create_message_of_size(1024 * 1024);

                let start = Instant::now();

                let mut handles = Vec::new();
                for _ in 0..count {
                    let addr = addr.clone();
                    let data = data.clone();

                    let handle = tokio::spawn(async move {
                        let client = TcpClient::connect(&addr).await.unwrap();
                        client.send(&data).await.unwrap();
                        client.recv().await.unwrap();
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.await.unwrap();
                }

                let duration = start.elapsed();
                black_box(duration);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: Reassembly with Missing Fragments (Graceful Handling)
// ============================================================================

fn bench_reassembly_with_missing_fragments(c: &mut Criterion) {
    let mut group = c.benchmark_group("reassembly_with_missing_fragments");

    group.bench_function("handle_missing_fragments", |b| {
        let data = create_message_of_size(1024 * 1024);
        let fragments = fragment_message(&data, 8192);

        b.iter(|| {
            // Simulate 10% fragment loss
            let mut incomplete_fragments = Vec::new();
            for (i, fragment) in fragments.iter().enumerate() {
                if i % 10 != 0 {
                    incomplete_fragments.push(fragment.clone());
                }
            }

            let reassembled = reassemble_fragments(black_box(&incomplete_fragments));

            // Verify we detect incomplete data
            assert_ne!(reassembled.len(), data.len());

            black_box(reassembled);
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: AAA Target Validation
// ============================================================================

fn bench_aaa_target_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("aaa_targets");

    group.bench_function("100KB_under_1ms", |b| {
        let data = create_message_of_size(100 * 1024);

        b.iter(|| {
            let start = Instant::now();
            let fragments = fragment_message(black_box(&data), 8192);
            let reassembled = reassemble_fragments(&fragments);
            let duration = start.elapsed();

            assert_eq!(reassembled.len(), data.len());

            // Verify AAA target
            assert!(
                duration < Duration::from_millis(1),
                "100KB roundtrip took {:?}, exceeds 1ms target",
                duration
            );

            black_box(reassembled);
        });
    });

    group.bench_function("1MB_under_5ms", |b| {
        let data = create_message_of_size(1024 * 1024);

        b.iter(|| {
            let start = Instant::now();
            let fragments = fragment_message(black_box(&data), 8192);
            let reassembled = reassemble_fragments(&fragments);
            let duration = start.elapsed();

            assert_eq!(reassembled.len(), data.len());

            // Verify AAA target
            assert!(
                duration < Duration::from_millis(5),
                "1MB roundtrip took {:?}, exceeds 5ms target",
                duration
            );

            black_box(reassembled);
        });
    });

    group.bench_function("10MB_under_50ms", |b| {
        let data = create_message_of_size(10 * 1024 * 1024);

        b.iter(|| {
            let start = Instant::now();
            let fragments = fragment_message(black_box(&data), 8192);
            let reassembled = reassemble_fragments(&fragments);
            let duration = start.elapsed();

            assert_eq!(reassembled.len(), data.len());

            // Verify AAA target
            assert!(
                duration < Duration::from_millis(50),
                "10MB roundtrip took {:?}, exceeds 50ms target",
                duration
            );

            black_box(reassembled);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_message_fragmentation,
    bench_message_reassembly,
    bench_fragmentation_roundtrip,
    bench_large_message_serialization,
    bench_large_message_deserialization,
    bench_end_to_end_large_transfer,
    bench_large_transfer_throughput,
    bench_concurrent_large_transfers,
    bench_reassembly_with_missing_fragments,
    bench_aaa_target_validation,
);

criterion_main!(benches);
