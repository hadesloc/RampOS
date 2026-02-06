//! Yield Service - Auto-optimization and management
//!
//! Provides high-level yield management:
//! - Automatic protocol selection based on APY
//! - Position tracking across protocols
//! - Yield reporting and analytics
//! - Safety controls and emergency withdrawal

use chrono::{DateTime, Duration, Utc};
use ethers::types::{Address, H256, U256};
use ramp_common::{Error, Result};
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::{error, info, warn};

use super::{
    ProtocolId, ProtocolRegistry, YieldAllocationConfig, YieldOperation,
    YieldPosition, YieldPositionReport, YieldProtocol, YieldReport, YieldTransaction,
    YieldTxStatus,
};

/// Yield service for managing stablecoin yield across protocols
pub struct YieldService {
    registry: ProtocolRegistry,
    config: YieldAllocationConfig,
    positions: RwLock<Vec<YieldPosition>>,
    transactions: RwLock<Vec<YieldTransaction>>,
    // Track total allocation per protocol
    allocations: RwLock<HashMap<ProtocolId, U256>>,
}

impl YieldService {
    pub fn new(registry: ProtocolRegistry, config: YieldAllocationConfig) -> Self {
        Self {
            registry,
            config,
            positions: RwLock::new(Vec::new()),
            transactions: RwLock::new(Vec::new()),
            allocations: RwLock::new(HashMap::new()),
        }
    }

    /// Get the protocol registry
    pub fn registry(&self) -> &ProtocolRegistry {
        &self.registry
    }

    /// Get current allocations per protocol
    pub fn get_allocations(&self) -> Result<HashMap<ProtocolId, U256>> {
        let allocations = self.allocations.read().map_err(|_| Error::Internal("Lock poisoned".to_string()))?;
        Ok(allocations.clone())
    }

    /// Deposit tokens to the best yield protocol
    pub async fn deposit_to_best_protocol(
        &self,
        token: Address,
        amount: U256,
    ) -> Result<(ProtocolId, H256)> {
        // Find best protocol by APY
        let best_protocol = self
            .registry
            .best_for_token(token)
            .await?
            .ok_or_else(|| Error::NotFound(format!("No protocol supports token {:?}", token)))?;

        let protocol_id = best_protocol.protocol_id();

        // Check allocation limits
        self.check_allocation_limit(protocol_id, amount)?;

        // Get current APY for position tracking
        let apy = best_protocol.current_apy(token).await?;

        info!(
            protocol = %protocol_id,
            token = ?token,
            amount = %amount,
            apy = apy,
            "Depositing to best yield protocol"
        );

        // Execute deposit
        let tx_hash = best_protocol.deposit(token, amount).await?;

        // Track position
        self.add_position(protocol_id, token, amount, apy)?;

        // Record transaction
        self.record_transaction(tx_hash, protocol_id, token, YieldOperation::Deposit, amount)?;

        // Update allocation
        self.update_allocation(protocol_id, amount, true)?;

        Ok((protocol_id, tx_hash))
    }

    /// Deposit to a specific protocol
    pub async fn deposit_to_protocol(
        &self,
        protocol_id: ProtocolId,
        token: Address,
        amount: U256,
    ) -> Result<H256> {
        let protocol = self
            .registry
            .get(protocol_id)
            .ok_or_else(|| Error::NotFound(format!("Protocol not found: {}", protocol_id)))?;

        if !protocol.supports_token(token) {
            return Err(Error::Business(format!("Token not supported: {:?}", token)));
        }

        // Check allocation limits
        self.check_allocation_limit(protocol_id, amount)?;

        let apy = protocol.current_apy(token).await?;

        info!(
            protocol = %protocol_id,
            token = ?token,
            amount = %amount,
            "Depositing to specified protocol"
        );

        let tx_hash = protocol.deposit(token, amount).await?;

        self.add_position(protocol_id, token, amount, apy)?;
        self.record_transaction(tx_hash, protocol_id, token, YieldOperation::Deposit, amount)?;
        self.update_allocation(protocol_id, amount, true)?;

        Ok(tx_hash)
    }

