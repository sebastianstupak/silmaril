//! Benchmarks for asset network transfer.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_assets::{
    AssetId, AssetNetworkClient, AssetNetworkMessage, AssetNetworkServer, TransferPriority,
};

fn bench_small_asset_transfer(c: &mut Criterion) {
    let mut group = c.benchmark_group("small_asset_transfer");

    for size in [100usize, 1024, 10 * 1024] {
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let mut server = AssetNetworkServer::new(1024 * 1024);
                let mut client = AssetNetworkClient::new(4);

                let id = AssetId::from_content(&size.to_le_bytes());
                let data = vec![0x42u8; size];

                server.register_asset(id, data.clone());
                client.request_asset(id, TransferPriority::Critical);

                let request = client.next_request().unwrap();
                let responses = server.handle_request(request);

                for response in responses {
                    client.handle_message(response).unwrap();
                }

                black_box(client.take_completed(id).unwrap());
            });
        });
    }

    group.finish();
}

fn bench_large_asset_transfer(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_asset_transfer");

    for size in [100 * 1024usize, 1024 * 1024, 5 * 1024 * 1024] {
        group.throughput(Throughput::Bytes(size as u64));
        group.sample_size(10); // Fewer samples for large transfers

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let mut server = AssetNetworkServer::new(1024 * 1024);
                let mut client = AssetNetworkClient::new(4);

                let id = AssetId::from_content(&size.to_le_bytes());
                let data = vec![0x42u8; size];

                server.register_asset(id, data.clone());
                client.request_asset(id, TransferPriority::Critical);

                let request = client.next_request().unwrap();
                let responses = server.handle_request(request);

                for response in responses {
                    client.handle_message(response).unwrap();
                }

                black_box(client.take_completed(id).unwrap());
            });
        });
    }

    group.finish();
}

fn bench_compression_ratios(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratios");

    // Highly compressible data (text)
    let text_data = b"This is a test string that should compress very well. ".repeat(100);

    // Random data (incompressible)
    let random_data: Vec<u8> = (0..5000).map(|i| (i % 256) as u8).collect();

    group.bench_function("compressible_text", |b| {
        b.iter(|| {
            let mut server = AssetNetworkServer::new(1024 * 1024);
            let id = AssetId::from_content(b"text");
            server.register_asset(id, text_data.clone());

            let request = AssetNetworkMessage::Request { asset_id: id, resume_offset: None };
            black_box(server.handle_request(request));
        });
    });

    group.bench_function("incompressible_random", |b| {
        b.iter(|| {
            let mut server = AssetNetworkServer::new(1024 * 1024);
            let id = AssetId::from_content(b"random");
            server.register_asset(id, random_data.clone());

            let request = AssetNetworkMessage::Request { asset_id: id, resume_offset: None };
            black_box(server.handle_request(request));
        });
    });

    group.finish();
}

fn bench_checksum_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("checksum_validation");

    for size in [1024usize, 10 * 1024, 100 * 1024, 1024 * 1024] {
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let data = vec![0x42u8; size];
            let checksum = *blake3::hash(&data).as_bytes();

            b.iter(|| {
                let computed = *blake3::hash(black_box(&data)).as_bytes();
                black_box(computed == checksum);
            });
        });
    }

    group.finish();
}

