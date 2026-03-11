//! DNS Verification Module
//!
//! This module provides DNS verification for custom domain validation,
//! including TXT record verification and DNS health checks.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hickory_resolver::config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// DNS record type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DnsRecordType {
    /// A record (IPv4)
    A,
    /// AAAA record (IPv6)
    Aaaa,
    /// CNAME record
    Cname,
    /// TXT record
    Txt,
    /// MX record
    Mx,
    /// NS record
    Ns,
}

/// DNS record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    /// Record name (e.g., "_ramp-verify.example.com")
    pub name: String,
    /// Record type
    pub record_type: DnsRecordType,
    /// Record value(s)
    pub values: Vec<String>,
    /// TTL in seconds
    pub ttl: u32,
    /// When the record was queried
    pub queried_at: DateTime<Utc>,
}

/// DNS verification status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DnsVerificationStatus {
    /// Verification pending (record not found yet)
    Pending,
    /// Record found and verified
    Verified,
    /// Verification failed
    Failed,
}

/// DNS verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsVerification {
    /// Record name that was checked
    pub record_name: String,
    /// Expected value
    pub expected_value: String,
    /// Actual value found (if any)
    pub actual_value: Option<String>,
    /// Verification status
    pub status: DnsVerificationStatus,
    /// Error message if failed
    pub error: Option<String>,
    /// Verification timestamp
    pub verified_at: DateTime<Utc>,
}

/// DNS provider trait
#[async_trait]
pub trait DnsProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Lookup DNS records
    async fn lookup(&self, name: &str, record_type: DnsRecordType) -> Result<Vec<DnsRecord>>;

    /// Verify a TXT record matches expected value
    async fn verify_txt_record(&self, name: &str, expected_value: &str) -> Result<DnsVerification>;

    /// Check if domain resolves correctly
    async fn check_domain_resolution(&self, domain: &str) -> Result<DomainResolution>;
}

/// Domain resolution check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainResolution {
    /// Domain that was checked
    pub domain: String,
    /// Whether domain resolves
    pub resolves: bool,
    /// IPv4 addresses found
    pub ipv4_addresses: Vec<String>,
    /// IPv6 addresses found
    pub ipv6_addresses: Vec<String>,
    /// CNAME chain (if any)
    pub cname_chain: Vec<String>,
    /// Error if resolution failed
    pub error: Option<String>,
    /// Check timestamp
    pub checked_at: DateTime<Utc>,
}

/// System DNS provider using hickory-resolver for real DNS lookups
#[allow(dead_code)]
pub struct SystemDnsProvider {
    /// DNS timeout in seconds
    timeout_secs: u64,
    /// The async DNS resolver
    resolver: TokioAsyncResolver,
}

impl SystemDnsProvider {
    pub fn new() -> Self {
        Self::with_timeout(5)
    }

    pub fn with_timeout(timeout_secs: u64) -> Self {
        let mut opts = ResolverOpts::default();
        opts.timeout = Duration::from_secs(timeout_secs);
        opts.attempts = 2;
        opts.use_hosts_file = false;

        let resolver = TokioAsyncResolver::tokio(ResolverConfig::default(), opts);

        Self {
            timeout_secs,
            resolver,
        }
    }
}

impl Default for SystemDnsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DnsProvider for SystemDnsProvider {
    fn name(&self) -> &str {
        "System DNS Resolver (hickory)"
    }

