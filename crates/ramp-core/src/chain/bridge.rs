//! Bridge Adapter Layer
//!
//! Provides traits and mock implementations for cross-chain bridge operations.

use async_trait::async_trait;
use ramp_common::onchain_gate;
use serde::{Deserialize, Serialize};

use super::{ChainError, ChainId, Result};

/// Quote for a cross-chain bridge transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeQuote {
    pub source_chain: ChainId,
    pub dest_chain: ChainId,
    /// Token address on source chain
    pub token: String,
    /// Amount to bridge (in smallest denomination)
    pub amount: u128,
    /// Bridge fee (in smallest denomination)
    pub fee: u128,
    /// Estimated gas cost on destination chain
    pub dest_gas_cost: u128,
    /// Amount received on destination (amount - fee - gas)
    pub amount_received: u128,
    /// Estimated transfer time in seconds
    pub estimated_time_secs: u64,
}

/// Status of a bridge transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BridgeTransferStatus {
    Initiated,
    SourceConfirmed,
    InTransit,
    DestConfirmed,
    Completed,
    Failed,
}

/// Result of initiating a bridge transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTransferResult {
    /// Unique bridge transfer ID
    pub transfer_id: String,
    /// Source chain transaction hash
    pub source_tx_hash: String,
    pub status: BridgeTransferStatus,
}

/// Current status of a bridge transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStatusResponse {
    pub transfer_id: String,
    pub status: BridgeTransferStatus,
    /// Destination chain transaction hash (available once completed)
    pub dest_tx_hash: Option<String>,
    /// Elapsed time in seconds
    pub elapsed_secs: u64,
}

/// Trait for cross-chain bridge adapters
#[async_trait]
pub trait BridgeAdapter: Send + Sync {
    /// Get a quote for bridging tokens
    async fn get_bridge_quote(
        &self,
        source_chain: ChainId,
        dest_chain: ChainId,
        token: &str,
        amount: u128,
    ) -> Result<BridgeQuote>;

    /// Initiate a bridge transfer
    async fn initiate_bridge(
        &self,
        quote: &BridgeQuote,
        sender: &str,
        recipient: &str,
    ) -> Result<BridgeTransferResult>;

    /// Check the status of a bridge transfer
    async fn check_bridge_status(&self, transfer_id: &str) -> Result<BridgeStatusResponse>;
}

/// Mock bridge adapter simulating a Stargate-like cross-chain bridge.
///
/// Uses 0.1% fee + fixed gas cost for tests and local simulation only. It is
/// not registered anywhere in production; constructors also fail closed when
/// `RUST_ENV`/`RAMPOS_ENV` is `production`.
pub struct MockBridgeAdapter {
    /// Fee in basis points (10 = 0.1%)
    fee_bps: u32,
    /// Fixed gas cost in raw token units
    fixed_gas_cost: u128,
}

impl MockBridgeAdapter {
    fn ensure_not_production() {
        assert!(
            !onchain_gate::is_production(),
            "MockBridgeAdapter is test/local simulation only and must not be constructed in production"
        );
    }

    pub fn new() -> Self {
        Self::ensure_not_production();
        Self {
            fee_bps: 10,
            fixed_gas_cost: 50_000_000_000_000, // ~0.00005 ETH in wei
        }
    }

    pub fn with_params(fee_bps: u32, fixed_gas_cost: u128) -> Self {
        Self::ensure_not_production();
        Self {
            fee_bps,
            fixed_gas_cost,
        }
    }

    /// Estimated time based on chain pair
    fn estimated_time(source: ChainId, dest: ChainId) -> u64 {
        // L2 -> L2 is fast, L1 involved is slower
        match (source, dest) {
            (s, d) if s == ChainId::ETHEREUM || d == ChainId::ETHEREUM => 900, // 15 min
            _ => 120,                                                          // 2 min for L2<->L2
        }
    }
}

