//! Yield Strategy Automation Module
//!
//! Provides automated yield strategies with different risk profiles:
//! - Conservative: Low risk, stable yield, prioritize safety
//! - Balanced: Mixed approach with moderate risk/reward
//! - Aggressive: Higher yield potential, accepts more risk
//!
//! Includes auto-rebalancing based on APY changes and risk controls.

use alloy::primitives::{Address, B256, U256};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use ramp_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::{ProtocolId, ProtocolRegistry, YieldService};

/// Strategy identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StrategyId {
    Conservative,
    Balanced,
    Aggressive,
    Custom(u32),
}

impl std::fmt::Display for StrategyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyId::Conservative => write!(f, "conservative"),
            StrategyId::Balanced => write!(f, "balanced"),
            StrategyId::Aggressive => write!(f, "aggressive"),
            StrategyId::Custom(id) => write!(f, "custom-{}", id),
        }
    }
}

/// Risk level for strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Strategy configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Strategy identifier
    pub id: StrategyId,
    /// Human-readable name
    pub name: String,
    /// Description of the strategy
    pub description: String,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Maximum exposure per protocol (percentage 0-100)
    pub max_protocol_exposure: u8,
    /// Maximum exposure per token (percentage 0-100)
    pub max_token_exposure: u8,
    /// Minimum APY threshold to consider a protocol
    pub min_apy_threshold: f64,
    /// APY difference threshold for rebalancing (percentage)
    pub rebalance_apy_threshold: f64,
    /// Minimum health factor before emergency exit
    pub min_health_factor: f64,
    /// Allowed protocols for this strategy
    pub allowed_protocols: Vec<ProtocolId>,
    /// Rebalance check interval in seconds
    pub rebalance_interval_secs: u64,
    /// Consider gas costs when rebalancing
    pub gas_aware_rebalancing: bool,
    /// Minimum amount to rebalance (avoid dust)
    pub min_rebalance_amount: U256,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            id: StrategyId::Balanced,
            name: "Balanced Strategy".to_string(),
            description: "Mixed approach with moderate risk and reward".to_string(),
            risk_level: RiskLevel::Medium,
            max_protocol_exposure: 50,
            max_token_exposure: 60,
            min_apy_threshold: 1.0,
            rebalance_apy_threshold: 0.5,
            min_health_factor: 1.5,
            allowed_protocols: vec![ProtocolId::AaveV3, ProtocolId::CompoundV3],
            rebalance_interval_secs: 3600, // 1 hour
            gas_aware_rebalancing: true,
            min_rebalance_amount: U256::from(100) * U256::from(1_000_000u64), // 100 USDC
        }
    }
}

impl StrategyConfig {
    pub fn treasury_posture_label(&self) -> &'static str {
        match self.risk_level {
            RiskLevel::Low => "capital_preservation",
            RiskLevel::Medium => "balanced_yield",
            RiskLevel::High => "yield_seeking",
        }
    }

    pub fn max_protocol_exposure_ratio(&self) -> f64 {
        self.max_protocol_exposure as f64 / 100.0
    }
}

impl StrategyConfig {
    pub fn treasury_policy_hint(&self) -> String {
        format!(
            "{} strategy keeps protocol exposure under {}% and only recommends rebalancing when APY delta exceeds {:.1}%.",
            self.name, self.max_protocol_exposure, self.rebalance_apy_threshold
        )
    }
}

/// Yield strategy trait
#[async_trait]
pub trait YieldStrategy: Send + Sync {
    /// Get strategy configuration
    fn config(&self) -> &StrategyConfig;

    /// Get strategy ID
    fn id(&self) -> StrategyId {
        self.config().id
    }

    /// Evaluate if rebalancing is needed
    async fn should_rebalance(
        &self,
        registry: &ProtocolRegistry,
        token: Address,
        current_allocations: &HashMap<ProtocolId, U256>,
    ) -> Result<bool>;