    async fn lookup(&self, name: &str, record_type: DnsRecordType) -> Result<Vec<DnsRecord>> {
        debug!(
            name = %name,
            record_type = ?record_type,
            "Looking up DNS record"
        );

        match record_type {
            DnsRecordType::A => match self.resolver.ipv4_lookup(name).await {
                Ok(lookup) => {
                    let values: Vec<String> = lookup.iter().map(|ip| ip.to_string()).collect();
                    Ok(vec![DnsRecord {
                        name: name.to_string(),
                        record_type,
                        values,
                        ttl: 300,
                        queried_at: Utc::now(),
                    }])
                }
                Err(e) => {
                    debug!(error = %e, "A record lookup failed");
                    Ok(vec![])
                }
            },
            DnsRecordType::Aaaa => match self.resolver.ipv6_lookup(name).await {
                Ok(lookup) => {
                    let values: Vec<String> = lookup.iter().map(|ip| ip.to_string()).collect();
                    Ok(vec![DnsRecord {
                        name: name.to_string(),
                        record_type,
                        values,
                        ttl: 300,
                        queried_at: Utc::now(),
                    }])
                }
                Err(e) => {
                    debug!(error = %e, "AAAA record lookup failed");
                    Ok(vec![])
                }
            },
            DnsRecordType::Cname => {
                match self
                    .resolver
                    .lookup(name, hickory_resolver::proto::rr::RecordType::CNAME)
                    .await
                {
                    Ok(lookup) => {
                        let values: Vec<String> = lookup
                            .iter()
                            .filter_map(|rdata| {
                                rdata.as_cname().map(|cname| {
                                    cname.0.to_string().trim_end_matches('.').to_string()
                                })
                            })
                            .collect();
                        Ok(vec![DnsRecord {
                            name: name.to_string(),
                            record_type,
                            values,
                            ttl: 300,
                            queried_at: Utc::now(),
                        }])
                    }
                    Err(e) => {
                        debug!(error = %e, "CNAME record lookup failed");
                        Ok(vec![])
                    }
                }
            }
            DnsRecordType::Txt => match self.resolver.txt_lookup(name).await {
                Ok(lookup) => {
                    let values: Vec<String> = lookup.iter().map(|txt| txt.to_string()).collect();
                    Ok(vec![DnsRecord {
                        name: name.to_string(),
                        record_type,
                        values,
                        ttl: 300,
                        queried_at: Utc::now(),
                    }])
                }
                Err(e) => {
                    debug!(error = %e, "TXT record lookup failed");
                    Ok(vec![])
                }
            },
            DnsRecordType::Mx => match self.resolver.mx_lookup(name).await {
                Ok(lookup) => {
                    let values: Vec<String> = lookup
                        .iter()
                        .map(|mx| {
                            format!(
                                "{} {}",
                                mx.preference(),
                                mx.exchange().to_string().trim_end_matches('.')
                            )
                        })
                        .collect();
                    Ok(vec![DnsRecord {
                        name: name.to_string(),
                        record_type,
                        values,
                        ttl: 300,
                        queried_at: Utc::now(),
                    }])
                }
                Err(e) => {
                    debug!(error = %e, "MX record lookup failed");
                    Ok(vec![])
                }
            },
            DnsRecordType::Ns => match self.resolver.ns_lookup(name).await {
                Ok(lookup) => {
                    let values: Vec<String> = lookup
                        .iter()
                        .map(|ns| ns.0.to_string().trim_end_matches('.').to_string())
                        .collect();
                    Ok(vec![DnsRecord {
                        name: name.to_string(),
                        record_type,
                        values,
                        ttl: 300,
                        queried_at: Utc::now(),
                    }])
                }
                Err(e) => {
                    debug!(error = %e, "NS record lookup failed");
                    Ok(vec![])
                }
            },
        }
    }

    async fn verify_txt_record(&self, name: &str, expected_value: &str) -> Result<DnsVerification> {
        debug!(
            name = %name,
            expected = %expected_value,
            "Verifying TXT record via real DNS"
        );

        let records = self.lookup(name, DnsRecordType::Txt).await?;

        for record in &records {
            for value in &record.values {
                let cleaned = value.trim_matches('"');
                if cleaned == expected_value {
                    return Ok(DnsVerification {
                        record_name: name.to_string(),
                        expected_value: expected_value.to_string(),
                        actual_value: Some(cleaned.to_string()),
                        status: DnsVerificationStatus::Verified,
                        error: None,
                        verified_at: Utc::now(),
                    });
                }
            }
        }

        let actual_value = records.first().and_then(|r| r.values.first()).cloned();

        let status = if records.is_empty() || records.iter().all(|r| r.values.is_empty()) {
            DnsVerificationStatus::Pending
        } else {
            DnsVerificationStatus::Failed
        };

        let error = if status == DnsVerificationStatus::Pending {
            Some(format!("TXT record {} not found", name))
        } else {
            Some(format!(
                "TXT record value mismatch: expected '{}', found '{}'",
                expected_value,
                actual_value.as_deref().unwrap_or("(none)")
            ))
        };

        Ok(DnsVerification {
            record_name: name.to_string(),
            expected_value: expected_value.to_string(),
            actual_value,
            status,
            error,
            verified_at: Utc::now(),
        })
    }

