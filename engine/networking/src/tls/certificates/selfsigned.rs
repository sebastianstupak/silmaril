//! Self-signed certificate generation
//!
//! Provides utilities for generating self-signed certificates for development and testing.

use crate::tls::error::{TlsError, TlsResult};
use rcgen::{Certificate, CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use std::path::Path;
use tracing::{debug, info, warn};

/// Self-signed certificate configuration
#[derive(Debug, Clone)]
pub struct SelfSignedConfig {
    /// Common name (CN) for the certificate
    pub common_name: String,
    /// Subject alternative names (SANs)
    pub sans: Vec<String>,
    /// Organization name
    pub organization: Option<String>,
    /// Organizational unit
    pub organizational_unit: Option<String>,
    /// Country code (2 letters)
    pub country: Option<String>,
    /// State or province
    pub state_province: Option<String>,
    /// Locality or city
    pub locality: Option<String>,
    /// Validity period in days
    pub validity_days: u32,
}

impl Default for SelfSignedConfig {
    fn default() -> Self {
        Self {
            common_name: "localhost".to_string(),
            sans: vec!["localhost".to_string(), "127.0.0.1".to_string()],
            organization: Some("Game Engine Dev".to_string()),
            organizational_unit: Some("Development".to_string()),
            country: None,
            state_province: None,
            locality: None,
            validity_days: 365,
        }
    }
}

impl SelfSignedConfig {
    /// Create a new self-signed certificate configuration
    pub fn new(common_name: impl Into<String>) -> Self {
        Self { common_name: common_name.into(), ..Default::default() }
    }

    /// Add a subject alternative name
    pub fn add_san(mut self, san: impl Into<String>) -> Self {
        self.sans.push(san.into());
        self
    }

    /// Set organization
    pub fn organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }

    /// Set organizational unit
    pub fn organizational_unit(mut self, unit: impl Into<String>) -> Self {
        self.organizational_unit = Some(unit.into());
        self
    }

    /// Set validity period in days
    pub fn validity_days(mut self, days: u32) -> Self {
        self.validity_days = days;
        self
    }
}

/// Generate a self-signed certificate
pub fn generate_self_signed_cert(config: &SelfSignedConfig) -> TlsResult<(String, String)> {
    warn!(
        common_name = %config.common_name,
        validity_days = config.validity_days,
        "Generating self-signed certificate (NOT FOR PRODUCTION)"
    );

    // Create certificate parameters
    let mut params = CertificateParams::default();

    // Set subject distinguished name
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, &config.common_name);

    if let Some(org) = &config.organization {
        dn.push(DnType::OrganizationName, org);
    }
    if let Some(unit) = &config.organizational_unit {
        dn.push(DnType::OrganizationalUnitName, unit);
    }
    if let Some(country) = &config.country {
        dn.push(DnType::CountryName, country);
    }
    if let Some(state) = &config.state_province {
        dn.push(DnType::StateOrProvinceName, state);
    }
    if let Some(locality) = &config.locality {
        dn.push(DnType::LocalityName, locality);
    }

    params.distinguished_name = dn;

    // Set subject alternative names
    params.subject_alt_names = config
        .sans
        .iter()
        .map(|san| {
            // Try to parse as IP address first
            if let Ok(ip) = san.parse() {
                SanType::IpAddress(ip)
            } else {
                SanType::DnsName(san.clone())
            }
        })
        .collect();

    // Set validity period
    params.not_before = chrono::Utc::now() - chrono::Duration::days(1);
    params.not_after = chrono::Utc::now() + chrono::Duration::days(config.validity_days as i64);

    // Generate certificate
    let cert = Certificate::from_params(params).map_err(|e| TlsError::InvalidCertificate {
        reason: format!("Failed to generate certificate: {}", e),
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace::capture(),
    })?;

    let cert_pem = cert.serialize_pem().map_err(|e| TlsError::InvalidCertificate {
        reason: format!("Failed to serialize certificate: {}", e),
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace::capture(),
    })?;

    let key_pem = cert.serialize_private_key_pem();

    info!(
        common_name = %config.common_name,
        sans = ?config.sans,
        "Self-signed certificate generated successfully"
    );

    Ok((cert_pem, key_pem))
}

