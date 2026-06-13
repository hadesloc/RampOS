use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use ramp_common::types::IntentId;
use ramp_core::repository::{
    OfframpIntentRepository, OfframpIntentRow, OnchainObservationRepository, OnchainObservationRow,
    PgOfframpIntentRepository, PgOnchainObservationRepository, PgSettlementRepository,
    SettlementRepository, SettlementRow,
};
use ramp_core::service::reconciliation::{
    ReconciliationEvidenceSource, ReconciliationLineageRecord,
};
use ramp_core::service::reconciliation_export::{
    ReconciliationExportFormat, ReconciliationExportService, ReconciliationWorkbenchSnapshot,
};
use ramp_core::service::settlement::{Settlement, SettlementStatus};
use ramp_core::service::{
    IncidentTimelineEntry, OnChainTransaction, ReconciliationEvidencePack, ReconciliationService,
    ReplayTimelineEntry, SettlementRecord,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::AppState;

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
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Query(query): Query<ReconciliationWorkbenchQuery>,
) -> Result<Json<ReconciliationWorkbenchResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let (snapshot, _) = build_workbench(&app_state, &tenant_ctx, query.scenario.as_deref()).await?;
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
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Query(query): Query<ReconciliationExportQuery>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let (snapshot, _) = build_workbench(&app_state, &tenant_ctx, query.scenario.as_deref()).await?;
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
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Path(discrepancy_id): Path<String>,
    Query(query): Query<ReconciliationWorkbenchQuery>,
) -> Result<Json<ReconciliationEvidencePack>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let (snapshot, settlements) =
        build_workbench(&app_state, &tenant_ctx, query.scenario.as_deref()).await?;
    let service = ReconciliationService::new();
    let mut evidence = service
        .build_evidence_pack(&snapshot.report, &settlements, &discrepancy_id)
        .map_err(ApiError::NotFound)?;
    enrich_evidence_with_runtime_lineage(&app_state, &tenant_ctx, &settlements, &mut evidence)
        .await?;

    Ok(Json(evidence))
}