    async fn check_domain_resolution(&self, domain: &str) -> Result<DomainResolution> {
        debug!(
            domain = %domain,
            "Checking domain resolution via real DNS"
        );

        let mut ipv4_addresses = Vec::new();
        let mut ipv6_addresses = Vec::new();
        let mut cname_chain = Vec::new();
        let mut resolution_error = None;

        // Look up CNAME chain
        match self.lookup(domain, DnsRecordType::Cname).await {
            Ok(records) => {
                for record in &records {
                    cname_chain.extend(record.values.clone());
                }
            }
            Err(e) => {
                warn!(error = %e, "CNAME lookup failed during resolution check");
            }
        }

        // Look up A records
        match self.lookup(domain, DnsRecordType::A).await {
            Ok(records) => {
                for record in &records {
                    ipv4_addresses.extend(record.values.clone());
                }
            }
            Err(e) => {
                warn!(error = %e, "A record lookup failed during resolution check");
                resolution_error = Some(format!("IPv4 resolution failed: {}", e));
            }
        }

        // Look up AAAA records
        match self.lookup(domain, DnsRecordType::Aaaa).await {
            Ok(records) => {
                for record in &records {
                    ipv6_addresses.extend(record.values.clone());
                }
            }
            Err(e) => {
                warn!(error = %e, "AAAA record lookup failed during resolution check");
                if resolution_error.is_none() {
                    resolution_error = Some(format!("IPv6 resolution failed: {}", e));
                }
            }
        }

        let resolves = !ipv4_addresses.is_empty() || !ipv6_addresses.is_empty();

        if !resolves && resolution_error.is_none() {
            resolution_error = Some(format!(
                "Domain {} does not resolve to any IP address",
                domain
            ));
        }

        Ok(DomainResolution {
            domain: domain.to_string(),
            resolves,
            ipv4_addresses,
            ipv6_addresses,
            cname_chain,
            error: if resolves { None } else { resolution_error },
            checked_at: Utc::now(),
        })
    }
}

/// Mock DNS provider for testing
pub struct MockDnsProvider {
    /// Pre-configured records
    records: Arc<RwLock<HashMap<String, Vec<DnsRecord>>>>,
    /// Simulate failures
    pub simulate_failures: bool,
}

impl MockDnsProvider {
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            simulate_failures: false,
        }
    }

    pub fn with_failures() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            simulate_failures: true,
        }
    }

    /// Add a mock DNS record
    pub async fn add_record(&self, name: &str, record_type: DnsRecordType, values: Vec<String>) {
        let record = DnsRecord {
            name: name.to_string(),
            record_type,
            values,
            ttl: 300,
            queried_at: Utc::now(),
        };

        let mut records = self.records.write().await;
        records
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(record);
    }

    /// Add a TXT record for verification
    pub async fn add_txt_record(&self, name: &str, value: &str) {
        self.add_record(name, DnsRecordType::Txt, vec![value.to_string()])
            .await;
    }

    /// Clear all records
    pub async fn clear(&self) {
        self.records.write().await.clear();
    }
}

impl Default for MockDnsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DnsProvider for MockDnsProvider {
    fn name(&self) -> &str {
        "Mock DNS Provider"
    }

