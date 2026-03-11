//! Comprehensive tests for the ramp-aa (Account Abstraction) module.
//!
//! Covers:
//! - SmartAccount creation, address computation, salt generation
//! - UserOperation building, hashing, builder pattern
//! - Paymaster signature verification (ECDSA/secp256k1)
//! - Session key validation and policy engine
//! - GasEstimator calculations
//! - Multi-token paymaster, cross-chain paymaster
//! - EIP-7702 authorization, delegation, session management
//! - Types and serialization

use alloy::primitives::{Address, Bytes, U256};
use ramp_aa::*;
use ramp_common::types::{TenantId, UserId};
use std::collections::HashMap;
use std::sync::Arc;

// ==========================================================================
// Helper functions
// ==========================================================================

fn test_entry_point() -> Address {
    "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
        .parse()
        .unwrap()
}

fn test_factory_address() -> Address {
    "0xaabbccddaabbccddaabbccddaabbccddaabbccdd"
        .parse()
        .unwrap()
}

fn test_address_1() -> Address {
    Address::from([0x11u8; 20])
}

fn test_address_2() -> Address {
    Address::from([0x22u8; 20])
}

fn test_signing_key() -> Vec<u8> {
    vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
        0x1f, 0x20,
    ]
}

fn make_user_op() -> UserOperation {
    UserOperation::new(
        test_address_1(),
        U256::from(1),
        Bytes::from(vec![0x01, 0x02, 0x03]),
    )
}

fn make_smart_account() -> smart_account::SmartAccount {
    smart_account::SmartAccount {
        address: test_address_1(),
        owner: test_address_2(),
        account_type: SmartAccountType::SimpleAccount,
        is_deployed: false,
        nonce: U256::ZERO,
    }
}

// ==========================================================================
// SmartAccountService tests
// ==========================================================================

mod smart_account_tests {
    use super::*;

    #[tokio::test]
    async fn test_smart_account_service_creation() {
        let service = SmartAccountService::new(1, test_factory_address(), test_entry_point());
        let tenant_id = TenantId::new("tenant_1");
        let user_id = UserId::new("user_1");

        let account = service
            .get_or_create_account(&tenant_id, &user_id, test_address_1())
            .await
            .expect("should create account");

        assert_eq!(account.owner, test_address_1());
        assert_eq!(account.account_type, SmartAccountType::SimpleAccount);
        assert!(!account.is_deployed);
        assert_eq!(account.nonce, U256::ZERO);
    }

    #[tokio::test]
    async fn test_deterministic_address_computation() {
        let service = SmartAccountService::new(1, test_factory_address(), test_entry_point());
        let tenant_id = TenantId::new("tenant_1");
        let user_id = UserId::new("user_1");

        let account1 = service
            .get_or_create_account(&tenant_id, &user_id, test_address_1())
            .await
            .unwrap();
        let account2 = service
            .get_or_create_account(&tenant_id, &user_id, test_address_1())
            .await
            .unwrap();

        // Same inputs should produce same address
        assert_eq!(account1.address, account2.address);
    }

    #[tokio::test]
    async fn test_different_users_get_different_addresses() {
        let service = SmartAccountService::new(1, test_factory_address(), test_entry_point());
        let tenant_id = TenantId::new("tenant_1");
        let user1 = UserId::new("user_1");
        let user2 = UserId::new("user_2");

        let account1 = service
            .get_or_create_account(&tenant_id, &user1, test_address_1())
            .await
            .unwrap();
        let account2 = service
            .get_or_create_account(&tenant_id, &user2, test_address_1())
            .await
            .unwrap();

        assert_ne!(account1.address, account2.address);
    }

    #[tokio::test]
    async fn test_different_tenants_get_different_addresses() {
        let service = SmartAccountService::new(1, test_factory_address(), test_entry_point());
        let tenant1 = TenantId::new("tenant_1");
        let tenant2 = TenantId::new("tenant_2");
        let user = UserId::new("user_1");

        let account1 = service
            .get_or_create_account(&tenant1, &user, test_address_1())
            .await
            .unwrap();
        let account2 = service
            .get_or_create_account(&tenant2, &user, test_address_1())
            .await
            .unwrap();

        assert_ne!(account1.address, account2.address);
    }

    #[test]
    fn test_build_create_account_op() {
        let service = SmartAccountService::new(1, test_factory_address(), test_entry_point());
        let account = make_smart_account();

        let op = service
            .build_create_account_op(&account, test_address_2(), U256::from(42))
            .expect("should build create op");

        assert!(op.is_account_creation());
        assert!(!op.init_code.is_empty());
        assert_eq!(op.sender, account.address);
    }

    #[test]
    fn test_build_transfer_op() {
        let service = SmartAccountService::new(1, test_factory_address(), test_entry_point());
        let account = make_smart_account();
        let to = Address::from([0x33u8; 20]);
        let value = U256::from(1000);

        let op = service
            .build_transfer_op(&account, to, value, None)
            .expect("should build transfer op");

        assert!(!op.is_account_creation());
        assert_eq!(op.sender, account.address);
        // call_data should contain execute() selector
        assert!(op.call_data.len() > 4);
        assert_eq!(&op.call_data[0..4], &[0xb6, 0x1d, 0x27, 0xf6]);
    }

    #[test]
    fn test_build_transfer_op_with_data() {
        let service = SmartAccountService::new(1, test_factory_address(), test_entry_point());
        let account = make_smart_account();
        let to = Address::from([0x33u8; 20]);
        let extra_data = Bytes::from(vec![0xaa, 0xbb, 0xcc]);

        let op = service
            .build_transfer_op(&account, to, U256::ZERO, Some(extra_data))
            .expect("should build transfer op with data");

        assert!(op.call_data.len() > 4);
    }

    #[test]
    fn test_build_batch_op() {
        let service = SmartAccountService::new(1, test_factory_address(), test_entry_point());
        let account = make_smart_account();

        let calls = vec![
            (
                Address::from([0x33u8; 20]),
                U256::from(100),
                Bytes::default(),
            ),
            (
                Address::from([0x44u8; 20]),
                U256::from(200),
                Bytes::from(vec![0x01]),
            ),
        ];

        let op = service
            .build_batch_op(&account, calls)
            .expect("should build batch op");

        // Should contain executeBatch selector
        assert_eq!(&op.call_data[0..4], &[0x34, 0xfc, 0xd5, 0xbe]);
    }

    #[test]
    fn test_build_batch_op_empty_calls() {
        let service = SmartAccountService::new(1, test_factory_address(), test_entry_point());
        let account = make_smart_account();

        let op = service
            .build_batch_op(&account, vec![])
            .expect("should handle empty batch");

        assert_eq!(&op.call_data[0..4], &[0x34, 0xfc, 0xd5, 0xbe]);
    }
}

// ==========================================================================
// UserOperation tests
// ==========================================================================

mod user_operation_tests {
    use super::*;

