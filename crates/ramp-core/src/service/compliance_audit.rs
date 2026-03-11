//! Compliance Audit Service
//!
//! Service layer for compliance audit trail operations.
//! Provides high-level APIs for logging compliance events and verifying integrity.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use ramp_common::{types::TenantId, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::repository::compliance_audit::{
    ActorType, AuditQueryFilter, ChainVerificationResult, ComplianceAuditEntry,
    ComplianceAuditRepository, ComplianceEventType, CreateComplianceAuditRequest,
};

/// Context for audit logging (extracted from request)
#[derive(Debug, Clone, Default)]
pub struct AuditContext {
    pub actor_id: Option<String>,
    pub actor_type: ActorType,
    pub ip_address: Option<std::net::IpAddr>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
}

impl AuditContext {
    pub fn system() -> Self {
        Self {
            actor_id: None,
            actor_type: ActorType::System,
            ip_address: None,
            user_agent: None,
            request_id: None,
        }
    }

    pub fn admin(admin_id: &str) -> Self {
        Self {
            actor_id: Some(admin_id.to_string()),
            actor_type: ActorType::Admin,
            ip_address: None,
            user_agent: None,
            request_id: None,
        }
    }

    pub fn user(user_id: &str) -> Self {
        Self {
            actor_id: Some(user_id.to_string()),
            actor_type: ActorType::User,
            ip_address: None,
            user_agent: None,
            request_id: None,
        }
    }

    pub fn api(api_key_id: &str) -> Self {
        Self {
            actor_id: Some(api_key_id.to_string()),
            actor_type: ActorType::Api,
            ip_address: None,
            user_agent: None,
            request_id: None,
        }
    }

    pub fn with_ip(mut self, ip: std::net::IpAddr) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn with_user_agent(mut self, ua: &str) -> Self {
        self.user_agent = Some(ua.to_string());
        self
    }

    pub fn with_request_id(mut self, rid: &str) -> Self {
        self.request_id = Some(rid.to_string());
        self
    }
}

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Csv,
}

/// Exported audit log with metadata
#[derive(Debug, Clone, Serialize)]
pub struct AuditLogExport {
    pub tenant_id: String,
    pub exported_at: DateTime<Utc>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub total_entries: usize,
    pub chain_verified: bool,
    pub entries: Vec<ComplianceAuditEntry>,
}

/// Compliance Audit Service
pub struct ComplianceAuditService {
    repo: Arc<dyn ComplianceAuditRepository>,
}

impl ComplianceAuditService {
    pub fn new(repo: Arc<dyn ComplianceAuditRepository>) -> Self {
        Self { repo }
    }

    /// Log a compliance event
    pub async fn log_compliance_event(
        &self,
        tenant_id: &TenantId,
        event_type: ComplianceEventType,
        action_details: serde_json::Value,
        resource_type: Option<&str>,
        resource_id: Option<&str>,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        info!(
            tenant = %tenant_id.0,
            event_type = ?event_type,
            actor = ?context.actor_id,
            resource = ?resource_id,
            "Logging compliance event"
        );

        let request = CreateComplianceAuditRequest {
            event_type,
            actor_id: context.actor_id,
            actor_type: context.actor_type,
            action_details,
            resource_type: resource_type.map(String::from),
            resource_id: resource_id.map(String::from),
            ip_address: context.ip_address,
            user_agent: context.user_agent,
            request_id: context.request_id,
        };

        self.repo.log_event(tenant_id, request).await
    }