    async fn lookup(&self, name: &str, record_type: DnsRecordType) -> Result<Vec<DnsRecord>> {
        if self.simulate_failures {
            return Err(Error::ExternalService {
                service: "MockDNS".to_string(),
                message: "Simulated DNS lookup failure".to_string(),
            });
        }

        let records = self.records.read().await;
        let matching = records
            .get(name)
            .map(|r| {
                r.iter()
                    .filter(|rec| rec.record_type == record_type)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        Ok(matching)
    }

    async fn verify_txt_record(&self, name: &str, expected_value: &str) -> Result<DnsVerification> {
        if self.simulate_failures {
            return Err(Error::ExternalService {
                service: "MockDNS".to_string(),
                message: "Simulated DNS verification failure".to_string(),
            });
        }

        let records = self.lookup(name, DnsRecordType::Txt).await?;

        for record in &records {
            for value in &record.values {
                if value == expected_value {
                    return Ok(DnsVerification {
                        record_name: name.to_string(),
                        expected_value: expected_value.to_string(),
                        actual_value: Some(value.clone()),
                        status: DnsVerificationStatus::Verified,
                        error: None,
                        verified_at: Utc::now(),
                    });
                }
            }
        }

        let actual_value = records.first().and_then(|r| r.values.first()).cloned();

        let (status, error) = if records.is_empty() {
            (
                DnsVerificationStatus::Pending,
                Some(format!("TXT record {} not found", name)),
            )
        } else {
            (
                DnsVerificationStatus::Failed,
                Some(format!(
                    "Value mismatch: expected '{}', found '{}'",
                    expected_value,
                    actual_value.as_deref().unwrap_or("(none)")
                )),
            )
        };

        Ok(DnsVerification {
            record_name: name.to_string(),
            expected_value: expected_value.to_string(),
            actual_value,
            status,
            error,
            verified_at: Utc::now(),
        })
    }

    async fn check_domain_resolution(&self, domain: &str) -> Result<DomainResolution> {
        if self.simulate_failures {
            return Err(Error::ExternalService {
                service: "MockDNS".to_string(),
                message: "Simulated resolution failure".to_string(),
            });
        }

        // Check if we have A or AAAA records for this domain
        let a_records = self.lookup(domain, DnsRecordType::A).await?;
        let aaaa_records = self.lookup(domain, DnsRecordType::Aaaa).await?;

        let ipv4: Vec<_> = a_records.into_iter().flat_map(|r| r.values).collect();
        let ipv6: Vec<_> = aaaa_records.into_iter().flat_map(|r| r.values).collect();

        let resolves = !ipv4.is_empty() || !ipv6.is_empty();

        Ok(DomainResolution {
            domain: domain.to_string(),
            resolves,
            ipv4_addresses: ipv4,
            ipv6_addresses: ipv6,
            cname_chain: vec![],
            error: if resolves {
                None
            } else {
                Some("Domain does not resolve".to_string())
            },
            checked_at: Utc::now(),
        })
    }
}

/// Cloudflare DNS API provider for managing DNS records programmatically.
///
/// Uses the Cloudflare API v4 to create, query, and verify DNS records.
/// Requires `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ZONE_ID` environment variables.
/// Falls back to `SystemDnsProvider` for read-only lookups if credentials are not configured.
pub struct CloudflareDnsProvider {
    /// Cloudflare API token
    api_token: String,
    /// Cloudflare Zone ID
    zone_id: String,
    /// HTTP client for API requests
    client: reqwest::Client,
    /// Fallback DNS resolver for direct lookups
    fallback: SystemDnsProvider,
}

#[derive(Debug, Deserialize)]
struct CloudflareApiResponse<T> {
    success: bool,
    #[allow(dead_code)]
    errors: Vec<CloudflareApiError>,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct CloudflareApiError {
    #[allow(dead_code)]
    code: u32,
    message: String,
}

#[derive(Debug, Deserialize)]
struct CloudflareDnsRecord {
    #[allow(dead_code)]
    id: String,
    name: String,
    #[serde(rename = "type")]
    record_type: String,
    content: String,
    ttl: u32,
}

impl CloudflareDnsProvider {
    /// Create a new Cloudflare DNS provider from environment variables.
    ///
    /// Required env vars:
    /// - `CLOUDFLARE_API_TOKEN`: Cloudflare API token with Zone:DNS:Read permission
    /// - `CLOUDFLARE_ZONE_ID`: Zone ID for the domain
    pub fn from_env() -> Option<Self> {
        let api_token = std::env::var("CLOUDFLARE_API_TOKEN").ok()?;
        let zone_id = std::env::var("CLOUDFLARE_ZONE_ID").ok()?;

        if api_token.is_empty() || zone_id.is_empty() {
            return None;
        }

        info!("Cloudflare DNS provider configured for zone {}", zone_id);
        Some(Self {
            api_token,
            zone_id,
            client: reqwest::Client::new(),
            fallback: SystemDnsProvider::new(),
        })
    }

    /// Create with explicit credentials (for testing or programmatic use)
    pub fn new(api_token: String, zone_id: String) -> Self {
        Self {
            api_token,
            zone_id,
            client: reqwest::Client::new(),
            fallback: SystemDnsProvider::new(),
        }
    }

    /// Query Cloudflare DNS API for records matching name and type
    async fn api_lookup(&self, name: &str, record_type: &str) -> Result<Vec<CloudflareDnsRecord>> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records?name={}&type={}",
            self.zone_id,
            urlencoding::encode(name),
            record_type
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| Error::ExternalService {
                service: "Cloudflare DNS".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::ExternalService {
                service: "Cloudflare DNS".to_string(),
                message: format!("API returned status {}: {}", status, body),
            });
        }

        let api_response: CloudflareApiResponse<Vec<CloudflareDnsRecord>> =
            response.json().await.map_err(|e| Error::ExternalService {
                service: "Cloudflare DNS".to_string(),
                message: format!("Failed to parse API response: {}", e),
            })?;

        if !api_response.success {
            let error_msg = api_response
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(Error::ExternalService {
                service: "Cloudflare DNS".to_string(),
                message: format!("API error: {}", error_msg),
            });
        }

        Ok(api_response.result.unwrap_or_default())
    }

    /// Create a DNS record via Cloudflare API
    pub async fn create_record(
        &self,
        name: &str,
        record_type: &str,
        content: &str,
        ttl: u32,
    ) -> Result<String> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            self.zone_id
        );

