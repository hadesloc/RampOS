//! Compliance Audit Repository
//!
//! Append-only audit log with hash chain for regulatory compliance.
//! This repository enforces immutability and provides hash chain verification.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool, Row};
use std::net::IpAddr;
use uuid::Uuid;

/// Compliance event types that are logged
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceEventType {
    ComplianceDecision,
    DocumentSubmitted,
    RuleChanged,
    UserAction,
    KycTierChange,
    TransactionApproval,
    TransactionRejection,
    AmlRuleModification,
    SarSubmission,
    CtrSubmission,
    LicenseStatusChange,
    SanctionsCheck,
    PepCheck,
}

impl ComplianceEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ComplianceDecision => "compliance_decision",
            Self::DocumentSubmitted => "document_submitted",
            Self::RuleChanged => "rule_changed",
            Self::UserAction => "user_action",
            Self::KycTierChange => "kyc_tier_change",
            Self::TransactionApproval => "transaction_approval",
            Self::TransactionRejection => "transaction_rejection",
            Self::AmlRuleModification => "aml_rule_modification",
            Self::SarSubmission => "sar_submission",
            Self::CtrSubmission => "ctr_submission",
            Self::LicenseStatusChange => "license_status_change",
            Self::SanctionsCheck => "sanctions_check",
            Self::PepCheck => "pep_check",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "compliance_decision" => Some(Self::ComplianceDecision),
            "document_submitted" => Some(Self::DocumentSubmitted),
            "rule_changed" => Some(Self::RuleChanged),
            "user_action" => Some(Self::UserAction),
            "kyc_tier_change" => Some(Self::KycTierChange),
            "transaction_approval" => Some(Self::TransactionApproval),
            "transaction_rejection" => Some(Self::TransactionRejection),
            "aml_rule_modification" => Some(Self::AmlRuleModification),
            "sar_submission" => Some(Self::SarSubmission),
            "ctr_submission" => Some(Self::CtrSubmission),
            "license_status_change" => Some(Self::LicenseStatusChange),
            "sanctions_check" => Some(Self::SanctionsCheck),
            "pep_check" => Some(Self::PepCheck),
            _ => None,
        }
    }
}

/// Actor types for audit entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ActorType {
    System,
    User,
    Admin,
    Api,
}

impl ActorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "SYSTEM",
            Self::User => "USER",
            Self::Admin => "ADMIN",
            Self::Api => "API",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "USER" => Self::User,
            "ADMIN" => Self::Admin,
            "API" => Self::Api,
            _ => Self::System,
        }
    }
}

/// Compliance audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceAuditEntry {
    pub id: Uuid,
    pub tenant_id: String,
    pub event_type: ComplianceEventType,
    pub actor_id: Option<String>,
    pub actor_type: ActorType,
    pub action_details: serde_json::Value,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub sequence_number: i64,
    pub previous_hash: Option<String>,
    pub current_hash: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Database row for compliance audit log
#[derive(Debug, Clone, FromRow)]
pub struct ComplianceAuditRow {
    pub id: Uuid,
    pub tenant_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_type: String,
    pub action_details: serde_json::Value,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub sequence_number: i64,
    pub previous_hash: Option<String>,
    pub current_hash: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<ComplianceAuditRow> for ComplianceAuditEntry {
    fn from(row: ComplianceAuditRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            event_type: ComplianceEventType::from_str(&row.event_type)
                .unwrap_or(ComplianceEventType::UserAction),
            actor_id: row.actor_id,
            actor_type: ActorType::from_str(&row.actor_type),
            action_details: row.action_details,
            resource_type: row.resource_type,
            resource_id: row.resource_id,
            sequence_number: row.sequence_number,
            previous_hash: row.previous_hash,
            current_hash: row.current_hash,
            ip_address: row.ip_address,
            user_agent: row.user_agent,
            request_id: row.request_id,
            created_at: row.created_at,
        }
    }
}

/// Request to create a new compliance audit entry
#[derive(Debug, Clone)]
pub struct CreateComplianceAuditRequest {
    pub event_type: ComplianceEventType,
    pub actor_id: Option<String>,
    pub actor_type: ActorType,
    pub action_details: serde_json::Value,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

/// Result of chain verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerificationResult {
    pub is_valid: bool,
    pub total_entries: i64,
    pub verified_entries: i64,
    pub first_invalid_sequence: Option<i64>,
    pub error_message: Option<String>,
}

