//! File verification using SHA-256 hashing and Ed25519 signatures.

use crate::error::UpdateError;
use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use tracing::{debug, trace};

/// Compute SHA-256 hash of a file.
pub fn compute_file_hash<P: AsRef<Path>>(path: P) -> Result<String, UpdateError> {
    let path = path.as_ref();
    trace!(path = %path.display(), "Computing file hash");

    let file = File::open(path).map_err(|e: std::io::Error| {
        UpdateError::ioerror(path.display().to_string(), e.to_string())
    })?;

    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = reader.read(&mut buffer).map_err(|e: std::io::Error| {
            UpdateError::ioerror(path.display().to_string(), e.to_string())
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let hash = hasher.finalize();
    let hash_string = hex::encode(hash);

    debug!(
        path = %path.display(),
        hash = %hash_string,
        "File hash computed"
    );

    Ok(hash_string)
}

/// Verify that a file matches the expected SHA-256 hash.
pub fn verify_file_hash<P: AsRef<Path>>(path: P, expected_hash: &str) -> Result<(), UpdateError> {
    let path = path.as_ref();
    let actual_hash = compute_file_hash(path)?;

    if actual_hash.to_lowercase() != expected_hash.to_lowercase() {
        return Err(UpdateError::verificationfailed(
            path.display().to_string(),
            format!("Hash mismatch: expected {}, got {}", expected_hash, actual_hash),
        ));
    }

    debug!(
        path = %path.display(),
        hash = %expected_hash,
        "File hash verified"
    );

    Ok(())
}

/// Verify Ed25519 signature of data.
pub fn verify_signature(data: &[u8], signature: &str, public_key: &str) -> Result<(), UpdateError> {
    let signature_bytes = hex::decode(signature).map_err(|e| {
        UpdateError::signatureverificationfailed(format!("Invalid signature hex: {}", e))
    })?;

    let public_key_bytes = hex::decode(public_key).map_err(|e| {
        UpdateError::signatureverificationfailed(format!("Invalid public key hex: {}", e))
    })?;

    let verifying_key =
        VerifyingKey::from_bytes(&public_key_bytes.as_slice().try_into().map_err(|_| {
            UpdateError::signatureverificationfailed("Invalid public key length".to_string())
        })?)
        .map_err(|e| {
            UpdateError::signatureverificationfailed(format!("Invalid public key: {}", e))
        })?;

    let signature =
        Signature::from_bytes(&signature_bytes.as_slice().try_into().map_err(|_| {
            UpdateError::signatureverificationfailed("Invalid signature length".to_string())
        })?);

    verifying_key.verify(data, &signature).map_err(|e| {
        UpdateError::signatureverificationfailed(format!("Signature verification failed: {}", e))
    })?;

    debug!("Signature verified successfully");
    Ok(())
}

/// Compute SHA-256 hash of bytes in memory.
pub fn compute_bytes_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_file_hash() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello, world!").unwrap();
        temp_file.flush().unwrap();

        let hash = compute_file_hash(temp_file.path()).unwrap();
        // SHA-256 of "Hello, world!"
        assert_eq!(hash, "315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3");
    }

    #[test]
    fn test_verify_file_hash_success() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello, world!").unwrap();
        temp_file.flush().unwrap();

        let result = verify_file_hash(
            temp_file.path(),
            "315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3",
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_file_hash_failure() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello, world!").unwrap();
        temp_file.flush().unwrap();

        let result = verify_file_hash(temp_file.path(), "invalid_hash");
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_bytes_hash() {
        let hash = compute_bytes_hash(b"Hello, world!");
        assert_eq!(hash, "315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3");
    }

    #[test]
    fn test_signature_verification() {
        // This is a test key pair generated for testing purposes only
        // In production, use a secure key management system
        use ed25519_dalek::{Signer, SigningKey};

        let signing_key = SigningKey::from_bytes(&[
            157, 097, 177, 157, 239, 253, 090, 096, 186, 132, 074, 244, 146, 236, 044, 196, 068,
            073, 197, 105, 123, 050, 105, 025, 112, 059, 172, 003, 028, 174, 127, 096,
        ]);

        let message = b"Test message";
        let signature = signing_key.sign(message);

        let public_key = hex::encode(signing_key.verifying_key().to_bytes());
        let signature_hex = hex::encode(signature.to_bytes());

        let result = verify_signature(message, &signature_hex, &public_key);
        assert!(result.is_ok());
    }

    #[test]
    fn test_signature_verification_failure() {
        let result = verify_signature(b"Test message", &"0".repeat(128), &"0".repeat(64));
        assert!(result.is_err());
    }
}