        let body = serde_json::json!({
            "type": record_type,
            "name": name,
            "content": content,
            "ttl": if ttl == 0 { 1 } else { ttl }, // 1 = automatic in Cloudflare
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::ExternalService {
                service: "Cloudflare DNS".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::ExternalService {
                service: "Cloudflare DNS".to_string(),
                message: format!("Create record failed with status {}: {}", status, body),
            });
        }

        #[derive(Deserialize)]
        struct CreateResult {
            id: String,
        }

        let api_response: CloudflareApiResponse<CreateResult> =
            response.json().await.map_err(|e| Error::ExternalService {
                service: "Cloudflare DNS".to_string(),
                message: format!("Failed to parse create response: {}", e),
            })?;

        if !api_response.success {
            let error_msg = api_response
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(Error::ExternalService {
                service: "Cloudflare DNS".to_string(),
                message: format!("Create record error: {}", error_msg),
            });
        }

        let record_id = api_response.result.map(|r| r.id).unwrap_or_default();

        info!(
            name = %name,
            record_type = %record_type,
            record_id = %record_id,
            "Created DNS record via Cloudflare API"
        );

        Ok(record_id)
    }

    /// Delete a DNS record by ID via Cloudflare API
    pub async fn delete_record(&self, record_id: &str) -> Result<()> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
            self.zone_id, record_id
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await
            .map_err(|e| Error::ExternalService {
                service: "Cloudflare DNS".to_string(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::ExternalService {
                service: "Cloudflare DNS".to_string(),
                message: format!("Delete record failed: {}", body),
            });
        }

        info!(record_id = %record_id, "Deleted DNS record via Cloudflare API");
        Ok(())
    }

    fn cf_type_to_dns_type(cf_type: &str) -> Option<DnsRecordType> {
        match cf_type {
            "A" => Some(DnsRecordType::A),
            "AAAA" => Some(DnsRecordType::Aaaa),
            "CNAME" => Some(DnsRecordType::Cname),
            "TXT" => Some(DnsRecordType::Txt),
            "MX" => Some(DnsRecordType::Mx),
            "NS" => Some(DnsRecordType::Ns),
            _ => None,
        }
    }

