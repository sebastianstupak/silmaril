//! Network Simulator
//!
//! Simulates realistic network conditions for testing and benchmarking:
//! - Latency (RTT)
//! - Packet loss
//! - Jitter (latency variance)
//! - Bandwidth throttling
//! - Packet reordering
//!
//! Supports multiple network profiles (LAN, Cable, DSL, 4G, 3G, etc.)

use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::time::{Duration, Instant};

/// Network simulation profile
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkProfile {
    /// Perfect LAN: 1ms latency, 0% loss
    Lan,
    /// Good cable: 20ms latency, 0.1% loss
    Cable,
    /// Average DSL: 50ms latency, 0.5% loss, 5ms jitter
    Dsl,
    /// 4G mobile: 80ms latency, 1% loss, 20ms jitter
    FourG,
    /// 3G mobile: 150ms latency, 3% loss, 50ms jitter
    ThreeG,
    /// Terrible connection: 300ms latency, 10% loss, 100ms jitter
    Terrible,
    /// Custom profile
    Custom(NetworkConditions),
}

impl NetworkProfile {
    /// Get network conditions for this profile
    pub fn conditions(&self) -> NetworkConditions {
        match self {
            Self::Lan => NetworkConditions {
                latency_ms: 1,
                jitter_ms: 0,
                packet_loss_percent: 0.0,
                bandwidth_kbps: 100_000, // 100 Mbps
                reorder_probability: 0.0,
            },
            Self::Cable => NetworkConditions {
                latency_ms: 20,
                jitter_ms: 2,
                packet_loss_percent: 0.1,
                bandwidth_kbps: 10_000, // 10 Mbps
                reorder_probability: 0.001,
            },
            Self::Dsl => NetworkConditions {
                latency_ms: 50,
                jitter_ms: 5,
                packet_loss_percent: 0.5,
                bandwidth_kbps: 2_000, // 2 Mbps
                reorder_probability: 0.01,
            },
            Self::FourG => NetworkConditions {
                latency_ms: 80,
                jitter_ms: 20,
                packet_loss_percent: 1.0,
                bandwidth_kbps: 5_000, // 5 Mbps
                reorder_probability: 0.02,
            },
            Self::ThreeG => NetworkConditions {
                latency_ms: 150,
                jitter_ms: 50,
                packet_loss_percent: 3.0,
                bandwidth_kbps: 1_000, // 1 Mbps
                reorder_probability: 0.05,
            },
            Self::Terrible => NetworkConditions {
                latency_ms: 300,
                jitter_ms: 100,
                packet_loss_percent: 10.0,
                bandwidth_kbps: 500, // 500 Kbps
                reorder_probability: 0.1,
            },
            Self::Custom(conditions) => *conditions,
        }
    }
}

/// Network conditions for simulation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetworkConditions {
    /// Base latency in milliseconds (one-way)
    pub latency_ms: u32,
    /// Latency variance (jitter) in milliseconds
    pub jitter_ms: u32,
    /// Packet loss probability (0-100%)
    pub packet_loss_percent: f32,
    /// Bandwidth limit in kilobits per second
    pub bandwidth_kbps: u32,
    /// Probability of packet reordering (0-1)
    pub reorder_probability: f32,
}

/// Queued packet with delivery time
#[derive(Debug)]
struct QueuedPacket {
    data: Vec<u8>,
    deliver_at: Instant,
}

impl PartialEq for QueuedPacket {
    fn eq(&self, other: &Self) -> bool {
        self.deliver_at == other.deliver_at
    }
}

impl Eq for QueuedPacket {}

impl PartialOrd for QueuedPacket {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedPacket {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (earliest delivery first)
        other.deliver_at.cmp(&self.deliver_at)
    }
}

/// Network simulator
///
/// Simulates realistic network conditions by delaying packets,
/// dropping them probabilistically, and throttling bandwidth.
pub struct NetworkSimulator {
    conditions: NetworkConditions,
    queue: BinaryHeap<QueuedPacket>,
    bandwidth_tracker: BandwidthTracker,
    rng_seed: u64,
}

impl NetworkSimulator {
    /// Create a new network simulator with the given profile
    pub fn new(profile: NetworkProfile) -> Self {
        Self::with_seed(profile, 0)
    }

    /// Create a new network simulator with a specific random seed
    pub fn with_seed(profile: NetworkProfile, seed: u64) -> Self {
        Self {
            conditions: profile.conditions(),
            queue: BinaryHeap::new(),
            bandwidth_tracker: BandwidthTracker::new(),
            rng_seed: seed,
        }
    }

    /// Create a simulator with custom conditions
    pub fn with_conditions(conditions: NetworkConditions) -> Self {
        Self {
            conditions,
            queue: BinaryHeap::new(),
            bandwidth_tracker: BandwidthTracker::new(),
            rng_seed: 0,
        }
    }

    /// Send a packet through the simulator
    ///
    /// The packet may be:
    /// - Dropped (packet loss)
    /// - Delayed (latency + jitter)
    /// - Reordered (delivered out of sequence)
    /// - Throttled (bandwidth limiting)
    pub fn send(&mut self, data: Vec<u8>) {
        let now = Instant::now();

        // Check packet loss
        if self.should_drop_packet() {
            return; // Packet lost
        }

        // Calculate delivery time (latency + jitter)
        let latency = self.calculate_latency();

        // Apply bandwidth throttling
        let bandwidth_delay = self.bandwidth_tracker.calculate_delay(data.len(), &self.conditions);

        let total_delay = latency + bandwidth_delay;
        let deliver_at = now + total_delay;

        // Check for reordering
        let deliver_at = if self.should_reorder() {
            // Add extra delay to reorder this packet
            deliver_at + Duration::from_millis(10)
        } else {
            deliver_at
        };

        self.queue.push(QueuedPacket { data, deliver_at });
    }