/// Query filters for listing audit entries
#[derive(Debug, Clone, Default)]
pub struct AuditQueryFilter {
    pub event_type: Option<ComplianceEventType>,
    pub actor_id: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: i64,
    pub offset: i64,
}

/// Compliance Audit Repository trait
#[async_trait]
pub trait ComplianceAuditRepository: Send + Sync {
    /// Log a new compliance event (append-only)
    async fn log_event(
        &self,
        tenant_id: &TenantId,
        request: CreateComplianceAuditRequest,
    ) -> Result<ComplianceAuditEntry>;

    /// Get audit entries with filters
    async fn get_entries(
        &self,
        tenant_id: &TenantId,
        filter: AuditQueryFilter,
    ) -> Result<Vec<ComplianceAuditEntry>>;

    /// Count audit entries matching filters
    async fn count_entries(
        &self,
        tenant_id: &TenantId,
        filter: AuditQueryFilter,
    ) -> Result<i64>;

    /// Verify the hash chain integrity
    async fn verify_chain(&self, tenant_id: &TenantId) -> Result<ChainVerificationResult>;

    /// Export audit log for regulators (returns all entries in order)
    async fn export_audit_log(
        &self,
        tenant_id: &TenantId,
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
    ) -> Result<Vec<ComplianceAuditEntry>>;

    /// Get the latest entry (for hash chain)
    async fn get_latest_entry(&self, tenant_id: &TenantId) -> Result<Option<ComplianceAuditEntry>>;
}

/// PostgreSQL implementation of ComplianceAuditRepository
pub struct PgComplianceAuditRepository {
    pool: PgPool,
}

impl PgComplianceAuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Calculate SHA256 hash for an entry
    fn calculate_hash(
        event_type: &str,
        actor_id: Option<&str>,
        action_details: &serde_json::Value,
        resource_id: Option<&str>,
        created_at: &DateTime<Utc>,
        previous_hash: Option<&str>,
    ) -> String {
        let mut hasher = Sha256::new();

        hasher.update(event_type.as_bytes());
        hasher.update(actor_id.unwrap_or("").as_bytes());
        hasher.update(action_details.to_string().as_bytes());
        hasher.update(resource_id.unwrap_or("").as_bytes());
        hasher.update(created_at.to_rfc3339().as_bytes());
        hasher.update(previous_hash.unwrap_or("genesis").as_bytes());

        hex::encode(hasher.finalize())
    }

    /// Verify a single entry's hash
    fn verify_entry_hash(entry: &ComplianceAuditEntry) -> bool {
        let calculated = Self::calculate_hash(
            entry.event_type.as_str(),
            entry.actor_id.as_deref(),
            &entry.action_details,
            entry.resource_id.as_deref(),
            &entry.created_at,
            entry.previous_hash.as_deref(),
        );

        calculated == entry.current_hash
    }
}

