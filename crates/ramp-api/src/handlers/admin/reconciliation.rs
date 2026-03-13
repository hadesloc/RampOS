use axum::{
    extract::{Path, Query},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use ramp_core::service::reconciliation_export::{
    ReconciliationExportFormat, ReconciliationExportService, ReconciliationWorkbenchSnapshot,
};
use ramp_core::service::settlement::{Settlement, SettlementStatus};
use ramp_core::service::{
    OnChainTransaction, ReconciliationEvidencePack, ReconciliationService, SettlementRecord,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::error::ApiError;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationWorkbenchQuery {
    pub scenario: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationExportQuery {
    pub scenario: Option<String>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationWorkbenchResponse {
    pub snapshot: ReconciliationWorkbenchSnapshot,
    pub action_mode: String,
    pub export_formats: Vec<String>,
    pub incident_link_hint: String,
    pub gated_actions: Vec<ReconciliationGatedAction>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationGatedAction {
    pub discrepancy_id: String,
    pub action_code: String,
    pub action_mode: String,
    pub approval_required: bool,
    pub approval_reference_id: String,
    pub audit_scope: String,
    pub operator_assist_reason: String,
}

pub async fn get_reconciliation_workbench(
    headers: HeaderMap,
    Query(query): Query<ReconciliationWorkbenchQuery>,
) -> Result<Json<ReconciliationWorkbenchResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let (snapshot, _) = build_workbench(query.scenario.as_deref());
    let gated_actions = build_gated_actions(&snapshot);
    info!("Admin: loading reconciliation workbench");

    Ok(Json(ReconciliationWorkbenchResponse {
        snapshot,
        action_mode: "recommendation_only".to_string(),
        export_formats: vec!["json".to_string(), "csv".to_string()],
        incident_link_hint: "/v1/admin/incidents/timeline".to_string(),
        gated_actions,
    }))
}

pub async fn export_reconciliation_workbench(
    headers: HeaderMap,
    Query(query): Query<ReconciliationExportQuery>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let (snapshot, _) = build_workbench(query.scenario.as_deref());
    let format = parse_export_format(query.format.as_deref())?;
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");

    Ok(match format {
        ReconciliationExportFormat::Csv => (
            [
                (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    &format!("attachment; filename=\"reconciliation_queue_{timestamp}.csv\""),
                ),
            ],
            ReconciliationExportService::export_queue_csv(&snapshot),
        )
            .into_response(),
        ReconciliationExportFormat::Json => (
            [
                (axum::http::header::CONTENT_TYPE, "application/json"),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    &format!("attachment; filename=\"reconciliation_snapshot_{timestamp}.json\""),
                ),
            ],
            serde_json::to_string_pretty(&ReconciliationExportService::export_snapshot_json(
                &snapshot,
            ))
            .map_err(|error| ApiError::Internal(error.to_string()))?,
        )
            .into_response(),
    })
}

pub async fn get_reconciliation_evidence(
    headers: HeaderMap,
    Path(discrepancy_id): Path<String>,
    Query(query): Query<ReconciliationWorkbenchQuery>,
) -> Result<Json<ReconciliationEvidencePack>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let (snapshot, settlements) = build_workbench(query.scenario.as_deref());
    let service = ReconciliationService::new();
    let evidence = service
        .build_evidence_pack(&snapshot.report, &settlements, &discrepancy_id)
        .map_err(ApiError::NotFound)?;

    Ok(Json(evidence))
}

pub async fn export_reconciliation_evidence(
    headers: HeaderMap,
    Path(discrepancy_id): Path<String>,
    Query(query): Query<ReconciliationExportQuery>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let (snapshot, settlements) = build_workbench(query.scenario.as_deref());
    let service = ReconciliationService::new();
    let evidence = service
        .build_evidence_pack(&snapshot.report, &settlements, &discrepancy_id)
        .map_err(ApiError::NotFound)?;
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");

    let body =
        serde_json::to_string_pretty(&evidence).map_err(|error| ApiError::Internal(error.to_string()))?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/json"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                &format!(
                    "attachment; filename=\"reconciliation_evidence_{}_{}.json\"",
                    discrepancy_id, timestamp
                ),
            ),
        ],
        body,
    )
        .into_response())
}

fn parse_export_format(raw: Option<&str>) -> Result<ReconciliationExportFormat, ApiError> {
    match raw.unwrap_or("json").to_ascii_lowercase().as_str() {
        "json" => Ok(ReconciliationExportFormat::Json),
        "csv" => Ok(ReconciliationExportFormat::Csv),
        other => Err(ApiError::Validation(format!(
            "Unsupported reconciliation export format '{}'",
            other
        ))),
    }
}

fn build_workbench(
    scenario: Option<&str>,
) -> (ReconciliationWorkbenchSnapshot, Vec<Settlement>) {
    let service = ReconciliationService::new();
    let (on_chain_txs, settlement_records, settlements) = sample_fixture_set(scenario);
    let mut snapshot =
        ReconciliationExportService::build_snapshot(&service, &on_chain_txs, &settlement_records);
    stabilize_snapshot_ids(&mut snapshot, scenario.unwrap_or("active"));
    (snapshot, settlements)
}