    /// Receive packets that are ready for delivery
    ///
    /// Returns all packets whose delivery time has passed.
    pub fn recv(&mut self) -> Vec<Vec<u8>> {
        let now = Instant::now();
        let mut packets = Vec::new();

        while let Some(packet) = self.queue.peek() {
            if packet.deliver_at <= now {
                packets.push(self.queue.pop().unwrap().data);
            } else {
                break;
            }
        }

        packets
    }

    /// Get the number of packets currently in flight
    pub fn in_flight(&self) -> usize {
        self.queue.len()
    }

    /// Get current network conditions
    pub fn conditions(&self) -> &NetworkConditions {
        &self.conditions
    }

    /// Update network conditions (for dynamic testing)
    pub fn set_conditions(&mut self, conditions: NetworkConditions) {
        self.conditions = conditions;
    }

    /// Clear all queued packets
    pub fn clear(&mut self) {
        self.queue.clear();
        self.bandwidth_tracker.reset();
    }

    // Internal helpers

    fn should_drop_packet(&mut self) -> bool {
        let threshold = (self.conditions.packet_loss_percent / 100.0 * u32::MAX as f32) as u32;
        self.next_random() < threshold
    }

    fn calculate_latency(&mut self) -> Duration {
        let base_latency = self.conditions.latency_ms;
        let jitter = if self.conditions.jitter_ms > 0 {
            (self.next_random() % self.conditions.jitter_ms) as i32
                - (self.conditions.jitter_ms as i32 / 2)
        } else {
            0
        };

        let total_latency = (base_latency as i32 + jitter).max(0) as u64;
        Duration::from_millis(total_latency)
    }

    fn should_reorder(&mut self) -> bool {
        let threshold = (self.conditions.reorder_probability * u32::MAX as f32) as u32;
        self.next_random() < threshold
    }

    // Simple LCG random number generator (fast, deterministic)
    fn next_random(&mut self) -> u32 {
        self.rng_seed = self.rng_seed.wrapping_mul(1103515245).wrapping_add(12345);
        (self.rng_seed >> 16) as u32
    }
}

/// Bandwidth tracker for throttling
struct BandwidthTracker {
    bytes_sent: VecDeque<(Instant, usize)>,
    window_duration: Duration,
}

impl BandwidthTracker {
    fn new() -> Self {
        Self { bytes_sent: VecDeque::new(), window_duration: Duration::from_secs(1) }
    }

    fn calculate_delay(&mut self, packet_size: usize, conditions: &NetworkConditions) -> Duration {
        let now = Instant::now();

        // Remove old entries outside the window
        while let Some((timestamp, _)) = self.bytes_sent.front() {
            if now.duration_since(*timestamp) > self.window_duration {
                self.bytes_sent.pop_front();
            } else {
                break;
            }
        }

        // Calculate current bandwidth usage
        let bytes_in_window: usize = self.bytes_sent.iter().map(|(_, size)| size).sum();
        let bytes_per_second = bytes_in_window;

        // Convert bandwidth limit to bytes/sec
        let bandwidth_limit_bytes = (conditions.bandwidth_kbps * 1024) / 8;

        // Track this packet
        self.bytes_sent.push_back((now, packet_size));

        // Calculate delay needed to throttle
        if bytes_per_second > bandwidth_limit_bytes as usize {
            let excess_bytes = bytes_per_second - bandwidth_limit_bytes as usize;
            let delay_ms = (excess_bytes as f64 / bandwidth_limit_bytes as f64 * 1000.0) as u64;
            Duration::from_millis(delay_ms)
        } else {
            Duration::ZERO
        }
    }

    fn reset(&mut self) {
        self.bytes_sent.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_lan() {
        let mut sim = NetworkSimulator::new(NetworkProfile::Lan);

        let data = vec![1, 2, 3, 4, 5];
        sim.send(data.clone());

        // Should be delivered almost immediately
        std::thread::sleep(Duration::from_millis(10));
        let received = sim.recv();

        assert_eq!(received.len(), 1);
        assert_eq!(received[0], data);
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

        // Send packets
        for _ in 0..100 {
            sim.send(vec![1, 2, 3]);
        }

        // Should all be dropped
        std::thread::sleep(Duration::from_millis(100));
        let received = sim.recv();
        assert_eq!(received.len(), 0);
    }

    #[test]
    fn test_latency() {
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
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(100));
    }

    #[test]
    fn test_network_profiles() {
        let profiles = vec![
            NetworkProfile::Lan,
            NetworkProfile::Cable,
            NetworkProfile::Dsl,
            NetworkProfile::FourG,
            NetworkProfile::ThreeG,
            NetworkProfile::Terrible,
        ];

        for profile in profiles {
            let conditions = profile.conditions();
            assert!(conditions.latency_ms > 0);
            assert!(conditions.packet_loss_percent >= 0.0);
            assert!(conditions.bandwidth_kbps > 0);
        }
    }

    #[test]
    fn test_in_flight_count() {
        let mut sim = NetworkSimulator::new(NetworkProfile::Dsl);

        assert_eq!(sim.in_flight(), 0);

        sim.send(vec![1, 2, 3]);
        sim.send(vec![4, 5, 6]);

        // Packets should be in flight (not delivered yet)
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
}
