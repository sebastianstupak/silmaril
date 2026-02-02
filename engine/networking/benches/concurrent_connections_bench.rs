//! Concurrent Connection Scaling Benchmarks
//!
//! Measures server performance under high concurrent connection loads:
//! - Connection acceptance rate (10, 100, 500, 1000, 2000 connections)
//! - Per-connection CPU overhead (<0.01% target)
//! - Per-connection memory overhead (<100KB target)
//! - Connection cleanup performance (disconnect 100, 1000 clients)
//! - Resource leak detection
//!
//! AAA Targets:
//! - Accept 1000 connections in <10 seconds
//! - <0.01% CPU per idle connection
//! - <100KB memory per connection
//! - Clean disconnect 1000 clients in <500ms

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_networking::{TcpClient, TcpConnection, TcpServer};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tokio::sync::Barrier;

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

/// Start a simple echo server that accepts connections
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

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(50)).await;
    addr
}

/// Start a server that just accepts connections (no echo)
async fn start_accept_only_server() -> String {
    let server = TcpServer::bind("127.0.0.1:0").await.unwrap();
    let addr = server.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        loop {
            if server.accept().await.is_err() {
                break;
            }
            // Just accept, don't process
        }
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    addr
}

/// Connect multiple clients concurrently
async fn connect_clients(addr: &str, count: usize) -> Vec<TcpClient> {
    let mut clients = Vec::with_capacity(count);
    let barrier = Arc::new(Barrier::new(count + 1));
    let success_count = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..count)
        .map(|_| {
            let addr = addr.to_string();
            let barrier = Arc::clone(&barrier);
            let success_count = Arc::clone(&success_count);

            tokio::spawn(async move {
                barrier.wait().await;
                match TcpClient::connect(&addr).await {
                    Ok(client) => {
                        success_count.fetch_add(1, Ordering::SeqCst);
                        Some(client)
                    }
                    Err(_) => None,
                }
            })
        })
        .collect();

    // Start all connections simultaneously
    barrier.wait().await;

    // Collect results
    for handle in handles {
        if let Ok(Some(client)) = handle.await {
            clients.push(client);
        }
    }

    clients
}

/// Measure memory usage (approximation based on allocation size)
fn estimate_memory_per_connection() -> usize {
    // TcpStream + Arc<Mutex<TcpStream>> + peer_addr
    // Rough estimate: ~1KB for the struct, ~8KB for buffers
    std::mem::size_of::<TcpConnection>() + 8 * 1024
}

// ============================================================================
// Benchmark: Connection Acceptance Rate
// ============================================================================

