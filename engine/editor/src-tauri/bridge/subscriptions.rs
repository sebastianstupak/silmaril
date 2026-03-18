//! Subscription tracking, throttling, and lifecycle management.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Unique identifier for a subscription.
pub type SubscriptionId = u64;

/// Configuration for creating a new subscription.
pub struct SubscriptionConfig {
    /// Optional entity filter.
    pub entity_id: Option<u64>,
    /// Optional throttle interval in milliseconds.
    pub throttle_ms: Option<u64>,
}

/// A single active subscription.
pub struct Subscription {
    /// Unique subscription identifier.
    pub id: SubscriptionId,
    /// Channel name this subscription listens on.
    pub channel: String,
    /// Optional entity filter.
    pub filter: Option<u64>,
    /// Minimum interval between pushes.
    pub throttle: Duration,
    /// Timestamp of the last push.
    pub last_push: Instant,
}

/// Manages active subscriptions with throttling support.
pub struct SubscriptionManager {
    subscriptions: HashMap<SubscriptionId, Subscription>,
    next_id: SubscriptionId,
}

impl SubscriptionManager {
    /// Creates a new empty subscription manager.
    pub fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
            next_id: 1,
        }
    }

    /// Registers a new subscription and returns its identifier.
    pub fn subscribe(&mut self, channel: String, config: SubscriptionConfig) -> SubscriptionId {
        let id = self.next_id;
        self.next_id += 1;
        let sub = Subscription {
            id,
            channel,
            filter: config.entity_id,
            throttle: Duration::from_millis(config.throttle_ms.unwrap_or(0)),
            last_push: Instant::now() - Duration::from_secs(1000), // ensure first push goes through
        };
        self.subscriptions.insert(id, sub);
        id
    }

    /// Removes a subscription. Returns true if it existed.
    pub fn unsubscribe(&mut self, id: SubscriptionId) -> bool {
        self.subscriptions.remove(&id).is_some()
    }

    /// Checks if a subscription should push now (respecting throttle).
    /// Updates the last push timestamp if returning true.
    pub fn should_push(&mut self, id: SubscriptionId) -> bool {
        if let Some(sub) = self.subscriptions.get_mut(&id) {
            if sub.last_push.elapsed() >= sub.throttle {
                sub.last_push = Instant::now();
                return true;
            }
        }
        false
    }

    /// Returns all subscriptions for a given channel.
    pub fn get_channel_subscriptions(&self, channel: &str) -> Vec<&Subscription> {
        self.subscriptions
            .values()
            .filter(|s| s.channel == channel)
            .collect()
    }
}
