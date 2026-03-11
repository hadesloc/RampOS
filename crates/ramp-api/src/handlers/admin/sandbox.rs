use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::Arc;
use tracing::info;
use validator::Validate;

use crate::error::ApiError;
use crate::extract::ValidatedJson;
use ramp_core::service::{
    redact_replay_bundle, ReplayBundle, ReplayTimelineEntry, ReplayTimelineSource, SandboxPreset,
    SandboxResetResult, SandboxResetStrategy, SandboxScenarioRun, SandboxSeedRequest,
    SandboxSeedResult, SandboxService,
};

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SandboxPresetResponse {
    pub code: String,
    pub name: String,
    pub seed_package_version: String,
    pub default_scenarios: Vec<String>,
    pub metadata: Value,
    pub reset_strategy: String,
    pub reset_semantics: Value,
}

#[derive(Debug, Clone, Deserialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SandboxSeedTenantRequest {
    #[validate(length(min = 3, max = 120))]
    pub tenant_name: String,
    #[validate(length(min = 3, max = 64))]
    pub preset_code: String,
    #[validate(length(min = 3, max = 64))]
    pub scenario_code: Option<String>,
    #[serde(default = "empty_object")]
    pub config_overrides: Value,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SandboxSeedTenantResponse {
    pub tenant_id: String,
    pub tenant_name: String,
    pub tenant_status: String,
    pub preset_code: String,
    pub scenario_code: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Validate, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SandboxResetRequest {
    #[validate(length(min = 3, max = 120))]
    pub tenant_id: String,
    #[validate(length(min = 3, max = 64))]
    pub preset_code: Option<String>,
    #[validate(length(min = 3, max = 200))]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SandboxPlaceholderResponse {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SandboxReplayEntryResponse {
    pub sequence: usize,
    pub source: String,
    pub reference_id: String,
    pub occurred_at: String,
    pub label: String,
    pub status: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SandboxReplayBundleResponse {
    pub journey_id: String,
    pub generated_at: String,
    pub redaction_applied: bool,
    pub entries: Vec<SandboxReplayEntryResponse>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SandboxReplayExportResponse {
    pub format: String,
    pub file_name: String,
    pub content_type: String,
    pub redaction_applied: bool,
    pub bundle: Value,
}

pub fn routes() -> Router<Arc<SandboxService>> {
    Router::new()
        .route("/", get(list_presets))
        .route("/seed", post(seed_tenant))
        .route("/replay/:journey_id", get(get_replay_bundle))
        .route("/replay/:journey_id/export", get(export_replay_bundle))
        .route("/reset", post(reset_tenant))
        .route("/runs/:run_id", get(get_run_status))
}

pub async fn list_presets(
    headers: HeaderMap,
    State(sandbox_service): State<Arc<SandboxService>>,
) -> Result<Json<Vec<SandboxPresetResponse>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!("Listing sandbox presets");

    let presets = sandbox_service
        .list_presets()
        .into_iter()
        .map(SandboxPresetResponse::from)
        .collect();

    Ok(Json(presets))
}

pub async fn seed_tenant(
    headers: HeaderMap,
    State(sandbox_service): State<Arc<SandboxService>>,
    ValidatedJson(request): ValidatedJson<SandboxSeedTenantRequest>,
) -> Result<Json<SandboxSeedTenantResponse>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;
    info!(
        role = ?auth.role,
        tenant_name = %request.tenant_name,
        preset_code = %request.preset_code,
        "Seeding sandbox tenant"
    );

    let seeded = sandbox_service
        .seed_tenant(SandboxSeedRequest {
            tenant_name: request.tenant_name,
            preset_code: request.preset_code,
            scenario_code: request.scenario_code,
            config_overrides: request.config_overrides,
        })
        .await
        .map_err(map_sandbox_error)?;

    Ok(Json(SandboxSeedTenantResponse::from(seeded)))
}

pub async fn reset_tenant(
    headers: HeaderMap,
    State(sandbox_service): State<Arc<SandboxService>>,
    ValidatedJson(request): ValidatedJson<SandboxResetRequest>,
) -> Result<Json<SandboxPlaceholderResponse>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;
    info!(
        role = ?auth.role,
        tenant_id = %request.tenant_id,
        preset_code = ?request.preset_code,
        "Sandbox reset requested through service-backed bounded workflow"
    );

    let result = sandbox_service
        .reset_tenant(
            &request.tenant_id,
            request.preset_code.as_deref(),
            request.reason.as_deref(),
        )
        .map_err(map_sandbox_error)?;

    Ok(Json(SandboxPlaceholderResponse::from(result)))
}