    /// Log KYC tier change
    pub async fn log_kyc_tier_change(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        old_tier: i16,
        new_tier: i16,
        reason: &str,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::KycTierChange,
            serde_json::json!({
                "old_tier": old_tier,
                "new_tier": new_tier,
                "reason": reason,
                "user_id": user_id
            }),
            Some("user"),
            Some(user_id),
            context,
        )
        .await
    }

    /// Log transaction approval
    pub async fn log_transaction_approval(
        &self,
        tenant_id: &TenantId,
        intent_id: &str,
        amount: &str,
        currency: &str,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::TransactionApproval,
            serde_json::json!({
                "intent_id": intent_id,
                "amount": amount,
                "currency": currency,
                "approved_at": Utc::now().to_rfc3339()
            }),
            Some("intent"),
            Some(intent_id),
            context,
        )
        .await
    }

    /// Log transaction rejection
    pub async fn log_transaction_rejection(
        &self,
        tenant_id: &TenantId,
        intent_id: &str,
        reason: &str,
        rule_id: Option<&str>,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::TransactionRejection,
            serde_json::json!({
                "intent_id": intent_id,
                "reason": reason,
                "rule_id": rule_id,
                "rejected_at": Utc::now().to_rfc3339()
            }),
            Some("intent"),
            Some(intent_id),
            context,
        )
        .await
    }

    /// Log AML rule modification
    pub async fn log_aml_rule_modification(
        &self,
        tenant_id: &TenantId,
        rule_id: &str,
        action: &str,
        old_config: Option<serde_json::Value>,
        new_config: Option<serde_json::Value>,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::AmlRuleModification,
            serde_json::json!({
                "rule_id": rule_id,
                "action": action,
                "old_config": old_config,
                "new_config": new_config,
                "modified_at": Utc::now().to_rfc3339()
            }),
            Some("aml_rule"),
            Some(rule_id),
            context,
        )
        .await
    }

    /// Log SAR submission
    pub async fn log_sar_submission(
        &self,
        tenant_id: &TenantId,
        case_id: &str,
        sar_id: &str,
        user_id: Option<&str>,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::SarSubmission,
            serde_json::json!({
                "case_id": case_id,
                "sar_id": sar_id,
                "user_id": user_id,
                "submitted_at": Utc::now().to_rfc3339()
            }),
            Some("sar"),
            Some(sar_id),
            context,
        )
        .await
    }

    /// Log CTR submission
    pub async fn log_ctr_submission(
        &self,
        tenant_id: &TenantId,
        ctr_id: &str,
        transaction_ids: Vec<String>,
        total_amount: &str,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::CtrSubmission,
            serde_json::json!({
                "ctr_id": ctr_id,
                "transaction_ids": transaction_ids,
                "total_amount": total_amount,
                "submitted_at": Utc::now().to_rfc3339()
            }),
            Some("ctr"),
            Some(ctr_id),
            context,
        )
        .await
    }

    /// Log license status change
    pub async fn log_license_status_change(
        &self,
        tenant_id: &TenantId,
        license_id: &str,
        old_status: &str,
        new_status: &str,
        reason: &str,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::LicenseStatusChange,
            serde_json::json!({
                "license_id": license_id,
                "old_status": old_status,
                "new_status": new_status,
                "reason": reason,
                "changed_at": Utc::now().to_rfc3339()
            }),
            Some("license"),
            Some(license_id),
            context,
        )
        .await
    }

    pub async fn log_passport_consent_granted(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        package_id: &str,
        target_tenant_id: &str,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::PassportConsentGranted,
            serde_json::json!({
                "user_id": user_id,
                "package_id": package_id,
                "target_tenant_id": target_tenant_id,
                "granted_at": Utc::now().to_rfc3339()
            }),
            Some("passport_package"),
            Some(package_id),
            context,
        )
        .await
    }

    pub async fn log_passport_consent_revoked(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        package_id: &str,
        target_tenant_id: &str,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::PassportConsentRevoked,
            serde_json::json!({
                "user_id": user_id,
                "package_id": package_id,
                "target_tenant_id": target_tenant_id,
                "revoked_at": Utc::now().to_rfc3339()
            }),
            Some("passport_package"),
            Some(package_id),
            context,
        )
        .await
    }

    /// Log sanctions check result
    pub async fn log_sanctions_check(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        result: &str,
        matched_lists: Vec<String>,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::SanctionsCheck,
            serde_json::json!({
                "user_id": user_id,
                "result": result,
                "matched_lists": matched_lists,
                "checked_at": Utc::now().to_rfc3339()
            }),
            Some("user"),
            Some(user_id),
            context,
        )
        .await
    }

    /// Log PEP check result
    pub async fn log_pep_check(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        result: &str,
        pep_details: Option<serde_json::Value>,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::PepCheck,
            serde_json::json!({
                "user_id": user_id,
                "result": result,
                "pep_details": pep_details,
                "checked_at": Utc::now().to_rfc3339()
            }),
            Some("user"),
            Some(user_id),
            context,
        )
        .await
    }

    /// Log a completed rescreening run.
    pub async fn log_rescreening_run_completed(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        trigger_kind: &str,
        outcome: &str,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::RescreeningRunCompleted,
            serde_json::json!({
                "user_id": user_id,
                "trigger_kind": trigger_kind,
                "outcome": outcome,
                "completed_at": Utc::now().to_rfc3339()
            }),
            Some("user"),
            Some(user_id),
            context,
        )
        .await
    }

    /// Log a queued rescreening alert.
    pub async fn log_rescreening_alert_queued(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        priority: &str,
        alert_codes: &[String],
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::RescreeningAlertQueued,
            serde_json::json!({
                "user_id": user_id,
                "priority": priority,
                "alert_codes": alert_codes,
                "queued_at": Utc::now().to_rfc3339()
            }),
            Some("user"),
            Some(user_id),
            context,
        )
        .await
    }

    /// Log an applied rescreening restriction.
    pub async fn log_rescreening_restriction_applied(
        &self,
        tenant_id: &TenantId,
        user_id: &str,
        restriction_status: &str,
        reason: &str,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::RescreeningRestrictionApplied,
            serde_json::json!({
                "user_id": user_id,
                "restriction_status": restriction_status,
                "reason": reason,
                "applied_at": Utc::now().to_rfc3339()
            }),
            Some("user"),
            Some(user_id),
            context,
        )
        .await
    }

    /// Log Travel Rule policy evaluation.
    pub async fn log_travel_rule_policy_evaluation(
        &self,
        tenant_id: &TenantId,
        disclosure_id: &str,
        policy_code: Option<&str>,
        action: &str,
        direction: &str,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::TravelRulePolicyEvaluated,
            serde_json::json!({
                "disclosure_id": disclosure_id,
                "policy_code": policy_code,
                "action": action,
                "direction": direction,
                "evaluated_at": Utc::now().to_rfc3339()
            }),
            Some("travel_rule_disclosure"),
            Some(disclosure_id),
            context,
        )
        .await
    }

    /// Log Travel Rule disclosure state updates.
    pub async fn log_travel_rule_disclosure_update(
        &self,
        tenant_id: &TenantId,
        disclosure_id: &str,
        stage: &str,
        transport_profile: Option<&str>,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::TravelRuleDisclosureUpdated,
            serde_json::json!({
                "disclosure_id": disclosure_id,
                "stage": stage,
                "transport_profile": transport_profile,
                "updated_at": Utc::now().to_rfc3339()
            }),
            Some("travel_rule_disclosure"),
            Some(disclosure_id),
            context,
        )
        .await
    }

    /// Log Travel Rule exception queueing.
    pub async fn log_travel_rule_exception_queued(
        &self,
        tenant_id: &TenantId,
        exception_id: &str,
        disclosure_id: &str,
        reason_code: &str,
        context: AuditContext,
    ) -> Result<ComplianceAuditEntry> {
        self.log_compliance_event(
            tenant_id,
            ComplianceEventType::TravelRuleExceptionQueued,
            serde_json::json!({
                "exception_id": exception_id,
                "disclosure_id": disclosure_id,
                "reason_code": reason_code,
                "queued_at": Utc::now().to_rfc3339()
            }),
            Some("travel_rule_exception"),
            Some(exception_id),
            context,
        )
        .await
    }

    /// Verify the hash chain integrity
    pub async fn verify_chain(&self, tenant_id: &TenantId) -> Result<ChainVerificationResult> {
        info!(tenant = %tenant_id.0, "Verifying audit chain integrity");

        let result = self.repo.verify_chain(tenant_id).await?;

        if result.is_valid {
            info!(
                tenant = %tenant_id.0,
                entries = result.verified_entries,
                "Audit chain verified successfully"
            );
        } else {
            warn!(
                tenant = %tenant_id.0,
                error = ?result.error_message,
                sequence = ?result.first_invalid_sequence,
                "Audit chain verification FAILED"
            );
        }

        Ok(result)
    }

    /// Export audit log for regulators
    pub async fn export_audit_log(
        &self,
        tenant_id: &TenantId,
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
    ) -> Result<AuditLogExport> {
        info!(
            tenant = %tenant_id.0,
            from = ?from_date,
            to = ?to_date,
            "Exporting audit log for regulators"
        );

        // Verify chain before export
        let verification = self.repo.verify_chain(tenant_id).await?;

        // Get entries
        let entries = self
            .repo
            .export_audit_log(tenant_id, from_date, to_date)
            .await?;

        Ok(AuditLogExport {
            tenant_id: tenant_id.0.clone(),
            exported_at: Utc::now(),
            from_date,
            to_date,
            total_entries: entries.len(),
            chain_verified: verification.is_valid,
            entries,
        })
    }

    /// List audit entries with filtering
    pub async fn list_entries(
        &self,
        tenant_id: &TenantId,
        filter: AuditQueryFilter,
    ) -> Result<(Vec<ComplianceAuditEntry>, i64)> {
        let entries = self.repo.get_entries(tenant_id, filter.clone()).await?;
        let total = self.repo.count_entries(tenant_id, filter).await?;

        Ok((entries, total))
    }

    /// Convert export to CSV format
    pub fn export_to_csv(export: &AuditLogExport) -> String {
        let mut csv = String::from(
            "id,tenant_id,event_type,actor_id,actor_type,resource_type,resource_id,sequence_number,created_at,current_hash\n"
        );

        for entry in &export.entries {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                entry.id,
                entry.tenant_id,
                entry.event_type.as_str(),
                entry.actor_id.as_deref().unwrap_or(""),
                entry.actor_type.as_str(),
                entry.resource_type.as_deref().unwrap_or(""),
                entry.resource_id.as_deref().unwrap_or(""),
                entry.sequence_number,
                entry.created_at.to_rfc3339(),
                entry.current_hash
            ));
        }

        csv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    struct MockComplianceAuditRepository {
        requests: Mutex<Vec<CreateComplianceAuditRequest>>,
    }

    #[async_trait]
    impl ComplianceAuditRepository for MockComplianceAuditRepository {
        async fn log_event(
            &self,
            tenant_id: &TenantId,
            request: CreateComplianceAuditRequest,
        ) -> Result<ComplianceAuditEntry> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(ComplianceAuditEntry {
                id: uuid::Uuid::new_v4(),
                tenant_id: tenant_id.0.clone(),
                event_type: request.event_type,
                actor_id: request.actor_id,
                actor_type: request.actor_type,
                action_details: request.action_details,
                resource_type: request.resource_type,
                resource_id: request.resource_id,
                sequence_number: 1,
                previous_hash: None,
                current_hash: "hash".to_string(),
                ip_address: request.ip_address.map(|ip| ip.to_string()),
                user_agent: request.user_agent,
                request_id: request.request_id,
                created_at: Utc::now(),
            })
        }

        async fn get_entries(
            &self,
            _tenant_id: &TenantId,
            _filter: AuditQueryFilter,
        ) -> Result<Vec<ComplianceAuditEntry>> {
            Ok(vec![])
        }

        async fn count_entries(
            &self,
            _tenant_id: &TenantId,
            _filter: AuditQueryFilter,
        ) -> Result<i64> {
            Ok(0)
        }

        async fn verify_chain(&self, _tenant_id: &TenantId) -> Result<ChainVerificationResult> {
            Ok(ChainVerificationResult {
                is_valid: true,
                total_entries: 0,
                verified_entries: 0,
                first_invalid_sequence: None,
                error_message: None,
            })
        }

        async fn export_audit_log(
            &self,
            _tenant_id: &TenantId,
            _from_date: Option<DateTime<Utc>>,
            _to_date: Option<DateTime<Utc>>,
        ) -> Result<Vec<ComplianceAuditEntry>> {
            Ok(vec![])
        }

        async fn get_latest_entry(
            &self,
            _tenant_id: &TenantId,
        ) -> Result<Option<ComplianceAuditEntry>> {
            Ok(None)
        }
    }

    #[test]
    fn test_audit_context_builder() {
        let ctx = AuditContext::admin("admin123").with_request_id("req_abc");

        assert_eq!(ctx.actor_id, Some("admin123".to_string()));
        assert_eq!(ctx.actor_type, ActorType::Admin);
        assert_eq!(ctx.request_id, Some("req_abc".to_string()));
    }

    #[test]
    fn test_csv_export() {
        let export = AuditLogExport {
            tenant_id: "tenant1".to_string(),
            exported_at: Utc::now(),
            from_date: None,
            to_date: None,
            total_entries: 1,
            chain_verified: true,
            entries: vec![ComplianceAuditEntry {
                id: uuid::Uuid::new_v4(),
                tenant_id: "tenant1".to_string(),
                event_type: ComplianceEventType::KycTierChange,
                actor_id: Some("admin1".to_string()),
                actor_type: ActorType::Admin,
                action_details: serde_json::json!({}),
                resource_type: Some("user".to_string()),
                resource_id: Some("user123".to_string()),
                sequence_number: 1,
                previous_hash: None,
                current_hash: "abc123".to_string(),
                ip_address: None,
                user_agent: None,
                request_id: None,
                created_at: Utc::now(),
            }],
        };

        let csv = ComplianceAuditService::export_to_csv(&export);
        assert!(csv.contains("kyc_tier_change"));
        assert!(csv.contains("admin1"));
        assert!(csv.contains("user123"));
    }

    #[tokio::test]
    async fn test_travel_rule_helpers_emit_expected_event_types() {
        let repo = Arc::new(MockComplianceAuditRepository {
            requests: Mutex::new(vec![]),
        });
        let service = ComplianceAuditService::new(repo.clone());
        let tenant_id = TenantId::new("tenant_travel_rule_audit");

        service
            .log_travel_rule_policy_evaluation(
                &tenant_id,
                "trd_001",
                Some("fatf-default"),
                "DISCLOSE_BEFORE_SETTLEMENT",
                "OUTBOUND",
                AuditContext::system(),
            )
            .await
            .unwrap();
        service
            .log_travel_rule_disclosure_update(
                &tenant_id,
                "trd_001",
                "SENT",
                Some("TRISA"),
                AuditContext::system(),
            )
            .await
            .unwrap();
        service
            .log_travel_rule_exception_queued(
                &tenant_id,
                "tre_001",
                "trd_001",
                "COUNTERPARTY_UNAVAILABLE",
                AuditContext::system(),
            )
            .await
            .unwrap();

        let requests = repo.requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(
            requests[0].event_type,
            ComplianceEventType::TravelRulePolicyEvaluated
        );
        assert_eq!(
            requests[1].event_type,
            ComplianceEventType::TravelRuleDisclosureUpdated
        );
        assert_eq!(
            requests[2].event_type,
            ComplianceEventType::TravelRuleExceptionQueued
        );
    }
}