fn bench_concurrent_requests(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_requests");

    for num_clients in [1, 10, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_clients),
            &num_clients,
            |b, &num_clients| {
                b.iter(|| {
                    let mut server = AssetNetworkServer::new(1024 * 1024);
                    let data = vec![0x42u8; 1024];

                    // Register assets
                    for i in 0u32..num_clients {
                        let id = AssetId::from_content(&i.to_le_bytes());
                        server.register_asset(id, data.clone());
                    }

                    // Simulate concurrent client requests
                    for i in 0u32..num_clients {
                        let id = AssetId::from_content(&i.to_le_bytes());
                        let request =
                            AssetNetworkMessage::Request { asset_id: id, resume_offset: None };
                        black_box(server.handle_request(request));
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_chunked_transfer(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunked_transfer");

    let chunk_sizes = [64 * 1024, 256 * 1024, 1024 * 1024];
    let asset_size = 5 * 1024 * 1024; // 5MB asset

    for chunk_size in chunk_sizes {
        group.throughput(Throughput::Bytes(asset_size));
        group.sample_size(10);

        group.bench_with_input(
            BenchmarkId::from_parameter(chunk_size),
            &chunk_size,
            |b, &chunk_size| {
                b.iter(|| {
                    let mut server = AssetNetworkServer::new(chunk_size);
                    let mut client = AssetNetworkClient::new(4);

                    let id = AssetId::from_content(b"large");
                    let data = vec![0x42u8; asset_size as usize];

                    server.register_asset(id, data.clone());
                    client.request_asset(id, TransferPriority::Critical);

                    let request = client.next_request().unwrap();
                    let responses = server.handle_request(request);

                    for response in responses {
                        client.handle_message(response).unwrap();
                    }

                    black_box(client.take_completed(id).unwrap());
                });
            },
        );
    }

    group.finish();
}

fn bench_priority_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("priority_queue");

    for num_assets in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_assets),
            &num_assets,
            |b, &num_assets| {
                b.iter(|| {
                    let mut client = AssetNetworkClient::new(100);

                    // Add assets with random priorities
                    for i in 0u32..num_assets {
                        let id = AssetId::from_content(&i.to_le_bytes());
                        let priority = match i % 4 {
                            0 => TransferPriority::Critical,
                            1 => TransferPriority::High,
                            2 => TransferPriority::Normal,
                            _ => TransferPriority::Low,
                        };
                        client.request_asset(id, priority);
                    }

                    // Dequeue all
                    while client.next_request().is_some() {}

                    black_box(&client);
                });
            },
        );
    }

    group.finish();
}

fn bench_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_serialization");

    let messages = vec![
        (
            "request",
            AssetNetworkMessage::Request {
                asset_id: AssetId::from_content(b"test"),
                resume_offset: None,
            },
        ),
        (
            "response_small",
            AssetNetworkMessage::Response {
                asset_id: AssetId::from_content(b"test"),
                data: vec![0x42u8; 100],
                checksum: [0u8; 32],
                compressed: false,
            },
        ),
        (
            "response_large",
            AssetNetworkMessage::Response {
                asset_id: AssetId::from_content(b"test"),
                data: vec![0x42u8; 10000],
                checksum: [0u8; 32],
                compressed: false,
            },
        ),
        (
            "chunk",
            AssetNetworkMessage::Chunk {
                asset_id: AssetId::from_content(b"test"),
                offset: 1024,
                total_size: 5000,
                data: vec![0x42u8; 1000],
                compressed: false,
            },
        ),
    ];

    for (name, msg) in messages {
        group.bench_function(format!("serialize_{}", name), |b| {
            b.iter(|| {
                black_box(bincode::serialize(&msg).unwrap());
            });
        });

        let bytes = bincode::serialize(&msg).unwrap();
        group.bench_function(format!("deserialize_{}", name), |b| {
            b.iter(|| {
                black_box(bincode::deserialize::<AssetNetworkMessage>(&bytes).unwrap());
            });
        });
    }

    group.finish();
}

fn bench_resumable_download(c: &mut Criterion) {
    let mut group = c.benchmark_group("resumable_download");

    let asset_size = 1024 * 1024; // 1MB
    let chunk_size = 100 * 1024; // 100KB chunks

    group.bench_function("resume_50_percent", |b| {
        b.iter(|| {
            let mut server = AssetNetworkServer::new(chunk_size);
            let mut client = AssetNetworkClient::new(4);

            let id = AssetId::from_content(b"resume");
            let data = vec![0x42u8; asset_size];

            server.register_asset(id, data.clone());

            // Simulate 50% downloaded
            client.chunk_buffers.insert(id, vec![0x42u8; asset_size / 2]);

            client.request_asset(id, TransferPriority::Critical);
            let request = client.next_request().unwrap();
            let responses = server.handle_request(request);

            for response in responses {
                client.handle_message(response).unwrap();
            }

            black_box(client.take_completed(id).unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_small_asset_transfer,
    bench_large_asset_transfer,
    bench_compression_ratios,
    bench_checksum_validation,
    bench_concurrent_requests,
    bench_chunked_transfer,
    bench_priority_queue,
    bench_message_serialization,
    bench_resumable_download,
);
criterion_main!(benches);
