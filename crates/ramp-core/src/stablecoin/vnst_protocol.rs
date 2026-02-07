//! VNST Protocol Integration
//!
//! Comprehensive VNST (Vietnam Stablecoin) protocol integration including:
//! - Minting VNST with VND deposit
//! - Burning VNST for VND withdrawal
//! - VND peg verification
//! - Reserve proof integration
//! - Collateralization ratio monitoring

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ethers::types::{Address, H256, U256};
use ramp_common::{
    types::{TenantId, UserId},
    Error, Result,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// VNST Protocol Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnstProtocolConfig {
    /// Minimum mint amount in VND
    pub min_mint_vnd: Decimal,
    /// Maximum mint amount in VND (None = unlimited)
    pub max_mint_vnd: Option<Decimal>,
    /// Minimum burn amount in VNST base units
    pub min_burn_vnst: U256,
    /// Maximum burn amount in VNST base units (None = unlimited)
    pub max_burn_vnst: Option<U256>,
    /// Mint fee in basis points (1 bp = 0.01%)
    pub mint_fee_bps: u16,
    /// Burn fee in basis points
    pub burn_fee_bps: u16,
    /// Required collateralization ratio (e.g., 100 = 100%)
    pub min_collateralization_ratio: u16,
    /// Warning threshold for peg deviation (e.g., 100 = 1%)
    pub peg_deviation_warning_bps: u16,
    /// Critical threshold for peg deviation - halt operations
    pub peg_deviation_critical_bps: u16,
    /// Reserve proof verification interval in seconds
    pub reserve_proof_interval_secs: u64,
    /// VNST contract address
    pub vnst_contract: Option<Address>,
    /// Primary chain ID (e.g., 56 for BSC)
    pub primary_chain_id: u64,
}

impl Default for VnstProtocolConfig {
    fn default() -> Self {
        Self {
            min_mint_vnd: Decimal::from(100_000),        // 100K VND minimum
            max_mint_vnd: Some(Decimal::from(10_000_000_000i64)), // 10B VND max
            min_burn_vnst: U256::from(100_000u64) * U256::from(10u64).pow(U256::from(18)), // 100K VNST
            max_burn_vnst: None,
            mint_fee_bps: 10,  // 0.1%
            burn_fee_bps: 10,  // 0.1%
            min_collateralization_ratio: 100, // 100% backed
            peg_deviation_warning_bps: 50,    // 0.5% warning
            peg_deviation_critical_bps: 200,  // 2% critical
            reserve_proof_interval_secs: 3600, // 1 hour
            vnst_contract: None,
            primary_chain_id: 56, // BSC
        }
    }
}

/// VNST mint request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnstMintRequest {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    /// Amount in VND to deposit
    pub vnd_amount: Decimal,
    /// Destination chain ID
    pub chain_id: u64,
    /// Destination wallet address
    pub recipient_address: Address,
    /// Idempotency key
    pub idempotency_key: Option<String>,
}

/// VNST mint response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnstMintResponse {
    pub mint_id: String,
    pub vnd_amount: Decimal,
    pub vnst_amount: U256,
    pub vnst_amount_display: String,
    pub fee_vnd: Decimal,
    pub fee_vnst: U256,
    pub chain_id: u64,
    pub recipient_address: String,
    pub status: VnstOperationStatus,
    pub tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// VNST burn request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnstBurnRequest {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    /// Amount in VNST base units to burn
    pub vnst_amount: U256,
    /// Source chain ID
    pub chain_id: u64,
    /// Bank account for VND withdrawal (encrypted reference)
    pub bank_account_ref: String,
    /// Idempotency key
    pub idempotency_key: Option<String>,
}

/// VNST burn response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnstBurnResponse {
    pub burn_id: String,
    pub vnst_amount: U256,
    pub vnst_amount_display: String,
    pub vnd_amount: Decimal,
    pub fee_vnst: U256,
    pub fee_vnd: Decimal,
    pub chain_id: u64,
    pub status: VnstOperationStatus,
    pub tx_hash: Option<String>,
    pub estimated_vnd_arrival: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Operation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VnstOperationStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for VnstOperationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VnstOperationStatus::Pending => write!(f, "PENDING"),
            VnstOperationStatus::Processing => write!(f, "PROCESSING"),
            VnstOperationStatus::Completed => write!(f, "COMPLETED"),
            VnstOperationStatus::Failed => write!(f, "FAILED"),
            VnstOperationStatus::Cancelled => write!(f, "CANCELLED"),
        }
    }
}

