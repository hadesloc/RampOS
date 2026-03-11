use axum::{
    extract::Query,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;

use ramp_core::service::{NetSettlementService, NetSettlementWorkbenchSnapshot};

use crate::error::ApiError;

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

    Ok(match query
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
    })
}

fn export_csv(snapshot: &NetSettlementWorkbenchSnapshot) -> String {
    let mut rows = vec![
        "proposal_id,counterparty_id,asset,net_amount,direction,status,approval_required"
            .to_string(),
    ];
    for proposal in &snapshot.proposals {
        rows.push(format!(
            "{},{},{},{},{},{},{}",
            proposal.id,
            proposal.counterparty_id,
            proposal.asset,
            proposal.net_amount,
            proposal.direction,
            proposal.status,
            proposal.approval_required
        ));
    }
    rows.join("\n")
}