    /// Calculate optimal allocation for a deposit
    async fn calculate_allocation(
        &self,
        registry: &ProtocolRegistry,
        token: Address,
        amount: U256,
        current_allocations: &HashMap<ProtocolId, U256>,
    ) -> Result<Vec<(ProtocolId, U256)>>;

    /// Execute rebalancing based on strategy rules
    async fn execute_rebalance(&self, service: &YieldService, token: Address) -> Result<Vec<B256>>;

    /// Check if emergency exit is needed
    async fn check_emergency_exit(&self, registry: &ProtocolRegistry) -> Result<Vec<ProtocolId>>;

    /// Estimate gas cost for rebalancing
    fn estimate_rebalance_gas(&self, num_operations: usize) -> U256;
}

/// Conservative strategy - prioritizes safety and stable returns
pub struct ConservativeStrategy {
    config: StrategyConfig,
}

impl ConservativeStrategy {
    pub fn new() -> Self {
        Self {
            config: StrategyConfig {
                id: StrategyId::Conservative,
                name: "Conservative Strategy".to_string(),
                description: "Low risk strategy prioritizing safety and stable yields. \
                    Limits exposure to well-established protocols only."
                    .to_string(),
                risk_level: RiskLevel::Low,
                max_protocol_exposure: 40, // Max 40% per protocol
                max_token_exposure: 50,
                min_apy_threshold: 2.0,       // Only accept 2%+ APY
                rebalance_apy_threshold: 1.0, // Higher threshold to reduce churn
                min_health_factor: 2.0,       // Higher safety margin
                allowed_protocols: vec![ProtocolId::AaveV3], // Only Aave (more established)
                rebalance_interval_secs: 86400, // Daily check
                gas_aware_rebalancing: true,
                min_rebalance_amount: U256::from(500) * U256::from(1_000_000u64), // 500 USDC min
            },
        }
    }
}

impl Default for ConservativeStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl YieldStrategy for ConservativeStrategy {
    fn config(&self) -> &StrategyConfig {
        &self.config
    }

    async fn should_rebalance(
        &self,
        registry: &ProtocolRegistry,
        token: Address,
        current_allocations: &HashMap<ProtocolId, U256>,
    ) -> Result<bool> {
        // Conservative: only rebalance if APY difference is significant
        let mut apys: Vec<(ProtocolId, f64)> = Vec::new();

        for protocol in registry.all() {
            if !self
                .config
                .allowed_protocols
                .contains(&protocol.protocol_id())
            {
                continue;
            }
            if !protocol.supports_token(token) {
                continue;
            }
            if let Ok(apy) = protocol.current_apy(token).await {
                apys.push((protocol.protocol_id(), apy));
            }
        }

        if apys.len() < 2 {
            return Ok(false);
        }

        apys.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let best_apy = apys[0].1;
        let current_apy = apys
            .iter()
            .find(|(id, _)| {
                current_allocations
                    .get(id)
                    .map(|a| !a.is_zero())
                    .unwrap_or(false)
            })
            .map(|(_, apy)| *apy)
            .unwrap_or(0.0);

        Ok(best_apy - current_apy > self.config.rebalance_apy_threshold)
    }

    async fn calculate_allocation(
        &self,
        registry: &ProtocolRegistry,
        token: Address,
        amount: U256,
        _current_allocations: &HashMap<ProtocolId, U256>,
    ) -> Result<Vec<(ProtocolId, U256)>> {
        // Conservative: put all in the best allowed protocol
        let mut best: Option<(ProtocolId, f64)> = None;

        for protocol in registry.all() {
            let pid = protocol.protocol_id();
            if !self.config.allowed_protocols.contains(&pid) {
                continue;
            }
            if !protocol.supports_token(token) {
                continue;
            }
            if let Ok(apy) = protocol.current_apy(token).await {
                if apy >= self.config.min_apy_threshold {
                    if best.is_none() || apy > best.as_ref().map(|(_, a)| *a).unwrap_or(0.0) {
                        best = Some((pid, apy));
                    }
                }
            } else {
                tracing::warn!(
                    protocol = %pid,
                    "Failed to fetch APY for allocation, skipping protocol"
                );
            }
        }

        match best {
            Some((protocol_id, _)) => Ok(vec![(protocol_id, amount)]),
            None => Ok(vec![]),
        }
    }