    /// Withdraw from a specific protocol
    pub async fn withdraw_from_protocol(
        &self,
        protocol_id: ProtocolId,
        token: Address,
        amount: U256,
    ) -> Result<H256> {
        let protocol = self
            .registry
            .get(protocol_id)
            .ok_or_else(|| Error::NotFound(format!("Protocol not found: {}", protocol_id)))?;

        let balance = protocol.balance(token).await?;
        if balance < amount {
            return Err(Error::Business(format!(
                "Insufficient balance: {} < {}",
                balance,
                amount
            )));
        }

        info!(
            protocol = %protocol_id,
            token = ?token,
            amount = %amount,
            "Withdrawing from protocol"
        );

        let tx_hash = protocol.withdraw(token, amount).await?;

        self.record_transaction(tx_hash, protocol_id, token, YieldOperation::Withdraw, amount)?;
        self.update_allocation(protocol_id, amount, false)?;

        Ok(tx_hash)
    }

    /// Emergency withdraw all funds from a protocol
    pub async fn emergency_withdraw(
        &self,
        protocol_id: ProtocolId,
        token: Address,
    ) -> Result<H256> {
        let protocol = self
            .registry
            .get(protocol_id)
            .ok_or_else(|| Error::NotFound(format!("Protocol not found: {}", protocol_id)))?;

        let balance = protocol.balance(token).await?;
        if balance.is_zero() {
            return Err(Error::Business("No balance to withdraw".to_string()));
        }

        warn!(
            protocol = %protocol_id,
            token = ?token,
            amount = %balance,
            "Emergency withdrawal initiated"
        );

        let tx_hash = protocol.withdraw(token, balance).await?;

        self.record_transaction(
            tx_hash,
            protocol_id,
            token,
            YieldOperation::EmergencyWithdraw,
            balance,
        )?;
        self.update_allocation(protocol_id, balance, false)?;

        Ok(tx_hash)
    }

    /// Rebalance funds to the best protocol
    pub async fn rebalance(&self, token: Address) -> Result<Vec<H256>> {
        let mut transactions = Vec::new();

        // Get current APYs for all protocols
        let mut protocol_apys: Vec<(ProtocolId, f64, U256)> = Vec::new();

        for protocol in self.registry.all() {
            if !protocol.supports_token(token) {
                continue;
            }

            let apy = protocol.current_apy(token).await.unwrap_or(0.0);
            let balance = protocol.balance(token).await.unwrap_or_default();

            protocol_apys.push((protocol.protocol_id(), apy, balance));
        }

        if protocol_apys.len() < 2 {
            return Ok(transactions);
        }

        // Sort by APY descending
        protocol_apys.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let (best_protocol_id, best_apy, _) = protocol_apys[0];
        let apy_threshold = 0.5; // Rebalance if APY difference > 0.5%

        // Move funds from lower APY protocols to best
        for (protocol_id, apy, balance) in protocol_apys.iter().skip(1) {
            if balance.is_zero() {
                continue;
            }

            let apy_diff = best_apy - apy;
            if apy_diff < apy_threshold {
                continue;
            }

            info!(
                from = %protocol_id,
                to = %best_protocol_id,
                amount = %balance,
                apy_gain = apy_diff,
                "Rebalancing funds"
            );

            // Withdraw from lower APY protocol
            let withdraw_tx = self.withdraw_from_protocol(*protocol_id, token, *balance).await?;
            transactions.push(withdraw_tx);

            // Deposit to best protocol
            let deposit_tx = self
                .deposit_to_protocol(best_protocol_id, token, *balance)
                .await?;
            transactions.push(deposit_tx);
        }

        Ok(transactions)
    }

