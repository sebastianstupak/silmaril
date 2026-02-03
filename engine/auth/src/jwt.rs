//! JWT token generation and validation using RS256 (RSA with SHA-256).
//!
//! Implements secure token-based authentication with:
//! - Access tokens (1 hour lifetime)
//! - Refresh tokens (30 day lifetime)
//! - RS256 signing (asymmetric keys)
//! - Token revocation support
//! - Claims validation

use crate::error::AuthError;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use tracing::{debug, info};
use uuid::Uuid;

/// Access token lifetime (1 hour).
pub const ACCESS_TOKEN_LIFETIME_HOURS: i64 = 1;

/// Refresh token lifetime (30 days).
pub const REFRESH_TOKEN_LIFETIME_DAYS: i64 = 30;

/// JWT claims for access tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Username
    pub username: String,
    /// Email
    pub email: String,
    /// Issued at timestamp
    pub iat: i64,
    /// Expiration timestamp
    pub exp: i64,
    /// Token ID (for revocation)
    pub jti: String,
    /// Issuer
    pub iss: String,
}

/// JWT claims for refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Token ID (for revocation)
    pub jti: String,
    /// Issued at timestamp
    pub iat: i64,
    /// Expiration timestamp
    pub exp: i64,
    /// Issuer
    pub iss: String,
}

/// Token pair returned after successful authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    /// Access token (short-lived)
    pub access_token: String,
    /// Refresh token (long-lived)
    pub refresh_token: String,
    /// Access token expiration
    pub access_token_expires_at: DateTime<Utc>,
    /// Refresh token expiration
    pub refresh_token_expires_at: DateTime<Utc>,
}

/// JWT token manager with RS256 signing.
///
/// Uses RSA keys for signing and verification. Private key is used for signing,
/// public key for verification. This allows verification without access to the
/// private key (useful for distributed systems).
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    revoked_tokens: Arc<RwLock<HashSet<String>>>,
}

