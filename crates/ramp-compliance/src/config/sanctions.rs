use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanctionsConfig {
    pub provider: String, // "opensanctions", "mock", etc.
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub fuzzy_threshold: Option<f64>,
}

impl Default for SanctionsConfig {
    fn default() -> Self {
        Self {
            provider: "mock".to_string(),
            api_key: None,
            base_url: None,
            fuzzy_threshold: Some(0.8),
        }
    }
}