#[async_trait]
impl ComplianceAuditRepository for PgComplianceAuditRepository {
    async fn log_event(
        &self,
        tenant_id: &TenantId,
        request: CreateComplianceAuditRequest,
    ) -> Result<ComplianceAuditEntry> {
        // Get the latest entry for hash chain
        let latest = self.get_latest_entry(tenant_id).await?;
        let previous_hash = latest.as_ref().map(|e| e.current_hash.clone());

        let id = Uuid::new_v4();
        let created_at = Utc::now();

        // Calculate hash
        let current_hash = Self::calculate_hash(
            request.event_type.as_str(),
            request.actor_id.as_deref(),
            &request.action_details,
            request.resource_id.as_deref(),
            &created_at,
            previous_hash.as_deref(),
        );

        let ip_str = request.ip_address.map(|ip| ip.to_string());

        let row = sqlx::query_as::<_, ComplianceAuditRow>(
            r#"
            INSERT INTO compliance_audit_log (
                id, tenant_id, event_type, actor_id, actor_type,
                action_details, resource_type, resource_id,
                previous_hash, current_hash, ip_address, user_agent,
                request_id, created_at
            ) VALUES (
                $1, $2, $3::compliance_event_type, $4, $5,
                $6, $7, $8,
                $9, $10, $11::inet, $12,
                $13, $14
            )
            RETURNING id, tenant_id, event_type::text, actor_id, actor_type,
                action_details, resource_type, resource_id, sequence_number,
                previous_hash, current_hash, host(ip_address) as ip_address,
                user_agent, request_id, created_at
            "#,
        )
        .bind(id)
        .bind(&tenant_id.0)
        .bind(request.event_type.as_str())
        .bind(&request.actor_id)
        .bind(request.actor_type.as_str())
        .bind(&request.action_details)
        .bind(&request.resource_type)
        .bind(&request.resource_id)
        .bind(&previous_hash)
        .bind(&current_hash)
        .bind(&ip_str)
        .bind(&request.user_agent)
        .bind(&request.request_id)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.into())
    }

    async fn get_entries(
        &self,
        tenant_id: &TenantId,
        filter: AuditQueryFilter,
    ) -> Result<Vec<ComplianceAuditEntry>> {
        let mut query = String::from(
            r#"
            SELECT id, tenant_id, event_type::text, actor_id, actor_type,
                action_details, resource_type, resource_id, sequence_number,
                previous_hash, current_hash, host(ip_address) as ip_address,
                user_agent, request_id, created_at
            FROM compliance_audit_log
            WHERE tenant_id = $1
            "#,
        );

        let mut param_count = 1;

        if filter.event_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND event_type = ${}::compliance_event_type", param_count));
        }

        if filter.actor_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND actor_id = ${}", param_count));
        }

        if filter.resource_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND resource_type = ${}", param_count));
        }

        if filter.resource_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND resource_id = ${}", param_count));
        }

        if filter.from_date.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND created_at >= ${}", param_count));
        }

        if filter.to_date.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND created_at <= ${}", param_count));
        }

        query.push_str(" ORDER BY sequence_number DESC");
        query.push_str(&format!(" LIMIT ${} OFFSET ${}", param_count + 1, param_count + 2));

        let mut q = sqlx::query_as::<_, ComplianceAuditRow>(&query).bind(&tenant_id.0);

        if let Some(et) = &filter.event_type {
            q = q.bind(et.as_str());
        }
        if let Some(aid) = &filter.actor_id {
            q = q.bind(aid);
        }
        if let Some(rt) = &filter.resource_type {
            q = q.bind(rt);
        }
        if let Some(rid) = &filter.resource_id {
            q = q.bind(rid);
        }
        if let Some(fd) = &filter.from_date {
            q = q.bind(fd);
        }
        if let Some(td) = &filter.to_date {
            q = q.bind(td);
        }

        q = q.bind(filter.limit).bind(filter.offset);

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn count_entries(
        &self,
        tenant_id: &TenantId,
        filter: AuditQueryFilter,
    ) -> Result<i64> {
        let mut query = String::from(
            "SELECT COUNT(*) FROM compliance_audit_log WHERE tenant_id = $1",
        );

        let mut param_count = 1;

        if filter.event_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND event_type = ${}::compliance_event_type", param_count));
        }

        if filter.actor_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND actor_id = ${}", param_count));
        }

        if filter.resource_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND resource_type = ${}", param_count));
        }

        if filter.resource_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND resource_id = ${}", param_count));
        }

        if filter.from_date.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND created_at >= ${}", param_count));
        }

        if filter.to_date.is_some() {
            query.push_str(&format!(" AND created_at <= ${}", param_count + 1));
        }

        let mut q = sqlx::query(&query).bind(&tenant_id.0);

        if let Some(et) = &filter.event_type {
            q = q.bind(et.as_str());
        }
        if let Some(aid) = &filter.actor_id {
            q = q.bind(aid);
        }
        if let Some(rt) = &filter.resource_type {
            q = q.bind(rt);
        }
        if let Some(rid) = &filter.resource_id {
            q = q.bind(rid);
        }
        if let Some(fd) = &filter.from_date {
            q = q.bind(fd);
        }
        if let Some(td) = &filter.to_date {
            q = q.bind(td);
        }

        let row = q
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let count: i64 = row.get(0);
        Ok(count)
    }

    async fn verify_chain(&self, tenant_id: &TenantId) -> Result<ChainVerificationResult> {
        // Get all entries in order
        let rows = sqlx::query_as::<_, ComplianceAuditRow>(
            r#"
            SELECT id, tenant_id, event_type::text, actor_id, actor_type,
                action_details, resource_type, resource_id, sequence_number,
                previous_hash, current_hash, host(ip_address) as ip_address,
                user_agent, request_id, created_at
            FROM compliance_audit_log
            WHERE tenant_id = $1
            ORDER BY sequence_number ASC
            "#,
        )
        .bind(&tenant_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        let total_entries = rows.len() as i64;
        let mut verified_entries = 0i64;
        let mut previous_hash: Option<String> = None;

        for row in rows {
            let entry: ComplianceAuditEntry = row.into();

            // Verify hash chain linkage
            if entry.previous_hash != previous_hash {
                return Ok(ChainVerificationResult {
                    is_valid: false,
                    total_entries,
                    verified_entries,
                    first_invalid_sequence: Some(entry.sequence_number),
                    error_message: Some(format!(
                        "Hash chain broken at sequence {}: expected previous_hash {:?}, got {:?}",
                        entry.sequence_number, previous_hash, entry.previous_hash
                    )),
                });
            }

            // Verify entry hash
            if !Self::verify_entry_hash(&entry) {
                return Ok(ChainVerificationResult {
                    is_valid: false,
                    total_entries,
                    verified_entries,
                    first_invalid_sequence: Some(entry.sequence_number),
                    error_message: Some(format!(
                        "Invalid hash at sequence {}: hash does not match content",
                        entry.sequence_number
                    )),
                });
            }

            previous_hash = Some(entry.current_hash);
            verified_entries += 1;
        }

        Ok(ChainVerificationResult {
            is_valid: true,
            total_entries,
            verified_entries,
            first_invalid_sequence: None,
            error_message: None,
        })
    }

    async fn export_audit_log(
        &self,
        tenant_id: &TenantId,
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
    ) -> Result<Vec<ComplianceAuditEntry>> {
        let mut query = String::from(
            r#"
            SELECT id, tenant_id, event_type::text, actor_id, actor_type,
                action_details, resource_type, resource_id, sequence_number,
                previous_hash, current_hash, host(ip_address) as ip_address,
                user_agent, request_id, created_at
            FROM compliance_audit_log
            WHERE tenant_id = $1
            "#,
        );

        if from_date.is_some() {
            query.push_str(" AND created_at >= $2");
        }
        if to_date.is_some() {
            let param = if from_date.is_some() { "$3" } else { "$2" };
            query.push_str(&format!(" AND created_at <= {}", param));
        }

        query.push_str(" ORDER BY sequence_number ASC");

        let mut q = sqlx::query_as::<_, ComplianceAuditRow>(&query).bind(&tenant_id.0);

        if let Some(fd) = from_date {
            q = q.bind(fd);
        }
        if let Some(td) = to_date {
            q = q.bind(td);
        }

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_latest_entry(&self, tenant_id: &TenantId) -> Result<Option<ComplianceAuditEntry>> {
        let row = sqlx::query_as::<_, ComplianceAuditRow>(
            r#"
            SELECT id, tenant_id, event_type::text, actor_id, actor_type,
                action_details, resource_type, resource_id, sequence_number,
                previous_hash, current_hash, host(ip_address) as ip_address,
                user_agent, request_id, created_at
            FROM compliance_audit_log
            WHERE tenant_id = $1
            ORDER BY sequence_number DESC
            LIMIT 1
            "#,
        )
        .bind(&tenant_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_conversion() {
        assert_eq!(ComplianceEventType::KycTierChange.as_str(), "kyc_tier_change");
        assert_eq!(
            ComplianceEventType::from_str("kyc_tier_change"),
            Some(ComplianceEventType::KycTierChange)
        );
    }

    #[test]
    fn test_hash_calculation() {
        let hash1 = PgComplianceAuditRepository::calculate_hash(
            "kyc_tier_change",
            Some("user123"),
            &serde_json::json!({"old_tier": 1, "new_tier": 2}),
            Some("user123"),
            &Utc::now(),
            None,
        );

        assert_eq!(hash1.len(), 64); // SHA256 hex = 64 chars

        // Same input should produce same hash
        let created_at = Utc::now();
        let hash2 = PgComplianceAuditRepository::calculate_hash(
            "kyc_tier_change",
            Some("user123"),
            &serde_json::json!({"old_tier": 1, "new_tier": 2}),
            Some("user123"),
            &created_at,
            Some(&hash1),
        );

        let hash3 = PgComplianceAuditRepository::calculate_hash(
            "kyc_tier_change",
            Some("user123"),
            &serde_json::json!({"old_tier": 1, "new_tier": 2}),
            Some("user123"),
            &created_at,
            Some(&hash1),
        );

        assert_eq!(hash2, hash3);
    }
}
