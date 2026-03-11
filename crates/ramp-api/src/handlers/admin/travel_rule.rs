use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use axum::{
    extract::{Extension, Path, Query},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use ramp_compliance::{
    DisclosureLifecycleEvent, DisclosureLifecycleStage, DisclosureStateMachine,
    DisclosureTransitionRequest, ExceptionQueueStatus, TransportAttemptStatus, TravelRuleDirection,
    VaspInteroperabilityUpdate, VaspRegistryRecord, VaspRegistryRecordInput, VaspRegistryService,
    VaspReviewUpdate,
};

static TRAVEL_RULE_STATE: OnceLock<RwLock<HashMap<String, TenantTravelRuleState>>> =
    OnceLock::new();

#[derive(Debug, Default, Clone)]
struct TenantTravelRuleState {
    registry: BTreeMap<String, VaspRegistryRecord>,
    disclosures: BTreeMap<String, TravelRuleDisclosureRecord>,
    exceptions: BTreeMap<String, TravelRuleExceptionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRuleDisclosureRecord {
    pub disclosure_id: String,
    pub direction: TravelRuleDirection,
    pub stage: DisclosureLifecycleStage,
    pub queue_status: Option<ExceptionQueueStatus>,
    pub failure_count: u32,
    pub max_failures_before_exception: u32,
    pub attempt_count: u32,
    pub transport_profile: Option<String>,
    pub matched_policy_code: Option<String>,
    pub action: Option<String>,
    pub unmet_requirements: Vec<String>,
    pub retry_recommended: bool,
    pub terminal: bool,
    pub last_attempt_status: Option<TransportAttemptStatus>,
    pub metadata: Value,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRuleExceptionRecord {
    pub exception_id: String,
    pub disclosure_id: String,
    pub status: ExceptionQueueStatus,
    pub reason_code: String,
    pub resolution_note: Option<String>,
    pub resolved_by: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRegistryQuery {
    pub review_status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTravelRuleDisclosureRequest {
    pub disclosure_id: String,
    pub direction: String,
    pub transport_profile: Option<String>,
    pub matched_policy_code: Option<String>,
    pub action: Option<String>,
    #[serde(default = "default_failure_threshold")]
    pub max_failures_before_exception: u32,
    #[serde(default = "default_metadata")]
    pub metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDisclosuresQuery {
    pub stage: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryDisclosureRequest {
    pub simulated_status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListExceptionsQuery {
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveExceptionRequest {
    pub resolution_note: Option<String>,
}

fn default_limit() -> usize {
    20
}

fn default_failure_threshold() -> u32 {
    3
}

fn default_metadata() -> Value {
    serde_json::json!({})
}

fn state_store() -> &'static RwLock<HashMap<String, TenantTravelRuleState>> {
    TRAVEL_RULE_STATE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub async fn list_registry(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<ListRegistryQuery>,
) -> Result<Json<Vec<VaspRegistryRecord>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let state = state_store().read().await;
    let mut rows: Vec<_> = state
        .get(&tenant_ctx.tenant_id.0)
        .map(|tenant| tenant.registry.values().cloned().collect())
        .unwrap_or_default();

    if let Some(review_status) = query.review_status.as_deref() {
        let review_status = review_status.to_ascii_uppercase();
        rows.retain(|row| format!("{:?}", row.review.status).to_ascii_uppercase() == review_status);
    }

    rows.truncate(query.limit.clamp(1, 100));
    Ok(Json(rows))
}

pub async fn create_registry_record(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(request): Json<VaspRegistryRecordInput>,
) -> Result<Json<VaspRegistryRecord>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;
    let record = VaspRegistryService::register(request)
        .map_err(|error| ApiError::Validation(error.to_string()))?;

    let mut state = state_store().write().await;
    let tenant = state.entry(tenant_ctx.tenant_id.0.clone()).or_default();
    tenant
        .registry
        .insert(record.vasp_code.clone(), record.clone());

    info!(tenant = %tenant_ctx.tenant_id, vasp_code = %record.vasp_code, "Admin: created travel rule registry record");
    Ok(Json(record))
}

pub async fn review_registry_record(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(vasp_code): Path<String>,
    Json(request): Json<VaspReviewUpdate>,
) -> Result<Json<VaspRegistryRecord>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;

    let mut state = state_store().write().await;
    let tenant = state.entry(tenant_ctx.tenant_id.0.clone()).or_default();
    let record =
        tenant.registry.get(&vasp_code).cloned().ok_or_else(|| {
            ApiError::NotFound(format!("Travel Rule VASP '{}' not found", vasp_code))
        })?;
    let updated = VaspRegistryService::apply_review_update(&record, &request)
        .map_err(|error| ApiError::Validation(error.to_string()))?;
    tenant.registry.insert(vasp_code.clone(), updated.clone());
    Ok(Json(updated))
}

pub async fn update_registry_interoperability(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(vasp_code): Path<String>,
    Json(request): Json<VaspInteroperabilityUpdate>,
) -> Result<Json<VaspRegistryRecord>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;

    let mut state = state_store().write().await;
    let tenant = state.entry(tenant_ctx.tenant_id.0.clone()).or_default();
    let record =
        tenant.registry.get(&vasp_code).cloned().ok_or_else(|| {
            ApiError::NotFound(format!("Travel Rule VASP '{}' not found", vasp_code))
        })?;
    let updated = VaspRegistryService::apply_interoperability_update(&record, &request)
        .map_err(|error| ApiError::Validation(error.to_string()))?;
    tenant.registry.insert(vasp_code.clone(), updated.clone());
    Ok(Json(updated))
}

pub async fn list_disclosures(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<ListDisclosuresQuery>,
) -> Result<Json<Vec<TravelRuleDisclosureRecord>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let state = state_store().read().await;
    let mut rows: Vec<_> = state
        .get(&tenant_ctx.tenant_id.0)
        .map(|tenant| tenant.disclosures.values().cloned().collect())
        .unwrap_or_default();

    if let Some(stage) = query.stage.as_deref() {
        let stage = stage.to_ascii_uppercase();
        rows.retain(|row| format!("{:?}", row.stage).to_ascii_uppercase() == stage);
    }

    rows.truncate(query.limit.clamp(1, 100));
    Ok(Json(rows))
}

pub async fn create_disclosure(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(request): Json<CreateTravelRuleDisclosureRequest>,
) -> Result<Json<TravelRuleDisclosureRecord>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;
    ensure_object(&request.metadata, "metadata")?;

    let direction = parse_direction(&request.direction)?;
    let transition = DisclosureStateMachine::transition(&DisclosureTransitionRequest {
        current_stage: DisclosureLifecycleStage::Pending,
        event: DisclosureLifecycleEvent::MarkReady {
            transport_profile: request.transport_profile.clone(),
        },
        failure_count: 0,
        max_failures_before_exception: request.max_failures_before_exception.max(1),
    })
    .map_err(|error| ApiError::Validation(error.to_string()))?;

    let disclosure = TravelRuleDisclosureRecord {
        disclosure_id: request.disclosure_id.clone(),
        direction,
        stage: transition.stage,
        queue_status: transition.queue_status,
        failure_count: transition.failure_count,
        max_failures_before_exception: request.max_failures_before_exception.max(1),
        attempt_count: 0,
        transport_profile: request.transport_profile.clone(),
        matched_policy_code: request.matched_policy_code.clone(),
        action: request.action.clone(),
        unmet_requirements: transition
            .unmet_requirements
            .into_iter()
            .map(|value| format!("{:?}", value))
            .collect(),
        retry_recommended: transition.retry_recommended,
        terminal: transition.terminal,
        last_attempt_status: None,
        metadata: request.metadata,
        updated_at: Utc::now().to_rfc3339(),
    };

    let mut state = state_store().write().await;
    let tenant = state.entry(tenant_ctx.tenant_id.0.clone()).or_default();
    tenant
        .disclosures
        .insert(disclosure.disclosure_id.clone(), disclosure.clone());

    Ok(Json(disclosure))
}

pub async fn retry_disclosure(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(disclosure_id): Path<String>,
    Json(request): Json<RetryDisclosureRequest>,
) -> Result<Json<TravelRuleDisclosureRecord>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;

    let simulated_status = parse_transport_status(request.simulated_status.as_deref())?;
    let mut state = state_store().write().await;
    let tenant = state.entry(tenant_ctx.tenant_id.0.clone()).or_default();
    let disclosure = tenant
        .disclosures
        .get(&disclosure_id)
        .cloned()
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Travel Rule disclosure '{}' not found",
                disclosure_id
            ))
        })?;

    let mut next = disclosure.clone();
    if next.stage == DisclosureLifecycleStage::Exception {
        let resolved = DisclosureStateMachine::transition(&DisclosureTransitionRequest {
            current_stage: next.stage,
            event: DisclosureLifecycleEvent::ExceptionQueueUpdated {
                status: ExceptionQueueStatus::Resolved,
            },
            failure_count: next.failure_count,
            max_failures_before_exception: next.max_failures_before_exception,
        })
        .map_err(|error| ApiError::Validation(error.to_string()))?;
        next.stage = resolved.stage;
        next.queue_status = resolved.queue_status;
        next.retry_recommended = resolved.retry_recommended;
        next.terminal = resolved.terminal;
    }

    let ready = DisclosureStateMachine::transition(&DisclosureTransitionRequest {
        current_stage: next.stage,
        event: DisclosureLifecycleEvent::MarkReady {
            transport_profile: next.transport_profile.clone(),
        },
        failure_count: next.failure_count,
        max_failures_before_exception: next.max_failures_before_exception,
    })
    .map_err(|error| ApiError::Validation(error.to_string()))?;
    next.stage = ready.stage;
    next.queue_status = ready.queue_status;
    next.failure_count = ready.failure_count;
    next.unmet_requirements = ready
        .unmet_requirements
        .into_iter()
        .map(|value| format!("{:?}", value))
        .collect();
    next.retry_recommended = ready.retry_recommended;
    next.terminal = ready.terminal;

    let attempt_number = next.attempt_count + 1;
    let dispatched = DisclosureStateMachine::transition(&DisclosureTransitionRequest {
        current_stage: next.stage,
        event: DisclosureLifecycleEvent::TransportUpdated {
            status: simulated_status,
            attempt_number,
        },
        failure_count: next.failure_count,
        max_failures_before_exception: next.max_failures_before_exception,
    })
    .map_err(|error| ApiError::Validation(error.to_string()))?;

    next.stage = dispatched.stage;
    next.queue_status = dispatched.queue_status;
    next.failure_count = dispatched.failure_count;
    let entered_exception_queue = dispatched.entered_exception_queue();
    next.unmet_requirements = dispatched
        .unmet_requirements
        .into_iter()
        .map(|value| format!("{:?}", value))
        .collect();
    next.retry_recommended = dispatched.retry_recommended;
    next.terminal = dispatched.terminal;
    next.last_attempt_status = Some(simulated_status);
    next.attempt_count = attempt_number;
    next.updated_at = Utc::now().to_rfc3339();

    if entered_exception_queue {
        let exception_id = format!("tre_{}", disclosure_id);
        tenant.exceptions.insert(
            exception_id.clone(),
            TravelRuleExceptionRecord {
                exception_id,
                disclosure_id: disclosure_id.clone(),
                status: ExceptionQueueStatus::Open,
                reason_code: format!("{:?}", simulated_status).to_ascii_uppercase(),
                resolution_note: None,
                resolved_by: None,
                updated_at: Utc::now().to_rfc3339(),
            },
        );
    }

    tenant.disclosures.insert(disclosure_id, next.clone());
    Ok(Json(next))
}

