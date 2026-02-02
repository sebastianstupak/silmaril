//! TLS configuration
//!
//! Provides configuration structures for TLS/DTLS with strong security defaults.

use super::error::{TlsError, TlsResult};
use rustls::{Certificate, ClientConfig, PrivateKey, RootCertStore, ServerConfig};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// TLS protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    /// TLS 1.3 only (most secure)
    Tls13Only,
}

/// Cipher suite selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherSuiteSelection {
    /// Strong ciphers only (AES-GCM, ChaCha20-Poly1305)
    Strong,
    /// All supported ciphers (not recommended for production)
    All,
}

/// Certificate verification mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateVerification {
    /// Full verification with system roots
    Full,
    /// Verify with custom roots only
    CustomRoots,
    /// Disable verification (INSECURE - dev only)
    Disabled,
}

/// TLS client configuration builder
#[derive(Debug, Clone)]
pub struct TlsClientConfigBuilder {
    version: TlsVersion,
    cipher_suites: CipherSuiteSelection,
    verification: CertificateVerification,
    custom_roots: Vec<PathBuf>,
    client_cert: Option<PathBuf>,
    client_key: Option<PathBuf>,
    enable_sni: bool,
    enable_session_resumption: bool,
    enable_0rtt: bool,
}

impl Default for TlsClientConfigBuilder {
    fn default() -> Self {
        Self {
            version: TlsVersion::Tls13Only,
            cipher_suites: CipherSuiteSelection::Strong,
            verification: CertificateVerification::Full,
            custom_roots: Vec::new(),
            client_cert: None,
            client_key: None,
            enable_sni: true,
            enable_session_resumption: true,
            enable_0rtt: false,
        }
    }
}

impl TlsClientConfigBuilder {
    /// Create a new TLS client configuration builder with secure defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set TLS protocol version
    pub fn version(mut self, version: TlsVersion) -> Self {
        self.version = version;
        self
    }

    /// Set cipher suite selection
    pub fn cipher_suites(mut self, selection: CipherSuiteSelection) -> Self {
        self.cipher_suites = selection;
        self
    }

    /// Set certificate verification mode
    pub fn verification(mut self, verification: CertificateVerification) -> Self {
        self.verification = verification;
        self
    }

    /// Add custom root certificate
    pub fn add_root_certificate<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.custom_roots.push(path.into());
        self
    }

    /// Set client certificate for mutual TLS
    pub fn client_certificate<P: Into<PathBuf>>(mut self, cert_path: P, key_path: P) -> Self {
        self.client_cert = Some(cert_path.into());
        self.client_key = Some(key_path.into());
        self
    }

    /// Enable or disable SNI (Server Name Indication)
    pub fn enable_sni(mut self, enable: bool) -> Self {
        self.enable_sni = enable;
        self
    }

    /// Enable or disable session resumption
    pub fn enable_session_resumption(mut self, enable: bool) -> Self {
        self.enable_session_resumption = enable;
        self
    }

    /// Enable or disable 0-RTT (requires session resumption)
    pub fn enable_0rtt(mut self, enable: bool) -> Self {
        self.enable_0rtt = enable;
        self
    }

    /// Build the TLS client configuration
    pub fn build(self) -> TlsResult<Arc<ClientConfig>> {
        info!(
            version = ?self.version,
            cipher_suites = ?self.cipher_suites,
            verification = ?self.verification,
            "Building TLS client configuration"
        );

        // Create root certificate store
        let mut root_store = RootCertStore::empty();

        match self.verification {
            CertificateVerification::Full => {
                // Load system root certificates
                root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
                    rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                        ta.subject,
                        ta.spki,
                        ta.name_constraints,
                    )
                }));
                debug!("Loaded system root certificates");
            }
            CertificateVerification::CustomRoots => {
                debug!("Using custom roots only");
            }
            CertificateVerification::Disabled => {
                warn!("Certificate verification DISABLED - INSECURE!");
            }
        }

        // Load custom root certificates
        for root_path in &self.custom_roots {
            let certs = load_certificates(root_path)?;
            for cert in certs {
                root_store.add(&cert).map_err(|e| TlsError::InvalidCertificate {
                    reason: format!("Failed to add root certificate: {:?}", e),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                })?;
            }
            debug!(path = ?root_path, "Loaded custom root certificate");
        }

        // Build configuration
        let config = ClientConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| TlsError::ConfigError {
                reason: format!("Failed to set protocol version: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?
            .with_root_certificates(root_store);

        // Configure client authentication (mutual TLS)
        let config =
            if let (Some(cert_path), Some(key_path)) = (&self.client_cert, &self.client_key) {
                let certs = load_certificates(cert_path)?;
                let key = load_private_key(key_path)?;
                config.with_client_auth_cert(certs, key).map_err(|e| {
                    TlsError::InvalidCertificate {
                        reason: format!("Failed to set client certificate: {}", e),
                        #[cfg(feature = "backtrace")]
                        backtrace: std::backtrace::Backtrace::capture(),
                    }
                })?
            } else {
                config.with_no_client_auth()
            };

        // Enable session resumption
        if !self.enable_session_resumption {
            // Note: rustls 0.21 doesn't have a direct way to disable session resumption
            // It's enabled by default, which is what we want for performance
            debug!("Session resumption is enabled by default in rustls 0.21");
        }

        // 0-RTT is handled at connection time in TLS 1.3
        if self.enable_0rtt {
            debug!("0-RTT will be used when session tickets are available");
        }

        info!("TLS client configuration built successfully");
        Ok(Arc::new(config))
    }
}

