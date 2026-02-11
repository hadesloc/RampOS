//! Admin Fraud API Handlers
//!
//! Endpoints for fraud score management:
//! - List recent fraud scores
//! - Get specific fraud score by ID
//! - Submit manual fraud review decision

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::ApiError;
use crate::router::AppState;

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
// Mock Data
// ============================================================================

fn mock_fraud_scores() -> Vec<FraudScoreResponse> {
    vec![
        FraudScoreResponse {
            id: "fs_001".to_string(),
            user_id: "user_vn_12345".to_string(),
            intent_id: "intent_01HX7K9M3N".to_string(),
            score: 75,
            decision: "Review".to_string(),
            risk_factors: vec![
                RiskFactorResponse {
                    rule_name: "velocity_1h_exceeded".to_string(),
                    contribution: 15,
                    description: "8 txns in 1h (limit 5)".to_string(),
                },
                RiskFactorResponse {
                    rule_name: "high_value_transaction".to_string(),
                    contribution: 10,
                    description: "$7500.00 exceeds $5000.00 threshold".to_string(),
                },
                RiskFactorResponse {
                    rule_name: "new_account".to_string(),
                    contribution: 12,
                    description: "Account is 3 days old (threshold 7 days)".to_string(),
                },
            ],
            reviewed: false,
            reviewed_by: None,
            review_note: None,
            created_at: "2026-01-15T08:30:00Z".to_string(),
        },
        FraudScoreResponse {
            id: "fs_002".to_string(),
            user_id: "user_vn_67890".to_string(),
            intent_id: "intent_02AB3CD4EF".to_string(),
            score: 15,
            decision: "Allow".to_string(),
            risk_factors: vec![],
            reviewed: false,
            reviewed_by: None,
            review_note: None,
            created_at: "2026-01-15T09:00:00Z".to_string(),
        },
        FraudScoreResponse {
            id: "fs_003".to_string(),
            user_id: "user_vn_11111".to_string(),
            intent_id: "intent_03GH5IJ6KL".to_string(),
            score: 92,
            decision: "Block".to_string(),
            risk_factors: vec![
                RiskFactorResponse {
                    rule_name: "very_high_value_transaction".to_string(),
                    contribution: 25,
                    description: "$65000.00 exceeds $50000.00 threshold".to_string(),
                },
                RiskFactorResponse {
                    rule_name: "structuring_suspected".to_string(),
                    contribution: 20,
                    description: "Multiple round-amount transactions suggest structuring".to_string(),
                },
                RiskFactorResponse {
                    rule_name: "rapid_succession".to_string(),
                    contribution: 18,
                    description: "12 txns in 1h indicates rapid-fire activity".to_string(),
                },
                RiskFactorResponse {
                    rule_name: "cross_border_high_risk".to_string(),
                    contribution: 12,
                    description: "Cross-border to high-risk country (risk=0.85)".to_string(),
                },
            ],
            reviewed: true,
            reviewed_by: Some("analyst_01".to_string()),
            review_note: Some("Confirmed suspicious activity, escalated to compliance".to_string()),
            created_at: "2026-01-15T07:15:00Z".to_string(),
        },
    ]
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /v1/admin/fraud/scores - List recent fraud score results
pub async fn list_fraud_scores(
    headers: HeaderMap,
    State(_app_state): State<AppState>,
    Query(query): Query<ListFraudScoresQuery>,
) -> Result<Json<ListFraudScoresResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        limit = query.limit,
        offset = query.offset,
        decision_filter = ?query.decision,
        "Listing fraud scores"
    );

    let mut scores = mock_fraud_scores();

    // Filter by decision if specified
    if let Some(ref decision) = query.decision {
        let decision_upper = decision.to_uppercase();
        scores.retain(|s| s.decision.to_uppercase() == decision_upper);
    }

    // Filter by minimum score if specified
    if let Some(min_score) = query.min_score {
        scores.retain(|s| s.score >= min_score);
    }

    let total = scores.len() as i64;
    let limit = query.limit.min(100);
    let offset = query.offset.max(0);

    let data: Vec<FraudScoreResponse> = scores
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
    State(_app_state): State<AppState>,
    Path(score_id): Path<String>,
) -> Result<Json<FraudScoreResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(
        score_id = %score_id,
        "Fetching fraud score"
    );

    let scores = mock_fraud_scores();
    let score = scores
        .into_iter()
        .find(|s| s.id == score_id)
        .ok_or_else(|| ApiError::NotFound(format!("Fraud score {} not found", score_id)))?;

    Ok(Json(score))
}

/// POST /v1/admin/fraud/review - Submit manual fraud review decision
pub async fn submit_fraud_review(
    headers: HeaderMap,
    State(_app_state): State<AppState>,
    Json(request): Json<FraudReviewRequest>,
) -> Result<Json<FraudReviewResponse>, ApiError> {
    let auth = super::tier::check_admin_key_operator(&headers)?;
    info!(
        score_id = %request.score_id,
        decision = %request.decision,
        reviewer = %request.reviewer,
        admin_user = ?auth.user_id,
        "Submitting fraud review"
    );

    // Validate decision
    let decision_upper = request.decision.to_uppercase();
    if !["ALLOW", "REVIEW", "BLOCK"].contains(&decision_upper.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid decision '{}'. Must be one of: Allow, Review, Block",
            request.decision
        )));
    }

    if request.note.trim().is_empty() {
        return Err(ApiError::Validation(
            "Review note is required".to_string(),
        ));
    }

    if request.score_id.trim().is_empty() {
        return Err(ApiError::Validation(
            "Score ID is required".to_string(),
        ));
    }

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
    fn test_mock_fraud_scores_data() {
        let scores = mock_fraud_scores();
        assert_eq!(scores.len(), 3);

        // Verify different decisions are represented
        let decisions: Vec<&str> = scores.iter().map(|s| s.decision.as_str()).collect();
        assert!(decisions.contains(&"Review"));
        assert!(decisions.contains(&"Allow"));
        assert!(decisions.contains(&"Block"));

        // Verify the blocked score has risk factors
        let blocked = scores.iter().find(|s| s.decision == "Block").unwrap();
        assert!(!blocked.risk_factors.is_empty());
        assert!(blocked.score > 80);
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
