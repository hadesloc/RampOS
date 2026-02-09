use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::kyt::KytProvider;
use crate::types::{KytResult, RiskScore};
use ramp_common::resilience::ResilientClient;
use ramp_common::types::WalletAddress;
use ramp_common::Result;

/// Chainalysis KYT provider - real API integration
pub struct ChainalysisKytProvider {
    api_key: String,
    base_url: String,
    client: Client,
    resilient: ResilientClient,
}

// ── Chainalysis API request/response types ──

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterTransferRequest {
    network: String,
    asset: String,
    transfer_reference: String,
    direction: String,
    address: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterTransferResponse {
    external_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlertsResponse {
    alerts: Vec<ChainalysisAlert>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChainalysisAlert {
    alert_level: String,
    category: Option<String>,
    service: Option<String>,
    exposure_type: Option<String>,
}

impl ChainalysisKytProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.chainalysis.com/api/kyt".to_string()),
            client,
            resilient: ResilientClient::new("chainalysis"),
        }
    }

    fn map_chain_to_network(chain: &str) -> String {
        match chain.to_lowercase().as_str() {
            "ethereum" | "eth" => "Ethereum".to_string(),
            "bitcoin" | "btc" => "Bitcoin".to_string(),
            "polygon" | "matic" => "Polygon".to_string(),
            "arbitrum" | "arb" => "Arbitrum".to_string(),
            "optimism" | "op" => "Optimism".to_string(),
            "base" => "Base".to_string(),
            "bsc" | "bnb" => "BSC".to_string(),
            "solana" | "sol" => "Solana".to_string(),
            "tron" | "trx" => "Tron".to_string(),
            "avalanche" | "avax" => "Avalanche".to_string(),
            other => other.to_string(),
        }
    }

    /// Register a transfer and get alerts
    async fn register_and_check(
        &self,
        address: &str,
        chain: &str,
    ) -> Result<(Vec<ChainalysisAlert>, String)> {
        let network = Self::map_chain_to_network(chain);
        let transfer_ref = format!("ramp-{}", uuid::Uuid::now_v7());

        let body = RegisterTransferRequest {
            network: network.to_string(),
            asset: network.to_string(), // Default asset = network native token
            transfer_reference: transfer_ref.clone(),
            direction: "received".to_string(),
            address: address.to_string(),
        };

        // Step 1: Register transfer (with circuit breaker)
        let url = format!("{}/v2/transfers", self.base_url);
        let client = self.client.clone();
        let api_key = self.api_key.clone();

        let transfer = self
            .resilient
            .execute(|| {
                let client = client.clone();
                let api_key = api_key.clone();
                let url = url.clone();
                let body_json = serde_json::to_value(&body).unwrap();
                async move {
                    let response = client
                        .post(&url)
                        .header("Token", &api_key)
                        .header("Content-Type", "application/json")
                        .json(&body_json)
                        .send()
                        .await
                        .map_err(|e| {
                            ramp_common::Error::Internal(format!(
                                "Chainalysis register transfer failed: {}",
                                e
                            ))
                        })?;

                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        return Err(ramp_common::Error::Internal(format!(
                            "Chainalysis API error {}: {}",
                            status, text
                        )));
                    }

                    let transfer: RegisterTransferResponse =
                        response.json().await.map_err(|e| {
                            ramp_common::Error::Internal(format!(
                                "Failed to parse Chainalysis transfer response: {}",
                                e
                            ))
                        })?;

                    Ok(transfer)
                }
            })
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "chainalysis".to_string(),
                message: format!("{}", e),
            })?;

        // Step 2: Get alerts for the transfer (with circuit breaker)
        let alerts_url = format!(
            "{}/v2/transfers/{}/alerts",
            self.base_url, transfer.external_id
        );
        let client = self.client.clone();
        let api_key = self.api_key.clone();

        let alerts = self
            .resilient
            .execute(|| {
                let client = client.clone();
                let api_key = api_key.clone();
                let alerts_url = alerts_url.clone();
                async move {
                    let alerts_response = client
                        .get(&alerts_url)
                        .header("Token", &api_key)
                        .send()
                        .await
                        .map_err(|e| {
                            ramp_common::Error::Internal(format!(
                                "Chainalysis get alerts failed: {}",
                                e
                            ))
                        })?;

                    if !alerts_response.status().is_success() {
                        let status = alerts_response.status();
                        let text = alerts_response.text().await.unwrap_or_default();
                        return Err(ramp_common::Error::Internal(format!(
                            "Chainalysis alerts API error {}: {}",
                            status, text
                        )));
                    }

                    let alerts: AlertsResponse =
                        alerts_response.json().await.map_err(|e| {
                            ramp_common::Error::Internal(format!(
                                "Failed to parse Chainalysis alerts response: {}",
                                e
                            ))
                        })?;

                    Ok(alerts)
                }
            })
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "chainalysis".to_string(),
                message: format!("{}", e),
            })?;

        Ok((alerts.alerts, transfer.external_id))
    }

    /// Map Chainalysis alert level to risk score
    fn alert_level_to_score(level: &str) -> f64 {
        match level.to_lowercase().as_str() {
            "severe" => 100.0,
            "high" => 85.0,
            "medium" => 55.0,
            "low" => 25.0,
            _ => 10.0,
        }
    }
}

