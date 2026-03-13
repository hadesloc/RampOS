use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KybEntityType {
    Business,
    Director,
    Ubo,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KybGraphEdgeType {
    Shareholder,
    Director,
    Ubo,
    Controller,
    SubmittedDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KybEntityNode {
    pub id: String,
    pub entity_type: String,
    pub display_name: String,
    pub jurisdiction: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KybGraphEdge {
    pub source_id: String,
    pub target_id: String,
    pub edge_type: String,
    pub ownership_pct: Option<f64>,
    pub effective_from: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KybGraphSummary {
    pub entity_id: String,
    pub owner_count: usize,
    pub ubo_count: usize,
    pub director_count: usize,
    pub missing_requirements: Vec<String>,
    pub review_flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KybGraphReviewItem {
    pub entity_id: String,
    pub legal_name: String,
    pub review_status: String,
    pub summary: KybGraphSummary,
    pub nodes: Vec<KybEntityNode>,
    pub edges: Vec<KybGraphEdge>,
}

pub struct KybGraphService;

impl KybGraphService {
    pub fn new() -> Self {
        Self
    }

    pub fn list_reviews(&self, scenario: Option<&str>) -> Vec<KybGraphReviewItem> {
        sample_reviews(scenario)
    }

    pub fn graph_for_entity(
        &self,
        entity_id: &str,
        scenario: Option<&str>,
    ) -> Option<KybGraphReviewItem> {
        sample_reviews(scenario)
            .into_iter()
            .find(|item| item.entity_id == entity_id)
    }

    pub async fn list_reviews_from_pool(
        &self,
        pool: &PgPool,
        tenant_id: &str,
    ) -> Result<Vec<KybGraphReviewItem>, sqlx::Error> {
        let entities = sqlx::query_as::<_, KybEntityRow>(
            r#"
            SELECT id, tenant_id, entity_type, display_name, jurisdiction, status, metadata
            FROM kyb_entities
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(pool)
        .await?;

        let businesses: Vec<_> = entities
            .into_iter()
            .filter(|row| row.entity_type.eq_ignore_ascii_case("business"))
            .collect();

        let mut reviews = Vec::with_capacity(businesses.len());
        for business in businesses {
            if let Some(review) = self
                .graph_for_entity_from_pool(pool, tenant_id, &business.id)
                .await?
            {
                reviews.push(review);
            }
        }

        Ok(reviews)
    }

    pub async fn graph_for_entity_from_pool(
        &self,
        pool: &PgPool,
        tenant_id: &str,
        entity_id: &str,
    ) -> Result<Option<KybGraphReviewItem>, sqlx::Error> {
        let root = sqlx::query_as::<_, KybEntityRow>(
            r#"
            SELECT id, tenant_id, entity_type, display_name, jurisdiction, status, metadata
            FROM kyb_entities
            WHERE tenant_id = $1 AND id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_optional(pool)
        .await?;

        let Some(root) = root else {
            return Ok(None);
        };

        let edges = sqlx::query_as::<_, KybEdgeRow>(
            r#"
            SELECT id, tenant_id, source_id, target_id, edge_type, ownership_pct, effective_from, metadata
            FROM kyb_ownership_edges
            WHERE tenant_id = $1
              AND (target_id = $2 OR source_id = $2)
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(entity_id)
        .fetch_all(pool)
        .await?;

        let mut entity_ids = BTreeSet::new();
        entity_ids.insert(root.id.clone());
        for edge in &edges {
            entity_ids.insert(edge.source_id.clone());
            entity_ids.insert(edge.target_id.clone());
        }

        let related_entities = sqlx::query_as::<_, KybEntityRow>(
            r#"
            SELECT id, tenant_id, entity_type, display_name, jurisdiction, status, metadata
            FROM kyb_entities
            WHERE tenant_id = $1 AND id = ANY($2)
            ORDER BY created_at ASC
            "#,
        )
        .bind(tenant_id)
        .bind(entity_ids.into_iter().collect::<Vec<_>>())
        .fetch_all(pool)
        .await?;

        Ok(Some(build_review_item(root, related_entities, edges)))
    }
}

impl Default for KybGraphService {
    fn default() -> Self {
        Self::new()
    }
}

fn sample_reviews(scenario: Option<&str>) -> Vec<KybGraphReviewItem> {
    let now = Utc::now();
    if matches!(scenario, Some("clean")) {
        return vec![KybGraphReviewItem {
            entity_id: "biz_clean_001".to_string(),
            legal_name: "Clean Holdings JSC".to_string(),
            review_status: "ready_for_review".to_string(),
            summary: KybGraphSummary {
                entity_id: "biz_clean_001".to_string(),
                owner_count: 2,
                ubo_count: 1,
                director_count: 2,
                missing_requirements: Vec::new(),
                review_flags: vec!["local_presence_verified".to_string()],
            },
            nodes: vec![
                KybEntityNode {
                    id: "biz_clean_001".to_string(),
                    entity_type: "business".to_string(),
                    display_name: "Clean Holdings JSC".to_string(),
                    jurisdiction: Some("VN".to_string()),
                    status: "active".to_string(),
                },
                KybEntityNode {
                    id: "ubo_clean_001".to_string(),
                    entity_type: "ubo".to_string(),
                    display_name: "Tran Thi B".to_string(),
                    jurisdiction: Some("VN".to_string()),
                    status: "verified".to_string(),
                },
            ],
            edges: vec![KybGraphEdge {
                source_id: "ubo_clean_001".to_string(),
                target_id: "biz_clean_001".to_string(),
                edge_type: "ubo".to_string(),
                ownership_pct: Some(62.5),
                effective_from: Some((now - Duration::days(120)).to_rfc3339()),
            }],
        }];
    }

    vec![KybGraphReviewItem {
        entity_id: "biz_review_001".to_string(),
        legal_name: "Ramp Ops Vietnam Ltd".to_string(),
        review_status: "needs_review".to_string(),
        summary: KybGraphSummary {
            entity_id: "biz_review_001".to_string(),
            owner_count: 3,
            ubo_count: 1,
            director_count: 2,
            missing_requirements: vec![
                "shareholder_register".to_string(),
                "director_police_clearance".to_string(),
            ],
            review_flags: vec![
                "foreign_ownership_concentration".to_string(),
                "ubo_supporting_docs_missing".to_string(),
            ],
        },
        nodes: vec![
            KybEntityNode {
                id: "biz_review_001".to_string(),
                entity_type: "business".to_string(),
                display_name: "Ramp Ops Vietnam Ltd".to_string(),
                jurisdiction: Some("VN".to_string()),
                status: "active".to_string(),
            },
            KybEntityNode {
                id: "dir_001".to_string(),
                entity_type: "director".to_string(),
                display_name: "Nguyen Van A".to_string(),
                jurisdiction: Some("VN".to_string()),
                status: "pending_document_review".to_string(),
            },
            KybEntityNode {
                id: "ubo_001".to_string(),
                entity_type: "ubo".to_string(),
                display_name: "Global HoldCo Pte".to_string(),
                jurisdiction: Some("SG".to_string()),
                status: "ownership_verified".to_string(),
            },
        ],
        edges: vec![
            KybGraphEdge {
                source_id: "dir_001".to_string(),
                target_id: "biz_review_001".to_string(),
                edge_type: "director".to_string(),
                ownership_pct: None,
                effective_from: Some((now - Duration::days(200)).to_rfc3339()),
            },
            KybGraphEdge {
                source_id: "ubo_001".to_string(),
                target_id: "biz_review_001".to_string(),
                edge_type: "ubo".to_string(),
                ownership_pct: Some(78.0),
                effective_from: Some((now - Duration::days(300)).to_rfc3339()),
            },
        ],
    }]
}

#[derive(Debug, Clone, FromRow)]
struct KybEntityRow {
    id: String,
    tenant_id: String,
    entity_type: String,
    display_name: String,
    jurisdiction: Option<String>,
    status: String,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
struct KybEdgeRow {
    id: String,
    tenant_id: String,
    source_id: String,
    target_id: String,
    edge_type: String,
    ownership_pct: Option<f64>,
    effective_from: Option<DateTime<Utc>>,
    metadata: serde_json::Value,
}

fn build_review_item(
    root: KybEntityRow,
    entities: Vec<KybEntityRow>,
    edges: Vec<KybEdgeRow>,
) -> KybGraphReviewItem {
    let entity_kind_map: HashMap<_, _> = entities
        .iter()
        .map(|entity| (entity.id.clone(), entity.entity_type.clone()))
        .collect();

    let nodes = entities
        .into_iter()
        .map(|entity| KybEntityNode {
            id: entity.id,
            entity_type: entity.entity_type,
            display_name: entity.display_name,
            jurisdiction: entity.jurisdiction,
            status: entity.status,
        })
        .collect::<Vec<_>>();

    let review_flags = json_string_array(root.metadata.get("reviewFlags"));
    let missing_requirements = json_string_array(root.metadata.get("missingRequirements"));

    let mut owner_ids = BTreeSet::new();
    let mut ubo_ids = BTreeSet::new();
    let mut director_ids = BTreeSet::new();

    let edge_models = edges
        .into_iter()
        .map(|edge| {
            if edge.target_id == root.id {
                owner_ids.insert(edge.source_id.clone());
                let source_kind = entity_kind_map
                    .get(&edge.source_id)
                    .map(|entity_type| entity_type.as_str())
                    .unwrap_or("");

                if edge.edge_type.eq_ignore_ascii_case("ubo")
                    || source_kind.eq_ignore_ascii_case("ubo")
                {
                    ubo_ids.insert(edge.source_id.clone());
                }
                if edge.edge_type.eq_ignore_ascii_case("director")
                    || source_kind.eq_ignore_ascii_case("director")
                {
                    director_ids.insert(edge.source_id.clone());
                }
            }

            KybGraphEdge {
                source_id: edge.source_id,
                target_id: edge.target_id,
                edge_type: edge.edge_type,
                ownership_pct: edge.ownership_pct,
                effective_from: edge.effective_from.map(|value| value.to_rfc3339()),
            }
        })
        .collect::<Vec<_>>();

    let legal_name = root.display_name.clone();
    let entity_id = root.id.clone();
    let review_status = root
        .metadata
        .get("reviewStatus")
        .and_then(|value| value.as_str())
        .unwrap_or(root.status.as_str())
        .to_string();

    KybGraphReviewItem {
        entity_id: entity_id.clone(),
        legal_name,
        review_status,
        summary: KybGraphSummary {
            entity_id,
            owner_count: owner_ids.len(),
            ubo_count: ubo_ids.len(),
            director_count: director_ids.len(),
            missing_requirements,
            review_flags,
        },
        nodes,
        edges: edge_models,
    }
}

fn json_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}