impl JwtManager {
    /// Create a new JWT manager with RSA keys.
    ///
    /// # Arguments
    ///
    /// * `private_key_pem` - RSA private key in PEM format (for signing)
    /// * `public_key_pem` - RSA public key in PEM format (for verification)
    /// * `issuer` - Token issuer identifier (e.g., "silmaril")
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::TokenGenerationFailed`] if keys are invalid.
    pub fn new(
        private_key_pem: &[u8],
        public_key_pem: &[u8],
        issuer: String,
    ) -> Result<Self, AuthError> {
        let encoding_key = EncodingKey::from_rsa_pem(private_key_pem).map_err(|e| {
            AuthError::TokenGenerationFailed {
                reason: format!("Failed to load private key: {e}"),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            }
        })?;

        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem).map_err(|e| {
            AuthError::TokenValidationFailed {
                reason: format!("Failed to load public key: {e}"),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            }
        })?;

        Ok(Self {
            encoding_key,
            decoding_key,
            issuer,
            revoked_tokens: Arc::new(RwLock::new(HashSet::new())),
        })
    }

    /// Generate a new token pair (access + refresh).
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::TokenGenerationFailed`] if generation fails.
    pub fn generate_token_pair(
        &self,
        user_id: &str,
        username: &str,
        email: &str,
    ) -> Result<TokenPair, AuthError> {
        let now = Utc::now();

        // Generate access token
        let access_token_id = Uuid::new_v4().to_string();
        let access_exp = now + Duration::hours(ACCESS_TOKEN_LIFETIME_HOURS);
        let access_claims = AccessTokenClaims {
            sub: user_id.to_string(),
            username: username.to_string(),
            email: email.to_string(),
            iat: now.timestamp(),
            exp: access_exp.timestamp(),
            jti: access_token_id.clone(),
            iss: self.issuer.clone(),
        };

        let access_token =
            encode(&Header::new(Algorithm::RS256), &access_claims, &self.encoding_key).map_err(
                |e| AuthError::TokenGenerationFailed {
                    reason: format!("Failed to encode access token: {e}"),
                    #[cfg(feature = "backtrace")]
                    backtrace: Backtrace::capture(),
                },
            )?;

        // Generate refresh token
        let refresh_token_id = Uuid::new_v4().to_string();
        let refresh_exp = now + Duration::days(REFRESH_TOKEN_LIFETIME_DAYS);
        let refresh_claims = RefreshTokenClaims {
            sub: user_id.to_string(),
            jti: refresh_token_id.clone(),
            iat: now.timestamp(),
            exp: refresh_exp.timestamp(),
            iss: self.issuer.clone(),
        };

        let refresh_token =
            encode(&Header::new(Algorithm::RS256), &refresh_claims, &self.encoding_key).map_err(
                |e| AuthError::TokenGenerationFailed {
                    reason: format!("Failed to encode refresh token: {e}"),
                    #[cfg(feature = "backtrace")]
                    backtrace: Backtrace::capture(),
                },
            )?;

        info!(
            user_id = user_id,
            access_token_id = %access_token_id,
            refresh_token_id = %refresh_token_id,
            "Generated token pair"
        );

        Ok(TokenPair {
            access_token,
            refresh_token,
            access_token_expires_at: access_exp,
            refresh_token_expires_at: refresh_exp,
        })
    }

    /// Validate and decode an access token.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::TokenValidationFailed`] if token is invalid.
    /// Returns [`AuthError::TokenExpired`] if token has expired.
    /// Returns [`AuthError::TokenRevoked`] if token has been revoked.
    pub fn validate_access_token(&self, token: &str) -> Result<AccessTokenClaims, AuthError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);

        let token_data = decode::<AccessTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| {
                // Check if it's an expiration error
                if let jsonwebtoken::errors::ErrorKind::ExpiredSignature = e.kind() {
                    AuthError::TokenExpired {
                        expired_at: Utc::now(), // Approximate
                        #[cfg(feature = "backtrace")]
                        backtrace: Backtrace::capture(),
                    }
                } else {
                    AuthError::TokenValidationFailed {
                        reason: format!("Token validation failed: {e}"),
                        #[cfg(feature = "backtrace")]
                        backtrace: Backtrace::capture(),
                    }
                }
            })?;

        let claims = token_data.claims;

        // Check if token is revoked
        if self.is_revoked(&claims.jti) {
            return Err(AuthError::TokenRevoked {
                token_id: claims.jti.clone(),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            });
        }

        debug!(
            user_id = %claims.sub,
            token_id = %claims.jti,
            "Access token validated"
        );

        Ok(claims)
    }

    /// Validate and decode a refresh token.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::TokenValidationFailed`] if token is invalid.
    /// Returns [`AuthError::TokenExpired`] if token has expired.
    /// Returns [`AuthError::TokenRevoked`] if token has been revoked.
    pub fn validate_refresh_token(&self, token: &str) -> Result<RefreshTokenClaims, AuthError> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);

        let token_data = decode::<RefreshTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| {
                if let jsonwebtoken::errors::ErrorKind::ExpiredSignature = e.kind() {
                    AuthError::TokenExpired {
                        expired_at: Utc::now(),
                        #[cfg(feature = "backtrace")]
                        backtrace: Backtrace::capture(),
                    }
                } else {
                    AuthError::TokenValidationFailed {
                        reason: format!("Token validation failed: {e}"),
                        #[cfg(feature = "backtrace")]
                        backtrace: Backtrace::capture(),
                    }
                }
            })?;

        let claims = token_data.claims;

        // Check if token is revoked
        if self.is_revoked(&claims.jti) {
            return Err(AuthError::TokenRevoked {
                token_id: claims.jti.clone(),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            });
        }

        debug!(
            user_id = %claims.sub,
            token_id = %claims.jti,
            "Refresh token validated"
        );

        Ok(claims)
    }

    /// Revoke a token by its ID (jti claim).
    ///
    /// Revoked tokens are stored in memory. In production, this should be
    /// backed by Redis or a database.
    pub fn revoke_token(&self, token_id: &str) {
        let mut revoked = self.revoked_tokens.write().unwrap();
        revoked.insert(token_id.to_string());
        info!(token_id = token_id, "Token revoked");
    }

    /// Check if a token is revoked.
    #[must_use]
    pub fn is_revoked(&self, token_id: &str) -> bool {
        let revoked = self.revoked_tokens.read().unwrap();
        revoked.contains(token_id)
    }

    /// Clear all revoked tokens (for testing or cleanup).
    pub fn clear_revoked_tokens(&self) {
        let mut revoked = self.revoked_tokens.write().unwrap();
        revoked.clear();
    }
}