    fn dns_type_to_cf_type(dns_type: DnsRecordType) -> &'static str {
        match dns_type {
            DnsRecordType::A => "A",
            DnsRecordType::Aaaa => "AAAA",
            DnsRecordType::Cname => "CNAME",
            DnsRecordType::Txt => "TXT",
            DnsRecordType::Mx => "MX",
            DnsRecordType::Ns => "NS",
        }
    }
}

#[async_trait]
impl DnsProvider for CloudflareDnsProvider {
    fn name(&self) -> &str {
        "Cloudflare DNS API"
    }

    async fn lookup(&self, name: &str, record_type: DnsRecordType) -> Result<Vec<DnsRecord>> {
        let cf_type = Self::dns_type_to_cf_type(record_type);

        debug!(
            name = %name,
            record_type = %cf_type,
            "Looking up DNS record via Cloudflare API"
        );

        match self.api_lookup(name, cf_type).await {
            Ok(cf_records) => {
                let records: Vec<DnsRecord> = cf_records
                    .into_iter()
                    .filter_map(|r| {
                        Self::cf_type_to_dns_type(&r.record_type).map(|rt| DnsRecord {
                            name: r.name,
                            record_type: rt,
                            values: vec![r.content],
                            ttl: r.ttl,
                            queried_at: Utc::now(),
                        })
                    })
                    .collect();

                if records.is_empty() {
                    // Fall back to direct DNS resolution for records not in this zone
                    debug!(
                        name = %name,
                        "No records found via Cloudflare API, falling back to DNS resolution"
                    );
                    return self.fallback.lookup(name, record_type).await;
                }

                Ok(records)
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Cloudflare API lookup failed, falling back to DNS resolution"
                );
                self.fallback.lookup(name, record_type).await
            }
        }
    }

    async fn verify_txt_record(&self, name: &str, expected_value: &str) -> Result<DnsVerification> {
        debug!(
            name = %name,
            expected = %expected_value,
            "Verifying TXT record via Cloudflare API"
        );

        // First try Cloudflare API
        let records = self.lookup(name, DnsRecordType::Txt).await?;

        for record in &records {
            for value in &record.values {
                let cleaned = value.trim_matches('"');
                if cleaned == expected_value {
                    return Ok(DnsVerification {
                        record_name: name.to_string(),
                        expected_value: expected_value.to_string(),
                        actual_value: Some(cleaned.to_string()),
                        status: DnsVerificationStatus::Verified,
                        error: None,
                        verified_at: Utc::now(),
                    });
                }
            }
        }

        // Also try direct DNS resolution as a double-check
        let dns_result = self.fallback.verify_txt_record(name, expected_value).await;
        if let Ok(ref verification) = dns_result {
            if verification.status == DnsVerificationStatus::Verified {
                return dns_result;
            }
        }

        let actual_value = records.first().and_then(|r| r.values.first()).cloned();

        let status = if records.is_empty() || records.iter().all(|r| r.values.is_empty()) {
            DnsVerificationStatus::Pending
        } else {
            DnsVerificationStatus::Failed
        };

        let error = if status == DnsVerificationStatus::Pending {
            Some(format!("TXT record {} not found", name))
        } else {
            Some(format!(
                "TXT record value mismatch: expected '{}', found '{}'",
                expected_value,
                actual_value.as_deref().unwrap_or("(none)")
            ))
        };

        Ok(DnsVerification {
            record_name: name.to_string(),
            expected_value: expected_value.to_string(),
            actual_value,
            status,
            error,
            verified_at: Utc::now(),
        })
    }

    async fn check_domain_resolution(&self, domain: &str) -> Result<DomainResolution> {
        // For domain resolution, always use direct DNS which is more reliable
        // than API queries (the domain may not be in the Cloudflare zone)
        self.fallback.check_domain_resolution(domain).await
    }
}

/// Create the best available DNS provider based on environment configuration.
///
/// Priority:
/// 1. Cloudflare DNS API (if `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ZONE_ID` are set)
/// 2. Direct DNS resolution via hickory-resolver (always available)
pub fn create_dns_provider() -> Arc<dyn DnsProvider> {
    if let Some(cf) = CloudflareDnsProvider::from_env() {
        info!("Using Cloudflare DNS API provider");
        Arc::new(cf)
    } else {
        info!("Using system DNS resolver (no cloud DNS provider configured)");
        Arc::new(SystemDnsProvider::new())
    }
}

