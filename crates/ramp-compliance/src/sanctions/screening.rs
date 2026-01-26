use crate::sanctions::SanctionsProvider;
use crate::types::RiskScore;
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningResult {
    pub risk_score: RiskScore,
    pub matched_entries: Vec<crate::sanctions::SanctionsEntry>,
}

pub struct SanctionsScreeningService {
    provider: Arc<dyn SanctionsProvider>,
}

impl SanctionsScreeningService {
    pub fn new(provider: Arc<dyn SanctionsProvider>) -> Self {
        Self { provider }
    }

    pub async fn screen_user(&self, name: &str, country: Option<&str>) -> Result<ScreeningResult> {
        let result = self
            .provider
            .check_individual(name, None, country)
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "sanctions".to_string(),
                message: e.to_string(),
            })?;

        Ok(ScreeningResult {
            risk_score: RiskScore::new(result.score),
            matched_entries: result.matched_entries,
        })
    }
}
