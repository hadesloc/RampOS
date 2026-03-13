use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use tokio::sync::RwLock;
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::AppState;
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


const REVIEW_META_KEY: &str = "_reviewState";
const INTEROP_META_KEY: &str = "_interoperabilityState";
const DISCLOSURE_MATCHED_POLICY_KEY: &str = "_matchedPolicyCode";
const DISCLOSURE_ACTION_KEY: &str = "_action";
const DISCLOSURE_MAX_FAILURES_KEY: &str = "_maxFailuresBeforeException";
const DISCLOSURE_FAILURE_COUNT_KEY: &str = "_failureCount";
const DISCLOSURE_ATTEMPT_COUNT_KEY: &str = "_attemptCount";
const DISCLOSURE_LAST_ATTEMPT_STATUS_KEY: &str = "_lastAttemptStatus";
const DISCLOSURE_QUEUE_STATUS_KEY: &str = "_queueStatus";
const DISCLOSURE_UNMET_REQUIREMENTS_KEY: &str = "_unmetRequirements";
const DISCLOSURE_RETRY_RECOMMENDED_KEY: &str = "_retryRecommended";
const DISCLOSURE_TERMINAL_KEY: &str = "_terminal";
const EXCEPTION_RESOLVED_BY_KEY: &str = "_resolvedBy";

#[derive(Debug, Clone, FromRow)]
struct TravelRuleVaspRow {
    vasp_code: String,
    legal_name: String,
    display_name: Option<String>,
    jurisdiction_code: Option<String>,
    registration_number: Option<String>,
    travel_rule_profile: Option<String>,
    transport_profile: Option<String>,
    endpoint_uri: Option<String>,
    endpoint_public_key: Option<String>,
    review_status: String,
    interoperability_status: String,
    supports_inbound: bool,
    supports_outbound: bool,
    metadata: Value,
}

#[derive(Debug, Clone, FromRow)]
struct TravelRuleDisclosureRow {
    id: String,
    direction: String,
    lifecycle_stage: String,
    transport_profile: Option<String>,
    metadata: Value,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct TravelRuleAttemptRow {
    attempt_number: i32,
    status: String,
}

#[derive(Debug, Clone, FromRow)]
struct TravelRuleExceptionRow {
    id: String,
    disclosure_id: String,
    queue_status: String,
    reason_code: String,
    resolution_notes: Option<String>,
    metadata: Value,
    updated_at: DateTime<Utc>,
}

fn db_error(context: &str, error: sqlx::Error) -> ApiError {
    ApiError::Internal(format!("Travel Rule persistence failed for {context}: {error}"))
}

fn sanitize_id(input: &str) -> String {
    input
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '_' })
        .collect()
}

fn metadata_map(value: &Value) -> serde_json::Map<String, Value> {
    match value {
        Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    }
}

fn strip_keys(value: &Value, reserved_keys: &[&str]) -> Value {
    let mut map = metadata_map(value);
    for key in reserved_keys {
        map.remove(*key);
    }
    Value::Object(map)
}

fn registry_metadata_for_storage(
    metadata: &Value,
    review: &ramp_compliance::VaspReviewState,
    interoperability: &ramp_compliance::VaspInteroperabilityState,
) -> Result<Value, ApiError> {
    let mut map = metadata_map(metadata);
    map.insert(
        REVIEW_META_KEY.to_string(),
        serde_json::to_value(review)
            .map_err(|error| ApiError::Internal(format!("Travel Rule review serialization failed: {error}")))?,
    );
    map.insert(
        INTEROP_META_KEY.to_string(),
        serde_json::to_value(interoperability)
            .map_err(|error| ApiError::Internal(format!("Travel Rule interoperability serialization failed: {error}")))?,
    );
    Ok(Value::Object(map))
}

