use crate::sanctions::provider::{SanctionsEntry, SanctionsProvider, SanctionsResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MockSanctionsProvider {
    blocked_names: Arc<Mutex<HashMap<String, f64>>>, // Name -> Match Score
    blocked_entities: Arc<Mutex<HashMap<String, f64>>>, // Name -> Match Score
    list_version: String,
}

impl Default for MockSanctionsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSanctionsProvider {
    pub fn new() -> Self {
        Self {
            blocked_names: Arc::new(Mutex::new(HashMap::new())),
            blocked_entities: Arc::new(Mutex::new(HashMap::new())),
            list_version: "2024-01-01-MOCK".to_string(),
        }
    }

    pub fn add_blocked_individual(&self, name: &str, score: f64) {
        if let Ok(mut names) = self.blocked_names.lock() {
            names.insert(name.to_lowercase(), score);
        }
    }

    pub fn add_blocked_entity(&self, name: &str, score: f64) {
        if let Ok(mut entities) = self.blocked_entities.lock() {
            entities.insert(name.to_lowercase(), score);
        }
    }

    pub fn clear(&self) {
        if let Ok(mut names) = self.blocked_names.lock() {
            names.clear();
        }
        if let Ok(mut entities) = self.blocked_entities.lock() {
            entities.clear();
        }
    }
}

#[async_trait]
impl SanctionsProvider for MockSanctionsProvider {
    async fn check_individual(
        &self,
        name: &str,
        _dob: Option<&str>,
        _country: Option<&str>,
    ) -> anyhow::Result<SanctionsResult> {
        let names = self
            .blocked_names
            .lock()
            .map_err(|e| anyhow::anyhow!("Blocked names lock poisoned: {}", e))?;
        let normalized_name = name.to_lowercase();

        if let Some(score) = names.get(&normalized_name) {
            let entry = SanctionsEntry {
                id: "MOCK-123".to_string(),
                list_name: "OFAC-MOCK".to_string(),
                name: name.to_string(),
                match_score: *score,
            };

            return Ok(SanctionsResult::matched(
                *score,
                "OFAC-MOCK".to_string(),
                vec![entry],
            ));
        }

        // Check partial matches
        for (blocked_name, score) in names.iter() {
            if normalized_name.contains(blocked_name) || blocked_name.contains(&normalized_name) {
                let entry = SanctionsEntry {
                    id: "MOCK-456".to_string(),
                    list_name: "EU-MOCK".to_string(),
                    name: blocked_name.to_string(),
                    match_score: *score,
                };
                return Ok(SanctionsResult::matched(
                    *score,
                    "EU-MOCK".to_string(),
                    vec![entry],
                ));
            }
        }

        Ok(SanctionsResult::clean())
    }

    async fn check_entity(
        &self,
        name: &str,
        _country: Option<&str>,
    ) -> anyhow::Result<SanctionsResult> {
        let entities = self
            .blocked_entities
            .lock()
            .map_err(|e| anyhow::anyhow!("Blocked entities lock poisoned: {}", e))?;
        let normalized_name = name.to_lowercase();

        if let Some(score) = entities.get(&normalized_name) {
            let entry = SanctionsEntry {
                id: "MOCK-ENT-123".to_string(),
                list_name: "UN-MOCK".to_string(),
                name: name.to_string(),
                match_score: *score,
            };

            return Ok(SanctionsResult::matched(
                *score,
                "UN-MOCK".to_string(),
                vec![entry],
            ));
        }

        Ok(SanctionsResult::clean())
    }

    async fn check_address(&self, _address: &str) -> anyhow::Result<SanctionsResult> {
        // Mock implementation for address check - always clean for now
        Ok(SanctionsResult::clean())
    }

    fn get_list_version(&self) -> String {
        self.list_version.clone()
    }
}