impl Default for MockBridgeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BridgeAdapter for MockBridgeAdapter {
    async fn get_bridge_quote(
        &self,
        source_chain: ChainId,
        dest_chain: ChainId,
        token: &str,
        amount: u128,
    ) -> Result<BridgeQuote> {
        if amount == 0 {
            return Err(ChainError::Internal("Bridge amount must be > 0".into()));
        }
        if source_chain == dest_chain {
            return Err(ChainError::Internal(
                "Source and destination chain must differ".into(),
            ));
        }
        if token.is_empty() {
            return Err(ChainError::Internal("Token address is empty".into()));
        }

        let fee = amount * self.fee_bps as u128 / 10_000;
        let dest_gas_cost = self.fixed_gas_cost;
        let amount_received = amount.saturating_sub(fee).saturating_sub(dest_gas_cost);
        let estimated_time_secs = Self::estimated_time(source_chain, dest_chain);

        Ok(BridgeQuote {
            source_chain,
            dest_chain,
            token: token.to_string(),
            amount,
            fee,
            dest_gas_cost,
            amount_received,
            estimated_time_secs,
        })
    }

    async fn initiate_bridge(
        &self,
        quote: &BridgeQuote,
        sender: &str,
        recipient: &str,
    ) -> Result<BridgeTransferResult> {
        if sender.is_empty() {
            return Err(ChainError::InvalidAddress("Sender address is empty".into()));
        }
        if recipient.is_empty() {
            return Err(ChainError::InvalidAddress(
                "Recipient address is empty".into(),
            ));
        }

        // Deterministic IDs based on inputs
        let transfer_id = format!(
            "bridge-{}-{}-{}",
            quote.source_chain.0, quote.dest_chain.0, quote.amount
        );
        let source_tx_hash = format!("0x{:0>64x}", (quote.amount as u64).wrapping_mul(0xCAFEBABE));

        Ok(BridgeTransferResult {
            transfer_id,
            source_tx_hash,
            status: BridgeTransferStatus::Initiated,
        })
    }