fn disclosure_metadata_for_storage(record: &TravelRuleDisclosureRecord) -> Value {
    let mut map = metadata_map(&record.metadata);
    map.insert(
        DISCLOSURE_MATCHED_POLICY_KEY.to_string(),
        record
            .matched_policy_code
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    map.insert(
        DISCLOSURE_ACTION_KEY.to_string(),
        record.action.clone().map(Value::String).unwrap_or(Value::Null),
    );
    map.insert(
        DISCLOSURE_MAX_FAILURES_KEY.to_string(),
        Value::from(record.max_failures_before_exception),
    );
    map.insert(
        DISCLOSURE_FAILURE_COUNT_KEY.to_string(),
        Value::from(record.failure_count),
    );
    map.insert(
        DISCLOSURE_ATTEMPT_COUNT_KEY.to_string(),
        Value::from(record.attempt_count),
    );
    map.insert(
        DISCLOSURE_LAST_ATTEMPT_STATUS_KEY.to_string(),
        record
            .last_attempt_status
            .map(transport_status_to_value)
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        DISCLOSURE_QUEUE_STATUS_KEY.to_string(),
        record
            .queue_status
            .map(exception_status_to_value)
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        DISCLOSURE_UNMET_REQUIREMENTS_KEY.to_string(),
        Value::Array(
            record
                .unmet_requirements
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
    map.insert(
        DISCLOSURE_RETRY_RECOMMENDED_KEY.to_string(),
        Value::Bool(record.retry_recommended),
    );
    map.insert(DISCLOSURE_TERMINAL_KEY.to_string(), Value::Bool(record.terminal));
    Value::Object(map)
}

fn exception_metadata_for_storage(
    metadata: &Value,
    resolved_by: Option<&str>,
) -> Value {
    let mut map = metadata_map(metadata);
    map.insert(
        EXCEPTION_RESOLVED_BY_KEY.to_string(),
        resolved_by
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
    );
    Value::Object(map)
}

fn parse_u32_control(metadata: &Value, key: &str) -> Option<u32> {
    metadata.get(key).and_then(Value::as_u64).map(|value| value as u32)
}

fn parse_bool_control(metadata: &Value, key: &str) -> Option<bool> {
    metadata.get(key).and_then(Value::as_bool)
}

fn parse_string_control(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn parse_string_vec_control(metadata: &Value, key: &str) -> Vec<String> {
    metadata
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn review_status_to_value(status: ramp_compliance::VaspReviewStatus) -> &'static str {
    match status {
        ramp_compliance::VaspReviewStatus::Pending => "PENDING",
        ramp_compliance::VaspReviewStatus::Approved => "APPROVED",
        ramp_compliance::VaspReviewStatus::Rejected => "REJECTED",
        ramp_compliance::VaspReviewStatus::Suspended => "SUSPENDED",
    }
}

fn parse_review_status(value: &str) -> Result<ramp_compliance::VaspReviewStatus, ApiError> {
    match value.trim().to_ascii_uppercase().as_str() {
        "PENDING" => Ok(ramp_compliance::VaspReviewStatus::Pending),
        "APPROVED" => Ok(ramp_compliance::VaspReviewStatus::Approved),
        "REJECTED" => Ok(ramp_compliance::VaspReviewStatus::Rejected),
        "SUSPENDED" => Ok(ramp_compliance::VaspReviewStatus::Suspended),
        other => Err(ApiError::Internal(format!("Unsupported persisted review status '{other}'"))),
    }
}

fn interoperability_status_to_value(
    status: ramp_compliance::VaspInteroperabilityStatus,
) -> &'static str {
    match status {
        ramp_compliance::VaspInteroperabilityStatus::Unknown => "UNKNOWN",
        ramp_compliance::VaspInteroperabilityStatus::Ready => "READY",
        ramp_compliance::VaspInteroperabilityStatus::Limited => "LIMITED",
        ramp_compliance::VaspInteroperabilityStatus::Degraded => "DEGRADED",
        ramp_compliance::VaspInteroperabilityStatus::Disabled => "DISABLED",
    }
}

fn parse_interoperability_status(
    value: &str,
) -> Result<ramp_compliance::VaspInteroperabilityStatus, ApiError> {
    match value.trim().to_ascii_uppercase().as_str() {
        "UNKNOWN" => Ok(ramp_compliance::VaspInteroperabilityStatus::Unknown),
        "READY" => Ok(ramp_compliance::VaspInteroperabilityStatus::Ready),
        "LIMITED" => Ok(ramp_compliance::VaspInteroperabilityStatus::Limited),
        "DEGRADED" => Ok(ramp_compliance::VaspInteroperabilityStatus::Degraded),
        "DISABLED" => Ok(ramp_compliance::VaspInteroperabilityStatus::Disabled),
        other => Err(ApiError::Internal(format!(
            "Unsupported persisted interoperability status '{other}'"
        ))),
    }
}

fn disclosure_stage_to_value(stage: DisclosureLifecycleStage) -> &'static str {
    match stage {
        DisclosureLifecycleStage::Pending => "PENDING",
        DisclosureLifecycleStage::Ready => "READY",
        DisclosureLifecycleStage::Sent => "SENT",
        DisclosureLifecycleStage::Acknowledged => "ACKNOWLEDGED",
        DisclosureLifecycleStage::Failed => "FAILED",
        DisclosureLifecycleStage::Exception => "EXCEPTION",
        DisclosureLifecycleStage::Waived => "WAIVED",
    }
}

fn parse_disclosure_stage(value: &str) -> Result<DisclosureLifecycleStage, ApiError> {
    match value.trim().to_ascii_uppercase().as_str() {
        "PENDING" => Ok(DisclosureLifecycleStage::Pending),
        "READY" => Ok(DisclosureLifecycleStage::Ready),
        "SENT" => Ok(DisclosureLifecycleStage::Sent),
        "ACKNOWLEDGED" => Ok(DisclosureLifecycleStage::Acknowledged),
        "FAILED" => Ok(DisclosureLifecycleStage::Failed),
        "EXCEPTION" => Ok(DisclosureLifecycleStage::Exception),
        "WAIVED" => Ok(DisclosureLifecycleStage::Waived),
        other => Err(ApiError::Internal(format!("Unsupported persisted disclosure stage '{other}'"))),
    }
}

fn transport_status_to_value(status: TransportAttemptStatus) -> &'static str {
    match status {
        TransportAttemptStatus::Pending => "PENDING",
        TransportAttemptStatus::Sent => "SENT",
        TransportAttemptStatus::Acknowledged => "ACKNOWLEDGED",
        TransportAttemptStatus::Failed => "FAILED",
        TransportAttemptStatus::Timeout => "TIMEOUT",
        TransportAttemptStatus::Rejected => "REJECTED",
    }
}

fn exception_status_to_value(status: ExceptionQueueStatus) -> &'static str {
    match status {
        ExceptionQueueStatus::Open => "OPEN",
        ExceptionQueueStatus::InReview => "IN_REVIEW",
        ExceptionQueueStatus::Escalated => "ESCALATED",
        ExceptionQueueStatus::Resolved => "RESOLVED",
        ExceptionQueueStatus::Dismissed => "DISMISSED",
    }
}