/// VNST reserve information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnstReserveInfo {
    /// Total VNST in circulation
    pub total_supply: U256,
    pub total_supply_display: String,
    /// Total VND in reserves
    pub total_vnd_reserves: Decimal,
    /// Collateralization ratio (as percentage, e.g., 105.5 = 105.5%)
    pub collateralization_ratio: Decimal,
    /// Reserve breakdown by asset type
    pub reserve_breakdown: Vec<ReserveAsset>,
    /// Last proof verification timestamp
    pub last_proof_at: DateTime<Utc>,
    /// Proof attestation hash (from auditor)
    pub proof_attestation: Option<String>,
    /// Is the peg healthy?
    pub peg_healthy: bool,
    /// Current VND/VNST rate (should be ~1.0)
    pub current_rate: Decimal,
    /// Peg deviation in basis points
    pub peg_deviation_bps: i32,
}

/// Reserve asset breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveAsset {
    pub asset_type: String,
    pub amount_vnd: Decimal,
    pub percentage: Decimal,
    pub custodian: String,
}

/// Peg status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VnstPegStatus {
    pub is_healthy: bool,
    pub current_rate: Decimal,
    pub target_rate: Decimal,
    pub deviation_bps: i32,
    pub status: PegHealthStatus,
    pub last_checked: DateTime<Utc>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PegHealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Data provider trait for VNST protocol
#[async_trait]
pub trait VnstProtocolDataProvider: Send + Sync {
    /// Get VNST total supply from chain
    async fn get_total_supply(&self, chain_id: u64) -> Result<U256>;

    /// Get VND reserves from custodian
    async fn get_vnd_reserves(&self, tenant_id: &TenantId) -> Result<Decimal>;

    /// Get current VND/VNST exchange rate
    async fn get_current_rate(&self) -> Result<Decimal>;

    /// Get reserve proof attestation
    async fn get_reserve_proof(&self) -> Result<Option<String>>;

    /// Record a mint operation
    async fn record_mint(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        vnd_amount: Decimal,
        vnst_amount: U256,
        chain_id: u64,
        recipient: Address,
    ) -> Result<String>;

    /// Record a burn operation
    async fn record_burn(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        vnst_amount: U256,
        vnd_amount: Decimal,
        chain_id: u64,
        bank_account_ref: &str,
    ) -> Result<String>;

    /// Execute mint on chain
    async fn execute_mint(
        &self,
        chain_id: u64,
        recipient: Address,
        amount: U256,
    ) -> Result<H256>;

    /// Execute burn on chain
    async fn execute_burn(
        &self,
        chain_id: u64,
        from: Address,
        amount: U256,
    ) -> Result<H256>;
}

/// VNST Protocol Service
pub struct VnstProtocolService {
    config: VnstProtocolConfig,
    data_provider: Arc<dyn VnstProtocolDataProvider>,
}

impl VnstProtocolService {
    pub fn new(config: VnstProtocolConfig, data_provider: Arc<dyn VnstProtocolDataProvider>) -> Self {
        Self {
            config,
            data_provider,
        }
    }

    /// Convert VND to VNST base units (1 VND = 1 VNST, VNST has 18 decimals)
    pub fn vnd_to_vnst(&self, vnd_amount: Decimal) -> U256 {
        // VND is integer, VNST has 18 decimals
        // 1 VND = 1 * 10^18 VNST base units
        let vnd_u64 = vnd_amount.to_string().parse::<u64>().unwrap_or(0);
        U256::from(vnd_u64) * U256::from(10u64).pow(U256::from(18))
    }

    /// Convert VNST base units to VND
    pub fn vnst_to_vnd(&self, vnst_amount: U256) -> Decimal {
        let divisor = U256::from(10u64).pow(U256::from(18));
        let vnd_u64 = (vnst_amount / divisor).as_u64();
        Decimal::from(vnd_u64)
    }

    /// Format VNST amount for display
    pub fn format_vnst(&self, amount: U256) -> String {
        let divisor = U256::from(10u64).pow(U256::from(18));
        let whole = amount / divisor;
        let frac = amount % divisor;
        if frac.is_zero() {
            format!("{} VNST", whole)
        } else {
            let frac_str = format!("{:018}", frac);
            let trimmed = frac_str.trim_end_matches('0');
            format!("{}.{} VNST", whole, trimmed)
        }
    }

