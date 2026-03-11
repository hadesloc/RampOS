use axum::{
    extract::Query,
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::info;

use ramp_core::service::{TreasuryControlTowerSnapshot, TreasuryService};

use crate::error::ApiError;

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
    Query(query): Query<TreasuryWorkbenchQuery>,
) -> Result<Json<TreasuryWorkbenchResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let service = TreasuryService::new();
    let snapshot = service.build_control_tower(query.scenario.as_deref());

    info!("Admin: loading treasury workbench");

    Ok(Json(TreasuryWorkbenchResponse {
        action_mode: snapshot.action_mode.clone(),
        recommendation_count: snapshot.recommendations.len(),
        stress_alert_count: snapshot.alerts.len(),
        snapshot,
    }))
}

pub async fn export_treasury_workbench(
    headers: HeaderMap,
    Query(query): Query<TreasuryExportQuery>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let snapshot = TreasuryService::new().build_control_tower(query.scenario.as_deref());
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
    })
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

    rows.join("\n")
}
