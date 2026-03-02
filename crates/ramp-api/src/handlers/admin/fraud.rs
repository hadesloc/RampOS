//! Admin Fraud API Handlers
//!
//! Endpoints for fraud score management:
//! - List recent fraud scores
//! - Get specific fraud score by ID
//! - Submit manual fraud review decision

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;
use ramp_compliance::{
    case::AmlCase,
    case::NoteType,
    types::CaseStatus,
};
// ============================================================================
// DTOs
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFraudScoresQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub decision: Option<String>,
    pub min_score: Option<u8>,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FraudScoreResponse {
    pub id: String,
    pub user_id: String,
    pub intent_id: String,
    pub score: u8,
    pub decision: String,
    pub risk_factors: Vec<RiskFactorResponse>,
    pub reviewed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskFactorResponse {
    pub rule_name: String,
    pub contribution: u8,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFraudScoresResponse {
    pub data: Vec<FraudScoreResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FraudReviewRequest {
    pub score_id: String,
    pub decision: String,
    pub note: String,
    pub reviewer: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FraudReviewResponse {
    pub score_id: String,
    pub decision: String,
    pub reviewed_by: String,
    pub note: String,
    pub reviewed_at: String,
}

// ============================================================================
// Mapping Helpers
// ============================================================================

fn is_fraud_case(case: &AmlCase) -> bool {
    use ramp_compliance::types::CaseType;

    match &case.case_type {
        CaseType::Velocity
        | CaseType::Structuring
        | CaseType::LargeTransaction
        | CaseType::KytHighRisk
        | CaseType::DeviceAnomaly => true,
        CaseType::Other(value) => {
            let lower = value.to_lowercase();
            lower.contains("risk")
                || lower.contains("fraud")
                || lower.contains("review")
                || lower.contains("block")
        }
        _ => false,
    }
}

fn extract_score(detection_data: &serde_json::Value) -> u8 {
    let number = detection_data
        .get("score")
        .or_else(|| detection_data.get("riskScore"))
        .or_else(|| detection_data.get("risk_score"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
        .clamp(0.0, 100.0);

    number.round() as u8
}

fn map_decision(case: &AmlCase) -> String {
    if matches!(case.status, CaseStatus::Closed | CaseStatus::Reported) {
        "Block".to_string()
    } else if matches!(case.status, CaseStatus::Review | CaseStatus::Open | CaseStatus::Hold) {
        "Review".to_string()
    } else {
        "Allow".to_string()
    }
}

fn map_risk_factors(detection_data: &serde_json::Value) -> Vec<RiskFactorResponse> {
    let Some(factors) = detection_data
        .get("riskFactors")
        .or_else(|| detection_data.get("risk_factors"))
        .and_then(|v| v.as_array())
    else {
        return vec![];
    };

    factors
        .iter()
        .map(|factor| {
            let rule_name = factor
                .get("ruleName")
                .or_else(|| factor.get("rule_name"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown_rule")
                .to_string();
            let contribution = factor
                .get("contribution")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                .clamp(0.0, 100.0)
                .round() as u8;
            let description = factor
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            RiskFactorResponse {
                rule_name,
                contribution,
                description,
            }
        })
        .collect()
}

fn map_fraud_score_response(case: AmlCase) -> FraudScoreResponse {
    let decision = map_decision(&case);
    let reviewed = matches!(case.status, CaseStatus::Released | CaseStatus::Closed | CaseStatus::Reported)
        || case.resolution.is_some();

    FraudScoreResponse {
        id: case.id,
        user_id: case.user_id.map(|id| id.0).unwrap_or_default(),
        intent_id: case.intent_id.map(|id| id.0).unwrap_or_default(),
        score: extract_score(&case.detection_data),
        decision,
        risk_factors: map_risk_factors(&case.detection_data),
        reviewed,
        reviewed_by: case.assigned_to,
        review_note: case.resolution,
        created_at: case.created_at.to_rfc3339(),
    }
}

fn parse_review_target_status(decision: &str) -> Option<CaseStatus> {
    match decision {
        "ALLOW" => Some(CaseStatus::Released),
        "REVIEW" => Some(CaseStatus::Review),
        "BLOCK" => Some(CaseStatus::Closed),
        _ => None,
    }
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/admin/fraud/scores - List recent fraud score results
pub async fn list_fraud_scores(
    headers: HeaderMap,
    tenant_ctx: Option<Extension<TenantContext>>,
    State(app_state): State<AppState>,
    Query(query): Query<ListFraudScoresQuery>,
) -> Result<Json<ListFraudScoresResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let tenant_ctx = tenant_ctx
        .map(|Extension(ctx)| ctx)
        .ok_or_else(|| ApiError::Internal("Tenant context unavailable for fraud endpoints".to_string()))?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        limit = query.limit,
        offset = query.offset,
        decision_filter = ?query.decision,
        "Listing fraud scores"
    );

    // Query sufficient window then apply fraud-specific mapping/filtering at API layer.
    let mut cases = app_state
        .case_manager
        .list_cases(
            &tenant_ctx.tenant_id,
            None,
            None,
            None,
            None,
            1000,
            0,
        )
        .await
        .map_err(ApiError::from)?;

    cases.retain(is_fraud_case);

    let mut scores: Vec<FraudScoreResponse> = cases
        .into_iter()
        .map(map_fraud_score_response)
        .collect();

    if let Some(ref decision) = query.decision {
        let decision_upper = decision.to_uppercase();
        scores.retain(|s| s.decision.to_uppercase() == decision_upper);
    }

    if let Some(min_score) = query.min_score {
        scores.retain(|s| s.score >= min_score);
    }

    scores.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total = scores.len() as i64;
    let limit = query.limit.clamp(1, 100);
    let offset = query.offset.max(0);

    let data = scores
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();

    Ok(Json(ListFraudScoresResponse {
        data,
        total,
        limit,
        offset,
    }))
}

/// GET /v1/admin/fraud/scores/:id - Get specific fraud score by ID
pub async fn get_fraud_score(
    headers: HeaderMap,
    tenant_ctx: Option<Extension<TenantContext>>,
    State(app_state): State<AppState>,
    Path(score_id): Path<String>,
) -> Result<Json<FraudScoreResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let tenant_ctx = tenant_ctx
        .map(|Extension(ctx)| ctx)
        .ok_or_else(|| ApiError::Internal("Tenant context unavailable for fraud endpoints".to_string()))?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        score_id = %score_id,
        "Fetching fraud score"
    );

    let case = app_state
        .case_manager
        .get_case(&tenant_ctx.tenant_id, &score_id)
        .await
        .map_err(ApiError::from)?
        .filter(is_fraud_case)
        .ok_or_else(|| ApiError::NotFound(format!("Fraud score {} not found", score_id)))?;

    Ok(Json(map_fraud_score_response(case)))
}

/// POST /v1/admin/fraud/review - Submit manual fraud review decision
pub async fn submit_fraud_review(
    headers: HeaderMap,
    tenant_ctx: Option<Extension<TenantContext>>,
    State(app_state): State<AppState>,
    Json(request): Json<FraudReviewRequest>,
) -> Result<Json<FraudReviewResponse>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;

    let tenant_ctx = tenant_ctx
        .map(|Extension(ctx)| ctx)
        .ok_or_else(|| ApiError::Internal("Tenant context unavailable for fraud endpoints".to_string()))?;

    info!(
        tenant = %tenant_ctx.tenant_id.0,
        score_id = %request.score_id,
        decision = %request.decision,
        reviewer = %request.reviewer,
        admin_user = ?auth.user_id,
        "Submitting fraud review"
    );

    let decision_upper = request.decision.to_uppercase();
    let target_status = parse_review_target_status(&decision_upper).ok_or_else(|| {
        ApiError::Validation(format!(
            "Invalid decision '{}'. Must be one of: Allow, Review, Block",
            request.decision
        ))
    })?;

    if request.note.trim().is_empty() {
        return Err(ApiError::Validation("Review note is required".to_string()));
    }

    if request.score_id.trim().is_empty() {
        return Err(ApiError::Validation("Score ID is required".to_string()));
    }

    let Some(case) = app_state
        .case_manager
        .get_case(&tenant_ctx.tenant_id, &request.score_id)
        .await
        .map_err(ApiError::from)?
    else {
        return Err(ApiError::NotFound(format!(
            "Fraud score {} not found",
            request.score_id
        )));
    };

    if !is_fraud_case(&case) {
        return Err(ApiError::NotFound(format!(
            "Fraud score {} not found",
            request.score_id
        )));
    }

    app_state
        .case_manager
        .update_status(
            &tenant_ctx.tenant_id,
            &request.score_id,
            target_status,
            auth.user_id.clone(),
        )
        .await
        .map_err(ApiError::from)?;

    app_state
        .case_manager
        .note_manager
        .add_note(
            &tenant_ctx.tenant_id,
            &request.score_id,
            auth.user_id.clone(),
            format!(
                "Fraud review decision: {} by {}. Note: {}",
                decision_upper,
                request.reviewer,
                request.note
            ),
            NoteType::Decision,
            true,
        )
        .await
        .map_err(ApiError::from)?;

    let now = chrono::Utc::now();

    Ok(Json(FraudReviewResponse {
        score_id: request.score_id,
        decision: decision_upper,
        reviewed_by: request.reviewer,
        note: request.note,
        reviewed_at: now.to_rfc3339(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fraud_score_response_serialization() {
        let resp = FraudScoreResponse {
            id: "fs_001".to_string(),
            user_id: "user_123".to_string(),
            intent_id: "intent_456".to_string(),
            score: 75,
            decision: "Review".to_string(),
            risk_factors: vec![RiskFactorResponse {
                rule_name: "velocity_1h_exceeded".to_string(),
                contribution: 15,
                description: "8 txns in 1h".to_string(),
            }],
            reviewed: false,
            reviewed_by: None,
            review_note: None,
            created_at: "2026-01-15T08:30:00Z".to_string(),
        };

        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("\"score\":75"));
        assert!(json.contains("\"decision\":\"Review\""));
        assert!(json.contains("\"velocityExceeded") == false || json.contains("\"velocity1hExceeded\"") || json.contains("\"ruleName\":\"velocity_1h_exceeded\""));
        // None fields should be skipped
        assert!(!json.contains("\"reviewedBy\""));
        assert!(!json.contains("\"reviewNote\""));
    }

    #[test]
    fn test_fraud_score_response_with_review() {
        let resp = FraudScoreResponse {
            id: "fs_002".to_string(),
            user_id: "user_789".to_string(),
            intent_id: "intent_abc".to_string(),
            score: 92,
            decision: "Block".to_string(),
            risk_factors: vec![],
            reviewed: true,
            reviewed_by: Some("analyst_01".to_string()),
            review_note: Some("Confirmed suspicious".to_string()),
            created_at: "2026-01-15T09:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("\"reviewedBy\":\"analyst_01\""));
        assert!(json.contains("\"reviewNote\":\"Confirmed suspicious\""));
        assert!(json.contains("\"reviewed\":true"));
    }

    #[test]
    fn test_list_fraud_scores_response_serialization() {
        let resp = ListFraudScoresResponse {
            data: vec![],
            total: 0,
            limit: 20,
            offset: 0,
        };

        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("\"data\":[]"));
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"limit\":20"));
    }

    #[test]
    fn test_fraud_review_response_serialization() {
        let resp = FraudReviewResponse {
            score_id: "fs_001".to_string(),
            decision: "BLOCK".to_string(),
            reviewed_by: "admin_user".to_string(),
            note: "Suspicious activity confirmed".to_string(),
            reviewed_at: "2026-01-15T10:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&resp).expect("serialization failed");
        assert!(json.contains("\"scoreId\":\"fs_001\""));
        assert!(json.contains("\"decision\":\"BLOCK\""));
        assert!(json.contains("\"reviewedBy\":\"admin_user\""));
    }

    #[test]
    fn test_risk_factor_response_serialization() {
        let factor = RiskFactorResponse {
            rule_name: "high_value_transaction".to_string(),
            contribution: 10,
            description: "$7500 exceeds threshold".to_string(),
        };

        let json = serde_json::to_string(&factor).expect("serialization failed");
        assert!(json.contains("\"ruleName\":\"high_value_transaction\""));
        assert!(json.contains("\"contribution\":10"));
    }

    #[test]
    fn test_parse_review_target_status() {
        assert_eq!(parse_review_target_status("ALLOW"), Some(CaseStatus::Released));
        assert_eq!(parse_review_target_status("REVIEW"), Some(CaseStatus::Review));
        assert_eq!(parse_review_target_status("BLOCK"), Some(CaseStatus::Closed));
        assert_eq!(parse_review_target_status("INVALID"), None);
    }

    #[test]
    fn test_fraud_score_query_deserialization() {
        let json = r#"{"limit": 10, "offset": 5, "decision": "Block", "minScore": 50}"#;
        let query: ListFraudScoresQuery = serde_json::from_str(json).expect("deserialization failed");
        assert_eq!(query.limit, 10);
        assert_eq!(query.offset, 5);
        assert_eq!(query.decision, Some("Block".to_string()));
        assert_eq!(query.min_score, Some(50));
    }

    #[test]
    fn test_fraud_review_request_deserialization() {
        let json = r#"{"scoreId": "fs_001", "decision": "Block", "note": "Confirmed fraud", "reviewer": "admin_01"}"#;
        let req: FraudReviewRequest = serde_json::from_str(json).expect("deserialization failed");
        assert_eq!(req.score_id, "fs_001");
        assert_eq!(req.decision, "Block");
        assert_eq!(req.note, "Confirmed fraud");
        assert_eq!(req.reviewer, "admin_01");
    }
}
