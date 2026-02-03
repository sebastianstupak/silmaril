//! E2E connectivity benchmarks (Quick Win #3)
//!
//! Measures end-to-end connection performance including handshake and disconnection.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use engine_networking::{TcpServer, TcpClient, ServerLoop};
use engine_core::ecs::World;
use std::time::Duration;

fn bench_single_connection(c: &mut Criterion) {
    let mut group = c.benchmark_group("e2e_single_connection");

    // Note: These are async benchmarks, so we use tokio runtime
    let runtime = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("connect_and_disconnect", |b| {
        b.to_async(&runtime).iter(|| async {
            // Start server
            let world = World::new();
            let server_loop = ServerLoop::new(world);
            let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
            let addr = tcp_server.local_addr().unwrap();
            server_loop.start_accepting(tcp_server).await;

            // Connect client
            let client = TcpClient::connect(&addr.to_string()).await.unwrap();

            // Wait for connection to stabilize
            tokio::time::sleep(Duration::from_millis(50)).await;

            // Disconnect
            client.close().await.unwrap();
        });
    });

    group.finish();
}

fn bench_concurrent_connections(c: &mut Criterion) {
    let mut group = c.benchmark_group("e2e_concurrent_connections");

    let runtime = tokio::runtime::Runtime::new().unwrap();

    for client_count in [5, 10, 25, 50].iter() {
        group.throughput(Throughput::Elements(*client_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(client_count),
            client_count,
            |b, &count| {
                b.to_async(&runtime).iter(|| async move {
                    // Start server
                    let world = World::new();
                    let server_loop = ServerLoop::new(world);
                    let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
                    let addr = tcp_server.local_addr().unwrap();
                    server_loop.start_accepting(tcp_server).await;

                    // Connect multiple clients concurrently
                    let mut handles = Vec::new();
                    for _ in 0..count {
                        let addr_clone = addr.to_string();
                        handles.push(tokio::spawn(async move {
                            TcpClient::connect(&addr_clone).await.unwrap()
                        }));
                    }

                    // Wait for all connections
                    let clients = futures::future::join_all(handles).await;

                    // Brief stabilization
                    tokio::time::sleep(Duration::from_millis(100)).await;

                    // Disconnect all
                    for client_result in clients {
                        client_result.unwrap().close().await.ok();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_connection_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("e2e_connection_throughput");
    group.throughput(Throughput::Elements(1));

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Measure how fast we can establish connections sequentially
    group.bench_function("sequential_connections", |b| {
        b.to_async(&runtime).iter(|| async {
            // Start server
            let world = World::new();
            let server_loop = ServerLoop::new(world);
            let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
            let addr = tcp_server.local_addr().unwrap();
            server_loop.start_accepting(tcp_server).await;

            // Connect 10 clients sequentially
            for _ in 0..10 {
                let client = TcpClient::connect(&addr.to_string()).await.unwrap();
                client.close().await.ok();
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_connection,
    bench_concurrent_connections,
    bench_connection_throughput
);
criterion_main!(benches);
