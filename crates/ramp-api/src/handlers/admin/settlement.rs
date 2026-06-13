use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use ramp_common::types::TenantId;
use ramp_core::repository::{PgOfframpIntentRepository, PgRfqRepository, PgSettlementRepository};
use ramp_core::service::rfq::RfqService;
use ramp_core::service::{
    ApplySettlementOutcomeRequest, LinkedOfframpExecutionService, NetSettlementService,
    NetSettlementWorkbenchSnapshot, Settlement, SettlementOutcome,
};

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettlementWorkbenchQuery {
    pub scenario: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettlementExportQuery {
    pub scenario: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettlementWorkbenchResponse {
    pub snapshot: NetSettlementWorkbenchSnapshot,
    pub action_mode: String,
    pub approval_mode: String,
    pub proposal_count: usize,
    pub export_formats: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettlementOutcomeRequest {
    pub outcome: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettlementOutcomeResponse {
    pub settlement_id: String,
    pub offramp_intent_id: String,
    pub status: String,
    pub rfq_id: Option<String>,
    pub lp_id: Option<String>,
    pub final_rate: Option<String>,
}

pub async fn get_settlement_workbench(
    headers: HeaderMap,
    Query(query): Query<SettlementWorkbenchQuery>,
) -> Result<Json<SettlementWorkbenchResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let snapshot = NetSettlementService::new().build_workbench(query.scenario.as_deref());
    info!("Admin: loading bilateral settlement workbench");

    Ok(Json(SettlementWorkbenchResponse {
        action_mode: snapshot.action_mode.clone(),
        approval_mode: snapshot.approval_mode.clone(),
        proposal_count: snapshot.proposals.len(),
        export_formats: vec!["json".to_string(), "csv".to_string()],
        snapshot,
    }))
}

pub async fn export_settlement_workbench(
    headers: HeaderMap,
    Query(query): Query<SettlementExportQuery>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let snapshot = NetSettlementService::new().build_workbench(query.scenario.as_deref());
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
                        &format!("attachment; filename=\"settlement_workbench_{timestamp}.csv\""),
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
                        &format!("attachment; filename=\"settlement_workbench_{timestamp}.json\""),
                    ),
                ],
                serde_json::to_string_pretty(&snapshot)
                    .map_err(|error| ApiError::Internal(error.to_string()))?,
            )
                .into_response(),
            other => {
                return Err(ApiError::Validation(format!(
                    "Unsupported settlement export format '{}'",
                    other
                )))
            }
        },
    )
}

pub async fn apply_settlement_outcome(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SettlementOutcomeRequest>,
) -> Result<Json<SettlementOutcomeResponse>, ApiError> {
    super::tier::check_admin_key_operator(&headers)?;

    let pool = app_state.db_pool.as_ref().ok_or_else(|| {
        ApiError::Internal("Settlement runtime is unavailable: database not configured".to_string())
    })?;
    let tenant_id = TenantId(tenant_ctx.tenant_id.0.clone());
    let outcome = match req.outcome.to_ascii_uppercase().as_str() {
        "COMPLETED" => SettlementOutcome::Completed,
        "FAILED" => SettlementOutcome::Failed,
        other => {
            return Err(ApiError::Validation(format!(
                "Unsupported settlement outcome '{}'",
                other
            )))
        }
    };

    let rfq_repo = Arc::new(PgRfqRepository::new(pool.clone()));
    let coordinator = LinkedOfframpExecutionService::new(
        RfqService::new(rfq_repo.clone(), app_state.event_publisher.clone()),
        rfq_repo,
        Arc::new(PgOfframpIntentRepository::new(pool.clone())),
        Arc::new(PgSettlementRepository::new(pool.clone())),
    );
    let settlement = coordinator
        .apply_settlement_outcome(ApplySettlementOutcomeRequest {
            tenant_id,
            settlement_id: id,
            outcome,
            error_message: req.error_message,
        })
        .await
        .map_err(ApiError::from)?;

    Ok(Json(map_settlement_outcome_response(settlement)))
}

fn map_settlement_outcome_response(settlement: Settlement) -> SettlementOutcomeResponse {
    SettlementOutcomeResponse {
        settlement_id: settlement.id,
        offramp_intent_id: settlement.offramp_intent_id,
        status: settlement.status.as_db_str().to_string(),
        rfq_id: settlement.rfq_id,
        lp_id: settlement.lp_id,
        final_rate: settlement.final_rate.map(|rate| rate.to_string()),
    }
}

fn export_csv(snapshot: &NetSettlementWorkbenchSnapshot) -> String {
    let mut rows = vec![
        "proposal_id,counterparty_id,asset,net_amount,direction,status,approval_required,maker_checker_state,maker_user_id,checker_user_id,delegated_approver_id,delegation_expires_at,approval_reference_id"
            .to_string(),
    ];
    for proposal in &snapshot.proposals {
        rows.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            proposal.id,
            proposal.counterparty_id,
            proposal.asset,
            proposal.net_amount,
            proposal.direction,
            proposal.status,
            proposal.approval_required,
            proposal.maker_checker_state,
            proposal.maker_user_id.as_deref().unwrap_or(""),
            proposal.checker_user_id.as_deref().unwrap_or(""),
            proposal.delegated_approver_id.as_deref().unwrap_or(""),
            proposal.delegation_expires_at.as_deref().unwrap_or(""),
            proposal.approval_reference_id.as_deref().unwrap_or("")
        ));
    }
    rows.join("\n")
}