pub async fn list_exceptions(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<ListExceptionsQuery>,
) -> Result<Json<Vec<TravelRuleExceptionRecord>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let state = state_store().read().await;
    let mut rows: Vec<_> = state
        .get(&tenant_ctx.tenant_id.0)
        .map(|tenant| tenant.exceptions.values().cloned().collect())
        .unwrap_or_default();

    if let Some(status) = query.status.as_deref() {
        let status = status.to_ascii_uppercase();
        rows.retain(|row| format!("{:?}", row.status).to_ascii_uppercase() == status);
    }

    rows.truncate(query.limit.clamp(1, 100));
    Ok(Json(rows))
}

pub async fn resolve_exception(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(exception_id): Path<String>,
    Json(request): Json<ResolveExceptionRequest>,
) -> Result<Json<TravelRuleExceptionRecord>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;

    let mut state = state_store().write().await;
    let tenant = state.entry(tenant_ctx.tenant_id.0.clone()).or_default();
    let exception = tenant
        .exceptions
        .get(&exception_id)
        .cloned()
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "Travel Rule exception '{}' not found",
                exception_id
            ))
        })?;

    if let Some(disclosure) = tenant.disclosures.get(&exception.disclosure_id).cloned() {
        let resolved = DisclosureStateMachine::transition(&DisclosureTransitionRequest {
            current_stage: disclosure.stage,
            event: DisclosureLifecycleEvent::ExceptionQueueUpdated {
                status: ExceptionQueueStatus::Resolved,
            },
            failure_count: disclosure.failure_count,
            max_failures_before_exception: disclosure.max_failures_before_exception,
        })
        .map_err(|error| ApiError::Validation(error.to_string()))?;

        let mut next_disclosure = disclosure;
        next_disclosure.stage = resolved.stage;
        next_disclosure.queue_status = resolved.queue_status;
        next_disclosure.retry_recommended = resolved.retry_recommended;
        next_disclosure.terminal = resolved.terminal;
        next_disclosure.updated_at = Utc::now().to_rfc3339();
        tenant
            .disclosures
            .insert(next_disclosure.disclosure_id.clone(), next_disclosure);
    }

    let updated = TravelRuleExceptionRecord {
        status: ExceptionQueueStatus::Resolved,
        resolution_note: request.resolution_note,
        resolved_by: auth.user_id,
        updated_at: Utc::now().to_rfc3339(),
        ..exception
    };
    tenant
        .exceptions
        .insert(exception_id.clone(), updated.clone());

    Ok(Json(updated))
}

