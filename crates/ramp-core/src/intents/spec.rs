//! Intent DSL - Declarative specification for chain-abstracted operations
//!
//! Provides a high-level DSL for expressing user intents across chains:
//! - What the user wants to achieve (Swap, Bridge, Send, Stake)
//! - Constraints (slippage, deadline, preferred chains)
//! - Automatic execution plan generation

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// High-level action the user wants to perform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentAction {
    /// Swap one asset for another (same or cross-chain)
    Swap,
    /// Bridge an asset from one chain to another
    Bridge,
    /// Send an asset to a recipient
    Send,
    /// Stake an asset in a protocol
    Stake,
}

impl fmt::Display for IntentAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Swap => write!(f, "swap"),
            Self::Bridge => write!(f, "bridge"),
            Self::Send => write!(f, "send"),
            Self::Stake => write!(f, "stake"),
        }
    }
}

/// Asset identifier (chain + token address or symbol)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetId {
    /// Chain ID where the asset lives
    pub chain_id: u64,
    /// Token symbol (e.g., "USDC", "ETH")
    pub symbol: String,
    /// Token contract address (None for native tokens)
    pub address: Option<String>,
    /// Token decimals
    pub decimals: u8,
}

impl AssetId {
    pub fn new(chain_id: u64, symbol: &str, address: Option<String>, decimals: u8) -> Self {
        Self {
            chain_id,
            symbol: symbol.to_string(),
            address,
            decimals,
        }
    }

    /// Create a native token asset (ETH, MATIC, etc.)
    pub fn native(chain_id: u64) -> Self {
        let symbol = match chain_id {
            1 | 42161 | 10 | 8453 => "ETH",
            137 => "MATIC",
            56 => "BNB",
            43114 => "AVAX",
            _ => "ETH",
        };
        Self::new(chain_id, symbol, None, 18)
    }

    /// Create a USDC asset on a specific chain
    pub fn usdc(chain_id: u64) -> Self {
        let address = match chain_id {
            1 => Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string()),
            42161 => Some("0xaf88d065e77c8cC2239327C5EDb3A432268e5831".to_string()),
            8453 => Some("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string()),
            10 => Some("0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85".to_string()),
            137 => Some("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359".to_string()),
            _ => None,
        };
        Self::new(chain_id, "USDC", address, 6)
    }

    /// Create a USDT asset on a specific chain
    pub fn usdt(chain_id: u64) -> Self {
        let address = match chain_id {
            1 => Some("0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string()),
            42161 => Some("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".to_string()),
            10 => Some("0x94b008aA00579c1307B0EF2c499aD98a8ce58e58".to_string()),
            137 => Some("0xc2132D05D31c914a87C6611C10748AEb04B58e8F".to_string()),
            _ => None,
        };
        Self::new(chain_id, "USDT", address, 6)
    }

    pub fn is_native(&self) -> bool {
        self.address.is_none()
    }

    pub fn is_same_chain(&self, other: &AssetId) -> bool {
        self.chain_id == other.chain_id
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@chain:{}", self.symbol, self.chain_id)
    }
}

/// Constraints on intent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentConstraints {
    /// Maximum allowed slippage in basis points (e.g., 50 = 0.5%)
    pub max_slippage_bps: u16,
    /// Deadline for execution (None = no deadline)
    pub deadline: Option<DateTime<Utc>>,
    /// Preferred chains for intermediate operations
    pub preferred_chains: Vec<u64>,
    /// Maximum total gas cost in USD
    pub max_gas_cost_usd: Option<Decimal>,
    /// Maximum number of execution steps
    pub max_steps: Option<u32>,
    /// Require MEV protection
    pub mev_protection: bool,
}

impl Default for IntentConstraints {
    fn default() -> Self {
        Self {
            max_slippage_bps: 50, // 0.5%
            deadline: None,
            preferred_chains: vec![],
            max_gas_cost_usd: None,
            max_steps: None,
            mev_protection: true,
        }
    }
}

impl IntentConstraints {
    pub fn with_slippage(mut self, bps: u16) -> Self {
        self.max_slippage_bps = bps;
        self
    }

    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn with_preferred_chains(mut self, chains: Vec<u64>) -> Self {
        self.preferred_chains = chains;
        self
    }

    pub fn with_max_gas_usd(mut self, max: Decimal) -> Self {
        self.max_gas_cost_usd = Some(max);
        self
    }

    pub fn with_max_steps(mut self, max: u32) -> Self {
        self.max_steps = Some(max);
        self
    }

    pub fn is_expired(&self) -> bool {
        self.deadline.map_or(false, |d| Utc::now() > d)
    }
}