pub async fn get_run_status(
    headers: HeaderMap,
    State(sandbox_service): State<Arc<SandboxService>>,
    Path(run_id): Path<String>,
) -> Result<Json<SandboxPlaceholderResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        run_id = %run_id,
        "Sandbox run status requested from service-backed bounded workflow"
    );

    let run = sandbox_service
        .get_run_status(&run_id)
        .map_err(map_sandbox_error)?;

    Ok(Json(SandboxPlaceholderResponse::from(run)))
}

pub async fn get_replay_bundle(
    headers: HeaderMap,
    State(sandbox_service): State<Arc<SandboxService>>,
    Path(journey_id): Path<String>,
) -> Result<Json<SandboxReplayBundleResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        journey_id = %journey_id,
        "Returning assembled replay bundle with redaction applied"
    );

    let bundle = redact_replay_bundle(
        &sandbox_service
            .replay_bundle(&journey_id)
            .map_err(map_sandbox_error)?,
    );
    Ok(Json(SandboxReplayBundleResponse::from(bundle)))
}

pub async fn export_replay_bundle(
    headers: HeaderMap,
    State(sandbox_service): State<Arc<SandboxService>>,
    Path(journey_id): Path<String>,
) -> Result<Json<SandboxReplayExportResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        journey_id = %journey_id,
        "Exporting assembled replay bundle with redaction applied"
    );

    let bundle = redact_replay_bundle(
        &sandbox_service
            .replay_bundle(&journey_id)
            .map_err(map_sandbox_error)?,
    );
    Ok(Json(SandboxReplayExportResponse::from(bundle)))
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

fn map_sandbox_error(err: ramp_common::Error) -> ApiError {
    match err {
        ramp_common::Error::NotFound(message) => ApiError::NotFound(message),
        ramp_common::Error::Validation(message) => ApiError::BadRequest(message),
        ramp_common::Error::NotImplemented(message) => ApiError::BadRequest(message),
        other => ApiError::from(other),
    }
}

impl From<SandboxPreset> for SandboxPresetResponse {
    fn from(value: SandboxPreset) -> Self {
        Self {
            code: value.code,
            name: value.name,
            seed_package_version: value.seed_package_version,
            default_scenarios: value.default_scenarios,
            metadata: value.metadata,
            reset_strategy: reset_strategy_label(&value.reset_strategy).to_string(),
            reset_semantics: value.reset_semantics,
        }
    }
}

impl From<SandboxSeedResult> for SandboxSeedTenantResponse {
    fn from(value: SandboxSeedResult) -> Self {
        Self {
            tenant_id: value.tenant.id,
            tenant_name: value.tenant.name,
            tenant_status: value.tenant.status,
            preset_code: value.preset.code,
            scenario_code: value.scenario_code,
            created_at: value.tenant.created_at.to_rfc3339(),
        }
    }
}

fn reset_strategy_label(strategy: &SandboxResetStrategy) -> &'static str {
    match strategy {
        SandboxResetStrategy::ResetToPreset => "RESET_TO_PRESET",
        SandboxResetStrategy::ResetScenarioData => "RESET_SCENARIO_DATA",
        SandboxResetStrategy::ResetRuntimeArtifacts => "RESET_RUNTIME_ARTIFACTS",
    }
}

fn replay_source_label(source: &ReplayTimelineSource) -> &'static str {
    match source {
        ReplayTimelineSource::Webhook => "webhook",
        ReplayTimelineSource::Settlement => "settlement",
        ReplayTimelineSource::Reconciliation => "reconciliation",
    }
}

impl From<SandboxResetResult> for SandboxPlaceholderResponse {
    fn from(value: SandboxResetResult) -> Self {
        Self {
            status: value.status,
            message: format!(
                "{} (preset: {}, strategy: {})",
                value.message,
                value.preset_code.unwrap_or_else(|| "default".to_string()),
                value.reset_strategy
            ),
        }
    }
}

impl From<SandboxScenarioRun> for SandboxPlaceholderResponse {
    fn from(value: SandboxScenarioRun) -> Self {
        Self {
            status: format!("{:?}", value.status).to_uppercase(),
            message: format!(
                "Run {} for preset {} / scenario {}",
                value.run_id, value.preset_code, value.scenario_code
            ),
        }
    }
}