fn build_gated_actions(snapshot: &ReconciliationWorkbenchSnapshot) -> Vec<ReconciliationGatedAction> {
    snapshot
        .queue
        .iter()
        .take(3)
        .map(|item| ReconciliationGatedAction {
            discrepancy_id: item.discrepancy_id.clone(),
            action_code: "resolve_discrepancy".to_string(),
            action_mode: "operator_assisted".to_string(),
            approval_required: true,
            approval_reference_id: format!("approval_recon_{}", item.discrepancy_id),
            audit_scope: "reconciliation_discrepancy_resolution".to_string(),
            operator_assist_reason:
                "Mutable reconciliation actions stay operator-assisted and audit-linked."
                    .to_string(),
        })
        .collect()
}

fn stabilize_snapshot_ids(snapshot: &mut ReconciliationWorkbenchSnapshot, scenario: &str) {
    let report_id = format!("recon_{scenario}_workbench");
    let mut discrepancy_ids = HashMap::new();

    for (index, discrepancy) in snapshot.report.discrepancies.iter_mut().enumerate() {
        let stable_id = stable_discrepancy_id(discrepancy, index);
        discrepancy_ids.insert(discrepancy.id.clone(), stable_id.clone());
        discrepancy.id = stable_id;
    }

    snapshot.report.id = report_id.clone();
    for queue_item in &mut snapshot.queue {
        if let Some(stable_id) = discrepancy_ids.get(&queue_item.discrepancy_id) {
            queue_item.discrepancy_id = stable_id.clone();
        }
        queue_item.report_id = report_id.clone();
    }
}

fn stable_discrepancy_id(
    discrepancy: &ramp_core::service::Discrepancy,
    index: usize,
) -> String {
    let kind = format!("{:?}", discrepancy.kind).to_ascii_lowercase();
    let reference = discrepancy
        .settlement_id
        .as_deref()
        .or(discrepancy.on_chain_tx.as_deref())
        .unwrap_or("unscoped")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();

    format!("disc_{kind}_{reference}_{index}")
}

fn sample_fixture_set(
    scenario: Option<&str>,
) -> (Vec<OnChainTransaction>, Vec<SettlementRecord>, Vec<Settlement>) {
    let now = Utc::now();

    if matches!(scenario, Some("clean")) {
        let settlement_record = SettlementRecord {
            id: "stl_recon_clean_001".to_string(),
            tx_hash: Some("0xclean".to_string()),
            amount: 250.0,
            currency: "USDT".to_string(),
            status: "COMPLETED".to_string(),
            created_at: now - Duration::minutes(10),
            updated_at: now - Duration::minutes(5),
        };
        let settlement = Settlement {
            id: settlement_record.id.clone(),
            offramp_intent_id: "ofr_recon_clean_001".to_string(),
            status: SettlementStatus::Completed,
            bank_reference: Some("RAMP-CLEAN".to_string()),
            error_message: None,
            created_at: settlement_record.created_at,
            updated_at: settlement_record.updated_at,
        };

        return (
            vec![OnChainTransaction {
                tx_hash: "0xclean".to_string(),
                from: "0xsource".to_string(),
                to: "0xdestination".to_string(),
                amount: 250.0,
                currency: "USDT".to_string(),
                timestamp: now - Duration::minutes(4),
                confirmed: true,
            }],
            vec![settlement_record],
            vec![settlement],
        );
    }

    let settlement_records = vec![
        SettlementRecord {
            id: "stl_recon_processing_001".to_string(),
            tx_hash: None,
            amount: 100.005,
            currency: "USDT".to_string(),
            status: "PROCESSING".to_string(),
            created_at: now - Duration::minutes(55),
            updated_at: now - Duration::minutes(47),
        },
        SettlementRecord {
            id: "stl_recon_status_001".to_string(),
            tx_hash: Some("0xstatus".to_string()),
            amount: 250.0,
            currency: "USDT".to_string(),
            status: "COMPLETED".to_string(),
            created_at: now - Duration::minutes(30),
            updated_at: now - Duration::minutes(18),
        },
    ];

    let settlements = vec![
        Settlement {
            id: "stl_recon_processing_001".to_string(),
            offramp_intent_id: "ofr_recon_processing_001".to_string(),
            status: SettlementStatus::Processing,
            bank_reference: Some("RAMP-PROCESS".to_string()),
            error_message: None,
            created_at: now - Duration::minutes(55),
            updated_at: now - Duration::minutes(47),
        },
        Settlement {
            id: "stl_recon_status_001".to_string(),
            offramp_intent_id: "ofr_recon_status_001".to_string(),
            status: SettlementStatus::Completed,
            bank_reference: Some("RAMP-STATUS".to_string()),
            error_message: None,
            created_at: now - Duration::minutes(30),
            updated_at: now - Duration::minutes(18),
        },
    ];

    let on_chain_txs = vec![
        OnChainTransaction {
            tx_hash: "0xqueue".to_string(),
            from: "0xsource".to_string(),
            to: "0xdestination".to_string(),
            amount: 100.0,
            currency: "USDT".to_string(),
            timestamp: now - Duration::minutes(46),
            confirmed: true,
        },
        OnChainTransaction {
            tx_hash: "0xstatus".to_string(),
            from: "0xsource".to_string(),
            to: "0xdestination".to_string(),
            amount: 250.0,
            currency: "USDT".to_string(),
            timestamp: now - Duration::minutes(19),
            confirmed: false,
        },
    ];

    (on_chain_txs, settlement_records, settlements)
}