    /// Reallocate funds according to target distribution
    pub async fn reallocate(
        &self,
        token: Address,
        targets: Vec<(ProtocolId, U256)>,
    ) -> Result<Vec<H256>> {
        let mut tx_hashes = Vec::new();
        let mut current_balances = HashMap::new();

        // 1. Get current balances
        for protocol in self.registry.all() {
            if protocol.supports_token(token) {
                let bal = protocol.balance(token).await?;
                current_balances.insert(protocol.protocol_id(), bal);
            }
        }

        let mut target_map = HashMap::new();
        for (pid, amt) in targets {
            target_map.insert(pid, amt);
        }

        // 2. Execute Withdrawals (for protocols where current > target)
        for (pid, current) in &current_balances {
            let target = target_map.get(pid).copied().unwrap_or(U256::zero());

            if *current > target {
                let withdraw_amount = *current - target;
                info!(
                    protocol = %pid,
                    token = ?token,
                    amount = %withdraw_amount,
                    "Reallocating: Withdrawing excess"
                );

                let tx = self.withdraw_from_protocol(*pid, token, withdraw_amount).await?;
                tx_hashes.push(tx);
            }
        }

        // 3. Execute Deposits (for protocols where target > current)
        for (pid, target) in &target_map {
            let current = current_balances.get(pid).copied().unwrap_or(U256::zero());

            if *target > current {
                let deposit_amount = *target - current;
                info!(
                    protocol = %pid,
                    token = ?token,
                    amount = %deposit_amount,
                    "Reallocating: Depositing shortfall"
                );

                let tx = self.deposit_to_protocol(*pid, token, deposit_amount).await?;
                tx_hashes.push(tx);
            }
        }

        Ok(tx_hashes)
    }

    /// Claim all pending rewards
    pub async fn claim_all_rewards(&self) -> Result<Vec<H256>> {
        let mut tx_hashes = Vec::new();

        for protocol in self.registry.all() {
            match protocol.claim_rewards().await {
                Ok(Some(tx)) => {
                    info!(
                        protocol = protocol.name(),
                        tx_hash = ?tx,
                        "Claimed rewards"
                    );
                    tx_hashes.push(tx);
                }
                Ok(None) => {
                    // No rewards to claim
                }
                Err(e) => {
                    warn!(
                        protocol = protocol.name(),
                        error = %e,
                        "Failed to claim rewards"
                    );
                }
            }
        }

        Ok(tx_hashes)
    }

    /// Monitor health factors and trigger emergency withdrawal if needed
    pub async fn monitor_health(&self) -> Result<Vec<H256>> {
        let mut emergency_txs = Vec::new();

        for protocol in self.registry.all() {
            let health_factor = protocol.health_factor().await?;

            if health_factor < self.config.min_health_factor {
                error!(
                    protocol = protocol.name(),
                    health_factor = health_factor,
                    min_required = self.config.min_health_factor,
                    "Health factor too low! Initiating emergency withdrawal"
                );

                // Get all tokens and emergency withdraw
                for token in self.get_protocol_tokens(protocol.protocol_id()) {
                    if let Ok(tx) = self
                        .emergency_withdraw(protocol.protocol_id(), token)
                        .await
                    {
                        emergency_txs.push(tx);
                    }
                }
            }
        }

        Ok(emergency_txs)
    }