/// Generate and save a self-signed certificate to files
pub fn generate_and_save_self_signed_cert(
    config: &SelfSignedConfig,
    cert_path: impl AsRef<Path>,
    key_path: impl AsRef<Path>,
) -> TlsResult<()> {
    let (cert_pem, key_pem) = generate_self_signed_cert(config)?;

    // Write certificate to file
    std::fs::write(cert_path.as_ref(), cert_pem.as_bytes()).map_err(|e| TlsError::ConfigError {
        reason: format!("Failed to write certificate to {:?}: {}", cert_path.as_ref(), e),
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace::capture(),
    })?;

    // Write private key to file with restricted permissions
    std::fs::write(key_path.as_ref(), key_pem.as_bytes()).map_err(|e| TlsError::ConfigError {
        reason: format!("Failed to write private key to {:?}: {}", key_path.as_ref(), e),
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace::capture(),
    })?;

    // Set restrictive permissions on private key (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(key_path.as_ref())
            .map_err(|e| TlsError::ConfigError {
                reason: format!("Failed to get key file metadata: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?
            .permissions();
        perms.set_mode(0o600); // Read/write for owner only
        std::fs::set_permissions(key_path.as_ref(), perms).map_err(|e| TlsError::ConfigError {
            reason: format!("Failed to set key file permissions: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;
        debug!(path = ?key_path.as_ref(), "Set restrictive permissions on private key");
    }

    info!(
        cert_path = ?cert_path.as_ref(),
        key_path = ?key_path.as_ref(),
        "Self-signed certificate saved to files"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_self_signed_cert() {
        let config = SelfSignedConfig::new("test.local")
            .add_san("test.local")
            .add_san("127.0.0.1")
            .organization("Test Org")
            .validity_days(30);

        let result = generate_self_signed_cert(&config);
        assert!(result.is_ok());

        let (cert_pem, key_pem) = result.unwrap();
        assert!(cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_default_config() {
        let config = SelfSignedConfig::default();
        assert_eq!(config.common_name, "localhost");
        assert_eq!(config.validity_days, 365);
        assert!(config.sans.contains(&"localhost".to_string()));
        assert!(config.sans.contains(&"127.0.0.1".to_string()));
    }

    #[test]
    fn test_config_builder() {
        let config = SelfSignedConfig::new("example.com")
            .add_san("www.example.com")
            .add_san("192.168.1.1")
            .organization("Example Corp")
            .organizational_unit("IT")
            .validity_days(90);

        assert_eq!(config.common_name, "example.com");
        assert_eq!(config.organization, Some("Example Corp".to_string()));
        assert_eq!(config.organizational_unit, Some("IT".to_string()));
        assert_eq!(config.validity_days, 90);
        assert_eq!(config.sans.len(), 3);
    }

    #[test]
    fn test_generate_and_save() {
        let temp_dir = std::env::temp_dir();
        let cert_path = temp_dir.join("test_cert.pem");
        let key_path = temp_dir.join("test_key.pem");

        let config = SelfSignedConfig::new("test.local");
        let result = generate_and_save_self_signed_cert(&config, &cert_path, &key_path);
        assert!(result.is_ok());

        // Verify files exist
        assert!(cert_path.exists());
        assert!(key_path.exists());

        // Verify content
        let cert_content = std::fs::read_to_string(&cert_path).unwrap();
        let key_content = std::fs::read_to_string(&key_path).unwrap();
        assert!(cert_content.contains("BEGIN CERTIFICATE"));
        assert!(key_content.contains("BEGIN PRIVATE KEY"));

        // Clean up
        std::fs::remove_file(cert_path).ok();
        std::fs::remove_file(key_path).ok();
    }
}
