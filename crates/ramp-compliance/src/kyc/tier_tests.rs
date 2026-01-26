#[cfg(test)]
mod tier_tests {
    use crate::kyc::tier::{TierDataProvider, TierManager, UserKycInfo}; // Use absolute path if needed or correct relative path
    use crate::types::{KycStatus, KycTier};
    use async_trait::async_trait;
    use ramp_common::{
        types::{TenantId, UserId},
        Result,
    };

    struct MockTierDataProvider {
        info: UserKycInfo,
    }

    #[async_trait]
    impl TierDataProvider for MockTierDataProvider {
        async fn get_user_kyc_info(
            &self,
            _tenant_id: &TenantId,
            _user_id: &UserId,
        ) -> Result<UserKycInfo> {
            Ok(self.info.clone())
        }

        async fn update_tier_and_limits(
            &self,
            _tenant_id: &TenantId,
            _user_id: &UserId,
            _tier: KycTier,
        ) -> Result<()> {
            Ok(())
        }

        async fn emit_tier_change_event(
            &self,
            _tenant_id: &TenantId,
            _user_id: &UserId,
            _old_tier: KycTier,
            _new_tier: KycTier,
            _reason: Option<String>,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_cannot_upgrade_to_same_tier() {
        let provider = MockTierDataProvider {
            info: UserKycInfo {
                current_tier: KycTier::Tier1,
                kyc_status: KycStatus::Approved,
                verified_documents: vec!["ID_FRONT".to_string()],
            },
        };
        let manager = TierManager::new(Box::new(provider));
        let tenant_id = TenantId::new(uuid::Uuid::new_v4().to_string());
        let user_id = UserId::new(uuid::Uuid::new_v4().to_string());

        let result = manager
            .upgrade_tier(&tenant_id, &user_id, KycTier::Tier1)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_upgrade_tier1_to_tier2() {
        let provider = MockTierDataProvider {
            info: UserKycInfo {
                current_tier: KycTier::Tier1,
                kyc_status: KycStatus::Approved,
                verified_documents: vec!["ID_FRONT".to_string(), "PROOF_OF_ADDRESS".to_string()],
            },
        };
        let manager = TierManager::new(Box::new(provider));
        let tenant_id = TenantId::new(uuid::Uuid::new_v4().to_string());
        let user_id = UserId::new(uuid::Uuid::new_v4().to_string());

        let can_upgrade = manager
            .can_upgrade(&tenant_id, &user_id, KycTier::Tier2)
            .await
            .unwrap();
        assert!(can_upgrade);

        let result = manager
            .upgrade_tier(&tenant_id, &user_id, KycTier::Tier2)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_upgrade_fails_missing_docs() {
        let provider = MockTierDataProvider {
            info: UserKycInfo {
                current_tier: KycTier::Tier1,
                kyc_status: KycStatus::Approved,
                verified_documents: vec!["ID_FRONT".to_string()], // Missing proof of address
            },
        };
        let manager = TierManager::new(Box::new(provider));
        let tenant_id = TenantId::new(uuid::Uuid::new_v4().to_string());
        let user_id = UserId::new(uuid::Uuid::new_v4().to_string());

        let can_upgrade = manager
            .can_upgrade(&tenant_id, &user_id, KycTier::Tier2)
            .await
            .unwrap();
        assert!(!can_upgrade);

        let result = manager
            .upgrade_tier(&tenant_id, &user_id, KycTier::Tier2)
            .await;
        assert!(result.is_err());
    }
}