impl From<ReplayTimelineEntry> for SandboxReplayEntryResponse {
    fn from(value: ReplayTimelineEntry) -> Self {
        Self {
            sequence: value.sequence,
            source: replay_source_label(&value.source).to_string(),
            reference_id: value.reference_id,
            occurred_at: value.occurred_at.to_rfc3339(),
            label: value.label,
            status: value.status,
            payload: value.payload,
        }
    }
}

impl From<ReplayBundle> for SandboxReplayBundleResponse {
    fn from(value: ReplayBundle) -> Self {
        Self {
            journey_id: value.journey_id,
            generated_at: value.generated_at.to_rfc3339(),
            redaction_applied: true,
            entries: value
                .entries
                .into_iter()
                .map(SandboxReplayEntryResponse::from)
                .collect(),
        }
    }
}

impl From<ReplayBundle> for SandboxReplayExportResponse {
    fn from(value: ReplayBundle) -> Self {
        let file_name = format!("{}_replay.json", value.journey_id);
        Self {
            format: "json".to_string(),
            file_name,
            content_type: "application/json".to_string(),
            redaction_applied: true,
            bundle: serde_json::to_value(value).expect("Replay bundle should serialize"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use ramp_core::repository::tenant::TenantRow;

    #[test]
    fn sandbox_preset_response_preserves_contract_fields() {
        let response = SandboxPresetResponse::from(SandboxPreset {
            code: "BASELINE".to_string(),
            name: "Baseline".to_string(),
            seed_package_version: "2026-03-08".to_string(),
            default_scenarios: vec!["PAYIN_BASELINE".to_string()],
            metadata: serde_json::json!({"supportsReplay": true}),
            reset_strategy: SandboxResetStrategy::ResetRuntimeArtifacts,
            reset_semantics: serde_json::json!({"dropRuntimeEvents": true}),
        });

        assert_eq!(response.code, "BASELINE");
        assert_eq!(response.reset_strategy, "RESET_RUNTIME_ARTIFACTS");
        assert_eq!(response.default_scenarios, vec!["PAYIN_BASELINE"]);
    }

    #[test]
    fn sandbox_seed_response_keeps_seeded_tenant_identity() {
        let response = SandboxSeedTenantResponse::from(SandboxSeedResult {
            tenant: sample_tenant_row(),
            preset: SandboxPreset {
                code: "BASELINE".to_string(),
                name: "Baseline".to_string(),
                seed_package_version: "2026-03-08".to_string(),
                default_scenarios: vec!["PAYIN_BASELINE".to_string()],
                metadata: serde_json::json!({}),
                reset_strategy: SandboxResetStrategy::ResetToPreset,
                reset_semantics: serde_json::json!({}),
            },
            scenario_code: Some("PAYIN_BASELINE".to_string()),
        });

        assert_eq!(response.tenant_id, "tenant_sandbox_001");
        assert_eq!(response.preset_code, "BASELINE");
        assert_eq!(response.scenario_code.as_deref(), Some("PAYIN_BASELINE"));
    }

    #[test]
    fn replay_export_response_serializes_redacted_bundle_contract() {
        let response = SandboxReplayExportResponse::from(ReplayBundle {
            journey_id: "intent_sandbox_001".to_string(),
            generated_at: Utc::now(),
            entries: vec![ReplayTimelineEntry {
                sequence: 1,
                source: ReplayTimelineSource::Webhook,
                reference_id: "evt_intent_sandbox_001".to_string(),
                occurred_at: Utc::now(),
                label: "Replay entry".to_string(),
                status: "DELIVERED".to_string(),
                payload: serde_json::json!({
                    "apiSecret": "[REDACTED]",
                    "headers": {
                        "Authorization": "[REDACTED]"
                    }
                }),
            }],
        });

        assert_eq!(response.format, "json");
        assert_eq!(response.file_name, "intent_sandbox_001_replay.json");
        assert_eq!(
            response.bundle["entries"][0]["payload"]["apiSecret"],
            "[REDACTED]"
        );
        assert_eq!(
            response.bundle["entries"][0]["payload"]["headers"]["Authorization"],
            "[REDACTED]"
        );
    }

    fn sample_tenant_row() -> TenantRow {
        TenantRow {
            id: "tenant_sandbox_001".to_string(),
            name: "Sandbox Tenant".to_string(),
            status: "PENDING".to_string(),
            api_key_hash: "hash".to_string(),
            webhook_secret_hash: "wh_hash".to_string(),
            webhook_secret_encrypted: None,
            api_secret_encrypted: None,
            webhook_url: None,
            config: serde_json::json!({"environment": "sandbox"}),
            daily_payin_limit_vnd: None,
            daily_payout_limit_vnd: None,
            api_version: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