    /// Generate yield report for a time period
    pub async fn generate_report(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<YieldReport> {
        let positions = self.positions.read().map_err(|_| Error::Internal("Lock poisoned".to_string()))?;
        let transactions = self.transactions.read().map_err(|_| Error::Internal("Lock poisoned".to_string()))?;

        let mut total_deposited = U256::zero();
        let mut total_withdrawn = U256::zero();
        let mut total_yield_earned = U256::zero();
        let mut position_reports = Vec::new();

        // Calculate totals from transactions
        for tx in transactions.iter() {
            if tx.timestamp < start || tx.timestamp > end {
                continue;
            }

            match tx.operation {
                YieldOperation::Deposit => {
                    total_deposited = total_deposited.saturating_add(tx.amount);
                }
                YieldOperation::Withdraw | YieldOperation::EmergencyWithdraw => {
                    total_withdrawn = total_withdrawn.saturating_add(tx.amount);
                }
                YieldOperation::ClaimRewards => {}
            }
        }

        // Calculate yield per position
        for position in positions.iter() {
            if position.created_at > end {
                continue;
            }

            let yield_earned = position.accrued_yield;
            total_yield_earned = total_yield_earned.saturating_add(yield_earned);

            position_reports.push(YieldPositionReport {
                protocol: position.protocol,
                token: position.token,
                principal: position.principal,
                yield_earned,
                apy: position.apy_at_deposit,
            });
        }

        // Calculate average APY
        let average_apy = if !position_reports.is_empty() {
            position_reports.iter().map(|p| p.apy).sum::<f64>() / position_reports.len() as f64
        } else {
            0.0
        };

        Ok(YieldReport {
            period_start: start,
            period_end: end,
            total_deposited,
            total_withdrawn,
            total_yield_earned,
            average_apy,
            positions: position_reports,
        })
    }

    /// Get all positions
    pub fn get_positions(&self) -> Result<Vec<YieldPosition>> {
        let positions = self.positions.read().map_err(|_| Error::Internal("Lock poisoned".to_string()))?;
        Ok(positions.clone())
    }

    /// Get total balance across all protocols for a token
    pub async fn total_balance(&self, token: Address) -> Result<U256> {
        let mut total = U256::zero();

        for protocol in self.registry.all() {
            if protocol.supports_token(token) {
                let balance = protocol.balance(token).await?;
                total = total.saturating_add(balance);
            }
        }

        Ok(total)
    }

    /// Get APY comparison across protocols
    pub async fn compare_apys(&self, token: Address) -> Result<Vec<(ProtocolId, f64)>> {
        let mut apys = Vec::new();

        for protocol in self.registry.all() {
            if protocol.supports_token(token) {
                if let Ok(apy) = protocol.current_apy(token).await {
                    apys.push((protocol.protocol_id(), apy));
                }
            }
        }

        apys.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(apys)
    }

    // Private helper methods

    fn check_allocation_limit(&self, protocol_id: ProtocolId, amount: U256) -> Result<()> {
        let allocations = self.allocations.read().map_err(|_| Error::Internal("Lock poisoned".to_string()))?;
        let current = allocations.get(&protocol_id).copied().unwrap_or_default();
        let new_total = current.saturating_add(amount);

        if new_total > self.config.max_per_protocol {
            return Err(Error::Business(format!(
                "Allocation limit exceeded: max {}, requested {}",
                self.config.max_per_protocol, new_total
            )));
        }

        Ok(())
    }

    fn add_position(
        &self,
        protocol: ProtocolId,
        token: Address,
        amount: U256,
        apy: f64,
    ) -> Result<()> {
        let mut positions = self.positions.write().map_err(|_| Error::Internal("Lock poisoned".to_string()))?;
        positions.push(YieldPosition::new(protocol, token, amount, apy));
        Ok(())
    }

    fn record_transaction(
        &self,
        tx_hash: H256,
        protocol: ProtocolId,
        token: Address,
        operation: YieldOperation,
        amount: U256,
    ) -> Result<()> {
        let mut transactions = self.transactions.write().map_err(|_| Error::Internal("Lock poisoned".to_string()))?;
        transactions.push(YieldTransaction {
            id: uuid::Uuid::new_v4().to_string(),
            tx_hash,
            protocol,
            token,
            operation,
            amount,
            timestamp: Utc::now(),
            status: YieldTxStatus::Pending,
        });
        Ok(())
    }

    fn update_allocation(&self, protocol: ProtocolId, amount: U256, is_deposit: bool) -> Result<()> {
        let mut allocations = self.allocations.write().map_err(|_| Error::Internal("Lock poisoned".to_string()))?;
        let current = allocations.entry(protocol).or_insert(U256::zero());

        if is_deposit {
            *current = current.saturating_add(amount);
        } else {
            *current = current.saturating_sub(amount);
        }

        Ok(())
    }

    fn get_protocol_tokens(&self, _protocol_id: ProtocolId) -> Vec<Address> {
        // In production, would track which tokens are deposited in which protocol
        // For now, return common stablecoins
        vec![
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                .parse()
                .unwrap(), // USDC
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yield_service_creation() {
        let registry = ProtocolRegistry::new();
        let config = YieldAllocationConfig::default();
        let service = YieldService::new(registry, config);

        assert!(service.get_positions().unwrap().is_empty());
    }

    #[test]
    fn test_allocation_config_default() {
        let config = YieldAllocationConfig::default();
        assert_eq!(config.max_allocation_percent, 80);
        assert_eq!(config.min_health_factor, 1.5);
        assert!(config.enabled_protocols.contains(&ProtocolId::AaveV3));
        assert!(config.enabled_protocols.contains(&ProtocolId::CompoundV3));
    }
}