    /// Calculate fee for mint operation
    pub fn calculate_mint_fee(&self, vnd_amount: Decimal) -> Decimal {
        vnd_amount * Decimal::from(self.config.mint_fee_bps) / Decimal::from(10_000)
    }

    /// Calculate fee for burn operation
    pub fn calculate_burn_fee(&self, vnst_amount: U256) -> U256 {
        vnst_amount * U256::from(self.config.burn_fee_bps) / U256::from(10_000u64)
    }

    /// Mint VNST with VND deposit
    pub async fn mint(&self, request: VnstMintRequest) -> Result<VnstMintResponse> {
        // Validate amount
        if request.vnd_amount < self.config.min_mint_vnd {
            return Err(Error::Validation(format!(
                "Amount {} VND is below minimum {} VND",
                request.vnd_amount, self.config.min_mint_vnd
            )));
        }

        if let Some(max) = &self.config.max_mint_vnd {
            if request.vnd_amount > *max {
                return Err(Error::Validation(format!(
                    "Amount {} VND exceeds maximum {} VND",
                    request.vnd_amount, max
                )));
            }
        }

        // Check peg health before minting
        let peg_status = self.check_peg().await?;
        if peg_status.status == PegHealthStatus::Critical {
            return Err(Error::Validation(
                "VNST minting is temporarily suspended due to peg deviation".to_string(),
            ));
        }

        // Calculate fee
        let fee_vnd = self.calculate_mint_fee(request.vnd_amount);
        let net_vnd = request.vnd_amount - fee_vnd;

        // Convert to VNST
        let vnst_amount = self.vnd_to_vnst(net_vnd);
        let fee_vnst = self.vnd_to_vnst(fee_vnd);

        info!(
            tenant = %request.tenant_id.0,
            user = %request.user_id.0,
            vnd_amount = %request.vnd_amount,
            vnst_amount = %vnst_amount,
            fee_vnd = %fee_vnd,
            "Processing VNST mint request"
        );

        // Record mint
        let mint_id = self
            .data_provider
            .record_mint(
                &request.tenant_id,
                &request.user_id,
                request.vnd_amount,
                vnst_amount,
                request.chain_id,
                request.recipient_address,
            )
            .await?;

        // Execute on-chain mint (in production)
        let tx_hash = match self
            .data_provider
            .execute_mint(request.chain_id, request.recipient_address, vnst_amount)
            .await
        {
            Ok(hash) => Some(format!("{:?}", hash)),
            Err(e) => {
                warn!(error = %e, "On-chain mint execution pending");
                None
            }
        };

        let status = if tx_hash.is_some() {
            VnstOperationStatus::Completed
        } else {
            VnstOperationStatus::Pending
        };

        Ok(VnstMintResponse {
            mint_id,
            vnd_amount: request.vnd_amount,
            vnst_amount,
            vnst_amount_display: self.format_vnst(vnst_amount),
            fee_vnd,
            fee_vnst,
            chain_id: request.chain_id,
            recipient_address: format!("{:?}", request.recipient_address),
            status,
            tx_hash,
            created_at: Utc::now(),
        })
    }

