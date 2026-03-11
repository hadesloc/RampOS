use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportPortalSummary {
    pub package_id: String,
    pub source_tenant_id: String,
    pub status: String,
    pub consent_status: String,
    pub destination_tenant_id: Option<String>,
    pub fields_shared: Vec<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub reuse_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportQueueItem {
    pub package_id: String,
    pub user_id: String,
    pub source_tenant_id: String,
    pub target_tenant_id: String,
    pub status: String,
    pub consent_status: String,
    pub review_status: String,
    pub fields_shared: Vec<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportPackageDetail {
    pub package_id: String,
    pub user_id: String,
    pub source_tenant_id: String,
    pub target_tenant_id: String,
    pub status: String,
    pub consent_status: String,
    pub review_status: String,
    pub fields_shared: Vec<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub freshness_check_status: String,
    pub acceptance_policy: String,
}

pub fn passport_summary_from_flags(risk_flags: &Value) -> Option<PassportPortalSummary> {
    let passport = risk_flags.get("passport")?;
    let fields_shared = passport
        .get("fieldsShared")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(PassportPortalSummary {
        package_id: passport.get("packageId")?.as_str()?.to_string(),
        source_tenant_id: passport
            .get("sourceTenantId")
            .and_then(Value::as_str)
            .unwrap_or("unknown_source")
            .to_string(),
        status: passport
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("available")
            .to_string(),
        consent_status: passport
            .get("consentStatus")
            .and_then(Value::as_str)
            .unwrap_or("granted")
            .to_string(),
        destination_tenant_id: passport
            .get("destinationTenantId")
            .and_then(Value::as_str)
            .map(str::to_string),
        fields_shared,
        expires_at: passport
            .get("expiresAt")
            .and_then(Value::as_str)
            .map(str::to_string),
        revoked_at: passport
            .get("revokedAt")
            .and_then(Value::as_str)
            .map(str::to_string),
        reuse_allowed: passport
            .get("reuseAllowed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

pub struct PassportService;

impl PassportService {
    pub fn new() -> Self {
        Self
    }

    pub fn list_queue(&self, scenario: Option<&str>) -> Vec<PassportQueueItem> {
        sample_queue(scenario)
    }

    pub fn get_package(
        &self,
        package_id: &str,
        scenario: Option<&str>,
    ) -> Option<PassportPackageDetail> {
        sample_queue(scenario)
            .into_iter()
            .find(|item| item.package_id == package_id)
            .map(|item| PassportPackageDetail {
                package_id: item.package_id,
                user_id: item.user_id,
                source_tenant_id: item.source_tenant_id,
                target_tenant_id: item.target_tenant_id,
                status: item.status,
                consent_status: item.consent_status,
                review_status: item.review_status,
                fields_shared: item.fields_shared,
                expires_at: item.expires_at,
                revoked_at: item.revoked_at,
                freshness_check_status: "fresh".to_string(),
                acceptance_policy: "trusted_sources_only".to_string(),
            })
    }
}

impl Default for PassportService {
    fn default() -> Self {
        Self::new()
    }
}

fn sample_queue(scenario: Option<&str>) -> Vec<PassportQueueItem> {
    let now = Utc::now();
    if matches!(scenario, Some("revoked")) {
        return vec![PassportQueueItem {
            package_id: "pkg_passport_revoked_001".to_string(),
            user_id: "user_passport_001".to_string(),
            source_tenant_id: "tenant_origin".to_string(),
            target_tenant_id: "tenant_review".to_string(),
            status: "revoked".to_string(),
            consent_status: "revoked".to_string(),
            review_status: "closed".to_string(),
            fields_shared: vec!["identity".to_string(), "address".to_string()],
            expires_at: Some((now + Duration::days(10)).to_rfc3339()),
            revoked_at: Some((now - Duration::days(1)).to_rfc3339()),
        }];
    }

    vec![
        PassportQueueItem {
            package_id: "pkg_passport_active_001".to_string(),
            user_id: "user_passport_001".to_string(),
            source_tenant_id: "tenant_origin".to_string(),
            target_tenant_id: "tenant_review".to_string(),
            status: "available".to_string(),
            consent_status: "granted".to_string(),
            review_status: "pending_review".to_string(),
            fields_shared: vec!["identity".to_string(), "sanctions".to_string()],
            expires_at: Some((now + Duration::days(30)).to_rfc3339()),
            revoked_at: None,
        },
        PassportQueueItem {
            package_id: "pkg_passport_active_002".to_string(),
            user_id: "user_passport_002".to_string(),
            source_tenant_id: "tenant_origin".to_string(),
            target_tenant_id: "tenant_partner".to_string(),
            status: "freshness_check_due".to_string(),
            consent_status: "granted".to_string(),
            review_status: "needs_revalidation".to_string(),
            fields_shared: vec!["identity".to_string(), "tier".to_string()],
            expires_at: Some((now + Duration::days(7)).to_rfc3339()),
            revoked_at: None,
        },
    ]
}