fn parse_exception_status(value: &str) -> Result<ExceptionQueueStatus, ApiError> {
    match value.trim().to_ascii_uppercase().as_str() {
        "OPEN" => Ok(ExceptionQueueStatus::Open),
        "IN_REVIEW" => Ok(ExceptionQueueStatus::InReview),
        "ESCALATED" => Ok(ExceptionQueueStatus::Escalated),
        "RESOLVED" => Ok(ExceptionQueueStatus::Resolved),
        "DISMISSED" => Ok(ExceptionQueueStatus::Dismissed),
        other => Err(ApiError::Internal(format!(
            "Unsupported persisted exception status '{other}'"
        ))),
    }
}

fn vasp_row_to_record(row: TravelRuleVaspRow) -> Result<VaspRegistryRecord, ApiError> {
    let public_metadata = strip_keys(&row.metadata, &[REVIEW_META_KEY, INTEROP_META_KEY]);

    let mut review = row
        .metadata
        .get(REVIEW_META_KEY)
        .cloned()
        .and_then(|value| serde_json::from_value::<ramp_compliance::VaspReviewState>(value).ok())
        .unwrap_or_default();
    review.status = parse_review_status(&row.review_status)?;

    let mut interoperability = row
        .metadata
        .get(INTEROP_META_KEY)
        .cloned()
        .and_then(|value| serde_json::from_value::<ramp_compliance::VaspInteroperabilityState>(value).ok())
        .unwrap_or_default();
    interoperability.status = parse_interoperability_status(&row.interoperability_status)?;

    Ok(VaspRegistryRecord {
        vasp_code: row.vasp_code,
        legal_name: row.legal_name,
        display_name: row.display_name,
        jurisdiction_code: row.jurisdiction_code,
        registration_number: row.registration_number,
        travel_rule_profile: row.travel_rule_profile,
        transport_profile: row.transport_profile,
        endpoint_uri: row.endpoint_uri,
        endpoint_public_key: row.endpoint_public_key,
        review,
        interoperability,
        supports_inbound: row.supports_inbound,
        supports_outbound: row.supports_outbound,
        metadata: public_metadata,
    })
}

fn exception_row_to_record(row: TravelRuleExceptionRow) -> Result<TravelRuleExceptionRecord, ApiError> {
    Ok(TravelRuleExceptionRecord {
        exception_id: row.id,
        disclosure_id: row.disclosure_id,
        status: parse_exception_status(&row.queue_status)?,
        reason_code: row.reason_code,
        resolution_note: row.resolution_notes,
        resolved_by: parse_string_control(&row.metadata, EXCEPTION_RESOLVED_BY_KEY),
        updated_at: row.updated_at.to_rfc3339(),
    })
}

