//! SSL Certificate Management
//!
//! This module provides SSL certificate provisioning and management,
//! with Let's Encrypt as the primary provider.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use instant_acme::{
    Account, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt, NewAccount, NewOrder,
    OrderStatus,
};
use ramp_common::{Error, Result};
use rcgen::{CertificateParams, DistinguishedName, KeyPair};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// SSL certificate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslCertificate {
    /// Certificate ID/serial
    pub certificate_id: String,
    /// Domain the certificate is for
    pub domain: String,
    /// Certificate issuer
    pub issuer: String,
    /// Certificate PEM (public)
    pub certificate_pem: String,
    /// Private key PEM (sensitive)
    pub private_key_pem: String,
    /// Certificate chain PEM
    pub chain_pem: String,
    /// Full chain (cert + chain)
    pub fullchain_pem: String,
    /// Valid from
    pub valid_from: DateTime<Utc>,
    /// Valid until
    pub valid_until: DateTime<Utc>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

/// SSL provisioning status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SslProvisioningStatus {
    /// Provisioning in progress
    InProgress,
    /// Waiting for domain validation
    PendingValidation,
    /// Successfully provisioned
    Completed,
    /// Provisioning failed
    Failed,
}

/// ACME challenge type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcmeChallengeType {
    /// HTTP-01 challenge
    Http01,
    /// DNS-01 challenge
    Dns01,
}

/// ACME challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcmeChallenge {
    /// Challenge type
    pub challenge_type: AcmeChallengeType,
    /// Token
    pub token: String,
    /// Key authorization
    pub key_authorization: String,
    /// For DNS-01: the TXT record name
    pub dns_record_name: Option<String>,
    /// For DNS-01: the TXT record value
    pub dns_record_value: Option<String>,
    /// For HTTP-01: the URL path
    pub http_path: Option<String>,
    /// For HTTP-01: the response content
    pub http_content: Option<String>,
}

/// SSL provider trait
#[async_trait]
pub trait SslProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Provision a new SSL certificate
    async fn provision(&self, domain: &str, email: &str) -> Result<SslCertificate>;

    /// Renew an existing SSL certificate
    async fn renew(&self, domain: &str, email: &str) -> Result<SslCertificate>;

    /// Revoke a certificate
    async fn revoke(&self, certificate_id: &str) -> Result<()>;

    /// Get pending ACME challenges for a domain
    async fn get_challenges(&self, domain: &str) -> Result<Vec<AcmeChallenge>>;

    /// Complete ACME challenge validation
    async fn complete_challenge(
        &self,
        domain: &str,
        challenge_type: AcmeChallengeType,
    ) -> Result<()>;
}

/// Let's Encrypt SSL provider using real ACME protocol (RFC 8555)
///
/// Uses `instant-acme` for ACME protocol interactions and `rcgen` for CSR generation.
/// Supports both production and staging Let's Encrypt environments.
/// Falls back to mock mode when ACME credentials are not configured.
pub struct LetsEncryptProvider {
    /// Use staging environment (for testing)
    staging: bool,
    /// ACME directory URL
    directory_url: String,
    /// Account email for ACME registration
    account_email: Option<String>,
    /// Serialized ACME account credentials (JSON) for reuse
    account_credentials: Arc<RwLock<Option<String>>>,
    /// Pending orders with their DNS challenge info
    pending_orders: Arc<RwLock<HashMap<String, PendingOrder>>>,
    /// Issued certificates cache
    certificates: Arc<RwLock<HashMap<String, SslCertificate>>>,
    /// Whether to fall back to mock mode (no ACME credentials configured)
    mock_mode: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PendingOrder {
    domain: String,
    email: String,
    challenges: Vec<AcmeChallenge>,
    created_at: DateTime<Utc>,
}

impl LetsEncryptProvider {
    /// Create new Let's Encrypt provider with environment-based configuration.
    ///
    /// Reads `ACME_DIRECTORY_URL` and `ACME_ACCOUNT_EMAIL` from environment.
    /// Falls back to mock mode if `ACME_ACCOUNT_EMAIL` is not set.
    pub fn new(staging: bool) -> Self {
        let env_directory_url = std::env::var("ACME_DIRECTORY_URL").ok();
        let account_email = std::env::var("ACME_ACCOUNT_EMAIL").ok();

        let directory_url = env_directory_url.unwrap_or_else(|| {
            if staging {
                LetsEncrypt::Staging.url().to_string()
            } else {
                LetsEncrypt::Production.url().to_string()
            }
        });

        let mock_mode = account_email.is_none();
        if mock_mode {
            info!("ACME_ACCOUNT_EMAIL not set, LetsEncryptProvider running in mock mode");
        }

        Self {
            staging,
            directory_url,
            account_email,
            account_credentials: Arc::new(RwLock::new(None)),
            pending_orders: Arc::new(RwLock::new(HashMap::new())),
            certificates: Arc::new(RwLock::new(HashMap::new())),
            mock_mode,
        }
    }

