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

#[derive(Debug, Clone)]
pub struct ConfigBundleService {
    pool: Option<PgPool>,
}

impl ConfigBundleService {
    pub fn new() -> Self {
        Self { pool: None }
    }

    pub fn with_pool(pool: PgPool) -> Self {
        Self { pool: Some(pool) }
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
}
