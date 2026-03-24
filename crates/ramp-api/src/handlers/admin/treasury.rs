use axum::{
    extract::{Extension, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;

use ramp_core::service::{
    TreasuryControlTowerSnapshot, TreasuryDataSource, TreasuryEvidenceImportQuery,
    TreasuryEvidenceImportStore, TreasuryProvenance, TreasuryService,
};

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::AppState;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryWorkbenchQuery {
    pub scenario: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryExportQuery {
    pub scenario: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryWorkbenchResponse {
    pub snapshot: TreasuryControlTowerSnapshot,
    pub action_mode: String,
    pub recommendation_count: usize,
    pub stress_alert_count: usize,
}

pub async fn get_treasury_workbench(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Query(query): Query<TreasuryWorkbenchQuery>,
) -> Result<Json<TreasuryWorkbenchResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let snapshot =
        resolve_treasury_snapshot(&app_state, &tenant_ctx, query.scenario.as_deref()).await?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        data_source = %snapshot.data_source,
        "Admin: loading treasury workbench"
    );

    Ok(Json(TreasuryWorkbenchResponse {
        action_mode: snapshot.action_mode.clone(),
        recommendation_count: snapshot.recommendations.len(),
        stress_alert_count: snapshot.alerts.len(),
        snapshot,
    }))
}

pub async fn export_treasury_workbench(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Query(query): Query<TreasuryExportQuery>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let snapshot =
        resolve_treasury_snapshot(&app_state, &tenant_ctx, query.scenario.as_deref()).await?;
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");

    Ok(
        match query
            .format
            .as_deref()
            .unwrap_or("json")
            .to_ascii_lowercase()
            .as_str()
        {
            "csv" => (
                [
                    (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        &format!("attachment; filename=\"treasury_workbench_{timestamp}.csv\""),
                    ),
                ],
                export_csv(&snapshot),
            )
                .into_response(),
            "json" => (
                [
                    (axum::http::header::CONTENT_TYPE, "application/json"),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        &format!("attachment; filename=\"treasury_workbench_{timestamp}.json\""),
                    ),
                ],
                serde_json::to_string_pretty(&snapshot)
                    .map_err(|error| ApiError::Internal(error.to_string()))?,
            )
                .into_response(),
            other => {
                return Err(ApiError::Validation(format!(
                    "Unsupported treasury export format '{}'",
                    other
                )))
            }
        },
    )
}

async fn resolve_treasury_snapshot(
    app_state: &AppState,
    tenant_ctx: &TenantContext,
    scenario: Option<&str>,
) -> Result<TreasuryControlTowerSnapshot, ApiError> {
    let service = TreasuryService::new();

    // Scenario mode stays sample-backed for deterministic fixture exploration.
    if scenario.is_some() {
        return Ok(service.build_control_tower(scenario));
    }

    if let Some(pool) = app_state.db_pool.as_ref() {
        let store = TreasuryEvidenceImportStore::new(pool.clone());
        let imports = store
            .list_imports(&TreasuryEvidenceImportQuery {
                tenant_id: tenant_ctx.tenant_id.0.clone(),
                source_family: None,
                asset_code: None,
                account_scope: None,
            })
            .await
            .map_err(|error| {
                ApiError::Internal(format!(
                    "Treasury evidence lookup failed for tenant '{}': {}",
                    tenant_ctx.tenant_id.0, error
                ))
            })?;

        if !imports.is_empty() {
            let float_slices = TreasuryService::float_slices_from_evidence(&imports);
            let provenance = TreasuryProvenance::from_evidence(&imports);
            return Ok(
                service.build_control_tower_from(TreasuryDataSource::Evidence {
                    float_slices,
                    provenance,
                }),
            );
        }
    }

    Ok(service.build_control_tower(None))
}

fn export_csv(snapshot: &TreasuryControlTowerSnapshot) -> String {
    let mut rows = vec![
        "recommendation_id,category,asset,amount,source_segment,destination_segment,confidence"
            .to_string(),
    ];

    for recommendation in &snapshot.recommendations {
        rows.push(format!(
            "{},{},{},{},{},{},{}",
            recommendation.id,
            recommendation.category,
            recommendation.asset,
            recommendation.amount,
            recommendation.source_segment.as_deref().unwrap_or(""),
            recommendation.destination_segment.as_deref().unwrap_or(""),
            recommendation.confidence,
        ));
    }

    rows.push(String::new());
    rows.push("overlay_id,overlay_type,entity_scope,corridor_code,asset,ledger_mode,gross_balance,safeguarded_balance,shortfall_amount,coverage_ratio,evidence_import_id,source_family,source_ref".to_string());
    for overlay in &snapshot.safeguarding_overlays {
        rows.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            overlay.overlay_id,
            overlay.overlay_type,
            overlay.entity_scope,
            overlay.corridor_code.as_deref().unwrap_or(""),
            overlay.asset,
            overlay.ledger_mode,
            overlay.gross_balance,
            overlay.safeguarded_balance,
            overlay.shortfall_amount,
            overlay.coverage_ratio,
            overlay.evidence_ref.evidence_import_id,
            overlay.evidence_ref.source_family,
            overlay.evidence_ref.source_ref,
        ));
    }

    rows.push(String::new());
    rows.push("reserve_id,reserve_kind,entity_scope,corridor_code,asset,required_balance,available_balance,excess_or_shortfall,status,evidence_import_id,source_family,source_ref".to_string());
    for reserve in &snapshot.reserve_positions {
        rows.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            reserve.reserve_id,
            reserve.reserve_kind,
            reserve.entity_scope,
            reserve.corridor_code.as_deref().unwrap_or(""),
            reserve.asset,
            reserve.required_balance,
            reserve.available_balance,
            reserve.excess_or_shortfall,
            reserve.status,
            reserve.evidence_ref.evidence_import_id,
            reserve.evidence_ref.source_family,
            reserve.evidence_ref.source_ref,
        ));
    }

    rows.join("\n")
}