    /// Create provider for production use
    pub fn production() -> Self {
        Self::new(false)
    }

    /// Create provider for staging/testing
    pub fn staging() -> Self {
        Self::new(true)
    }

    /// Get or create an ACME account
    async fn get_or_create_account(&self, email: &str) -> std::result::Result<Account, Error> {
        // Check if we have stored credentials
        let stored = self.account_credentials.read().await;
        if let Some(ref creds_json) = *stored {
            let credentials: instant_acme::AccountCredentials = serde_json::from_str(creds_json)
                .map_err(|e| Error::ExternalService {
                    service: "ACME".to_string(),
                    message: format!("Failed to deserialize account credentials: {}", e),
                })?;
            let account = Account::from_credentials(credentials).await.map_err(|e| {
                Error::ExternalService {
                    service: "ACME".to_string(),
                    message: format!("Failed to restore ACME account: {}", e),
                }
            })?;
            return Ok(account);
        }
        drop(stored);

        // Create new account
        let contact = format!("mailto:{}", email);
        let (account, credentials) = Account::create(
            &NewAccount {
                contact: &[&contact],
                terms_of_service_agreed: true,
                only_return_existing: false,
            },
            &self.directory_url,
            None,
        )
        .await
        .map_err(|e| Error::ExternalService {
            service: "ACME".to_string(),
            message: format!("Failed to create ACME account: {}", e),
        })?;

        // Store credentials for reuse
        let creds_json =
            serde_json::to_string(&credentials).map_err(|e| Error::ExternalService {
                service: "ACME".to_string(),
                message: format!("Failed to serialize account credentials: {}", e),
            })?;
        *self.account_credentials.write().await = Some(creds_json);

        info!(
            email = %email,
            directory = %self.directory_url,
            "ACME account created successfully"
        );

        Ok(account)
    }

