//! Certificate lifecycle management
//!
//! Provides certificate storage, validation, and expiration tracking.

use crate::tls::error::{TlsError, TlsResult};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, info};

/// Certificate status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateStatus {
    /// Certificate is valid
    Valid,
    /// Certificate will expire soon (within renewal threshold)
    ExpiringSoon,
    /// Certificate has expired
    Expired,
    /// Certificate is not yet valid
    NotYetValid,
}

/// Certificate information
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    /// Certificate subject (Common Name)
    pub subject: String,
    /// Certificate issuer
    pub issuer: String,
    /// Valid from timestamp
    pub valid_from: SystemTime,
    /// Valid until timestamp
    pub valid_until: SystemTime,
    /// Subject alternative names
    pub sans: Vec<String>,
    /// Certificate status
    pub status: CertificateStatus,
    /// Days until expiration (if valid)
    pub days_until_expiration: Option<i64>,
}

impl CertificateInfo {
    /// Check if certificate needs renewal
    pub fn needs_renewal(&self, renewal_threshold_days: u32) -> bool {
        // Check if already expired
        if matches!(self.status, CertificateStatus::Expired) {
            return true;
        }

        // Check days until expiration against threshold
        self.days_until_expiration
            .map(|days| days <= renewal_threshold_days as i64)
            .unwrap_or(true)
    }

    /// Check if certificate is valid
    pub fn is_valid(&self) -> bool {
        matches!(self.status, CertificateStatus::Valid | CertificateStatus::ExpiringSoon)
    }
}

/// Certificate manager for tracking and managing certificates
pub struct CertificateManager {
    /// Certificate storage directory
    storage_dir: PathBuf,
    /// Loaded certificates indexed by domain/identifier
    certificates: Arc<RwLock<HashMap<String, CertificateInfo>>>,
    /// Renewal threshold in days (trigger renewal when cert has less than this many days left)
    renewal_threshold_days: u32,
}