fn disclosure_row_to_record(
    row: TravelRuleDisclosureRow,
    latest_attempt: Option<TravelRuleAttemptRow>,
    exception: Option<TravelRuleExceptionRow>,
) -> Result<TravelRuleDisclosureRecord, ApiError> {
    let public_metadata = strip_keys(
        &row.metadata,
        &[
            DISCLOSURE_MATCHED_POLICY_KEY,
            DISCLOSURE_ACTION_KEY,
            DISCLOSURE_MAX_FAILURES_KEY,
            DISCLOSURE_FAILURE_COUNT_KEY,
            DISCLOSURE_ATTEMPT_COUNT_KEY,
            DISCLOSURE_LAST_ATTEMPT_STATUS_KEY,
            DISCLOSURE_QUEUE_STATUS_KEY,
            DISCLOSURE_UNMET_REQUIREMENTS_KEY,
            DISCLOSURE_RETRY_RECOMMENDED_KEY,
            DISCLOSURE_TERMINAL_KEY,
        ],
    );

    let latest_attempt_status = latest_attempt
        .as_ref()
        .map(|attempt| parse_transport_status(Some(&attempt.status)))
        .transpose()?
        .or_else(|| {
            parse_string_control(&row.metadata, DISCLOSURE_LAST_ATTEMPT_STATUS_KEY)
                .and_then(|value| parse_transport_status(Some(&value)).ok())
        });

    let attempt_count = latest_attempt
        .as_ref()
        .map(|attempt| attempt.attempt_number.max(0) as u32)
        .or_else(|| parse_u32_control(&row.metadata, DISCLOSURE_ATTEMPT_COUNT_KEY))
        .unwrap_or_default();

    let queue_status = if let Some(exception_row) = exception.as_ref() {
        Some(parse_exception_status(&exception_row.queue_status)?)
    } else {
        parse_string_control(&row.metadata, DISCLOSURE_QUEUE_STATUS_KEY)
            .map(|value| parse_exception_status(&value))
            .transpose()?
    };

    Ok(TravelRuleDisclosureRecord {
        disclosure_id: row.id,
        direction: parse_direction(&row.direction)?,
        stage: parse_disclosure_stage(&row.lifecycle_stage)?,
        queue_status,
        failure_count: parse_u32_control(&row.metadata, DISCLOSURE_FAILURE_COUNT_KEY).unwrap_or_default(),
        max_failures_before_exception: parse_u32_control(&row.metadata, DISCLOSURE_MAX_FAILURES_KEY)
            .unwrap_or(1),
        attempt_count,
        transport_profile: row.transport_profile,
        matched_policy_code: parse_string_control(&row.metadata, DISCLOSURE_MATCHED_POLICY_KEY),
        action: parse_string_control(&row.metadata, DISCLOSURE_ACTION_KEY),
        unmet_requirements: parse_string_vec_control(&row.metadata, DISCLOSURE_UNMET_REQUIREMENTS_KEY),
        retry_recommended: parse_bool_control(&row.metadata, DISCLOSURE_RETRY_RECOMMENDED_KEY)
            .unwrap_or(false),
        terminal: parse_bool_control(&row.metadata, DISCLOSURE_TERMINAL_KEY).unwrap_or(false),
        last_attempt_status: latest_attempt_status,
        metadata: public_metadata,
        updated_at: row.updated_at.to_rfc3339(),
    })
}

