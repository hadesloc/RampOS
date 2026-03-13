use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Extension, Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use ramp_compliance::{
    KybEvidencePackageQuery, KybEvidencePackageRecord, KybEvidencePackageStore, KybGraphReviewItem,
    KybGraphService,
};

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;
use crate::AppState;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KybGraphQuery {
    pub scenario: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KybReviewResponse {
    pub queue: Vec<KybGraphReviewItem>,
    pub action_mode: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KybEvidencePackageListQuery {
    pub institution_entity_id: Option<String>,
    pub corridor_code: Option<String>,
    pub review_status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KybEvidencePackageListResponse {
    pub packages: Vec<KybEvidencePackageRecord>,
    pub action_mode: String,
    pub source: String,
}

pub async fn list_kyb_reviews(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<KybGraphQuery>,
) -> Result<Json<KybReviewResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let queue = if let Some(pool) = &state.db_pool {
        KybGraphService::new()
            .list_reviews_from_pool(pool, &tenant_ctx.tenant_id.0)
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?
    } else {
        KybGraphService::new().list_reviews(query.scenario.as_deref())
    };

    Ok(Json(KybReviewResponse {
        queue,
        action_mode: "review_only".to_string(),
    }))
}

pub async fn get_kyb_graph(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(entity_id): Path<String>,
    Query(query): Query<KybGraphQuery>,
) -> Result<Json<KybGraphReviewItem>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let item = if let Some(pool) = &state.db_pool {
        KybGraphService::new()
            .graph_for_entity_from_pool(pool, &tenant_ctx.tenant_id.0, &entity_id)
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?
    } else {
        KybGraphService::new().graph_for_entity(&entity_id, query.scenario.as_deref())
    };

    item.map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("KYB graph {} not found", entity_id)))
}

pub async fn list_kyb_evidence_packages(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Query(query): Query<KybEvidencePackageListQuery>,
) -> Result<Json<KybEvidencePackageListResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let packages = if let Some(pool) = &state.db_pool {
        KybEvidencePackageStore::new(pool.clone())
            .list_packages(&KybEvidencePackageQuery {
                tenant_id: tenant_ctx.tenant_id.0.clone(),
                institution_entity_id: query.institution_entity_id,
                corridor_code: query.corridor_code,
                review_status: query.review_status,
            })
            .await
            .map_err(|error| ApiError::Internal(error.to_string()))?
    } else {
        Vec::new()
    };

    Ok(Json(KybEvidencePackageListResponse {
        packages,
        action_mode: "review_only".to_string(),
        source: if state.db_pool.is_some() {
            "registry".to_string()
        } else {
            "fallback".to_string()
        },
    }))
}

pub async fn get_kyb_evidence_package(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(package_id): Path<String>,
) -> Result<Json<KybEvidencePackageRecord>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let pool = state
        .db_pool
        .clone()
        .ok_or_else(|| ApiError::NotFound(format!("KYB evidence package {} not found", package_id)))?;

    let package = KybEvidencePackageStore::new(pool)
        .get_package(&tenant_ctx.tenant_id.0, &package_id)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("KYB evidence package {} not found", package_id)))?;

    Ok(Json(package))
}

pub async fn export_kyb_evidence_package(
    headers: HeaderMap,
    State(state): State<AppState>,
    Extension(tenant_ctx): Extension<TenantContext>,
    Path(package_id): Path<String>,
) -> Result<Response, ApiError> {
    super::tier::check_admin_key(&headers)?;

    let pool = state
        .db_pool
        .clone()
        .ok_or_else(|| ApiError::NotFound(format!("KYB evidence package {} not found", package_id)))?;

    let package = KybEvidencePackageStore::new(pool)
        .get_package(&tenant_ctx.tenant_id.0, &package_id)
        .await
        .map_err(|error| ApiError::Internal(error.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("KYB evidence package {} not found", package_id)))?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/json"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"kyb_evidence_package_{package_id}_{timestamp}.json\""),
            ),
        ],
        serde_json::to_string_pretty(&package)
            .map_err(|error| ApiError::Internal(error.to_string()))?,
    )
        .into_response())
}