    async fn execute_rebalance(&self, service: &YieldService, token: Address) -> Result<Vec<B256>> {
        info!(
            strategy = %self.config.name,
            "Executing conservative rebalance"
        );

        // 1. Get total balance
        let total_balance = service.total_balance(token).await?;
        if total_balance.is_zero() {
            return Ok(vec![]);
        }

        // 2. Calculate target allocation
        let current_allocations = service.get_allocations().await?;
        let targets = self
            .calculate_allocation(
                service.registry(),
                token,
                total_balance,
                &current_allocations,
            )
            .await?;

        // 3. Execute reallocation
        service.reallocate(token, targets).await
    }

    async fn check_emergency_exit(&self, registry: &ProtocolRegistry) -> Result<Vec<ProtocolId>> {
        let mut exit_protocols = Vec::new();

        for protocol in registry.all() {
            if !self
                .config
                .allowed_protocols
                .contains(&protocol.protocol_id())
            {
                continue;
            }

            let health = protocol.health_factor().await?;
            if health < self.config.min_health_factor {
                warn!(
                    protocol = protocol.name(),
                    health_factor = health,
                    min_required = self.config.min_health_factor,
                    "Conservative strategy triggering emergency exit"
                );
                exit_protocols.push(protocol.protocol_id());
            }
        }

        Ok(exit_protocols)
    }

    fn estimate_rebalance_gas(&self, num_operations: usize) -> U256 {
        // Conservative estimate: 200k gas per operation
        U256::from(200_000) * U256::from(num_operations)
    }
}

/// Balanced strategy - moderate risk/reward balance
pub struct BalancedStrategy {
    config: StrategyConfig,
}

impl BalancedStrategy {
    pub fn new() -> Self {
        Self {
            config: StrategyConfig {
                id: StrategyId::Balanced,
                name: "Balanced Strategy".to_string(),
                description: "Moderate risk strategy balancing yield and safety. \
                    Diversifies across multiple protocols."
                    .to_string(),
                risk_level: RiskLevel::Medium,
                max_protocol_exposure: 50,
                max_token_exposure: 60,
                min_apy_threshold: 1.0,
                rebalance_apy_threshold: 0.5,
                min_health_factor: 1.5,
                allowed_protocols: vec![ProtocolId::AaveV3, ProtocolId::CompoundV3],
                rebalance_interval_secs: 3600, // Hourly
                gas_aware_rebalancing: true,
                min_rebalance_amount: U256::from(100) * U256::from(1_000_000u64),
            },
        }
    }
}

impl Default for BalancedStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl YieldStrategy for BalancedStrategy {
    fn config(&self) -> &StrategyConfig {
        &self.config
    }

    async fn should_rebalance(
        &self,
        registry: &ProtocolRegistry,
        token: Address,
        current_allocations: &HashMap<ProtocolId, U256>,
    ) -> Result<bool> {
        let mut apys: Vec<(ProtocolId, f64, U256)> = Vec::new();

        for protocol in registry.all() {
            if !self
                .config
                .allowed_protocols
                .contains(&protocol.protocol_id())
            {
                continue;
            }
            if !protocol.supports_token(token) {
                continue;
            }
            let apy = protocol.current_apy(token).await.unwrap_or(0.0);
            let allocation = current_allocations
                .get(&protocol.protocol_id())
                .copied()
                .unwrap_or_default();
            apys.push((protocol.protocol_id(), apy, allocation));
        }

        if apys.len() < 2 {
            return Ok(false);
        }

        apys.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Check if highest APY protocol has less allocation than it should
        let best = &apys[0];
        let total: U256 = apys
            .iter()
            .map(|(_, _, a)| *a)
            .fold(U256::ZERO, |acc, a| acc + a);

        if total.is_zero() {
            return Ok(false);
        }

        // If best protocol has less than 60% and APY diff > threshold, rebalance
        let best_alloc: u128 = best.2.try_into().unwrap_or(u128::MAX);
        let total_val: u128 = total.try_into().unwrap_or(u128::MAX);
        let best_allocation_pct = (best_alloc as f64 / total_val as f64) * 100.0;
        let apy_diff = best.1 - apys.last().map(|(_, a, _)| *a).unwrap_or(0.0);

        Ok(best_allocation_pct < 60.0 && apy_diff > self.config.rebalance_apy_threshold)
    }