fn parse_direction(value: &str) -> Result<TravelRuleDirection, ApiError> {
    match value.trim().to_ascii_uppercase().as_str() {
        "OUTBOUND" => Ok(TravelRuleDirection::Outbound),
        "INBOUND" => Ok(TravelRuleDirection::Inbound),
        other => Err(ApiError::Validation(format!(
            "Unsupported travel rule direction '{}'",
            other
        ))),
    }
}

fn parse_transport_status(value: Option<&str>) -> Result<TransportAttemptStatus, ApiError> {
    match value.unwrap_or("SENT").trim().to_ascii_uppercase().as_str() {
        "PENDING" => Ok(TransportAttemptStatus::Pending),
        "SENT" => Ok(TransportAttemptStatus::Sent),
        "ACKNOWLEDGED" => Ok(TransportAttemptStatus::Acknowledged),
        "FAILED" => Ok(TransportAttemptStatus::Failed),
        "TIMEOUT" => Ok(TransportAttemptStatus::Timeout),
        "REJECTED" => Ok(TransportAttemptStatus::Rejected),
        other => Err(ApiError::Validation(format!(
            "Unsupported travel rule transport status '{}'",
            other
        ))),
    }
}

fn ensure_object(value: &Value, field: &str) -> Result<(), ApiError> {
    if value.is_object() {
        Ok(())
    } else {
        Err(ApiError::Validation(format!(
            "{field} must be a JSON object"
        )))
    }
}
