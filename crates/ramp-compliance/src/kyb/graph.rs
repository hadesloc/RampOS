use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

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
