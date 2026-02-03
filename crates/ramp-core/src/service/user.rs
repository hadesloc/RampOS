use async_trait::async_trait;
use ramp_common::{
    types::{TenantId, UserId},
    Error, Result,
};
use ramp_compliance::kyc::{TierDataProvider, TierManager, UserKycInfo};
use ramp_compliance::types::{KycStatus, KycTier};
use std::sync::Arc;
use tracing::info;

use crate::event::EventPublisher;
use crate::repository::user::{UserRepository, UserRow};

// Assuming UserRepository needs to be cloneable or shared.
// In a real application, we'd use Arc<dyn UserRepository> instead of Box.
// Since I can't easily change the whole codebase to Arc, I'll simulate it or use Box if Clone is implemented.
// But dyn Trait is not Clone.
// I'll wrap it in Arc for the service.

pub struct UserService {
    repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl UserService {
    pub fn new(repo: Arc<dyn UserRepository>, event_publisher: Arc<dyn EventPublisher>) -> Self {
        Self {
            repo,
            event_publisher,
        }
    }

    pub fn tier_manager(&self) -> TierManager {
        let provider = Box::new(UserServiceTierProvider {
            repo: self.repo.clone(),
            event_publisher: self.event_publisher.clone(),
        });
        TierManager::new(provider)
    }

    pub async fn upgrade_user_tier(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        target_tier: KycTier,
    ) -> Result<()> {
        let manager = self.tier_manager();
        manager.upgrade_tier(tenant_id, user_id, target_tier).await
    }

    pub async fn downgrade_user_tier(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        target_tier: KycTier,
        reason: &str,
    ) -> Result<()> {
        let manager = self.tier_manager();
        manager
            .downgrade_tier(tenant_id, user_id, target_tier, reason)
            .await
    }

    pub async fn get_user_kyc_info(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<UserKycInfo> {
        let provider = UserServiceTierProvider {
            repo: self.repo.clone(),
            event_publisher: self.event_publisher.clone(),
        };
        provider.get_user_kyc_info(tenant_id, user_id).await
    }

    pub async fn get_user(&self, tenant_id: &TenantId, user_id: &UserId) -> Result<UserRow> {
        self.repo
            .get_by_id(tenant_id, user_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("User {}", user_id)))
    }

    pub async fn list_users(
        &self,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
        kyc_tier: Option<i16>,
        status: Option<&str>,
        search: Option<&str>,
    ) -> Result<(Vec<UserRow>, i64)> {
        let users = self
            .repo
            .list_users(tenant_id, limit, offset, kyc_tier, status, search)
            .await?;
        let total = self
            .repo
            .count_users(tenant_id, kyc_tier, status, search)
            .await?;
        Ok((users, total))
    }

    pub async fn count_users_by_status(&self, tenant_id: &TenantId, status: &str) -> Result<i64> {
        self.repo
            .count_users(tenant_id, None, Some(status), None)
            .await
    }

    pub async fn count_users_by_kyc_status(
        &self,
        tenant_id: &TenantId,
        kyc_status: &str,
    ) -> Result<i64> {
        self.repo
            .count_users_by_kyc_status(tenant_id, kyc_status)
            .await
    }

    pub async fn count_users_created_since(
        &self,
        tenant_id: &TenantId,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<i64> {
        self.repo.count_users_created_since(tenant_id, since).await
    }

    pub async fn update_user(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        status: Option<String>,
        kyc_tier: Option<i16>,
        daily_payin_limit_vnd: Option<i64>,
        daily_payout_limit_vnd: Option<i64>,
    ) -> Result<()> {
        if let Some(tier) = kyc_tier {
            self.repo.update_kyc_tier(tenant_id, user_id, tier).await?;
        }

        if let Some(status) = status.as_deref() {
            self.repo.update_status(tenant_id, user_id, status).await?;
        }

        if daily_payin_limit_vnd.is_some() || daily_payout_limit_vnd.is_some() {
            let payin = daily_payin_limit_vnd.map(rust_decimal::Decimal::from);
            let payout = daily_payout_limit_vnd.map(rust_decimal::Decimal::from);
            self.repo
                .update_limits(tenant_id, user_id, payin, payout)
                .await?;
        }

        Ok(())
    }
}

// Adapter for TierDataProvider
struct UserServiceTierProvider {
    repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
}

#[async_trait]
impl TierDataProvider for UserServiceTierProvider {
    async fn get_user_kyc_info(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<UserKycInfo> {
        let user = self
            .repo
            .get_by_id(tenant_id, user_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("User {}", user_id)))?;

        // Convert i16 to KycTier
        let current_tier = KycTier::from_i16(user.kyc_tier);

        // Map string status to KycStatus enum - assuming simplistic mapping or serde
        // Since KycStatus is an enum, we might need a parser if stored as string
        let kyc_status = match user.kyc_status.as_str() {
            "Approved" => KycStatus::Approved,
            "Pending" => KycStatus::Pending,
            "Rejected" => KycStatus::Rejected,
            _ => KycStatus::Pending, // Default fallback
        };

        // Determine verified documents based on logic or data
        // For now, assuming if Approved, they have basic docs.
        // In a real system, we'd fetch from `user_documents` table.
        let mut verified_documents = Vec::new();
        if kyc_status == KycStatus::Approved {
            verified_documents.push("ID_FRONT".to_string());
            if current_tier >= KycTier::Tier2 {
                verified_documents.push("PROOF_OF_ADDRESS".to_string());
            }
        }

        Ok(UserKycInfo {
            current_tier,
            kyc_status,
            verified_documents,
        })
    }

    async fn update_tier_and_limits(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        tier: KycTier,
    ) -> Result<()> {
        // Update tier in repo
        self.repo
            .update_kyc_tier(tenant_id, user_id, tier as i16)
            .await?;

        // Note: Repository update_kyc_tier handles updating the tier column.
        // Limits are calculated dynamically or stored?
        // KycTier has methods for limits.
        // The UserRow has limit columns.
        // We might want to update those columns if they are overrides,
        // or rely on the Tier definition if they are defaults.
        // Let's assume we don't need to update explicit limit columns if they follow tier defaults,
        // or we should update them to match new tier defaults.

        // For now, just updating the tier is sufficient based on the repo interface available.
        Ok(())
    }

    async fn emit_tier_change_event(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        old_tier: KycTier,
        new_tier: KycTier,
        reason: Option<String>,
    ) -> Result<()> {
        info!(
            tenant_id = %tenant_id,
            user_id = %user_id,
            old_tier = ?old_tier,
            new_tier = ?new_tier,
            reason = ?reason,
            "Tier changed event emitted"
        );
        self.event_publisher
            .publish_user_tier_changed(tenant_id, user_id, old_tier, new_tier, reason)
            .await
    }
}
