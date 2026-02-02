//! ACME protocol client for Let's Encrypt
//!
//! Provides automated certificate acquisition and renewal using the ACME protocol.

use crate::tls::certificates::manager::CertificateManager;
use crate::tls::error::{TlsError, TlsResult};
use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt,
    NewAccount, NewOrder, OrderStatus,
};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// ACME client configuration
#[derive(Debug, Clone)]
pub struct AcmeConfig {
    /// Contact email for Let's Encrypt notifications
    pub email: String,
    /// Directory for storing account credentials
    pub account_dir: PathBuf,
    /// Use Let's Encrypt staging environment (for testing)
    pub use_staging: bool,
}

impl AcmeConfig {
    /// Create a new ACME configuration
    pub fn new(email: impl Into<String>, account_dir: impl AsRef<Path>) -> Self {
        Self {
            email: email.into(),
            account_dir: account_dir.as_ref().to_path_buf(),
            use_staging: false,
        }
    }

    /// Use Let's Encrypt staging environment (for testing)
    pub fn use_staging(mut self) -> Self {
        self.use_staging = true;
        self
    }
}

/// ACME client for automated certificate management
pub struct AcmeClient {
    config: AcmeConfig,
    account: Option<Account>,
}

impl AcmeClient {
    /// Create a new ACME client
    pub fn new(config: AcmeConfig) -> Self {
        Self { config, account: None }
    }