/// Generate RSA key pair for testing.
///
/// **WARNING:** Only use for testing! In production, generate proper RSA keys
/// using openssl or similar tools.
#[cfg(test)]
pub fn generate_test_rsa_keys() -> (Vec<u8>, Vec<u8>) {
    use rsa::{
        pkcs8::{EncodePrivateKey, EncodePublicKey},
        RsaPrivateKey,
    };

    let mut rng = rand::thread_rng();
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate key");
    let public_key = private_key.to_public_key();

    let private_pem = private_key
        .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
        .expect("failed to encode private key")
        .as_bytes()
        .to_vec();

    let public_pem = public_key
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .expect("failed to encode public key")
        .as_bytes()
        .to_vec();

    (private_pem, public_pem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> JwtManager {
        let (private_pem, public_pem) = generate_test_rsa_keys();
        JwtManager::new(&private_pem, &public_pem, "test-issuer".to_string()).unwrap()
    }

    #[test]
    fn test_generate_token_pair() {
        let manager = create_test_manager();
        let pair = manager.generate_token_pair("user123", "testuser", "test@example.com").unwrap();

        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert!(pair.access_token_expires_at > Utc::now());
        assert!(pair.refresh_token_expires_at > pair.access_token_expires_at);
    }

    #[test]
    fn test_validate_access_token() {
        let manager = create_test_manager();
        let pair = manager.generate_token_pair("user123", "testuser", "test@example.com").unwrap();

        let claims = manager.validate_access_token(&pair.access_token).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.username, "testuser");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.iss, "test-issuer");
    }

    #[test]
    fn test_validate_refresh_token() {
        let manager = create_test_manager();
        let pair = manager.generate_token_pair("user123", "testuser", "test@example.com").unwrap();

        let claims = manager.validate_refresh_token(&pair.refresh_token).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.iss, "test-issuer");
    }

    #[test]
    fn test_revoke_token() {
        let manager = create_test_manager();
        let pair = manager.generate_token_pair("user123", "testuser", "test@example.com").unwrap();

        // Validate first time - should succeed
        let claims = manager.validate_access_token(&pair.access_token).unwrap();

        // Revoke token
        manager.revoke_token(&claims.jti);

        // Validate again - should fail
        let result = manager.validate_access_token(&pair.access_token);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::TokenRevoked { .. }));
    }

    #[test]
    fn test_invalid_token() {
        let manager = create_test_manager();
        let result = manager.validate_access_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_with_wrong_issuer() {
        let (private_pem, public_pem) = generate_test_rsa_keys();
        let manager1 = JwtManager::new(&private_pem, &public_pem, "issuer1".to_string()).unwrap();
        let manager2 = JwtManager::new(&private_pem, &public_pem, "issuer2".to_string()).unwrap();

        let pair = manager1.generate_token_pair("user123", "testuser", "test@example.com").unwrap();

        // Should fail - wrong issuer
        let result = manager2.validate_access_token(&pair.access_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_revoked_tokens() {
        let manager = create_test_manager();
        manager.revoke_token("token1");
        manager.revoke_token("token2");

        assert!(manager.is_revoked("token1"));
        assert!(manager.is_revoked("token2"));

        manager.clear_revoked_tokens();

        assert!(!manager.is_revoked("token1"));
        assert!(!manager.is_revoked("token2"));
    }
}
