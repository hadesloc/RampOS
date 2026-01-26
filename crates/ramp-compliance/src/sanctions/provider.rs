use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsEntry {
    pub id: String,
    pub list_name: String,
    pub name: String,
    pub match_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsResult {
    pub matched: bool,
    pub score: f64, // 0-100
    pub matched_entries: Vec<SanctionsEntry>,
    pub list_name: Option<String>,
}

impl SanctionsResult {
    pub fn clean() -> Self {
        Self {
            matched: false,
            score: 0.0,
            matched_entries: vec![],
            list_name: None,
        }
    }

    pub fn matched(score: f64, list_name: String, entries: Vec<SanctionsEntry>) -> Self {
        Self {
            matched: true,
            score,
            matched_entries: entries,
            list_name: Some(list_name),
        }
    }
}

#[async_trait]
pub trait SanctionsProvider: Send + Sync {
    async fn check_individual(
        &self,
        name: &str,
        dob: Option<&str>,
        country: Option<&str>,
    ) -> anyhow::Result<SanctionsResult>;
    async fn check_entity(
        &self,
        name: &str,
        country: Option<&str>,
    ) -> anyhow::Result<SanctionsResult>;
    async fn check_address(&self, address: &str) -> anyhow::Result<SanctionsResult>;
    fn get_list_version(&self) -> String;
}
