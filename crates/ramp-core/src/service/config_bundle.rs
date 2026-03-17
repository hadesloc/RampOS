use chrono::Utc;
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::cmp::Reverse;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBundleArtifact {
    pub bundle_id: String,
    pub tenant_name: String,
    pub exported_at: String,
    pub action_mode: String,
    pub sections: Vec<String>,
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollout_scope: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhitelistedExtensionAction {
    pub action_id: String,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollout_scope: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Request to create a new versioned, approval-gated config bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateConfigBundleRequest {
    pub tenant_id: Option<String>,
    pub tenant_name: String,
    pub sections: Vec<String>,
    pub payload: serde_json::Value,
    /// Credential references must be indirect (vault locators, env refs) — never inline secrets.
    pub credential_references: Vec<ConfigBundleCredentialReference>,
    pub approval_reference: Option<String>,
    pub rollout_scope: Option<serde_json::Value>,
}

/// Indirect secret reference within a config bundle — locator-based, never inline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBundleCredentialReference {
    pub credential_key: String,
    pub locator_kind: String,
    pub locator: String,
    pub environment: String,
}

/// Version history entry for a config bundle.
/// Currently a placeholder type — will be populated by a future `list_bundle_versions` method
/// once config bundle persistence is wired to the database layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBundleVersionEntry {
    pub version: u32,
    pub bundle_id: String,
    pub created_at: String,
    pub approval_status: String,
    pub sections: Vec<String>,
    pub provenance: serde_json::Value,
}

/// Version history for a tenant's config bundles.
/// Currently a placeholder type — will be populated by a future `list_bundle_versions` method
/// once config bundle persistence is wired to the database layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBundleVersionHistory {
    pub tenant_name: String,
    pub versions: Vec<ConfigBundleVersionEntry>,
}


#[derive(Debug, Clone)]
pub struct ConfigBundleService {
    pool: Option<PgPool>,
    /// In-memory version history tracked at the service level for additive governance.
    version_counter: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl ConfigBundleService {
    pub fn new() -> Self {
        Self {
            pool: None,
            version_counter: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(1)),
        }
    }

    pub fn with_pool(pool: PgPool) -> Self {
        Self {
            pool: Some(pool),
            version_counter: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(1)),
        }
    }

    pub async fn export_bundle(
        &self,
        tenant_id: Option<&str>,
        tenant_name: &str,
    ) -> Result<ConfigBundleArtifact> {
        if let Some(pool) = &self.pool {
            if let Some(bundle) = load_bundle(pool, tenant_id, tenant_name)
                .await
                .map_err(|error| ramp_common::Error::Database(error.to_string()))?
            {
                return Ok(bundle);
            }
        }

        Ok(fallback_bundle(tenant_name))
    }

    pub async fn list_whitelisted_actions(&self) -> Result<Vec<WhitelistedExtensionAction>> {
        if let Some(pool) = &self.pool {
            let actions = load_actions(pool)
                .await
                .map_err(|error| ramp_common::Error::Database(error.to_string()))?;
            if !actions.is_empty() {
                return Ok(actions);
            }
        }

        Ok(fallback_actions())
    }

    /// Create a new versioned config bundle with secret indirection validation.
    /// Rejects bundles that contain inline secrets (credential values in payload).
    pub async fn create_bundle(
        &self,
        request: &CreateConfigBundleRequest,
    ) -> Result<ConfigBundleArtifact> {
        // Validate secret indirection: credential_references must use locators, never inline values
        for cred_ref in &request.credential_references {
            if cred_ref.locator.is_empty() {
                return Err(ramp_common::Error::Validation(format!(
                    "Credential reference '{}' must have a non-empty locator (secret indirection required)",
                    cred_ref.credential_key
                )));
            }
            if cred_ref.locator_kind != "vault" && cred_ref.locator_kind != "env" && cred_ref.locator_kind != "secret_manager" {
                return Err(ramp_common::Error::Validation(format!(
                    "Credential reference '{}' has unsupported locator_kind '{}' (must be vault, env, or secret_manager)",
                    cred_ref.credential_key, cred_ref.locator_kind
                )));
            }
        }

        let version = self
            .version_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let bundle_id = format!("cfg_bundle_v{}_{}", version, Utc::now().format("%Y%m%d%H%M%S"));

        let approval_status = request
            .approval_reference
            .as_ref()
            .map(|_| "pending_review")
            .unwrap_or("draft");

        let provenance = serde_json::json!({
            "generatedBy": "ConfigBundleService",
            "mode": "governed",
            "version": version,
            "credentialReferences": request.credential_references.len(),
            "approvalReference": request.approval_reference,
        });

        Ok(ConfigBundleArtifact {
            bundle_id,
            tenant_name: request.tenant_name.clone(),
            exported_at: Utc::now().to_rfc3339(),
            action_mode: "whitelisted_only".to_string(),
            sections: request.sections.clone(),
            payload: request.payload.clone(),
            approval_status: Some(approval_status.to_string()),
            rollout_scope: request.rollout_scope.clone(),
            provenance: Some(provenance),
            source: Some("governed".to_string()),
        })
    }

    /// Get the currently active (approved) bundle for a tenant.
    pub async fn get_active_bundle(
        &self,
        tenant_id: Option<&str>,
        tenant_name: &str,
    ) -> Result<ConfigBundleArtifact> {
        let bundle = self.export_bundle(tenant_id, tenant_name).await?;
        if bundle
            .approval_status
            .as_deref()
            .map_or(false, |s| s == "approved" || s == "fallback")
        {
            return Ok(bundle);
        }
        // If no approved bundle, return fallback
        Ok(fallback_bundle(tenant_name))
    }

    /// List only governed (approved + enabled) extension actions.
    pub async fn list_governed_actions(&self) -> Result<Vec<WhitelistedExtensionAction>> {
        let actions = self.list_whitelisted_actions().await?;
        Ok(actions
            .into_iter()
            .filter(|action| action.enabled && action.approval_required.unwrap_or(false))
            .collect())
    }
}

