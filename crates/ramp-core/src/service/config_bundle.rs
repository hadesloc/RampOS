use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBundleArtifact {
    pub bundle_id: String,
    pub tenant_name: String,
    pub exported_at: String,
    pub action_mode: String,
    pub sections: Vec<String>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhitelistedExtensionAction {
    pub action_id: String,
    pub label: String,
    pub description: String,
    pub enabled: bool,
}

pub struct ConfigBundleService;

impl ConfigBundleService {
    pub fn new() -> Self {
        Self
    }

    pub fn export_bundle(&self, tenant_name: &str) -> ConfigBundleArtifact {
        ConfigBundleArtifact {
            bundle_id: "cfg_bundle_demo_001".to_string(),
            tenant_name: tenant_name.to_string(),
            exported_at: Utc::now().to_rfc3339(),
            action_mode: "whitelisted_only".to_string(),
            sections: vec![
                "branding".to_string(),
                "domains".to_string(),
                "rate_limits".to_string(),
                "webhook_preferences".to_string(),
            ],
            payload: serde_json::json!({
                "branding": {
                    "primaryColor": "#2563eb",
                    "wordmark": "RampOS Demo"
                },
                "domains": ["demo.rampos.local"],
                "rateLimits": {
                    "apiPerMinute": 100
                },
                "webhooks": {
                    "enabledEvents": ["intent.payin.created", "intent.payout.completed"]
                }
            }),
        }
    }

    pub fn list_whitelisted_actions(&self) -> Vec<WhitelistedExtensionAction> {
        vec![
            WhitelistedExtensionAction {
                action_id: "branding.apply".to_string(),
                label: "Apply branding bundle".to_string(),
                description: "Imports approved branding fields from a config bundle.".to_string(),
                enabled: true,
            },
            WhitelistedExtensionAction {
                action_id: "domains.attach".to_string(),
                label: "Attach domain bundle".to_string(),
                description: "Imports approved custom-domain configuration.".to_string(),
                enabled: true,
            },
            WhitelistedExtensionAction {
                action_id: "webhooks.sync".to_string(),
                label: "Sync webhook preferences".to_string(),
                description: "Imports approved webhook event selections only.".to_string(),
                enabled: true,
            },
        ]
    }
}

impl Default for ConfigBundleService {
    fn default() -> Self {
        Self::new()
    }
}