    /// Initialize the ACME account (creates new account if needed)
    pub async fn initialize(&mut self) -> TlsResult<()> {
        info!(
            email = %self.config.email,
            staging = self.config.use_staging,
            "Initializing ACME client"
        );

        // Create account directory if it doesn't exist
        if !self.config.account_dir.exists() {
            std::fs::create_dir_all(&self.config.account_dir).map_err(|e| TlsError::Acme {
                reason: format!("Failed to create account directory: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;
        }

        let account_path = self.config.account_dir.join("account.json");

        // Try to load existing account
        let account = if account_path.exists() {
            info!("Loading existing ACME account");
            let credentials_json =
                std::fs::read_to_string(&account_path).map_err(|e| TlsError::Acme {
                    reason: format!("Failed to read account credentials: {}", e),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                })?;

            let credentials: AccountCredentials =
                serde_json::from_str(&credentials_json).map_err(|e| TlsError::Acme {
                    reason: format!("Failed to parse account credentials: {}", e),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                })?;

            Account::from_credentials(credentials).map_err(|e| TlsError::Acme {
                reason: format!("Failed to restore account from credentials: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?
        } else {
            info!("Creating new ACME account");

            // Create new account
            let url = if self.config.use_staging {
                LetsEncrypt::Staging.url()
            } else {
                LetsEncrypt::Production.url()
            };

            let new_account = Account::create(
                &NewAccount {
                    contact: &[&format!("mailto:{}", self.config.email)],
                    terms_of_service_agreed: true,
                    only_return_existing: false,
                },
                url,
                None,
            )
            .await
            .map_err(|e| TlsError::Acme {
                reason: format!("Failed to create ACME account: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

            // Save account credentials (get from account)
            let credentials = new_account.credentials();
            let credentials_json =
                serde_json::to_string_pretty(&credentials).map_err(|e| TlsError::Acme {
                    reason: format!("Failed to serialize account credentials: {}", e),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                })?;

            std::fs::write(&account_path, credentials_json).map_err(|e| TlsError::Acme {
                reason: format!("Failed to save account credentials: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

            info!(path = ?account_path, "ACME account credentials saved");

            new_account
        };

        self.account = Some(account);
        info!("ACME client initialized successfully");
        Ok(())
    }

    /// Request a certificate for the given domain
    pub async fn request_certificate(
        &mut self,
        domain: &str,
        cert_manager: &CertificateManager,
    ) -> TlsResult<()> {
        if self.account.is_none() {
            return Err(TlsError::Acme {
                reason: "ACME client not initialized. Call initialize() first.".to_string(),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }

        info!(domain = %domain, "Requesting certificate from Let's Encrypt");

        let account = self.account.as_ref().unwrap();

        // Create new order
        let identifier = Identifier::Dns(domain.to_string());
        let mut order =
            account.new_order(&NewOrder { identifiers: &[identifier] }).await.map_err(|e| {
                TlsError::Acme {
                    reason: format!("Failed to create order: {}", e),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                }
            })?;

        debug!(domain = %domain, "Order created, processing authorizations");

        // Process authorizations
        let authorizations = order.authorizations().await.map_err(|e| TlsError::Acme {
            reason: format!("Failed to get authorizations: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        for authz in authorizations {
            match authz.status {
                AuthorizationStatus::Pending => {
                    // Find HTTP-01 challenge
                    let challenge = authz
                        .challenges
                        .iter()
                        .find(|c| c.r#type == ChallengeType::Http01)
                        .ok_or_else(|| TlsError::Acme {
                            reason: "No HTTP-01 challenge found".to_string(),
                            #[cfg(feature = "backtrace")]
                            backtrace: std::backtrace::Backtrace::capture(),
                        })?;

                    // Get challenge token and key authorization
                    let token = &challenge.token;
                    let _key_auth = order.key_authorization(challenge);

                    info!(
                        domain = %domain,
                        token = %token,
                        "HTTP-01 challenge: Place key authorization at http://{}/.well-known/acme-challenge/{}",
                        domain, token
                    );

                    // In production, this would:
                    // 1. Write key_auth to a file accessible via HTTP at /.well-known/acme-challenge/{token}
                    // 2. Ensure web server is configured to serve this file
                    // For now, we document the requirement
                    warn!(
                        "ACME HTTP-01 challenge requires web server configuration. \
                         See documentation for setup instructions."
                    );

                    // Tell ACME server we're ready for validation
                    order.set_challenge_ready(&challenge.url).await.map_err(|e| {
                        TlsError::Acme {
                            reason: format!("Failed to set challenge ready: {}", e),
                            #[cfg(feature = "backtrace")]
                            backtrace: std::backtrace::Backtrace::capture(),
                        }
                    })?;

                    debug!("Challenge ready, waiting for validation");

                    // Wait for validation (with timeout)
                    let mut attempts = 0;
                    loop {
                        sleep(Duration::from_secs(2)).await;
                        attempts += 1;

                        if attempts > 30 {
                            // 60 seconds timeout
                            return Err(TlsError::Acme {
                                reason: "Challenge validation timeout".to_string(),
                                #[cfg(feature = "backtrace")]
                                backtrace: std::backtrace::Backtrace::capture(),
                            });
                        }

                        let _state = order.refresh().await.map_err(|e| TlsError::Acme {
                            reason: format!("Failed to refresh order: {}", e),
                            #[cfg(feature = "backtrace")]
                            backtrace: std::backtrace::Backtrace::capture(),
                        })?;

                        let authz = order.authorizations().await.map_err(|e| TlsError::Acme {
                            reason: format!("Failed to get authorizations: {}", e),
                            #[cfg(feature = "backtrace")]
                            backtrace: std::backtrace::Backtrace::capture(),
                        })?;

                        if let Some(authz) = authz.first() {
                            match authz.status {
                                AuthorizationStatus::Valid => {
                                    info!("Challenge validated successfully");
                                    break;
                                }
                                AuthorizationStatus::Invalid => {
                                    return Err(TlsError::Acme {
                                        reason: "Challenge validation failed".to_string(),
                                        #[cfg(feature = "backtrace")]
                                        backtrace: std::backtrace::Backtrace::capture(),
                                    });
                                }
                                _ => {
                                    debug!(status = ?authz.status, "Challenge still pending");
                                }
                            }
                        }
                    }
                }
                AuthorizationStatus::Valid => {
                    debug!("Authorization already valid");
                }
                _ => {
                    return Err(TlsError::Acme {
                        reason: format!("Unexpected authorization status: {:?}", authz.status),
                        #[cfg(feature = "backtrace")]
                        backtrace: std::backtrace::Backtrace::capture(),
                    });
                }
            }
        }

        // Generate CSR (Certificate Signing Request)
        debug!("Generating CSR");
        let mut params = rcgen::CertificateParams::new(vec![domain.to_string()]);
        params.distinguished_name = rcgen::DistinguishedName::new();
        let cert_generator =
            rcgen::Certificate::from_params(params).map_err(|e| TlsError::Acme {
                reason: format!("Failed to generate certificate params: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        let csr = cert_generator.serialize_request_der().map_err(|e| TlsError::Acme {
            reason: format!("Failed to serialize CSR: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        // Finalize order
        order.finalize(&csr).await.map_err(|e| TlsError::Acme {
            reason: format!("Failed to finalize order: {}", e),
            #[cfg(feature = "backtrace")]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        // Wait for certificate issuance
        let mut attempts = 0;
        loop {
            sleep(Duration::from_secs(1)).await;
            attempts += 1;

            if attempts > 30 {
                return Err(TlsError::Acme {
                    reason: "Certificate issuance timeout".to_string(),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }

            let state = order.refresh().await.map_err(|e| TlsError::Acme {
                reason: format!("Failed to refresh order: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

            if let OrderStatus::Valid = state.status {
                info!("Certificate issued successfully");
                break;
            } else if let OrderStatus::Invalid = state.status {
                return Err(TlsError::Acme {
                    reason: "Order became invalid".to_string(),
                    #[cfg(feature = "backtrace")]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }

            debug!(status = ?state.status, "Waiting for certificate issuance");
        }

        // Download certificate
        let cert_chain_pem = order
            .certificate()
            .await
            .map_err(|e| TlsError::Acme {
                reason: format!("Failed to download certificate: {}", e),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?
            .ok_or_else(|| TlsError::Acme {
                reason: "Certificate not available".to_string(),
                #[cfg(feature = "backtrace")]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

        let key_pem = cert_generator.serialize_private_key_pem();

        // Save certificate and key
        cert_manager.save_certificate(domain, &cert_chain_pem, &key_pem)?;

        info!(domain = %domain, "Certificate obtained and saved successfully");
        Ok(())
    }

    /// Renew certificates that are expiring soon
    pub async fn renew_certificates(
        &mut self,
        cert_manager: &CertificateManager,
    ) -> TlsResult<Vec<String>> {
        info!("Checking for certificates needing renewal");

        let renewals = cert_manager.check_renewals();
        let mut renewed = Vec::new();

        for (domain, info) in renewals {
            info!(
                domain = %domain,
                days_until_expiration = ?info.days_until_expiration,
                "Certificate needs renewal"
            );

            match self.request_certificate(&domain, cert_manager).await {
                Ok(_) => {
                    renewed.push(domain.clone());
                    info!(domain = %domain, "Certificate renewed successfully");
                }
                Err(e) => {
                    warn!(
                        domain = %domain,
                        error = ?e,
                        "Failed to renew certificate"
                    );
                }
            }
        }

        info!(renewed_count = renewed.len(), "Certificate renewal complete");
        Ok(renewed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acme_config_creation() {
        let config = AcmeConfig::new("test@example.com", "/tmp/acme");
        assert_eq!(config.email, "test@example.com");
        assert!(!config.use_staging);
    }

    #[test]
    fn test_acme_config_staging() {
        let config = AcmeConfig::new("test@example.com", "/tmp/acme").use_staging();
        assert!(config.use_staging);
    }

    // Note: Full integration tests require actual ACME server interaction
    // These would be run in a separate test suite with proper infrastructure
}