#[async_trait]
impl KytProvider for ChainalysisKytProvider {
    async fn check_address(&self, address: &WalletAddress, chain: &str) -> Result<KytResult> {
        info!(
            address = %address,
            chain = chain,
            "Checking address with Chainalysis KYT"
        );

        let (alerts, _external_id) = self.register_and_check(&address.0, chain).await?;

        let mut risk_signals = Vec::new();
        let mut max_score: f64 = 0.0;
        let mut is_sanctioned = false;

        for alert in &alerts {
            let score = Self::alert_level_to_score(&alert.alert_level);
            if score > max_score {
                max_score = score;
            }

            let mut signal = format!("Alert: {}", alert.alert_level);
            if let Some(ref category) = alert.category {
                signal.push_str(&format!(" - {}", category));
                if category.to_lowercase().contains("sanctions") {
                    is_sanctioned = true;
                }
            }
            if let Some(ref service) = alert.service {
                signal.push_str(&format!(" ({})", service));
            }
            if let Some(ref exposure) = alert.exposure_type {
                signal.push_str(&format!(" [{}]", exposure));
            }
            risk_signals.push(signal);
        }

        info!(
            address = %address,
            risk_score = max_score,
            alert_count = alerts.len(),
            is_sanctioned = is_sanctioned,
            "Chainalysis KYT check completed"
        );

        Ok(KytResult {
            address: address.0.clone(),
            risk_score: RiskScore::new(max_score),
            risk_signals,
            is_sanctioned,
            checked_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_check_address_clean() {
        let mock_server = MockServer::start().await;

        // Mock register transfer
        Mock::given(method("POST"))
            .and(path("/v2/transfers"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "externalId": "ext-123"
            })))
            .mount(&mock_server)
            .await;

        // Mock get alerts - no alerts
        Mock::given(method("GET"))
            .and(path_regex(r"/v2/transfers/.+/alerts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "alerts": []
            })))
            .mount(&mock_server)
            .await;

        let provider =
            ChainalysisKytProvider::new("test-key".to_string(), Some(mock_server.uri()));
        let address = WalletAddress("0x1234567890abcdef".to_string());
        let result = provider
            .check_address(&address, "ethereum")
            .await
            .expect("check_address failed");

        assert_eq!(result.risk_score.0, 0.0);
        assert!(!result.is_sanctioned);
        assert!(result.risk_signals.is_empty());
    }

    #[tokio::test]
    async fn test_check_address_high_risk() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v2/transfers"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "externalId": "ext-456"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/v2/transfers/.+/alerts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "alerts": [
                    {
                        "alertLevel": "SEVERE",
                        "category": "sanctions",
                        "service": "OFAC SDN",
                        "exposureType": "direct"
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        let provider =
            ChainalysisKytProvider::new("test-key".to_string(), Some(mock_server.uri()));
        let address = WalletAddress("0xbadaddress".to_string());
        let result = provider
            .check_address(&address, "ethereum")
            .await
            .expect("check_address failed");

        assert_eq!(result.risk_score.0, 100.0);
        assert!(result.is_sanctioned);
        assert!(!result.risk_signals.is_empty());
    }

    #[tokio::test]
    async fn test_check_address_medium_risk() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v2/transfers"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "externalId": "ext-789"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/v2/transfers/.+/alerts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "alerts": [
                    {
                        "alertLevel": "MEDIUM",
                        "category": "darknet market",
                        "service": null,
                        "exposureType": "indirect"
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        let provider =
            ChainalysisKytProvider::new("test-key".to_string(), Some(mock_server.uri()));
        let address = WalletAddress("0xsuspicious".to_string());
        let result = provider
            .check_address(&address, "ethereum")
            .await
            .expect("check_address failed");

        assert_eq!(result.risk_score.0, 55.0);
        assert!(!result.is_sanctioned);
    }

    #[tokio::test]
    async fn test_api_error_handling() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v2/transfers"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&mock_server)
            .await;

        let provider =
            ChainalysisKytProvider::new("bad-key".to_string(), Some(mock_server.uri()));
        let address = WalletAddress("0x123".to_string());
        let result = provider.check_address(&address, "ethereum").await;

        assert!(result.is_err());
    }

    #[test]
    fn test_chain_mapping() {
        assert_eq!(ChainalysisKytProvider::map_chain_to_network("eth"), "Ethereum");
        assert_eq!(ChainalysisKytProvider::map_chain_to_network("btc"), "Bitcoin");
        assert_eq!(ChainalysisKytProvider::map_chain_to_network("polygon"), "Polygon");
        assert_eq!(ChainalysisKytProvider::map_chain_to_network("base"), "Base");
        assert_eq!(ChainalysisKytProvider::map_chain_to_network("unknown"), "unknown");
    }
}
