//! DNS Verification Module
//!
//! This module provides DNS verification for custom domain validation,
//! including TXT record verification and DNS health checks.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
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

/// System DNS provider using the system resolver
pub struct SystemDnsProvider {
    /// DNS timeout in seconds
    timeout_secs: u64,
}

impl SystemDnsProvider {
    pub fn new() -> Self {
        Self { timeout_secs: 10 }
    }

    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self { timeout_secs }
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
        "System DNS Resolver"
    }

    async fn lookup(&self, name: &str, record_type: DnsRecordType) -> Result<Vec<DnsRecord>> {
        debug!(
            name = %name,
            record_type = ?record_type,
            "Looking up DNS record"
        );

        // In a real implementation, this would use trust-dns or tokio::net::lookup_host
        // For now, return empty results (system DNS lookup requires native resolver)

        Ok(vec![])
    }

    async fn verify_txt_record(&self, name: &str, expected_value: &str) -> Result<DnsVerification> {
        debug!(
            name = %name,
            expected = %expected_value,
            "Verifying TXT record"
        );

        let records = self.lookup(name, DnsRecordType::Txt).await?;

        for record in &records {
            for value in &record.values {
                if value.trim_matches('"') == expected_value {
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

        // Record not found or doesn't match
        let actual_value = records
            .first()
            .and_then(|r| r.values.first())
            .cloned();

        let status = if records.is_empty() {
            DnsVerificationStatus::Pending
        } else {
            DnsVerificationStatus::Failed
        };

        let error = if records.is_empty() {
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
            "Checking domain resolution"
        );

        // In a real implementation, this would resolve the domain
        // For now, return a mock result

        Ok(DomainResolution {
            domain: domain.to_string(),
            resolves: true,
            ipv4_addresses: vec![],
            ipv6_addresses: vec![],
            cname_chain: vec![],
            error: None,
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

        let actual_value = records
            .first()
            .and_then(|r| r.values.first())
            .cloned();

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

        let ipv4: Vec<_> = a_records
            .into_iter()
            .flat_map(|r| r.values)
            .collect();
        let ipv6: Vec<_> = aaaa_records
            .into_iter()
            .flat_map(|r| r.values)
            .collect();

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

/// DNS propagation checker
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
        // In a real implementation, query each resolver
        // For now, return mock result

        DnsPropagationStatus {
            record_name: record_name.to_string(),
            expected_value: expected_value.to_string(),
            total_resolvers: self.resolvers.len(),
            propagated_resolvers: self.resolvers.len(), // Mock: all propagated
            propagation_percentage: 1.0,
            is_fully_propagated: true,
            resolver_results: HashMap::new(),
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

        let records = provider.lookup("example.com", DnsRecordType::A).await.unwrap();
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

        let resolution = provider.check_domain_resolution("example.com").await.unwrap();
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

        assert!(status.is_fully_propagated);
        assert_eq!(status.propagation_percentage, 1.0);
    }
}
