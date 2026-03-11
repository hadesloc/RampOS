use axum::{
    extract::{Path, Query},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};

use ramp_compliance::{KybGraphReviewItem, KybGraphService};

use crate::error::ApiError;

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

pub async fn list_kyb_reviews(
    headers: HeaderMap,
    Query(query): Query<KybGraphQuery>,
) -> Result<Json<KybReviewResponse>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    Ok(Json(KybReviewResponse {
        queue: KybGraphService::new().list_reviews(query.scenario.as_deref()),
        action_mode: "review_only".to_string(),
    }))
}

pub async fn get_kyb_graph(
    headers: HeaderMap,
    Path(entity_id): Path<String>,
    Query(query): Query<KybGraphQuery>,
) -> Result<Json<KybGraphReviewItem>, ApiError> {
    super::tier::check_admin_key(&headers)?;

    KybGraphService::new()
        .graph_for_entity(&entity_id, query.scenario.as_deref())
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("KYB graph {} not found", entity_id)))
}