    async fn calculate_allocation(
        &self,
        registry: &ProtocolRegistry,
        token: Address,
        amount: U256,
        _current_allocations: &HashMap<ProtocolId, U256>,
    ) -> Result<Vec<(ProtocolId, U256)>> {
        let mut protocol_apys: Vec<(ProtocolId, f64)> = Vec::new();

        for protocol in registry.all() {
            let pid = protocol.protocol_id();
            if !self.config.allowed_protocols.contains(&pid) {
                continue;
            }
            if !protocol.supports_token(token) {
                continue;
            }
            if let Ok(apy) = protocol.current_apy(token).await {
                if apy >= self.config.min_apy_threshold {
                    protocol_apys.push((pid, apy));
                }
            } else {
                tracing::warn!(
                    protocol = %pid,
                    "Failed to fetch APY for balanced allocation, skipping"
                );
            }
        }

        if protocol_apys.is_empty() {
            return Ok(vec![]);
        }

        protocol_apys.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Balanced: 60% to best, 40% to second best (if available)
        let mut allocations = Vec::new();

        if protocol_apys.len() >= 2 {
            let primary_amount = amount * U256::from(60) / U256::from(100);
            let secondary_amount = amount - primary_amount;

            allocations.push((protocol_apys[0].0, primary_amount));
            allocations.push((protocol_apys[1].0, secondary_amount));
        } else {
            allocations.push((protocol_apys[0].0, amount));
        }

        Ok(allocations)
    }

    async fn execute_rebalance(&self, service: &YieldService, token: Address) -> Result<Vec<B256>> {
        info!(
            strategy = %self.config.name,
            "Executing balanced rebalance"
        );

        let total_balance = service.total_balance(token).await?;
        if total_balance.is_zero() {
            return Ok(vec![]);
        }

        let current_allocations = service.get_allocations().await?;
        let targets = self
            .calculate_allocation(
                service.registry(),
                token,
                total_balance,
                &current_allocations,
            )
            .await?;

        service.reallocate(token, targets).await
    }

    async fn check_emergency_exit(&self, registry: &ProtocolRegistry) -> Result<Vec<ProtocolId>> {
        let mut exit_protocols = Vec::new();

        for protocol in registry.all() {
            if !self
                .config
                .allowed_protocols
                .contains(&protocol.protocol_id())
            {
                continue;
            }

            let health = protocol.health_factor().await?;
            if health < self.config.min_health_factor {
                exit_protocols.push(protocol.protocol_id());
            }
        }

        Ok(exit_protocols)
    }

    fn estimate_rebalance_gas(&self, num_operations: usize) -> U256 {
        U256::from(180_000) * U256::from(num_operations)
    }
}

/// Aggressive strategy - maximizes yield with higher risk tolerance
pub struct AggressiveStrategy {
    config: StrategyConfig,
}