    async fn check_bridge_status(&self, transfer_id: &str) -> Result<BridgeStatusResponse> {
        if transfer_id.is_empty() {
            return Err(ChainError::Internal("Transfer ID is empty".into()));
        }

        // Mock: always return Completed for valid IDs
        let dest_tx_hash = format!("0x{:0>64x}", transfer_id.len() as u64 * 0xBEEFCAFE);

        Ok(BridgeStatusResponse {
            transfer_id: transfer_id.to_string(),
            status: BridgeTransferStatus::Completed,
            dest_tx_hash: Some(dest_tx_hash),
            elapsed_secs: 120,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construct a `MockBridgeAdapter` deterministically.
    ///
    /// `MockBridgeAdapter::new()` asserts it is not in production by reading the
    /// process-global `RUST_ENV`/`RAMPOS_ENV`. Other tests in this binary toggle
    /// those vars to "production" (e.g. `test_mock_bridge_constructor_rejected_in_production`,
    /// the Stripe production test), so unguarded construction races and panics
    /// intermittently. Hold the shared env lock while clearing the vars and
    /// constructing; the adapter never re-reads env afterwards, so releasing the
    /// lock on return is safe even across later `.await` points.
    fn new_mock_adapter() -> MockBridgeAdapter {
        let _guard = onchain_gate::test_env_lock();
        std::env::remove_var("RUST_ENV");
        std::env::remove_var("RAMPOS_ENV");
        MockBridgeAdapter::new()
    }

    #[tokio::test]
    async fn test_mock_bridge_quote_basic() {
        let adapter = new_mock_adapter();
        let quote = adapter
            .get_bridge_quote(
                ChainId::ETHEREUM,
                ChainId::ARBITRUM,
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
                1_000_000_000_000_000_000,
            )
            .await
            .unwrap();

        assert_eq!(quote.source_chain, ChainId::ETHEREUM);
        assert_eq!(quote.dest_chain, ChainId::ARBITRUM);
        assert_eq!(quote.amount, 1_000_000_000_000_000_000);
        assert!(quote.fee > 0);
        assert!(quote.amount_received < quote.amount);
        assert_eq!(quote.estimated_time_secs, 900); // L1 involved
    }

    #[test]
    fn test_mock_bridge_constructor_rejected_in_production() {
        let _guard = onchain_gate::test_env_lock();
        std::env::set_var("RAMPOS_ENV", "production");

        let result = std::panic::catch_unwind(MockBridgeAdapter::new);
        assert!(result.is_err());

        std::env::remove_var("RAMPOS_ENV");
    }

    #[tokio::test]
    async fn test_mock_bridge_l2_to_l2_faster() {
        let adapter = new_mock_adapter();

        let l1_quote = adapter
            .get_bridge_quote(ChainId::ETHEREUM, ChainId::ARBITRUM, "USDC", 1_000_000)
            .await
            .unwrap();

        let l2_quote = adapter
            .get_bridge_quote(ChainId::ARBITRUM, ChainId::BASE, "USDC", 1_000_000)
            .await
            .unwrap();

        assert!(
            l2_quote.estimated_time_secs < l1_quote.estimated_time_secs,
            "L2->L2 should be faster than L1->L2"
        );
    }

    #[tokio::test]
    async fn test_mock_bridge_fee_calculation() {
        let adapter = new_mock_adapter();
        let amount = 10_000_000_000_000_000_000u128; // 10 ETH
        let quote = adapter
            .get_bridge_quote(ChainId::ARBITRUM, ChainId::BASE, "WETH", amount)
            .await
            .unwrap();

        // 0.1% of 10 ETH = 0.01 ETH
        let expected_fee = amount / 1000; // 10 bps = 0.1%
        assert_eq!(quote.fee, expected_fee);
        assert_eq!(
            quote.amount_received,
            amount - expected_fee - quote.dest_gas_cost
        );
    }

    #[tokio::test]
    async fn test_mock_bridge_zero_amount_rejected() {
        let adapter = new_mock_adapter();
        let result = adapter
            .get_bridge_quote(ChainId::ETHEREUM, ChainId::ARBITRUM, "USDC", 0)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_bridge_same_chain_rejected() {
        let adapter = new_mock_adapter();
        let result = adapter
            .get_bridge_quote(ChainId::ETHEREUM, ChainId::ETHEREUM, "USDC", 1000)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_bridge_initiate() {
        let adapter = new_mock_adapter();
        let quote = adapter
            .get_bridge_quote(ChainId::ETHEREUM, ChainId::ARBITRUM, "USDC", 1_000_000)
            .await
            .unwrap();

        let result = adapter
            .initiate_bridge(
                &quote,
                "0x1234567890123456789012345678901234567890",
                "0x0987654321098765432109876543210987654321",
            )
            .await
            .unwrap();

        assert_eq!(result.status, BridgeTransferStatus::Initiated);
        assert!(!result.transfer_id.is_empty());
        assert!(result.source_tx_hash.starts_with("0x"));
    }

    #[tokio::test]
    async fn test_mock_bridge_initiate_empty_sender_rejected() {
        let adapter = new_mock_adapter();
        let quote = adapter
            .get_bridge_quote(ChainId::ETHEREUM, ChainId::BASE, "USDC", 1000)
            .await
            .unwrap();

        let result = adapter.initiate_bridge(&quote, "", "0xrecipient").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_bridge_check_status() {
        let adapter = new_mock_adapter();
        let status = adapter
            .check_bridge_status("bridge-1-42161-1000000")
            .await
            .unwrap();

        assert_eq!(status.status, BridgeTransferStatus::Completed);
        assert!(status.dest_tx_hash.is_some());
        assert_eq!(status.transfer_id, "bridge-1-42161-1000000");
    }

    #[tokio::test]
    async fn test_mock_bridge_check_status_empty_id_rejected() {
        let adapter = new_mock_adapter();
        let result = adapter.check_bridge_status("").await;
        assert!(result.is_err());
    }
}