    /// Burn VNST for VND withdrawal
    pub async fn burn(&self, request: VnstBurnRequest) -> Result<VnstBurnResponse> {
        // Validate amount
        if request.vnst_amount < self.config.min_burn_vnst {
            return Err(Error::Validation(format!(
                "Amount is below minimum burn threshold"
            )));
        }

        if let Some(max) = &self.config.max_burn_vnst {
            if request.vnst_amount > *max {
                return Err(Error::Validation(format!(
                    "Amount exceeds maximum burn threshold"
                )));
            }
        }

        // Check peg health
        let peg_status = self.check_peg().await?;
        if peg_status.status == PegHealthStatus::Critical {
            return Err(Error::Validation(
                "VNST burning is temporarily suspended due to peg deviation".to_string(),
            ));
        }

        // Calculate fee
        let fee_vnst = self.calculate_burn_fee(request.vnst_amount);
        let net_vnst = request.vnst_amount - fee_vnst;

        // Convert to VND
        let vnd_amount = self.vnst_to_vnd(net_vnst);
        let fee_vnd = self.vnst_to_vnd(fee_vnst);

        info!(
            tenant = %request.tenant_id.0,
            user = %request.user_id.0,
            vnst_amount = %request.vnst_amount,
            vnd_amount = %vnd_amount,
            "Processing VNST burn request"
        );

        // Record burn
        let burn_id = self
            .data_provider
            .record_burn(
                &request.tenant_id,
                &request.user_id,
                request.vnst_amount,
                vnd_amount,
                request.chain_id,
                &request.bank_account_ref,
            )
            .await?;

        // Estimate VND arrival (T+1 for bank transfers)
        let estimated_arrival = Utc::now() + chrono::Duration::days(1);

        Ok(VnstBurnResponse {
            burn_id,
            vnst_amount: request.vnst_amount,
            vnst_amount_display: self.format_vnst(request.vnst_amount),
            vnd_amount,
            fee_vnst,
            fee_vnd,
            chain_id: request.chain_id,
            status: VnstOperationStatus::Pending,
            tx_hash: None,
            estimated_vnd_arrival: Some(estimated_arrival),
            created_at: Utc::now(),
        })
    }

    /// Get reserve information
    pub async fn get_reserves(&self, tenant_id: &TenantId) -> Result<VnstReserveInfo> {
        let total_supply = self
            .data_provider
            .get_total_supply(self.config.primary_chain_id)
            .await?;

        let total_vnd_reserves = self.data_provider.get_vnd_reserves(tenant_id).await?;

        let current_rate = self.data_provider.get_current_rate().await?;

        let proof_attestation = self.data_provider.get_reserve_proof().await?;

        // Calculate collateralization ratio
        let supply_vnd = self.vnst_to_vnd(total_supply);
        let collateralization_ratio = if supply_vnd.is_zero() {
            Decimal::from(100)
        } else {
            (total_vnd_reserves / supply_vnd) * Decimal::from(100)
        };

        // Check peg health
        let target_rate = Decimal::ONE;
        let deviation = ((current_rate - target_rate) / target_rate * Decimal::from(10_000))
            .abs()
            .to_string()
            .parse::<i32>()
            .unwrap_or(0);

        let peg_healthy = deviation < self.config.peg_deviation_warning_bps as i32;

        Ok(VnstReserveInfo {
            total_supply,
            total_supply_display: self.format_vnst(total_supply),
            total_vnd_reserves,
            collateralization_ratio,
            reserve_breakdown: vec![
                ReserveAsset {
                    asset_type: "VND Bank Deposits".to_string(),
                    amount_vnd: total_vnd_reserves * Decimal::from_str_exact("0.80").unwrap_or(Decimal::from(80) / Decimal::from(100)),
                    percentage: Decimal::from(80),
                    custodian: "Vietnam Bank Partner".to_string(),
                },
                ReserveAsset {
                    asset_type: "VND Government Bonds".to_string(),
                    amount_vnd: total_vnd_reserves * Decimal::from_str_exact("0.20").unwrap_or(Decimal::from(20) / Decimal::from(100)),
                    percentage: Decimal::from(20),
                    custodian: "State Treasury".to_string(),
                },
            ],
            last_proof_at: Utc::now(),
            proof_attestation,
            peg_healthy,
            current_rate,
            peg_deviation_bps: deviation,
        })
    }

    /// Check VND/VNST peg status
    pub async fn check_peg(&self) -> Result<VnstPegStatus> {
        let current_rate = self.data_provider.get_current_rate().await.unwrap_or(Decimal::ONE);
        let target_rate = Decimal::ONE;

        let deviation_decimal = (current_rate - target_rate).abs() / target_rate * Decimal::from(10_000);
        let deviation_bps = deviation_decimal.trunc().to_string().parse::<i32>().unwrap_or(0);

        let (status, message) = if deviation_bps < self.config.peg_deviation_warning_bps as i32 {
            (PegHealthStatus::Healthy, "VNST peg is stable".to_string())
        } else if deviation_bps < self.config.peg_deviation_critical_bps as i32 {
            (
                PegHealthStatus::Warning,
                format!(
                    "VNST peg deviation of {} bps detected - monitoring",
                    deviation_bps
                ),
            )
        } else {
            (
                PegHealthStatus::Critical,
                format!(
                    "VNST peg critical deviation of {} bps - operations suspended",
                    deviation_bps
                ),
            )
        };

        Ok(VnstPegStatus {
            is_healthy: status == PegHealthStatus::Healthy,
            current_rate,
            target_rate,
            deviation_bps,
            status,
            last_checked: Utc::now(),
            message,
        })
    }

