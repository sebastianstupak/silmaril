//! Delta compression for network state synchronization
//!
//! This module provides network-specific delta compression functionality,
//! wrapping the core delta compression with network-aware optimizations
//! and adaptive switching between full snapshots and deltas.

use engine_core::serialization::{WorldState, WorldStateDelta};
use serde::{Deserialize, Serialize};

/// Network delta packet
///
/// Wraps delta compression with metadata for network transmission,
/// including decision logic for when to send full vs delta snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDelta {
    /// The actual delta data
    pub delta: WorldStateDelta,
    /// Size of this delta when serialized
    pub delta_size: usize,
    /// Size of full snapshot when serialized
    pub full_size: usize,
    /// Compression ratio (delta_size / full_size)
    pub compression_ratio: f32,
}

impl NetworkDelta {
    /// Create a network delta from two world states
    ///
    /// Automatically computes compression ratio and metadata.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_networking::delta::NetworkDelta;
    /// # use engine_core::serialization::WorldState;
    /// # let old_state = WorldState::new();
    /// # let new_state = WorldState::new();
    /// let net_delta = NetworkDelta::from_states(&old_state, &new_state);
    /// ```
    pub fn from_states(old: &WorldState, new: &WorldState) -> Self {
        let delta = WorldStateDelta::compute(old, new);

        // Compute sizes using bincode (network format)
        let delta_size = bincode::serialize(&delta).unwrap().len();
        let full_size = bincode::serialize(new).unwrap().len();
        let compression_ratio = delta_size as f32 / full_size as f32;

        Self { delta, delta_size, full_size, compression_ratio }
    }

    /// Check if sending delta is more efficient than full snapshot
    ///
    /// Returns true if delta should be sent, false if full snapshot is better.
    /// Includes a small overhead threshold to account for processing costs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_networking::delta::NetworkDelta;
    /// # use engine_core::serialization::WorldState;
    /// # let old_state = WorldState::new();
    /// # let new_state = WorldState::new();
    /// let net_delta = NetworkDelta::from_states(&old_state, &new_state);
    /// if net_delta.should_use_delta() {
    ///     // Send delta
    /// } else {
    ///     // Send full snapshot
    /// }
    /// ```
    pub fn should_use_delta(&self) -> bool {
        // Use delta if it's at least 20% smaller than full snapshot
        // This threshold accounts for processing overhead
        self.compression_ratio < 0.8
    }

    /// Serialize to bytes for network transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    /// Deserialize from network bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }

    /// Apply this delta to a base state
    pub fn apply(&self, base: &mut WorldState) {
        self.delta.apply(base);
    }
}

/// Adaptive delta strategy
///
/// Decides when to send full snapshots vs deltas based on bandwidth
/// and change rate statistics.
pub struct AdaptiveDeltaStrategy {
    /// Recent compression ratios (for averaging)
    recent_ratios: Vec<f32>,
    /// Maximum history size
    max_history: usize,
    /// Threshold for switching to full snapshot
    threshold: f32,
}

impl AdaptiveDeltaStrategy {
    /// Create a new adaptive strategy
    ///
    /// # Arguments
    ///
    /// * `max_history` - Number of recent deltas to track
    /// * `threshold` - Compression ratio threshold (default 0.8)
    pub fn new(max_history: usize, threshold: f32) -> Self {
        Self { recent_ratios: Vec::with_capacity(max_history), max_history, threshold }
    }

    /// Record a delta's compression ratio
    pub fn record_delta(&mut self, ratio: f32) {
        self.recent_ratios.push(ratio);
        if self.recent_ratios.len() > self.max_history {
            self.recent_ratios.remove(0);
        }
    }

    /// Decide whether to use delta for next update
    ///
    /// Based on recent compression ratio history, decides if delta
    /// compression is worthwhile or if a full snapshot should be sent.
    pub fn should_use_delta(&self, current_ratio: f32) -> bool {
        if self.recent_ratios.is_empty() {
            return current_ratio < self.threshold;
        }

        // Use average of recent ratios to smooth out decisions
        let avg_ratio: f32 = self.recent_ratios.iter().sum::<f32>() / self.recent_ratios.len() as f32;

        // Use delta if both current and average are below threshold
        current_ratio < self.threshold && avg_ratio < self.threshold
    }

    /// Get average compression ratio
    pub fn average_ratio(&self) -> f32 {
        if self.recent_ratios.is_empty() {
            return 1.0;
        }
        self.recent_ratios.iter().sum::<f32>() / self.recent_ratios.len() as f32
    }

    /// Reset history
    pub fn reset(&mut self) {
        self.recent_ratios.clear();
    }
}

impl Default for AdaptiveDeltaStrategy {
    fn default() -> Self {
        Self::new(10, 0.8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_delta_creation() {
        let state1 = WorldState::new();
        let state2 = WorldState::new();

        let net_delta = NetworkDelta::from_states(&state1, &state2);

        assert!(net_delta.delta_size > 0);
        assert!(net_delta.full_size > 0);
        assert!(net_delta.compression_ratio >= 0.0);
    }

    #[test]
    fn test_network_delta_serialization() {
        let state1 = WorldState::new();
        let state2 = WorldState::new();

        let net_delta = NetworkDelta::from_states(&state1, &state2);
        let bytes = net_delta.to_bytes();
        let restored = NetworkDelta::from_bytes(&bytes).unwrap();

        assert_eq!(net_delta.delta_size, restored.delta_size);
        assert_eq!(net_delta.full_size, restored.full_size);
    }

    #[test]
    fn test_adaptive_strategy() {
        let mut strategy = AdaptiveDeltaStrategy::new(5, 0.8);

        // Record some good compression ratios
        strategy.record_delta(0.3);
        strategy.record_delta(0.4);
        strategy.record_delta(0.5);

        // Should use delta with good average
        assert!(strategy.should_use_delta(0.6));

        // Record some bad ratios
        strategy.record_delta(0.9);
        strategy.record_delta(0.95);

        // Should not use delta with bad average
        assert!(!strategy.should_use_delta(0.85));
    }

    #[test]
    fn test_adaptive_strategy_history_limit() {
        let mut strategy = AdaptiveDeltaStrategy::new(3, 0.8);

        strategy.record_delta(0.1);
        strategy.record_delta(0.2);
        strategy.record_delta(0.3);
        strategy.record_delta(0.4); // Should evict 0.1

        assert_eq!(strategy.recent_ratios.len(), 3);
        assert_eq!(strategy.recent_ratios[0], 0.2);
    }

    #[test]
    fn test_adaptive_strategy_reset() {
        let mut strategy = AdaptiveDeltaStrategy::new(5, 0.8);

        strategy.record_delta(0.5);
        strategy.record_delta(0.6);
        assert_eq!(strategy.recent_ratios.len(), 2);

        strategy.reset();
        assert_eq!(strategy.recent_ratios.len(), 0);
    }
}