    #[test]
    fn test_user_op_new_defaults() {
        let op = make_user_op();

        assert_eq!(op.sender, test_address_1());
        assert_eq!(op.nonce, U256::from(1));
        assert!(op.init_code.is_empty());
        assert_eq!(op.call_gas_limit, U256::from(100_000));
        assert_eq!(op.verification_gas_limit, U256::from(100_000));
        assert_eq!(op.pre_verification_gas, U256::from(21_000));
        assert_eq!(op.max_fee_per_gas, U256::from(1_000_000_000));
        assert_eq!(op.max_priority_fee_per_gas, U256::from(1_000_000_000));
        assert!(op.paymaster_and_data.is_empty());
        assert!(op.signature.is_empty());
    }

    #[test]
    fn test_user_op_is_not_account_creation() {
        let op = make_user_op();
        assert!(!op.is_account_creation());
    }

    #[test]
    fn test_user_op_is_account_creation() {
        let op = make_user_op().with_init_code(Bytes::from(vec![0x01, 0x02]));
        assert!(op.is_account_creation());
    }

    #[test]
    fn test_user_op_with_gas() {
        let op =
            make_user_op().with_gas(U256::from(200_000), U256::from(300_000), U256::from(50_000));

        assert_eq!(op.call_gas_limit, U256::from(200_000));
        assert_eq!(op.verification_gas_limit, U256::from(300_000));
        assert_eq!(op.pre_verification_gas, U256::from(50_000));
    }

    #[test]
    fn test_user_op_with_fees() {
        let op =
            make_user_op().with_fees(U256::from(50_000_000_000u64), U256::from(2_000_000_000u64));

        assert_eq!(op.max_fee_per_gas, U256::from(50_000_000_000u64));
        assert_eq!(op.max_priority_fee_per_gas, U256::from(2_000_000_000u64));
    }

    #[test]
    fn test_user_op_with_paymaster() {
        let paymaster_data = Bytes::from(vec![0xaa; 97]);
        let op = make_user_op().with_paymaster(paymaster_data.clone());

        assert_eq!(op.paymaster_and_data, paymaster_data);
    }

    #[test]
    fn test_user_op_with_signature() {
        let sig = Bytes::from(vec![0xbb; 65]);
        let op = make_user_op().with_signature(sig.clone());

        assert_eq!(op.signature, sig);
    }

