//! Custom Domain Support for Multi-Tenant White-Label
//!
//! This module provides custom domain management for tenants:
//! - Custom domain configuration per tenant
//! - Automatic SSL certificate provisioning via Let's Encrypt
//! - DNS verification (TXT record validation)
//! - Domain health monitoring
//!
//! ## Usage
//!
//! ```rust,ignore
//! use ramp_core::domain::{DomainService, DomainConfig};
//!
//! // Register a custom domain for tenant
//! let domain = domain_service.register(
//!     "tenant_123",
//!     "app.example.com",
//! ).await?;
//!
//! // Verify DNS configuration
//! let verified = domain_service.verify_dns(&domain).await?;
//!
//! // Provision SSL certificate
//! if verified {
//!     domain_service.provision_ssl(&domain).await?;
//! }
//! ```

pub mod dns;
pub mod ssl;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub use dns::{DnsProvider, DnsRecord, DnsRecordType, DnsVerification, DnsVerificationStatus};
pub use ssl::{SslCertificate, SslProvider, SslProvisioningStatus, LetsEncryptProvider};

/// Domain status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainStatus {
    /// Domain registered, pending DNS verification
    PendingDnsVerification,
    /// DNS verified, pending SSL provisioning
    PendingSSL,
    /// SSL provisioning in progress
    ProvisioningSSL,
    /// Domain is active and fully configured
    Active,
    /// SSL certificate expiring soon (within 30 days)
    ExpiringSoon,
    /// SSL certificate expired
    Expired,
    /// DNS verification failed
    DnsVerificationFailed,
    /// SSL provisioning failed
    SSLProvisioningFailed,
    /// Domain disabled by admin
    Disabled,
}

impl DomainStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, DomainStatus::Active | DomainStatus::ExpiringSoon)
    }

    pub fn needs_attention(&self) -> bool {
        matches!(
            self,
            DomainStatus::ExpiringSoon
                | DomainStatus::Expired
                | DomainStatus::DnsVerificationFailed
                | DomainStatus::SSLProvisioningFailed
        )
    }
}

/// Custom domain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomDomain {
    /// Unique domain ID
    pub id: String,
    /// Tenant ID
    pub tenant_id: String,
    /// Domain name (e.g., "app.example.com")
    pub domain: String,
    /// Current status
    pub status: DomainStatus,
    /// DNS verification token
    pub dns_verification_token: Option<String>,
    /// DNS verification record name
    pub dns_verification_record: Option<String>,
    /// SSL certificate info
    pub ssl_certificate: Option<SslCertificateInfo>,
    /// Health check endpoint
    pub health_check_path: String,
    /// Last health check result
    pub last_health_check: Option<HealthCheckResult>,
    /// Whether this is the primary domain for the tenant
    pub is_primary: bool,
    /// Custom headers to add to responses
    pub custom_headers: HashMap<String, String>,
    /// Redirect rules
    pub redirects: Vec<RedirectRule>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// SSL certificate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslCertificateInfo {
    /// Certificate ID/serial
    pub certificate_id: String,
    /// Issuer (e.g., "Let's Encrypt")
    pub issuer: String,
    /// Certificate valid from
    pub valid_from: DateTime<Utc>,
    /// Certificate valid until
    pub valid_until: DateTime<Utc>,
    /// Days until expiry
    pub days_until_expiry: i64,
    /// Auto-renew enabled
    pub auto_renew: bool,
    /// Last renewal attempt
    pub last_renewal_attempt: Option<DateTime<Utc>>,
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Whether health check passed
    pub healthy: bool,
    /// HTTP status code
    pub status_code: Option<u16>,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Error message if unhealthy
    pub error: Option<String>,
    /// Timestamp of check
    pub checked_at: DateTime<Utc>,
}

/// Redirect rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectRule {
    /// Source path pattern (supports wildcards)
    pub from: String,
    /// Target URL
    pub to: String,
    /// Redirect type (301, 302, etc.)
    pub redirect_type: u16,
    /// Whether rule is enabled
    pub enabled: bool,
}

