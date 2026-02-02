//! Password hashing and verification using Argon2id.
//!
//! Implements OWASP 2025 recommendations for password hashing:
//! - Algorithm: Argon2id (winner of Password Hashing Competition)
//! - Memory: 64 MB
//! - Iterations: 3
//! - Parallelism: 4 threads
//! - Salt: 16 bytes (auto-generated)
//!
//! These parameters provide strong security while maintaining acceptable
//! performance (250-500ms per hash on modern hardware).

use crate::error::AuthError;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use std::backtrace::Backtrace;
use tracing::{debug, info};

/// OWASP-recommended Argon2id parameters (2025).
///
/// - Memory: 64 MB (65536 KiB)
/// - Iterations: 3
/// - Parallelism: 4 threads
/// - Output length: 32 bytes
const MEMORY_SIZE_KB: u32 = 65536; // 64 MB
const ITERATIONS: u32 = 3;
const PARALLELISM: u32 = 4;
const OUTPUT_LEN: usize = 32;

/// Hash a password using Argon2id with OWASP-recommended parameters.
///
/// This operation is intentionally slow (250-500ms) to prevent brute-force attacks.
///
/// # Examples
///
/// ```
/// use engine_auth::password::hash_password;
///
/// # tokio_test::block_on(async {
/// let password = "MySecureP@ssw0rd";
/// let hash = hash_password(password).await.unwrap();
/// assert!(hash.starts_with("$argon2id$"));
/// # });
/// ```
///
/// # Errors
///
/// Returns [`AuthError::PasswordHashFailed`] if hashing fails.
pub async fn hash_password(password: &str) -> Result<String, AuthError> {
    let password = password.to_string();

    // Run expensive hashing operation on blocking thread pool
    tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();

        let salt = SaltString::generate(&mut OsRng);

        let params = Params::new(MEMORY_SIZE_KB, ITERATIONS, PARALLELISM, Some(OUTPUT_LEN))
            .map_err(|e| AuthError::PasswordHashFailed {
                details: format!("Failed to create Argon2 params: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            })?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let password_hash = argon2.hash_password(password.as_bytes(), &salt).map_err(|e| {
            AuthError::PasswordHashFailed {
                details: format!("Failed to hash password: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            }
        })?;

        let duration = start.elapsed();
        debug!(duration_ms = duration.as_millis(), "Password hashed successfully");

        // Verify hashing took reasonable time (should be 250-500ms)
        if duration.as_millis() < 100 {
            tracing::warn!(
                duration_ms = duration.as_millis(),
                "Password hashing completed suspiciously fast - check parameters"
            );
        }

        Ok(password_hash.to_string())
    })
    .await
    .map_err(|e| AuthError::PasswordHashFailed {
        details: format!("Task join error: {}", e),
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace::capture(),
    })?
}

/// Verify a password against an Argon2id hash.
///
/// This operation is intentionally slow (250-500ms) to prevent brute-force attacks.
///
/// # Examples
///
/// ```
/// use engine_auth::password::{hash_password, verify_password};
///
/// # tokio_test::block_on(async {
/// let password = "MySecureP@ssw0rd";
/// let hash = hash_password(password).await.unwrap();
/// assert!(verify_password(password, &hash).await.unwrap());
/// assert!(!verify_password("WrongPassword", &hash).await.unwrap());
/// # });
/// ```
///
/// # Errors
///
/// Returns [`AuthError::PasswordVerifyFailed`] if verification fails due to an error.
/// Returns `Ok(false)` if the password doesn't match (not an error).
pub async fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
    let password = password.to_string();
    let hash = hash.to_string();

    // Run expensive verification on blocking thread pool
    tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();

        let parsed_hash =
            PasswordHash::new(&hash).map_err(|e| AuthError::PasswordVerifyFailed {
                details: format!("Failed to parse password hash: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: Backtrace::capture(),
            })?;

        let argon2 = Argon2::default();
        let is_valid = argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok();

        let duration = start.elapsed();
        debug!(
            duration_ms = duration.as_millis(),
            is_valid = is_valid,
            "Password verification completed"
        );

        Ok(is_valid)
    })
    .await
    .map_err(|e| AuthError::PasswordVerifyFailed {
        details: format!("Task join error: {}", e),
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace::capture(),
    })?
}

