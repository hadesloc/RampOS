//! Comprehensive unit tests for ramp-adapter module
//!
//! Tests cover:
//! - NapasAdapter: request building, response parsing, webhook parsing, status mapping, simulation mode
//! - VietQRAdapter: QR generation, EMVCo content, bank info, webhook parsing
//! - MockAdapter: payin/payout flows, configurable behavior, webhook payloads
//! - AdapterFactory: registration, creation, case insensitivity, error cases
//! - eKYC providers: mock provider, FPT.AI config, VNPay config, FullEkycResult calculation
//! - Types: serialization, default configs

use chrono::{Duration, Utc};
use ramp_adapter::adapters::ekyc::mock::MockEkycBehavior;
use ramp_adapter::adapters::mock::MockBehavior;
use ramp_adapter::*;
use rust_decimal::Decimal;
use std::collections::HashMap;

// ============================================================================
// NapasAdapter Tests
// ============================================================================

mod napas_tests {
    use super::*;

    fn make_napas() -> NapasAdapter {
        NapasAdapter::new("napas_test", "webhook_secret_123").unwrap()
    }

    #[test]
    fn test_napas_new_creates_adapter() {
        let adapter = make_napas();
        assert_eq!(adapter.provider_code(), "napas_test");
        assert_eq!(adapter.provider_name(), "Napas");
        assert!(adapter.is_simulation_mode());
    }

    #[test]
    fn test_napas_with_config() {
        let config = NapasConfig {
            base: AdapterConfig {
                provider_code: "napas_prod".to_string(),
                api_base_url: "https://custom.napas.vn".to_string(),
                api_key: "prod_key".to_string(),
                api_secret: "prod_secret".to_string(),
                webhook_secret: "prod_webhook".to_string(),
                timeout_secs: 60,
                extra: serde_json::json!({"env": "production"}),
            },
            merchant_id: "MERCHANT001".to_string(),
            terminal_id: "TERM001".to_string(),
            partner_code: "PARTNER001".to_string(),
            enable_real_api: false,
            private_key_pem: None,
            napas_public_key_pem: None,
        };
        let adapter = NapasAdapter::with_config(config).unwrap();
        assert_eq!(adapter.provider_code(), "napas_prod");
        assert!(adapter.is_simulation_mode());
    }

    #[test]
    fn test_napas_with_real_api_enabled() {
        let config = NapasConfig {
            enable_real_api: true,
            ..NapasConfig::default()
        };
        let adapter = NapasAdapter::with_config(config).unwrap();
        assert!(!adapter.is_simulation_mode());
    }

    #[tokio::test]
    async fn test_napas_create_payin_instruction_returns_valid_instruction() {
        let adapter = make_napas();
        let request = CreatePayinInstructionRequest {
            reference_code: "PAY_IN_001".to_string(),
            user_id: "user_abc".to_string(),
            amount_vnd: Decimal::from(250_000),
            expires_at: Utc::now() + Duration::hours(2),
            metadata: serde_json::json!({"source": "test"}),
        };

        let result = adapter.create_payin_instruction(request).await.unwrap();
        assert_eq!(result.reference_code, "PAY_IN_001");
        assert_eq!(result.bank_code, "NAPAS");
        assert!(result.account_number.starts_with("NAPAS"));
        assert_eq!(result.account_name, "Napas Merchant");
        assert_eq!(result.amount_vnd, Decimal::from(250_000));
        assert_eq!(result.instructions, "Pay via Napas gateway");
    }

    #[tokio::test]
    async fn test_napas_create_payin_preserves_expires_at() {
        let adapter = make_napas();
        let expires = Utc::now() + Duration::hours(6);
        let request = CreatePayinInstructionRequest {
            reference_code: "EXP_TEST".to_string(),
            user_id: "user1".to_string(),
            amount_vnd: Decimal::from(100_000),
            expires_at: expires,
            metadata: serde_json::json!({}),
        };

        let result = adapter.create_payin_instruction(request).await.unwrap();
        assert_eq!(result.expires_at, expires);
    }