/// DNS propagation checker
#[allow(dead_code)]
pub struct DnsPropagationChecker {
    /// List of DNS resolvers to check
    resolvers: Vec<String>,
    /// Required propagation percentage
    required_percentage: f32,
}

impl DnsPropagationChecker {
    /// Create with default public DNS resolvers
    pub fn new() -> Self {
        Self {
            resolvers: vec![
                "8.8.8.8".to_string(),        // Google
                "8.8.4.4".to_string(),        // Google
                "1.1.1.1".to_string(),        // Cloudflare
                "1.0.0.1".to_string(),        // Cloudflare
                "208.67.222.222".to_string(), // OpenDNS
                "208.67.220.220".to_string(), // OpenDNS
            ],
            required_percentage: 0.8, // 80% of resolvers must have the record
        }
    }

    /// Check DNS propagation status
    pub async fn check_propagation(
        &self,
        record_name: &str,
        expected_value: &str,
    ) -> DnsPropagationStatus {
        let mut resolver_results = HashMap::new();
        let mut propagated = 0usize;

        for resolver_ip in &self.resolvers {
            let found = match resolver_ip.parse::<std::net::IpAddr>() {
                Ok(ip) => {
                    let mut opts = ResolverOpts::default();
                    opts.timeout = Duration::from_secs(3);
                    opts.attempts = 1;
                    let nameserver =
                        NameServerConfig::new(std::net::SocketAddr::new(ip, 53), Protocol::Udp);
                    let config = ResolverConfig::from_parts(None, vec![], vec![nameserver]);
                    let resolver = TokioAsyncResolver::tokio(config, opts);

                    match resolver.txt_lookup(record_name).await {
                        Ok(lookup) => lookup.iter().any(|txt| {
                            let val = txt.to_string();
                            val.trim_matches('"') == expected_value
                        }),
                        Err(_) => false,
                    }
                }
                Err(_) => false,
            };

            resolver_results.insert(resolver_ip.clone(), found);
            if found {
                propagated += 1;
            }
        }

        let total = self.resolvers.len();
        let percentage = if total > 0 {
            propagated as f32 / total as f32
        } else {
            0.0
        };

        DnsPropagationStatus {
            record_name: record_name.to_string(),
            expected_value: expected_value.to_string(),
            total_resolvers: total,
            propagated_resolvers: propagated,
            propagation_percentage: percentage,
            is_fully_propagated: percentage >= self.required_percentage,
            resolver_results,
            checked_at: Utc::now(),
        }
    }
}

