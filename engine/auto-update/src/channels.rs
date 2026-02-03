//! Update channel management (stable, beta, etc.).

use crate::error::UpdateError;
use crate::version::Version;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Update channel type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Channel {
    /// Stable release channel
    #[default]
    Stable,
    /// Beta testing channel
    Beta,
    /// Development/alpha channel
    Dev,
}

impl Channel {
    /// Get the channel name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
            Channel::Dev => "dev",
        }
    }

    /// Parse a channel from a string.
    pub fn from_str(s: &str) -> Result<Self, UpdateError> {
        match s.to_lowercase().as_str() {
            "stable" => Ok(Channel::Stable),
            "beta" => Ok(Channel::Beta),
            "dev" | "development" | "alpha" => Ok(Channel::Dev),
            _ => Err(UpdateError::channelnotfound(s.to_string())),
        }
    }

    /// Check if this channel is more stable than another.
    pub fn is_more_stable_than(&self, other: &Channel) -> bool {
        self.stability_level() > other.stability_level()
    }

    fn stability_level(&self) -> u8 {
        match self {
            Channel::Stable => 2,
            Channel::Beta => 1,
            Channel::Dev => 0,
        }
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Channel subscription information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSubscription {
    /// Current channel
    pub channel: Channel,
    /// Whether to automatically switch to stable when beta becomes stable
    pub auto_promote: bool,
    /// Opt-in for beta testing
    pub beta_opt_in: bool,
}

impl ChannelSubscription {
    /// Create a new channel subscription.
    pub fn new(channel: Channel) -> Self {
        Self { channel, auto_promote: false, beta_opt_in: channel != Channel::Stable }
    }

    /// Switch to a different channel.
    pub fn switch_to(&mut self, channel: Channel) -> Result<(), UpdateError> {
        // Validate channel switch
        if channel == Channel::Beta && !self.beta_opt_in {
            return Err(UpdateError::channelnotfound("Beta channel requires opt-in".to_string()));
        }

        self.channel = channel;
        Ok(())
    }

    /// Check if a version can be installed on this channel.
    pub fn can_install_version(&self, _version: &Version, version_channel: &Channel) -> bool {
        // Can always downgrade to more stable channels
        if version_channel.is_more_stable_than(&self.channel) {
            return true;
        }

        // Can install same channel
        if version_channel == &self.channel {
            return true;
        }

        // Cannot install less stable versions unless explicitly allowed
        false
    }
}

impl Default for ChannelSubscription {
    fn default() -> Self {
        Self::new(Channel::Stable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_as_str() {
        assert_eq!(Channel::Stable.as_str(), "stable");
        assert_eq!(Channel::Beta.as_str(), "beta");
        assert_eq!(Channel::Dev.as_str(), "dev");
    }

    #[test]
    fn test_channel_from_str() {
        assert_eq!(Channel::from_str("stable").unwrap(), Channel::Stable);
        assert_eq!(Channel::from_str("STABLE").unwrap(), Channel::Stable);
        assert_eq!(Channel::from_str("beta").unwrap(), Channel::Beta);
        assert_eq!(Channel::from_str("dev").unwrap(), Channel::Dev);
        assert_eq!(Channel::from_str("development").unwrap(), Channel::Dev);
        assert_eq!(Channel::from_str("alpha").unwrap(), Channel::Dev);
        assert!(Channel::from_str("invalid").is_err());
    }

    #[test]
    fn test_channel_display() {
        assert_eq!(format!("{}", Channel::Stable), "stable");
        assert_eq!(format!("{}", Channel::Beta), "beta");
        assert_eq!(format!("{}", Channel::Dev), "dev");
    }

    #[test]
    fn test_channel_stability() {
        assert!(Channel::Stable.is_more_stable_than(&Channel::Beta));
        assert!(Channel::Stable.is_more_stable_than(&Channel::Dev));
        assert!(Channel::Beta.is_more_stable_than(&Channel::Dev));
        assert!(!Channel::Dev.is_more_stable_than(&Channel::Stable));
    }

    #[test]
    fn test_channel_subscription_new() {
        let sub = ChannelSubscription::new(Channel::Stable);
        assert_eq!(sub.channel, Channel::Stable);
        assert!(!sub.auto_promote);
        assert!(!sub.beta_opt_in);

        let beta_sub = ChannelSubscription::new(Channel::Beta);
        assert_eq!(beta_sub.channel, Channel::Beta);
        assert!(beta_sub.beta_opt_in);
    }

    #[test]
    fn test_channel_subscription_switch() {
        let mut sub = ChannelSubscription::new(Channel::Stable);

        // Cannot switch to beta without opt-in
        assert!(sub.switch_to(Channel::Beta).is_err());

        // Enable beta opt-in
        sub.beta_opt_in = true;
        assert!(sub.switch_to(Channel::Beta).is_ok());
        assert_eq!(sub.channel, Channel::Beta);

        // Can switch back to stable
        assert!(sub.switch_to(Channel::Stable).is_ok());
        assert_eq!(sub.channel, Channel::Stable);
    }

    #[test]
    fn test_can_install_version() {
        let stable_sub = ChannelSubscription::new(Channel::Stable);
        let beta_sub = ChannelSubscription::new(Channel::Beta);

        let version = Version::new(1, 0, 0);

        // Stable subscription can install stable versions
        assert!(stable_sub.can_install_version(&version, &Channel::Stable));

        // Stable subscription cannot install beta versions
        assert!(!stable_sub.can_install_version(&version, &Channel::Beta));

        // Beta subscription can install both beta and stable versions
        assert!(beta_sub.can_install_version(&version, &Channel::Beta));
        assert!(beta_sub.can_install_version(&version, &Channel::Stable));
    }

    #[test]
    fn test_channel_serialization() {
        let channel = Channel::Beta;
        let json = serde_json::to_string(&channel).unwrap();
        let deserialized: Channel = serde_json::from_str(&json).unwrap();
        assert_eq!(channel, deserialized);
    }
}
