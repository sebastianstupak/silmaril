//! Benchmarks for connection timeout detection (Quick Win #5)
//!
//! Measures the performance of timeout detection in the server loop.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BatchSize, Throughput};
use std::time::{Duration, Instant};

/// Simulates client state for benchmarking
struct BenchClientState {
    id: u64,
    last_message_at: Instant,
}

impl BenchClientState {
    fn new(id: u64) -> Self {
        Self {
            id,
            last_message_at: Instant::now(),
        }
    }

    fn with_age(id: u64, age: Duration) -> Self {
        Self {
            id,
            last_message_at: Instant::now() - age,
        }
    }
}

fn bench_timeout_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("timeout_detection");
    const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

    // Benchmark single client timeout check
    group.bench_function("check_single_client", |b| {
        b.iter(|| {
            let client = BenchClientState::new(1);
            let now = Instant::now();
            black_box(now.duration_since(client.last_message_at) > CLIENT_TIMEOUT)
        });
    });

    // Benchmark timeout check with aged client
    group.bench_function("check_timed_out_client", |b| {
        b.iter(|| {
            let client = BenchClientState::with_age(1, Duration::from_secs(35));
            let now = Instant::now();
            black_box(now.duration_since(client.last_message_at) > CLIENT_TIMEOUT)
        });
    });

    group.finish();
}

fn bench_timeout_detection_bulk(c: &mut Criterion) {
    let mut group = c.benchmark_group("timeout_detection_bulk");

    // Benchmark timeout detection across many clients
    for client_count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*client_count as u64));

        group.bench_with_input(
            format!("{}_clients_all_active", client_count),
            client_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        // Setup: Create all active clients
                        (0..count).map(|i| BenchClientState::new(i as u64)).collect::<Vec<_>>()
                    },
                    |clients| {
                        // Benchmark: Check all clients
                        let now = Instant::now();
                        let timed_out: Vec<_> = clients
                            .iter()
                            .filter(|client| {
                                now.duration_since(client.last_message_at) > Duration::from_secs(30)
                            })
                            .map(|c| c.id)
                            .collect();
                        black_box(timed_out)
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(
            format!("{}_clients_10pct_timed_out", client_count),
            client_count,
            |b, &count| {
                b.iter_batched(
                    || {
                        // Setup: 10% of clients are timed out
                        (0..count)
                            .map(|i| {
                                if i % 10 == 0 {
                                    BenchClientState::with_age(i as u64, Duration::from_secs(35))
                                } else {
                                    BenchClientState::new(i as u64)
                                }
                            })
                            .collect::<Vec<_>>()
                    },
                    |clients| {
                        let now = Instant::now();
                        let timed_out: Vec<_> = clients
                            .iter()
                            .filter(|client| {
                                now.duration_since(client.last_message_at) > Duration::from_secs(30)
                            })
                            .map(|c| c.id)
                            .collect();
                        black_box(timed_out)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_timeout_cleanup_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("timeout_cleanup");

    // Simulate the two-phase cleanup approach
    group.bench_function("two_phase_cleanup_100_clients", |b| {
        b.iter_batched(
            || {
                // Setup: 100 clients, 10% timed out
                (0..100)
                    .map(|i| {
                        if i % 10 == 0 {
                            BenchClientState::with_age(i as u64, Duration::from_secs(35))
                        } else {
                            BenchClientState::new(i as u64)
                        }
                    })
                    .collect::<Vec<_>>()
            },
            |mut clients| {
                let now = Instant::now();

                // Phase 1: Collect timed-out client IDs
                let timed_out: Vec<_> = clients
                    .iter()
                    .filter(|client| now.duration_since(client.last_message_at) > Duration::from_secs(30))
                    .map(|c| c.id)
                    .collect();

                // Phase 2: Remove timed-out clients
                clients.retain(|client| !timed_out.contains(&client.id));

                black_box((clients.len(), timed_out.len()))
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_timeout_detection,
    bench_timeout_detection_bulk,
    bench_timeout_cleanup_simulation
);
criterion_main!(benches);
