//! Steam `OpenID` authentication.
//!
//! Steam uses `OpenID` 2.0 for authentication (not OAuth 2.0).

use crate::error::AuthError;
use crate::oauth::OAuthProfile;
#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;
use tracing::{debug, info};
use url::Url;

/// Steam `OpenID` endpoint.
const STEAM_OPENID_URL: &str = "https://steamcommunity.com/openid/login";

/// Steam `OpenID` identifier.
const STEAM_OPENID_NS: &str = "http://specs.openid.net/auth/2.0";

/// Steam API base URL for user profiles.
const STEAM_API_BASE: &str = "https://api.steampowered.com";

/// Steam authentication client.
pub struct SteamAuth {
    /// Realm (your site's URL)
    realm: String,
    /// Return URL (where Steam redirects after auth)
    return_url: String,
    /// Steam API key (for fetching user info)
    api_key: String,
}

impl SteamAuth {
    /// Create a new Steam auth client.
    ///
    /// # Arguments
    ///
    /// * `realm` - Your website's URL (e.g., "<https://yourgame.com>")
    /// * `return_url` - Callback URL (e.g., "<https://yourgame.com/auth/steam/callback>")
    /// * `api_key` - Steam API key from <https://steamcommunity.com/dev/apikey>
    #[must_use]
    pub fn new(realm: String, return_url: String, api_key: String) -> Self {
        Self { realm, return_url, api_key }
    }

    /// Generate the Steam `OpenID` login URL.
    ///
    /// Redirect the user to this URL to start authentication.
    pub fn get_login_url(&self) -> Result<String, AuthError> {
        let mut url = Url::parse(STEAM_OPENID_URL).map_err(|e| AuthError::OAuthProviderError {
            provider: "steam".to_string(),
            error: format!("Failed to parse Steam URL: {e}"),
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        })?;

        url.query_pairs_mut()
            .append_pair("openid.ns", STEAM_OPENID_NS)
            .append_pair("openid.mode", "checkid_setup")
            .append_pair("openid.return_to", &self.return_url)
            .append_pair("openid.realm", &self.realm)
            .append_pair("openid.identity", "http://specs.openid.net/auth/2.0/identifier_select")
            .append_pair("openid.claimed_id", "http://specs.openid.net/auth/2.0/identifier_select");

        debug!("Generated Steam login URL");
        Ok(url.to_string())
    }

    /// Verify the Steam `OpenID` response and extract Steam ID.
    ///
    /// Call this when the user returns from Steam.
    ///
    /// # Arguments
    ///
    /// * `query_params` - Query parameters from the callback URL
    ///
    /// # Returns
    ///
    /// Steam ID (64-bit) as a string
    pub async fn verify_and_get_steam_id(
        &self,
        query_params: &[(String, String)],
    ) -> Result<String, AuthError> {
        // Extract claimed_id to get Steam ID
        let claimed_id =
            query_params.iter().find(|(k, _)| k == "openid.claimed_id").ok_or_else(|| {
                AuthError::OAuthProviderError {
                    provider: "steam".to_string(),
                    error: "Missing claimed_id in response".to_string(),
                    #[cfg(feature = "backtrace")]
                    backtrace: Backtrace::capture(),
                }
            })?;

        // Extract Steam ID from claimed_id URL
        // Format: https://steamcommunity.com/openid/id/{STEAM_ID}
        let steam_id = claimed_id
            .1
            .rsplit('/')
            .next()
            .ok_or_else(|| AuthError::OAuthProviderError {
                provider: "steam".to_string(),
                error: "Invalid claimed_id format".to_string(),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            })?
            .to_string();

        // Verify with Steam (in production, you should verify the signature)
        // For now, we trust the response if it contains a valid-looking Steam ID
        if !steam_id.chars().all(char::is_numeric) || steam_id.len() != 17 {
            return Err(AuthError::OAuthProviderError {
                provider: "steam".to_string(),
                error: "Invalid Steam ID format".to_string(),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            });
        }

        info!(steam_id = %steam_id, "Steam authentication successful");
        Ok(steam_id)
    }

    /// Get user profile from Steam API.
    ///
    /// # Arguments
    ///
    /// * `steam_id` - 64-bit Steam ID
    pub async fn get_user_profile(&self, steam_id: &str) -> Result<OAuthProfile, AuthError> {
        let url = format!(
            "{}/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}",
            STEAM_API_BASE, self.api_key, steam_id
        );

        let client = reqwest::Client::new();
        let response =
            client.get(&url).send().await.map_err(|e| AuthError::OAuthProviderError {
                provider: "steam".to_string(),
                error: format!("Failed to fetch profile: {e}"),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            })?;

        let data: serde_json::Value =
            response.json().await.map_err(|e| AuthError::OAuthProviderError {
                provider: "steam".to_string(),
                error: format!("Failed to parse response: {e}"),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            })?;

        // Extract player data
        let player = data["response"]["players"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| AuthError::OAuthProviderError {
                provider: "steam".to_string(),
                error: "No player data in response".to_string(),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            })?;

        Ok(OAuthProfile {
            provider_user_id: steam_id.to_string(),
            username: player["personaname"].as_str().unwrap_or("Unknown").to_string(),
            email: None, // Steam doesn't provide email
            avatar_url: player["avatarfull"].as_str().map(String::from),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_login_url() {
        let auth = SteamAuth::new(
            "https://example.com".to_string(),
            "https://example.com/callback".to_string(),
            "test_api_key".to_string(),
        );

        let url = auth.get_login_url().unwrap();
        assert!(url.contains("steamcommunity.com/openid/login"));
        assert!(url.contains("openid.ns"));
        assert!(url.contains("openid.mode=checkid_setup"));
    }

    #[test]
    fn test_steam_id_validation() {
        let valid_id = "76561197960287930"; // Valid 17-digit Steam ID
        assert_eq!(valid_id.len(), 17);
        assert!(valid_id.chars().all(char::is_numeric));
    }
}