/// TLS server configuration builder
#[derive(Debug, Clone)]
pub struct TlsServerConfigBuilder {
    version: TlsVersion,
    cipher_suites: CipherSuiteSelection,
    cert_path: Option<PathBuf>,
    key_path: Option<PathBuf>,
    require_client_auth: bool,
    client_ca_path: Option<PathBuf>,
    enable_session_resumption: bool,
    enable_0rtt: bool,
}

impl Default for TlsServerConfigBuilder {
    fn default() -> Self {
        Self {
            version: TlsVersion::Tls13Only,
            cipher_suites: CipherSuiteSelection::Strong,
            cert_path: None,
            key_path: None,
            require_client_auth: false,
            client_ca_path: None,
            enable_session_resumption: true,
            enable_0rtt: false,
        }
    }
}

impl TlsServerConfigBuilder {
    /// Create a new TLS server configuration builder with secure defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set TLS protocol version
    pub fn version(mut self, version: TlsVersion) -> Self {
        self.version = version;
        self
    }

    /// Set cipher suite selection
    pub fn cipher_suites(mut self, selection: CipherSuiteSelection) -> Self {
        self.cipher_suites = selection;
        self
    }

    /// Set server certificate and private key
    pub fn certificate<P: Into<PathBuf>>(mut self, cert_path: P, key_path: P) -> Self {
        self.cert_path = Some(cert_path.into());
        self.key_path = Some(key_path.into());
        self
    }

    /// Require client authentication (mutual TLS)
    pub fn require_client_auth<P: Into<PathBuf>>(mut self, ca_path: P) -> Self {
        self.require_client_auth = true;
        self.client_ca_path = Some(ca_path.into());
        self
    }

    /// Enable or disable session resumption
    pub fn enable_session_resumption(mut self, enable: bool) -> Self {
        self.enable_session_resumption = enable;
        self
    }

    /// Enable or disable 0-RTT (requires session resumption)
    pub fn enable_0rtt(mut self, enable: bool) -> Self {
        self.enable_0rtt = enable;
        self
    }

