//! Benchmarks for dev mode state handoff (serialize/restore) and ReloadClient TCP RTT.
//!
//! These measure the overhead of ECS world serialization during `silm dev`
//! code-change restarts. Targets:
//!   - StateHandoff::save   < 100ms for 1K entities, < 1s for 10K
//!   - StateHandoff::restore < 100ms for 1K entities, < 1s for 10K
//!   - ReloadClient RTT     < 5ms per message on loopback

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_core::ecs::World;
use engine_dev_tools_hot_reload::handoff::StateHandoff;
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

fn make_world(entity_count: usize) -> World {
    let mut world = World::new();
    for _ in 0..entity_count {
        world.spawn();
    }
    world
}

// ── StateHandoff::save ───────────────────────────────────────────────────────

fn bench_state_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("dev_state_save");

    for &size in &[100_usize, 1_000, 10_000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            let world = make_world(n);
            let dir = TempDir::new().unwrap();
            let handoff = StateHandoff::new(dir.path());

            b.iter(|| {
                handoff.save(black_box(&world)).unwrap();
            });
        });
    }

    group.finish();
}

// ── StateHandoff::restore ────────────────────────────────────────────────────

fn bench_state_restore(c: &mut Criterion) {
    let mut group = c.benchmark_group("dev_state_restore");

    for &size in &[100_usize, 1_000, 10_000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            let world = make_world(n);
            let dir = TempDir::new().unwrap();
            let handoff = StateHandoff::new(dir.path());
            // Pre-seed a valid state file so the restore always finds data.
            handoff.save(&world).unwrap();

            b.iter(|| {
                // Re-save before each restore so the file is always present
                // (restore consumes/deletes the file on success).
                handoff.save(&world).unwrap();
                let mut restored = World::new();
                handoff.restore(black_box(&mut restored)).unwrap();
            });
        });
    }

    group.finish();
}

// ── StateHandoff round-trip (save + restore) ─────────────────────────────────

fn bench_state_round_trip(c: &mut Criterion) {
    let mut group = c.benchmark_group("dev_state_round_trip");

    for &size in &[100_usize, 1_000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            let world = make_world(n);
            let dir = TempDir::new().unwrap();
            let handoff = StateHandoff::new(dir.path());

            b.iter(|| {
                handoff.save(black_box(&world)).unwrap();
                let mut restored = World::new();
                handoff.restore(black_box(&mut restored)).unwrap();
            });
        });
    }

    group.finish();
}

// ── ReloadClient TCP RTT ─────────────────────────────────────────────────────
//
// We spin up a minimal echo server on a loopback port and measure the RTT
// for a newline-delimited JSON message (the same format used by ReloadClient).
// This isolates TCP loopback latency from actual world serialization time.
//
// Note: criterion's `to_async` requires an optional feature flag that this
// workspace does not enable. We use `Runtime::block_on` inside the iter closure
// instead, which is equivalent for latency benchmarks.

fn bench_reload_client_rtt(c: &mut Criterion) {
    use engine_dev_tools_hot_reload::messages::ReloadMessage;
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{TcpListener, TcpStream};

    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Bind a loopback echo server that acks every message.
    let port: u16 = runtime.block_on(async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else { break };
                tokio::spawn(async move {
                    let (reader, mut writer) = stream.into_split();
                    let mut lines = BufReader::new(reader).lines();
                    while let Ok(Some(_line)) = lines.next_line().await {
                        // Ack every message regardless of type.
                        let ack = serde_json::to_string(&ReloadMessage::Ack).unwrap() + "\n";
                        let _ = writer.write_all(ack.as_bytes()).await;
                    }
                });
            }
        });

        addr.port()
    });

    let mut group = c.benchmark_group("reload_client_rtt");
    group.measurement_time(Duration::from_secs(5));

    // Benchmark: connect → send SerializeState → wait for Ack → disconnect.
    group.bench_function("serialize_state_ack", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let mut stream =
                    TcpStream::connect(("127.0.0.1", black_box(port))).await.unwrap();
                let msg =
                    serde_json::to_string(&ReloadMessage::SerializeState).unwrap() + "\n";
                stream.write_all(msg.as_bytes()).await.unwrap();

                let mut reader = BufReader::new(&mut stream);
                let mut response = String::new();
                reader.read_line(&mut response).await.unwrap();
            });
        });
    });

    // Benchmark: send a ReloadAsset message and wait for the server ack.
    group.bench_function("reload_asset_ack", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let mut stream =
                    TcpStream::connect(("127.0.0.1", black_box(port))).await.unwrap();
                let msg = serde_json::to_string(&ReloadMessage::ReloadAsset {
                    path: "assets/player.png".into(),
                })
                .unwrap()
                    + "\n";
                stream.write_all(msg.as_bytes()).await.unwrap();

                // Wait for server ack so we get a full RTT measurement.
                let mut reader = BufReader::new(&mut stream);
                let mut response = String::new();
                reader.read_line(&mut response).await.unwrap();
            });
        });
    });

    group.finish();
}

// ── criterion entry points ────────────────────────────────────────────────────

criterion_group!(
    dev_benches,
    bench_state_save,
    bench_state_restore,
    bench_state_round_trip,
    bench_reload_client_rtt
);
criterion_main!(dev_benches);