    #[tokio::test]
    async fn test_napas_initiate_payout_simulation_mode() {
        let adapter = make_napas();
        let request = InitiatePayoutRequest {
            reference_code: "PAYOUT_SIM_001".to_string(),
            amount_vnd: Decimal::from(1_000_000),
            recipient_bank_code: "970436".to_string(),
            recipient_account_number: "0987654321".to_string(),
            recipient_account_name: "TRAN VAN B".to_string(),
            description: "Test payout simulation".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = adapter.initiate_payout(request).await.unwrap();
        assert_eq!(result.reference_code, "PAYOUT_SIM_001");
        assert!(result.provider_tx_id.starts_with("NAPAS_SIM_"));
        assert_eq!(result.status, PayoutStatus::Processing);
        assert!(result.estimated_completion.is_some());
    }

    #[tokio::test]
    async fn test_napas_check_payout_status_simulation() {
        let adapter = make_napas();
        let status = adapter.check_payout_status("ANY_REF").await.unwrap();
        assert_eq!(status, PayoutStatus::Completed);
    }

    #[tokio::test]
    async fn test_napas_parse_payin_webhook_valid_json() {
        let adapter = make_napas();
        let payload = serde_json::json!({
            "merchantTxnRef": "REF_PAYIN_001",
            "napasTransactionId": "NAPAS_TX_789",
            "amount": 500_000,
            "senderName": "NGUYEN VAN C",
            "senderAccount": "1111222233"
        });

        let result = adapter
            .parse_payin_webhook(payload.to_string().as_bytes(), None)
            .await
            .unwrap();

        assert_eq!(result.reference_code, "REF_PAYIN_001");
        assert_eq!(result.bank_tx_id, "NAPAS_TX_789");
        assert_eq!(result.amount_vnd, Decimal::from(500_000));
        assert_eq!(result.sender_name, Some("NGUYEN VAN C".to_string()));
        assert_eq!(result.sender_account, Some("1111222233".to_string()));
    }

    #[tokio::test]
    async fn test_napas_parse_payin_webhook_alternative_field_names() {
        let adapter = make_napas();
        let payload = serde_json::json!({
            "reference_code": "ALT_REF_001",
            "bank_tx_id": "ALT_TX_001",
            "amount": 300_000
        });

        let result = adapter
            .parse_payin_webhook(payload.to_string().as_bytes(), None)
            .await
            .unwrap();

        assert_eq!(result.reference_code, "ALT_REF_001");
        assert_eq!(result.bank_tx_id, "ALT_TX_001");
        assert_eq!(result.amount_vnd, Decimal::from(300_000));
    }

    #[tokio::test]
    async fn test_napas_parse_payin_webhook_missing_fields_use_defaults() {
        let adapter = make_napas();
        let payload = serde_json::json!({});

        let result = adapter
            .parse_payin_webhook(payload.to_string().as_bytes(), None)
            .await;

        assert!(
            result.is_err(),
            "Empty payload should be rejected after validation hardening"
        );
    }

    #[tokio::test]
    async fn test_napas_parse_payin_webhook_invalid_json() {
        let adapter = make_napas();
        let result = adapter.parse_payin_webhook(b"not valid json", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_napas_parse_payout_webhook_completed() {
        let adapter = make_napas();
        let payload = serde_json::json!({
            "eventType": "TRANSFER_COMPLETED",
            "merchantTxnRef": "PAYOUT_WH_001",
            "napasTransactionId": "NAPAS_WH_TX_001",
            "amount": 750_000,
            "status": "COMPLETED",
            "timestamp": "2026-01-15T10:30:00Z"
        });

        let result = adapter
            .parse_payout_webhook(payload.to_string().as_bytes(), None)
            .await
            .unwrap();

        assert_eq!(result.reference_code, "PAYOUT_WH_001");
        assert_eq!(result.bank_tx_id, "NAPAS_WH_TX_001");
        assert_eq!(result.status, PayoutStatus::Completed);
        assert!(result.completed_at.is_some());
        assert!(result.failure_reason.is_none());
    }

    #[tokio::test]
    async fn test_napas_parse_payout_webhook_failed() {
        let adapter = make_napas();
        let payload = serde_json::json!({
            "eventType": "TRANSFER_FAILED",
            "merchantTxnRef": "PAYOUT_FAIL_001",
            "napasTransactionId": "NAPAS_FAIL_TX",
            "amount": 200_000,
            "status": "FAILED",
            "failureReason": "Insufficient funds",
            "timestamp": "2026-01-15T10:30:00Z"
        });

        let result = adapter
            .parse_payout_webhook(payload.to_string().as_bytes(), None)
            .await
            .unwrap();

        assert_eq!(result.status, PayoutStatus::Failed);
        assert_eq!(
            result.failure_reason,
            Some("Insufficient funds".to_string())
        );
        assert!(result.completed_at.is_none());
    }

    #[tokio::test]
    async fn test_napas_parse_payout_webhook_cancelled() {
        let adapter = make_napas();
        let payload = serde_json::json!({
            "eventType": "TRANSFER_CANCELLED",
            "merchantTxnRef": "PAYOUT_CXL_001",
            "napasTransactionId": "NAPAS_CXL_TX",
            "amount": 100_000,
            "status": "CANCELLED",
            "timestamp": "2026-01-15T10:30:00Z"
        });

        let result = adapter
            .parse_payout_webhook(payload.to_string().as_bytes(), None)
            .await
            .unwrap();

        assert_eq!(result.status, PayoutStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_napas_parse_payout_webhook_invalid_payload() {
        let adapter = make_napas();
        let result = adapter.parse_payout_webhook(b"invalid", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_napas_health_check_simulation() {
        let adapter = make_napas();
        let result = adapter.health_check().await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_napas_parse_payin_webhook_real_mode_requires_signature() {
        let config = NapasConfig {
            enable_real_api: true,
            ..NapasConfig::default()
        };
        let adapter = NapasAdapter::with_config(config).unwrap();
        let payload = serde_json::json!({"amount": 100}).to_string();

        let result = adapter.parse_payin_webhook(payload.as_bytes(), None).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Missing webhook signature"));
    }
}

// ============================================================================
// VietQR Adapter Tests
// ============================================================================

mod vietqr_tests {
    use super::*;

    fn make_vietqr() -> VietQRAdapter {
        VietQRAdapter::new("vietqr_test", "vietqr_webhook_secret").unwrap()
    }

    fn make_vietqr_with_config() -> VietQRAdapter {
        let config = VietQRConfig {
            base: AdapterConfig {
                provider_code: "vietqr_configured".to_string(),
                api_base_url: "https://api.vietqr.io".to_string(),
                api_key: "test_key".to_string(),
                api_secret: "test_secret".to_string(),
                webhook_secret: "test_webhook".to_string(),
                timeout_secs: 30,
                extra: serde_json::json!({}),
            },
            client_id: Some("test_client".to_string()),
            merchant_account_number: "9876543210".to_string(),
            merchant_bank_bin: "970407".to_string(),
            merchant_name: "Test Merchant".to_string(),
            enable_real_api: false,
        };
        VietQRAdapter::with_config(config).unwrap()
    }

    #[test]
    fn test_vietqr_new_creates_adapter() {
        let adapter = make_vietqr();
        assert_eq!(adapter.provider_code(), "vietqr_test");
        assert_eq!(adapter.provider_name(), "VietQR");
        assert!(adapter.is_simulation_mode());
    }

    #[test]
    fn test_vietqr_with_config_uses_custom_provider_code() {
        let adapter = make_vietqr_with_config();
        assert_eq!(adapter.provider_code(), "vietqr_configured");
    }

    #[tokio::test]
    async fn test_vietqr_create_payin_instruction_generates_qr() {
        let adapter = make_vietqr();
        let request = CreatePayinInstructionRequest {
            reference_code: "VQRPAY001".to_string(),
            user_id: "user_vqr".to_string(),
            amount_vnd: Decimal::from(200_000),
            expires_at: Utc::now() + Duration::hours(1),
            metadata: serde_json::json!({}),
        };

        let result = adapter.create_payin_instruction(request).await.unwrap();
        assert_eq!(result.reference_code, "VQRPAY001");
        assert_eq!(result.bank_code, "VIETQR");
        assert_eq!(result.amount_vnd, Decimal::from(200_000));

        // The instructions should contain QR data
        let instructions: serde_json::Value = serde_json::from_str(&result.instructions).unwrap();
        assert_eq!(instructions["type"], "vietqr");
        assert!(instructions["qr_image_base64"].as_str().unwrap().len() > 0);
        assert!(instructions["qr_content"].as_str().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_vietqr_create_payin_with_merchant_account() {
        let adapter = make_vietqr_with_config();
        let request = CreatePayinInstructionRequest {
            reference_code: "VQRPAY002".to_string(),
            user_id: "user2".to_string(),
            amount_vnd: Decimal::from(500_000),
            expires_at: Utc::now() + Duration::hours(1),
            metadata: serde_json::json!({}),
        };

        let result = adapter.create_payin_instruction(request).await.unwrap();
        assert_eq!(result.account_number, "9876543210");
        assert_eq!(result.account_name, "Test Merchant");
    }

    #[tokio::test]
    async fn test_vietqr_generate_qr_code_with_amount() {
        let adapter = make_vietqr();
        let qr = adapter
            .generate_qr_code(
                "1234567890",
                Some(Decimal::from(100_000)),
                "Test payment",
                "REF_QR_001",
            )
            .await
            .unwrap();

        assert!(!qr.image_base64.is_empty());
        assert!(!qr.qr_content.is_empty());
        assert_eq!(qr.account_number, "1234567890");
        assert_eq!(qr.amount_vnd, Some(Decimal::from(100_000)));
        assert!(qr.description.contains("REF_QR_001"));
    }

    #[tokio::test]
    async fn test_vietqr_generate_qr_code_without_amount() {
        let adapter = make_vietqr();
        let qr = adapter
            .generate_qr_code("1234567890", None, "Open amount", "REF_QR_002")
            .await
            .unwrap();

        assert!(!qr.image_base64.is_empty());
        assert_eq!(qr.amount_vnd, None);
    }

    #[tokio::test]
    async fn test_vietqr_generate_qr_code_empty_description_uses_reference() {
        let adapter = make_vietqr();
        let qr = adapter
            .generate_qr_code("1234567890", Some(Decimal::from(50_000)), "", "REF003")
            .await
            .unwrap();

        assert!(qr.description.contains("REF003"));
        assert!(qr.description.contains("RAMPOS"));
    }

    #[tokio::test]
    async fn test_vietqr_qr_content_contains_emvco_markers() {
        let adapter = make_vietqr();
        let qr = adapter
            .generate_qr_code("1234567890", Some(Decimal::from(100_000)), "Test", "REF")
            .await
            .unwrap();

        // EMVCo markers
        assert!(qr.qr_content.starts_with("000201")); // Payload Format Indicator
        assert!(qr.qr_content.contains("5303704")); // VND currency
        assert!(qr.qr_content.contains("5802VN")); // Vietnam country code
    }

    #[tokio::test]
    async fn test_vietqr_list_supported_banks() {
        let adapter = make_vietqr();
        let banks = adapter.list_supported_banks().await.unwrap();

        assert!(!banks.is_empty());
        // Check that well-known banks are present
        let codes: Vec<&str> = banks.iter().map(|b| b.code.as_str()).collect();
        assert!(codes.contains(&"VCB"));
        assert!(codes.contains(&"TCB"));
        assert!(codes.contains(&"BIDV"));
        assert!(codes.contains(&"VPB"));
    }

    #[tokio::test]
    async fn test_vietqr_list_banks_cache_returns_same_data() {
        let adapter = make_vietqr();
        let banks1 = adapter.list_supported_banks().await.unwrap();
        let banks2 = adapter.list_supported_banks().await.unwrap();

        assert_eq!(banks1.len(), banks2.len());
        assert_eq!(banks1[0].code, banks2[0].code);
    }

    #[tokio::test]
    async fn test_vietqr_get_bank_info_by_code() {
        let adapter = make_vietqr();
        let bank = adapter.get_bank_info("VCB").await.unwrap();

        assert_eq!(bank.code, "VCB");
        assert_eq!(bank.bin, "970436");
        assert!(bank.is_supported);
    }

    #[tokio::test]
    async fn test_vietqr_get_bank_info_by_bin() {
        let adapter = make_vietqr();
        let bank = adapter.get_bank_info("970407").await.unwrap();

        assert_eq!(bank.code, "TCB");
    }

    #[tokio::test]
    async fn test_vietqr_get_bank_info_case_insensitive() {
        let adapter = make_vietqr();
        let bank = adapter.get_bank_info("vcb").await.unwrap();
        assert_eq!(bank.code, "VCB");
    }

    #[tokio::test]
    async fn test_vietqr_get_bank_info_not_found() {
        let adapter = make_vietqr();
        let result = adapter.get_bank_info("NONEXISTENT").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vietqr_payout_not_supported() {
        let adapter = make_vietqr();
        let request = InitiatePayoutRequest {
            reference_code: "PAYOUT".to_string(),
            amount_vnd: Decimal::from(100_000),
            recipient_bank_code: "VCB".to_string(),
            recipient_account_number: "123".to_string(),
            recipient_account_name: "Test".to_string(),
            description: "Test".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = adapter.initiate_payout(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vietqr_check_payout_status_not_supported() {
        let adapter = make_vietqr();
        let result = adapter.check_payout_status("REF").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vietqr_parse_payout_webhook_not_supported() {
        let adapter = make_vietqr();
        let result = adapter.parse_payout_webhook(b"{}", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vietqr_parse_payin_webhook_transaction_remarks() {
        let adapter = make_vietqr();
        let payload = serde_json::json!({
            "transactionRemarks": "RAMPOS REF_VQR_001",
            "transactionId": "FT12345678",
            "amount": 150_000,
            "senderName": "LE THI D",
            "senderAccount": "5555666677"
        });

        let result = adapter
            .parse_payin_webhook(payload.to_string().as_bytes(), None)
            .await
            .unwrap();

        assert_eq!(result.reference_code, "RAMPOS REF_VQR_001");
        assert_eq!(result.bank_tx_id, "FT12345678");
        assert_eq!(result.amount_vnd, Decimal::from(150_000));
        assert_eq!(result.sender_name, Some("LE THI D".to_string()));
    }

    #[tokio::test]
    async fn test_vietqr_parse_payin_webhook_alternative_field_names() {
        let adapter = make_vietqr();
        let payload = serde_json::json!({
            "description": "ALT_DESC",
            "ftCode": "FT_ALT_001",
            "creditAmount": 250_000,
            "sender_name": "ALT SENDER",
            "sender_account": "ALT_ACC"
        });

        let result = adapter
            .parse_payin_webhook(payload.to_string().as_bytes(), None)
            .await
            .unwrap();

        assert_eq!(result.reference_code, "ALT_DESC");
        assert_eq!(result.bank_tx_id, "FT_ALT_001");
        assert_eq!(result.amount_vnd, Decimal::from(250_000));
    }

    #[tokio::test]
    async fn test_vietqr_parse_payin_webhook_invalid_json() {
        let adapter = make_vietqr();
        let result = adapter.parse_payin_webhook(b"not json", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vietqr_parse_payin_webhook_real_mode_requires_signature() {
        let config = VietQRConfig {
            enable_real_api: true,
            ..VietQRConfig::default()
        };
        let adapter = VietQRAdapter::with_config(config).unwrap();
        let payload = serde_json::json!({"amount": 100}).to_string();

        let result = adapter.parse_payin_webhook(payload.as_bytes(), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vietqr_health_check_simulation() {
        let adapter = make_vietqr();
        let result = adapter.health_check().await.unwrap();
        assert!(result);
    }
}

// ============================================================================
// MockAdapter Tests
// ============================================================================

mod mock_adapter_tests {
    use super::*;

    fn make_mock() -> MockAdapter {
        MockAdapter::new("mock_test", "mock_secret")
    }

    #[test]
    fn test_mock_provider_info() {
        let adapter = make_mock();
        assert_eq!(adapter.provider_code(), "mock_test");
        assert_eq!(adapter.provider_name(), "Mock Bank");
        assert!(adapter.is_simulation_mode());
    }

    #[tokio::test]
    async fn test_mock_create_payin_and_retrieve() {
        let adapter = make_mock();
        let request = CreatePayinInstructionRequest {
            reference_code: "MOCK_REF_001".to_string(),
            user_id: "mock_user".to_string(),
            amount_vnd: Decimal::from(300_000),
            expires_at: Utc::now() + Duration::hours(1),
            metadata: serde_json::json!({"test": true}),
        };

        let result = adapter.create_payin_instruction(request).await.unwrap();
        assert_eq!(result.reference_code, "MOCK_REF_001");
        assert_eq!(result.bank_code, "MOCK");
        assert!(result.account_number.starts_with("VA"));
        assert_eq!(result.amount_vnd, Decimal::from(300_000));

        // Verify stored
        let stored = adapter.get_payin_instruction("MOCK_REF_001").await;
        assert!(stored.is_some());
        let stored = stored.unwrap();
        assert_eq!(stored.reference_code, "MOCK_REF_001");
    }

    #[tokio::test]
    async fn test_mock_initiate_payout_and_retrieve() {
        let adapter = make_mock();
        let request = InitiatePayoutRequest {
            reference_code: "MOCK_PAYOUT_001".to_string(),
            amount_vnd: Decimal::from(500_000),
            recipient_bank_code: "VCB".to_string(),
            recipient_account_number: "1234567890".to_string(),
            recipient_account_name: "RECEIVER".to_string(),
            description: "Test payout".to_string(),
            metadata: serde_json::json!({}),
        };

        let result = adapter.initiate_payout(request).await.unwrap();
        assert_eq!(result.reference_code, "MOCK_PAYOUT_001");
        assert!(result.provider_tx_id.starts_with("MOCK_"));
        assert_eq!(result.status, PayoutStatus::Processing);
        assert!(result.estimated_completion.is_some());

        // Verify stored
        let stored = adapter.get_payout("MOCK_PAYOUT_001").await;
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn test_mock_check_payout_status_default() {
        let adapter = make_mock();
        let status = adapter.check_payout_status("ANY").await.unwrap();
        assert_eq!(status, PayoutStatus::Completed);
    }

    #[tokio::test]
    async fn test_mock_set_and_check_payout_status() {
        let adapter = make_mock();

        adapter
            .set_payout_status("REF_STATUS", PayoutStatus::Failed)
            .await;
        let status = adapter.check_payout_status("REF_STATUS").await.unwrap();
        assert_eq!(status, PayoutStatus::Failed);

        adapter
            .set_payout_status("REF_STATUS", PayoutStatus::Completed)
            .await;
        let status = adapter.check_payout_status("REF_STATUS").await.unwrap();
        assert_eq!(status, PayoutStatus::Completed);
    }

    #[tokio::test]
    async fn test_mock_parse_payin_webhook() {
        let adapter = make_mock();
        let payload =
            MockAdapter::create_payin_webhook_payload("MOCK_WH_001", 123_456, "BANK_TX_001");

        let result = adapter.parse_payin_webhook(&payload, None).await.unwrap();

        assert_eq!(result.reference_code, "MOCK_WH_001");
        assert_eq!(result.bank_tx_id, "BANK_TX_001");
        assert_eq!(result.amount_vnd, Decimal::from(123_456));
        assert_eq!(result.sender_name, Some("MOCK SENDER".to_string()));
        assert_eq!(result.sender_account, Some("1234567890".to_string()));
    }

    #[tokio::test]
    async fn test_mock_parse_payout_webhook_completed() {
        let adapter = make_mock();
        let payload =
            MockAdapter::create_payout_webhook_payload("MOCK_PO_001", "completed", "TX_PO_001");

        let result = adapter.parse_payout_webhook(&payload, None).await.unwrap();

        assert_eq!(result.reference_code, "MOCK_PO_001");
        assert_eq!(result.bank_tx_id, "TX_PO_001");
        assert_eq!(result.status, PayoutStatus::Completed);
        assert!(result.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_mock_parse_payout_webhook_failed() {
        let adapter = make_mock();
        let payload =
            MockAdapter::create_payout_webhook_payload("MOCK_PO_002", "failed", "TX_PO_002");

        let result = adapter.parse_payout_webhook(&payload, None).await.unwrap();

        assert_eq!(result.status, PayoutStatus::Failed);
        assert!(result.completed_at.is_none());
    }

    #[tokio::test]
    async fn test_mock_parse_payout_webhook_cancelled() {
        let adapter = make_mock();
        let payload =
            MockAdapter::create_payout_webhook_payload("MOCK_PO_003", "cancelled", "TX_PO_003");

        let result = adapter.parse_payout_webhook(&payload, None).await.unwrap();

        assert_eq!(result.status, PayoutStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_mock_parse_payout_webhook_unknown_status() {
        let adapter = make_mock();
        let payload =
            MockAdapter::create_payout_webhook_payload("MOCK_PO_004", "something_else", "TX_004");

        let result = adapter.parse_payout_webhook(&payload, None).await.unwrap();

        assert_eq!(result.status, PayoutStatus::Processing);
    }

    #[tokio::test]
    async fn test_mock_with_custom_behavior() {
        let behavior = MockBehavior {
            response_delay_ms: 0,
            simulate_failures: false,
            failure_rate: 0.0,
            default_payout_status: PayoutStatus::Pending,
        };
        let adapter = MockAdapter::with_behavior("custom_mock", "secret", behavior);
        assert_eq!(adapter.provider_code(), "custom_mock");

        let status = adapter.check_payout_status("ANY").await.unwrap();
        assert_eq!(status, PayoutStatus::Pending);
    }

    #[tokio::test]
    async fn test_mock_health_check() {
        let adapter = make_mock();
        assert!(adapter.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_parse_webhook_invalid_json() {
        let adapter = make_mock();
        let result = adapter.parse_payin_webhook(b"invalid", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_get_nonexistent_instruction() {
        let adapter = make_mock();
        let result = adapter.get_payin_instruction("DOESNT_EXIST").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_mock_get_nonexistent_payout() {
        let adapter = make_mock();
        let result = adapter.get_payout("DOESNT_EXIST").await;
        assert!(result.is_none());
    }
}

// ============================================================================
// AdapterFactory Tests
// ============================================================================

mod factory_tests {
    use super::*;

    #[test]
    fn test_factory_new_registers_builtin_adapters() {
        let factory = AdapterFactory::new().unwrap();
        let types = factory.list_types();
        assert!(types.contains(&"mock".to_string()));
        assert!(types.contains(&"vietqr".to_string()));
        assert!(types.contains(&"napas".to_string()));
    }

    #[test]
    fn test_factory_is_registered() {
        let factory = AdapterFactory::new().unwrap();
        assert!(factory.is_registered("mock"));
        assert!(factory.is_registered("vietqr"));
        assert!(factory.is_registered("napas"));
        assert!(!factory.is_registered("unknown"));
    }

    #[test]
    fn test_factory_is_registered_case_insensitive() {
        let factory = AdapterFactory::new().unwrap();
        assert!(factory.is_registered("MOCK"));
        assert!(factory.is_registered("Mock"));
        assert!(factory.is_registered("VIETQR"));
        assert!(factory.is_registered("Napas"));
    }

    #[test]
    fn test_factory_create_mock() {
        let factory = AdapterFactory::new().unwrap();
        let config = AdapterConfig {
            provider_code: "mock_from_factory".to_string(),
            api_base_url: "http://localhost".to_string(),
            api_key: "key".to_string(),
            api_secret: "secret".to_string(),
            webhook_secret: "wh_secret".to_string(),
            timeout_secs: 30,
            extra: serde_json::json!({}),
        };

        let adapter = factory.create("mock", config).unwrap();
        assert_eq!(adapter.provider_code(), "mock_from_factory");
        assert_eq!(adapter.provider_name(), "Mock Bank");
        assert!(adapter.is_simulation_mode());
    }

    #[test]
    fn test_factory_create_vietqr_basic() {
        let factory = AdapterFactory::new().unwrap();
        let config = AdapterConfig {
            provider_code: "vietqr_basic".to_string(),
            api_base_url: "https://api.vietqr.io".to_string(),
            api_key: "key".to_string(),
            api_secret: "secret".to_string(),
            webhook_secret: "wh_secret".to_string(),
            timeout_secs: 30,
            extra: serde_json::json!({}),
        };

        let adapter = factory.create("vietqr", config).unwrap();
        assert_eq!(adapter.provider_code(), "vietqr_basic");
        assert_eq!(adapter.provider_name(), "VietQR");
    }

    #[test]
    fn test_factory_create_napas_basic() {
        let factory = AdapterFactory::new().unwrap();
        let config = AdapterConfig {
            provider_code: "napas_basic".to_string(),
            api_base_url: "https://api.napas.com.vn".to_string(),
            api_key: "key".to_string(),
            api_secret: "secret".to_string(),
            webhook_secret: "wh_secret".to_string(),
            timeout_secs: 30,
            extra: serde_json::json!({}),
        };

        let adapter = factory.create("napas", config).unwrap();
        assert_eq!(adapter.provider_code(), "napas_basic");
        assert_eq!(adapter.provider_name(), "Napas");
    }

    #[test]
    fn test_factory_create_unknown_type_error() {
        let factory = AdapterFactory::new().unwrap();
        let config = AdapterConfig {
            provider_code: "test".to_string(),
            api_base_url: "http://localhost".to_string(),
            api_key: "key".to_string(),
            api_secret: "secret".to_string(),
            webhook_secret: "secret".to_string(),
            timeout_secs: 30,
            extra: serde_json::json!({}),
        };

        let result = factory.create("nonexistent", config);
        assert!(result.is_err());
    }

    #[test]
    fn test_factory_create_vietqr_from_json() {
        let factory = AdapterFactory::new().unwrap();
        let config = serde_json::json!({
            "provider_code": "vietqr_json",
            "api_base_url": "https://api.vietqr.io",
            "api_key": "test_key",
            "api_secret": "test_secret",
            "webhook_secret": "wh_secret",
            "timeout_secs": 30,
            "extra": {},
            "merchant_account_number": "1234567890",
            "merchant_bank_bin": "970436",
            "merchant_name": "JSON Merchant",
            "enable_real_api": false
        });

        let adapter = factory.create_from_json("vietqr", config).unwrap();
        assert_eq!(adapter.provider_code(), "vietqr_json");
        assert!(adapter.is_simulation_mode());
    }

    #[test]
    fn test_factory_create_napas_from_json() {
        let factory = AdapterFactory::new().unwrap();
        let config = serde_json::json!({
            "provider_code": "napas_json",
            "api_base_url": "https://api.napas.com.vn",
            "api_key": "test_key",
            "api_secret": "test_secret",
            "webhook_secret": "wh_secret",
            "timeout_secs": 30,
            "extra": {},
            "merchant_id": "MID001",
            "terminal_id": "TID001",
            "partner_code": "PC001",
            "enable_real_api": false
        });

        let adapter = factory.create_from_json("napas", config).unwrap();
        assert_eq!(adapter.provider_code(), "napas_json");
    }

    #[test]
    fn test_factory_create_from_json_falls_back_to_basic() {
        let factory = AdapterFactory::new().unwrap();
        let config = serde_json::json!({
            "provider_code": "mock_fallback",
            "api_base_url": "http://localhost",
            "api_key": "key",
            "api_secret": "secret",
            "webhook_secret": "secret",
            "timeout_secs": 30,
            "extra": {}
        });

        let adapter = factory.create_from_json("mock", config).unwrap();
        assert_eq!(adapter.provider_code(), "mock_fallback");
    }

    #[test]
    fn test_factory_register_custom_adapter() {
        let factory = AdapterFactory::new().unwrap();
        factory
            .register("custom", |config| {
                Ok(Box::new(MockAdapter::new(
                    config.provider_code,
                    config.webhook_secret,
                )))
            })
            .unwrap();

        assert!(factory.is_registered("custom"));
    }

    #[test]
    fn test_factory_create_test_adapters() {
        let adapters = create_test_adapters();
        assert!(adapters.contains_key("mock"));
        assert!(adapters.contains_key("vietqr"));
        assert!(adapters.contains_key("napas"));
        assert_eq!(adapters.len(), 3);
    }

    #[test]
    fn test_factory_default_creates_factory() {
        let factory = AdapterFactory::default();
        assert!(factory.is_registered("mock"));
    }

    #[test]
    fn test_factory_create_from_config_map() {
        let factory = AdapterFactory::new().unwrap();
        let mut config_map = HashMap::new();
        config_map.insert(
            "my_mock".to_string(),
            serde_json::json!({
                "adapter_type": "mock",
                "provider_code": "my_mock",
                "api_base_url": "http://localhost",
                "api_key": "key",
                "api_secret": "secret",
                "webhook_secret": "secret",
                "timeout_secs": 30,
                "extra": {}
            }),
        );

        let adapters = factory.create_from_config_map(&config_map).unwrap();
        assert!(adapters.contains_key("my_mock"));
    }
}

// ============================================================================
// eKYC Tests
// ============================================================================

mod ekyc_tests {
    use super::*;

    fn make_mock_ekyc() -> MockEkycProvider {
        MockEkycProvider::with_defaults()
    }

    // --- MockEkycProvider tests ---

    #[tokio::test]
    async fn test_mock_ekyc_verify_id_cccd() {
        let provider = make_mock_ekyc();
        let request = IdVerificationRequest {
            id_front_image: vec![0u8; 100],
            id_back_image: Some(vec![0u8; 100]),
            document_type: IdDocumentType::Cccd,
            request_id: "ekyc_test_001".to_string(),
        };

        let result = provider.verify_id(request).await.unwrap();
        assert!(!result.verification_id.is_empty());
        assert!(result.full_name.is_some());
        assert!(result.id_number.is_some());
        assert_eq!(result.document_type, IdDocumentType::Cccd);
        assert!(result.confidence_score >= 0.0 && result.confidence_score <= 1.0);
    }

    #[tokio::test]
    async fn test_mock_ekyc_verify_id_passport() {
        let provider = make_mock_ekyc();
        let request = IdVerificationRequest {
            id_front_image: vec![0u8; 100],
            id_back_image: None,
            document_type: IdDocumentType::Passport,
            request_id: "ekyc_passport_001".to_string(),
        };

        let result = provider.verify_id(request).await.unwrap();
        assert!(result.full_name.is_some());
        assert_eq!(result.document_type, IdDocumentType::Passport);
    }

    #[tokio::test]
    async fn test_mock_ekyc_verify_id_stores_result() {
        let provider = make_mock_ekyc();
        let request = IdVerificationRequest {
            id_front_image: vec![0u8; 100],
            id_back_image: Some(vec![0u8; 100]),
            document_type: IdDocumentType::Cccd,
            request_id: "store_test".to_string(),
        };

        let result = provider.verify_id(request).await.unwrap();
        let stored = provider.get_id_verification(&result.verification_id).await;
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn test_mock_ekyc_face_matching() {
        let provider = make_mock_ekyc();
        let request = FaceMatchRequest {
            selfie_image: vec![1u8; 100],
            id_photo_image: vec![2u8; 100],
            request_id: "face_test_001".to_string(),
        };

        let result = provider.match_face(request).await.unwrap();
        assert!(!result.match_id.is_empty());
        assert!(result.similarity_score >= 0.0 && result.similarity_score <= 1.0);
    }

    #[tokio::test]
    async fn test_mock_ekyc_face_match_stores_result() {
        let provider = make_mock_ekyc();
        let request = FaceMatchRequest {
            selfie_image: vec![1u8; 100],
            id_photo_image: vec![2u8; 100],
            request_id: "store_face".to_string(),
        };

        let result = provider.match_face(request).await.unwrap();
        let stored = provider.get_face_match(&result.match_id).await;
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn test_mock_ekyc_liveness_active() {
        let provider = make_mock_ekyc();
        let request = LivenessRequest {
            video_data: vec![0u8; 200],
            request_id: "liveness_test_001".to_string(),
            check_type: LivenessCheckType::Active,
        };

        let result = provider.check_liveness(request).await.unwrap();
        assert!(!result.liveness_id.is_empty());
        assert!(result.liveness_score >= 0.0 && result.liveness_score <= 1.0);
    }

    #[tokio::test]
    async fn test_mock_ekyc_liveness_passive() {
        let provider = make_mock_ekyc();
        let request = LivenessRequest {
            video_data: vec![0u8; 200],
            request_id: "liveness_passive".to_string(),
            check_type: LivenessCheckType::Passive,
        };

        let result = provider.check_liveness(request).await.unwrap();
        assert!(!result.liveness_id.is_empty());
    }

    #[tokio::test]
    async fn test_mock_ekyc_liveness_stores_result() {
        let provider = make_mock_ekyc();
        let request = LivenessRequest {
            video_data: vec![0u8; 200],
            request_id: "store_liveness".to_string(),
            check_type: LivenessCheckType::Video,
        };

        let result = provider.check_liveness(request).await.unwrap();
        let stored = provider.get_liveness_result(&result.liveness_id).await;
        assert!(stored.is_some());
    }

    #[tokio::test]
    async fn test_mock_ekyc_address_verification_valid() {
        let provider = make_mock_ekyc();
        let request = AddressVerificationRequest {
            address: "123 Duong ABC, Quan Hoan Kiem, Ha Noi, Vietnam".to_string(),
            province: Some("Ha Noi".to_string()),
            district: Some("Hoan Kiem".to_string()),
            ward: Some("Phuc Tan".to_string()),
            request_id: "addr_test_001".to_string(),
        };

        let result = provider.verify_address(request).await.unwrap();
        assert!(result.is_valid);
        assert!(result.normalized_address.is_some());
        assert!(result.province_code.is_some());
        assert!(result.district_code.is_some());
        assert!(result.ward_code.is_some());
    }

    #[tokio::test]
    async fn test_mock_ekyc_address_verification_short_invalid() {
        let provider = make_mock_ekyc();
        let request = AddressVerificationRequest {
            address: "short".to_string(),
            province: None,
            district: None,
            ward: None,
            request_id: "addr_short".to_string(),
        };

        let result = provider.verify_address(request).await.unwrap();
        assert!(!result.is_valid);
        assert!(result.normalized_address.is_none());
        assert!(result.error_message.is_some());
    }

    #[tokio::test]
    async fn test_mock_ekyc_blocked_id() {
        let behavior = MockEkycBehavior {
            blocked_ids: vec!["001234567890".to_string()],
            ..Default::default()
        };
        let provider = MockEkycProvider::with_behavior(EkycProviderConfig::default(), behavior);

        let request = IdVerificationRequest {
            id_front_image: vec![0u8; 100],
            id_back_image: Some(vec![0u8; 100]),
            document_type: IdDocumentType::Cccd,
            request_id: "blocked_test".to_string(),
        };

        let result = provider.verify_id(request).await.unwrap();
        assert!(!result.success);
        assert!(result.error_message.is_some());
        assert!(result.error_message.unwrap().contains("blocked"));
    }

    #[tokio::test]
    async fn test_mock_ekyc_provider_info() {
        let provider = make_mock_ekyc();
        assert_eq!(provider.provider_code(), "mock_ekyc");
        assert_eq!(provider.provider_name(), "Mock eKYC Provider");
        assert!(provider.is_sandbox_mode());
    }

    #[tokio::test]
    async fn test_mock_ekyc_health_check() {
        let provider = make_mock_ekyc();
        assert!(provider.health_check().await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_ekyc_set_custom_results() {
        let provider = make_mock_ekyc();

        let custom_verification = IdVerification {
            verification_id: "custom_vid".to_string(),
            success: true,
            full_name: Some("CUSTOM NAME".to_string()),
            id_number: Some("999888777666".to_string()),
            document_type: IdDocumentType::Cccd,
            date_of_birth: None,
            gender: None,
            nationality: None,
            place_of_origin: None,
            place_of_residence: None,
            expiry_date: None,
            issue_date: None,
            issuing_authority: None,
            confidence_score: 0.99,
            field_confidences: HashMap::new(),
            error_message: None,
            raw_response: None,
            verified_at: Utc::now(),
        };

        provider
            .set_id_verification("custom_vid".to_string(), custom_verification)
            .await;

        let stored = provider.get_id_verification("custom_vid").await.unwrap();
        assert_eq!(stored.full_name, Some("CUSTOM NAME".to_string()));
    }

    // --- FaceMatchConfidence tests ---

    #[test]
    fn test_face_match_confidence_from_score() {
        assert_eq!(
            FaceMatchConfidence::from_score(0.98),
            FaceMatchConfidence::VeryHigh
        );
        assert_eq!(
            FaceMatchConfidence::from_score(0.90),
            FaceMatchConfidence::High
        );
        assert_eq!(
            FaceMatchConfidence::from_score(0.75),
            FaceMatchConfidence::Medium
        );
        assert_eq!(
            FaceMatchConfidence::from_score(0.55),
            FaceMatchConfidence::Low
        );
        assert_eq!(
            FaceMatchConfidence::from_score(0.30),
            FaceMatchConfidence::VeryLow
        );
    }

    #[test]
    fn test_face_match_confidence_boundary_values() {
        assert_eq!(
            FaceMatchConfidence::from_score(0.95),
            FaceMatchConfidence::VeryHigh
        );
        assert_eq!(
            FaceMatchConfidence::from_score(0.85),
            FaceMatchConfidence::High
        );
        assert_eq!(
            FaceMatchConfidence::from_score(0.70),
            FaceMatchConfidence::Medium
        );
        assert_eq!(
            FaceMatchConfidence::from_score(0.50),
            FaceMatchConfidence::Low
        );
        assert_eq!(
            FaceMatchConfidence::from_score(0.49),
            FaceMatchConfidence::VeryLow
        );
    }

    // --- IdDocumentType tests ---

    #[test]
    fn test_id_document_type_as_str() {
        assert_eq!(IdDocumentType::Cccd.as_str(), "CCCD");
        assert_eq!(IdDocumentType::Cmnd.as_str(), "CMND");
        assert_eq!(IdDocumentType::Passport.as_str(), "PASSPORT");
        assert_eq!(IdDocumentType::DriverLicense.as_str(), "DRIVER_LICENSE");
    }

    // --- FullEkycResult tests ---

    #[test]
    fn test_full_ekyc_result_all_pass() {
        let id_verification = IdVerification {
            verification_id: "id_001".to_string(),
            success: true,
            full_name: Some("NGUYEN VAN A".to_string()),
            id_number: Some("001234567890".to_string()),
            document_type: IdDocumentType::Cccd,
            date_of_birth: Some("01/01/1990".to_string()),
            gender: Some("Nam".to_string()),
            nationality: Some("Viet Nam".to_string()),
            place_of_origin: None,
            place_of_residence: None,
            expiry_date: None,
            issue_date: None,
            issuing_authority: None,
            confidence_score: 0.95,
            field_confidences: HashMap::new(),
            error_message: None,
            raw_response: None,
            verified_at: Utc::now(),
        };

        let face_match = FaceMatch {
            match_id: "face_001".to_string(),
            is_match: true,
            similarity_score: 0.92,
            confidence: FaceMatchConfidence::High,
            error_message: None,
            raw_response: None,
            matched_at: Utc::now(),
        };

        let liveness = LivenessResult {
            liveness_id: "live_001".to_string(),
            is_live: true,
            liveness_score: 0.97,
            spoofing_types: vec![],
            error_message: None,
            raw_response: None,
            checked_at: Utc::now(),
        };

        let result = FullEkycResult::calculate(id_verification, face_match, liveness, None);
        assert!(result.passed);
        assert!(result.overall_score > 0.9);
        assert!(result.failure_reasons.is_empty());
        assert!(result.provider_reference.contains("id_001"));
        assert!(result.provider_reference.contains("face_001"));
    }

    #[test]
    fn test_full_ekyc_result_id_verification_failed() {
        let id_verification = IdVerification {
            verification_id: "id_fail".to_string(),
            success: false,
            full_name: None,
            id_number: None,
            document_type: IdDocumentType::Cccd,
            date_of_birth: None,
            gender: None,
            nationality: None,
            place_of_origin: None,
            place_of_residence: None,
            expiry_date: None,
            issue_date: None,
            issuing_authority: None,
            confidence_score: 0.3,
            field_confidences: HashMap::new(),
            error_message: Some("Document unreadable".to_string()),
            raw_response: None,
            verified_at: Utc::now(),
        };

        let face_match = FaceMatch {
            match_id: "face_ok".to_string(),
            is_match: true,
            similarity_score: 0.92,
            confidence: FaceMatchConfidence::High,
            error_message: None,
            raw_response: None,
            matched_at: Utc::now(),
        };

        let liveness = LivenessResult {
            liveness_id: "live_ok".to_string(),
            is_live: true,
            liveness_score: 0.95,
            spoofing_types: vec![],
            error_message: None,
            raw_response: None,
            checked_at: Utc::now(),
        };

        let result = FullEkycResult::calculate(id_verification, face_match, liveness, None);
        assert!(!result.passed);
        assert!(!result.failure_reasons.is_empty());
        assert!(result.failure_reasons[0].contains("ID verification failed"));
        assert!(result.failure_reasons[0].contains("Document unreadable"));
    }

    #[test]
    fn test_full_ekyc_result_face_match_failed() {
        let id_verification = IdVerification {
            verification_id: "id_ok".to_string(),
            success: true,
            full_name: Some("A".to_string()),
            id_number: Some("123".to_string()),
            document_type: IdDocumentType::Cccd,
            date_of_birth: None,
            gender: None,
            nationality: None,
            place_of_origin: None,
            place_of_residence: None,
            expiry_date: None,
            issue_date: None,
            issuing_authority: None,
            confidence_score: 0.95,
            field_confidences: HashMap::new(),
            error_message: None,
            raw_response: None,
            verified_at: Utc::now(),
        };

        let face_match = FaceMatch {
            match_id: "face_fail".to_string(),
            is_match: false,
            similarity_score: 0.40,
            confidence: FaceMatchConfidence::VeryLow,
            error_message: None,
            raw_response: None,
            matched_at: Utc::now(),
        };

        let liveness = LivenessResult {
            liveness_id: "live_ok".to_string(),
            is_live: true,
            liveness_score: 0.95,
            spoofing_types: vec![],
            error_message: None,
            raw_response: None,
            checked_at: Utc::now(),
        };

        let result = FullEkycResult::calculate(id_verification, face_match, liveness, None);
        assert!(!result.passed);
        assert!(result
            .failure_reasons
            .iter()
            .any(|r| r.contains("Face match failed")));
    }

    #[test]
    fn test_full_ekyc_result_liveness_failed_with_spoofing() {
        let id_verification = IdVerification {
            verification_id: "id_ok".to_string(),
            success: true,
            full_name: Some("A".to_string()),
            id_number: Some("123".to_string()),
            document_type: IdDocumentType::Cccd,
            date_of_birth: None,
            gender: None,
            nationality: None,
            place_of_origin: None,
            place_of_residence: None,
            expiry_date: None,
            issue_date: None,
            issuing_authority: None,
            confidence_score: 0.95,
            field_confidences: HashMap::new(),
            error_message: None,
            raw_response: None,
            verified_at: Utc::now(),
        };

        let face_match = FaceMatch {
            match_id: "face_ok".to_string(),
            is_match: true,
            similarity_score: 0.92,
            confidence: FaceMatchConfidence::High,
            error_message: None,
            raw_response: None,
            matched_at: Utc::now(),
        };

        let liveness = LivenessResult {
            liveness_id: "live_fail".to_string(),
            is_live: false,
            liveness_score: 0.2,
            spoofing_types: vec![SpoofingType::PrintedPhoto],
            error_message: Some("Printed photo detected".to_string()),
            raw_response: None,
            checked_at: Utc::now(),
        };

        let result = FullEkycResult::calculate(id_verification, face_match, liveness, None);
        assert!(!result.passed);
        assert!(result
            .failure_reasons
            .iter()
            .any(|r| r.contains("Liveness") && r.contains("PrintedPhoto")));
    }

    #[test]
    fn test_full_ekyc_result_overall_score_calculation() {
        let id_verification = IdVerification {
            verification_id: "id".to_string(),
            success: true,
            full_name: None,
            id_number: None,
            document_type: IdDocumentType::Cccd,
            date_of_birth: None,
            gender: None,
            nationality: None,
            place_of_origin: None,
            place_of_residence: None,
            expiry_date: None,
            issue_date: None,
            issuing_authority: None,
            confidence_score: 1.0, // 0.3 weight
            field_confidences: HashMap::new(),
            error_message: None,
            raw_response: None,
            verified_at: Utc::now(),
        };

        let face_match = FaceMatch {
            match_id: "face".to_string(),
            is_match: true,
            similarity_score: 1.0, // 0.4 weight
            confidence: FaceMatchConfidence::VeryHigh,
            error_message: None,
            raw_response: None,
            matched_at: Utc::now(),
        };

        let liveness = LivenessResult {
            liveness_id: "live".to_string(),
            is_live: true,
            liveness_score: 1.0, // 0.3 weight
            spoofing_types: vec![],
            error_message: None,
            raw_response: None,
            checked_at: Utc::now(),
        };

        let result = FullEkycResult::calculate(id_verification, face_match, liveness, None);
        // 1.0 * 0.3 + 1.0 * 0.4 + 1.0 * 0.3 = 1.0
        assert!((result.overall_score - 1.0).abs() < 0.001);
    }
}

// ============================================================================
// Types Tests
// ============================================================================

mod types_tests {
    use super::*;

    #[test]
    fn test_payout_status_serialization() {
        let status = PayoutStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"Completed\"");

        let deserialized: PayoutStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, PayoutStatus::Completed);
    }

    #[test]
    fn test_payout_status_all_variants() {
        let statuses = vec![
            PayoutStatus::Pending,
            PayoutStatus::Processing,
            PayoutStatus::Completed,
            PayoutStatus::Failed,
            PayoutStatus::Cancelled,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let back: PayoutStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    #[test]
    fn test_vietqr_config_default() {
        let config = VietQRConfig::default();
        assert_eq!(config.base.provider_code, "vietqr");
        assert_eq!(config.base.api_base_url, "https://api.vietqr.io");
        assert_eq!(config.merchant_name, "RampOS");
        assert!(!config.enable_real_api);
    }

    #[test]
    fn test_napas_config_default() {
        let config = NapasConfig::default();
        assert_eq!(config.base.provider_code, "napas");
        assert_eq!(config.base.api_base_url, "https://api.napas.com.vn");
        assert!(!config.enable_real_api);
        assert!(config.private_key_pem.is_none());
        assert!(config.napas_public_key_pem.is_none());
    }

    #[test]
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_retry_delay_ms, 100);
        assert_eq!(config.max_retry_delay_ms, 5000);
        assert!(config.user_agent.contains("RampOS"));
    }

    #[test]
    fn test_adapter_config_serialization() {
        let config = AdapterConfig {
            provider_code: "test".to_string(),
            api_base_url: "https://api.example.com".to_string(),
            api_key: "key123".to_string(),
            api_secret: "secret456".to_string(),
            webhook_secret: "whsec789".to_string(),
            timeout_secs: 60,
            extra: serde_json::json!({"custom": "value"}),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AdapterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.provider_code, "test");
        assert_eq!(deserialized.timeout_secs, 60);
    }

    #[test]
    fn test_payin_instruction_serialization() {
        let instruction = PayinInstruction {
            reference_code: "REF001".to_string(),
            bank_code: "VCB".to_string(),
            account_number: "1234567890".to_string(),
            account_name: "TEST ACCOUNT".to_string(),
            amount_vnd: Decimal::from(100_000),
            expires_at: Utc::now(),
            instructions: "Transfer to account".to_string(),
        };

        let json = serde_json::to_string(&instruction).unwrap();
        let back: PayinInstruction = serde_json::from_str(&json).unwrap();
        assert_eq!(back.reference_code, "REF001");
        assert_eq!(back.amount_vnd, Decimal::from(100_000));
    }

    #[test]
    fn test_qr_code_data_serialization() {
        let qr = QrCodeData {
            image_base64: "base64data".to_string(),
            qr_content: "qrcontent".to_string(),
            bank_bin: "970436".to_string(),
            account_number: "1234567890".to_string(),
            amount_vnd: Some(Decimal::from(50_000)),
            description: "Test QR".to_string(),
            expires_at: None,
        };

        let json = serde_json::to_string(&qr).unwrap();
        let back: QrCodeData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.bank_bin, "970436");
        assert_eq!(back.amount_vnd, Some(Decimal::from(50_000)));
    }

    #[test]
    fn test_vietqr_bank_info_serialization() {
        let bank = VietQRBankInfo {
            code: "VCB".to_string(),
            bin: "970436".to_string(),
            name: "Vietcombank".to_string(),
            short_name: "VCB".to_string(),
            is_supported: true,
        };

        let json = serde_json::to_string(&bank).unwrap();
        let back: VietQRBankInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.code, "VCB");
        assert!(back.is_supported);
    }

    #[test]
    fn test_ekyc_provider_config_default() {
        let config = EkycProviderConfig::default();
        assert_eq!(config.provider_code, "mock");
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
        assert!(config.sandbox_mode);
    }

    #[test]
    fn test_create_payin_instruction_request_serialization() {
        let request = CreatePayinInstructionRequest {
            reference_code: "REQ001".to_string(),
            user_id: "user_001".to_string(),
            amount_vnd: Decimal::from(750_000),
            expires_at: Utc::now(),
            metadata: serde_json::json!({"intent_id": "INT001"}),
        };

        let json = serde_json::to_string(&request).unwrap();
        let back: CreatePayinInstructionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.reference_code, "REQ001");
        assert_eq!(back.user_id, "user_001");
    }

    #[test]
    fn test_initiate_payout_request_serialization() {
        let request = InitiatePayoutRequest {
            reference_code: "PO001".to_string(),
            amount_vnd: Decimal::from(1_500_000),
            recipient_bank_code: "970436".to_string(),
            recipient_account_number: "0987654321".to_string(),
            recipient_account_name: "RECEIVER NAME".to_string(),
            description: "Payout test".to_string(),
            metadata: serde_json::json!({}),
        };

        let json = serde_json::to_string(&request).unwrap();
        let back: InitiatePayoutRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.reference_code, "PO001");
        assert_eq!(back.recipient_bank_code, "970436");
    }

    #[test]
    fn test_virtual_account_types_serialization() {
        let va_request = CreateVirtualAccountRequest {
            user_id: "user1".to_string(),
            user_name: "User One".to_string(),
            expires_at: Some(Utc::now()),
            metadata: serde_json::json!({}),
        };
        let json = serde_json::to_string(&va_request).unwrap();
        let back: CreateVirtualAccountRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.user_id, "user1");

        let va_info = VirtualAccountInfo {
            bank_code: "VCB".to_string(),
            account_number: "VA123".to_string(),
            account_name: "VA USER".to_string(),
            is_active: true,
            created_at: Utc::now(),
            expires_at: None,
        };
        let json = serde_json::to_string(&va_info).unwrap();
        let back: VirtualAccountInfo = serde_json::from_str(&json).unwrap();
        assert!(back.is_active);
    }
}