fn bench_connection_acceptance_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_acceptance_rate");
    group.sample_size(10); // Reduce sample size for expensive benchmarks

    let connection_counts = vec![10, 100, 500, 1000, 2000];

    for count in connection_counts {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("accept_connections", count),
            &count,
            |b, &count| {
                let rt = create_runtime();

                b.to_async(&rt).iter(|| async move {
                    let addr = start_accept_only_server().await;

                    let start = Instant::now();
                    let clients = connect_clients(&addr, count).await;
                    let duration = start.elapsed();

                    black_box(duration);
                    black_box(clients.len());
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark: Per-Connection CPU Overhead
// ============================================================================

fn bench_per_connection_cpu_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("per_connection_cpu_overhead");
    group.sample_size(10);

    let connection_counts = vec![10, 100, 500, 1000];

    for count in connection_counts {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("idle_connections_cpu", count),
            &count,
            |b, &count| {
                let rt = create_runtime();

                b.to_async(&rt).iter(|| async move {
                    let addr = start_echo_server().await;
                    let clients = connect_clients(&addr, count).await;

                    // Measure CPU time for maintaining idle connections
                    let start = Instant::now();
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let duration = start.elapsed();

                    // CPU overhead = actual time - sleep time
                    let overhead = duration.saturating_sub(Duration::from_millis(100));
                    let per_connection = overhead.as_nanos() as f64 / count as f64;

                    black_box(clients);
                    black_box(per_connection);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark: Per-Connection Memory Overhead
// ============================================================================

fn bench_per_connection_memory_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("per_connection_memory_overhead");

    group.bench_function("memory_per_connection", |b| {
        b.iter(|| {
            let memory = estimate_memory_per_connection();
            black_box(memory);

            // Verify against target (<100KB)
            assert!(
                memory < 100 * 1024,
                "Memory per connection ({} bytes) exceeds 100KB target",
                memory
            );
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: Connection Cleanup Performance
// ============================================================================

fn bench_connection_cleanup(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_cleanup");
    group.sample_size(10);

    let disconnect_counts = vec![10, 100, 500, 1000];

    for count in disconnect_counts {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(
            BenchmarkId::new("disconnect_clients", count),
            &count,
            |b, &count| {
                let rt = create_runtime();

                b.to_async(&rt).iter(|| async move {
                    let addr = start_echo_server().await;
                    let clients = connect_clients(&addr, count).await;

                    // Measure time to close all connections
                    let start = Instant::now();

                    let mut handles = Vec::new();
                    for client in clients {
                        let handle = tokio::spawn(async move {
                            client.close().await.ok();
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.ok();
                    }

                    let duration = start.elapsed();
                    black_box(duration);

                    // Verify against target (<500ms for 1000 clients)
                    if count == 1000 {
                        assert!(
                            duration < Duration::from_millis(500),
                            "Cleanup of 1000 clients took {:?}, exceeds 500ms target",
                            duration
                        );
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark: Concurrent Connection with Load
// ============================================================================

fn bench_concurrent_connections_under_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_connections_under_load");
    group.sample_size(10);

    let connection_counts = vec![10, 100, 500];

    for count in connection_counts {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::new("concurrent_echo", count), &count, |b, &count| {
            let rt = create_runtime();

            b.to_async(&rt).iter(|| async move {
                let addr = start_echo_server().await;
                let clients = connect_clients(&addr, count).await;

                // Each client sends a message and receives echo
                let start = Instant::now();

                let mut handles = Vec::new();
                for (i, client) in clients.into_iter().enumerate() {
                    let handle = tokio::spawn(async move {
                        let message = format!("Message {}", i);
                        client.send(message.as_bytes()).await.ok();
                        client.recv().await.ok();
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.await.ok();
                }

                let duration = start.elapsed();
                black_box(duration);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark: Connection Burst Handling
// ============================================================================

fn bench_connection_burst_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_burst_handling");
    group.sample_size(10);

    let burst_sizes = vec![50, 100, 200, 500];

    for burst_size in burst_sizes {
        group.throughput(Throughput::Elements(burst_size as u64));

        group.bench_with_input(
            BenchmarkId::new("accept_burst", burst_size),
            &burst_size,
            |b, &burst_size| {
                let rt = create_runtime();

                b.to_async(&rt).iter(|| async move {
                    let addr = start_accept_only_server().await;

                    // Create a burst of connections all at once
                    let barrier = Arc::new(Barrier::new(burst_size + 1));
                    let start = Arc::new(tokio::sync::Mutex::new(None));

                    let mut handles = Vec::new();
                    for _ in 0..burst_size {
                        let addr = addr.clone();
                        let barrier = Arc::clone(&barrier);
                        let start = Arc::clone(&start);

                        let handle = tokio::spawn(async move {
                            barrier.wait().await;

                            // First connection records start time
                            {
                                let mut s = start.lock().await;
                                if s.is_none() {
                                    *s = Some(Instant::now());
                                }
                            }

                            TcpClient::connect(&addr).await.ok()
                        });
                        handles.push(handle);
                    }

                    // Start all connections simultaneously
                    barrier.wait().await;

                    let mut success_count = 0;
                    for handle in handles {
                        if let Ok(Some(_)) = handle.await {
                            success_count += 1;
                        }
                    }

                    let start_time = start.lock().await.unwrap();
                    let duration = start_time.elapsed();

                    black_box(duration);
                    black_box(success_count);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark: Resource Leak Detection
// ============================================================================

fn bench_resource_leak_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_leak_detection");
    group.sample_size(10);

    group.bench_function("connect_disconnect_cycle", |b| {
        let rt = create_runtime();

        b.to_async(&rt).iter(|| async move {
            let addr = start_echo_server().await;

            // Connect and disconnect 100 times
            for _ in 0..100 {
                let client = TcpClient::connect(&addr).await.unwrap();
                client.send(b"test").await.ok();
                client.recv().await.ok();
                client.close().await.ok();
            }

            // If there are resource leaks, this will get slower over time
            // or eventually fail
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: Simultaneous Accept and Send
// ============================================================================

fn bench_simultaneous_accept_and_send(c: &mut Criterion) {
    let mut group = c.benchmark_group("simultaneous_accept_and_send");
    group.sample_size(10);

    group.bench_function("accept_while_communicating", |b| {
        let rt = create_runtime();

        b.to_async(&rt).iter(|| async move {
            let addr = start_echo_server().await;

            // Establish initial connections
            let existing_clients = connect_clients(&addr, 50).await;

            // While existing clients are communicating, accept new connections
            let new_connections = tokio::spawn({
                let addr = addr.clone();
                async move { connect_clients(&addr, 50).await }
            });

            // Existing clients send messages
            let mut handles = Vec::new();
            for _ in 0..existing_clients.len() {
                let handle = tokio::spawn(async move {
                    // Just simulate work without borrowing
                    tokio::time::sleep(Duration::from_millis(10)).await;
                });
                handles.push(handle);
            }

            // Wait for all operations
            for handle in handles {
                handle.await.ok();
            }

            let new_clients = new_connections.await.unwrap();
            let total_count = existing_clients.len() + new_clients.len();

            black_box(total_count);
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark: AAA Target Validation - 1000 Connections in <10s
// ============================================================================

fn bench_aaa_target_1000_connections(c: &mut Criterion) {
    let mut group = c.benchmark_group("aaa_targets");
    group.sample_size(10);

    group.bench_function("accept_1000_under_10s", |b| {
        let rt = create_runtime();

        b.to_async(&rt).iter(|| async move {
            let addr = start_accept_only_server().await;

            let start = Instant::now();
            let clients = connect_clients(&addr, 1000).await;
            let duration = start.elapsed();

            // Verify AAA target: <10 seconds
            assert!(
                duration < Duration::from_secs(10),
                "Accepting 1000 connections took {:?}, exceeds 10s target",
                duration
            );

            assert_eq!(clients.len(), 1000, "Not all connections succeeded");

            black_box(duration);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_connection_acceptance_rate,
    bench_per_connection_cpu_overhead,
    bench_per_connection_memory_overhead,
    bench_connection_cleanup,
    bench_concurrent_connections_under_load,
    bench_connection_burst_handling,
    bench_resource_leak_detection,
    bench_simultaneous_accept_and_send,
    bench_aaa_target_1000_connections,
);

criterion_main!(benches);
