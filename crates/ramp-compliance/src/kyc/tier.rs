use crate::types::{KycStatus, KycTier};
use async_trait::async_trait;
use ramp_common::{
    types::{TenantId, UserId},
    Error, Result,
};
use tracing::info;

/// Data provider for TierManager
#[async_trait]
pub trait TierDataProvider: Send + Sync {
    async fn get_user_kyc_info(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<UserKycInfo>;
    async fn update_tier_and_limits(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        tier: KycTier,
    ) -> Result<()>;
    async fn emit_tier_change_event(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        old_tier: KycTier,
        new_tier: KycTier,
        reason: Option<String>,
    ) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct UserKycInfo {
    pub current_tier: KycTier,
    pub kyc_status: KycStatus,
    pub verified_documents: Vec<String>, // List of verified document types
}

/// Manages KYC tier upgrades and downgrades
pub struct TierManager {
    provider: Box<dyn TierDataProvider>,
}

impl TierManager {
    pub fn new(provider: Box<dyn TierDataProvider>) -> Self {
        Self { provider }
    }

    /// Check if a user can upgrade to a target tier
    pub async fn can_upgrade(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        target_tier: KycTier,
    ) -> Result<bool> {
        let info = self.provider.get_user_kyc_info(tenant_id, user_id).await?;
        Ok(self.check_upgrade_logic(&info, target_tier))
    }

    fn check_upgrade_logic(&self, info: &UserKycInfo, target_tier: KycTier) -> bool {
        // Cannot upgrade to same or lower tier (use downgrade_tier for that)
        if target_tier <= info.current_tier {
            return false;
        }

        // Must be in approved status generally
        if info.kyc_status != KycStatus::Approved {
             return false;
        }

        match target_tier {
            KycTier::Tier0 => false,
            KycTier::Tier1 => {
                // Tier0 -> Tier1: requires basic KYC
                info.verified_documents
                    .iter()
                    .any(|d| d == "ID_FRONT" || d == "PASSPORT" || d == "CCCD")
            }
            KycTier::Tier2 => {
                // Tier1 -> Tier2: requires enhanced KYC + address verification
                info.current_tier >= KycTier::Tier1
                    && info
                        .verified_documents
                        .contains(&"PROOF_OF_ADDRESS".to_string())
            }
            KycTier::Tier3 => {
                // Tier2 -> Tier3: requires enhanced KYC + source of funds
                info.current_tier >= KycTier::Tier2
                    && info
                        .verified_documents
                        .contains(&"SOURCE_OF_FUNDS".to_string())
            }
        }
    }

    /// Upgrade user to a new tier
    pub async fn upgrade_tier(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        new_tier: KycTier,
    ) -> Result<()> {
        let info = self.provider.get_user_kyc_info(tenant_id, user_id).await?;
        let old_tier = info.current_tier;

        if !self.check_upgrade_logic(&info, new_tier) {
            return Err(Error::Business(format!(
                "User {} cannot be upgraded from {:?} to {:?}",
                user_id, old_tier, new_tier
            )));
        }

        info!(
            user_id = %user_id,
            old_tier = ?old_tier,
            new_tier = ?new_tier,
            "Upgrading user tier"
        );

        self.provider
            .update_tier_and_limits(tenant_id, user_id, new_tier)
            .await?;
        self.provider
            .emit_tier_change_event(tenant_id, user_id, old_tier, new_tier, None)
            .await?;

        Ok(())
    }

    /// Downgrade user tier
    pub async fn downgrade_tier(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        new_tier: KycTier,
        reason: &str,
    ) -> Result<()> {
        let info = self.provider.get_user_kyc_info(tenant_id, user_id).await?;
        let old_tier = info.current_tier;

        if new_tier >= old_tier {
            return Err(Error::Business(format!(
                "Target tier {:?} must be lower than current tier {:?} for downgrade",
                new_tier, old_tier
            )));
        }

        info!(
            user_id = %user_id,
            old_tier = ?old_tier,
            new_tier = ?new_tier,
            reason = %reason,
            "Downgrading user tier"
        );

        self.provider
            .update_tier_and_limits(tenant_id, user_id, new_tier)
            .await?;
        self.provider
            .emit_tier_change_event(
                tenant_id,
                user_id,
                old_tier,
                new_tier,
                Some(reason.to_string()),
            )
            .await?;

        Ok(())
    }
    pub async fn get_tier_limits(&self, tier: KycTier) -> TierLimits {
        TierLimits {
            daily_payin: tier.daily_payin_limit_vnd(),
            daily_payout: tier.daily_payout_limit_vnd(),
            single_transaction: tier.single_transaction_limit_vnd(),
        }
    }
}

pub struct TierLimits {
    pub daily_payin: rust_decimal::Decimal,
    pub daily_payout: rust_decimal::Decimal,
    pub single_transaction: rust_decimal::Decimal,
}

/// Check if a transaction is compliant with tier limits
pub fn check_tier_compliance(
    tier: KycTier,
    amount: rust_decimal::Decimal,
    _tx_type: &str,
) -> Result<()> {
    let single_limit = tier.single_transaction_limit_vnd();
    if !single_limit.is_zero() && amount > single_limit {
        return Err(Error::UserLimitExceeded {
            limit_type: format!("Single transaction limit for tier {:?}", tier),
        });
    }
    // Note: Daily limits need to be checked against aggregated history, which is not available here.
    // This function only checks static single transaction limits.
    Ok(())
}

#[cfg(test)]
#[path = "tier_tests.rs"]
mod tests;
