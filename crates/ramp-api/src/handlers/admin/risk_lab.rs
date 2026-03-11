use axum::{
    extract::{Extension, Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;
use ramp_compliance::fraud::{
    DecisionThresholds, FraudDecisionEngine, FraudDecisionExplanation, FraudFeatureVector,
    OnnxModelScorer, RuleBasedScorer, ScorerConfig,
};
use ramp_compliance::risk_graph::{RiskGraphAssembler, RiskGraphView};
use ramp_compliance::risk_lab::{
    RiskLabComparisonQuery as DomainComparisonQuery, RiskLabComparisonRecord,
    RiskLabComparisonStats, RiskLabManager, RiskLabReplayRecord, RiskLabRuleVersion,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabCompareQuery {
    pub user_id: Option<String>,
    pub intent_id: Option<String>,
    #[serde(default)]
    pub changed_only: bool,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabRuleVersionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<String>,
    pub version_metadata: serde_json::Value,
    pub scorer_config: serde_json::Value,
    pub decision_thresholds: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabReplayResponse {
    pub score_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_id: Option<String>,
    pub score: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow_score: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow_decision: Option<String>,
    pub decision_changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_taken: Option<String>,
    pub triggered_rules: serde_json::Value,
    pub feature_vector: serde_json::Value,
    pub score_explanation: serde_json::Value,
    pub decision_snapshot: serde_json::Value,
    pub replay_metadata: serde_json::Value,
    pub created_at: String,
    pub rule_version: RiskLabRuleVersionResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabComparisonItemResponse {
    pub score_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_id: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_version_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_mode: Option<String>,
    pub score: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow_score: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow_decision: Option<String>,
    pub decision_changed: bool,
    pub triggered_rule_count: usize,
    pub factor_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabComparisonStatsResponse {
    pub compared_scores: usize,
    pub changed_decisions: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_abs_score_delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_abs_score_delta: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabCompareResponse {
    pub data: Vec<RiskLabComparisonItemResponse>,
    pub total: usize,
    pub stats: RiskLabComparisonStatsResponse,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabExplainResponse {
    pub score_id: String,
    pub graph: RiskGraphView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabCatalogEntryResponse {
    pub scorer_kind: String,
    pub label: String,
    pub supports_shadow_compare: bool,
    pub safe_fallback: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabCatalogResponse {
    pub entries: Vec<RiskLabCatalogEntryResponse>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabReplayApiRequest {
    pub replay_id: String,
    pub feature_vector: FraudFeatureVector,
    pub rule_version_id: Option<String>,
    pub scorer_config: Option<ScorerConfig>,
    pub decision_thresholds: Option<DecisionThresholds>,
    pub challenger: Option<RiskLabReplayChallenger>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabReplayChallenger {
    pub scorer_kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabReplayGraphNodeResponse {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub weight: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabReplayGraphEdgeResponse {
    pub source_id: String,
    pub target_id: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabReplayGraphResponse {
    pub nodes: Vec<RiskLabReplayGraphNodeResponse>,
    pub edges: Vec<RiskLabReplayGraphEdgeResponse>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLabReplayApiResponse {
    pub replay_id: String,
    pub primary_score: serde_json::Value,
    pub primary_decision: serde_json::Value,
    pub challenger_score: Option<serde_json::Value>,
    pub challenger_decision: Option<serde_json::Value>,
    pub score_delta: Option<i16>,
    pub graph: RiskLabReplayGraphResponse,
}

fn default_limit() -> i64 {
    20
}

fn require_pool(state: &AppState) -> Result<sqlx::PgPool, ApiError> {
    state
        .db_pool
        .clone()
        .ok_or_else(|| ApiError::Internal("Risk lab requires database-backed services".to_string()))
}

fn parse_score_id(score_id: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(score_id)
        .map_err(|_| ApiError::Validation(format!("Invalid score ID '{}'", score_id)))
}

fn map_rule_version_response(version: RiskLabRuleVersion) -> RiskLabRuleVersionResponse {
    RiskLabRuleVersionResponse {
        id: version.id.map(|value| value.to_string()),
        version_number: version.version_number,
        version_state: version.version_state,
        version_label: version.version_label,
        parent_version_id: version.parent_version_id.map(|value| value.to_string()),
        is_active: version.is_active,
        created_at: version.created_at.map(|value| value.to_rfc3339()),
        activated_at: version.activated_at.map(|value| value.to_rfc3339()),
        version_metadata: version.version_metadata,
        scorer_config: version.scorer_config,
        decision_thresholds: version.decision_thresholds,
    }
}

fn map_replay_response(replay: RiskLabReplayRecord) -> RiskLabReplayResponse {
    RiskLabReplayResponse {
        score_id: replay.score_id.to_string(),
        user_id: replay.user_id.0,
        intent_id: replay.intent_id.map(|value| value.0),
        score: replay.score.to_string(),
        shadow_score: replay.shadow_score.map(|value| value.to_string()),
        score_delta: replay.score_delta.map(|value| value.to_string()),
        decision: replay.decision,
        shadow_decision: replay.shadow_decision,
        decision_changed: replay.decision_changed,
        action_taken: replay.action_taken,
        triggered_rules: replay.triggered_rules,
        feature_vector: replay.feature_vector,
        score_explanation: replay.score_explanation,
        decision_snapshot: replay.decision_snapshot,
        replay_metadata: replay.replay_metadata,
        created_at: replay.created_at.to_rfc3339(),
        rule_version: map_rule_version_response(replay.rule_version),
    }
}

fn map_comparison_item(record: RiskLabComparisonRecord) -> RiskLabComparisonItemResponse {
    RiskLabComparisonItemResponse {
        score_id: record.score_id.to_string(),
        user_id: record.user_id.0,
        intent_id: record.intent_id.map(|value| value.0),
        created_at: record.created_at.to_rfc3339(),
        current_version_id: record.current_version_id.map(|value| value.to_string()),
        current_version_label: record.current_version_label,
        current_version_state: record.current_version_state,
        candidate_version_id: record.candidate_version_id,
        candidate_version_label: record.candidate_version_label,
        replay_mode: record.replay_mode,
        score: record.score.to_string(),
        shadow_score: record.shadow_score.map(|value| value.to_string()),
        score_delta: record.score_delta.map(|value| value.to_string()),
        decision: record.decision,
        shadow_decision: record.shadow_decision,
        decision_changed: record.decision_changed,
        triggered_rule_count: record.triggered_rule_count,
        factor_count: record.factor_count,
    }
}

fn map_comparison_stats(stats: RiskLabComparisonStats) -> RiskLabComparisonStatsResponse {
    RiskLabComparisonStatsResponse {
        compared_scores: stats.compared_scores,
        changed_decisions: stats.changed_decisions,
        avg_abs_score_delta: stats.avg_abs_score_delta.map(|value| value.to_string()),
        max_abs_score_delta: stats.max_abs_score_delta.map(|value| value.to_string()),
    }
}

pub async fn get_risk_lab_catalog(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
) -> Result<Json<RiskLabCatalogResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    info!(tenant = %tenant_ctx.tenant_id, "Loading risk lab catalog");

    Ok(Json(RiskLabCatalogResponse {
        entries: vec![
            RiskLabCatalogEntryResponse {
                scorer_kind: "RULE_BASED".to_string(),
                label: "Rule-based scorer".to_string(),
                supports_shadow_compare: true,
                safe_fallback: "primary_default".to_string(),
            },
            RiskLabCatalogEntryResponse {
                scorer_kind: "ONNX_HEURISTIC".to_string(),
                label: "ONNX heuristic scorer".to_string(),
                supports_shadow_compare: true,
                safe_fallback: "heuristic_when_model_unloaded".to_string(),
            },
        ],
    }))
}

pub async fn replay_risk_lab(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(_app_state): State<AppState>,
    Json(request): Json<RiskLabReplayApiRequest>,
) -> Result<Json<RiskLabReplayApiResponse>, ApiError> {
    super::tier::check_admin_key_operator(&headers)?;

    if request.replay_id.trim().is_empty() {
        return Err(ApiError::Validation("replayId is required".to_string()));
    }

    let primary_scorer = RuleBasedScorer::with_config(request.scorer_config.unwrap_or_default());
    let primary_score = primary_scorer
        .score_with_metadata(&request.feature_vector, request.rule_version_id.as_deref());
    let decision_engine =
        FraudDecisionEngine::with_thresholds(request.decision_thresholds.unwrap_or_default());
    let primary_decision = decision_engine.decide_with_explanation(&primary_score.risk_score);

    let (challenger_score, challenger_decision, score_delta) = if let Some(challenger) =
        request.challenger
    {
        let challenger_score = match challenger.scorer_kind.to_ascii_uppercase().as_str() {
            "ONNX_HEURISTIC" => {
                let scorer = OnnxModelScorer::new("/models/fraud_shadow.onnx");
                scorer.score_with_metadata(
                    &request.feature_vector,
                    request.rule_version_id.as_deref(),
                )
            }
            _ => primary_scorer
                .score_with_metadata(&request.feature_vector, request.rule_version_id.as_deref()),
        };
        let challenger_decision =
            decision_engine.decide_with_explanation(&challenger_score.risk_score);
        let score_delta = i16::from(challenger_score.risk_score.score)
            - i16::from(primary_score.risk_score.score);
        (
            Some(challenger_score),
            Some(challenger_decision),
            Some(score_delta),
        )
    } else {
        (None, None, None)
    };

    info!(
        tenant = %tenant_ctx.tenant_id,
        replay_id = %request.replay_id,
        challenger = challenger_score.is_some(),
        "Running risk lab replay"
    );

    Ok(Json(RiskLabReplayApiResponse {
        replay_id: request.replay_id.clone(),
        primary_score: serde_json::to_value(&primary_score).map_err(|error| {
            ApiError::Internal(format!("Failed to encode primary score: {}", error))
        })?,
        primary_decision: serde_json::to_value(&primary_decision).map_err(|error| {
            ApiError::Internal(format!("Failed to encode primary decision: {}", error))
        })?,
        challenger_score: challenger_score
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| {
                ApiError::Internal(format!("Failed to encode challenger score: {}", error))
            })?,
        challenger_decision: challenger_decision
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| {
                ApiError::Internal(format!("Failed to encode challenger decision: {}", error))
            })?,
        score_delta,
        graph: build_replay_graph(&request.replay_id, &primary_score, &primary_decision),
    }))
}

pub async fn get_risk_lab_replay(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Path(score_id): Path<String>,
) -> Result<Json<RiskLabReplayResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let pool = require_pool(&app_state)?;
    let manager = RiskLabManager::new(pool);
    let replay = manager
        .get_replay(&tenant_ctx.tenant_id, parse_score_id(&score_id)?)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(map_replay_response(replay)))
}

fn build_replay_graph(
    replay_id: &str,
    primary_score: &ramp_compliance::fraud::ExplainedRiskScore,
    primary_decision: &FraudDecisionExplanation,
) -> RiskLabReplayGraphResponse {
    let replay_node_id = format!("replay:{replay_id}");
    let decision_node_id = format!("decision:{replay_id}");
    let feature_node_id = format!("features:{replay_id}");

    let mut nodes = vec![
        RiskLabReplayGraphNodeResponse {
            id: replay_node_id.clone(),
            kind: "REPLAY".to_string(),
            label: format!("Replay {}", replay_id),
            weight: Some(primary_score.risk_score.score),
        },
        RiskLabReplayGraphNodeResponse {
            id: decision_node_id.clone(),
            kind: "DECISION".to_string(),
            label: format!("{:?}", primary_decision.decision),
            weight: Some(primary_decision.boundary_distance),
        },
        RiskLabReplayGraphNodeResponse {
            id: feature_node_id.clone(),
            kind: "FEATURE_SNAPSHOT".to_string(),
            label: "Feature snapshot".to_string(),
            weight: None,
        },
    ];

    let mut edges = vec![
        RiskLabReplayGraphEdgeResponse {
            source_id: replay_node_id,
            target_id: decision_node_id.clone(),
            kind: "EXPLAINS".to_string(),
        },
        RiskLabReplayGraphEdgeResponse {
            source_id: feature_node_id,
            target_id: decision_node_id.clone(),
            kind: "EVALUATED_FROM".to_string(),
        },
    ];

    for factor in &primary_score.metadata.top_risk_factors {
        let factor_id = format!("factor:{}:{}", replay_id, factor.rule_name);
        nodes.push(RiskLabReplayGraphNodeResponse {
            id: factor_id.clone(),
            kind: "RULE_FACTOR".to_string(),
            label: factor.rule_name.clone(),
            weight: Some(factor.contribution),
        });
        edges.push(RiskLabReplayGraphEdgeResponse {
            source_id: factor_id,
            target_id: decision_node_id.clone(),
            kind: "TRIGGERED".to_string(),
        });
    }

    RiskLabReplayGraphResponse { nodes, edges }
}

pub async fn compare_risk_lab_replays(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Query(query): Query<RiskLabCompareQuery>,
) -> Result<Json<RiskLabCompareResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let pool = require_pool(&app_state)?;
    let manager = RiskLabManager::new(pool);
    let comparison = manager
        .list_comparisons(
            &tenant_ctx.tenant_id,
            &DomainComparisonQuery {
                user_id: query.user_id,
                intent_id: query.intent_id,
                changed_only: query.changed_only,
                limit: query.limit,
            },
        )
        .await
        .map_err(ApiError::from)?;

    Ok(Json(RiskLabCompareResponse {
        data: comparison
            .data
            .into_iter()
            .map(map_comparison_item)
            .collect(),
        total: comparison.total,
        stats: map_comparison_stats(comparison.stats),
    }))
}

pub async fn explain_risk_lab_replay(
    headers: HeaderMap,
    Extension(tenant_ctx): Extension<TenantContext>,
    State(app_state): State<AppState>,
    Path(score_id): Path<String>,
) -> Result<Json<RiskLabExplainResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let pool = require_pool(&app_state)?;
    let manager = RiskLabManager::new(pool);
    let replay = manager
        .get_replay(&tenant_ctx.tenant_id, parse_score_id(&score_id)?)
        .await
        .map_err(ApiError::from)?;
    let graph = RiskGraphAssembler::assemble(&replay);

    Ok(Json(RiskLabExplainResponse {
        score_id: replay.score_id.to_string(),
        graph,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::tenant::{TenantContext, TenantTier};
    use crate::router::AppState;
    use chrono::Utc;
    use ramp_common::types::TenantId;
    use ramp_compliance::{
        case::CaseManager, reports::ReportGenerator, storage::MockDocumentStorage, InMemoryCaseStore,
    };
    use ramp_core::{
        billing::{mock::MockBillingDataProvider, BillingConfig, BillingService},
        event::InMemoryEventPublisher,
        service::{
            ledger::LedgerService, onboarding::OnboardingService, payin::PayinService,
            payout::PayoutService, trade::TradeService, user::UserService, webhook::WebhookService,
            MetricsRegistry,
        },
        stablecoin::{MockVnstProtocolDataProvider, VnstProtocolConfig, VnstProtocolService},
        test_utils::{MockIntentRepository, MockLedgerRepository, MockTenantRepository, MockUserRepository, MockWebhookRepository},
        sso::SsoService,
    };
    use sqlx::PgPool;
    use std::sync::Arc;
    use axum::{http::HeaderMap, Extension, Json};

    #[test]
    fn compare_query_deserializes_camel_case_filters() {
        let query: RiskLabCompareQuery = serde_json::from_str(
            r#"{"userId":"user_01","intentId":"intent_01","changedOnly":true,"limit":50}"#,
        )
        .expect("query should deserialize");

        assert_eq!(query.user_id.as_deref(), Some("user_01"));
        assert_eq!(query.intent_id.as_deref(), Some("intent_01"));
        assert!(query.changed_only);
        assert_eq!(query.limit, 50);
    }

    #[test]
    fn replay_response_omits_optional_fields_when_absent() {
        let response = RiskLabReplayResponse {
            score_id: "risk-score-01".to_string(),
            user_id: "user_01".to_string(),
            intent_id: None,
            score: "42".to_string(),
            shadow_score: None,
            score_delta: None,
            decision: Some("Review".to_string()),
            shadow_decision: None,
            decision_changed: false,
            action_taken: None,
            triggered_rules: serde_json::json!(["velocity_1h"]),
            feature_vector: serde_json::json!({}),
            score_explanation: serde_json::json!({}),
            decision_snapshot: serde_json::json!({}),
            replay_metadata: serde_json::json!({}),
            created_at: "2026-03-08T10:00:00Z".to_string(),
            rule_version: RiskLabRuleVersionResponse {
                id: None,
                version_number: Some(4),
                version_state: Some("ACTIVE".to_string()),
                version_label: Some("v4".to_string()),
                parent_version_id: None,
                is_active: Some(true),
                created_at: None,
                activated_at: None,
                version_metadata: serde_json::json!({}),
                scorer_config: serde_json::json!({}),
                decision_thresholds: serde_json::json!({}),
            },
        };

        let json = serde_json::to_string(&response).expect("response should serialize");

        assert!(json.contains("\"scoreId\":\"risk-score-01\""));
        assert!(json.contains("\"decision\":\"Review\""));
        assert!(!json.contains("\"shadowScore\""));
        assert!(!json.contains("\"intentId\""));
    }

    #[tokio::test]
    async fn replay_risk_lab_rejects_viewer_role() {
        std::env::set_var("RAMPOS_ADMIN_KEY", "admin-secret-key");
        std::env::set_var("RAMPOS_ADMIN_ROLE", "viewer");

        let intent_repo = Arc::new(MockIntentRepository::new());
        let ledger_repo = Arc::new(MockLedgerRepository::new());
        let user_repo = Arc::new(MockUserRepository::new());
        let tenant_repo = Arc::new(MockTenantRepository::new());
        let webhook_repo = Arc::new(MockWebhookRepository::new());
        let event_publisher = Arc::new(InMemoryEventPublisher::new());
        let payin_service = Arc::new(PayinService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        ));
        let payout_service = Arc::new(PayoutService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            user_repo.clone(),
            event_publisher.clone(),
        ));
        let trade_service = Arc::new(TradeService::new(
            intent_repo.clone(),
            ledger_repo.clone(),
            event_publisher.clone(),
        ));
        let ledger_service = Arc::new(LedgerService::new(ledger_repo));
        let onboarding_service = Arc::new(OnboardingService::new(
            tenant_repo.clone(),
            ledger_service.clone(),
        ));
        let user_service = Arc::new(UserService::new(user_repo, event_publisher.clone()));
        let report_generator = Arc::new(ReportGenerator::new(
            PgPool::connect_lazy("postgres://postgres:postgres@localhost/postgres")
                .expect("lazy pool"),
            Arc::new(MockDocumentStorage::new()),
        ));
        let app_state = AppState {
            payin_service,
            payout_service,
            trade_service,
            ledger_service,
            onboarding_service,
            user_service,
            webhook_service: Arc::new(
                WebhookService::new(webhook_repo, tenant_repo.clone()).expect("webhook service"),
            ),
            tenant_repo,
            intent_repo,
            report_generator,
            case_manager: Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new()))),
            rule_manager: None,
            rate_limiter: None,
            idempotency_handler: None,
            aa_service: None,
            portal_auth_config: Arc::new(crate::middleware::PortalAuthConfig {
                jwt_secret: "risk-lab-test-secret".to_string(),
                issuer: None,
                audience: None,
                allow_missing_tenant: false,
            }),
            bank_confirmation_repo: None,
            licensing_repo: None,
            compliance_audit_service: None,
            sso_service: Arc::new(SsoService::new()),
            billing_service: Arc::new(BillingService::new(
                BillingConfig::default(),
                Arc::new(MockBillingDataProvider::new()),
            )),
            vnst_protocol: Arc::new(VnstProtocolService::new(
                VnstProtocolConfig::default(),
                Arc::new(MockVnstProtocolDataProvider::new()),
            )),
            db_pool: None,
            ctr_service: None,
            ws_state: None,
            metrics_registry: Arc::new(MetricsRegistry::new()),
            event_publisher,
        };

        let tenant_ctx = TenantContext {
            tenant_id: TenantId::new("tenant_risk_lab"),
            name: "Tenant".to_string(),
            tier: TenantTier::Standard,
        };
        let mut headers = HeaderMap::new();
        headers.insert("X-Admin-Key", "admin-secret-key".parse().unwrap());

        let request = RiskLabReplayApiRequest {
            replay_id: "replay-1".to_string(),
            feature_vector: ramp_compliance::fraud::FraudFeatureVector {
                velocity_1h: 10.0,
                amount_usd: 500000.0,
                ..Default::default()
            },
            rule_version_id: None,
            scorer_config: None,
            decision_thresholds: None,
            challenger: None,
        };

        let err = replay_risk_lab(
            headers,
            Extension(tenant_ctx),
            axum::extract::State(app_state),
            Json(request),
        )
        .await
        .unwrap_err();

        match err {
            ApiError::Forbidden(message) => {
                assert!(message.contains("Insufficient permissions"));
            }
            other => panic!("expected forbidden error, got {other:?}"),
        }

        std::env::remove_var("RAMPOS_ADMIN_ROLE");
    }
}