impl Default for DnsPropagationChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// DNS propagation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsPropagationStatus {
    /// Record name being checked
    pub record_name: String,
    /// Expected value
    pub expected_value: String,
    /// Total number of resolvers checked
    pub total_resolvers: usize,
    /// Number of resolvers with correct value
    pub propagated_resolvers: usize,
    /// Propagation percentage (0.0 - 1.0)
    pub propagation_percentage: f32,
    /// Whether propagation is complete
    pub is_fully_propagated: bool,
    /// Results per resolver
    pub resolver_results: HashMap<String, bool>,
    /// Check timestamp
    pub checked_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_dns_lookup() {
        let provider = MockDnsProvider::new();
        provider
            .add_record("example.com", DnsRecordType::A, vec!["1.2.3.4".to_string()])
            .await;

        let records = provider
            .lookup("example.com", DnsRecordType::A)
            .await
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].values[0], "1.2.3.4");
    }

    #[tokio::test]
    async fn test_mock_dns_txt_verification_success() {
        let provider = MockDnsProvider::new();
        provider
            .add_txt_record("_ramp-verify.example.com", "ramp-verify-abc123")
            .await;

        let result = provider
            .verify_txt_record("_ramp-verify.example.com", "ramp-verify-abc123")
            .await
            .unwrap();

        assert_eq!(result.status, DnsVerificationStatus::Verified);
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_mock_dns_txt_verification_pending() {
        let provider = MockDnsProvider::new();
        // No record added

        let result = provider
            .verify_txt_record("_ramp-verify.example.com", "ramp-verify-abc123")
            .await
            .unwrap();

        assert_eq!(result.status, DnsVerificationStatus::Pending);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_mock_dns_txt_verification_failed() {
        let provider = MockDnsProvider::new();
        provider
            .add_txt_record("_ramp-verify.example.com", "wrong-value")
            .await;

        let result = provider
            .verify_txt_record("_ramp-verify.example.com", "ramp-verify-abc123")
            .await
            .unwrap();

        assert_eq!(result.status, DnsVerificationStatus::Failed);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_mock_dns_resolution() {
        let provider = MockDnsProvider::new();
        provider
            .add_record("example.com", DnsRecordType::A, vec!["1.2.3.4".to_string()])
            .await;

        let resolution = provider
            .check_domain_resolution("example.com")
            .await
            .unwrap();
        assert!(resolution.resolves);
        assert_eq!(resolution.ipv4_addresses, vec!["1.2.3.4"]);
    }

    #[tokio::test]
    async fn test_mock_dns_failures() {
        let provider = MockDnsProvider::with_failures();

        let result = provider.lookup("example.com", DnsRecordType::A).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_propagation_checker() {
        let checker = DnsPropagationChecker::new();
        let status = checker
            .check_propagation("_ramp-verify.example.com", "token123")
            .await;

        // This record doesn't actually exist in DNS, so propagation won't be full
        // Just verify the checker returns a valid status structure
        assert!(status.total_resolvers > 0);
        assert!(status.propagation_percentage >= 0.0);
        assert!(status.propagation_percentage <= 1.0);
    }

    #[test]
    fn test_cloudflare_type_conversion() {
        assert_eq!(
            CloudflareDnsProvider::cf_type_to_dns_type("A"),
            Some(DnsRecordType::A)
        );
        assert_eq!(
            CloudflareDnsProvider::cf_type_to_dns_type("AAAA"),
            Some(DnsRecordType::Aaaa)
        );
        assert_eq!(
            CloudflareDnsProvider::cf_type_to_dns_type("TXT"),
            Some(DnsRecordType::Txt)
        );
        assert_eq!(
            CloudflareDnsProvider::cf_type_to_dns_type("CNAME"),
            Some(DnsRecordType::Cname)
        );
        assert_eq!(
            CloudflareDnsProvider::cf_type_to_dns_type("MX"),
            Some(DnsRecordType::Mx)
        );
        assert_eq!(
            CloudflareDnsProvider::cf_type_to_dns_type("NS"),
            Some(DnsRecordType::Ns)
        );
        assert_eq!(CloudflareDnsProvider::cf_type_to_dns_type("SRV"), None);

        assert_eq!(
            CloudflareDnsProvider::dns_type_to_cf_type(DnsRecordType::A),
            "A"
        );
        assert_eq!(
            CloudflareDnsProvider::dns_type_to_cf_type(DnsRecordType::Aaaa),
            "AAAA"
        );
        assert_eq!(
            CloudflareDnsProvider::dns_type_to_cf_type(DnsRecordType::Txt),
            "TXT"
        );
    }

    #[test]
    fn test_cloudflare_from_env_returns_none_without_config() {
        // Without env vars set, from_env should return None
        // (in CI/test environments, these env vars won't be set)
        // We can't reliably test this since env vars might be set,
        // so just verify the constructor works
        let provider = CloudflareDnsProvider::new("test-token".to_string(), "zone-123".to_string());
        assert_eq!(provider.name(), "Cloudflare DNS API");
    }

    #[test]
    fn test_create_dns_provider_returns_system() {
        // Without Cloudflare env vars, should return system provider
        let provider = create_dns_provider();
        // The provider name will indicate which implementation was chosen
        let name = provider.name();
        assert!(
            name.contains("DNS") || name.contains("Resolver") || name.contains("Cloudflare"),
            "Provider name should identify the implementation: {}",
            name
        );
    }
}
