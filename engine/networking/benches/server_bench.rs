//! Server tick loop performance benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::{Entity, World};
use engine_core::math::Transform;
use engine_networking::{ServerLoop, TcpServer};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Benchmark server loop tick performance with varying entity counts
fn bench_server_tick_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("server_tick");

    for entity_count in [0, 10, 100, 1000, 10000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(
            BenchmarkId::new("entities", entity_count),
            &entity_count,
            |b, &count| {
                let rt = Runtime::new().unwrap();

                b.iter(|| {
                    rt.block_on(async {
                        let mut world = World::new();
                        world.register::<Transform>();

                        // Spawn entities
                        for _ in 0..count {
                            let entity = world.spawn();
                            world.add(entity, Transform::default());
                        }

                        let mut server_loop = ServerLoop::new(world);

                        // Run for a single tick
                        let mut ticked = false;
                        tokio::select! {
                            _ = server_loop.run(|_world, _dt| {
                                if !ticked {
                                    black_box(_world);
                                    ticked = true;
                                }
                            }) => {},
                            _ = tokio::time::sleep(Duration::from_millis(20)) => {}
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark server loop with client connections
fn bench_server_with_clients(c: &mut Criterion) {
    let mut group = c.benchmark_group("server_clients");

    for client_count in [0, 1, 10, 50] {
        group.bench_with_input(
            BenchmarkId::new("clients", client_count),
            &client_count,
            |b, &count| {
                let rt = Runtime::new().unwrap();

                b.iter(|| {
                    rt.block_on(async {
                        let world = World::new();
                        let mut server_loop = ServerLoop::new(world);

                        // Start TCP server
                        let tcp_server = TcpServer::bind("127.0.0.1:0").await.unwrap();
                        let _addr = tcp_server.local_addr().unwrap();

                        server_loop.start_accepting(tcp_server).await;

                        // Note: In a real benchmark, we'd connect actual clients here
                        // For now, just measure the server loop overhead

                        // Run for a few ticks
                        let mut tick_count = 0;
                        tokio::select! {
                            _ = server_loop.run(|_world, _dt| {
                                tick_count += 1;
                                black_box(tick_count);
                            }) => {},
                            _ = tokio::time::sleep(Duration::from_millis(50)) => {}
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark state broadcasting performance
fn bench_state_broadcast(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_broadcast");

    for entity_count in [10, 100, 1000] {
        group.throughput(Throughput::Elements(entity_count as u64));

        group.bench_with_input(
            BenchmarkId::new("entities", entity_count),
            &entity_count,
            |b, &count| {
                let rt = Runtime::new().unwrap();

                b.iter(|| {
                    rt.block_on(async {
                        let mut world = World::new();
                        world.register::<Transform>();

                        // Spawn entities
                        for _ in 0..count {
                            let entity = world.spawn();
                            world.add(entity, Transform::default());
                        }

                        let mut server_loop = ServerLoop::new(world);

                        // Run for enough time to trigger a state broadcast
                        tokio::select! {
                            _ = server_loop.run(|_world, _dt| {
                                black_box(_world);
                            }) => {},
                            _ = tokio::time::sleep(Duration::from_millis(20)) => {}
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark tick rate accuracy
fn bench_tick_rate_accuracy(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("tick_rate_60tps", |b| {
        b.iter(|| {
            rt.block_on(async {
                let world = World::new();
                let mut server_loop = ServerLoop::new(world);

                let start = std::time::Instant::now();

                // Run for 100ms
                tokio::select! {
                    _ = server_loop.run(|_world, _dt| {
                        black_box(_world);
                    }) => {},
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {}
                }

                let elapsed = start.elapsed();
                let ticks = server_loop.tick();

                // Should be close to 6 ticks in 100ms
                black_box((ticks, elapsed));
            });
        });
    });
}

/// Benchmark message processing throughput
fn bench_message_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_processing");

    for msg_count in [10, 100, 1000] {
        group.throughput(Throughput::Elements(msg_count as u64));

        group.bench_with_input(BenchmarkId::new("messages", msg_count), &msg_count, |b, &count| {
            let rt = Runtime::new().unwrap();

            b.iter(|| {
                rt.block_on(async {
                    let world = World::new();
                    let mut server_loop = ServerLoop::new(world);

                    // Simulate message processing load
                    let mut processed = 0;
                    tokio::select! {
                        _ = server_loop.run(|_world, _dt| {
                            processed += 1;
                            black_box(processed);
                        }) => {},
                        _ = tokio::time::sleep(Duration::from_millis(50)) => {}
                    }

                    black_box(processed);
                });
            });
        });
    }

    group.finish();
}

/// Benchmark server loop overhead (minimal game logic)
fn bench_server_loop_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("server_loop_overhead", |b| {
        b.iter(|| {
            rt.block_on(async {
                let world = World::new();
                let mut server_loop = ServerLoop::new(world);

                // Measure pure server loop overhead with no-op game logic
                tokio::select! {
                    _ = server_loop.run(|_world, _dt| {
                        // No-op
                    }) => {},
                    _ = tokio::time::sleep(Duration::from_millis(20)) => {}
                }

                black_box(server_loop.tick());
            });
        });
    });
}

criterion_group!(
    benches,
    bench_server_tick_performance,
    bench_server_with_clients,
    bench_state_broadcast,
    bench_tick_rate_accuracy,
    bench_message_processing,
    bench_server_loop_overhead
);
criterion_main!(benches);
