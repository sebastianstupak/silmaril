//! OAuth 2.0 providers for social login.
//!
//! Supports Steam, Discord, and Epic Games authentication.

pub mod discord;
pub mod steam;

use crate::error::AuthError;
use serde::{Deserialize, Serialize};
#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;

/// OAuth provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OAuthProvider {
    /// Steam
    Steam,
    /// Discord
    Discord,
    /// Epic Games
    Epic,
}

impl OAuthProvider {
    /// Get provider name as string.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Steam => "steam",
            Self::Discord => "discord",
            Self::Epic => "epic",
        }
    }

    /// Parse provider from string.
    pub fn from_str(s: &str) -> Result<Self, AuthError> {
        match s.to_lowercase().as_str() {
            "steam" => Ok(Self::Steam),
            "discord" => Ok(Self::Discord),
            "epic" => Ok(Self::Epic),
            _ => Err(AuthError::OAuthProviderError {
                provider: s.to_string(),
                error: "Unknown provider".to_string(),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            }),
        }
    }
}

/// OAuth user profile information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProfile {
    /// Provider-specific user ID
    pub provider_user_id: String,
    /// Username/display name
    pub username: String,
    /// Email (if available)
    pub email: Option<String>,
    /// Avatar URL (if available)
    pub avatar_url: Option<String>,
}

/// OAuth state for preventing CSRF attacks.
///
/// This should be stored in a session and verified when the user returns from OAuth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    /// Random state token
    pub token: String,
    /// Provider
    pub provider: OAuthProvider,
    /// Redirect URI
    pub redirect_uri: String,
    /// Expiration timestamp
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl OAuthState {
    /// Create a new OAuth state.
    #[must_use]
    pub fn new(provider: OAuthProvider, redirect_uri: String) -> Self {
        Self {
            token: uuid::Uuid::new_v4().to_string(),
            provider,
            redirect_uri,
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
        }
    }

    /// Check if state has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_as_str() {
        assert_eq!(OAuthProvider::Steam.as_str(), "steam");
        assert_eq!(OAuthProvider::Discord.as_str(), "discord");
        assert_eq!(OAuthProvider::Epic.as_str(), "epic");
    }

    #[test]
    fn test_provider_from_str() {
        assert_eq!(OAuthProvider::from_str("steam").unwrap(), OAuthProvider::Steam);
        assert_eq!(OAuthProvider::from_str("DISCORD").unwrap(), OAuthProvider::Discord);
        assert!(OAuthProvider::from_str("invalid").is_err());
    }

    #[test]
    fn test_oauth_state_creation() {
        let state = OAuthState::new(OAuthProvider::Steam, "http://localhost/callback".to_string());
        assert!(!state.token.is_empty());
        assert_eq!(state.provider, OAuthProvider::Steam);
        assert!(!state.is_expired());
    }

    #[test]
    fn test_oauth_state_expiration() {
        let mut state =
            OAuthState::new(OAuthProvider::Steam, "http://localhost/callback".to_string());

        // Set expiration to past
        state.expires_at = chrono::Utc::now() - chrono::Duration::hours(1);
        assert!(state.is_expired());
    }
}