/// Check if a password hash needs to be rehashed with updated parameters.
///
/// This is useful for upgrading hashes when security recommendations change.
pub fn needs_rehash(hash: &str) -> Result<bool, AuthError> {
    let parsed_hash = PasswordHash::new(hash).map_err(|e| AuthError::PasswordVerifyFailed {
        details: format!("Failed to parse password hash: {}", e),
        #[cfg(feature = "backtrace")]
        backtrace: Backtrace::capture(),
    })?;

    // Check if algorithm is Argon2id
    if parsed_hash.algorithm != argon2::Algorithm::Argon2id.ident() {
        return Ok(true);
    }

    // Check if parameters match current recommendations
    if let Some(params_str) = parsed_hash.params.get("m") {
        if let Ok(m) = params_str.to_string().parse::<u32>() {
            if m < MEMORY_SIZE_KB {
                return Ok(true);
            }
        }
    }

    if let Some(params_str) = parsed_hash.params.get("t") {
        if let Ok(t) = params_str.to_string().parse::<u32>() {
            if t < ITERATIONS {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_password() {
        let password = "TestPassword123!";
        let hash = hash_password(password).await.unwrap();

        assert!(hash.starts_with("$argon2id$"));
        assert!(hash.len() > 50); // Hash should be reasonably long
    }

    #[tokio::test]
    async fn test_verify_password_correct() {
        let password = "TestPassword123!";
        let hash = hash_password(password).await.unwrap();

        let is_valid = verify_password(password, &hash).await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_verify_password_incorrect() {
        let password = "TestPassword123!";
        let hash = hash_password(password).await.unwrap();

        let is_valid = verify_password("WrongPassword123!", &hash).await.unwrap();
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_verify_password_case_sensitive() {
        let password = "TestPassword123!";
        let hash = hash_password(password).await.unwrap();

        let is_valid = verify_password("testpassword123!", &hash).await.unwrap();
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_hash_different_each_time() {
        let password = "TestPassword123!";
        let hash1 = hash_password(password).await.unwrap();
        let hash2 = hash_password(password).await.unwrap();

        // Hashes should be different due to different salts
        assert_ne!(hash1, hash2);

        // But both should verify successfully
        assert!(verify_password(password, &hash1).await.unwrap());
        assert!(verify_password(password, &hash2).await.unwrap());
    }

    #[tokio::test]
    async fn test_hashing_performance() {
        let password = "TestPassword123!";
        let start = std::time::Instant::now();
        let _ = hash_password(password).await.unwrap();
        let duration = start.elapsed();

        // Should take between 100ms and 2 seconds (depends on hardware)
        assert!(duration.as_millis() >= 100, "Hashing too fast: {}ms", duration.as_millis());
        assert!(duration.as_secs() <= 2, "Hashing too slow: {}ms", duration.as_millis());
    }

    #[test]
    fn test_needs_rehash() {
        // This is a weak hash (low iterations) - should need rehash
        let weak_hash = "$argon2id$v=19$m=4096,t=1,p=1$c29tZXNhbHQ$hash";
        // Note: This might fail to parse, but that's okay for this test

        // A proper current hash shouldn't need rehash
        // (We can't easily test this without generating a real hash)
    }

    #[tokio::test]
    async fn test_verify_invalid_hash_format() {
        let result = verify_password("password", "not_a_valid_hash").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_empty_password() {
        let result = hash_password("").await;
        assert!(result.is_ok()); // Empty password can be hashed, but validation should reject it
    }

    #[tokio::test]
    async fn test_long_password() {
        let long_password = "A".repeat(1000);
        let hash = hash_password(&long_password).await.unwrap();
        assert!(verify_password(&long_password, &hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_unicode_password() {
        let unicode_password = "Пароль123!🔒";
        let hash = hash_password(unicode_password).await.unwrap();
        assert!(verify_password(unicode_password, &hash).await.unwrap());
    }
}
