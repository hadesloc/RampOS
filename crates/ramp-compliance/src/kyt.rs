use async_trait::async_trait;
use ramp_common::{types::WalletAddress, Result};
use tracing::info;

use crate::types::{KytResult, RiskScore};

/// KYT Provider trait
#[async_trait]
pub trait KytProvider: Send + Sync {
    async fn check_address(&self, address: &WalletAddress, chain: &str) -> Result<KytResult>;
}

/// KYT Service
pub struct KytService {
    provider: Box<dyn KytProvider>,
    high_risk_threshold: f64,
}

impl KytService {
    pub fn new(provider: Box<dyn KytProvider>) -> Self {
        Self {
            provider,
            high_risk_threshold: 70.0,
        }
    }

    /// Check if an address is safe to transact with
    pub async fn check_address(&self, address: &WalletAddress, chain: &str) -> Result<KytResult> {
        info!(
            address = %address,
            chain = chain,
            "Checking address with KYT"
        );

        let result = self.provider.check_address(address, chain).await?;

        info!(
            address = %address,
            risk_score = result.risk_score.0,
            is_sanctioned = result.is_sanctioned,
            "KYT check completed"
        );

        Ok(result)
    }

    /// Check if address is high risk
    pub async fn is_high_risk(&self, address: &WalletAddress, chain: &str) -> Result<bool> {
        let result = self.check_address(address, chain).await?;
        Ok(result.risk_score.0 >= self.high_risk_threshold || result.is_sanctioned)
    }
}

/// Mock KYT provider for testing
pub struct MockKytProvider;

#[async_trait]
impl KytProvider for MockKytProvider {
    async fn check_address(&self, address: &WalletAddress, _chain: &str) -> Result<KytResult> {
        // Simulate KYT check
        // Known "bad" addresses for testing
        let is_sanctioned = address.0.to_lowercase().contains("bad");
        let risk_score = if is_sanctioned { 100.0 } else { 10.0 };

        Ok(KytResult {
            address: address.0.clone(),
            risk_score: RiskScore::new(risk_score),
            risk_signals: if is_sanctioned {
                vec!["Sanctioned address".to_string()]
            } else {
                vec![]
            },
            is_sanctioned,
            checked_at: chrono::Utc::now(),
        })
    }
}
