use crate::sanctions::{SanctionsEntry, SanctionsProvider, SanctionsResult};
use anyhow::Context;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

#[derive(Clone)]
pub struct OpenSanctionsProvider {
    api_key: String,
    base_url: String,
    client: Client,
    fuzzy_threshold: f64,
}

#[derive(Debug, Deserialize)]
struct OpenSanctionsMatch {
    schema: String,
    properties: serde_json::Map<String, serde_json::Value>,
    id: String,
    score: f64,
    caption: Option<String>,
}

impl OpenSanctionsProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.opensanctions.org".to_string()),
            client: Client::new(),
            fuzzy_threshold: 70.0, // Default to 70% match
        }
    }

    pub fn with_fuzzy_threshold(mut self, threshold: f64) -> Self {
        self.fuzzy_threshold = threshold;
        self
    }

    async fn query_opensanctions(
        &self,
        query: serde_json::Value,
    ) -> anyhow::Result<SanctionsResult> {
        // Handle trailing slash in base_url
        let url = if self.base_url.ends_with('/') {
            format!("{}match/default", self.base_url)
        } else {
            format!("{}/match/default", self.base_url)
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("ApiKey {}", self.api_key))
            .json(&query)
            .send()
            .await
            .context("Failed to send request to OpenSanctions")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "OpenSanctions API error: {} - {}",
                status,
                text
            ));
        }

        let body: serde_json::Value = response.json().await.context("Failed to parse response")?;

        // Extract results for "q1"
        // Structure expected: { "responses": { "q1": { "results": [...] } } }
        let matches: Vec<OpenSanctionsMatch> = if let Some(responses) = body.get("responses") {
            if let Some(q1) = responses.get("q1") {
                if let Some(results) = q1.get("results") {
                    serde_json::from_value::<Vec<OpenSanctionsMatch>>(results.clone())?
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else {
            // Fallback for direct results
            if let Some(results) = body.get("results") {
                serde_json::from_value::<Vec<OpenSanctionsMatch>>(results.clone())?
            } else {
                vec![]
            }
        };

        let mut matched_entries = Vec::new();
        let mut best_score = 0.0;

        for m in matches {
            // OpenSanctions scores are 0-100
            if m.score >= self.fuzzy_threshold {
                if m.score > best_score {
                    best_score = m.score;
                }

                // Extract name
                let name = m
                    .caption
                    .clone()
                    .or_else(|| {
                        m.properties
                            .get("name")
                            .and_then(|v| v.as_array())
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_else(|| "Unknown".to_string());

                matched_entries.push(SanctionsEntry {
                    id: m.id,
                    list_name: m.schema, // Using schema as list name for now
                    name,
                    match_score: m.score,
                });
            }
        }

        if matched_entries.is_empty() {
            Ok(SanctionsResult::clean())
        } else {
            Ok(SanctionsResult::matched(
                best_score,
                "OpenSanctions".to_string(),
                matched_entries,
            ))
        }
    }
}

#[async_trait]
impl SanctionsProvider for OpenSanctionsProvider {
    async fn check_individual(
        &self,
        name: &str,
        dob: Option<&str>,
        country: Option<&str>,
    ) -> anyhow::Result<SanctionsResult> {
        let mut properties = serde_json::Map::new();
        properties.insert("name".to_string(), serde_json::json!([name]));

        if let Some(d) = dob {
            properties.insert("birthDate".to_string(), serde_json::json!([d]));
        }

        if let Some(c) = country {
            properties.insert("country".to_string(), serde_json::json!([c]));
        }

        let query = serde_json::json!({
            "queries": {
                "q1": {
                    "schema": "Person",
                    "properties": properties
                }
            }
        });

        self.query_opensanctions(query).await
    }

    async fn check_entity(
        &self,
        name: &str,
        country: Option<&str>,
    ) -> anyhow::Result<SanctionsResult> {
        let mut properties = serde_json::Map::new();
        properties.insert("name".to_string(), serde_json::json!([name]));

        if let Some(c) = country {
            properties.insert("country".to_string(), serde_json::json!([c]));
        }

        let query = serde_json::json!({
            "queries": {
                "q1": {
                    "schema": "LegalEntity",
                    "properties": properties
                }
            }
        });

        self.query_opensanctions(query).await
    }

    async fn check_address(&self, address: &str) -> anyhow::Result<SanctionsResult> {
        let mut properties = serde_json::Map::new();
        properties.insert("full".to_string(), serde_json::json!([address]));

        let query = serde_json::json!({
            "queries": {
                "q1": {
                    "schema": "Address",
                    "properties": properties
                }
            }
        });

        self.query_opensanctions(query).await
    }

    fn get_list_version(&self) -> String {
        "OPEN-SANCTIONS-LIVE".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_check_individual_match() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "responses": {
                "q1": {
                    "results": [
                        {
                            "id": "test-id-1",
                            "schema": "Person",
                            "score": 95.0,
                            "caption": "Bad Guy",
                            "properties": {
                                "name": ["Bad Guy"],
                                "birthDate": ["1980-01-01"]
                            }
                        }
                    ]
                }
            }
        });

        Mock::given(method("POST"))
            .and(path("/match/default"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let provider = OpenSanctionsProvider::new("test-key".to_string(), Some(mock_server.uri()));
        let result = provider
            .check_individual("Bad Guy", None, None)
            .await
            .expect("Request failed");

        assert!(result.matched);
        assert_eq!(result.score, 95.0);
        assert_eq!(result.matched_entries.len(), 1);
        assert_eq!(result.matched_entries[0].name, "Bad Guy");
    }

    #[tokio::test]
    async fn test_check_individual_no_match() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "responses": {
                "q1": {
                    "results": []
                }
            }
        });

        Mock::given(method("POST"))
            .and(path("/match/default"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
            .mount(&mock_server)
            .await;

        let provider = OpenSanctionsProvider::new("test-key".to_string(), Some(mock_server.uri()));
        let result = provider
            .check_individual("Good Guy", None, None)
            .await
            .expect("Request failed");

        assert!(!result.matched);
        assert_eq!(result.score, 0.0);
        assert!(result.matched_entries.is_empty());
    }
}