impl CertificateManager {
    /// Create a new certificate manager
    pub fn new(storage_dir: impl AsRef<Path>) -> TlsResult<Self> {
        let storage_dir = storage_dir.as_ref().to_path_buf();

        // Create storage directory if it doesn't exist
        if !storage_dir.exists() {
            std::fs::create_dir_all(&storage_dir).map_err(|e| TlsError::ConfigError {
                reason: format!("Failed to create certificate storage directory: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;
            info!(path = ?storage_dir, "Created certificate storage directory");
        }

        Ok(Self {
            storage_dir,
            certificates: Arc::new(RwLock::new(HashMap::new())),
            renewal_threshold_days: 30, // Default: renew 30 days before expiration
        })
    }

    /// Set the renewal threshold in days
    pub fn set_renewal_threshold(&mut self, days: u32) {
        self.renewal_threshold_days = days;
        info!(days = days, "Updated certificate renewal threshold");
    }

    /// Get certificate information for a domain
    pub fn get_certificate_info(&self, domain: &str) -> Option<CertificateInfo> {
        self.certificates.read().get(domain).cloned()
    }

    /// Load certificate from file and track it
    pub fn load_certificate(
        &self,
        domain: impl Into<String>,
        cert_path: impl AsRef<Path>,
    ) -> TlsResult<CertificateInfo> {
        let domain = domain.into();
        let cert_path = cert_path.as_ref();

        debug!(domain = %domain, path = ?cert_path, "Loading certificate");

        // Read certificate file
        let cert_pem =
            std::fs::read_to_string(cert_path).map_err(|e| TlsError::InvalidCertificate {
                reason: format!("Failed to read certificate from {:?}: {}", cert_path, e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        // Parse certificate to extract information
        let info = self.parse_certificate_info(&domain, &cert_pem)?;

        // Store certificate info
        self.certificates.write().insert(domain.clone(), info.clone());

        info!(
            domain = %domain,
            status = ?info.status,
            days_until_expiration = ?info.days_until_expiration,
            "Certificate loaded and tracked"
        );

        Ok(info)
    }

    /// Parse certificate PEM to extract information
    fn parse_certificate_info(&self, domain: &str, _cert_pem: &str) -> TlsResult<CertificateInfo> {
        // For now, we'll use x509-parser to extract certificate details
        // This is a simplified implementation - a production version would use a full X.509 parser

        // Create mock info for now - in production, parse actual certificate
        let now = SystemTime::now();
        let valid_from = now;
        let valid_until = now + Duration::from_secs(90 * 24 * 60 * 60); // 90 days

        let days_until_expiration =
            valid_until.duration_since(now).ok().map(|d| d.as_secs() as i64 / 86400);

        let status = if now < valid_from {
            CertificateStatus::NotYetValid
        } else if now > valid_until {
            CertificateStatus::Expired
        } else if days_until_expiration.unwrap_or(0) <= self.renewal_threshold_days as i64 {
            CertificateStatus::ExpiringSoon
        } else {
            CertificateStatus::Valid
        };

        Ok(CertificateInfo {
            subject: domain.to_string(),
            issuer: "Unknown".to_string(), // Would be parsed from certificate
            valid_from,
            valid_until,
            sans: vec![domain.to_string()],
            status,
            days_until_expiration,
        })
    }

    /// Check all tracked certificates and return those needing renewal
    pub fn check_renewals(&self) -> Vec<(String, CertificateInfo)> {
        let certs = self.certificates.read();
        certs
            .iter()
            .filter(|(_, info)| info.needs_renewal(self.renewal_threshold_days))
            .map(|(domain, info)| (domain.clone(), info.clone()))
            .collect()
    }

    /// Get path for storing certificate
    pub fn get_cert_path(&self, domain: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.crt", domain))
    }

    /// Get path for storing private key
    pub fn get_key_path(&self, domain: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.key", domain))
    }

    /// Save certificate and key to storage directory
    pub fn save_certificate(
        &self,
        domain: impl Into<String>,
        cert_pem: &str,
        key_pem: &str,
    ) -> TlsResult<()> {
        let domain = domain.into();
        let cert_path = self.get_cert_path(&domain);
        let key_path = self.get_key_path(&domain);

        // Write certificate
        std::fs::write(&cert_path, cert_pem).map_err(|e| TlsError::ConfigError {
            reason: format!("Failed to write certificate to {:?}: {}", cert_path, e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        // Write private key with restrictive permissions
        std::fs::write(&key_path, key_pem).map_err(|e| TlsError::ConfigError {
            reason: format!("Failed to write private key to {:?}: {}", key_path, e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        // Set restrictive permissions on private key (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&key_path)
                .map_err(|e| TlsError::ConfigError {
                    reason: format!("Failed to get key file metadata: {}", e),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                })?
                .permissions();
            perms.set_mode(0o600); // Read/write for owner only
            std::fs::set_permissions(&key_path, perms).map_err(|e| TlsError::ConfigError {
                reason: format!("Failed to set key file permissions: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;
        }

        // Load and track the certificate
        self.load_certificate(domain, cert_path)?;

        Ok(())
    }

    /// List all tracked certificates
    pub fn list_certificates(&self) -> Vec<(String, CertificateInfo)> {
        self.certificates
            .read()
            .iter()
            .map(|(domain, info)| (domain.clone(), info.clone()))
            .collect()
    }

    /// Remove certificate from tracking
    pub fn remove_certificate(&self, domain: &str) {
        if self.certificates.write().remove(domain).is_some() {
            info!(domain = %domain, "Certificate removed from tracking");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_certificate_manager_creation() {
        let temp_dir = env::temp_dir().join("test_cert_manager");
        let manager = CertificateManager::new(&temp_dir);
        assert!(manager.is_ok());

        // Clean up
        std::fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn test_certificate_info_needs_renewal() {
        let now = SystemTime::now();
        let info = CertificateInfo {
            subject: "test.com".to_string(),
            issuer: "Test CA".to_string(),
            valid_from: now - Duration::from_secs(86400),
            valid_until: now + Duration::from_secs(10 * 86400), // 10 days
            sans: vec!["test.com".to_string()],
            status: CertificateStatus::ExpiringSoon,
            days_until_expiration: Some(10),
        };

        assert!(info.needs_renewal(30)); // Threshold 30 days, cert expires in 10
        assert!(!info.needs_renewal(5)); // Threshold 5 days, cert expires in 10
    }

    #[test]
    fn test_certificate_info_is_valid() {
        let now = SystemTime::now();

        let valid_info = CertificateInfo {
            subject: "test.com".to_string(),
            issuer: "Test CA".to_string(),
            valid_from: now - Duration::from_secs(86400),
            valid_until: now + Duration::from_secs(90 * 86400),
            sans: vec!["test.com".to_string()],
            status: CertificateStatus::Valid,
            days_until_expiration: Some(90),
        };
        assert!(valid_info.is_valid());

        let expired_info = CertificateInfo {
            subject: "test.com".to_string(),
            issuer: "Test CA".to_string(),
            valid_from: now - Duration::from_secs(100 * 86400),
            valid_until: now - Duration::from_secs(10 * 86400),
            sans: vec!["test.com".to_string()],
            status: CertificateStatus::Expired,
            days_until_expiration: None,
        };
        assert!(!expired_info.is_valid());
    }

    #[test]
    fn test_get_cert_key_paths() {
        let temp_dir = env::temp_dir().join("test_cert_paths");
        let manager = CertificateManager::new(&temp_dir).unwrap();

        let cert_path = manager.get_cert_path("example.com");
        let key_path = manager.get_key_path("example.com");

        assert!(cert_path.to_string_lossy().contains("example.com.crt"));
        assert!(key_path.to_string_lossy().contains("example.com.key"));

        // Clean up
        std::fs::remove_dir_all(temp_dir).ok();
    }
}