/// Full intent specification - what the user wants to achieve
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentSpec {
    /// Unique intent ID
    pub id: String,
    /// Action to perform
    pub action: IntentAction,
    /// Source asset
    pub from_asset: AssetId,
    /// Destination asset (same as from_asset for Send)
    pub to_asset: AssetId,
    /// Amount in from_asset's smallest unit (as string to avoid precision loss)
    pub amount: String,
    /// Recipient address (for Send actions)
    pub recipient: Option<String>,
    /// Execution constraints
    pub constraints: IntentConstraints,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl IntentSpec {
    pub fn new(
        action: IntentAction,
        from_asset: AssetId,
        to_asset: AssetId,
        amount: &str,
    ) -> Self {
        Self {
            id: format!("intent_{}", Uuid::now_v7()),
            action,
            from_asset,
            to_asset,
            amount: amount.to_string(),
            recipient: None,
            constraints: IntentConstraints::default(),
            created_at: Utc::now(),
        }
    }

    pub fn with_recipient(mut self, recipient: &str) -> Self {
        self.recipient = Some(recipient.to_string());
        self
    }

    pub fn with_constraints(mut self, constraints: IntentConstraints) -> Self {
        self.constraints = constraints;
        self
    }

    /// Check if this intent requires cross-chain execution
    pub fn is_cross_chain(&self) -> bool {
        self.from_asset.chain_id != self.to_asset.chain_id
    }

    /// Check if this intent is expired
    pub fn is_expired(&self) -> bool {
        self.constraints.is_expired()
    }

    /// Validate the intent specification
    pub fn validate(&self) -> Result<(), String> {
        if self.amount.is_empty() || self.amount == "0" {
            return Err("Amount must be greater than zero".to_string());
        }

        // Parse amount to verify it is numeric
        if self.amount.parse::<u128>().is_err() {
            return Err(format!("Invalid amount: {}", self.amount));
        }

        if self.action == IntentAction::Send && self.recipient.is_none() {
            return Err("Send action requires a recipient".to_string());
        }

        if self.constraints.max_slippage_bps > 10000 {
            return Err("Slippage cannot exceed 100%".to_string());
        }

        if self.is_expired() {
            return Err("Intent has expired".to_string());
        }

        Ok(())
    }
}

impl fmt::Display for IntentSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Intent[{}]: {} {} {} -> {}",
            self.id, self.action, self.amount, self.from_asset, self.to_asset,
        )
    }
}

/// A single step in an execution plan
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStepKind {
    /// Approve a token for spending by a contract
    Approve {
        token: String,
        spender: String,
        chain_id: u64,
    },
    /// Swap tokens on a DEX
    Swap {
        from_token: String,
        to_token: String,
        chain_id: u64,
        aggregator: Option<String>,
    },
    /// Bridge tokens to another chain
    Bridge {
        token: String,
        from_chain: u64,
        to_chain: u64,
        bridge_provider: Option<String>,
    },
    /// Transfer tokens to a recipient
    Transfer {
        token: String,
        recipient: String,
        chain_id: u64,
    },
    /// Stake tokens in a protocol
    Stake {
        token: String,
        protocol: String,
        chain_id: u64,
    },
    /// Wait for a bridge transfer to complete
    WaitForBridge {
        bridge_provider: String,
        from_chain: u64,
        to_chain: u64,
    },
}

impl fmt::Display for ExecutionStepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Approve { token, chain_id, .. } => {
                write!(f, "Approve {} on chain {}", token, chain_id)
            }
            Self::Swap { from_token, to_token, chain_id, .. } => {
                write!(f, "Swap {} -> {} on chain {}", from_token, to_token, chain_id)
            }
            Self::Bridge { token, from_chain, to_chain, .. } => {
                write!(f, "Bridge {} from chain {} to chain {}", token, from_chain, to_chain)
            }
            Self::Transfer { token, chain_id, .. } => {
                write!(f, "Transfer {} on chain {}", token, chain_id)
            }
            Self::Stake { token, protocol, chain_id } => {
                write!(f, "Stake {} in {} on chain {}", token, protocol, chain_id)
            }
            Self::WaitForBridge { from_chain, to_chain, .. } => {
                write!(f, "Wait for bridge {} -> {}", from_chain, to_chain)
            }
        }
    }
}

/// Estimated cost of a single execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepEstimate {
    /// Estimated gas units
    pub gas_units: u64,
    /// Estimated gas cost in USD
    pub gas_cost_usd: Decimal,
    /// Estimated time in seconds
    pub estimated_time_secs: u64,
}

/// A step in an execution plan with its estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Step index (0-based)
    pub index: u32,
    /// Step description
    pub kind: ExecutionStepKind,
    /// Amount for this step (in the relevant token's smallest unit)
    pub amount: String,
    /// Estimated costs
    pub estimate: StepEstimate,
}