impl AggressiveStrategy {
    pub fn new() -> Self {
        Self {
            config: StrategyConfig {
                id: StrategyId::Aggressive,
                name: "Aggressive Strategy".to_string(),
                description: "High yield strategy accepting more risk. \
                    Actively chases highest APY with frequent rebalancing."
                    .to_string(),
                risk_level: RiskLevel::High,
                max_protocol_exposure: 70, // Can concentrate more
                max_token_exposure: 80,
                min_apy_threshold: 0.5, // Accept lower APY if it's the best
                rebalance_apy_threshold: 0.3, // More aggressive rebalancing
                min_health_factor: 1.2, // Lower safety margin
                allowed_protocols: vec![ProtocolId::AaveV3, ProtocolId::CompoundV3],
                rebalance_interval_secs: 1800, // Every 30 minutes
                gas_aware_rebalancing: false,  // Chase yield regardless of gas
                min_rebalance_amount: U256::from(50) * U256::from(1_000_000u64), // Lower threshold
            },
        }
    }
}

impl Default for AggressiveStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl YieldStrategy for AggressiveStrategy {
    fn config(&self) -> &StrategyConfig {
        &self.config
    }

    async fn should_rebalance(
        &self,
        registry: &ProtocolRegistry,
        token: Address,
        current_allocations: &HashMap<ProtocolId, U256>,
    ) -> Result<bool> {
        // Aggressive: rebalance on any meaningful APY difference
        let mut best_apy = 0.0f64;
        let mut current_weighted_apy = 0.0f64;
        let mut total_allocation = U256::ZERO;

        for protocol in registry.all() {
            if !protocol.supports_token(token) {
                continue;
            }

            let apy = protocol.current_apy(token).await.unwrap_or(0.0);
            let allocation = current_allocations
                .get(&protocol.protocol_id())
                .copied()
                .unwrap_or_default();

            if apy > best_apy {
                best_apy = apy;
            }

            if !allocation.is_zero() {
                current_weighted_apy +=
                    apy * u128::try_from(allocation).unwrap_or(u128::MAX) as f64;
                total_allocation = total_allocation + allocation;
            }
        }

        if total_allocation.is_zero() {
            return Ok(false);
        }

        current_weighted_apy /= u128::try_from(total_allocation).unwrap_or(u128::MAX) as f64;

        Ok(best_apy - current_weighted_apy > self.config.rebalance_apy_threshold)
    }

    async fn calculate_allocation(
        &self,
        registry: &ProtocolRegistry,
        token: Address,
        amount: U256,
        _current_allocations: &HashMap<ProtocolId, U256>,
    ) -> Result<Vec<(ProtocolId, U256)>> {
        // Aggressive: put everything in the highest APY protocol
        let mut best: Option<(ProtocolId, f64)> = None;

        for protocol in registry.all() {
            let pid = protocol.protocol_id();
            if !protocol.supports_token(token) {
                continue;
            }
            if let Ok(apy) = protocol.current_apy(token).await {
                if best.is_none() || apy > best.as_ref().map(|(_, a)| *a).unwrap_or(0.0) {
                    best = Some((pid, apy));
                }
            } else {
                tracing::warn!(
                    protocol = %pid,
                    "Failed to fetch APY for aggressive allocation, skipping"
                );
            }
        }

        match best {
            Some((protocol_id, _)) => Ok(vec![(protocol_id, amount)]),
            None => Ok(vec![]),
        }
    }

    async fn execute_rebalance(&self, service: &YieldService, token: Address) -> Result<Vec<B256>> {
        info!(
            strategy = %self.config.name,
            "Executing aggressive rebalance"
        );

        let total_balance = service.total_balance(token).await?;
        if total_balance.is_zero() {
            return Ok(vec![]);
        }

        let current_allocations = service.get_allocations().await?;
        let targets = self
            .calculate_allocation(
                service.registry(),
                token,
                total_balance,
                &current_allocations,
            )
            .await?;

        service.reallocate(token, targets).await
    }

