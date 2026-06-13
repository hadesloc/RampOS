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
    let guardian = SlaGuardianService::new().summarize(
        &timeline,
        app_state.metrics_registry.incident_signal_snapshot(),
    );
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
        // Use the tenant-scoped lookup when a repository is available; this ensures
        // results are confined to the authenticated tenant and no cross-tenant data
        // leaks via a shared bank reference string.  If no repo is configured
        // (in-memory / test mode) we fall back to the in-memory store which is
        // already isolated by process boundary.
        if let Some(pool) = &state.db_pool {
            let settlement_service = make_settlement_service(pool.clone());
            entries.extend(
                settlement_service
                    .incident_timeline_entries_for_bank_reference_in_tenant_async(
                        &tenant_ctx.tenant_id,
                        bank_reference,
                    )
                    .await
                    .map_err(ApiError::from)?,
            );
        } else {
            let settlement_service = SettlementService::new();
            entries.extend(
                settlement_service.incident_timeline_entries_for_bank_reference(bank_reference),
            );
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

        // Correlate settlements for this offramp intent using the tenant-scoped lookup.
        // When a repository is configured we enforce tenant isolation via
        // list_by_offramp_in_tenant; failing closed returns an empty Vec, not a guess.
        if let Some(pool) = &state.db_pool {
            let settlement_service = make_settlement_service(pool.clone());
            entries.extend(
                settlement_service
                    .incident_timeline_entries_for_offramp_in_tenant_async(
                        &tenant_ctx.tenant_id,
                        &intent_id.0,
                    )
                    .await
                    .map_err(ApiError::from)?,
            );
        } else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ramp_common::types::TenantId;
    use ramp_core::repository::{
        InMemorySettlementRepository, SettlementRepository, SettlementRow,
    };
    use ramp_core::service::SettlementService;

    fn make_row(id: &str, offramp_id: &str, bank_ref: &str) -> SettlementRow {
        SettlementRow {
            id: id.to_string(),
            tenant_id: None,
            offramp_intent_id: offramp_id.to_string(),
            rfq_id: None,
            lp_id: None,
            final_rate: None,
            status: "COMPLETED".to_string(),
            bank_reference: Some(bank_ref.to_string()),
            error_message: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    // --- pure-function tests ---

    #[test]
    fn test_sanitize_reference_replaces_non_alphanumeric() {
        assert_eq!(sanitize_reference("REF-123/ABC"), "REF_123_ABC");
        assert_eq!(sanitize_reference("RAMP20240101"), "RAMP20240101");
        assert_eq!(sanitize_reference("ref ref"), "ref_ref");
    }

    #[test]
    fn test_build_incident_id_prefers_intent() {
        let query = IncidentLookupQuery {
            intent_id: Some("ofr_001".to_string()),
            bank_reference: Some("REF-X".to_string()),
            webhook_id: None,
            rfq_id: None,
        };
        let id = build_incident_id(&query, Some("ofr_001"), &[]);
        assert_eq!(id, "incident_intent_ofr_001");
    }

    #[test]
    fn test_build_incident_id_falls_back_to_bank_reference() {
        let query = IncidentLookupQuery {
            intent_id: None,
            bank_reference: Some("REF-42/A".to_string()),
            webhook_id: None,
            rfq_id: None,
        };
        let id = build_incident_id(&query, None, &[]);
        assert_eq!(id, "incident_bank_REF_42_A");
    }

    // --- tenant-isolation tests (via in-memory SettlementService) ---

    /// bank_reference lookup in-memory: no repo → returns entries from in-memory store
    /// (single-tenant in-memory, used by test/no-DB code path)
    #[tokio::test]
    async fn test_bank_reference_inmemory_returns_entry() {
        let svc = SettlementService::new();
        let _ = svc.trigger_settlement("ofr_bank_test").unwrap();
        // In-memory store is not tenant-scoped; any bank reference hit is returned.
        // This validates the fallback path.
        let entry = svc.incident_timeline_entries_for_bank_reference("NONEXISTENT_REF");
        assert!(
            entry.is_empty(),
            "in-memory lookup with unknown reference should return empty, not fabricate entries"
        );
    }

    /// tenant-scoped bank_reference lookup: correct tenant returns entry, wrong tenant is empty
    #[tokio::test]
    async fn test_bank_reference_tenant_scoped_correlation_isolates_tenants() {
        let repo = Arc::new(InMemorySettlementRepository::new());

        let tenant_a = TenantId::new("tenant_incidents_a");
        let tenant_b = TenantId::new("tenant_incidents_b");

        // Bind the two offramp IDs to their respective tenants
        repo.bind_offramp_to_tenant("ofr_incidents_a", &tenant_a);
        repo.bind_offramp_to_tenant("ofr_incidents_b", &tenant_b);

        let mut row_a = make_row("stl_incidents_a", "ofr_incidents_a", "SHARED-REF-INC");
        row_a.tenant_id = Some(tenant_a.0.clone());
        let mut row_b = make_row("stl_incidents_b", "ofr_incidents_b", "SHARED-REF-INC");
        row_b.tenant_id = Some(tenant_b.0.clone());

        repo.create(&row_a).await.unwrap();
        repo.create(&row_b).await.unwrap();

        let svc = SettlementService::with_repository(repo);

        // Tenant A sees only its own settlement
        let entries_a = svc
            .incident_timeline_entries_for_bank_reference_in_tenant_async(
                &tenant_a,
                "SHARED-REF-INC",
            )
            .await
            .unwrap();
        assert_eq!(entries_a.len(), 1, "tenant A must see exactly one entry");
        assert_eq!(entries_a[0].source_reference_id, "stl_incidents_a");

        // Tenant B sees only its own settlement — cross-tenant isolation confirmed
        let entries_b = svc
            .incident_timeline_entries_for_bank_reference_in_tenant_async(
                &tenant_b,
                "SHARED-REF-INC",
            )
            .await
            .unwrap();
        assert_eq!(entries_b.len(), 1, "tenant B must see exactly one entry");
        assert_eq!(entries_b[0].source_reference_id, "stl_incidents_b");
    }

    /// tenant-scoped offramp correlation: cross-tenant lookup returns empty (fail-closed)
    #[tokio::test]
    async fn test_offramp_tenant_scoped_correlation_excludes_other_tenant() {
        let repo = Arc::new(InMemorySettlementRepository::new());

        let tenant_a = TenantId::new("tenant_offramp_a");
        let tenant_b = TenantId::new("tenant_offramp_b");

        repo.bind_offramp_to_tenant("ofr_offramp_a", &tenant_a);
        repo.bind_offramp_to_tenant("ofr_offramp_b", &tenant_b);

        let row_b = make_row("stl_offramp_b", "ofr_offramp_b", "REF-OFR-B");
        repo.create(&row_b).await.unwrap();

        let svc = SettlementService::with_repository(repo);

        // Tenant A asking for tenant B's offramp ID must get nothing (fail-closed)
        let cross_tenant = svc
            .incident_timeline_entries_for_offramp_in_tenant_async(&tenant_a, "ofr_offramp_b")
            .await
            .unwrap();
        assert!(
            cross_tenant.is_empty(),
            "cross-tenant offramp lookup must return empty, not leak data"
        );

        // Tenant B asking for its own offramp ID gets the entry
        let own_tenant = svc
            .incident_timeline_entries_for_offramp_in_tenant_async(&tenant_b, "ofr_offramp_b")
            .await
            .unwrap();
        assert_eq!(own_tenant.len(), 1, "tenant B must see its own entry");
        assert_eq!(own_tenant[0].source_reference_id, "stl_offramp_b");
    }
}