    /// Build the TLS server configuration
    pub fn build(self) -> TlsResult<Arc<ServerConfig>> {
        info!(
            version = ?self.version,
            cipher_suites = ?self.cipher_suites,
            require_client_auth = self.require_client_auth,
            "Building TLS server configuration"
        );

        // Load server certificate and key
        let cert_path = self.cert_path.ok_or_else(|| TlsError::ConfigError {
            reason: "Server certificate path not set".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;
        let key_path = self.key_path.ok_or_else(|| TlsError::ConfigError {
            reason: "Server private key path not set".to_string(),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        let certs = load_certificates(&cert_path)?;
        let key = load_private_key(&key_path)?;

        // Build base configuration
        let config = ServerConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| TlsError::ConfigError {
                reason: format!("Failed to set protocol version: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        // Configure client authentication if required
        let config = if self.require_client_auth {
            let ca_path = self.client_ca_path.ok_or_else(|| TlsError::ConfigError {
                reason: "Client CA path not set but client auth is required".to_string(),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

            let mut client_root_store = RootCertStore::empty();
            let ca_certs = load_certificates(&ca_path)?;
            for cert in ca_certs {
                client_root_store.add(&cert).map_err(|e| TlsError::InvalidCertificate {
                    reason: format!("Failed to add client CA certificate: {:?}", e),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                })?;
            }

            let verifier = rustls::server::AllowAnyAuthenticatedClient::new(client_root_store);
            config.with_client_cert_verifier(Arc::new(verifier))
        } else {
            config.with_no_client_auth()
        };

        let config =
            config.with_single_cert(certs, key).map_err(|e| TlsError::InvalidCertificate {
                reason: format!("Failed to set server certificate: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        // Configure session resumption and 0-RTT
        if self.enable_session_resumption {
            // Session resumption is enabled by default in rustls 0.21
            debug!("Session resumption enabled");
        }

        if self.enable_0rtt {
            // 0-RTT requires additional configuration in rustls
            debug!("0-RTT configuration requested (implementation depends on rustls version)");
        }

        info!("TLS server configuration built successfully");
        Ok(Arc::new(config))
    }
}

/// Load certificates from PEM file
fn load_certificates(path: &Path) -> TlsResult<Vec<Certificate>> {
    let file = std::fs::File::open(path).map_err(|e| TlsError::InvalidCertificate {
        reason: format!("Failed to open certificate file {:?}: {}", path, e),
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace::capture(),
    })?;

    let mut reader = std::io::BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader)
        .map_err(|e| TlsError::InvalidCertificate {
            reason: format!("Failed to parse certificate file {:?}: {}", path, e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?
        .into_iter()
        .map(Certificate)
        .collect();

    Ok(certs)
}

/// Load private key from PEM file
fn load_private_key(path: &Path) -> TlsResult<PrivateKey> {
    let file = std::fs::File::open(path).map_err(|e| TlsError::InvalidCertificate {
        reason: format!("Failed to open private key file {:?}: {}", path, e),
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace::capture(),
    })?;

    let mut reader = std::io::BufReader::new(file);

    // Try parsing as PKCS#8 first
    if let Ok(keys) = rustls_pemfile::pkcs8_private_keys(&mut reader) {
        if !keys.is_empty() {
            return Ok(PrivateKey(keys[0].clone()));
        }
    }

    // Reset reader and try RSA private key format
    let file = std::fs::File::open(path).map_err(|e| TlsError::InvalidCertificate {
        reason: format!("Failed to reopen private key file {:?}: {}", path, e),
        #[cfg(feature = "backtrace")]
        backtrace: std::backtrace::Backtrace::capture(),
    })?;
    let mut reader = std::io::BufReader::new(file);

    let keys = rustls_pemfile::rsa_private_keys(&mut reader).map_err(|e| {
        TlsError::InvalidCertificate {
            reason: format!("Failed to parse private key file {:?}: {}", path, e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    })?;

    if keys.is_empty() {
        return Err(TlsError::InvalidCertificate {
            reason: format!("No private key found in file {:?}", path),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        });
    }

    Ok(PrivateKey(keys[0].clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_builder_defaults() {
        let builder = TlsClientConfigBuilder::new();
        assert_eq!(builder.version, TlsVersion::Tls13Only);
        assert_eq!(builder.cipher_suites, CipherSuiteSelection::Strong);
        assert_eq!(builder.verification, CertificateVerification::Full);
        assert!(builder.enable_sni);
        assert!(builder.enable_session_resumption);
        assert!(!builder.enable_0rtt);
    }

    #[test]
    fn test_server_config_builder_defaults() {
        let builder = TlsServerConfigBuilder::new();
        assert_eq!(builder.version, TlsVersion::Tls13Only);
        assert_eq!(builder.cipher_suites, CipherSuiteSelection::Strong);
        assert!(!builder.require_client_auth);
        assert!(builder.enable_session_resumption);
        assert!(!builder.enable_0rtt);
    }

    #[test]
    fn test_client_config_builder_customization() {
        let builder = TlsClientConfigBuilder::new()
            .version(TlsVersion::Tls13Only)
            .cipher_suites(CipherSuiteSelection::Strong)
            .verification(CertificateVerification::Disabled)
            .enable_sni(false)
            .enable_0rtt(true);

        assert_eq!(builder.version, TlsVersion::Tls13Only);
        assert_eq!(builder.verification, CertificateVerification::Disabled);
        assert!(!builder.enable_sni);
        assert!(builder.enable_0rtt);
    }
}
