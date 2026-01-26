use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Action to take based on risk score vs thresholds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThresholdAction {
    Approve,
    ApproveWithFlag,
    HoldForReview,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    pub tenant_id: String,
    /// Score below this is auto-approved (e.g., 30.0)
    pub auto_approve_threshold: f64,
    /// Score below this but above auto-approve is flagged (e.g., 60.0)
    pub manual_review_threshold: f64,
    /// Score above this is auto-blocked (e.g., 80.0)
    pub auto_block_threshold: f64,

    pub velocity_max_per_hour: i32,
    pub velocity_max_per_day: i32,
    pub structuring_window_hours: i32,
    pub structuring_threshold_vnd: i64,
}

impl ThresholdConfig {
    pub fn default_for(tenant_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            // Default values based on 0-100 RiskScore scale
            // Note: Requirement mentioned e.g. 0.3, assuming that meant normalized,
            // but RiskScore is 0-100 in types.rs. Using 30.0, 60.0, 80.0 for consistency.
            auto_approve_threshold: 30.0,
            manual_review_threshold: 60.0,
            auto_block_threshold: 80.0,
            velocity_max_per_hour: 5,
            velocity_max_per_day: 20,
            structuring_window_hours: 24,
            structuring_threshold_vnd: 50_000_000,
        }
    }

    pub fn determine_action(&self, score: f64) -> ThresholdAction {
        if score < self.auto_approve_threshold {
            ThresholdAction::Approve
        } else if score < self.manual_review_threshold {
            ThresholdAction::ApproveWithFlag
        } else if score < self.auto_block_threshold {
            ThresholdAction::HoldForReview
        } else {
            ThresholdAction::Block
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThresholdManager {
    // In-memory storage for now, replaced by DB in future
    configs: Arc<RwLock<HashMap<String, ThresholdConfig>>>,
}

impl ThresholdManager {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_config(&self, tenant_id: &str) -> ThresholdConfig {
        let configs = self.configs.read().unwrap();
        configs
            .get(tenant_id)
            .cloned()
            .unwrap_or_else(|| ThresholdConfig::default_for(tenant_id))
    }

    pub fn update_config(&self, tenant_id: &str, config: ThresholdConfig) -> Result<()> {
        let mut configs = self.configs.write().unwrap();
        configs.insert(tenant_id.to_string(), config);
        Ok(())
    }

    pub fn get_default_config() -> ThresholdConfig {
        ThresholdConfig::default_for("default")
    }
}

impl Default for ThresholdManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ThresholdManager::get_default_config();
        assert_eq!(config.tenant_id, "default");
        assert_eq!(config.auto_approve_threshold, 30.0);
    }

    #[test]
    fn test_determine_action() {
        let config = ThresholdConfig::default_for("test");

        // < 30 -> Approve
        assert_eq!(config.determine_action(10.0), ThresholdAction::Approve);

        // 30 <= score < 60 -> ApproveWithFlag
        assert_eq!(
            config.determine_action(30.0),
            ThresholdAction::ApproveWithFlag
        );
        assert_eq!(
            config.determine_action(50.0),
            ThresholdAction::ApproveWithFlag
        );

        // 60 <= score < 80 -> HoldForReview
        assert_eq!(
            config.determine_action(60.0),
            ThresholdAction::HoldForReview
        );
        assert_eq!(
            config.determine_action(70.0),
            ThresholdAction::HoldForReview
        );

        // >= 80 -> Block
        assert_eq!(config.determine_action(80.0), ThresholdAction::Block);
        assert_eq!(config.determine_action(90.0), ThresholdAction::Block);
    }

    #[test]
    fn test_manager_update() {
        let manager = ThresholdManager::new();
        let tenant = "tenant1";

        let initial = manager.get_config(tenant);
        assert_eq!(initial.velocity_max_per_hour, 5); // Default

        let mut new_config = initial.clone();
        new_config.velocity_max_per_hour = 10;

        manager.update_config(tenant, new_config).unwrap();

        let updated = manager.get_config(tenant);
        assert_eq!(updated.velocity_max_per_hour, 10);
    }
}