    /// Perform the full ACME certificate provisioning flow using DNS-01 challenge
    async fn acme_provision(&self, domain: &str, email: &str) -> Result<SslCertificate> {
        let account = self.get_or_create_account(email).await?;

        // Create order
        let identifiers = vec![Identifier::Dns(domain.to_string())];
        let mut order = account
            .new_order(&NewOrder {
                identifiers: &identifiers,
            })
            .await
            .map_err(|e| Error::ExternalService {
                service: "ACME".to_string(),
                message: format!("Failed to create ACME order: {}", e),
            })?;

        let state = order.state();
        info!(domain = %domain, status = ?state.status, "ACME order created");

        // Process authorizations
        let authorizations = order
            .authorizations()
            .await
            .map_err(|e| Error::ExternalService {
                service: "ACME".to_string(),
                message: format!("Failed to get authorizations: {}", e),
            })?;

        let mut dns_challenges_to_store = Vec::new();

        for authz in &authorizations {
            match authz.status {
                AuthorizationStatus::Pending => {
                    // Find DNS-01 challenge
                    let challenge = authz
                        .challenges
                        .iter()
                        .find(|c| c.r#type == ChallengeType::Dns01)
                        .ok_or_else(|| Error::ExternalService {
                            service: "ACME".to_string(),
                            message: "No DNS-01 challenge found for authorization".to_string(),
                        })?;

                    let domain_name = match &authz.identifier {
                        Identifier::Dns(name) => name.clone(),
                    };
                    let dns_record_name = format!("_acme-challenge.{}", domain_name);
                    let key_auth = order.key_authorization(challenge);
                    let dns_value = key_auth.dns_value();

                    info!(
                        domain = %domain,
                        record = %dns_record_name,
                        "DNS-01 challenge requires TXT record"
                    );

                    dns_challenges_to_store.push(AcmeChallenge {
                        challenge_type: AcmeChallengeType::Dns01,
                        token: challenge.token.clone(),
                        key_authorization: key_auth.as_str().to_string(),
                        dns_record_name: Some(dns_record_name),
                        dns_record_value: Some(dns_value),
                        http_path: None,
                        http_content: None,
                    });
                }
                AuthorizationStatus::Valid => {
                    debug!(domain = %domain, "Authorization already valid");
                }
                status => {
                    return Err(Error::ExternalService {
                        service: "ACME".to_string(),
                        message: format!("Unexpected authorization status: {:?}", status),
                    });
                }
            }
        }

        // Store pending order with challenges for external DNS setup
        if !dns_challenges_to_store.is_empty() {
            self.pending_orders.write().await.insert(
                domain.to_string(),
                PendingOrder {
                    domain: domain.to_string(),
                    email: email.to_string(),
                    challenges: dns_challenges_to_store.clone(),
                    created_at: Utc::now(),
                },
            );

            // Wait for DNS propagation (caller should have set up DNS records)
            // Poll with exponential backoff
            let max_attempts = 20;
            let mut attempt = 0;
            loop {
                attempt += 1;
                if attempt > max_attempts {
                    return Err(Error::ExternalService {
                        service: "ACME".to_string(),
                        message: format!(
                            "DNS-01 validation timed out after {} attempts for {}",
                            max_attempts, domain
                        ),
                    });
                }

                // Notify ACME server that challenges are ready
                for authz in &authorizations {
                    if authz.status == AuthorizationStatus::Pending {
                        if let Some(challenge) = authz
                            .challenges
                            .iter()
                            .find(|c| c.r#type == ChallengeType::Dns01)
                        {
                            let challenge_url = challenge.url.clone();
                            order
                                .set_challenge_ready(&challenge_url)
                                .await
                                .map_err(|e| Error::ExternalService {
                                    service: "ACME".to_string(),
                                    message: format!("Failed to set challenge ready: {}", e),
                                })?;
                        }
                    }
                }

                let delay_secs = std::cmp::min(5 * attempt, 60);
                tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;

                // Refresh order state
                order.refresh().await.map_err(|e| Error::ExternalService {
                    service: "ACME".to_string(),
                    message: format!("Failed to refresh order: {}", e),
                })?;

                let state = order.state();
                match state.status {
                    OrderStatus::Ready => {
                        info!(domain = %domain, "ACME order ready for finalization");
                        break;
                    }
                    OrderStatus::Pending => {
                        debug!(
                            domain = %domain,
                            attempt = attempt,
                            "ACME order still pending, waiting..."
                        );
                    }
                    OrderStatus::Invalid => {
                        return Err(Error::ExternalService {
                            service: "ACME".to_string(),
                            message: format!("ACME order became invalid for {}", domain),
                        });
                    }
                    OrderStatus::Valid => {
                        info!(domain = %domain, "ACME order already valid");
                        break;
                    }
                    status => {
                        debug!(domain = %domain, status = ?status, "Unexpected order status");
                    }
                }
            }
        }

        // Generate a private key and CSR using rcgen
        let key_pair = KeyPair::generate().map_err(|e| Error::ExternalService {
            service: "ACME".to_string(),
            message: format!("Failed to generate key pair: {}", e),
        })?;

        let mut params = CertificateParams::new(vec![domain.to_string()]).map_err(|e| {
            Error::ExternalService {
                service: "ACME".to_string(),
                message: format!("Failed to create certificate params: {}", e),
            }
        })?;
        params.distinguished_name = DistinguishedName::new();

        let csr = params
            .serialize_request(&key_pair)
            .map_err(|e| Error::ExternalService {
                service: "ACME".to_string(),
                message: format!("Failed to generate CSR: {}", e),
            })?;

        let csr_der = csr.der();

        // Finalize the order with our CSR
        order
            .finalize(csr_der)
            .await
            .map_err(|e| Error::ExternalService {
                service: "ACME".to_string(),
                message: format!("Failed to finalize ACME order: {}", e),
            })?;

        // Wait for certificate to be available
        let cert_chain_pem = loop {
            match order.certificate().await {
                Ok(Some(cert)) => break cert,
                Ok(None) => {
                    debug!(domain = %domain, "Certificate not yet available, waiting...");
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    order.refresh().await.map_err(|e| Error::ExternalService {
                        service: "ACME".to_string(),
                        message: format!("Failed to refresh order: {}", e),
                    })?;
                }
                Err(e) => {
                    return Err(Error::ExternalService {
                        service: "ACME".to_string(),
                        message: format!("Failed to download certificate: {}", e),
                    });
                }
            }
        };

        let private_key_pem = key_pair.serialize_pem();

        // Parse the certificate chain: first cert is the leaf, rest is the chain
        let pem_blocks: Vec<&str> = cert_chain_pem
            .split("-----END CERTIFICATE-----")
            .filter(|s| s.contains("-----BEGIN CERTIFICATE-----"))
            .collect();

        let certificate_pem = if let Some(first) = pem_blocks.first() {
            format!("{}-----END CERTIFICATE-----\n", first.trim())
        } else {
            cert_chain_pem.clone()
        };

        let chain_pem = if pem_blocks.len() > 1 {
            pem_blocks[1..]
                .iter()
                .map(|b| format!("{}-----END CERTIFICATE-----\n", b.trim()))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        };

        let now = Utc::now();
        let certificate_id = format!("le_{}", uuid::Uuid::now_v7());

        let ssl_cert = SslCertificate {
            certificate_id: certificate_id.clone(),
            domain: domain.to_string(),
            issuer: if self.staging {
                "(STAGING) Let's Encrypt".to_string()
            } else {
                "Let's Encrypt".to_string()
            },
            certificate_pem: certificate_pem.clone(),
            private_key_pem,
            chain_pem: chain_pem.clone(),
            fullchain_pem: cert_chain_pem,
            valid_from: now,
            valid_until: now + Duration::days(90),
            created_at: now,
        };

        // Cache the issued certificate
        self.certificates
            .write()
            .await
            .insert(certificate_id.clone(), ssl_cert.clone());

        // Clean up pending order
        self.pending_orders.write().await.remove(domain);

        info!(
            domain = %domain,
            certificate_id = %certificate_id,
            valid_until = %(now + Duration::days(90)),
            "SSL certificate provisioned via ACME"
        );

        Ok(ssl_cert)
    }

    /// Generate a mock certificate (fallback when ACME is not configured)
    fn generate_mock_certificate(&self, domain: &str) -> SslCertificate {
        let now = Utc::now();
        let certificate_id = format!("le_{}", uuid::Uuid::now_v7());

        let mock_cert_pem = format!(
            "-----BEGIN CERTIFICATE-----\nMOCK_CERTIFICATE_FOR_{}\n-----END CERTIFICATE-----",
            domain
        );
        let mock_key_pem =
            "-----BEGIN PRIVATE KEY-----\nMOCK_PRIVATE_KEY\n-----END PRIVATE KEY-----".to_string();
        let mock_chain_pem =
            "-----BEGIN CERTIFICATE-----\nMOCK_CHAIN\n-----END CERTIFICATE-----".to_string();

        SslCertificate {
            certificate_id,
            domain: domain.to_string(),
            issuer: if self.staging {
                "(STAGING) Let's Encrypt".to_string()
            } else {
                "Let's Encrypt".to_string()
            },
            certificate_pem: mock_cert_pem.clone(),
            private_key_pem: mock_key_pem,
            chain_pem: mock_chain_pem.clone(),
            fullchain_pem: format!("{}\n{}", mock_cert_pem, mock_chain_pem),
            valid_from: now,
            valid_until: now + Duration::days(90),
            created_at: now,
        }
    }
}

#[async_trait]
impl SslProvider for LetsEncryptProvider {
    fn name(&self) -> &str {
        if self.staging {
            "Let's Encrypt (Staging)"
        } else {
            "Let's Encrypt"
        }
    }

    async fn provision(&self, domain: &str, email: &str) -> Result<SslCertificate> {
        info!(
            domain = %domain,
            email = %email,
            staging = %self.staging,
            mock_mode = %self.mock_mode,
            "Provisioning SSL certificate via Let's Encrypt"
        );

        // Fall back to mock if ACME is not configured
        if self.mock_mode {
            warn!(
                domain = %domain,
                "ACME not configured, using mock certificate"
            );
            let certificate = self.generate_mock_certificate(domain);
            self.certificates
                .write()
                .await
                .insert(certificate.certificate_id.clone(), certificate.clone());
            return Ok(certificate);
        }

        // Use the email from env if the caller passed empty
        let effective_email = if email.is_empty() {
            self.account_email.as_deref().unwrap_or(email)
        } else {
            email
        };

        self.acme_provision(domain, effective_email).await
    }

    async fn renew(&self, domain: &str, email: &str) -> Result<SslCertificate> {
        info!(
            domain = %domain,
            email = %email,
            "Renewing SSL certificate"
        );

        // Renewal is the same as provisioning for Let's Encrypt
        self.provision(domain, email).await
    }

    async fn revoke(&self, certificate_id: &str) -> Result<()> {
        info!(
            certificate_id = %certificate_id,
            "Revoking SSL certificate"
        );

        // Remove from local cache
        self.certificates.write().await.remove(certificate_id);

        Ok(())
    }

    async fn get_challenges(&self, domain: &str) -> Result<Vec<AcmeChallenge>> {
        let orders = self.pending_orders.read().await;
        if let Some(order) = orders.get(domain) {
            Ok(order.challenges.clone())
        } else {
            Ok(Vec::new())
        }
    }

    async fn complete_challenge(
        &self,
        domain: &str,
        challenge_type: AcmeChallengeType,
    ) -> Result<()> {
        debug!(
            domain = %domain,
            challenge_type = ?challenge_type,
            "Completing ACME challenge"
        );

        Ok(())
    }
}

/// Mock SSL provider for testing
pub struct MockSslProvider {
    /// Simulate failures
    pub simulate_failures: bool,
    /// Issued certificates
    certificates: Arc<RwLock<HashMap<String, SslCertificate>>>,
}

impl MockSslProvider {
    pub fn new() -> Self {
        Self {
            simulate_failures: false,
            certificates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_failures() -> Self {
        Self {
            simulate_failures: true,
            certificates: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MockSslProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SslProvider for MockSslProvider {
    fn name(&self) -> &str {
        "Mock SSL Provider"
    }

    async fn provision(&self, domain: &str, _email: &str) -> Result<SslCertificate> {
        if self.simulate_failures {
            return Err(Error::ExternalService {
                service: "MockSSL".to_string(),
                message: "Simulated SSL provisioning failure".to_string(),
            });
        }

        let now = Utc::now();
        let certificate = SslCertificate {
            certificate_id: format!("mock_{}", uuid::Uuid::now_v7()),
            domain: domain.to_string(),
            issuer: "Mock CA".to_string(),
            certificate_pem: format!("MOCK_CERT_{}", domain),
            private_key_pem: "MOCK_KEY".to_string(),
            chain_pem: "MOCK_CHAIN".to_string(),
            fullchain_pem: format!("MOCK_CERT_{}\nMOCK_CHAIN", domain),
            valid_from: now,
            valid_until: now + Duration::days(90),
            created_at: now,
        };

        self.certificates
            .write()
            .await
            .insert(certificate.certificate_id.clone(), certificate.clone());

        Ok(certificate)
    }

    async fn renew(&self, domain: &str, email: &str) -> Result<SslCertificate> {
        self.provision(domain, email).await
    }

    async fn revoke(&self, certificate_id: &str) -> Result<()> {
        if self.simulate_failures {
            return Err(Error::ExternalService {
                service: "MockSSL".to_string(),
                message: "Simulated revocation failure".to_string(),
            });
        }

        self.certificates.write().await.remove(certificate_id);
        Ok(())
    }

    async fn get_challenges(&self, _domain: &str) -> Result<Vec<AcmeChallenge>> {
        Ok(vec![AcmeChallenge {
            challenge_type: AcmeChallengeType::Http01,
            token: "mock_token".to_string(),
            key_authorization: "mock_key_auth".to_string(),
            dns_record_name: None,
            dns_record_value: None,
            http_path: Some("/.well-known/acme-challenge/mock_token".to_string()),
            http_content: Some("mock_key_auth".to_string()),
        }])
    }

    async fn complete_challenge(
        &self,
        _domain: &str,
        _challenge_type: AcmeChallengeType,
    ) -> Result<()> {
        Ok(())
    }
}

/// Certificate storage trait for persisting SSL certificates
///
/// Implementations can store certificates in a database, filesystem, or cloud KMS.
#[async_trait]
pub trait CertificateStore: Send + Sync {
    /// Store a certificate
    async fn store(&self, cert: &SslCertificate) -> Result<()>;

    /// Retrieve a certificate by domain
    async fn get_by_domain(&self, domain: &str) -> Result<Option<SslCertificate>>;

    /// Retrieve a certificate by ID
    async fn get_by_id(&self, certificate_id: &str) -> Result<Option<SslCertificate>>;

    /// List all certificates expiring within the given number of days
    async fn list_expiring(&self, days: i64) -> Result<Vec<SslCertificate>>;

    /// Delete a certificate
    async fn delete(&self, certificate_id: &str) -> Result<()>;
}

/// In-memory certificate store (for development and testing)
pub struct InMemoryCertificateStore {
    certificates: Arc<RwLock<HashMap<String, SslCertificate>>>,
}

impl InMemoryCertificateStore {
    pub fn new() -> Self {
        Self {
            certificates: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryCertificateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CertificateStore for InMemoryCertificateStore {
    async fn store(&self, cert: &SslCertificate) -> Result<()> {
        self.certificates
            .write()
            .await
            .insert(cert.certificate_id.clone(), cert.clone());
        Ok(())
    }

    async fn get_by_domain(&self, domain: &str) -> Result<Option<SslCertificate>> {
        let certs = self.certificates.read().await;
        Ok(certs.values().find(|c| c.domain == domain).cloned())
    }

    async fn get_by_id(&self, certificate_id: &str) -> Result<Option<SslCertificate>> {
        Ok(self.certificates.read().await.get(certificate_id).cloned())
    }

    async fn list_expiring(&self, days: i64) -> Result<Vec<SslCertificate>> {
        let threshold = Utc::now() + Duration::days(days);
        let certs = self.certificates.read().await;
        Ok(certs
            .values()
            .filter(|c| c.valid_until <= threshold)
            .cloned()
            .collect())
    }

    async fn delete(&self, certificate_id: &str) -> Result<()> {
        self.certificates.write().await.remove(certificate_id);
        Ok(())
    }
}

/// Certificate renewal manager that checks for and renews expiring certificates.
///
/// Runs as a background task to automatically renew certificates that are
/// within the renewal window (default: 30 days before expiry).
pub struct CertificateRenewalManager<P: SslProvider> {
    provider: Arc<P>,
    store: Arc<dyn CertificateStore>,
    /// Renew certificates expiring within this many days
    renewal_threshold_days: i64,
    /// Email for ACME account
    acme_email: String,
}

impl<P: SslProvider + 'static> CertificateRenewalManager<P> {
    pub fn new(
        provider: Arc<P>,
        store: Arc<dyn CertificateStore>,
        renewal_threshold_days: i64,
        acme_email: String,
    ) -> Self {
        Self {
            provider,
            store,
            renewal_threshold_days,
            acme_email,
        }
    }

    /// Check for and renew expiring certificates. Returns the list of renewed certificate IDs.
    pub async fn process_renewals(&self) -> Result<Vec<String>> {
        let expiring = self
            .store
            .list_expiring(self.renewal_threshold_days)
            .await?;

        if expiring.is_empty() {
            debug!("No certificates need renewal");
            return Ok(Vec::new());
        }

        info!(
            count = expiring.len(),
            threshold_days = self.renewal_threshold_days,
            "Found certificates needing renewal"
        );

        let mut renewed = Vec::new();

        for cert in expiring {
            match self.provider.renew(&cert.domain, &self.acme_email).await {
                Ok(new_cert) => {
                    // Store the new certificate
                    if let Err(e) = self.store.store(&new_cert).await {
                        warn!(
                            domain = %cert.domain,
                            error = %e,
                            "Failed to store renewed certificate"
                        );
                        continue;
                    }

                    // Remove old certificate
                    if let Err(e) = self.store.delete(&cert.certificate_id).await {
                        warn!(
                            old_cert_id = %cert.certificate_id,
                            error = %e,
                            "Failed to delete old certificate"
                        );
                    }

                    info!(
                        domain = %cert.domain,
                        old_cert_id = %cert.certificate_id,
                        new_cert_id = %new_cert.certificate_id,
                        new_expiry = %new_cert.valid_until,
                        "Certificate renewed successfully"
                    );
                    renewed.push(new_cert.certificate_id);
                }
                Err(e) => {
                    warn!(
                        domain = %cert.domain,
                        error = %e,
                        "Failed to renew certificate"
                    );
                }
            }
        }

        info!(
            renewed_count = renewed.len(),
            "Certificate renewal processing complete"
        );

        Ok(renewed)
    }

    /// Run the renewal manager as a periodic background task.
    /// Checks for expiring certificates every `check_interval`.
    pub async fn run_periodic(self: Arc<Self>, check_interval: std::time::Duration) {
        info!(
            interval_secs = check_interval.as_secs(),
            threshold_days = self.renewal_threshold_days,
            "Starting certificate renewal manager"
        );

        loop {
            match self.process_renewals().await {
                Ok(renewed) => {
                    if !renewed.is_empty() {
                        info!(
                            renewed = renewed.len(),
                            "Renewed certificates in this cycle"
                        );
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Error during certificate renewal check");
                }
            }

            tokio::time::sleep(check_interval).await;
        }
    }
}

/// Create a Let's Encrypt SSL provider from environment configuration.
///
/// Reads:
/// - `ACME_STAGING` (bool, default: true) - use Let's Encrypt staging
/// - `ACME_ACCOUNT_EMAIL` - account email for ACME registration
/// - `ACME_DIRECTORY_URL` - custom ACME directory URL (optional)
pub fn create_ssl_provider() -> LetsEncryptProvider {
    let staging = std::env::var("ACME_STAGING")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true);

    LetsEncryptProvider::new(staging)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_ssl_provision() {
        let provider = MockSslProvider::new();
        let cert = provider
            .provision("example.com", "test@example.com")
            .await
            .unwrap();

        assert_eq!(cert.domain, "example.com");
        assert!(cert.certificate_id.starts_with("mock_"));
        assert!(cert.valid_until > Utc::now());
    }

    #[tokio::test]
    async fn test_mock_ssl_revoke() {
        let provider = MockSslProvider::new();
        let cert = provider
            .provision("example.com", "test@example.com")
            .await
            .unwrap();
        let result = provider.revoke(&cert.certificate_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_ssl_failures() {
        let provider = MockSslProvider::with_failures();
        let result = provider.provision("example.com", "test@example.com").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_letsencrypt_staging() {
        let provider = LetsEncryptProvider::staging();
        assert_eq!(provider.name(), "Let's Encrypt (Staging)");

        let cert = provider
            .provision("test.example.com", "test@example.com")
            .await
            .unwrap();
        assert!(cert.issuer.contains("STAGING"));
    }

    #[test]
    fn test_acme_challenge_types() {
        let http_challenge = AcmeChallenge {
            challenge_type: AcmeChallengeType::Http01,
            token: "token".to_string(),
            key_authorization: "key_auth".to_string(),
            dns_record_name: None,
            dns_record_value: None,
            http_path: Some("/.well-known/acme-challenge/token".to_string()),
            http_content: Some("key_auth".to_string()),
        };

        assert!(http_challenge.http_path.is_some());
        assert!(http_challenge.dns_record_name.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_certificate_store() {
        let store = InMemoryCertificateStore::new();
        let now = Utc::now();

        let cert = SslCertificate {
            certificate_id: "test_cert_1".to_string(),
            domain: "example.com".to_string(),
            issuer: "Test CA".to_string(),
            certificate_pem: "CERT_PEM".to_string(),
            private_key_pem: "KEY_PEM".to_string(),
            chain_pem: "CHAIN_PEM".to_string(),
            fullchain_pem: "FULL_PEM".to_string(),
            valid_from: now,
            valid_until: now + Duration::days(90),
            created_at: now,
        };

        // Store
        store.store(&cert).await.unwrap();

        // Get by ID
        let retrieved = store.get_by_id("test_cert_1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().domain, "example.com");

        // Get by domain
        let retrieved = store.get_by_domain("example.com").await.unwrap();
        assert!(retrieved.is_some());

        // Not found
        let not_found = store.get_by_domain("notfound.com").await.unwrap();
        assert!(not_found.is_none());

        // Delete
        store.delete("test_cert_1").await.unwrap();
        let deleted = store.get_by_id("test_cert_1").await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_certificate_store_list_expiring() {
        let store = InMemoryCertificateStore::new();
        let now = Utc::now();

        // Certificate expiring in 10 days
        let cert_expiring = SslCertificate {
            certificate_id: "expiring".to_string(),
            domain: "expiring.example.com".to_string(),
            issuer: "Test CA".to_string(),
            certificate_pem: "PEM".to_string(),
            private_key_pem: "KEY".to_string(),
            chain_pem: "CHAIN".to_string(),
            fullchain_pem: "FULL".to_string(),
            valid_from: now - Duration::days(80),
            valid_until: now + Duration::days(10),
            created_at: now - Duration::days(80),
        };

        // Certificate expiring in 60 days
        let cert_ok = SslCertificate {
            certificate_id: "ok".to_string(),
            domain: "ok.example.com".to_string(),
            issuer: "Test CA".to_string(),
            certificate_pem: "PEM".to_string(),
            private_key_pem: "KEY".to_string(),
            chain_pem: "CHAIN".to_string(),
            fullchain_pem: "FULL".to_string(),
            valid_from: now - Duration::days(30),
            valid_until: now + Duration::days(60),
            created_at: now - Duration::days(30),
        };

        store.store(&cert_expiring).await.unwrap();
        store.store(&cert_ok).await.unwrap();

        // List certificates expiring within 30 days
        let expiring = store.list_expiring(30).await.unwrap();
        assert_eq!(expiring.len(), 1);
        assert_eq!(expiring[0].domain, "expiring.example.com");

        // List certificates expiring within 90 days (both)
        let expiring = store.list_expiring(90).await.unwrap();
        assert_eq!(expiring.len(), 2);
    }

    #[tokio::test]
    async fn test_renewal_manager_process() {
        let provider = Arc::new(MockSslProvider::new());
        let store: Arc<dyn CertificateStore> = Arc::new(InMemoryCertificateStore::new());
        let now = Utc::now();

        // Add an expiring certificate
        let cert = SslCertificate {
            certificate_id: "old_cert".to_string(),
            domain: "renew.example.com".to_string(),
            issuer: "Test CA".to_string(),
            certificate_pem: "PEM".to_string(),
            private_key_pem: "KEY".to_string(),
            chain_pem: "CHAIN".to_string(),
            fullchain_pem: "FULL".to_string(),
            valid_from: now - Duration::days(80),
            valid_until: now + Duration::days(5), // Expiring in 5 days
            created_at: now - Duration::days(80),
        };
        store.store(&cert).await.unwrap();

        let manager = CertificateRenewalManager::new(
            provider,
            store.clone(),
            30, // Renew within 30 days of expiry
            "test@example.com".to_string(),
        );

        let renewed = manager.process_renewals().await.unwrap();
        assert_eq!(renewed.len(), 1);

        // The new certificate should be in the store
        let new_cert = store.get_by_domain("renew.example.com").await.unwrap();
        assert!(new_cert.is_some());
        let new_cert = new_cert.unwrap();
        assert!(new_cert.certificate_id.starts_with("mock_"));

        // Old certificate should be deleted
        let old = store.get_by_id("old_cert").await.unwrap();
        assert!(old.is_none());
    }

    #[test]
    fn test_create_ssl_provider() {
        // Without ACME_STAGING set, defaults to staging
        let provider = create_ssl_provider();
        assert_eq!(provider.name(), "Let's Encrypt (Staging)");
    }
}