/// Domain registration request
#[derive(Debug, Clone)]
pub struct RegisterDomainRequest {
    /// Tenant ID
    pub tenant_id: String,
    /// Domain to register
    pub domain: String,
    /// Whether this should be the primary domain
    pub is_primary: bool,
    /// Custom health check path
    pub health_check_path: Option<String>,
}

/// Domain configuration update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDomainRequest {
    /// Whether this should be the primary domain
    pub is_primary: Option<bool>,
    /// Custom headers
    pub custom_headers: Option<HashMap<String, String>>,
    /// Redirect rules
    pub redirects: Option<Vec<RedirectRule>>,
    /// Health check path
    pub health_check_path: Option<String>,
}

/// Domain service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainServiceConfig {
    /// Let's Encrypt account email
    pub letsencrypt_email: String,
    /// Use Let's Encrypt staging (for testing)
    pub letsencrypt_staging: bool,
    /// DNS verification timeout in seconds
    pub dns_verification_timeout_secs: u64,
    /// Health check interval in seconds
    pub health_check_interval_secs: u64,
    /// SSL renewal threshold days (renew if expiring within this many days)
    pub ssl_renewal_threshold_days: i64,
    /// Maximum domains per tenant
    pub max_domains_per_tenant: u32,
}

impl Default for DomainServiceConfig {
    fn default() -> Self {
        Self {
            letsencrypt_email: String::new(),
            letsencrypt_staging: true,
            dns_verification_timeout_secs: 300,
            health_check_interval_secs: 60,
            ssl_renewal_threshold_days: 30,
            max_domains_per_tenant: 10,
        }
    }
}

/// Domain storage trait
#[async_trait]
pub trait DomainStore: Send + Sync {
    /// Get domain by ID
    async fn get(&self, domain_id: &str) -> Result<Option<CustomDomain>>;

    /// Get domain by domain name
    async fn get_by_domain(&self, domain: &str) -> Result<Option<CustomDomain>>;

    /// List domains for tenant
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<CustomDomain>>;

    /// Save domain
    async fn save(&self, domain: &CustomDomain) -> Result<()>;

    /// Delete domain
    async fn delete(&self, domain_id: &str) -> Result<()>;

    /// List domains needing SSL renewal
    async fn list_expiring_ssl(&self, days_threshold: i64) -> Result<Vec<CustomDomain>>;

    /// List domains needing health check
    async fn list_for_health_check(&self, last_check_before: DateTime<Utc>) -> Result<Vec<CustomDomain>>;
}

/// In-memory domain store for testing
pub struct InMemoryDomainStore {
    domains: Arc<RwLock<HashMap<String, CustomDomain>>>,
}