    async fn check_emergency_exit(&self, registry: &ProtocolRegistry) -> Result<Vec<ProtocolId>> {
        let mut exit_protocols = Vec::new();

        for protocol in registry.all() {
            let health = protocol.health_factor().await?;
            // Aggressive has lower threshold but still exits on danger
            if health < self.config.min_health_factor {
                exit_protocols.push(protocol.protocol_id());
            }
        }

        Ok(exit_protocols)
    }

    fn estimate_rebalance_gas(&self, num_operations: usize) -> U256 {
        U256::from(150_000) * U256::from(num_operations)
    }
}

/// Strategy manager for coordinating automated yield strategies
pub struct StrategyManager {
    strategies: HashMap<StrategyId, Box<dyn YieldStrategy>>,
    active_strategy: RwLock<Option<StrategyId>>,
    last_rebalance: RwLock<HashMap<Address, DateTime<Utc>>>,
}

impl StrategyManager {
    pub fn new() -> Self {
        let mut strategies: HashMap<StrategyId, Box<dyn YieldStrategy>> = HashMap::new();

        strategies.insert(
            StrategyId::Conservative,
            Box::new(ConservativeStrategy::new()),
        );
        strategies.insert(StrategyId::Balanced, Box::new(BalancedStrategy::new()));
        strategies.insert(StrategyId::Aggressive, Box::new(AggressiveStrategy::new()));

        Self {
            strategies,
            active_strategy: RwLock::new(None),
            last_rebalance: RwLock::new(HashMap::new()),
        }
    }

    /// Get all available strategies
    pub fn list_strategies(&self) -> Vec<&StrategyConfig> {
        self.strategies.values().map(|s| s.config()).collect()
    }

    /// Get a specific strategy
    pub fn get_strategy(&self, id: StrategyId) -> Option<&dyn YieldStrategy> {
        self.strategies.get(&id).map(|s| s.as_ref())
    }

    /// Activate a strategy
    pub async fn activate_strategy(&self, id: StrategyId) -> Result<()> {
        if !self.strategies.contains_key(&id) {
            return Err(Error::NotFound(format!("Strategy not found: {}", id)));
        }

        let mut active = self.active_strategy.write().await;
        *active = Some(id);

        info!(strategy = %id, "Strategy activated");
        Ok(())
    }

    /// Get active strategy
    pub async fn active_strategy(&self) -> Result<Option<StrategyId>> {
        let active = self.active_strategy.read().await;
        Ok(*active)
    }

    /// Deactivate current strategy
    pub async fn deactivate_strategy(&self) -> Result<()> {
        let mut active = self.active_strategy.write().await;
        *active = None;
        Ok(())
    }

    /// Check if rebalancing is due for a token
    pub async fn is_rebalance_due(&self, token: Address) -> Result<bool> {
        let active_id = {
            let active = self.active_strategy.read().await;
            match *active {
                Some(id) => id,
                None => return Ok(false),
            }
        };

        let strategy = self
            .strategies
            .get(&active_id)
            .ok_or_else(|| Error::NotFound("Strategy not found".to_string()))?;

        let last_rebalance = self.last_rebalance.read().await;

        let interval = Duration::seconds(strategy.config().rebalance_interval_secs as i64);

        match last_rebalance.get(&token) {
            Some(last) => Ok(Utc::now() - *last > interval),
            None => Ok(true), // Never rebalanced
        }
    }

    /// Record a rebalance
    pub async fn record_rebalance(&self, token: Address) -> Result<()> {
        let mut last_rebalance = self.last_rebalance.write().await;
        last_rebalance.insert(token, Utc::now());
        Ok(())
    }