    #[test]
    fn test_user_op_hash_deterministic() {
        let op = make_user_op();
        let entry_point = test_entry_point();

        let hash1 = op.hash(entry_point, 1);
        let hash2 = op.hash(entry_point, 1);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_user_op_hash_different_chain_ids() {
        let op = make_user_op();
        let entry_point = test_entry_point();

        let hash1 = op.hash(entry_point, 1);
        let hash2 = op.hash(entry_point, 137);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_user_op_hash_different_entry_points() {
        let op = make_user_op();

        let hash1 = op.hash(test_entry_point(), 1);
        let hash2 = op.hash(test_address_1(), 1);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_user_op_hash_different_nonces() {
        let entry_point = test_entry_point();

        let op1 = UserOperation::new(test_address_1(), U256::from(0), Bytes::default());
        let op2 = UserOperation::new(test_address_1(), U256::from(1), Bytes::default());

        assert_ne!(op1.hash(entry_point, 1), op2.hash(entry_point, 1));
    }

    #[test]
    fn test_user_op_builder_chain() {
        let op = UserOperation::new(test_address_1(), U256::ZERO, Bytes::default())
            .with_gas(U256::from(500_000), U256::from(200_000), U256::from(50_000))
            .with_fees(U256::from(30_000_000_000u64), U256::from(1_000_000_000u64))
            .with_paymaster(Bytes::from(vec![0x42; 32]))
            .with_signature(Bytes::from(vec![0xff; 65]))
            .with_init_code(Bytes::from(vec![0xab; 20]));

        assert_eq!(op.call_gas_limit, U256::from(500_000));
        assert_eq!(op.verification_gas_limit, U256::from(200_000));
        assert_eq!(op.pre_verification_gas, U256::from(50_000));
        assert_eq!(op.max_fee_per_gas, U256::from(30_000_000_000u64));
        assert_eq!(op.max_priority_fee_per_gas, U256::from(1_000_000_000u64));
        assert!(op.is_account_creation());
        assert_eq!(op.signature.len(), 65);
        assert_eq!(op.paymaster_and_data.len(), 32);
    }

    #[test]
    fn test_user_op_serialization() {
        let op = make_user_op();
        let json = serde_json::to_value(&op).expect("should serialize");

        assert!(json.get("sender").is_some());
        assert!(json.get("nonce").is_some());
        assert!(json.get("callData").is_some());
        assert!(json.get("callGasLimit").is_some());
        assert!(json.get("verificationGasLimit").is_some());
        assert!(json.get("preVerificationGas").is_some());
        assert!(json.get("maxFeePerGas").is_some());
        assert!(json.get("maxPriorityFeePerGas").is_some());
    }
}

// ==========================================================================
// GasEstimator tests
// ==========================================================================

mod gas_estimator_tests {
    use super::*;
    use async_trait::async_trait;
    use ramp_aa::gas::GasProvider;

    struct MockGasProvider;

    #[async_trait]
    impl GasProvider for MockGasProvider {
        async fn get_gas_price(&self) -> ramp_common::Result<U256> {
            Ok(U256::from(1_000_000_000u64)) // 1 gwei
        }

        async fn estimate_gas(
            &self,
            _from: Address,
            _to: Address,
            _data: Bytes,
        ) -> ramp_common::Result<U256> {
            Ok(U256::from(21000u64))
        }

        async fn estimate_eip1559_fees(&self) -> ramp_common::Result<(U256, U256)> {
            Ok((U256::from(20_000_000_000u64), U256::from(1_000_000_000u64)))
        }
    }

    fn make_estimator() -> GasEstimator<MockGasProvider> {
        GasEstimator::new(Arc::new(MockGasProvider), test_entry_point())
    }

    #[tokio::test]
    async fn test_estimate_user_op_gas() {
        let estimator = make_estimator();
        let op = make_user_op();

        let estimate = estimator
            .estimate_user_op_gas(&op)
            .await
            .expect("should estimate gas");

        assert!(estimate.pre_verification_gas > U256::ZERO);
        assert!(estimate.call_gas > U256::ZERO);
        assert!(estimate.verification_gas > U256::ZERO);
        assert!(estimate.max_fee_per_gas > U256::ZERO);
        assert!(estimate.max_priority_fee_per_gas > U256::ZERO);
    }

    #[tokio::test]
    async fn test_estimate_call_gas_with_calldata() {
        let estimator = make_estimator();

        let op_small = UserOperation::new(test_address_1(), U256::ZERO, Bytes::from(vec![1u8; 10]));
        let op_large =
            UserOperation::new(test_address_1(), U256::ZERO, Bytes::from(vec![1u8; 1000]));

        let gas_small = estimator.estimate_call_gas(&op_small).await.unwrap();
        let gas_large = estimator.estimate_call_gas(&op_large).await.unwrap();

        // Both return same mock value (21000 + 10% = 23100), so compare pre-verification gas instead
        // estimate_call_gas uses provider.estimate_gas which returns constant 21000
        assert_eq!(gas_small, gas_large);
    }

    #[tokio::test]
    async fn test_estimate_call_gas_mock_returns_constant() {
        let estimator = make_estimator();

        let op = UserOperation::new(test_address_1(), U256::ZERO, Bytes::from(vec![0xff; 100]));

        let gas = estimator.estimate_call_gas(&op).await.unwrap();
        // Mock returns 21000, + 10% buffer = 23100
        assert_eq!(gas, U256::from(23100u64));
    }

    #[tokio::test]
    async fn test_estimate_verification_gas_constant() {
        let estimator = make_estimator();
        let op = make_user_op();

        let gas = estimator.estimate_verification_gas(&op).await.unwrap();
        assert_eq!(gas, U256::from(100_000));
    }
}

// ==========================================================================
// PolicyEngine tests
// ==========================================================================

mod policy_tests {
    use super::*;
    use ramp_aa::policy::PolicyEngine;

    #[tokio::test]
    async fn test_validate_user_op_within_limits() {
        let engine = PolicyEngine::new(TenantId::new("test"));
        let op = make_user_op(); // call_gas_limit = 100_000

        let result = engine
            .validate_user_operation(&op)
            .await
            .expect("should validate");

        // Default PolicyResult.is_valid starts as true and stays true if no violations
        assert!(result.violations.is_empty());
    }

    #[tokio::test]
    async fn test_validate_user_op_gas_too_high() {
        let engine = PolicyEngine::new(TenantId::new("test"));
        let op = make_user_op().with_gas(
            U256::from(2_000_000), // Exceeds 1_000_000 limit
            U256::from(100_000),
            U256::from(21_000),
        );

        let result = engine
            .validate_user_operation(&op)
            .await
            .expect("should validate");

        assert!(!result.violations.is_empty());
        assert!(result.violations[0].contains("gas limit too high"));
    }

    #[tokio::test]
    async fn test_validate_session_key_valid() {
        let engine = PolicyEngine::new(TenantId::new("test"));

        let session = engine.create_session_key(
            test_address_1(),
            3600,
            vec![SessionPermission {
                target: test_address_2(),
                selector: [0xb6, 0x1d, 0x27, 0xf6],
                max_value: U256::from(1_000_000),
                rules: vec![],
            }],
        );

        let result = engine
            .validate_session_key(
                &session,
                test_address_2(),
                [0xb6, 0x1d, 0x27, 0xf6],
                U256::from(500),
            )
            .await
            .expect("should validate");

        assert!(result.violations.is_empty());
    }

    #[tokio::test]
    async fn test_validate_session_key_expired() {
        let engine = PolicyEngine::new(TenantId::new("test"));

        // Create session that already expired
        let session = SessionKey {
            key_address: test_address_1(),
            valid_after: 1000,
            valid_until: 2000, // In the past
            permissions: vec![],
        };

        let result = engine
            .validate_session_key(&session, test_address_2(), [0x00; 4], U256::ZERO)
            .await
            .expect("should validate");

        assert!(!result.violations.is_empty());
        assert!(result.violations.iter().any(|v| v.contains("expired")));
    }

    #[tokio::test]
    async fn test_validate_session_key_not_yet_valid() {
        let engine = PolicyEngine::new(TenantId::new("test"));

        let session = SessionKey {
            key_address: test_address_1(),
            valid_after: u64::MAX - 1000,
            valid_until: u64::MAX,
            permissions: vec![],
        };

        let result = engine
            .validate_session_key(&session, test_address_2(), [0x00; 4], U256::ZERO)
            .await
            .expect("should validate");

        assert!(!result.violations.is_empty());
        assert!(result
            .violations
            .iter()
            .any(|v| v.contains("not yet valid")));
    }

    #[tokio::test]
    async fn test_validate_session_key_no_permission() {
        let engine = PolicyEngine::new(TenantId::new("test"));

        let session = engine.create_session_key(
            test_address_1(),
            3600,
            vec![SessionPermission {
                target: test_address_2(),
                selector: [0xaa, 0xbb, 0xcc, 0xdd],
                max_value: U256::from(1_000),
                rules: vec![],
            }],
        );

        // Wrong selector
        let result = engine
            .validate_session_key(
                &session,
                test_address_2(),
                [0x11, 0x22, 0x33, 0x44], // Different selector
                U256::ZERO,
            )
            .await
            .expect("should validate");

        assert!(!result.violations.is_empty());
        assert!(result
            .violations
            .iter()
            .any(|v| v.contains("No permission")));
    }

    #[tokio::test]
    async fn test_validate_session_key_value_exceeds_limit() {
        let engine = PolicyEngine::new(TenantId::new("test"));

        let session = engine.create_session_key(
            test_address_1(),
            3600,
            vec![SessionPermission {
                target: test_address_2(),
                selector: [0xb6, 0x1d, 0x27, 0xf6],
                max_value: U256::from(1_000),
                rules: vec![],
            }],
        );

        let result = engine
            .validate_session_key(
                &session,
                test_address_2(),
                [0xb6, 0x1d, 0x27, 0xf6],
                U256::from(2_000), // Exceeds max
            )
            .await
            .expect("should validate");

        assert!(!result.violations.is_empty());
        assert!(result.violations.iter().any(|v| v.contains("exceeds")));
    }

    #[tokio::test]
    async fn test_validate_session_key_max_amount_rule() {
        let engine = PolicyEngine::new(TenantId::new("test"));

        let session = engine.create_session_key(
            test_address_1(),
            3600,
            vec![SessionPermission {
                target: test_address_2(),
                selector: [0xb6, 0x1d, 0x27, 0xf6],
                max_value: U256::from(10_000),
                rules: vec![PermissionRule::MaxAmount(U256::from(500))],
            }],
        );

        let result = engine
            .validate_session_key(
                &session,
                test_address_2(),
                [0xb6, 0x1d, 0x27, 0xf6],
                U256::from(1_000), // Exceeds MaxAmount rule
            )
            .await
            .expect("should validate");

        assert!(!result.violations.is_empty());
        assert!(result.violations.iter().any(|v| v.contains("rule limit")));
    }

    #[tokio::test]
    async fn test_validate_session_key_time_window_rule() {
        let engine = PolicyEngine::new(TenantId::new("test"));

        let session = engine.create_session_key(
            test_address_1(),
            3600,
            vec![SessionPermission {
                target: test_address_2(),
                selector: [0xb6, 0x1d, 0x27, 0xf6],
                max_value: U256::MAX,
                rules: vec![PermissionRule::TimeWindow {
                    start: 0,
                    end: 1, // Already ended
                }],
            }],
        );

        let result = engine
            .validate_session_key(
                &session,
                test_address_2(),
                [0xb6, 0x1d, 0x27, 0xf6],
                U256::ZERO,
            )
            .await
            .expect("should validate");

        assert!(!result.violations.is_empty());
        assert!(result.violations.iter().any(|v| v.contains("time window")));
    }

    #[test]
    fn test_create_session_key() {
        let engine = PolicyEngine::new(TenantId::new("test"));
        let session = engine.create_session_key(test_address_1(), 7200, vec![]);

        assert_eq!(session.key_address, test_address_1());
        assert!(session.valid_until > session.valid_after);
        assert!((session.valid_until - session.valid_after) <= 7200 + 1);
        assert!(session.permissions.is_empty());
    }
}

// ==========================================================================
// PaymasterService tests
// ==========================================================================

mod paymaster_tests {
    use super::*;
    use ramp_aa::paymaster::{Paymaster, PaymasterService};

    #[test]
    fn test_paymaster_service_creation_valid_key() {
        let result = PaymasterService::new(test_address_1(), test_signing_key());
        assert!(result.is_ok());
    }

    #[test]
    fn test_paymaster_service_creation_invalid_key_length() {
        let result = PaymasterService::new(test_address_1(), vec![0x01; 16]); // Too short
        assert!(result.is_err());
    }

    #[test]
    fn test_paymaster_service_creation_invalid_key_too_long() {
        let result = PaymasterService::new(test_address_1(), vec![0x01; 64]); // Too long
        assert!(result.is_err());
    }

    #[test]
    fn test_signer_address_nonzero() {
        let service = PaymasterService::new(test_address_1(), test_signing_key()).unwrap();
        let address = service.signer_address();
        assert_ne!(address, Address::ZERO);
    }

    #[test]
    fn test_signer_address_deterministic() {
        let service1 = PaymasterService::new(test_address_1(), test_signing_key()).unwrap();
        let service2 = PaymasterService::new(test_address_1(), test_signing_key()).unwrap();
        assert_eq!(service1.signer_address(), service2.signer_address());
    }

    #[test]
    fn test_different_keys_different_addresses() {
        let key1 = vec![0x01u8; 32];
        let key2 = vec![0x02u8; 32];

        let service1 = PaymasterService::new(test_address_1(), key1).unwrap();
        let service2 = PaymasterService::new(test_address_1(), key2).unwrap();

        assert_ne!(service1.signer_address(), service2.signer_address());
    }

    #[tokio::test]
    async fn test_can_sponsor_within_limits() {
        let service = PaymasterService::new(test_address_1(), test_signing_key()).unwrap();

        let policy = SponsorshipPolicy {
            max_gas_per_op: U256::from(500_000),
            ..Default::default()
        };

        let op = make_user_op(); // total gas = 221_000

        let can = service.can_sponsor(&op, &policy).await.unwrap();
        assert!(can);
    }

    #[tokio::test]
    async fn test_can_sponsor_exceeds_gas_limit() {
        let service = PaymasterService::new(test_address_1(), test_signing_key()).unwrap();

        let policy = SponsorshipPolicy {
            max_gas_per_op: U256::from(100_000), // Lower than total
            ..Default::default()
        };

        let op = make_user_op(); // total gas = 221_000

        let can = service.can_sponsor(&op, &policy).await.unwrap();
        assert!(!can);
    }

    #[tokio::test]
    async fn test_sponsor_produces_correct_format() {
        let paymaster_address = Address::from([0x42u8; 20]);
        let service = PaymasterService::new(paymaster_address, test_signing_key()).unwrap();

        let op = make_user_op();
        let data = service.sponsor(&op).await.unwrap();

        // Format: 20 (address) + 6 (validUntil) + 6 (validAfter) + 65 (sig) = 97
        assert_eq!(data.paymaster_and_data.len(), 97);
        assert_eq!(data.paymaster_address, paymaster_address);
        assert!(data.valid_until > data.valid_after);

        // Verify embedded address
        assert_eq!(
            &data.paymaster_and_data[0..20],
            paymaster_address.as_slice()
        );

        // Verify signature v value
        let v = data.paymaster_and_data[96];
        assert!(v == 27 || v == 28);
    }

    #[test]
    fn test_recover_signer_invalid_v() {
        let hash = [0xab; 32];
        let mut sig = [0u8; 65];
        sig[64] = 30; // Invalid v
        let result = PaymasterService::recover_signer(&hash, &sig);
        assert!(result.is_err());
    }

    #[test]
    fn test_sponsorship_policy_default() {
        let policy = SponsorshipPolicy::default();
        assert_eq!(policy.max_gas_per_op, U256::from(500_000));
        assert_eq!(policy.max_ops_per_user_per_day, 100);
        assert!(policy.allowed_contracts.is_empty());
        assert!(policy.allowed_selectors.is_empty());
    }
}

// ==========================================================================
// Multi-token Paymaster tests
// ==========================================================================

mod multi_token_tests {
    use super::*;
    use ramp_aa::paymaster::{
        GasToken, MockPriceOracle, MultiTokenPaymaster, MultiTokenPaymasterConfig, PriceOracle,
        TenantGasLimits, TokenConfig,
    };

    fn test_config() -> MultiTokenPaymasterConfig {
        MultiTokenPaymasterConfig {
            paymaster_address: Address::from([0x42u8; 20]),
            chain_id: 1,
            supported_tokens: vec![
                TokenConfig {
                    token: GasToken::USDT,
                    chain_id: 1,
                    token_address: Address::from([0x11u8; 20]),
                    oracle_address: None,
                    enabled: true,
                },
                TokenConfig {
                    token: GasToken::USDC,
                    chain_id: 1,
                    token_address: Address::from([0x22u8; 20]),
                    oracle_address: None,
                    enabled: true,
                },
            ],
            default_markup_percentage: 5,
            quote_validity_seconds: 300,
        }
    }

    #[test]
    fn test_gas_token_symbol() {
        assert_eq!(GasToken::Native.symbol(), "ETH");
        assert_eq!(GasToken::USDT.symbol(), "USDT");
        assert_eq!(GasToken::USDC.symbol(), "USDC");
        assert_eq!(GasToken::DAI.symbol(), "DAI");
        assert_eq!(GasToken::VNST.symbol(), "VNST");
    }

    #[test]
    fn test_gas_token_decimals() {
        assert_eq!(GasToken::Native.decimals(), 18);
        assert_eq!(GasToken::USDT.decimals(), 6);
        assert_eq!(GasToken::USDC.decimals(), 6);
        assert_eq!(GasToken::DAI.decimals(), 18);
        assert_eq!(GasToken::VNST.decimals(), 18);
    }

    #[test]
    fn test_gas_token_from_symbol() {
        assert_eq!(GasToken::from_symbol("ETH"), Some(GasToken::Native));
        assert_eq!(GasToken::from_symbol("MATIC"), Some(GasToken::Native));
        assert_eq!(GasToken::from_symbol("BNB"), Some(GasToken::Native));
        assert_eq!(GasToken::from_symbol("NATIVE"), Some(GasToken::Native));
        assert_eq!(GasToken::from_symbol("usdt"), Some(GasToken::USDT));
        assert_eq!(GasToken::from_symbol("USDC"), Some(GasToken::USDC));
        assert_eq!(GasToken::from_symbol("dai"), Some(GasToken::DAI));
        assert_eq!(GasToken::from_symbol("VNST"), Some(GasToken::VNST));
        assert_eq!(GasToken::from_symbol("INVALID"), None);
    }

    #[test]
    fn test_get_supported_tokens_with_limits() {
        let oracle = Arc::new(MockPriceOracle::new());
        let mut paymaster = MultiTokenPaymaster::new(test_config(), oracle);

        let tenant_id = TenantId::new("test");
        paymaster.set_tenant_limits(TenantGasLimits {
            tenant_id: tenant_id.clone(),
            allowed_tokens: vec![GasToken::USDT],
            ..Default::default()
        });

        let tokens = paymaster.get_supported_tokens(&tenant_id);
        assert_eq!(tokens, vec![GasToken::USDT]);
    }

    #[test]
    fn test_get_supported_tokens_default() {
        let oracle = Arc::new(MockPriceOracle::new());
        let paymaster = MultiTokenPaymaster::new(test_config(), oracle);

        let tokens = paymaster.get_supported_tokens(&TenantId::new("unknown"));
        assert!(tokens.contains(&GasToken::Native));
        assert!(tokens.contains(&GasToken::USDT));
        assert!(tokens.contains(&GasToken::USDC));
    }

    #[tokio::test]
    async fn test_quote_gas_usdt() {
        let oracle = Arc::new(MockPriceOracle::new());
        let mut paymaster = MultiTokenPaymaster::new(test_config(), oracle);

        let tenant_id = TenantId::new("test");
        paymaster.set_tenant_limits(TenantGasLimits {
            tenant_id: tenant_id.clone(),
            allowed_tokens: vec![GasToken::USDT, GasToken::USDC],
            ..Default::default()
        });

        let op = make_user_op();
        let quote = paymaster
            .quote_gas(&op, GasToken::USDT, &tenant_id)
            .await
            .unwrap();

        assert_eq!(quote.token, GasToken::USDT);
        assert_eq!(quote.chain_id, 1);
        assert!(quote.token_gas_cost > U256::ZERO);
        assert!(quote.native_gas_cost > U256::ZERO);
        assert_eq!(quote.markup_percentage, 5);
    }

    #[tokio::test]
    async fn test_quote_gas_unsupported_token() {
        let oracle = Arc::new(MockPriceOracle::new());
        let mut paymaster = MultiTokenPaymaster::new(test_config(), oracle);

        let tenant_id = TenantId::new("test");
        paymaster.set_tenant_limits(TenantGasLimits {
            tenant_id: tenant_id.clone(),
            allowed_tokens: vec![GasToken::USDT],
            ..Default::default()
        });

        let op = make_user_op();
        let result = paymaster.quote_gas(&op, GasToken::DAI, &tenant_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_can_sponsor_allowed_token() {
        let oracle = Arc::new(MockPriceOracle::new());
        let mut paymaster = MultiTokenPaymaster::new(test_config(), oracle);

        let tenant_id = TenantId::new("test");
        paymaster.set_tenant_limits(TenantGasLimits {
            tenant_id: tenant_id.clone(),
            allowed_tokens: vec![GasToken::USDT],
            max_gas_per_op: U256::from(100_000_000_000_000_000u64),
            ..Default::default()
        });

        let op = make_user_op();
        let can = paymaster
            .can_sponsor(&op, &tenant_id, GasToken::USDT)
            .await
            .unwrap();
        assert!(can);
    }

    #[tokio::test]
    async fn test_can_sponsor_disallowed_token() {
        let oracle = Arc::new(MockPriceOracle::new());
        let mut paymaster = MultiTokenPaymaster::new(test_config(), oracle);

        let tenant_id = TenantId::new("test");
        paymaster.set_tenant_limits(TenantGasLimits {
            tenant_id: tenant_id.clone(),
            allowed_tokens: vec![GasToken::USDT],
            ..Default::default()
        });

        let op = make_user_op();
        let can = paymaster
            .can_sponsor(&op, &tenant_id, GasToken::DAI)
            .await
            .unwrap();
        assert!(!can);
    }

    #[tokio::test]
    async fn test_can_sponsor_no_limits_configured() {
        let oracle = Arc::new(MockPriceOracle::new());
        let paymaster = MultiTokenPaymaster::new(test_config(), oracle);

        let op = make_user_op();
        let can = paymaster
            .can_sponsor(&op, &TenantId::new("unknown"), GasToken::USDT)
            .await
            .unwrap();
        assert!(!can); // No limits = no sponsorship
    }

    #[test]
    fn test_generate_approval_data() {
        let oracle = Arc::new(MockPriceOracle::new());
        let paymaster = MultiTokenPaymaster::new(test_config(), oracle);

        let data = paymaster
            .generate_approval_data(GasToken::USDT, U256::from(1_000_000))
            .unwrap();

        // approve(address,uint256) selector
        assert_eq!(&data[0..4], &[0x09, 0x5e, 0xa7, 0xb3]);
        // 4 (selector) + 32 (spender) + 32 (amount) = 68
        assert_eq!(data.len(), 68);
    }

    #[test]
    fn test_generate_approval_data_unsupported_token() {
        let oracle = Arc::new(MockPriceOracle::new());
        let paymaster = MultiTokenPaymaster::new(test_config(), oracle);

        let result = paymaster.generate_approval_data(GasToken::DAI, U256::from(1_000));
        assert!(result.is_err());
    }

    #[test]
    fn test_record_usage() {
        let oracle = Arc::new(MockPriceOracle::new());
        let mut paymaster = MultiTokenPaymaster::new(test_config(), oracle);

        let tenant_id = TenantId::new("test");
        let user = test_address_1();

        paymaster.record_usage(&tenant_id, user, U256::from(100_000));
        paymaster.record_usage(&tenant_id, user, U256::from(200_000));
    }

    #[tokio::test]
    async fn test_mock_price_oracle() {
        let oracle = MockPriceOracle::new();

        let eth_price = oracle.get_price(GasToken::Native, 1).await.unwrap();
        assert!(eth_price > U256::ZERO);

        let usdt_price = oracle.get_price(GasToken::USDT, 1).await.unwrap();
        assert!(usdt_price > U256::ZERO);

        // ETH should be more expensive in token terms
        assert!(eth_price > usdt_price);
    }

    #[tokio::test]
    async fn test_mock_price_oracle_unknown() {
        let oracle = MockPriceOracle::new();

        let result = oracle.get_price(GasToken::VNST, 999).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_price_oracle_cached() {
        let oracle = MockPriceOracle::new();

        let (price, expiry) = oracle.get_cached_price(GasToken::USDT, 1).await.unwrap();
        assert!(price > U256::ZERO);
        assert!(expiry > 0);
    }

    #[test]
    fn test_tenant_gas_limits_default() {
        let limits = TenantGasLimits::default();
        assert_eq!(limits.max_ops_per_user_daily, 100);
        assert!(!limits.full_sponsorship);
        assert!(limits.custom_markup.is_none());
        assert!(limits.allowed_tokens.contains(&GasToken::Native));
    }
}

// ==========================================================================
// Cross-chain Paymaster tests
// ==========================================================================

mod cross_chain_tests {
    use super::*;
    use ramp_aa::paymaster::{
        CrossChainPaymaster, CrossChainPaymasterConfig, GasToken, LiquidityProvider,
        MockLiquidityProvider, MockPriceOracle, SupportedChain, TenantGasLimits,
    };

    fn test_config() -> CrossChainPaymasterConfig {
        let mut addresses = HashMap::new();
        addresses.insert(1, Address::from([0x11u8; 20]));
        addresses.insert(137, Address::from([0x22u8; 20]));
        addresses.insert(42161, Address::from([0x33u8; 20]));

        CrossChainPaymasterConfig {
            paymaster_addresses: addresses,
            bridge_fee_bps: 10,
            paymaster_fee_bps: 50,
            quote_validity_seconds: 300,
            supported_routes: vec![
                (SupportedChain::Ethereum, SupportedChain::Arbitrum),
                (SupportedChain::Ethereum, SupportedChain::Polygon),
                (SupportedChain::Polygon, SupportedChain::Ethereum),
                (SupportedChain::Arbitrum, SupportedChain::Optimism),
            ],
        }
    }

    #[test]
    fn test_supported_chain_chain_id() {
        assert_eq!(SupportedChain::Ethereum.chain_id(), 1);
        assert_eq!(SupportedChain::Polygon.chain_id(), 137);
        assert_eq!(SupportedChain::BnbChain.chain_id(), 56);
        assert_eq!(SupportedChain::Arbitrum.chain_id(), 42161);
        assert_eq!(SupportedChain::Optimism.chain_id(), 10);
        assert_eq!(SupportedChain::Base.chain_id(), 8453);
    }

    #[test]
    fn test_supported_chain_name() {
        assert_eq!(SupportedChain::Ethereum.name(), "Ethereum");
        assert_eq!(SupportedChain::Polygon.name(), "Polygon");
        assert_eq!(SupportedChain::BnbChain.name(), "BNB Chain");
    }

    #[test]
    fn test_supported_chain_native_token() {
        assert_eq!(SupportedChain::Ethereum.native_token(), "ETH");
        assert_eq!(SupportedChain::Polygon.native_token(), "MATIC");
        assert_eq!(SupportedChain::BnbChain.native_token(), "BNB");
        assert_eq!(SupportedChain::Arbitrum.native_token(), "ETH");
    }

    #[test]
    fn test_supported_chain_from_chain_id() {
        assert_eq!(
            SupportedChain::from_chain_id(1),
            Some(SupportedChain::Ethereum)
        );
        assert_eq!(
            SupportedChain::from_chain_id(137),
            Some(SupportedChain::Polygon)
        );
        assert_eq!(SupportedChain::from_chain_id(999), None);
    }

    #[test]
    fn test_supported_chain_is_l2() {
        assert!(!SupportedChain::Ethereum.is_l2());
        assert!(!SupportedChain::Polygon.is_l2());
        assert!(!SupportedChain::BnbChain.is_l2());
        assert!(SupportedChain::Arbitrum.is_l2());
        assert!(SupportedChain::Optimism.is_l2());
        assert!(SupportedChain::Base.is_l2());
    }

    #[test]
    fn test_route_supported() {
        let oracle = Arc::new(MockPriceOracle::new());
        let liquidity = Arc::new(MockLiquidityProvider::new());
        let paymaster = CrossChainPaymaster::new(test_config(), oracle, liquidity);

        assert!(paymaster.is_route_supported(SupportedChain::Ethereum, SupportedChain::Arbitrum));
        assert!(paymaster.is_route_supported(SupportedChain::Ethereum, SupportedChain::Polygon));
        assert!(!paymaster.is_route_supported(SupportedChain::Polygon, SupportedChain::Arbitrum));
    }

    #[test]
    fn test_get_available_routes() {
        let oracle = Arc::new(MockPriceOracle::new());
        let liquidity = Arc::new(MockLiquidityProvider::new());
        let paymaster = CrossChainPaymaster::new(test_config(), oracle, liquidity);

        let routes = paymaster.get_available_routes(SupportedChain::Ethereum);
        assert!(routes.contains(&SupportedChain::Arbitrum));
        assert!(routes.contains(&SupportedChain::Polygon));
        assert!(!routes.contains(&SupportedChain::Optimism));
    }

    #[tokio::test]
    async fn test_quote_cross_chain_gas() {
        let oracle = Arc::new(MockPriceOracle::new());
        let liquidity = Arc::new(MockLiquidityProvider::new());
        let mut paymaster = CrossChainPaymaster::new(test_config(), oracle, liquidity);

        let tenant_id = TenantId::new("test");
        paymaster.set_tenant_limits(TenantGasLimits {
            tenant_id: tenant_id.clone(),
            allowed_tokens: vec![GasToken::Native],
            ..Default::default()
        });

        let op = make_user_op();
        let quote = paymaster
            .quote_cross_chain_gas(
                &op,
                SupportedChain::Ethereum,
                SupportedChain::Arbitrum,
                GasToken::Native,
                &tenant_id,
            )
            .await
            .unwrap();

        assert_eq!(quote.source_chain_id, 1);
        assert_eq!(quote.target_chain_id, 42161);
        assert!(quote.source_payment_amount > U256::ZERO);
        assert!(quote.bridge_fee > U256::ZERO);
        assert!(quote.paymaster_fee > U256::ZERO);
    }

    #[tokio::test]
    async fn test_quote_unsupported_route() {
        let oracle = Arc::new(MockPriceOracle::new());
        let liquidity = Arc::new(MockLiquidityProvider::new());
        let paymaster = CrossChainPaymaster::new(test_config(), oracle, liquidity);

        let op = make_user_op();
        let result = paymaster
            .quote_cross_chain_gas(
                &op,
                SupportedChain::BnbChain, // Not in routes
                SupportedChain::Arbitrum,
                GasToken::Native,
                &TenantId::new("test"),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_liquidity_provider() {
        let provider = MockLiquidityProvider::new();

        let eth_liquidity = provider
            .get_liquidity(SupportedChain::Ethereum, GasToken::Native)
            .await
            .unwrap();
        assert!(eth_liquidity > U256::ZERO);

        let unknown = provider
            .get_liquidity(SupportedChain::Base, GasToken::USDT)
            .await
            .unwrap();
        assert_eq!(unknown, U256::ZERO);
    }

    #[tokio::test]
    async fn test_mock_liquidity_reserve_release() {
        let provider = MockLiquidityProvider::new();

        let reserved = provider
            .reserve_liquidity(
                SupportedChain::Ethereum,
                GasToken::Native,
                U256::from(1_000_000_000_000_000_000u64), // 1 ETH
                "test_reservation",
            )
            .await
            .unwrap();
        assert!(reserved);

        provider
            .release_liquidity("test_reservation")
            .await
            .unwrap();
        provider
            .confirm_usage("test_reservation", U256::from(500_000_000_000_000_000u64))
            .await
            .unwrap();
    }
}

// ==========================================================================
// EIP-7702 tests
// ==========================================================================

mod eip7702_tests {
    use super::*;
    use ramp_aa::eip7702::{
        authorization::{Authorization, AuthorizationList, Signature, SignedAuthorization},
        delegation::{
            Delegation, DelegationManager, DelegationRegistry, DelegationStatus, SessionDelegation,
            SessionPermissions,
        },
        Eip7702Config, Eip7702Error,
    };

    fn test_delegate() -> Address {
        "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
            .parse()
            .unwrap()
    }

    #[test]
    fn test_authorization_new() {
        let auth = Authorization::new(U256::from(1), test_delegate(), 5);
        assert_eq!(auth.chain_id, U256::from(1));
        assert_eq!(auth.address, test_delegate());
        assert_eq!(auth.nonce, 5);
    }

    #[test]
    fn test_authorization_for_chain() {
        let auth = Authorization::for_chain(137, test_delegate(), 10);
        assert_eq!(auth.chain_id, U256::from(137));
        assert_eq!(auth.nonce, 10);
    }

    #[test]
    fn test_authorization_signing_hash_deterministic() {
        let auth1 = Authorization::for_chain(1, test_delegate(), 0);
        let auth2 = Authorization::for_chain(1, test_delegate(), 0);
        assert_eq!(auth1.signing_hash(), auth2.signing_hash());
    }

    #[test]
    fn test_authorization_signing_hash_different_nonces() {
        let auth1 = Authorization::for_chain(1, test_delegate(), 0);
        let auth2 = Authorization::for_chain(1, test_delegate(), 1);
        assert_ne!(auth1.signing_hash(), auth2.signing_hash());
    }

    #[test]
    fn test_authorization_signing_hash_different_chains() {
        let auth1 = Authorization::for_chain(1, test_delegate(), 0);
        let auth2 = Authorization::for_chain(137, test_delegate(), 0);
        assert_ne!(auth1.signing_hash(), auth2.signing_hash());
    }

    #[test]
    fn test_authorization_rlp_encode() {
        let auth = Authorization::for_chain(1, test_delegate(), 0);
        let encoded = auth.rlp_encode();
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_signed_authorization_rlp_roundtrip() {
        let auth = Authorization::for_chain(1, test_delegate(), 5);
        let signed = SignedAuthorization::new(
            auth,
            Signature {
                r: U256::from(12345),
                s: U256::from(67890),
                v: 27,
            },
        );

        let encoded = signed.rlp_encode();
        let decoded = SignedAuthorization::rlp_decode(&encoded).expect("should decode");

        assert_eq!(
            decoded.authorization.chain_id,
            signed.authorization.chain_id
        );
        assert_eq!(decoded.authorization.address, signed.authorization.address);
        assert_eq!(decoded.authorization.nonce, signed.authorization.nonce);
        assert_eq!(decoded.signature.v, signed.signature.v);
    }

    #[test]
    fn test_authorization_list_empty() {
        let list = AuthorizationList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_authorization_list_add() {
        let mut list = AuthorizationList::new();
        let auth = Authorization::for_chain(1, Address::ZERO, 0);
        let signed = SignedAuthorization::new(
            auth,
            Signature {
                r: U256::from(1),
                s: U256::from(2),
                v: 27,
            },
        );

        list.add(signed);
        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
    }

    #[test]
    fn test_authorization_list_builder_pattern() {
        let auth = Authorization::for_chain(1, Address::ZERO, 0);
        let signed = SignedAuthorization::new(
            auth,
            Signature {
                r: U256::from(1),
                s: U256::from(2),
                v: 27,
            },
        );

        let list = AuthorizationList::new().with_authorization(signed);
        assert_eq!(list.len(), 1);
    }

    // -- Delegation tests --

    #[test]
    fn test_delegation_new_is_pending() {
        let d = Delegation::new(test_address_1(), test_delegate(), U256::from(1), 0);
        assert_eq!(d.status, DelegationStatus::Pending);
        assert!(!d.is_active());
    }

    #[test]
    fn test_delegation_activate() {
        let mut d = Delegation::new(test_address_1(), test_delegate(), U256::from(1), 0);
        d.activate();
        assert!(d.is_active());
        assert_eq!(d.status, DelegationStatus::Active);
    }

    #[test]
    fn test_delegation_revoke() {
        let mut d = Delegation::new(test_address_1(), test_delegate(), U256::from(1), 0);
        d.activate();
        d.revoke();
        assert!(!d.is_active());
        assert_eq!(d.status, DelegationStatus::Revoked);
    }

    #[test]
    fn test_session_delegation_creation() {
        let session =
            SessionDelegation::new(test_address_1(), test_delegate(), U256::from(1), 0, 3600);

        assert!(!session.is_expired());
        assert!(session.remaining_seconds() > 0);
        assert!(session.remaining_seconds() <= 3600);
    }

    #[test]
    fn test_session_permissions_permit_all_when_empty() {
        let perms = SessionPermissions::new();
        assert!(perms.is_permitted(test_address_1(), U256::from(1000), None));
    }

    #[test]
    fn test_session_permissions_target_filter() {
        let perms = SessionPermissions::new().allow_target(test_address_1());

        assert!(perms.is_permitted(test_address_1(), U256::ZERO, None));
        assert!(!perms.is_permitted(test_address_2(), U256::ZERO, None));
    }

    #[test]
    fn test_session_permissions_max_value() {
        let perms = SessionPermissions::new().with_max_value(U256::from(1000));

        assert!(perms.is_permitted(test_address_1(), U256::from(500), None));
        assert!(!perms.is_permitted(test_address_1(), U256::from(1500), None));
    }

    #[test]
    fn test_session_permissions_selector_filter() {
        let perms = SessionPermissions::new().allow_selector([0xaa, 0xbb, 0xcc, 0xdd]);

        assert!(perms.is_permitted(test_address_1(), U256::ZERO, Some([0xaa, 0xbb, 0xcc, 0xdd])));
        assert!(!perms.is_permitted(test_address_1(), U256::ZERO, Some([0x11, 0x22, 0x33, 0x44])));
        assert!(perms.is_permitted(test_address_1(), U256::ZERO, None)); // No selector = OK
    }

    #[test]
    fn test_delegation_registry_register() {
        let registry = DelegationRegistry::new();
        let d = Delegation::new(test_address_1(), test_delegate(), U256::from(1), 0);
        registry.register(d).unwrap();

        // Pending delegation not returned as active
        assert!(registry.get_active(test_address_1()).is_none());
    }

    #[test]
    fn test_delegation_registry_revoke() {
        let registry = DelegationRegistry::new();
        let mut d = Delegation::new(test_address_1(), test_delegate(), U256::from(1), 0);
        d.activate();
        registry.register(d).unwrap();

        // Should find active
        assert!(registry.get_active(test_address_1()).is_some());

        // Revoke
        registry.revoke(test_address_1()).unwrap();
        assert!(registry.get_active(test_address_1()).is_none());
    }

    #[test]
    fn test_delegation_registry_revoke_not_found() {
        let registry = DelegationRegistry::new();
        let result = registry.revoke(test_address_1());
        assert!(result.is_err());
    }

    #[test]
    fn test_delegation_manager_create() {
        let config = Eip7702Config::new(1, test_delegate());
        let manager = DelegationManager::new(config);

        let d = manager
            .create_delegation(test_address_1(), test_delegate(), 0)
            .unwrap();
        assert_eq!(d.chain_id, U256::from(1));
        assert_eq!(d.status, DelegationStatus::Pending);
    }

    #[test]
    fn test_delegation_manager_duplicate_fails() {
        let config = Eip7702Config::new(1, test_delegate());
        let manager = DelegationManager::new(config);

        manager
            .create_delegation(test_address_1(), test_delegate(), 0)
            .unwrap();

        let result = manager.create_delegation(test_address_1(), test_delegate(), 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_delegation_manager_create_session() {
        let config = Eip7702Config::new(1, test_delegate()).with_max_duration(7200);
        let manager = DelegationManager::new(config);

        let session = manager
            .create_session(test_address_1(), test_delegate(), 0, 3600, None)
            .unwrap();

        assert!(session.remaining_seconds() > 0);
    }

    #[test]
    fn test_delegation_manager_session_exceeds_max_duration() {
        let config = Eip7702Config::new(1, test_delegate()).with_max_duration(3600);
        let manager = DelegationManager::new(config);

        let result = manager.create_session(test_address_1(), test_delegate(), 0, 7200, None);
        assert!(matches!(
            result,
            Err(Eip7702Error::DurationExceedsMax(_, _))
        ));
    }

    #[test]
    fn test_delegation_manager_revoke() {
        let config = Eip7702Config::new(1, test_delegate());
        let manager = DelegationManager::new(config);

        manager
            .create_delegation(test_address_1(), test_delegate(), 0)
            .unwrap();

        manager.revoke(test_address_1()).unwrap();
    }

    #[test]
    fn test_delegation_manager_revoke_not_allowed() {
        let config = Eip7702Config::new(1, test_delegate()).with_revocation(false);
        let manager = DelegationManager::new(config);

        manager
            .create_delegation(test_address_1(), test_delegate(), 0)
            .unwrap();

        let result = manager.revoke(test_address_1());
        assert!(matches!(result, Err(Eip7702Error::RevocationNotAllowed)));
    }

    #[test]
    fn test_eip7702_config_default() {
        let config = Eip7702Config::default();
        assert_eq!(config.chain_id, U256::from(1));
        assert!(config.allow_revocation);
        assert_eq!(config.max_delegation_duration, 86400 * 30);
    }

    #[test]
    fn test_eip7702_config_builder() {
        let config = Eip7702Config::new(137, test_delegate())
            .with_max_duration(3600)
            .with_revocation(false);

        assert_eq!(config.chain_id, U256::from(137));
        assert_eq!(config.default_delegate, test_delegate());
        assert_eq!(config.max_delegation_duration, 3600);
        assert!(!config.allow_revocation);
    }
}

// ==========================================================================
// Types tests
// ==========================================================================

mod types_tests {
    use super::*;

    #[test]
    fn test_smart_account_type_variants() {
        let types = vec![
            SmartAccountType::SimpleAccount,
            SmartAccountType::SafeAccount,
            SmartAccountType::KernelAccount,
            SmartAccountType::BiconomyAccount,
            SmartAccountType::Eip7702Account,
        ];

        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            let back: SmartAccountType = serde_json::from_str(&json).unwrap();
            assert_eq!(*t, back);
        }
    }

    #[test]
    fn test_user_op_status_variants() {
        let statuses = vec![
            UserOpStatus::Pending,
            UserOpStatus::Submitted,
            UserOpStatus::Bundled,
            UserOpStatus::OnChain,
            UserOpStatus::Success,
            UserOpStatus::Failed,
            UserOpStatus::Reverted,
        ];

        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: UserOpStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn test_chain_config_ethereum() {
        let config = ChainConfig::ethereum_mainnet().unwrap();
        assert_eq!(config.chain_id, 1);
        assert_eq!(config.name, "Ethereum Mainnet");
        assert!(config.paymaster_address.is_none());
    }

    #[test]
    fn test_chain_config_polygon() {
        let config = ChainConfig::polygon_mainnet().unwrap();
        assert_eq!(config.chain_id, 137);
        assert_eq!(config.name, "Polygon Mainnet");
    }

    #[test]
    fn test_chain_config_bnb() {
        let config = ChainConfig::bnb_chain().unwrap();
        assert_eq!(config.chain_id, 56);
        assert_eq!(config.name, "BNB Chain");
    }

    #[test]
    fn test_gas_estimation_serialization() {
        let estimation = GasEstimation {
            pre_verification_gas: U256::from(21_000),
            verification_gas_limit: U256::from(100_000),
            call_gas_limit: U256::from(200_000),
            max_fee_per_gas: U256::from(30_000_000_000u64),
            max_priority_fee_per_gas: U256::from(1_000_000_000u64),
        };

        let json = serde_json::to_string(&estimation).unwrap();
        let back: GasEstimation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.pre_verification_gas, estimation.pre_verification_gas);
    }

    #[test]
    fn test_session_key_serialization() {
        let key = SessionKey {
            key_address: test_address_1(),
            valid_until: 1700000000,
            valid_after: 1699999000,
            permissions: vec![SessionPermission {
                target: test_address_2(),
                selector: [0xaa, 0xbb, 0xcc, 0xdd],
                max_value: U256::from(1000),
                rules: vec![
                    PermissionRule::MaxAmount(U256::from(500)),
                    PermissionRule::TimeWindow {
                        start: 1000,
                        end: 2000,
                    },
                ],
            }],
        };

        let json = serde_json::to_string(&key).unwrap();
        let back: SessionKey = serde_json::from_str(&json).unwrap();
        assert_eq!(back.key_address, key.key_address);
        assert_eq!(back.permissions.len(), 1);
        assert_eq!(back.permissions[0].rules.len(), 2);
    }

    #[test]
    fn test_permission_rule_serialization() {
        let rules = vec![
            PermissionRule::MaxAmount(U256::from(1000)),
            PermissionRule::AllowedRecipients(vec![test_address_1()]),
            PermissionRule::TimeWindow {
                start: 100,
                end: 200,
            },
            PermissionRule::RateLimit {
                count: 10,
                period_secs: 3600,
            },
        ];

        for rule in &rules {
            let json = serde_json::to_string(rule).unwrap();
            let _back: PermissionRule = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_paymaster_data_serialization() {
        let data = PaymasterData {
            paymaster_address: test_address_1(),
            paymaster_and_data: Bytes::from(vec![0x42; 97]),
            valid_until: 1700000000,
            valid_after: 1699999000,
        };

        let json = serde_json::to_string(&data).unwrap();
        let back: PaymasterData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.paymaster_address, data.paymaster_address);
        assert_eq!(back.valid_until, data.valid_until);
    }
}

// ==========================================================================
// Bundler client tests (unit-level, no network)
// ==========================================================================

mod bundler_tests {
    use super::*;
    use ramp_aa::bundler::BundlerClient;

    #[test]
    fn test_bundler_client_creation() {
        let config = ChainConfig::ethereum_mainnet().unwrap();
        let _client = BundlerClient::new(config);
    }

    #[test]
    fn test_bundler_client_with_custom_config() {
        let config = ChainConfig {
            chain_id: 31337,
            name: "Local".to_string(),
            entry_point_address: test_entry_point(),
            bundler_url: "http://localhost:3000".to_string(),
            paymaster_address: Some(test_address_1()),
        };
        let _client = BundlerClient::new(config);
    }
}