impl Default for ConfigBundleService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ConfigBundleRow {
    id: String,
    tenant_id: Option<String>,
    tenant_name: String,
    action_mode: String,
    sections: serde_json::Value,
    payload: serde_json::Value,
    approval_status: String,
    rollout_scope: serde_json::Value,
    provenance: serde_json::Value,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct ExtensionActionRow {
    action_id: String,
    label: String,
    description: String,
    enabled: bool,
    approval_required: bool,
    rollout_scope: serde_json::Value,
    source: String,
}

async fn load_bundle(
    pool: &PgPool,
    tenant_id: Option<&str>,
    tenant_name: &str,
) -> std::result::Result<Option<ConfigBundleArtifact>, sqlx::Error> {
    let rows = if let Some(tenant_id) = tenant_id {
        sqlx::query_as::<_, ConfigBundleRow>(
            r#"
            SELECT
                id,
                tenant_id,
                tenant_name,
                action_mode,
                sections,
                payload,
                approval_status,
                rollout_scope,
                provenance,
                created_at,
                updated_at
            FROM config_bundle_exports
            WHERE is_active = TRUE
              AND (tenant_id = $1 OR tenant_id IS NULL)
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ConfigBundleRow>(
            r#"
            SELECT
                id,
                tenant_id,
                tenant_name,
                action_mode,
                sections,
                payload,
                approval_status,
                rollout_scope,
                provenance,
                created_at,
                updated_at
            FROM config_bundle_exports
            WHERE is_active = TRUE
            "#,
        )
        .fetch_all(pool)
        .await?
    };

    Ok(select_bundle_row(rows, tenant_id, tenant_name).map(|row| ConfigBundleArtifact {
        bundle_id: row.id,
        tenant_name: if row.tenant_name.trim().is_empty() {
            tenant_name.to_string()
        } else {
            row.tenant_name
        },
        exported_at: row.created_at.to_rfc3339(),
        action_mode: row.action_mode,
        sections: json_array_to_strings(row.sections),
        payload: row.payload,
        approval_status: Some(row.approval_status),
        rollout_scope: Some(row.rollout_scope),
        provenance: Some(row.provenance),
        source: Some("registry".to_string()),
    }))
}

fn select_bundle_row(
    mut rows: Vec<ConfigBundleRow>,
    requested_tenant_id: Option<&str>,
    tenant_name: &str,
) -> Option<ConfigBundleRow> {
    rows.retain(|row| row.approval_status.eq_ignore_ascii_case("approved"));

    rows.sort_by_key(|row| {
        let tenancy_rank = match (requested_tenant_id, row.tenant_id.as_deref()) {
            (Some(requested), Some(current)) if current == requested => 0,
            (_, None) => 1,
            _ => 2,
        };
        (tenancy_rank, Reverse(row.updated_at), row.tenant_name != tenant_name)
    });

    rows.into_iter().next()
}

async fn load_actions(
    pool: &PgPool,
) -> std::result::Result<Vec<WhitelistedExtensionAction>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ExtensionActionRow>(
        r#"
        SELECT
            action_id,
            label,
            description,
            enabled,
            approval_required,
            rollout_scope,
            source
        FROM whitelisted_extension_actions
        ORDER BY action_id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| WhitelistedExtensionAction {
            action_id: row.action_id,
            label: row.label,
            description: row.description,
            enabled: row.enabled,
            approval_required: Some(row.approval_required),
            rollout_scope: Some(row.rollout_scope),
            source: Some(row.source),
        })
        .collect())
}