    /// Run auto-rebalancing check for a token
    pub async fn auto_rebalance(
        &self,
        service: &YieldService,
        registry: &ProtocolRegistry,
        token: Address,
        current_allocations: &HashMap<ProtocolId, U256>,
    ) -> Result<Vec<B256>> {
        // Check if rebalancing is due
        if !self.is_rebalance_due(token).await? {
            return Ok(vec![]);
        }

        let active_id = {
            let active = self.active_strategy.read().await;
            match *active {
                Some(id) => id,
                None => return Ok(vec![]),
            }
        };

        let strategy = self
            .strategies
            .get(&active_id)
            .ok_or_else(|| Error::NotFound("Strategy not found".to_string()))?;

        // Check if rebalancing is needed
        if !strategy
            .should_rebalance(registry, token, current_allocations)
            .await?
        {
            return Ok(vec![]);
        }

        // Gas cost consideration
        if strategy.config().gas_aware_rebalancing {
            let estimated_gas = strategy.estimate_rebalance_gas(2); // Typical: withdraw + deposit
                                                                    // In production, compare gas cost vs expected yield gain
            info!(estimated_gas = %estimated_gas, "Gas-aware rebalancing check");
        }

        // Execute rebalance
        let txs = strategy.execute_rebalance(service, token).await?;

        // Record rebalance
        self.record_rebalance(token).await?;

        Ok(txs)
    }

    /// Check and execute emergency exits
    pub async fn check_emergency_exits(
        &self,
        service: &YieldService,
        registry: &ProtocolRegistry,
        tokens: &[Address],
    ) -> Result<Vec<B256>> {
        let active_id = {
            let active = self.active_strategy.read().await;
            match *active {
                Some(id) => id,
                None => return Ok(vec![]),
            }
        };

        let strategy = self
            .strategies
            .get(&active_id)
            .ok_or_else(|| Error::NotFound("Strategy not found".to_string()))?;

        let exit_protocols = strategy.check_emergency_exit(registry).await?;

        let mut txs = Vec::new();
        for protocol_id in exit_protocols {
            for token in tokens {
                match service.emergency_withdraw(protocol_id, *token).await {
                    Ok(tx) => txs.push(tx),
                    Err(e) => {
                        tracing::error!(
                            protocol = %protocol_id,
                            error = %e,
                            "Emergency withdraw failed for token"
                        );
                    }
                }
            }
        }

        Ok(txs)
    }
}

impl Default for StrategyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics for yield strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPerformance {
    pub strategy_id: StrategyId,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_yield_earned: U256,
    pub average_apy: f64,
    pub num_rebalances: u32,
    pub total_gas_used: U256,
    pub net_yield: U256, // yield - gas costs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_strategy_manager_creation() {
        let manager = StrategyManager::new();
        let strategies = manager.list_strategies();

        assert_eq!(strategies.len(), 3);
        assert!(manager.active_strategy().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_activate_strategy() {
        let manager = StrategyManager::new();

        manager
            .activate_strategy(StrategyId::Conservative)
            .await
            .unwrap();
        assert_eq!(
            manager.active_strategy().await.unwrap(),
            Some(StrategyId::Conservative)
        );

        manager
            .activate_strategy(StrategyId::Aggressive)
            .await
            .unwrap();
        assert_eq!(
            manager.active_strategy().await.unwrap(),
            Some(StrategyId::Aggressive)
        );
    }

    #[test]
    fn test_conservative_strategy_config() {
        let strategy = ConservativeStrategy::new();
        let config = strategy.config();

        assert_eq!(config.risk_level, RiskLevel::Low);
        assert_eq!(config.max_protocol_exposure, 40);
        assert_eq!(config.min_health_factor, 2.0);
    }

    #[test]
    fn test_aggressive_strategy_config() {
        let strategy = AggressiveStrategy::new();
        let config = strategy.config();

        assert_eq!(config.risk_level, RiskLevel::High);
        assert_eq!(config.max_protocol_exposure, 70);
        assert_eq!(config.rebalance_apy_threshold, 0.3);
    }

    #[test]
    fn test_balanced_strategy_config() {
        let strategy = BalancedStrategy::new();
        let config = strategy.config();

        assert_eq!(config.risk_level, RiskLevel::Medium);
        assert_eq!(config.max_protocol_exposure, 50);
        assert_eq!(config.allowed_protocols.len(), 2);
    }
}
