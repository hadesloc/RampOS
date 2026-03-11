use std::collections::BTreeSet;
use std::sync::Arc;

use axum::{
    extract::{Extension, Query, State},
    http::HeaderMap,
    Json,
};
use ramp_common::types::IntentId;
use ramp_core::repository::{PgRfqRepository, PgSettlementRepository};
use ramp_core::service::rfq::RfqService;
use ramp_core::service::{
    IncidentTimeline, IncidentTimelineAssembler, IncidentTimelineEntry, SettlementService,
    SlaGuardianService, SlaGuardianSnapshot,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncidentLookupQuery {
    pub intent_id: Option<String>,
    pub bank_reference: Option<String>,
    pub webhook_id: Option<String>,
    pub rfq_id: Option<String>,
}

impl IncidentLookupQuery {
    fn is_empty(&self) -> bool {
        self.intent_id.is_none()
            && self.bank_reference.is_none()
            && self.webhook_id.is_none()
            && self.rfq_id.is_none()
    }

    fn matched_by(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.intent_id.is_some() {
            fields.push("intentId");
        }
        if self.bank_reference.is_some() {
            fields.push("bankReference");
        }
        if self.webhook_id.is_some() {
            fields.push("webhookId");
        }
        if self.rfq_id.is_some() {
            fields.push("rfqId");
        }
        fields
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncidentSearchResult {
    pub incident_id: String,
    pub matched_by: Vec<String>,
    pub related_reference_ids: Vec<String>,
    pub entry_count: usize,
    pub recommendation_count: usize,
    pub sla_guardian: SlaGuardianSnapshot,
    pub latest_status: Option<String>,
    pub latest_occurred_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncidentSearchResponse {
    pub data: Vec<IncidentSearchResult>,
    pub total: usize,
}

pub async fn search_incidents(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Query(query): Query<IncidentLookupQuery>,
) -> Result<Json<IncidentSearchResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    ensure_lookup(&query)?;

    let timeline = load_incident_timeline(&app_state, &tenant_ctx, &query).await?;
    let guardian = SlaGuardianService::new()
        .summarize(&timeline, app_state.metrics_registry.incident_signal_snapshot());
    let search_result = summarize_timeline(&timeline, &query, guardian);

    info!(
        tenant = %tenant_ctx.tenant_id,
        incident_id = %timeline.incident_id,
        matched_by = ?query.matched_by(),
        "Admin: searching incidents"
    );

    Ok(Json(IncidentSearchResponse {
        data: vec![search_result],
        total: 1,
    }))
}

pub async fn get_incident_timeline(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Query(query): Query<IncidentLookupQuery>,
) -> Result<Json<IncidentTimeline>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    ensure_lookup(&query)?;

    let timeline = load_incident_timeline(&app_state, &tenant_ctx, &query).await?;

    info!(
        tenant = %tenant_ctx.tenant_id,
        incident_id = %timeline.incident_id,
        matched_by = ?query.matched_by(),
        "Admin: loading incident timeline"
    );

    Ok(Json(timeline))
}

fn ensure_lookup(query: &IncidentLookupQuery) -> Result<(), ApiError> {
    if query.is_empty() {
        return Err(ApiError::Validation(
            "At least one of intentId, bankReference, webhookId, or rfqId is required".to_string(),
        ));
    }

    Ok(())
}

fn require_pool(state: &AppState, lookup_name: &str) -> Result<sqlx::PgPool, ApiError> {
    state.db_pool.clone().ok_or_else(|| {
        ApiError::Internal(format!(
            "Incident lookup by {} requires database-backed services",
            lookup_name
        ))
    })
}

fn make_settlement_service(pool: sqlx::PgPool) -> SettlementService {
    SettlementService::with_repository(Arc::new(PgSettlementRepository::new(pool)))
}

fn make_rfq_service(pool: sqlx::PgPool, state: &AppState) -> RfqService {
    RfqService::new(
        Arc::new(PgRfqRepository::new(pool)),
        state.event_publisher.clone(),
    )
}

async fn load_incident_timeline(
    state: &AppState,
    tenant_ctx: &TenantContext,
    query: &IncidentLookupQuery,
) -> Result<IncidentTimeline, ApiError> {
    let mut entries = Vec::new();

    if state.db_pool.is_some() && query.bank_reference.is_some() {
        return Err(ApiError::Validation(
            "bankReference incident lookups are disabled until tenant-scoped settlement correlation is available".to_string(),
        ));
    }

    if let Some(webhook_id) = &query.webhook_id {
        if let Some(entry) = state
            .webhook_service
            .incident_timeline_entry_for_event(&tenant_ctx.tenant_id, webhook_id)
            .await
            .map_err(ApiError::from)?
        {
            entries.push(entry);
        }
    }

    if let Some(bank_reference) = &query.bank_reference {
        if let Some(pool) = &state.db_pool {
            let settlement_service = make_settlement_service(pool.clone());
            entries.extend(
                settlement_service
                    .incident_timeline_entries_for_bank_reference_async(bank_reference)
                    .await
                    .map_err(ApiError::from)?,
            );
        } else {
            let settlement_service = SettlementService::new();
            entries.extend(settlement_service.incident_timeline_entries_for_bank_reference(
                bank_reference,
            ));
        }
    }

    if let Some(rfq_id) = &query.rfq_id {
        let pool = require_pool(state, "rfqId")?;
        let rfq_service = make_rfq_service(pool, state);
        entries.extend(
            rfq_service
                .incident_timeline_entries_for_request(&tenant_ctx.tenant_id, rfq_id)
                .await
                .map_err(ApiError::from)?,
        );
    }

    let resolved_intent_id = query
        .intent_id
        .clone()
        .or_else(|| extract_intent_id_from_entries(&entries))
        .or_else(|| extract_offramp_id_from_entries(&entries));

    if let Some(intent_id) = resolved_intent_id.as_deref() {
        let intent_id = IntentId::new(intent_id);
        entries.extend(
            state
                .webhook_service
                .incident_timeline_entries_for_intent(&tenant_ctx.tenant_id, &intent_id)
                .await
                .map_err(ApiError::from)?,
        );

        if state.db_pool.is_none() {
            // Repository-backed settlement correlation remains disabled until settlement
            // persistence carries an enforceable tenant scope.
        } else if let Some(pool) = &state.db_pool {
            let _ = pool;
        }

        if state.db_pool.is_none() {
            let settlement_service = SettlementService::new();
            entries.extend(settlement_service.incident_timeline_entries_for_offramp(&intent_id.0));
        }
    }

    dedupe_entries(&mut entries);
    if entries.is_empty() {
        return Err(ApiError::NotFound(
            "No incident data found for the provided lookup".to_string(),
        ));
    }

    let incident_id = build_incident_id(query, resolved_intent_id.as_deref(), &entries);
    Ok(IncidentTimelineAssembler::assemble_with_signals(
        incident_id,
        entries,
        Vec::new(),
        state.metrics_registry.incident_signal_snapshot(),
    ))
}

fn extract_intent_id_from_entries(entries: &[IncidentTimelineEntry]) -> Option<String> {
    entries.iter().find_map(|entry| {
        entry
            .details
            .get("intentId")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
    })
}

fn extract_offramp_id_from_entries(entries: &[IncidentTimelineEntry]) -> Option<String> {
    entries.iter().find_map(|entry| {
        entry
            .details
            .get("offrampIntentId")
            .or_else(|| entry.details.get("offrampId"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
    })
}

fn dedupe_entries(entries: &mut Vec<IncidentTimelineEntry>) {
    let mut seen = BTreeSet::new();
    entries.retain(|entry| {
        seen.insert((entry.source_kind.clone(), entry.source_reference_id.clone()))
    });
}

fn build_incident_id(
    query: &IncidentLookupQuery,
    resolved_intent_id: Option<&str>,
    entries: &[IncidentTimelineEntry],
) -> String {
    if let Some(intent_id) = resolved_intent_id.or(query.intent_id.as_deref()) {
        return format!("incident_intent_{}", intent_id);
    }
    if let Some(bank_reference) = query.bank_reference.as_deref() {
        return format!("incident_bank_{}", sanitize_reference(bank_reference));
    }
    if let Some(webhook_id) = query.webhook_id.as_deref() {
        return format!("incident_webhook_{}", webhook_id);
    }
    if let Some(rfq_id) = query.rfq_id.as_deref() {
        return format!("incident_rfq_{}", rfq_id);
    }

    format!("incident_{}", entries[0].source_reference_id)
}

fn sanitize_reference(reference: &str) -> String {
    reference
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn summarize_timeline(
    timeline: &IncidentTimeline,
    query: &IncidentLookupQuery,
    guardian: SlaGuardianSnapshot,
) -> IncidentSearchResult {
    let latest_entry = timeline.entries.last();

    IncidentSearchResult {
        incident_id: timeline.incident_id.clone(),
        matched_by: query
            .matched_by()
            .into_iter()
            .map(|field| field.to_string())
            .collect(),
        related_reference_ids: collect_related_reference_ids(&timeline.entries),
        entry_count: timeline.entries.len(),
        recommendation_count: timeline.recommendations.len(),
        sla_guardian: guardian,
        latest_status: latest_entry.map(|entry| entry.status.clone()),
        latest_occurred_at: latest_entry.map(|entry| entry.occurred_at.to_rfc3339()),
    }
}

fn collect_related_reference_ids(entries: &[IncidentTimelineEntry]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut related = Vec::new();

    for entry in entries {
        if seen.insert(entry.source_reference_id.clone()) {
            related.push(entry.source_reference_id.clone());
        }

        for reference in &entry.related_reference_ids {
            if seen.insert(reference.clone()) {
                related.push(reference.clone());
            }
        }
    }

    related
}
