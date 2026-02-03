//! Backup codes for MFA recovery.
//!
//! Provides one-time use backup codes for account recovery when TOTP is unavailable.

use crate::error::AuthError;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(feature = "backtrace")]
use std::backtrace::Backtrace;
use tracing::{debug, info};

/// Number of backup codes to generate.
pub const BACKUP_CODE_COUNT: usize = 10;

/// Length of each backup code (8 characters).
pub const BACKUP_CODE_LENGTH: usize = 8;

/// Backup code (one-time use).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCode {
    /// Hashed code (SHA-256)
    pub hash: String,
    /// Whether this code has been used
    pub used: bool,
}

impl BackupCode {
    /// Create a new backup code from plaintext.
    #[must_use]
    pub fn new(code: &str) -> Self {
        Self { hash: hash_backup_code(code), used: false }
    }

    /// Verify a plaintext code against this backup code.
    #[must_use]
    pub fn verify(&self, code: &str) -> bool {
        if self.used {
            return false;
        }
        let code_hash = hash_backup_code(code);
        code_hash == self.hash
    }

    /// Mark this code as used.
    pub fn mark_used(&mut self) {
        self.used = true;
    }
}

/// Hash a backup code using SHA-256.
fn hash_backup_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate a random backup code (8 alphanumeric characters).
fn generate_backup_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // No ambiguous chars
    let mut rng = rand::thread_rng();

    (0..BACKUP_CODE_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Backup code manager.
pub struct BackupCodeManager;

impl BackupCodeManager {
    /// Generate a set of backup codes.
    ///
    /// Returns both the plaintext codes (to show user) and hashed codes (to store).
    ///
    /// # Returns
    ///
    /// Tuple of (`plaintext_codes`, `hashed_backup_codes`)
    pub fn generate_codes() -> (Vec<String>, Vec<BackupCode>) {
        let mut plaintext_codes = Vec::with_capacity(BACKUP_CODE_COUNT);
        let mut backup_codes = Vec::with_capacity(BACKUP_CODE_COUNT);

        for _ in 0..BACKUP_CODE_COUNT {
            let code = generate_backup_code();
            backup_codes.push(BackupCode::new(&code));
            plaintext_codes.push(code);
        }

        info!("Generated {} backup codes", BACKUP_CODE_COUNT);

        (plaintext_codes, backup_codes)
    }

    /// Verify a backup code and mark it as used.
    ///
    /// # Arguments
    ///
    /// * `backup_codes` - Mutable reference to stored backup codes
    /// * `code` - Plaintext code to verify
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::BackupCodeVerificationFailed`] if code is invalid or already used.
    pub fn verify_and_use(backup_codes: &mut [BackupCode], code: &str) -> Result<(), AuthError> {
        // Find matching code
        for backup_code in backup_codes.iter_mut() {
            if backup_code.verify(code) {
                backup_code.mark_used();
                debug!("Backup code verified and marked as used");
                return Ok(());
            }
        }

        Err(AuthError::BackupCodeVerificationFailed {
            #[cfg(feature = "backtrace")]
            backtrace: Backtrace::capture(),
        })
    }

    /// Check how many unused backup codes remain.
    #[must_use]
    pub fn unused_count(backup_codes: &[BackupCode]) -> usize {
        backup_codes.iter().filter(|c| !c.used).count()
    }

    /// Check if backup codes are running low (< 3 remaining).
    #[must_use]
    pub fn is_low(backup_codes: &[BackupCode]) -> bool {
        Self::unused_count(backup_codes) < 3
    }

    /// Format backup codes for display (grouped with dashes).
    ///
    /// Example: ABCD-EFGH
    #[must_use]
    pub fn format_codes_for_display(codes: &[String]) -> Vec<String> {
        codes
            .iter()
            .map(|code| {
                if code.len() == BACKUP_CODE_LENGTH {
                    format!(
                        "{}-{}",
                        &code[0..BACKUP_CODE_LENGTH / 2],
                        &code[BACKUP_CODE_LENGTH / 2..]
                    )
                } else {
                    code.clone()
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_backup_code() {
        let code = generate_backup_code();
        assert_eq!(code.len(), BACKUP_CODE_LENGTH);
        assert!(code.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_generate_codes() {
        let (plaintext, hashed) = BackupCodeManager::generate_codes();
        assert_eq!(plaintext.len(), BACKUP_CODE_COUNT);
        assert_eq!(hashed.len(), BACKUP_CODE_COUNT);

        // All codes should be different
        for i in 0..plaintext.len() {
            for j in (i + 1)..plaintext.len() {
                assert_ne!(plaintext[i], plaintext[j]);
            }
        }
    }

    #[test]
    fn test_backup_code_verify() {
        let code = "TESTCODE";
        let backup_code = BackupCode::new(code);

        assert!(backup_code.verify(code));
        assert!(!backup_code.verify("WRONGCODE"));
    }

    #[test]
    fn test_backup_code_used() {
        let code = "TESTCODE";
        let mut backup_code = BackupCode::new(code);

        assert!(backup_code.verify(code));

        backup_code.mark_used();
        assert!(!backup_code.verify(code)); // Should fail after marking used
    }

    #[test]
    fn test_verify_and_use() {
        let (plaintext, mut hashed) = BackupCodeManager::generate_codes();

        // Verify first code
        let result = BackupCodeManager::verify_and_use(&mut hashed, &plaintext[0]);
        assert!(result.is_ok());

        // Try to use same code again - should fail
        let result = BackupCodeManager::verify_and_use(&mut hashed, &plaintext[0]);
        assert!(result.is_err());

        // Verify second code - should still work
        let result = BackupCodeManager::verify_and_use(&mut hashed, &plaintext[1]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_invalid_code() {
        let (_plaintext, mut hashed) = BackupCodeManager::generate_codes();

        let result = BackupCodeManager::verify_and_use(&mut hashed, "INVALIDCODE");
        assert!(result.is_err());
    }

    #[test]
    fn test_unused_count() {
        let (_plaintext, mut hashed) = BackupCodeManager::generate_codes();

        assert_eq!(BackupCodeManager::unused_count(&hashed), BACKUP_CODE_COUNT);

        // Use one code
        hashed[0].mark_used();
        assert_eq!(BackupCodeManager::unused_count(&hashed), BACKUP_CODE_COUNT - 1);

        // Use another
        hashed[1].mark_used();
        assert_eq!(BackupCodeManager::unused_count(&hashed), BACKUP_CODE_COUNT - 2);
    }

    #[test]
    fn test_is_low() {
        let (_plaintext, mut hashed) = BackupCodeManager::generate_codes();

        assert!(!BackupCodeManager::is_low(&hashed));

        // Use most codes
        for code in hashed.iter_mut().take(BACKUP_CODE_COUNT - 2) {
            code.mark_used();
        }

        assert!(BackupCodeManager::is_low(&hashed)); // Only 2 left
    }

    #[test]
    fn test_format_codes_for_display() {
        let codes = vec!["ABCDEFGH".to_string(), "12345678".to_string()];
        let formatted = BackupCodeManager::format_codes_for_display(&codes);

        assert_eq!(formatted[0], "ABCD-EFGH");
        assert_eq!(formatted[1], "1234-5678");
    }

    #[test]
    fn test_hash_consistency() {
        let code = "TESTCODE";
        let hash1 = hash_backup_code(code);
        let hash2 = hash_backup_code(code);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_codes_different_hashes() {
        let hash1 = hash_backup_code("CODE1234");
        let hash2 = hash_backup_code("CODE5678");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_all_codes_unused_initially() {
        let (_plaintext, hashed) = BackupCodeManager::generate_codes();
        assert!(hashed.iter().all(|c| !c.used));
    }
}