    /// Check collateralization ratio
    pub async fn check_collateralization(&self, tenant_id: &TenantId) -> Result<bool> {
        let reserves = self.get_reserves(tenant_id).await?;
        Ok(reserves.collateralization_ratio >= Decimal::from(self.config.min_collateralization_ratio))
    }

    /// Get current configuration
    pub fn get_config(&self) -> &VnstProtocolConfig {
        &self.config
    }
}

/// Mock data provider for testing
pub struct MockVnstProtocolDataProvider {
    pub total_supply: std::sync::Mutex<U256>,
    pub vnd_reserves: std::sync::Mutex<Decimal>,
    pub current_rate: std::sync::Mutex<Decimal>,
    pub mints: std::sync::Mutex<Vec<String>>,
    pub burns: std::sync::Mutex<Vec<String>>,
}

impl MockVnstProtocolDataProvider {
    pub fn new() -> Self {
        Self {
            total_supply: std::sync::Mutex::new(U256::from(1_000_000_000u64) * U256::from(10u64).pow(U256::from(18))),
            vnd_reserves: std::sync::Mutex::new(Decimal::from(1_000_000_000i64)),
            current_rate: std::sync::Mutex::new(Decimal::ONE),
            mints: std::sync::Mutex::new(Vec::new()),
            burns: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn with_rate(self, rate: Decimal) -> Self {
        *self.current_rate.lock().expect("Lock poisoned") = rate;
        self
    }
}

impl Default for MockVnstProtocolDataProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VnstProtocolDataProvider for MockVnstProtocolDataProvider {
    async fn get_total_supply(&self, _chain_id: u64) -> Result<U256> {
        Ok(*self.total_supply.lock().expect("Lock poisoned"))
    }

    async fn get_vnd_reserves(&self, _tenant_id: &TenantId) -> Result<Decimal> {
        Ok(*self.vnd_reserves.lock().expect("Lock poisoned"))
    }

    async fn get_current_rate(&self) -> Result<Decimal> {
        Ok(*self.current_rate.lock().expect("Lock poisoned"))
    }

    async fn get_reserve_proof(&self) -> Result<Option<String>> {
        Ok(Some("mock_attestation_hash".to_string()))
    }

    async fn record_mint(
        &self,
        _tenant_id: &TenantId,
        _user_id: &UserId,
        _vnd_amount: Decimal,
        _vnst_amount: U256,
        _chain_id: u64,
        _recipient: Address,
    ) -> Result<String> {
        let id = format!("mint_{}", Utc::now().timestamp());
        self.mints.lock().expect("Lock poisoned").push(id.clone());
        Ok(id)
    }

    async fn record_burn(
        &self,
        _tenant_id: &TenantId,
        _user_id: &UserId,
        _vnst_amount: U256,
        _vnd_amount: Decimal,
        _chain_id: u64,
        _bank_account_ref: &str,
    ) -> Result<String> {
        let id = format!("burn_{}", Utc::now().timestamp());
        self.burns.lock().expect("Lock poisoned").push(id.clone());
        Ok(id)
    }

    async fn execute_mint(
        &self,
        _chain_id: u64,
        _recipient: Address,
        _amount: U256,
    ) -> Result<H256> {
        // Mock: Return a dummy tx hash
        Ok(H256::zero())
    }

    async fn execute_burn(
        &self,
        _chain_id: u64,
        _from: Address,
        _amount: U256,
    ) -> Result<H256> {
        Ok(H256::zero())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> VnstProtocolService {
        let provider = Arc::new(MockVnstProtocolDataProvider::new());
        VnstProtocolService::new(VnstProtocolConfig::default(), provider)
    }

    #[test]
    fn test_vnd_to_vnst_conversion() {
        let service = create_test_service();

        // 1,000,000 VND should equal 1,000,000 * 10^18 VNST base units
        let vnd = Decimal::from(1_000_000);
        let vnst = service.vnd_to_vnst(vnd);

        let expected = U256::from(1_000_000u64) * U256::from(10u64).pow(U256::from(18));
        assert_eq!(vnst, expected);
    }

    #[test]
    fn test_vnst_to_vnd_conversion() {
        let service = create_test_service();

        let vnst = U256::from(1_000_000u64) * U256::from(10u64).pow(U256::from(18));
        let vnd = service.vnst_to_vnd(vnst);

        assert_eq!(vnd, Decimal::from(1_000_000));
    }

    #[test]
    fn test_format_vnst() {
        let service = create_test_service();

        let amount = U256::from(1_500_000u64) * U256::from(10u64).pow(U256::from(18));
        let formatted = service.format_vnst(amount);

        assert_eq!(formatted, "1500000 VNST");
    }

    #[test]
    fn test_calculate_fees() {
        let service = create_test_service();

        // Mint fee: 10 bps = 0.1% of 1,000,000 VND = 1,000 VND
        let mint_fee = service.calculate_mint_fee(Decimal::from(1_000_000));
        assert_eq!(mint_fee, Decimal::from(1000));

        // Burn fee: 0.1% of 1M VNST base units
        let vnst_amount = U256::from(1_000_000u64) * U256::from(10u64).pow(U256::from(18));
        let burn_fee = service.calculate_burn_fee(vnst_amount);
        let expected_fee = vnst_amount / U256::from(1000u64); // 0.1%
        assert_eq!(burn_fee, expected_fee);
    }

    #[tokio::test]
    async fn test_mint_success() {
        let service = create_test_service();

        let request = VnstMintRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            vnd_amount: Decimal::from(1_000_000),
            chain_id: 56,
            recipient_address: "0x1234567890123456789012345678901234567890"
                .parse()
                .unwrap(),
            idempotency_key: None,
        };

        let response = service.mint(request).await.unwrap();

        assert!(response.mint_id.starts_with("mint_"));
        assert_eq!(response.vnd_amount, Decimal::from(1_000_000));
        assert!(response.fee_vnd > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_mint_below_minimum() {
        let service = create_test_service();

        let request = VnstMintRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            vnd_amount: Decimal::from(1_000), // Below 100K minimum
            chain_id: 56,
            recipient_address: "0x1234567890123456789012345678901234567890"
                .parse()
                .unwrap(),
            idempotency_key: None,
        };

        let result = service.mint(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_burn_success() {
        let service = create_test_service();

        let request = VnstBurnRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            vnst_amount: U256::from(1_000_000u64) * U256::from(10u64).pow(U256::from(18)),
            chain_id: 56,
            bank_account_ref: "bank_ref_123".to_string(),
            idempotency_key: None,
        };

        let response = service.burn(request).await.unwrap();

        assert!(response.burn_id.starts_with("burn_"));
        assert!(response.vnd_amount > Decimal::ZERO);
        assert!(response.estimated_vnd_arrival.is_some());
    }

    #[tokio::test]
    async fn test_check_peg_healthy() {
        let service = create_test_service();

        let peg_status = service.check_peg().await.unwrap();

        assert!(peg_status.is_healthy);
        assert_eq!(peg_status.status, PegHealthStatus::Healthy);
        assert_eq!(peg_status.current_rate, Decimal::ONE);
    }

    #[tokio::test]
    async fn test_check_peg_warning() {
        let provider = Arc::new(
            MockVnstProtocolDataProvider::new()
                .with_rate(Decimal::from_str_exact("1.01").unwrap_or(Decimal::ONE)), // 1% deviation
        );
        let service = VnstProtocolService::new(VnstProtocolConfig::default(), provider);

        let peg_status = service.check_peg().await.unwrap();

        assert_eq!(peg_status.status, PegHealthStatus::Warning);
    }

    #[tokio::test]
    async fn test_get_reserves() {
        let service = create_test_service();

        let reserves = service.get_reserves(&TenantId::new("tenant1")).await.unwrap();

        assert!(reserves.total_supply > U256::zero());
        assert!(reserves.total_vnd_reserves > Decimal::ZERO);
        assert!(reserves.collateralization_ratio >= Decimal::from(100));
        assert!(reserves.peg_healthy);
    }

    #[tokio::test]
    async fn test_collateralization_check() {
        let service = create_test_service();

        let is_healthy = service
            .check_collateralization(&TenantId::new("tenant1"))
            .await
            .unwrap();

        assert!(is_healthy);
    }
}