async fn load_latest_attempt(
    pool: &PgPool,
    tenant_id: &str,
    disclosure_id: &str,
) -> Result<Option<TravelRuleAttemptRow>, ApiError> {
    sqlx::query_as::<_, TravelRuleAttemptRow>(
        r#"
        SELECT attempt_number, status
        FROM travel_rule_transport_attempts
        WHERE tenant_id = $1 AND disclosure_id = $2
        ORDER BY attempt_number DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(disclosure_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| db_error("load travel rule attempt", error))
}

async fn load_exception_for_disclosure(
    pool: &PgPool,
    tenant_id: &str,
    disclosure_id: &str,
) -> Result<Option<TravelRuleExceptionRow>, ApiError> {
    sqlx::query_as::<_, TravelRuleExceptionRow>(
        r#"
        SELECT id, disclosure_id, queue_status, reason_code, resolution_notes, metadata, updated_at
        FROM travel_rule_exception_queue
        WHERE tenant_id = $1 AND disclosure_id = $2
        ORDER BY updated_at DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(disclosure_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| db_error("load travel rule exception", error))
}

async fn fetch_disclosure_db(
    pool: &PgPool,
    tenant_id: &str,
    disclosure_id: &str,
) -> Result<TravelRuleDisclosureRecord, ApiError> {
    let row = sqlx::query_as::<_, TravelRuleDisclosureRow>(
        r#"
        SELECT id, direction, lifecycle_stage, transport_profile, metadata, updated_at
        FROM travel_rule_disclosures
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(disclosure_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| db_error("load travel rule disclosure", error))?
    .ok_or_else(|| ApiError::NotFound(format!("Travel Rule disclosure '{}' not found", disclosure_id)))?;

    let latest_attempt = load_latest_attempt(pool, tenant_id, disclosure_id).await?;
    let exception = load_exception_for_disclosure(pool, tenant_id, disclosure_id).await?;
    disclosure_row_to_record(row, latest_attempt, exception)
}

async fn list_registry_db(
    pool: &PgPool,
    tenant_id: &str,
    query: &ListRegistryQuery,
) -> Result<Vec<VaspRegistryRecord>, ApiError> {
    let mut records = sqlx::query_as::<_, TravelRuleVaspRow>(
        r#"
        SELECT
            vasp_code,
            legal_name,
            display_name,
            jurisdiction_code,
            registration_number,
            travel_rule_profile,
            transport_profile,
            endpoint_uri,
            endpoint_public_key,
            review_status,
            interoperability_status,
            supports_inbound,
            supports_outbound,
            metadata
        FROM travel_rule_vasps
        WHERE tenant_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(|error| db_error("list travel rule registry", error))?
    .into_iter()
    .map(vasp_row_to_record)
    .collect::<Result<Vec<_>, _>>()?;

    if let Some(review_status) = query.review_status.as_deref() {
        let review_status = review_status.to_ascii_uppercase();
        records.retain(|row| review_status_to_value(row.review.status) == review_status);
    }

    records.truncate(query.limit.clamp(1, 100));
    Ok(records)
}

async fn create_registry_record_db(
    pool: &PgPool,
    tenant_id: &str,
    request: VaspRegistryRecordInput,
) -> Result<VaspRegistryRecord, ApiError> {
    let record = VaspRegistryService::register(request)
        .map_err(|error| ApiError::Validation(error.to_string()))?;
    let metadata = registry_metadata_for_storage(&record.metadata, &record.review, &record.interoperability)?;

    sqlx::query(
        r#"
        INSERT INTO travel_rule_vasps (
            id,
            tenant_id,
            vasp_code,
            legal_name,
            display_name,
            jurisdiction_code,
            registration_number,
            travel_rule_profile,
            transport_profile,
            endpoint_uri,
            endpoint_public_key,
            review_status,
            interoperability_status,
            supports_inbound,
            supports_outbound,
            metadata
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16
        )
        "#,
    )
    .bind(format!("trv_{}", sanitize_id(&record.vasp_code)))
    .bind(tenant_id)
    .bind(&record.vasp_code)
    .bind(&record.legal_name)
    .bind(&record.display_name)
    .bind(&record.jurisdiction_code)
    .bind(&record.registration_number)
    .bind(&record.travel_rule_profile)
    .bind(&record.transport_profile)
    .bind(&record.endpoint_uri)
    .bind(&record.endpoint_public_key)
    .bind(review_status_to_value(record.review.status))
    .bind(interoperability_status_to_value(record.interoperability.status))
    .bind(record.supports_inbound)
    .bind(record.supports_outbound)
    .bind(metadata)
    .execute(pool)
    .await
    .map_err(|error| db_error("create travel rule registry record", error))?;

    Ok(record)
}

async fn update_registry_review_db(
    pool: &PgPool,
    tenant_id: &str,
    vasp_code: &str,
    request: &VaspReviewUpdate,
) -> Result<VaspRegistryRecord, ApiError> {
    let existing = sqlx::query_as::<_, TravelRuleVaspRow>(
        r#"
        SELECT
            vasp_code,
            legal_name,
            display_name,
            jurisdiction_code,
            registration_number,
            travel_rule_profile,
            transport_profile,
            endpoint_uri,
            endpoint_public_key,
            review_status,
            interoperability_status,
            supports_inbound,
            supports_outbound,
            metadata
        FROM travel_rule_vasps
        WHERE tenant_id = $1 AND vasp_code = $2
        "#,
    )
    .bind(tenant_id)
    .bind(vasp_code)
    .fetch_optional(pool)
    .await
    .map_err(|error| db_error("load travel rule registry record", error))?
    .ok_or_else(|| ApiError::NotFound(format!("Travel Rule VASP '{}' not found", vasp_code)))?;

    let record = vasp_row_to_record(existing)?;
    let updated = VaspRegistryService::apply_review_update(&record, request)
        .map_err(|error| ApiError::Validation(error.to_string()))?;
    let metadata = registry_metadata_for_storage(&updated.metadata, &updated.review, &updated.interoperability)?;

    sqlx::query(
        r#"
        UPDATE travel_rule_vasps
        SET review_status = $3,
            metadata = $4,
            updated_at = NOW()
        WHERE tenant_id = $1 AND vasp_code = $2
        "#,
    )
    .bind(tenant_id)
    .bind(vasp_code)
    .bind(review_status_to_value(updated.review.status))
    .bind(metadata)
    .execute(pool)
    .await
    .map_err(|error| db_error("update travel rule review", error))?;

    Ok(updated)
}

async fn update_registry_interoperability_db(
    pool: &PgPool,
    tenant_id: &str,
    vasp_code: &str,
    request: &VaspInteroperabilityUpdate,
) -> Result<VaspRegistryRecord, ApiError> {
    let existing = sqlx::query_as::<_, TravelRuleVaspRow>(
        r#"
        SELECT
            vasp_code,
            legal_name,
            display_name,
            jurisdiction_code,
            registration_number,
            travel_rule_profile,
            transport_profile,
            endpoint_uri,
            endpoint_public_key,
            review_status,
            interoperability_status,
            supports_inbound,
            supports_outbound,
            metadata
        FROM travel_rule_vasps
        WHERE tenant_id = $1 AND vasp_code = $2
        "#,
    )
    .bind(tenant_id)
    .bind(vasp_code)
    .fetch_optional(pool)
    .await
    .map_err(|error| db_error("load travel rule registry record", error))?
    .ok_or_else(|| ApiError::NotFound(format!("Travel Rule VASP '{}' not found", vasp_code)))?;

    let record = vasp_row_to_record(existing)?;
    let updated = VaspRegistryService::apply_interoperability_update(&record, request)
        .map_err(|error| ApiError::Validation(error.to_string()))?;
    let metadata = registry_metadata_for_storage(&updated.metadata, &updated.review, &updated.interoperability)?;

    sqlx::query(
        r#"
        UPDATE travel_rule_vasps
        SET interoperability_status = $3,
            metadata = $4,
            updated_at = NOW()
        WHERE tenant_id = $1 AND vasp_code = $2
        "#,
    )
    .bind(tenant_id)
    .bind(vasp_code)
    .bind(interoperability_status_to_value(updated.interoperability.status))
    .bind(metadata)
    .execute(pool)
    .await
    .map_err(|error| db_error("update travel rule interoperability", error))?;

    Ok(updated)
}

async fn list_disclosures_db(
    pool: &PgPool,
    tenant_id: &str,
    query: &ListDisclosuresQuery,
) -> Result<Vec<TravelRuleDisclosureRecord>, ApiError> {
    let rows = sqlx::query_as::<_, TravelRuleDisclosureRow>(
        r#"
        SELECT id, direction, lifecycle_stage, transport_profile, metadata, updated_at
        FROM travel_rule_disclosures
        WHERE tenant_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(|error| db_error("list travel rule disclosures", error))?;

    let mut records = Vec::with_capacity(rows.len());
    for row in rows {
        let latest_attempt = load_latest_attempt(pool, tenant_id, &row.id).await?;
        let exception = load_exception_for_disclosure(pool, tenant_id, &row.id).await?;
        records.push(disclosure_row_to_record(row, latest_attempt, exception)?);
    }

    if let Some(stage) = query.stage.as_deref() {
        let stage = stage.to_ascii_uppercase();
        records.retain(|row| disclosure_stage_to_value(row.stage) == stage);
    }

    records.truncate(query.limit.clamp(1, 100));
    Ok(records)
}

async fn create_disclosure_db(
    pool: &PgPool,
    tenant_id: &str,
    request: CreateTravelRuleDisclosureRequest,
) -> Result<TravelRuleDisclosureRecord, ApiError> {
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

    let metadata = disclosure_metadata_for_storage(&disclosure);

    sqlx::query(
        r#"
        INSERT INTO travel_rule_disclosures (
            id,
            tenant_id,
            direction,
            lifecycle_stage,
            asset_symbol,
            asset_amount,
            transport_profile,
            disclosure_payload,
            correlation_id,
            metadata
        ) VALUES (
            $1, $2, $3, $4, 'UNKNOWN', 0, $5, '{}'::jsonb, $6, $7
        )
        "#,
    )
    .bind(&disclosure.disclosure_id)
    .bind(tenant_id)
    .bind(match disclosure.direction {
        TravelRuleDirection::Outbound => "OUTBOUND",
        TravelRuleDirection::Inbound => "INBOUND",
    })
    .bind(disclosure_stage_to_value(disclosure.stage))
    .bind(&disclosure.transport_profile)
    .bind(&disclosure.disclosure_id)
    .bind(metadata)
    .execute(pool)
    .await
    .map_err(|error| db_error("create travel rule disclosure", error))?;

    Ok(disclosure)
}

async fn retry_disclosure_db(
    pool: &PgPool,
    tenant_id: &str,
    disclosure_id: &str,
    request: RetryDisclosureRequest,
) -> Result<TravelRuleDisclosureRecord, ApiError> {
    let simulated_status = parse_transport_status(request.simulated_status.as_deref())?;
    let disclosure = fetch_disclosure_db(pool, tenant_id, disclosure_id).await?;

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

        sqlx::query(
            r#"
            UPDATE travel_rule_exception_queue
            SET queue_status = 'RESOLVED',
                resolved_at = NOW(),
                updated_at = NOW()
            WHERE tenant_id = $1 AND disclosure_id = $2 AND queue_status <> 'RESOLVED'
            "#,
        )
        .bind(tenant_id)
        .bind(disclosure_id)
        .execute(pool)
        .await
        .map_err(|error| db_error("resolve stale travel rule exception", error))?;
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

    sqlx::query(
        r#"
        INSERT INTO travel_rule_transport_attempts (
            id,
            tenant_id,
            disclosure_id,
            attempt_number,
            transport_kind,
            status,
            request_payload,
            response_payload,
            metadata,
            completed_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, '{}'::jsonb, '{}'::jsonb, '{}'::jsonb, NOW()
        )
        "#,
    )
    .bind(format!("trta_{}_{}", sanitize_id(disclosure_id), attempt_number))
    .bind(tenant_id)
    .bind(disclosure_id)
    .bind(i32::try_from(attempt_number).unwrap_or(i32::MAX))
    .bind(next.transport_profile.clone().unwrap_or_else(|| "manual".to_string()))
    .bind(transport_status_to_value(simulated_status))
    .execute(pool)
    .await
    .map_err(|error| db_error("insert travel rule transport attempt", error))?;

    if entered_exception_queue {
        sqlx::query(
            r#"
            INSERT INTO travel_rule_exception_queue (
                id,
                tenant_id,
                disclosure_id,
                queue_status,
                severity,
                reason_code,
                metadata
            ) VALUES (
                $1, $2, $3, 'OPEN', 'MEDIUM', $4, '{}'::jsonb
            )
            ON CONFLICT (id) DO UPDATE
            SET queue_status = EXCLUDED.queue_status,
                reason_code = EXCLUDED.reason_code,
                updated_at = NOW(),
                resolved_at = NULL,
                resolution_notes = NULL
            "#,
        )
        .bind(format!("tre_{}", disclosure_id))
        .bind(tenant_id)
        .bind(disclosure_id)
        .bind(transport_status_to_value(simulated_status))
        .execute(pool)
        .await
        .map_err(|error| db_error("upsert travel rule exception", error))?;
    }

    let metadata = disclosure_metadata_for_storage(&next);
    sqlx::query(
        r#"
        UPDATE travel_rule_disclosures
        SET lifecycle_stage = $3,
            metadata = $4,
            updated_at = NOW()
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(disclosure_id)
    .bind(disclosure_stage_to_value(next.stage))
    .bind(metadata)
    .execute(pool)
    .await
    .map_err(|error| db_error("update travel rule disclosure", error))?;

    Ok(next)
}

async fn list_exceptions_db(
    pool: &PgPool,
    tenant_id: &str,
    query: &ListExceptionsQuery,
) -> Result<Vec<TravelRuleExceptionRecord>, ApiError> {
    let rows = sqlx::query_as::<_, TravelRuleExceptionRow>(
        r#"
        SELECT id, disclosure_id, queue_status, reason_code, resolution_notes, metadata, updated_at
        FROM travel_rule_exception_queue
        WHERE tenant_id = $1
        ORDER BY updated_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await
    .map_err(|error| db_error("list travel rule exceptions", error))?;

    let mut records = rows
        .into_iter()
        .map(exception_row_to_record)
        .collect::<Result<Vec<_>, _>>()?;

    if let Some(status) = query.status.as_deref() {
        let status = status.to_ascii_uppercase();
        records.retain(|row| exception_status_to_value(row.status) == status);
    }

    records.truncate(query.limit.clamp(1, 100));
    Ok(records)
}

async fn resolve_exception_db(
    pool: &PgPool,
    tenant_id: &str,
    exception_id: &str,
    resolution_note: Option<String>,
    resolved_by: Option<String>,
) -> Result<TravelRuleExceptionRecord, ApiError> {
    let exception = sqlx::query_as::<_, TravelRuleExceptionRow>(
        r#"
        SELECT id, disclosure_id, queue_status, reason_code, resolution_notes, metadata, updated_at
        FROM travel_rule_exception_queue
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(exception_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| db_error("load travel rule exception", error))?
    .ok_or_else(|| ApiError::NotFound(format!("Travel Rule exception '{}' not found", exception_id)))?;

    if let Ok(disclosure) = fetch_disclosure_db(pool, tenant_id, &exception.disclosure_id).await {
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

        sqlx::query(
            r#"
            UPDATE travel_rule_disclosures
            SET lifecycle_stage = $3,
                metadata = $4,
                updated_at = NOW()
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(&next_disclosure.disclosure_id)
        .bind(disclosure_stage_to_value(next_disclosure.stage))
        .bind(disclosure_metadata_for_storage(&next_disclosure))
        .execute(pool)
        .await
        .map_err(|error| db_error("update travel rule disclosure during resolve", error))?;
    }

    let metadata = exception_metadata_for_storage(&exception.metadata, resolved_by.as_deref());
    sqlx::query(
        r#"
        UPDATE travel_rule_exception_queue
        SET queue_status = 'RESOLVED',
            resolution_notes = $3,
            metadata = $4,
            resolved_at = NOW(),
            updated_at = NOW()
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(exception_id)
    .bind(&resolution_note)
    .bind(metadata)
    .execute(pool)
    .await
    .map_err(|error| db_error("resolve travel rule exception", error))?;

    let row = sqlx::query_as::<_, TravelRuleExceptionRow>(
        r#"
        SELECT id, disclosure_id, queue_status, reason_code, resolution_notes, metadata, updated_at
        FROM travel_rule_exception_queue
        WHERE tenant_id = $1 AND id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(exception_id)
    .fetch_one(pool)
    .await
    .map_err(|error| db_error("reload resolved travel rule exception", error))?;

    exception_row_to_record(row)
}


pub async fn list_registry(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<ListRegistryQuery>,
) -> Result<Json<Vec<VaspRegistryRecord>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    if let Some(pool) = state.db_pool.as_ref() {
        return Ok(Json(list_registry_db(pool, &tenant_ctx.tenant_id.0, &query).await?));
    }

    let state = state_store().read().await;
    let mut rows: Vec<_> = state
        .get(&tenant_ctx.tenant_id.0)
        .map(|tenant| tenant.registry.values().cloned().collect())
        .unwrap_or_default();

    if let Some(review_status) = query.review_status.as_deref() {
        let review_status = review_status.to_ascii_uppercase();
        rows.retain(|row| review_status_to_value(row.review.status) == review_status);
    }

    rows.truncate(query.limit.clamp(1, 100));
    Ok(Json(rows))
}

pub async fn create_registry_record(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(request): Json<VaspRegistryRecordInput>,
) -> Result<Json<VaspRegistryRecord>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;

    if let Some(pool) = state.db_pool.as_ref() {
        let record = create_registry_record_db(pool, &tenant_ctx.tenant_id.0, request).await?;
        info!(tenant = %tenant_ctx.tenant_id, vasp_code = %record.vasp_code, "Admin: created persisted travel rule registry record");
        return Ok(Json(record));
    }

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
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(vasp_code): Path<String>,
    Json(request): Json<VaspReviewUpdate>,
) -> Result<Json<VaspRegistryRecord>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;

    if let Some(pool) = state.db_pool.as_ref() {
        return Ok(Json(
            update_registry_review_db(pool, &tenant_ctx.tenant_id.0, &vasp_code, &request).await?,
        ));
    }

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
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(vasp_code): Path<String>,
    Json(request): Json<VaspInteroperabilityUpdate>,
) -> Result<Json<VaspRegistryRecord>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;

    if let Some(pool) = state.db_pool.as_ref() {
        return Ok(Json(
            update_registry_interoperability_db(pool, &tenant_ctx.tenant_id.0, &vasp_code, &request).await?,
        ));
    }

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
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<ListDisclosuresQuery>,
) -> Result<Json<Vec<TravelRuleDisclosureRecord>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    if let Some(pool) = state.db_pool.as_ref() {
        return Ok(Json(list_disclosures_db(pool, &tenant_ctx.tenant_id.0, &query).await?));
    }

    let state = state_store().read().await;
    let mut rows: Vec<_> = state
        .get(&tenant_ctx.tenant_id.0)
        .map(|tenant| tenant.disclosures.values().cloned().collect())
        .unwrap_or_default();

    if let Some(stage) = query.stage.as_deref() {
        let stage = stage.to_ascii_uppercase();
        rows.retain(|row| disclosure_stage_to_value(row.stage) == stage);
    }

    rows.truncate(query.limit.clamp(1, 100));
    Ok(Json(rows))
}

pub async fn create_disclosure(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(request): Json<CreateTravelRuleDisclosureRequest>,
) -> Result<Json<TravelRuleDisclosureRecord>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;

    if let Some(pool) = state.db_pool.as_ref() {
        return Ok(Json(create_disclosure_db(pool, &tenant_ctx.tenant_id.0, request).await?));
    }

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
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(disclosure_id): Path<String>,
    Json(request): Json<RetryDisclosureRequest>,
) -> Result<Json<TravelRuleDisclosureRecord>, ApiError> {
    let _auth = super::tier::check_admin_key_operator(&headers)?;

    if let Some(pool) = state.db_pool.as_ref() {
        return Ok(Json(
            retry_disclosure_db(pool, &tenant_ctx.tenant_id.0, &disclosure_id, request).await?,
        ));
    }

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
                reason_code: transport_status_to_value(simulated_status).to_string(),
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
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<ListExceptionsQuery>,
) -> Result<Json<Vec<TravelRuleExceptionRecord>>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    if let Some(pool) = state.db_pool.as_ref() {
        return Ok(Json(list_exceptions_db(pool, &tenant_ctx.tenant_id.0, &query).await?));
    }

    let state = state_store().read().await;
    let mut rows: Vec<_> = state
        .get(&tenant_ctx.tenant_id.0)
        .map(|tenant| tenant.exceptions.values().cloned().collect())
        .unwrap_or_default();

    if let Some(status) = query.status.as_deref() {
        let status = status.to_ascii_uppercase();
        rows.retain(|row| exception_status_to_value(row.status) == status);
    }

    rows.truncate(query.limit.clamp(1, 100));
    Ok(Json(rows))
}

pub async fn resolve_exception(
    State(state): State<AppState>,
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(exception_id): Path<String>,
    Json(request): Json<ResolveExceptionRequest>,
) -> Result<Json<TravelRuleExceptionRecord>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;

    if let Some(pool) = state.db_pool.as_ref() {
        return Ok(Json(
            resolve_exception_db(
                pool,
                &tenant_ctx.tenant_id.0,
                &exception_id,
                request.resolution_note,
                auth.user_id,
            )
            .await?,
        ));
    }

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