pub async fn export_reconciliation_evidence(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Path(discrepancy_id): Path<String>,
    Query(query): Query<ReconciliationExportQuery>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let (snapshot, settlements) =
        build_workbench(&app_state, &tenant_ctx, query.scenario.as_deref()).await?;
    let service = ReconciliationService::new();
    let mut evidence = service
        .build_evidence_pack(&snapshot.report, &settlements, &discrepancy_id)
        .map_err(ApiError::NotFound)?;
    enrich_evidence_with_runtime_lineage(&app_state, &tenant_ctx, &settlements, &mut evidence)
        .await?;
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");

    let body = serde_json::to_string_pretty(&evidence)
        .map_err(|error| ApiError::Internal(error.to_string()))?;

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

async fn build_workbench(
    app_state: &AppState,
    tenant_ctx: &TenantContext,
    scenario: Option<&str>,
) -> Result<(ReconciliationWorkbenchSnapshot, Vec<Settlement>), ApiError> {
    let service = ReconciliationService::new();

    if scenario.is_none() {
        if let Some(runtime_snapshot) =
            build_runtime_workbench_from_db(app_state, tenant_ctx).await?
        {
            return Ok(runtime_snapshot);
        }
    }

    let (on_chain_txs, settlement_records, settlements) = sample_fixture_set(scenario);
    let mut snapshot = ReconciliationExportService::build_snapshot_with_source_kind(
        &service,
        &on_chain_txs,
        &settlement_records,
        "sample_fallback",
    );
    stabilize_snapshot_ids(&mut snapshot, scenario.unwrap_or("active"));
    Ok((snapshot, settlements))
}

async fn build_runtime_workbench_from_db(
    app_state: &AppState,
    tenant_ctx: &TenantContext,
) -> Result<Option<(ReconciliationWorkbenchSnapshot, Vec<Settlement>)>, ApiError> {
    let Some(pool) = app_state.db_pool.as_ref() else {
        return Ok(None);
    };

    let settlement_repo = PgSettlementRepository::new(pool.clone());
    let observation_repo = PgOnchainObservationRepository::new(pool.clone());
    let offramp_repo = PgOfframpIntentRepository::new(pool.clone());
    let tenant_id = &tenant_ctx.tenant_id;

    let intents = offramp_repo
        .list_by_tenant(tenant_id, 500, 0)
        .await
        .map_err(ApiError::from)?;
    if intents.is_empty() {
        return Ok(None);
    }
    let intent_map: HashMap<String, OfframpIntentRow> = intents
        .into_iter()
        .map(|intent| (intent.id.clone(), intent))
        .collect();

    let mut settlement_rows = Vec::new();
    for intent_id in intent_map.keys() {
        settlement_rows.extend(
            settlement_repo
                .list_by_offramp_in_tenant(tenant_id, intent_id)
                .await
                .map_err(ApiError::from)?,
        );
    }
    if settlement_rows.is_empty() {
        return Ok(None);
    }

    let service = ReconciliationService::new();
    let settlement_records = settlement_rows
        .iter()
        .map(|row| settlement_record_from_row(row, intent_map.get(&row.offramp_intent_id)))
        .collect::<Vec<_>>();
    let settlements = settlement_rows
        .iter()
        .map(settlement_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    let mut on_chain_txs = Vec::<OnChainTransaction>::new();
    let mut has_portal_submitted_receipts = false;
    for intent_id in intent_map.keys() {
        let observations = observation_repo
            .list_by_offramp_intent(tenant_id, intent_id)
            .await
            .map_err(ApiError::from)?;
        has_portal_submitted_receipts |= observations
            .iter()
            .any(|row| row.observation_source == "portal_offramp_crypto_received");
        on_chain_txs.extend(observations.iter().map(onchain_tx_from_row));
    }

    let mut snapshot = ReconciliationExportService::build_snapshot_with_source_kind(
        &service,
        &on_chain_txs,
        &settlement_records,
        "runtime_inputs",
    );
    if has_portal_submitted_receipts {
        append_freshness_warning(
            &mut snapshot,
            "Portal-submitted off-ramp receipts are present; treat them as runtime-linked evidence and not autonomous chain-monitor truth.",
        );
    }
    stabilize_snapshot_ids(&mut snapshot, "runtime_inputs");
    Ok(Some((snapshot, settlements)))
}

fn settlement_record_from_row(
    row: &SettlementRow,
    linked_intent: Option<&OfframpIntentRow>,
) -> SettlementRecord {
    SettlementRecord {
        id: row.id.clone(),
        tx_hash: linked_intent.and_then(|intent| intent.tx_hash.clone()),
        amount: 0.0,
        currency: linked_intent
            .map(|intent| intent.crypto_asset.clone())
            .unwrap_or_else(|| "UNKNOWN".to_string()),
        status: row.status.clone(),
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn settlement_from_row(row: &SettlementRow) -> Result<Settlement, ApiError> {
    let status = SettlementStatus::from_db_str(&row.status)
        .map_err(|error| ApiError::Internal(error.to_string()))?;
    Ok(Settlement {
        id: row.id.clone(),
        tenant_id: row.tenant_id.clone(),
        offramp_intent_id: row.offramp_intent_id.clone(),
        rfq_id: row.rfq_id.clone(),
        lp_id: row.lp_id.clone(),
        final_rate: row.final_rate,
        status,
        bank_reference: row.bank_reference.clone(),
        error_message: row.error_message.clone(),
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn onchain_tx_from_row(row: &OnchainObservationRow) -> OnChainTransaction {
    let is_confirmed = row.status == "CONFIRMED"
        || (row.required_confirmations > 0 && row.confirmations >= row.required_confirmations);
    OnChainTransaction {
        tx_hash: row.tx_hash.clone(),
        from: row.from_address.clone(),
        to: row.to_address.clone(),
        amount: row.amount.to_string().parse::<f64>().unwrap_or(0.0),
        currency: row.asset_code.clone(),
        timestamp: row.observed_at,
        confirmed: is_confirmed,
    }
}

fn append_freshness_warning(snapshot: &mut ReconciliationWorkbenchSnapshot, warning: &str) {
    let existing = snapshot.provenance.freshness_warning.take();
    snapshot.provenance.freshness_warning = Some(match existing {
        Some(message) => format!("{message} {warning}"),
        None => warning.to_string(),
    });
}

fn build_gated_actions(
    snapshot: &ReconciliationWorkbenchSnapshot,
) -> Vec<ReconciliationGatedAction> {
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

fn stable_discrepancy_id(discrepancy: &ramp_core::service::Discrepancy, index: usize) -> String {
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
) -> (
    Vec<OnChainTransaction>,
    Vec<SettlementRecord>,
    Vec<Settlement>,
) {
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
            tenant_id: None,
            offramp_intent_id: "ofr_recon_clean_001".to_string(),
            rfq_id: None,
            lp_id: None,
            final_rate: None,
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
            tenant_id: None,
            offramp_intent_id: "ofr_recon_processing_001".to_string(),
            rfq_id: None,
            lp_id: None,
            final_rate: None,
            status: SettlementStatus::Processing,
            bank_reference: Some("RAMP-PROCESS".to_string()),
            error_message: None,
            created_at: now - Duration::minutes(55),
            updated_at: now - Duration::minutes(47),
        },
        Settlement {
            id: "stl_recon_status_001".to_string(),
            tenant_id: None,
            offramp_intent_id: "ofr_recon_status_001".to_string(),
            rfq_id: None,
            lp_id: None,
            final_rate: None,
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

async fn enrich_evidence_with_runtime_lineage(
    app_state: &AppState,
    tenant_ctx: &TenantContext,
    settlements: &[Settlement],
    evidence: &mut ReconciliationEvidencePack,
) -> Result<(), ApiError> {
    let linked_settlement_ids: HashSet<&str> =
        evidence.settlement_ids.iter().map(String::as_str).collect();
    if linked_settlement_ids.is_empty() {
        return Ok(());
    }

    let linked_intent_ids: BTreeSet<String> = settlements
        .iter()
        .filter(|settlement| linked_settlement_ids.contains(settlement.id.as_str()))
        .map(|settlement| settlement.offramp_intent_id.clone())
        .filter(|intent_id| !intent_id.is_empty())
        .collect();
    if linked_intent_ids.is_empty() {
        return Ok(());
    }

    for intent_id in &linked_intent_ids {
        let intent = IntentId::new(intent_id);
        let replay_entries = app_state
            .webhook_service
            .replay_timeline_entries_for_intent(&tenant_ctx.tenant_id, &intent)
            .await
            .map_err(ApiError::from)?;
        evidence.replay_entries.extend(replay_entries);

        let incident_entries = app_state
            .webhook_service
            .incident_timeline_entries_for_intent(&tenant_ctx.tenant_id, &intent)
            .await
            .map_err(ApiError::from)?;
        evidence.incident_entries.extend(incident_entries);
    }

    dedupe_and_sort_replay_entries(&mut evidence.replay_entries);
    dedupe_and_sort_incident_entries(&mut evidence.incident_entries);

    if let Some(pool) = app_state.db_pool.as_ref() {
        let offramp_repo = PgOfframpIntentRepository::new(pool.clone());
        for intent_id in &linked_intent_ids {
            if let Some(intent) = offramp_repo
                .get_intent(&tenant_ctx.tenant_id, intent_id)
                .await
                .map_err(ApiError::from)?
            {
                append_offramp_intent_lineage(evidence, &intent);
            }
        }
    }

    Ok(())
}

fn append_offramp_intent_lineage(
    evidence: &mut ReconciliationEvidencePack,
    intent: &OfframpIntentRow,
) {
    let evidence_source_id = format!("evidence_offramp_intent_{}", intent.id);
    if !evidence
        .evidence_sources
        .iter()
        .any(|source| source.evidence_source_id == evidence_source_id)
    {
        evidence
            .evidence_sources
            .push(ReconciliationEvidenceSource {
                evidence_source_id: evidence_source_id.clone(),
                source_family: "offramp_intent".to_string(),
                source_ref: format!("offramp://{}", intent.id),
                snapshot_at: intent.updated_at,
                entity_scope: "treasury:offramp_intent".to_string(),
                corridor_code: Some(format!("{}_VN_OFFRAMP", intent.crypto_asset)),
            });
    }

    let lineage_id = format!("lineage_offramp_intent_{}", intent.id);
    if !evidence
        .lineage_records
        .iter()
        .any(|lineage| lineage.lineage_id == lineage_id)
    {
        evidence.lineage_records.push(ReconciliationLineageRecord {
            lineage_id,
            lineage_kind: "offramp_intent".to_string(),
            reference_id: intent.id.clone(),
            parent_reference_id: Some(evidence.queue_item.discrepancy_id.clone()),
            entity_scope: "treasury:offramp_intent".to_string(),
            corridor_code: Some(format!("{}_VN_OFFRAMP", intent.crypto_asset)),
            operator_review_state: "review_required".to_string(),
        });
    }
}

fn dedupe_and_sort_replay_entries(entries: &mut Vec<ReplayTimelineEntry>) {
    let mut seen = BTreeSet::new();
    entries.retain(|entry| {
        seen.insert((
            entry.source.clone(),
            entry.reference_id.clone(),
            entry.occurred_at,
            entry.status.clone(),
            entry.label.clone(),
        ))
    });

    entries.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.reference_id.cmp(&right.reference_id))
    });

    for (idx, entry) in entries.iter_mut().enumerate() {
        entry.sequence = idx + 1;
    }
}

fn dedupe_and_sort_incident_entries(entries: &mut Vec<IncidentTimelineEntry>) {
    let mut seen = BTreeSet::new();
    entries.retain(|entry| {
        seen.insert((
            entry.source_kind.clone(),
            entry.source_reference_id.clone(),
            entry.occurred_at,
            entry.status.clone(),
            entry.label.clone(),
        ))
    });

    entries.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.source_kind.cmp(&right.source_kind))
            .then_with(|| left.source_reference_id.cmp(&right.source_reference_id))
    });

    for (idx, entry) in entries.iter_mut().enumerate() {
        entry.sequence = idx + 1;
    }
}
