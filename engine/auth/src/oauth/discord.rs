//! Discord OAuth 2.0 authentication.

use crate::error::AuthError;
use crate::oauth::OAuthProfile;
use serde::Deserialize;
#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;
use tracing::{debug, info};
use url::Url;

/// Discord OAuth endpoints.
const DISCORD_OAUTH_URL: &str = "https://discord.com/api/oauth2/authorize";
const DISCORD_TOKEN_URL: &str = "https://discord.com/api/oauth2/token";
const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

/// Discord token response.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    expires_in: u64,
    #[allow(dead_code)]
    refresh_token: String,
    #[allow(dead_code)]
    scope: String,
}

/// Discord user response.
#[derive(Debug, Deserialize)]
struct UserResponse {
    id: String,
    username: String,
    discriminator: String,
    email: Option<String>,
    avatar: Option<String>,
}

/// Discord OAuth client.
pub struct DiscordAuth {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl DiscordAuth {
    /// Create a new Discord auth client.
    ///
    /// Get credentials from <https://discord.com/developers/applications>
    #[must_use]
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self { client_id, client_secret, redirect_uri }
    }

    /// Generate the Discord OAuth authorization URL.
    ///
    /// # Arguments
    ///
    /// * `state` - CSRF protection token
    pub fn get_authorization_url(&self, state: &str) -> Result<String, AuthError> {
        let mut url = Url::parse(DISCORD_OAUTH_URL).map_err(|e| AuthError::OAuthProviderError {
            provider: "discord".to_string(),
            error: format!("Failed to parse Discord URL: {e}"),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        })?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", "identify email")
            .append_pair("state", state);

        debug!("Generated Discord authorization URL");
        Ok(url.to_string())
    }

    /// Exchange authorization code for access token.
    pub async fn exchange_code(&self, code: &str) -> Result<String, AuthError> {
        let client = reqwest::Client::new();

        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.redirect_uri.as_str()),
        ];

        let response = client.post(DISCORD_TOKEN_URL).form(&params).send().await.map_err(|e| {
            AuthError::OAuthTokenExchangeFailed {
                provider: "discord".to_string(),
                reason: format!("Token exchange failed: {e}"),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            }
        })?;

        let token_data: TokenResponse =
            response.json().await.map_err(|e| AuthError::OAuthTokenExchangeFailed {
                provider: "discord".to_string(),
                reason: format!("Failed to parse token response: {e}"),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            })?;

        info!("Discord token exchange successful");
        Ok(token_data.access_token)
    }

    /// Get user profile using access token.
    pub async fn get_user_profile(&self, access_token: &str) -> Result<OAuthProfile, AuthError> {
        let client = reqwest::Client::new();
        let url = format!("{DISCORD_API_BASE}/users/@me");

        let response = client.get(&url).bearer_auth(access_token).send().await.map_err(|e| {
            AuthError::OAuthProviderError {
                provider: "discord".to_string(),
                error: format!("Failed to fetch profile: {e}"),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            }
        })?;

        let user: UserResponse =
            response.json().await.map_err(|e| AuthError::OAuthProviderError {
                provider: "discord".to_string(),
                error: format!("Failed to parse user response: {e}"),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            })?;

        let avatar_url = user
            .avatar
            .as_ref()
            .map(|hash| format!("https://cdn.discordapp.com/avatars/{}/{}.png", user.id, hash));

        Ok(OAuthProfile {
            provider_user_id: user.id,
            username: format!("{}#{}", user.username, user.discriminator),
            email: user.email,
            avatar_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_authorization_url() {
        let auth = DiscordAuth::new(
            "test_client_id".to_string(),
            "test_secret".to_string(),
            "https://example.com/callback".to_string(),
        );

        let url = auth.get_authorization_url("test_state").unwrap();
        assert!(url.contains("discord.com/api/oauth2/authorize"));
        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("state=test_state"));
        assert!(url.contains("scope=identify+email"));
    }
}
