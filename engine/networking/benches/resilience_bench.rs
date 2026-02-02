//! Network Resilience Benchmarks
//!
//! Comprehensive benchmarks for network resilience features:
//! - Packet loss recovery
//! - Burst packet loss handling
//! - Network jitter tolerance
//! - Connection quality metrics
//!
//! These benchmarks validate the engine's ability to maintain gameplay
//! quality under adverse network conditions.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use engine_networking::{
    serialize_client_message, ClientMessage, NetworkConditions, NetworkProfile, NetworkSimulator,
    SerializationFormat,
};
use std::time::{Duration, Instant};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test packet stream
fn create_test_packets(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| {
            let msg = ClientMessage::PlayerMove {
                x: i as f32,
                y: 0.0,
                z: i as f32 * 0.5,
                timestamp: i as u64,
            };
            serialize_client_message(&msg, SerializationFormat::Bincode).unwrap().payload
        })
        .collect()
}

/// Measure recovery time after packet loss
fn measure_recovery_time(
    packet_loss_percent: f32,
    total_packets: usize,
) -> (Duration, usize, usize) {
    let conditions = NetworkConditions {
        latency_ms: 50,
        jitter_ms: 10,
        packet_loss_percent,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(total_packets);

    // Send all packets
    for packet in &packets {
        sim.send(packet.clone());
    }

    // Wait for latency + some recovery time
    std::thread::sleep(Duration::from_millis(200));

    // Start recovery measurement
    let recovery_start = Instant::now();
    let mut received = Vec::new();

    // Poll for packets until stable (no new packets for 50ms)
    let mut last_receive_time = Instant::now();
    let stability_threshold = Duration::from_millis(50);

    loop {
        let new_packets = sim.recv();
        if !new_packets.is_empty() {
            received.extend(new_packets);
            last_receive_time = Instant::now();
        }

        if last_receive_time.elapsed() >= stability_threshold {
            break; // Stable - no new packets
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    let recovery_time = recovery_start.elapsed();
    let sent_count = packets.len();
    let received_count = received.len();

    (recovery_time, sent_count, received_count)
}

/// Measure burst loss recovery
fn measure_burst_loss_recovery(
    burst_size: usize,
    total_packets: usize,
) -> (Duration, usize, usize) {
    let conditions = NetworkConditions {
        latency_ms: 50,
        jitter_ms: 10,
        packet_loss_percent: 0.0, // We'll manually inject burst loss
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(total_packets);

    // Send packets, dropping a burst in the middle
    let burst_start = total_packets / 2;
    let burst_end = burst_start + burst_size;

    for (i, packet) in packets.iter().enumerate() {
        if i >= burst_start && i < burst_end {
            // Drop this packet (simulate burst loss)
            continue;
        }
        sim.send(packet.clone());
    }

    // Wait for latency
    std::thread::sleep(Duration::from_millis(200));

    // Measure recovery
    let recovery_start = Instant::now();
    let mut received = Vec::new();
    let mut last_receive_time = Instant::now();
    let stability_threshold = Duration::from_millis(50);

    loop {
        let new_packets = sim.recv();
        if !new_packets.is_empty() {
            received.extend(new_packets);
            last_receive_time = Instant::now();
        }

        if last_receive_time.elapsed() >= stability_threshold {
            break;
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    let recovery_time = recovery_start.elapsed();
    let expected_count = packets.len() - burst_size;
    let received_count = received.len();

    (recovery_time, expected_count, received_count)
}

/// Measure jitter buffer performance
fn measure_jitter_handling(jitter_ms: u32, packet_count: usize) -> (f64, f64) {
    let conditions = NetworkConditions {
        latency_ms: 50,
        jitter_ms,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(packet_count);

    // Send all packets
    let send_times: Vec<Instant> = packets
        .iter()
        .map(|packet| {
            let send_time = Instant::now();
            sim.send(packet.clone());
            send_time
        })
        .collect();

    // Wait for delivery
    std::thread::sleep(Duration::from_millis(300));

    // Receive and measure latency variance
    let mut latencies = Vec::new();
    let mut receive_idx = 0;

    loop {
        let received = sim.recv();
        if received.is_empty() {
            break;
        }

        for _ in received {
            if receive_idx < send_times.len() {
                let latency = send_times[receive_idx].elapsed();
                latencies.push(latency.as_micros() as f64);
                receive_idx += 1;
            }
        }

        if sim.in_flight() == 0 {
            break;
        }
    }

    // Calculate mean and variance
    if latencies.is_empty() {
        return (0.0, 0.0);
    }

    let mean = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let variance = latencies
        .iter()
        .map(|l| {
            let diff = l - mean;
            diff * diff
        })
        .sum::<f64>()
        / latencies.len() as f64;

    (mean, variance.sqrt()) // Return mean and std dev
}

/// Measure RTT estimation accuracy
fn measure_rtt_estimation(actual_latency_ms: u32) -> (f64, f64) {
    let conditions = NetworkConditions {
        latency_ms: actual_latency_ms,
        jitter_ms: actual_latency_ms / 10, // 10% jitter
        packet_loss_percent: 0.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(100);

    // Measure actual RTT
    let mut rtts = Vec::new();

    for packet in &packets {
        let send_time = Instant::now();
        sim.send(packet.clone());

        // Wait for delivery
        std::thread::sleep(Duration::from_millis((actual_latency_ms * 2 + 50) as u64));

        let received = sim.recv();
        if !received.is_empty() {
            let rtt = send_time.elapsed().as_micros() as f64;
            rtts.push(rtt);
        }
    }

    if rtts.is_empty() {
        return (0.0, 0.0);
    }

    let mean_rtt = rtts.iter().sum::<f64>() / rtts.len() as f64;
    let actual_rtt = (actual_latency_ms * 2) as f64 * 1000.0; // Convert to microseconds
    let error_percent = ((mean_rtt - actual_rtt).abs() / actual_rtt) * 100.0;

    (mean_rtt, error_percent)
}

// ============================================================================
// Packet Loss Recovery Benchmarks
// ============================================================================

fn bench_packet_loss_recovery(c: &mut Criterion) {
    let mut group = c.benchmark_group("packet_loss_recovery");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    // Target: 1% loss recovers in <50ms
    group.bench_function("1_percent_loss", |b| {
        b.iter(|| {
            let (recovery_time, sent, received) = measure_recovery_time(1.0, 100);
            black_box((recovery_time, sent, received))
        });
    });

    // Target: 5% loss recovers in <100ms
    group.bench_function("5_percent_loss", |b| {
        b.iter(|| {
            let (recovery_time, sent, received) = measure_recovery_time(5.0, 100);
            black_box((recovery_time, sent, received))
        });
    });

    // Target: 10% loss recovers in <200ms
    group.bench_function("10_percent_loss", |b| {
        b.iter(|| {
            let (recovery_time, sent, received) = measure_recovery_time(10.0, 100);
            black_box((recovery_time, sent, received))
        });
    });

    // Extreme: 25% loss (graceful degradation)
    group.bench_function("25_percent_loss", |b| {
        b.iter(|| {
            let (recovery_time, sent, received) = measure_recovery_time(25.0, 100);
            black_box((recovery_time, sent, received))
        });
    });

    group.finish();
}

// ============================================================================
// Burst Packet Loss Benchmarks
// ============================================================================

fn bench_burst_packet_loss(c: &mut Criterion) {
    let mut group = c.benchmark_group("burst_packet_loss");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    // Small burst: 10 packets
    group.bench_function("burst_10_packets", |b| {
        b.iter(|| {
            let (recovery_time, expected, received) = measure_burst_loss_recovery(10, 200);
            black_box((recovery_time, expected, received))
        });
    });

    // Medium burst: 50 packets
    group.bench_function("burst_50_packets", |b| {
        b.iter(|| {
            let (recovery_time, expected, received) = measure_burst_loss_recovery(50, 200);
            black_box((recovery_time, expected, received))
        });
    });

    // Large burst: 100 packets
    group.bench_function("burst_100_packets", |b| {
        b.iter(|| {
            let (recovery_time, expected, received) = measure_burst_loss_recovery(100, 300);
            black_box((recovery_time, expected, received))
        });
    });

    // Extreme burst: 200 packets
    group.bench_function("burst_200_packets", |b| {
        b.iter(|| {
            let (recovery_time, expected, received) = measure_burst_loss_recovery(200, 500);
            black_box((recovery_time, expected, received))
        });
    });

    group.finish();
}

// ============================================================================
// Network Jitter Handling Benchmarks
// ============================================================================

fn bench_jitter_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("jitter_handling");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    // Low jitter: 5ms
    group.bench_function("jitter_5ms", |b| {
        b.iter(|| {
            let (mean, std_dev) = measure_jitter_handling(5, 50);
            black_box((mean, std_dev))
        });
    });

    // Medium jitter: 20ms
    group.bench_function("jitter_20ms", |b| {
        b.iter(|| {
            let (mean, std_dev) = measure_jitter_handling(20, 50);
            black_box((mean, std_dev))
        });
    });

    // High jitter: 50ms
    group.bench_function("jitter_50ms", |b| {
        b.iter(|| {
            let (mean, std_dev) = measure_jitter_handling(50, 50);
            black_box((mean, std_dev))
        });
    });

    // Extreme jitter: 100ms
    group.bench_function("jitter_100ms", |b| {
        b.iter(|| {
            let (mean, std_dev) = measure_jitter_handling(100, 50);
            black_box((mean, std_dev))
        });
    });

    group.finish();
}

// ============================================================================
// Connection Quality Metrics Benchmarks
// ============================================================================

fn bench_connection_quality_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_quality_metrics");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    // RTT estimation accuracy - Low latency
    group.bench_function("rtt_estimation_50ms", |b| {
        b.iter(|| {
            let (rtt, error) = measure_rtt_estimation(50);
            black_box((rtt, error))
        });
    });

    // RTT estimation accuracy - Medium latency
    group.bench_function("rtt_estimation_100ms", |b| {
        b.iter(|| {
            let (rtt, error) = measure_rtt_estimation(100);
            black_box((rtt, error))
        });
    });

    // RTT estimation accuracy - High latency
    group.bench_function("rtt_estimation_200ms", |b| {
        b.iter(|| {
            let (rtt, error) = measure_rtt_estimation(200);
            black_box((rtt, error))
        });
    });

    // Packet loss detection speed
    group.bench_function("packet_loss_detection", |b| {
        b.iter(|| {
            let conditions = NetworkConditions {
                latency_ms: 50,
                jitter_ms: 10,
                packet_loss_percent: 10.0,
                bandwidth_kbps: 10_000,
                reorder_probability: 0.0,
            };

            let mut sim = NetworkSimulator::with_conditions(conditions);
            let packets = create_test_packets(100);

            let start = Instant::now();
            let mut sent_count = 0;

            for packet in &packets {
                sim.send(packet.clone());
                sent_count += 1;
            }

            // Wait for delivery
            std::thread::sleep(Duration::from_millis(200));

            let received = sim.recv();
            let detection_time = start.elapsed();
            let loss_detected = sent_count > received.len();

            black_box((detection_time, loss_detected, sent_count, received.len()))
        });
    });

    // Bandwidth estimation overhead
    group.bench_function("bandwidth_estimation_overhead", |b| {
        b.iter(|| {
            let mut sim = NetworkSimulator::new(NetworkProfile::Cable);
            let packets = create_test_packets(1000);

            let start = Instant::now();

            for packet in &packets {
                sim.send(packet.clone());
            }

            let overhead = start.elapsed();
            black_box(overhead)
        });
    });

    group.finish();
}

// ============================================================================
// Graceful Degradation Benchmarks
// ============================================================================

fn bench_graceful_degradation(c: &mut Criterion) {
    let mut group = c.benchmark_group("graceful_degradation");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    // Test throughput degradation under packet loss
    for loss_percent in [0.0, 1.0, 5.0, 10.0, 25.0, 50.0] {
        group.bench_with_input(
            BenchmarkId::new("throughput_degradation", loss_percent as u32),
            &loss_percent,
            |b, &loss| {
                b.iter(|| {
                    let conditions = NetworkConditions {
                        latency_ms: 50,
                        jitter_ms: 10,
                        packet_loss_percent: loss,
                        bandwidth_kbps: 10_000,
                        reorder_probability: 0.0,
                    };

                    let mut sim = NetworkSimulator::with_conditions(conditions);
                    let packets = create_test_packets(100);

                    let start = Instant::now();

                    for packet in &packets {
                        sim.send(packet.clone());
                    }

                    std::thread::sleep(Duration::from_millis(200));
                    let received = sim.recv();

                    let throughput = received.len() as f64 / start.elapsed().as_secs_f64();
                    black_box(throughput)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Network Profile Resilience Tests
// ============================================================================

fn bench_network_profile_resilience(c: &mut Criterion) {
    let mut group = c.benchmark_group("network_profile_resilience");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    let profiles = vec![
        ("lan", NetworkProfile::Lan),
        ("cable", NetworkProfile::Cable),
        ("dsl", NetworkProfile::Dsl),
        ("4g", NetworkProfile::FourG),
        ("3g", NetworkProfile::ThreeG),
        ("terrible", NetworkProfile::Terrible),
    ];

    for (name, profile) in profiles {
        group.bench_with_input(
            BenchmarkId::new("profile_resilience", name),
            &profile,
            |b, &profile| {
                b.iter(|| {
                    let mut sim = NetworkSimulator::new(profile);
                    let packets = create_test_packets(100);

                    let start = Instant::now();

                    for packet in &packets {
                        sim.send(packet.clone());
                    }

                    // Wait appropriate time for profile
                    let wait_time = match profile {
                        NetworkProfile::Lan => 50,
                        NetworkProfile::Cable => 100,
                        NetworkProfile::Dsl => 200,
                        NetworkProfile::FourG => 300,
                        NetworkProfile::ThreeG => 500,
                        NetworkProfile::Terrible => 1000,
                        _ => 200,
                    };

                    std::thread::sleep(Duration::from_millis(wait_time));
                    let received = sim.recv();

                    let delivery_rate = received.len() as f64 / packets.len() as f64;
                    let latency = start.elapsed();

                    black_box((delivery_rate, latency))
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Recovery Latency Distribution Benchmarks
// ============================================================================

fn bench_recovery_latency_distribution(c: &mut Criterion) {
    let mut group = c.benchmark_group("recovery_latency_distribution");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    // Measure p50, p95, p99 recovery times
    group.bench_function("recovery_percentiles", |b| {
        b.iter(|| {
            let mut recovery_times = Vec::new();

            // Collect 100 samples
            for _ in 0..100 {
                let (recovery_time, _, _) = measure_recovery_time(5.0, 50);
                recovery_times.push(recovery_time.as_micros());
            }

            recovery_times.sort_unstable();

            let p50 = recovery_times[50];
            let p95 = recovery_times[95];
            let p99 = recovery_times[99];

            black_box((p50, p95, p99))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_packet_loss_recovery,
    bench_burst_packet_loss,
    bench_jitter_handling,
    bench_connection_quality_metrics,
    bench_graceful_degradation,
    bench_network_profile_resilience,
    bench_recovery_latency_distribution,
);

criterion_main!(benches);