/// Full execution plan for an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Intent this plan was generated for
    pub intent_id: String,
    /// Ordered list of steps to execute
    pub steps: Vec<PlanStep>,
    /// Total estimated gas cost in USD
    pub total_gas_cost_usd: Decimal,
    /// Total estimated execution time in seconds
    pub total_estimated_time_secs: u64,
    /// Expected output amount (in to_asset's smallest unit)
    pub expected_output: String,
    /// Minimum output amount after slippage
    pub minimum_output: String,
    /// Plan creation timestamp
    pub created_at: DateTime<Utc>,
    /// Plan expiry timestamp
    pub expires_at: DateTime<Utc>,
}

impl ExecutionPlan {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Check if the plan satisfies the given constraints
    pub fn satisfies_constraints(&self, constraints: &IntentConstraints) -> Result<(), String> {
        if let Some(max_gas) = constraints.max_gas_cost_usd {
            if self.total_gas_cost_usd > max_gas {
                return Err(format!(
                    "Gas cost {} exceeds maximum {}",
                    self.total_gas_cost_usd, max_gas
                ));
            }
        }

        if let Some(max_steps) = constraints.max_steps {
            if self.steps.len() as u32 > max_steps {
                return Err(format!(
                    "Plan has {} steps, maximum allowed is {}",
                    self.steps.len(),
                    max_steps
                ));
            }
        }

        if constraints.is_expired() {
            return Err("Constraints deadline has passed".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_intent_action_display() {
        assert_eq!(IntentAction::Swap.to_string(), "swap");
        assert_eq!(IntentAction::Bridge.to_string(), "bridge");
        assert_eq!(IntentAction::Send.to_string(), "send");
        assert_eq!(IntentAction::Stake.to_string(), "stake");
    }

    #[test]
    fn test_asset_id_creation() {
        let usdc = AssetId::usdc(1);
        assert_eq!(usdc.symbol, "USDC");
        assert_eq!(usdc.chain_id, 1);
        assert_eq!(usdc.decimals, 6);
        assert!(!usdc.is_native());
        assert!(usdc.address.is_some());
    }

    #[test]
    fn test_asset_id_native() {
        let eth = AssetId::native(1);
        assert_eq!(eth.symbol, "ETH");
        assert!(eth.is_native());

        let matic = AssetId::native(137);
        assert_eq!(matic.symbol, "MATIC");
    }

    #[test]
    fn test_asset_id_same_chain() {
        let usdc_eth = AssetId::usdc(1);
        let usdt_eth = AssetId::usdt(1);
        let usdc_arb = AssetId::usdc(42161);

        assert!(usdc_eth.is_same_chain(&usdt_eth));
        assert!(!usdc_eth.is_same_chain(&usdc_arb));
    }

    #[test]
    fn test_intent_spec_creation() {
        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "1000000",
        );

        assert!(spec.id.starts_with("intent_"));
        assert_eq!(spec.action, IntentAction::Swap);
        assert_eq!(spec.amount, "1000000");
        assert!(!spec.is_cross_chain());
    }

    #[test]
    fn test_intent_spec_cross_chain() {
        let spec = IntentSpec::new(
            IntentAction::Bridge,
            AssetId::usdc(1),
            AssetId::usdc(42161),
            "1000000",
        );

        assert!(spec.is_cross_chain());
    }

    #[test]
    fn test_intent_spec_validation_valid() {
        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "1000000",
        );

        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_intent_spec_validation_zero_amount() {
        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "0",
        );

        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_intent_spec_validation_send_no_recipient() {
        let spec = IntentSpec::new(
            IntentAction::Send,
            AssetId::usdc(1),
            AssetId::usdc(1),
            "1000000",
        );

        let result = spec.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("recipient"));
    }

    #[test]
    fn test_intent_spec_validation_send_with_recipient() {
        let spec = IntentSpec::new(
            IntentAction::Send,
            AssetId::usdc(1),
            AssetId::usdc(1),
            "1000000",
        )
        .with_recipient("0x1234567890123456789012345678901234567890");

        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_intent_constraints_default() {
        let constraints = IntentConstraints::default();
        assert_eq!(constraints.max_slippage_bps, 50);
        assert!(constraints.deadline.is_none());
        assert!(constraints.preferred_chains.is_empty());
        assert!(constraints.mev_protection);
        assert!(!constraints.is_expired());
    }

    #[test]
    fn test_intent_constraints_expired() {
        let constraints = IntentConstraints::default()
            .with_deadline(Utc::now() - Duration::hours(1));

        assert!(constraints.is_expired());
    }

    #[test]
    fn test_intent_constraints_not_expired() {
        let constraints = IntentConstraints::default()
            .with_deadline(Utc::now() + Duration::hours(1));

        assert!(!constraints.is_expired());
    }

    #[test]
    fn test_intent_spec_expired() {
        let constraints = IntentConstraints::default()
            .with_deadline(Utc::now() - Duration::hours(1));

        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "1000000",
        )
        .with_constraints(constraints);

        assert!(spec.is_expired());
        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_execution_step_kind_display() {
        let approve = ExecutionStepKind::Approve {
            token: "USDC".to_string(),
            spender: "0xRouter".to_string(),
            chain_id: 1,
        };
        assert!(approve.to_string().contains("Approve"));
        assert!(approve.to_string().contains("USDC"));

        let bridge = ExecutionStepKind::Bridge {
            token: "USDC".to_string(),
            from_chain: 1,
            to_chain: 42161,
            bridge_provider: Some("Stargate".to_string()),
        };
        assert!(bridge.to_string().contains("Bridge"));
    }

    #[test]
    fn test_execution_plan_expired() {
        let plan = ExecutionPlan {
            intent_id: "test".to_string(),
            steps: vec![],
            total_gas_cost_usd: Decimal::ZERO,
            total_estimated_time_secs: 0,
            expected_output: "1000000".to_string(),
            minimum_output: "995000".to_string(),
            created_at: Utc::now() - Duration::hours(2),
            expires_at: Utc::now() - Duration::hours(1),
        };

        assert!(plan.is_expired());
    }

    #[test]
    fn test_execution_plan_satisfies_constraints() {
        let plan = ExecutionPlan {
            intent_id: "test".to_string(),
            steps: vec![],
            total_gas_cost_usd: Decimal::new(5, 0), // $5
            total_estimated_time_secs: 60,
            expected_output: "1000000".to_string(),
            minimum_output: "995000".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(1),
        };

        let constraints = IntentConstraints::default()
            .with_max_gas_usd(Decimal::new(10, 0)); // max $10

        assert!(plan.satisfies_constraints(&constraints).is_ok());
    }

    #[test]
    fn test_execution_plan_exceeds_gas_constraint() {
        let plan = ExecutionPlan {
            intent_id: "test".to_string(),
            steps: vec![],
            total_gas_cost_usd: Decimal::new(15, 0), // $15
            total_estimated_time_secs: 60,
            expected_output: "1000000".to_string(),
            minimum_output: "995000".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(1),
        };

        let constraints = IntentConstraints::default()
            .with_max_gas_usd(Decimal::new(10, 0)); // max $10

        assert!(plan.satisfies_constraints(&constraints).is_err());
    }

    #[test]
    fn test_asset_id_display() {
        let usdc = AssetId::usdc(1);
        assert_eq!(usdc.to_string(), "USDC@chain:1");
    }

    #[test]
    fn test_intent_spec_display() {
        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "1000000",
        );
        let display = spec.to_string();
        assert!(display.contains("swap"));
        assert!(display.contains("1000000"));
        assert!(display.contains("USDC"));
    }

    #[test]
    fn test_intent_spec_serialization() {
        let spec = IntentSpec::new(
            IntentAction::Bridge,
            AssetId::usdc(1),
            AssetId::usdc(42161),
            "5000000",
        );

        let json = serde_json::to_string(&spec).unwrap();
        let parsed: IntentSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.action, IntentAction::Bridge);
        assert_eq!(parsed.amount, "5000000");
        assert_eq!(parsed.from_asset.chain_id, 1);
        assert_eq!(parsed.to_asset.chain_id, 42161);
    }

    #[test]
    fn test_constraints_builder_chain() {
        let constraints = IntentConstraints::default()
            .with_slippage(100)
            .with_preferred_chains(vec![42161, 10])
            .with_max_steps(5);

        assert_eq!(constraints.max_slippage_bps, 100);
        assert_eq!(constraints.preferred_chains, vec![42161, 10]);
        assert_eq!(constraints.max_steps, Some(5));
    }

    #[test]
    fn test_intent_validation_invalid_amount() {
        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "not_a_number",
        );

        assert!(spec.validate().is_err());
    }

    #[test]
    fn test_intent_validation_excessive_slippage() {
        let constraints = IntentConstraints {
            max_slippage_bps: 15000, // 150% - invalid
            ..Default::default()
        };
        let spec = IntentSpec::new(
            IntentAction::Swap,
            AssetId::usdc(1),
            AssetId::usdt(1),
            "1000000",
        )
        .with_constraints(constraints);

        assert!(spec.validate().is_err());
    }
}
