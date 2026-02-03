use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::router::AppState;
use axum::{
    extract::{Extension, Path, State},
    http::HeaderMap,
    Json,
};
use ramp_compliance::rule_parser::RuleDefinition;
use tracing::info;

/// GET /v1/admin/rules
pub async fn list_rules(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
) -> Result<Json<Vec<RuleDefinition>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, "Listing rules");

    if let Some(rule_manager) = &state.rule_manager {
        let rules = rule_manager
            .get_rule_definitions(&tenant_ctx.tenant_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;
        Ok(Json(rules))
    } else {
        Ok(Json(vec![]))
    }
}

/// POST /v1/admin/rules
pub async fn create_rule(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Json(rule): Json<RuleDefinition>,
) -> Result<Json<RuleDefinition>, ApiError> {
    // Requires Operator role
    let _auth = super::tier::check_admin_key_operator(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, rule_id = %rule.id, "Creating rule");

    if let Some(rule_manager) = &state.rule_manager {
        // Fetch existing rules
        let mut rules = rule_manager
            .get_rule_definitions(&tenant_ctx.tenant_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?; // Propagate error

        // Check if rule already exists
        if rules.iter().any(|r| r.id == rule.id) {
            return Err(ApiError::Conflict(format!(
                "Rule {} already exists",
                rule.id
            )));
        }

        // Add new rule
        rules.push(rule.clone());

        // Save back
        rule_manager
            .set_rules(&tenant_ctx.tenant_id, &rules, None)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(Json(rule))
    } else {
        Err(ApiError::Internal(
            "Rule manager not configured".to_string(),
        ))
    }
}

/// PUT /v1/admin/rules/:id
pub async fn update_rule(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(id): Path<String>,
    Json(rule): Json<RuleDefinition>,
) -> Result<Json<RuleDefinition>, ApiError> {
    // Requires Operator role
    let _auth = super::tier::check_admin_key_operator(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, rule_id = %id, "Updating rule");

    if id != rule.id {
        return Err(ApiError::BadRequest(
            "Rule ID in path mismatch with body".to_string(),
        ));
    }

    if let Some(rule_manager) = &state.rule_manager {
        let mut rules = rule_manager
            .get_rule_definitions(&tenant_ctx.tenant_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        if let Some(idx) = rules.iter().position(|r| r.id == id) {
            rules[idx] = rule.clone();

            rule_manager
                .set_rules(&tenant_ctx.tenant_id, &rules, None)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            Ok(Json(rule))
        } else {
            Err(ApiError::NotFound(format!("Rule {} not found", id)))
        }
    } else {
        Err(ApiError::Internal(
            "Rule manager not configured".to_string(),
        ))
    }
}

/// PUT /v1/admin/rules/:id/toggle
pub async fn toggle_rule(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(id): Path<String>,
) -> Result<Json<RuleDefinition>, ApiError> {
    // Requires Operator role
    let _auth = super::tier::check_admin_key_operator(&headers)?;
    info!(tenant = %tenant_ctx.tenant_id, rule_id = %id, "Toggling rule");

    if let Some(rule_manager) = &state.rule_manager {
        let mut rules = rule_manager
            .get_rule_definitions(&tenant_ctx.tenant_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        if let Some(idx) = rules.iter().position(|r| r.id == id) {
            rules[idx].enabled = !rules[idx].enabled;
            let updated_rule = rules[idx].clone();

            rule_manager
                .set_rules(&tenant_ctx.tenant_id, &rules, None)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            Ok(Json(updated_rule))
        } else {
            Err(ApiError::NotFound(format!("Rule {} not found", id)))
        }
    } else {
        Err(ApiError::Internal(
            "Rule manager not configured".to_string(),
        ))
    }
}