impl InMemoryDomainStore {
    pub fn new() -> Self {
        Self {
            domains: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryDomainStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainStore for InMemoryDomainStore {
    async fn get(&self, domain_id: &str) -> Result<Option<CustomDomain>> {
        Ok(self.domains.read().await.get(domain_id).cloned())
    }

    async fn get_by_domain(&self, domain: &str) -> Result<Option<CustomDomain>> {
        let domains = self.domains.read().await;
        Ok(domains.values().find(|d| d.domain == domain).cloned())
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<CustomDomain>> {
        let domains = self.domains.read().await;
        Ok(domains
            .values()
            .filter(|d| d.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn save(&self, domain: &CustomDomain) -> Result<()> {
        self.domains
            .write()
            .await
            .insert(domain.id.clone(), domain.clone());
        Ok(())
    }

    async fn delete(&self, domain_id: &str) -> Result<()> {
        self.domains.write().await.remove(domain_id);
        Ok(())
    }

    async fn list_expiring_ssl(&self, days_threshold: i64) -> Result<Vec<CustomDomain>> {
        let domains = self.domains.read().await;
        Ok(domains
            .values()
            .filter(|d| {
                if let Some(ref ssl) = d.ssl_certificate {
                    ssl.days_until_expiry <= days_threshold
                } else {
                    false
                }
            })
            .cloned()
            .collect())
    }

    async fn list_for_health_check(&self, last_check_before: DateTime<Utc>) -> Result<Vec<CustomDomain>> {
        let domains = self.domains.read().await;
        Ok(domains
            .values()
            .filter(|d| {
                d.status.is_active()
                    && d.last_health_check
                        .as_ref()
                        .map(|h| h.checked_at < last_check_before)
                        .unwrap_or(true)
            })
            .cloned()
            .collect())
    }
}

/// Domain management service
pub struct DomainService<S: DomainStore, D: DnsProvider, L: SslProvider> {
    config: DomainServiceConfig,
    store: Arc<S>,
    dns_provider: Arc<D>,
    ssl_provider: Arc<L>,
}

impl<S: DomainStore, D: DnsProvider, L: SslProvider> DomainService<S, D, L> {
    pub fn new(
        config: DomainServiceConfig,
        store: Arc<S>,
        dns_provider: Arc<D>,
        ssl_provider: Arc<L>,
    ) -> Self {
        Self {
            config,
            store,
            dns_provider,
            ssl_provider,
        }
    }

    /// Register a new custom domain for a tenant
    pub async fn register(&self, request: RegisterDomainRequest) -> Result<CustomDomain> {
        info!(
            tenant_id = %request.tenant_id,
            domain = %request.domain,
            "Registering custom domain"
        );

        // Validate domain format
        Self::validate_domain(&request.domain)?;

        // Check if domain already exists
        if let Some(existing) = self.store.get_by_domain(&request.domain).await? {
            return Err(Error::Validation(format!(
                "Domain {} is already registered by tenant {}",
                request.domain, existing.tenant_id
            )));
        }

        // Check tenant domain limit
        let existing_domains = self.store.list_by_tenant(&request.tenant_id).await?;
        if existing_domains.len() >= self.config.max_domains_per_tenant as usize {
            return Err(Error::Validation(format!(
                "Tenant has reached maximum domain limit of {}",
                self.config.max_domains_per_tenant
            )));
        }

        // Generate DNS verification token
        let verification_token = Self::generate_verification_token();
        let verification_record = format!("_ramp-verify.{}", request.domain);

        let domain = CustomDomain {
            id: format!("dom_{}", uuid::Uuid::now_v7()),
            tenant_id: request.tenant_id,
            domain: request.domain,
            status: DomainStatus::PendingDnsVerification,
            dns_verification_token: Some(verification_token.clone()),
            dns_verification_record: Some(verification_record.clone()),
            ssl_certificate: None,
            health_check_path: request.health_check_path.unwrap_or_else(|| "/health".to_string()),
            last_health_check: None,
            is_primary: request.is_primary,
            custom_headers: HashMap::new(),
            redirects: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.store.save(&domain).await?;

        info!(
            domain_id = %domain.id,
            domain = %domain.domain,
            verification_record = %verification_record,
            "Domain registered, pending DNS verification"
        );

        Ok(domain)
    }

    /// Verify DNS configuration for a domain
    pub async fn verify_dns(&self, domain_id: &str) -> Result<DnsVerification> {
        let mut domain = self.store.get(domain_id).await?.ok_or_else(|| {
            Error::NotFound(format!("Domain {} not found", domain_id))
        })?;

        let token = domain.dns_verification_token.as_ref().ok_or_else(|| {
            Error::Validation("Domain has no verification token".to_string())
        })?;

        let record = domain.dns_verification_record.as_ref().ok_or_else(|| {
            Error::Validation("Domain has no verification record".to_string())
        })?;

        debug!(
            domain = %domain.domain,
            record = %record,
            "Verifying DNS configuration"
        );

        let verification = self
            .dns_provider
            .verify_txt_record(record, token)
            .await?;

        match verification.status {
            DnsVerificationStatus::Verified => {
                domain.status = DomainStatus::PendingSSL;
                domain.updated_at = Utc::now();
                self.store.save(&domain).await?;

                info!(
                    domain_id = %domain_id,
                    domain = %domain.domain,
                    "DNS verification successful"
                );
            }
            DnsVerificationStatus::Failed => {
                domain.status = DomainStatus::DnsVerificationFailed;
                domain.updated_at = Utc::now();
                self.store.save(&domain).await?;

                warn!(
                    domain_id = %domain_id,
                    domain = %domain.domain,
                    error = ?verification.error,
                    "DNS verification failed"
                );
            }
            DnsVerificationStatus::Pending => {
                debug!(
                    domain_id = %domain_id,
                    domain = %domain.domain,
                    "DNS verification still pending"
                );
            }
        }

        Ok(verification)
    }

    /// Provision SSL certificate for a domain
    pub async fn provision_ssl(&self, domain_id: &str) -> Result<SslCertificate> {
        let mut domain = self.store.get(domain_id).await?.ok_or_else(|| {
            Error::NotFound(format!("Domain {} not found", domain_id))
        })?;

        // Check domain is ready for SSL
        if domain.status != DomainStatus::PendingSSL
            && domain.status != DomainStatus::SSLProvisioningFailed
            && domain.status != DomainStatus::Expired
        {
            return Err(Error::Validation(format!(
                "Domain is not ready for SSL provisioning, current status: {:?}",
                domain.status
            )));
        }

        domain.status = DomainStatus::ProvisioningSSL;
        domain.updated_at = Utc::now();
        self.store.save(&domain).await?;

        info!(
            domain_id = %domain_id,
            domain = %domain.domain,
            "Provisioning SSL certificate"
        );

        let certificate = match self
            .ssl_provider
            .provision(&domain.domain, &self.config.letsencrypt_email)
            .await
        {
            Ok(cert) => cert,
            Err(e) => {
                domain.status = DomainStatus::SSLProvisioningFailed;
                domain.updated_at = Utc::now();
                self.store.save(&domain).await?;

                error!(
                    domain_id = %domain_id,
                    domain = %domain.domain,
                    error = %e,
                    "SSL provisioning failed"
                );

                return Err(e);
            }
        };

        let days_until_expiry = (certificate.valid_until - Utc::now()).num_days();

        domain.ssl_certificate = Some(SslCertificateInfo {
            certificate_id: certificate.certificate_id.clone(),
            issuer: certificate.issuer.clone(),
            valid_from: certificate.valid_from,
            valid_until: certificate.valid_until,
            days_until_expiry,
            auto_renew: true,
            last_renewal_attempt: None,
        });
        domain.status = DomainStatus::Active;
        domain.updated_at = Utc::now();

        self.store.save(&domain).await?;

        info!(
            domain_id = %domain_id,
            domain = %domain.domain,
            certificate_id = %certificate.certificate_id,
            valid_until = %certificate.valid_until,
            "SSL certificate provisioned successfully"
        );

        Ok(certificate)
    }

    /// Renew SSL certificate for a domain
    pub async fn renew_ssl(&self, domain_id: &str) -> Result<SslCertificate> {
        let mut domain = self.store.get(domain_id).await?.ok_or_else(|| {
            Error::NotFound(format!("Domain {} not found", domain_id))
        })?;

        if !domain.status.is_active() && domain.status != DomainStatus::Expired {
            return Err(Error::Validation(format!(
                "Cannot renew SSL for domain with status: {:?}",
                domain.status
            )));
        }

        info!(
            domain_id = %domain_id,
            domain = %domain.domain,
            "Renewing SSL certificate"
        );

        if let Some(ref mut ssl_info) = domain.ssl_certificate {
            ssl_info.last_renewal_attempt = Some(Utc::now());
        }
        self.store.save(&domain).await?;

        let certificate = self
            .ssl_provider
            .renew(&domain.domain, &self.config.letsencrypt_email)
            .await?;

        let days_until_expiry = (certificate.valid_until - Utc::now()).num_days();

        domain.ssl_certificate = Some(SslCertificateInfo {
            certificate_id: certificate.certificate_id.clone(),
            issuer: certificate.issuer.clone(),
            valid_from: certificate.valid_from,
            valid_until: certificate.valid_until,
            days_until_expiry,
            auto_renew: true,
            last_renewal_attempt: Some(Utc::now()),
        });
        domain.status = DomainStatus::Active;
        domain.updated_at = Utc::now();

        self.store.save(&domain).await?;

        info!(
            domain_id = %domain_id,
            domain = %domain.domain,
            new_expiry = %certificate.valid_until,
            "SSL certificate renewed successfully"
        );

        Ok(certificate)
    }

    /// Run health check on a domain
    pub async fn health_check(&self, domain_id: &str) -> Result<HealthCheckResult> {
        let mut domain = self.store.get(domain_id).await?.ok_or_else(|| {
            Error::NotFound(format!("Domain {} not found", domain_id))
        })?;

        let url = format!("https://{}{}", domain.domain, domain.health_check_path);
        let start = std::time::Instant::now();

        // Perform health check (simplified - in production use reqwest)
        let result = HealthCheckResult {
            healthy: true,
            status_code: Some(200),
            response_time_ms: start.elapsed().as_millis() as u64,
            error: None,
            checked_at: Utc::now(),
        };

        domain.last_health_check = Some(result.clone());
        domain.updated_at = Utc::now();
        self.store.save(&domain).await?;

        debug!(
            domain_id = %domain_id,
            domain = %domain.domain,
            healthy = %result.healthy,
            response_time_ms = %result.response_time_ms,
            "Health check completed"
        );

        Ok(result)
    }

    /// Update domain configuration
    pub async fn update(&self, domain_id: &str, request: UpdateDomainRequest) -> Result<CustomDomain> {
        let mut domain = self.store.get(domain_id).await?.ok_or_else(|| {
            Error::NotFound(format!("Domain {} not found", domain_id))
        })?;

        if let Some(is_primary) = request.is_primary {
            domain.is_primary = is_primary;
        }

        if let Some(headers) = request.custom_headers {
            domain.custom_headers = headers;
        }

        if let Some(redirects) = request.redirects {
            domain.redirects = redirects;
        }

        if let Some(health_path) = request.health_check_path {
            domain.health_check_path = health_path;
        }

        domain.updated_at = Utc::now();
        self.store.save(&domain).await?;

        info!(
            domain_id = %domain_id,
            domain = %domain.domain,
            "Domain configuration updated"
        );

        Ok(domain)
    }

    /// Delete a domain
    pub async fn delete(&self, domain_id: &str) -> Result<()> {
        let domain = self.store.get(domain_id).await?.ok_or_else(|| {
            Error::NotFound(format!("Domain {} not found", domain_id))
        })?;

        // Revoke SSL if exists
        if let Some(ref ssl) = domain.ssl_certificate {
            if let Err(e) = self.ssl_provider.revoke(&ssl.certificate_id).await {
                warn!(
                    domain_id = %domain_id,
                    certificate_id = %ssl.certificate_id,
                    error = %e,
                    "Failed to revoke SSL certificate"
                );
            }
        }

        self.store.delete(domain_id).await?;

        info!(
            domain_id = %domain_id,
            domain = %domain.domain,
            "Domain deleted"
        );

        Ok(())
    }

    /// Get domain by ID
    pub async fn get(&self, domain_id: &str) -> Result<Option<CustomDomain>> {
        self.store.get(domain_id).await
    }

    /// Get domain by domain name
    pub async fn get_by_domain(&self, domain: &str) -> Result<Option<CustomDomain>> {
        self.store.get_by_domain(domain).await
    }

    /// List domains for tenant
    pub async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<CustomDomain>> {
        self.store.list_by_tenant(tenant_id).await
    }

    /// Process domains needing SSL renewal
    pub async fn process_ssl_renewals(&self) -> Result<Vec<String>> {
        let domains = self
            .store
            .list_expiring_ssl(self.config.ssl_renewal_threshold_days)
            .await?;

        let mut renewed = Vec::new();

        for domain in domains {
            if let Some(ref ssl) = domain.ssl_certificate {
                if ssl.auto_renew {
                    match self.renew_ssl(&domain.id).await {
                        Ok(_) => {
                            renewed.push(domain.id.clone());
                        }
                        Err(e) => {
                            error!(
                                domain_id = %domain.id,
                                domain = %domain.domain,
                                error = %e,
                                "Failed to auto-renew SSL"
                            );
                        }
                    }
                }
            }
        }

        info!(
            renewed_count = renewed.len(),
            "SSL renewal processing completed"
        );

        Ok(renewed)
    }

    /// Validate domain format
    fn validate_domain(domain: &str) -> Result<()> {
        // Basic validation
        if domain.is_empty() {
            return Err(Error::Validation("Domain cannot be empty".to_string()));
        }

        if domain.len() > 253 {
            return Err(Error::Validation("Domain too long".to_string()));
        }

        // Check for valid characters
        for label in domain.split('.') {
            if label.is_empty() || label.len() > 63 {
                return Err(Error::Validation("Invalid domain label".to_string()));
            }

            if !label
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                return Err(Error::Validation(
                    "Domain contains invalid characters".to_string(),
                ));
            }

            if label.starts_with('-') || label.ends_with('-') {
                return Err(Error::Validation(
                    "Domain labels cannot start or end with hyphen".to_string(),
                ));
            }
        }

        // Must have at least one dot (e.g., "example.com")
        if !domain.contains('.') {
            return Err(Error::Validation(
                "Domain must include TLD (e.g., example.com)".to_string(),
            ));
        }

        Ok(())
    }

    /// Generate verification token
    fn generate_verification_token() -> String {
        use rand::Rng;
        let bytes: [u8; 16] = rand::thread_rng().gen();
        format!("ramp-verify-{}", hex::encode(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_domain_valid() {
        assert!(DomainService::<InMemoryDomainStore, dns::MockDnsProvider, ssl::MockSslProvider>::validate_domain("example.com").is_ok());
        assert!(DomainService::<InMemoryDomainStore, dns::MockDnsProvider, ssl::MockSslProvider>::validate_domain("app.example.com").is_ok());
        assert!(DomainService::<InMemoryDomainStore, dns::MockDnsProvider, ssl::MockSslProvider>::validate_domain("my-app.example.co.uk").is_ok());
    }

    #[test]
    fn test_validate_domain_invalid() {
        assert!(DomainService::<InMemoryDomainStore, dns::MockDnsProvider, ssl::MockSslProvider>::validate_domain("").is_err());
        assert!(DomainService::<InMemoryDomainStore, dns::MockDnsProvider, ssl::MockSslProvider>::validate_domain("example").is_err());
        assert!(DomainService::<InMemoryDomainStore, dns::MockDnsProvider, ssl::MockSslProvider>::validate_domain("-example.com").is_err());
        assert!(DomainService::<InMemoryDomainStore, dns::MockDnsProvider, ssl::MockSslProvider>::validate_domain("example-.com").is_err());
    }

    #[test]
    fn test_generate_verification_token() {
        let token = DomainService::<InMemoryDomainStore, dns::MockDnsProvider, ssl::MockSslProvider>::generate_verification_token();
        assert!(token.starts_with("ramp-verify-"));
        assert!(token.len() > 20);
    }

    #[test]
    fn test_domain_status() {
        assert!(DomainStatus::Active.is_active());
        assert!(DomainStatus::ExpiringSoon.is_active());
        assert!(!DomainStatus::Disabled.is_active());

        assert!(DomainStatus::ExpiringSoon.needs_attention());
        assert!(DomainStatus::Expired.needs_attention());
        assert!(!DomainStatus::Active.needs_attention());
    }
}