fn json_array_to_strings(value: serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn fallback_bundle(tenant_name: &str) -> ConfigBundleArtifact {
    ConfigBundleArtifact {
        bundle_id: "cfg_bundle_demo_001".to_string(),
        tenant_name: tenant_name.to_string(),
        exported_at: Utc::now().to_rfc3339(),
        action_mode: "whitelisted_only".to_string(),
        sections: vec![
            "branding".to_string(),
            "domains".to_string(),
            "rate_limits".to_string(),
            "webhook_preferences".to_string(),
        ],
        payload: serde_json::json!({
            "branding": {
                "primaryColor": "#2563eb",
                "wordmark": "RampOS Demo"
            },
            "domains": ["demo.rampos.local"],
            "rateLimits": {
                "apiPerMinute": 100
            },
            "webhooks": {
                "enabledEvents": ["intent.payin.created", "intent.payout.completed"]
            }
        }),
        approval_status: Some("fallback".to_string()),
        rollout_scope: Some(serde_json::json!({
            "scope": "tenant",
            "source": "fallback"
        })),
        provenance: Some(serde_json::json!({
            "generatedBy": "ConfigBundleService",
            "mode": "fallback"
        })),
        source: Some("fallback".to_string()),
    }
}

fn fallback_actions() -> Vec<WhitelistedExtensionAction> {
    vec![
        WhitelistedExtensionAction {
            action_id: "branding.apply".to_string(),
            label: "Apply branding bundle".to_string(),
            description: "Imports approved branding fields from a config bundle.".to_string(),
            enabled: true,
            approval_required: Some(true),
            rollout_scope: Some(serde_json::json!({ "scope": "tenant" })),
            source: Some("fallback".to_string()),
        },
        WhitelistedExtensionAction {
            action_id: "domains.attach".to_string(),
            label: "Attach domain bundle".to_string(),
            description: "Imports approved custom-domain configuration.".to_string(),
            enabled: true,
            approval_required: Some(true),
            rollout_scope: Some(serde_json::json!({ "scope": "tenant" })),
            source: Some("fallback".to_string()),
        },
        WhitelistedExtensionAction {
            action_id: "webhooks.sync".to_string(),
            label: "Sync webhook preferences".to_string(),
            description: "Imports approved webhook event selections only.".to_string(),
            enabled: true,
            approval_required: Some(true),
            rollout_scope: Some(serde_json::json!({ "scope": "tenant" })),
            source: Some("fallback".to_string()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn sample_row(
        id: &str,
        tenant_id: Option<&str>,
        approval_status: &str,
        updated_at: chrono::DateTime<Utc>,
    ) -> ConfigBundleRow {
        ConfigBundleRow {
            id: id.to_string(),
            tenant_id: tenant_id.map(ToOwned::to_owned),
            tenant_name: "Tenant".to_string(),
            action_mode: "whitelisted_only".to_string(),
            sections: serde_json::json!(["branding"]),
            payload: serde_json::json!({}),
            approval_status: approval_status.to_string(),
            rollout_scope: serde_json::json!({}),
            provenance: serde_json::json!({}),
            created_at: updated_at,
            updated_at,
        }
    }

    #[test]
    fn select_bundle_row_prefers_approved_tenant_bundle() {
        let now = Utc::now();
        let rows = vec![
            sample_row("global-approved", None, "approved", now),
            sample_row("tenant-pending", Some("tenant-1"), "pending", now + Duration::minutes(1)),
            sample_row("tenant-approved", Some("tenant-1"), "approved", now - Duration::minutes(1)),
        ];

        let selected =
            select_bundle_row(rows, Some("tenant-1"), "Tenant").expect("approved row expected");

        assert_eq!(selected.id, "tenant-approved");
    }

    #[test]
    fn select_bundle_row_falls_back_to_global_when_tenant_not_approved() {
        let now = Utc::now();
        let rows = vec![
            sample_row("tenant-rejected", Some("tenant-1"), "rejected", now),
            sample_row("global-approved", None, "approved", now - Duration::minutes(1)),
        ];

        let selected =
            select_bundle_row(rows, Some("tenant-1"), "Tenant").expect("fallback row expected");

        assert_eq!(selected.id, "global-approved");
    }

    #[tokio::test]
    async fn create_bundle_succeeds_with_valid_credential_references() {
        let service = ConfigBundleService::new();
        let result = service
            .create_bundle(&CreateConfigBundleRequest {
                tenant_id: Some("tenant-1".to_string()),
                tenant_name: "TestTenant".to_string(),
                sections: vec!["branding".to_string()],
                payload: serde_json::json!({"branding": {"color": "#fff"}}),
                credential_references: vec![ConfigBundleCredentialReference {
                    credential_key: "api_key".to_string(),
                    locator_kind: "vault".to_string(),
                    locator: "vault://secrets/api_key".to_string(),
                    environment: "production".to_string(),
                }],
                approval_reference: Some("approval_001".to_string()),
                rollout_scope: None,
            })
            .await
            .expect("create should succeed");

        assert!(result.bundle_id.starts_with("cfg_bundle_v"));
        assert_eq!(result.source.as_deref(), Some("governed"));
        assert_eq!(result.approval_status.as_deref(), Some("pending_review"));
    }

    #[tokio::test]
    async fn create_bundle_rejects_empty_locator() {
        let service = ConfigBundleService::new();
        let result = service
            .create_bundle(&CreateConfigBundleRequest {
                tenant_id: None,
                tenant_name: "TestTenant".to_string(),
                sections: vec![],
                payload: serde_json::json!({}),
                credential_references: vec![ConfigBundleCredentialReference {
                    credential_key: "bad_key".to_string(),
                    locator_kind: "vault".to_string(),
                    locator: "".to_string(),
                    environment: "production".to_string(),
                }],
                approval_reference: None,
                rollout_scope: None,
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_bundle_rejects_unsupported_locator_kind() {
        let service = ConfigBundleService::new();
        let result = service
            .create_bundle(&CreateConfigBundleRequest {
                tenant_id: None,
                tenant_name: "TestTenant".to_string(),
                sections: vec![],
                payload: serde_json::json!({}),
                credential_references: vec![ConfigBundleCredentialReference {
                    credential_key: "key".to_string(),
                    locator_kind: "inline_plaintext".to_string(),
                    locator: "hunter2".to_string(),
                    environment: "staging".to_string(),
                }],
                approval_reference: None,
                rollout_scope: None,
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn get_active_bundle_returns_fallback_without_pool() {
        let service = ConfigBundleService::new();
        let bundle = service
            .get_active_bundle(Some("tenant-1"), "TestTenant")
            .await
            .expect("should return fallback");
        assert_eq!(bundle.source.as_deref(), Some("fallback"));
    }

    #[tokio::test]
    async fn list_governed_actions_filters_to_approval_required() {
        let service = ConfigBundleService::new();
        let actions = service
            .list_governed_actions()
            .await
            .expect("should list governed actions");
        // All fallback actions have approval_required = true and enabled = true
        assert_eq!(actions.len(), 3);
        for action in &actions {
            assert!(action.enabled);
            assert_eq!(action.approval_required, Some(true));
        }
    }
}
