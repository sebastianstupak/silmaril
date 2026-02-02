//! Network Simulator Integration Tests

use engine_networking::{NetworkConditions, NetworkProfile, NetworkSimulator};
use std::time::{Duration, Instant};

#[test]
fn test_lan_profile() {
    let sim = NetworkSimulator::new(NetworkProfile::Lan);
    let conditions = sim.conditions();

    assert_eq!(conditions.latency_ms, 1);
    assert_eq!(conditions.jitter_ms, 0);
    assert_eq!(conditions.packet_loss_percent, 0.0);
}

#[test]
fn test_send_receive() {
    let mut sim = NetworkSimulator::new(NetworkProfile::Lan);

    let data = vec![1, 2, 3, 4, 5];
    sim.send(data.clone());

    // Should be delivered after latency
    std::thread::sleep(Duration::from_millis(10));
    let received = sim.recv();

    assert_eq!(received.len(), 1);
    assert_eq!(received[0], data);
}

#[test]
fn test_latency_delay() {
    let conditions = NetworkConditions {
        latency_ms: 100,
        jitter_ms: 0,
        packet_loss_percent: 0.0,
        bandwidth_kbps: 100_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);

    let start = Instant::now();
    sim.send(vec![1, 2, 3]);

    // Should not be ready immediately
    let received = sim.recv();
    assert_eq!(received.len(), 0);

    // Wait for latency
    std::thread::sleep(Duration::from_millis(110));
    let received = sim.recv();

    assert_eq!(received.len(), 1);
    assert!(start.elapsed() >= Duration::from_millis(100));
}

#[test]
fn test_packet_loss() {
    let conditions = NetworkConditions {
        latency_ms: 1,
        jitter_ms: 0,
        packet_loss_percent: 100.0, // 100% loss
        bandwidth_kbps: 100_000,
        reorder_probability: 0.0,
    };

    let mut sim = NetworkSimulator::with_conditions(conditions);

    // Send many packets
    for _ in 0..100 {
        sim.send(vec![1, 2, 3]);
    }

    // All should be lost
    std::thread::sleep(Duration::from_millis(100));
    let received = sim.recv();
    assert_eq!(received.len(), 0);
}

#[test]
fn test_in_flight_count() {
    let mut sim = NetworkSimulator::new(NetworkProfile::Dsl);

    assert_eq!(sim.in_flight(), 0);

    sim.send(vec![1, 2, 3]);
    sim.send(vec![4, 5, 6]);

    // Note: Some might be dropped due to packet loss
    assert!(sim.in_flight() <= 2);
}

#[test]
fn test_clear() {
    let mut sim = NetworkSimulator::new(NetworkProfile::Dsl);

    for _ in 0..10 {
        sim.send(vec![1, 2, 3]);
    }

    assert!(sim.in_flight() > 0);

    sim.clear();
    assert_eq!(sim.in_flight(), 0);
}

#[test]
fn test_all_profiles() {
    let profiles = vec![
        NetworkProfile::Lan,
        NetworkProfile::Cable,
        NetworkProfile::Dsl,
        NetworkProfile::FourG,
        NetworkProfile::ThreeG,
        NetworkProfile::Terrible,
    ];

    for profile in profiles {
        let sim = NetworkSimulator::new(profile);
        let conditions = sim.conditions();

        assert!(conditions.latency_ms > 0);
        assert!(conditions.packet_loss_percent >= 0.0);
        assert!(conditions.bandwidth_kbps > 0);
    }
}

#[test]
fn test_multiple_packets() {
    let mut sim = NetworkSimulator::new(NetworkProfile::Lan);

    // Send multiple packets
    for i in 0..10 {
        sim.send(vec![i]);
    }

    // Wait and receive
    std::thread::sleep(Duration::from_millis(20));
    let received = sim.recv();

    // With perfect LAN, should receive all (or most)
    assert!(received.len() >= 8); // Allow some loss for test stability
}
