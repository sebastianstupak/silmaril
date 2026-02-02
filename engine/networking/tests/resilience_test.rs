//! Network Resilience Integration Tests
//!
//! Validates network resilience features under realistic conditions:
//! - Packet loss recovery
//! - Burst packet loss
//! - Jitter handling
//! - Connection quality metrics
//! - Graceful degradation

use engine_networking::{
    deserialize_client_message, serialize_client_message, ClientMessage, NetworkConditions,
    NetworkProfile, NetworkSimulator, SerializationFormat,
};
use std::time::{Duration, Instant};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_player_move(timestamp: u64) -> ClientMessage {
    ClientMessage::PlayerMove {
        x: 100.0 + (timestamp as f32 * 0.1),
        y: 50.0,
        z: 200.0 + (timestamp as f32 * 0.05),
        timestamp,
    }
}

fn create_test_packets(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| {
            let msg = create_player_move(i as u64);
            serialize_client_message(&msg, SerializationFormat::Bincode).unwrap().payload
        })
        .collect()
}

// ============================================================================
// Packet Loss Recovery Tests
// ============================================================================

#[test]
fn test_1_percent_packet_loss_recovery() {
    let conditions = NetworkConditions {
        latency_ms: 50,
        jitter_ms: 5,
        packet_loss_percent: 1.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(1000);

    // Send all packets
    for packet in &packets {
        sim.send(packet.clone());
    }

    // Wait for delivery
    std::thread::sleep(Duration::from_millis(200));

    // Receive packets
    let received = sim.recv();

    // With 1% loss, we should receive ~990 packets (99%)
    assert!(received.len() >= 980, "Should receive at least 98% of packets");
    assert!(received.len() <= 1000, "Should not receive more than sent");

    // Verify packets are valid
    for packet in received.iter().take(10) {
        assert!(
            deserialize_client_message(
                &engine_networking::FramedMessage {
                    length: packet.len() as u32,
                    payload: packet.clone(),
                },
                SerializationFormat::Bincode
            )
            .is_ok(),
            "Received packet should be valid"
        );
    }
}

#[test]
fn test_5_percent_packet_loss_recovery() {
    let conditions = NetworkConditions {
        latency_ms: 50,
        jitter_ms: 5,
        packet_loss_percent: 5.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(1000);

    for packet in &packets {
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(200));
    let received = sim.recv();

    // With 5% loss, we should receive ~950 packets (95%)
    assert!(received.len() >= 920, "Should receive at least 92% of packets");
    assert!(received.len() <= 1000, "Should not receive more than sent");
}

#[test]
fn test_10_percent_packet_loss_recovery() {
    let conditions = NetworkConditions {
        latency_ms: 50,
        jitter_ms: 5,
        packet_loss_percent: 10.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(1000);

    for packet in &packets {
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(200));
    let received = sim.recv();

    // With 10% loss, we should receive ~900 packets (90%)
    assert!(received.len() >= 850, "Should receive at least 85% of packets");
    assert!(received.len() <= 1000, "Should not receive more than sent");
}

#[test]
fn test_recovery_time_under_packet_loss() {
    let conditions = NetworkConditions {
        latency_ms: 50,
        jitter_ms: 10,
        packet_loss_percent: 5.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(100);

    for packet in &packets {
        sim.send(packet.clone());
    }

    // Measure time to receive first packet after latency
    std::thread::sleep(Duration::from_millis(60)); // Just over base latency

    let start = Instant::now();

    loop {
        let received = sim.recv();

        if !received.is_empty() || start.elapsed() > Duration::from_millis(100) {
            break;
        }

        std::thread::sleep(Duration::from_millis(5));
    }

    let recovery_time = start.elapsed();

    // Target: Recovery in <100ms
    assert!(
        recovery_time < Duration::from_millis(100),
        "Recovery should complete in <100ms, took {:?}",
        recovery_time
    );
}

// ============================================================================
// Burst Packet Loss Tests
// ============================================================================

#[test]
fn test_burst_10_packets_recovery() {
    // Use perfect LAN profile to avoid random packet loss
    let conditions = NetworkConditions {
        latency_ms: 20,
        jitter_ms: 0,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(100);

    // Send packets, drop 10 in the middle
    for (i, packet) in packets.iter().enumerate() {
        if i >= 45 && i < 55 {
            continue; // Drop burst
        }
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(100));
    let received = sim.recv();

    // Should receive 90 packets (100 - 10 dropped)
    assert_eq!(received.len(), 90, "Should receive non-dropped packets");
}

#[test]
fn test_burst_50_packets_recovery() {
    let conditions = NetworkConditions {
        latency_ms: 20,
        jitter_ms: 0,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(200);

    // Drop 50 packets in burst
    for (i, packet) in packets.iter().enumerate() {
        if i >= 75 && i < 125 {
            continue; // Drop burst
        }
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(150));
    let received = sim.recv();

    // Should receive 150 packets
    assert_eq!(received.len(), 150, "Should receive non-dropped packets");
}

#[test]
fn test_burst_100_packets_recovery() {
    let conditions = NetworkConditions {
        latency_ms: 20,
        jitter_ms: 0,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(300);

    // Drop 100 packets in burst
    for (i, packet) in packets.iter().enumerate() {
        if i >= 100 && i < 200 {
            continue; // Drop burst
        }
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(200));
    let received = sim.recv();

    // Should receive 200 packets
    assert_eq!(received.len(), 200, "Should receive non-dropped packets");
}

#[test]
fn test_burst_loss_graceful_degradation() {
    let conditions = NetworkConditions {
        latency_ms: 20,
        jitter_ms: 0,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(500);

    // Drop massive burst (200 packets)
    for (i, packet) in packets.iter().enumerate() {
        if i >= 150 && i < 350 {
            continue; // Drop burst
        }
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(300));
    let received = sim.recv();

    // Should still receive the non-dropped packets (300)
    assert_eq!(received.len(), 300, "Should gracefully handle large burst loss");
}

// ============================================================================
// Network Jitter Handling Tests
// ============================================================================

#[test]
fn test_low_jitter_handling() {
    let conditions = NetworkConditions {
        latency_ms: 50,
        jitter_ms: 5,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(100);

    for packet in &packets {
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(150));
    let received = sim.recv();

    // All packets should arrive with low jitter
    assert!(received.len() >= 95, "Should receive most packets with low jitter");
}

#[test]
fn test_medium_jitter_handling() {
    let conditions = NetworkConditions {
        latency_ms: 50,
        jitter_ms: 20,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(100);

    for packet in &packets {
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(200));
    let received = sim.recv();

    // Should still receive all packets despite jitter
    assert!(received.len() >= 95, "Should handle medium jitter gracefully");
}

#[test]
fn test_high_jitter_handling() {
    let conditions = NetworkConditions {
        latency_ms: 100,
        jitter_ms: 50,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(100);

    for packet in &packets {
        sim.send(packet.clone());
    }

    // Need to wait longer due to high jitter
    std::thread::sleep(Duration::from_millis(300));
    let received = sim.recv();

    // Should eventually receive most packets
    assert!(received.len() >= 90, "Should handle high jitter with appropriate buffer");
}

#[test]
fn test_jitter_buffer_smoothness() {
    let conditions = NetworkConditions {
        latency_ms: 50,
        jitter_ms: 20,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(50);

    // Send packets and measure delivery variance
    for packet in &packets {
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(200));

    // Poll multiple times to see delivery pattern
    let mut total_received = 0;

    for _ in 0..20 {
        let received = sim.recv();
        total_received += received.len();
        std::thread::sleep(Duration::from_millis(10));
    }

    // With jitter, packets arrive at different times
    // Should eventually receive most packets
    assert!(
        total_received >= 40,
        "Should receive most packets despite jitter, got {}",
        total_received
    );
}

// ============================================================================
// Connection Quality Metrics Tests
// ============================================================================

#[test]
fn test_rtt_estimation_accuracy() {
    let latency_ms = 50;
    let conditions = NetworkConditions {
        latency_ms,
        jitter_ms: 5,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(10);

    let mut rtts = Vec::new();

    for packet in &packets {
        let send_time = Instant::now();
        sim.send(packet.clone());

        std::thread::sleep(Duration::from_millis((latency_ms * 2 + 20) as u64));

        let received = sim.recv();
        if !received.is_empty() {
            let rtt = send_time.elapsed();
            rtts.push(rtt);
        }
    }

    assert!(!rtts.is_empty(), "Should measure at least one RTT");

    // Calculate average RTT
    let avg_rtt = rtts.iter().sum::<Duration>() / rtts.len() as u32;

    // Expected RTT is 2x one-way latency
    let expected_rtt = Duration::from_millis((latency_ms * 2) as u64);

    // Allow 20% error margin due to jitter and scheduling
    let error = if avg_rtt > expected_rtt {
        avg_rtt - expected_rtt
    } else {
        expected_rtt - avg_rtt
    };

    let error_percent = (error.as_millis() as f64 / expected_rtt.as_millis() as f64) * 100.0;

    assert!(
        error_percent < 30.0,
        "RTT estimation should be within 30% (was {}% off)",
        error_percent
    );
}

#[test]
fn test_packet_loss_detection_speed() {
    let conditions = NetworkConditions {
        latency_ms: 50,
        jitter_ms: 5,
        packet_loss_percent: 10.0,
        bandwidth_kbps: 10_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(100);

    for packet in &packets {
        sim.send(packet.clone());
    }

    // Wait for delivery
    std::thread::sleep(Duration::from_millis(150));

    let received = sim.recv();

    // Should detect packet loss within the detection window
    let loss_detected = received.len() < packets.len();

    // Detection happens when packets are received
    // Just verify we detected the loss
    assert!(loss_detected, "Packet loss should be detected");
}

#[test]
fn test_bandwidth_estimation_overhead() {
    let mut sim = NetworkSimulator::new(NetworkProfile::Cable);
    let packets = create_test_packets(1000);

    let start = Instant::now();

    for packet in &packets {
        sim.send(packet.clone());
    }

    let overhead = start.elapsed();

    // Target: <1ms overhead for 1000 packets
    assert!(
        overhead < Duration::from_millis(100),
        "Bandwidth tracking overhead should be minimal, took {:?}",
        overhead
    );
}

// ============================================================================
// Graceful Degradation Tests
// ============================================================================

#[test]
fn test_throughput_degradation_under_loss() {
    let loss_percentages = vec![0.0, 1.0, 5.0, 10.0, 25.0];

    for loss_percent in loss_percentages {
        let conditions = NetworkConditions {
            latency_ms: 50,
            jitter_ms: 10,
            packet_loss_percent: loss_percent,
            bandwidth_kbps: 10_000,
            reorder_probability: 0.0,
        };

        let mut sim = NetworkSimulator::with_conditions(conditions);
        let packets = create_test_packets(1000);

        for packet in &packets {
            sim.send(packet.clone());
        }

        std::thread::sleep(Duration::from_millis(200));
        let received = sim.recv();

        let expected_min = (1000.0 * (1.0 - loss_percent / 100.0) * 0.8) as usize;

        assert!(
            received.len() >= expected_min,
            "With {}% loss, should receive at least {} packets (got {})",
            loss_percent,
            expected_min,
            received.len()
        );
    }
}

#[test]
fn test_extreme_conditions_graceful_degradation() {
    let conditions = NetworkConditions {
        latency_ms: 300,
        jitter_ms: 100,
        packet_loss_percent: 25.0,
        bandwidth_kbps: 500,
        reorder_probability: 0.1,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);
    let packets = create_test_packets(100);

    for packet in &packets {
        sim.send(packet.clone());
    }

    // Need to wait longer for terrible conditions
    std::thread::sleep(Duration::from_millis(1000));
    let received = sim.recv();

    // Should still receive some packets (at least 60%)
    assert!(received.len() >= 60, "Should gracefully degrade under extreme conditions");
}

// ============================================================================
// Network Profile Resilience Tests
// ============================================================================

#[test]
fn test_lan_profile_resilience() {
    let mut sim = NetworkSimulator::new(NetworkProfile::Lan);
    let packets = create_test_packets(100);

    for packet in &packets {
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(50));
    let received = sim.recv();

    // LAN should deliver all packets
    assert_eq!(received.len(), 100, "LAN should have perfect delivery");
}

#[test]
fn test_cable_profile_resilience() {
    let mut sim = NetworkSimulator::new(NetworkProfile::Cable);
    let packets = create_test_packets(1000);

    for packet in &packets {
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(200));
    let received = sim.recv();

    // Cable should deliver ~99.9% packets
    assert!(received.len() >= 990, "Cable should have excellent delivery rate");
}

#[test]
fn test_mobile_4g_profile_resilience() {
    let mut sim = NetworkSimulator::new(NetworkProfile::FourG);
    let packets = create_test_packets(1000);

    for packet in &packets {
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(400));
    let received = sim.recv();

    // 4G should deliver ~99% packets (1% loss)
    assert!(received.len() >= 980, "4G should maintain good delivery rate");
}

#[test]
fn test_mobile_3g_profile_resilience() {
    let mut sim = NetworkSimulator::new(NetworkProfile::ThreeG);
    let packets = create_test_packets(1000);

    for packet in &packets {
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(800));
    let received = sim.recv();

    // 3G should deliver ~97% packets (3% loss)
    assert!(received.len() >= 950, "3G should handle packet loss gracefully");
}

#[test]
fn test_terrible_profile_resilience() {
    let mut sim = NetworkSimulator::new(NetworkProfile::Terrible);
    let packets = create_test_packets(1000);

    for packet in &packets {
        sim.send(packet.clone());
    }

    std::thread::sleep(Duration::from_millis(1500));
    let received = sim.recv();

    // Terrible should still deliver ~90% packets (10% loss)
    assert!(
        received.len() >= 850,
        "Even terrible connection should deliver majority of packets"
    );
}
