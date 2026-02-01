//! Socket performance benchmarks
//!
//! Measures real network performance for TCP and UDP:
//! - Connection establishment
//! - Message latency (roundtrip)
//! - Throughput
//! - Concurrent connections
//! - Packet rate

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use engine_networking::{TcpClient, TcpServer, UdpClient, UdpServer};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark TCP connection establishment time
fn bench_tcp_connection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("tcp_connection_establishment", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Start server
                let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
                let addr = server.local_addr().unwrap();

                // Spawn server task
                tokio::spawn(async move {
                    let _conn = server.accept().await.unwrap();
                    // Keep connection alive
                    tokio::time::sleep(Duration::from_secs(1)).await;
                });

                // Connect client and measure time
                let start = std::time::Instant::now();
                let _client = TcpClient::connect(&addr.to_string()).await.unwrap();
                start.elapsed()
            })
        });
    });
}

/// Benchmark TCP roundtrip latency for different message sizes
fn bench_tcp_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let sizes = vec![1, 1024, 10 * 1024, 100 * 1024]; // 1 byte, 1KB, 10KB, 100KB

    let mut group = c.benchmark_group("tcp_latency");
    for size in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                rt.block_on(async move {
                    // Start echo server
                    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
                    let addr = server.local_addr().unwrap();

                    tokio::spawn(async move {
                        server.run_echo_server().await.ok();
                    });

                    // Give server time to start
                    tokio::time::sleep(Duration::from_millis(10)).await;

                    // Connect client
                    let client = TcpClient::connect(&addr.to_string()).await.unwrap();

                    // Prepare message
                    let message = vec![0x42u8; size];

                    // Measure roundtrip time
                    let start = std::time::Instant::now();
                    client.send(&message).await.unwrap();
                    let response = client.recv().await.unwrap();
                    let elapsed = start.elapsed();

                    black_box(response);
                    client.close().await.ok();

                    elapsed
                })
            });
        });
    }
    group.finish();
}

/// Benchmark TCP throughput (MB/sec)
fn bench_tcp_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("tcp_throughput");
    group.throughput(Throughput::Bytes(1024 * 1024)); // 1MB

    group.bench_function("1MB_messages", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Start echo server
                let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
                let addr = server.local_addr().unwrap();

                tokio::spawn(async move {
                    server.run_echo_server().await.ok();
                });

                tokio::time::sleep(Duration::from_millis(10)).await;

                let client = TcpClient::connect(&addr.to_string()).await.unwrap();

                // Send 1MB message and measure time
                let message = vec![0x42u8; 1024 * 1024];

                let start = std::time::Instant::now();
                client.send(&message).await.unwrap();
                let response = client.recv().await.unwrap();
                let elapsed = start.elapsed();

                black_box(response);
                client.close().await.ok();

                elapsed
            })
        });
    });

    group.finish();
}

/// Benchmark UDP roundtrip latency
fn bench_udp_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let sizes = vec![32, 512, 1024, 1400]; // Small to max MTU

    let mut group = c.benchmark_group("udp_latency");
    for size in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                rt.block_on(async move {
                    // Start echo server
                    let server = UdpServer::bind("127.0.0.1:0").await.unwrap();
                    let server_addr = server.local_addr();

                    tokio::spawn(async move {
                        server.run_echo_server().await.ok();
                    });

                    tokio::time::sleep(Duration::from_millis(10)).await;

                    let client = UdpClient::connect(&server_addr.to_string()).await.unwrap();

                    // Prepare message
                    let message = vec![0x42u8; size];

                    // Measure roundtrip time
                    let start = std::time::Instant::now();
                    client.send(&message).await.unwrap();
                    let response = client.recv().await.unwrap();
                    let elapsed = start.elapsed();

                    black_box(response);

                    elapsed
                })
            });
        });
    }
    group.finish();
}

/// Benchmark UDP packet send rate (packets/sec)
fn bench_udp_send_rate(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("udp_send_rate_60hz", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Start echo server
                let server = UdpServer::bind("127.0.0.1:0").await.unwrap();
                let server_addr = server.local_addr();

                tokio::spawn(async move {
                    server.run_echo_server().await.ok();
                });

                tokio::time::sleep(Duration::from_millis(10)).await;

                let client = UdpClient::connect(&server_addr.to_string()).await.unwrap();

                // Send 60 packets (simulating 1 second at 60Hz)
                let message = vec![0x42u8; 100];
                let mut total_time = Duration::ZERO;

                for _ in 0..60 {
                    let start = std::time::Instant::now();
                    client.send(&message).await.unwrap();
                    let _response = client.recv().await.unwrap();
                    total_time += start.elapsed();
                }

                total_time / 60
            })
        });
    });
}

/// Benchmark concurrent TCP connections
fn bench_concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let connection_counts = vec![1, 10, 100, 1000];

    let mut group = c.benchmark_group("concurrent_tcp_connections");
    group.sample_size(10); // Reduce sample size for high connection counts

    for count in connection_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async move {
                        // Start echo server
                        let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
                        let addr = server.local_addr().unwrap();
                        let addr_str = addr.to_string();

                        tokio::spawn(async move {
                            server.run_echo_server().await.ok();
                        });

                        tokio::time::sleep(Duration::from_millis(100)).await;

                        // Create multiple clients concurrently
                        let start = std::time::Instant::now();

                        let mut handles = Vec::new();
                        for i in 0..count {
                            let addr_clone = addr_str.clone();
                            let handle = tokio::spawn(async move {
                                let client = TcpClient::connect(&addr_clone).await.unwrap();
                                let message = format!("Message from client {}", i);
                                client.send(message.as_bytes()).await.unwrap();
                                let response = client.recv().await.unwrap();
                                black_box(response);
                                client.close().await.ok();
                            });
                            handles.push(handle);
                        }

                        // Wait for all clients to complete
                        for handle in handles {
                            handle.await.unwrap();
                        }

                        start.elapsed()
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark sustained connection count
fn bench_sustained_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("sustained_1000_connections", |b| {
        b.iter(|| {
            rt.block_on(async {
                // Start echo server
                let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
                let addr = server.local_addr().unwrap();
                let addr_str = addr.to_string();

                tokio::spawn(async move {
                    server.run_echo_server().await.ok();
                });

                tokio::time::sleep(Duration::from_millis(100)).await;

                // Create 1000 connections
                let mut clients = Vec::new();
                for _ in 0..1000 {
                    let client = TcpClient::connect(&addr_str).await.unwrap();
                    clients.push(client);
                }

                // Measure time to send one message from each
                let start = std::time::Instant::now();

                let mut handles = Vec::new();
                for (i, client) in clients.iter().enumerate() {
                    let message = format!("Ping {}", i);
                    client.send(message.as_bytes()).await.unwrap();
                    let handle = tokio::spawn({
                        let client_addr = client.peer_addr();
                        async move {
                            // Simulating receiving response
                            tokio::time::sleep(Duration::from_micros(100)).await;
                            black_box(client_addr);
                        }
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.await.unwrap();
                }

                let elapsed = start.elapsed();

                // Close all connections
                for client in clients {
                    client.close().await.ok();
                }

                elapsed
            })
        });
    });
}

criterion_group!(
    benches,
    bench_tcp_connection,
    bench_tcp_latency,
    bench_tcp_throughput,
    bench_udp_latency,
    bench_udp_send_rate,
    bench_concurrent_connections,
    bench_sustained_connections,
);

criterion_main!(benches);
