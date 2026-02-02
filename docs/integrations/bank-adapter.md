# Bank Adapter Integration Guide

This guide explains how to implement a custom bank adapter for RampOS. Bank adapters allow RampOS to integrate with different banking partners and payment service providers (PSPs) for fiat on-ramp and off-ramp operations.

## Overview

The `RailsAdapter` trait is the core interface for bank integrations. It defines methods for:

- Creating pay-in instructions (virtual accounts, QR codes, etc.)
- Parsing incoming pay-in webhooks
- Initiating pay-outs
- Parsing pay-out webhooks
- Checking pay-out status
- Verifying webhook signatures

## Architecture

```
                                RampOS Core
                                     |
                    +----------------+----------------+
                    |                                 |
              RailsAdapter                    VirtualAccountAdapter
              (required)                         (optional)
                    |                                 |
        +-----------+-----------+                     |
        |           |           |                     |
    MockBank    BankA       BankB              InstantTransfer
    Adapter    Adapter     Adapter                Adapter
```

## RailsAdapter Trait

The core trait that all bank adapters must implement:

```rust
use async_trait::async_trait;
use ramp_common::Result;

#[async_trait]
pub trait RailsAdapter: Send + Sync {
    /// Get adapter identifier (e.g., "vietcombank", "momo", "vnpay")
    fn provider_code(&self) -> &str;

    /// Get human-readable adapter name
    fn provider_name(&self) -> &str;

    /// Create pay-in instruction (e.g., virtual account, QR code)
    async fn create_payin_instruction(
        &self,
        request: CreatePayinInstructionRequest,
    ) -> Result<PayinInstruction>;

    /// Parse incoming pay-in webhook from the bank
    async fn parse_payin_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<PayinConfirmation>;

    /// Initiate a pay-out to a recipient
    async fn initiate_payout(
        &self,
        request: InitiatePayoutRequest,
    ) -> Result<PayoutResult>;

    /// Parse incoming pay-out webhook
    async fn parse_payout_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<PayoutConfirmation>;

    /// Check pay-out status by reference
    async fn check_payout_status(
        &self,
        reference: &str,
    ) -> Result<PayoutStatus>;

    /// Verify webhook signature
    fn verify_webhook_signature(
        &self,
        payload: &[u8],
        signature: &str,
    ) -> bool;
}
```

## Data Types

### CreatePayinInstructionRequest

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePayinInstructionRequest {
    /// Unique reference code for this pay-in
    pub reference_code: String,
    /// User ID for tracking
    pub user_id: String,
    /// Amount in VND
    pub amount_vnd: Decimal,
    /// When the instruction expires
    pub expires_at: DateTime<Utc>,
    /// Additional metadata
    pub metadata: serde_json::Value,
}
```

### PayinInstruction

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayinInstruction {
    /// Reference code (same as request)
    pub reference_code: String,
    /// Bank code (e.g., "VCB", "TCB")
    pub bank_code: String,
    /// Virtual account or destination account number
    pub account_number: String,
    /// Account name to display
    pub account_name: String,
    /// Amount in VND
    pub amount_vnd: Decimal,
    /// When the instruction expires
    pub expires_at: DateTime<Utc>,
    /// Human-readable instructions for the user
    pub instructions: String,
}
```

### PayinConfirmation

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayinConfirmation {
    /// Reference code from the original instruction
    pub reference_code: String,
    /// Bank's transaction ID
    pub bank_tx_id: String,
    /// Actual amount received in VND
    pub amount_vnd: Decimal,
    /// Sender's name (if available)
    pub sender_name: Option<String>,
    /// Sender's account number (if available)
    pub sender_account: Option<String>,
    /// When the payment was settled
    pub settled_at: DateTime<Utc>,
    /// Raw webhook payload for audit
    pub raw_payload: serde_json::Value,
}
```

### InitiatePayoutRequest

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiatePayoutRequest {
    /// Unique reference code for this pay-out
    pub reference_code: String,
    /// Amount in VND
    pub amount_vnd: Decimal,
    /// Recipient bank code
    pub recipient_bank_code: String,
    /// Recipient account number
    pub recipient_account_number: String,
    /// Recipient account name
    pub recipient_account_name: String,
    /// Transfer description/memo
    pub description: String,
    /// Additional metadata
    pub metadata: serde_json::Value,
}
```

### PayoutStatus

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PayoutStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}
```

## Implementing a Custom Adapter

### Step 1: Create the Adapter Struct

```rust
use async_trait::async_trait;
use ramp_adapter::{
    traits::RailsAdapter,
    types::*,
};
use ramp_common::Result;

pub struct VietcombankAdapter {
    api_base_url: String,
    api_key: String,
    api_secret: String,
    webhook_secret: String,
    http_client: reqwest::Client,
}

impl VietcombankAdapter {
    pub fn new(config: AdapterConfig) -> Self {
        Self {
            api_base_url: config.api_base_url,
            api_key: config.api_key,
            api_secret: config.api_secret,
            webhook_secret: config.webhook_secret,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(config.timeout_secs))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Generate API signature for requests
    fn generate_signature(&self, payload: &str, timestamp: i64) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let message = format!("{}|{}", timestamp, payload);
        let mut mac = Hmac::<Sha256>::new_from_slice(
            self.api_secret.as_bytes()
        ).expect("HMAC can take key of any size");
        mac.update(message.as_bytes());

        hex::encode(mac.finalize().into_bytes())
    }
}
```

### Step 2: Implement the RailsAdapter Trait

```rust
#[async_trait]
impl RailsAdapter for VietcombankAdapter {
    fn provider_code(&self) -> &str {
        "vietcombank"
    }

    fn provider_name(&self) -> &str {
        "Vietcombank (VCB)"
    }

    async fn create_payin_instruction(
        &self,
        request: CreatePayinInstructionRequest,
    ) -> Result<PayinInstruction> {
        let timestamp = chrono::Utc::now().timestamp();

        // Build request payload for VCB API
        let payload = serde_json::json!({
            "request_id": request.reference_code,
            "amount": request.amount_vnd.to_string(),
            "expiry": request.expires_at.to_rfc3339(),
            "customer_id": request.user_id,
        });

        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

        let signature = self.generate_signature(&payload_str, timestamp);

        // Call VCB API to create virtual account
        let response = self.http_client
            .post(format!("{}/api/v1/virtual-accounts", self.api_base_url))
            .header("X-Api-Key", &self.api_key)
            .header("X-Timestamp", timestamp.to_string())
            .header("X-Signature", signature)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ramp_common::Error::External(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ramp_common::Error::External(
                format!("VCB API error: {}", error_text)
            ));
        }

        let vcb_response: serde_json::Value = response.json().await
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

        Ok(PayinInstruction {
            reference_code: request.reference_code,
            bank_code: "VCB".to_string(),
            account_number: vcb_response["virtual_account_number"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            account_name: vcb_response["account_name"]
                .as_str()
                .unwrap_or("RAMPOS")
                .to_string(),
            amount_vnd: request.amount_vnd,
            expires_at: request.expires_at,
            instructions: format!(
                "Transfer {} VND to account {} at Vietcombank",
                request.amount_vnd,
                vcb_response["virtual_account_number"].as_str().unwrap_or("")
            ),
        })
    }

    async fn parse_payin_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<PayinConfirmation> {
        // Verify signature first
        if let Some(sig) = signature {
            if !self.verify_webhook_signature(payload, sig) {
                return Err(ramp_common::Error::Validation(
                    "Invalid webhook signature".to_string()
                ));
            }
        }

        let data: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| ramp_common::Error::Validation(e.to_string()))?;

        // Parse VCB webhook format
        Ok(PayinConfirmation {
            reference_code: data["reference_number"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            bank_tx_id: data["transaction_id"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            amount_vnd: data["amount"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or_default(),
            sender_name: data["sender_name"].as_str().map(String::from),
            sender_account: data["sender_account"].as_str().map(String::from),
            settled_at: data["transaction_time"]
                .as_str()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(chrono::Utc::now),
            raw_payload: data,
        })
    }

    async fn initiate_payout(
        &self,
        request: InitiatePayoutRequest,
    ) -> Result<PayoutResult> {
        let timestamp = chrono::Utc::now().timestamp();

        let payload = serde_json::json!({
            "request_id": request.reference_code,
            "amount": request.amount_vnd.to_string(),
            "recipient_bank": request.recipient_bank_code,
            "recipient_account": request.recipient_account_number,
            "recipient_name": request.recipient_account_name,
            "description": request.description,
        });

        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

        let signature = self.generate_signature(&payload_str, timestamp);

        let response = self.http_client
            .post(format!("{}/api/v1/transfers", self.api_base_url))
            .header("X-Api-Key", &self.api_key)
            .header("X-Timestamp", timestamp.to_string())
            .header("X-Signature", signature)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ramp_common::Error::External(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ramp_common::Error::External(
                format!("VCB payout API error: {}", error_text)
            ));
        }

        let vcb_response: serde_json::Value = response.json().await
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

        Ok(PayoutResult {
            reference_code: request.reference_code,
            provider_tx_id: vcb_response["transaction_id"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            status: PayoutStatus::Processing,
            estimated_completion: Some(
                chrono::Utc::now() + chrono::Duration::minutes(5)
            ),
        })
    }

    async fn parse_payout_webhook(
        &self,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<PayoutConfirmation> {
        if let Some(sig) = signature {
            if !self.verify_webhook_signature(payload, sig) {
                return Err(ramp_common::Error::Validation(
                    "Invalid webhook signature".to_string()
                ));
            }
        }

        let data: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| ramp_common::Error::Validation(e.to_string()))?;

        let status = match data["status"].as_str().unwrap_or("") {
            "SUCCESS" | "COMPLETED" => PayoutStatus::Completed,
            "FAILED" | "REJECTED" => PayoutStatus::Failed,
            "CANCELLED" => PayoutStatus::Cancelled,
            _ => PayoutStatus::Processing,
        };

        Ok(PayoutConfirmation {
            reference_code: data["reference_number"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            bank_tx_id: data["transaction_id"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            status,
            failure_reason: data["error_message"].as_str().map(String::from),
            completed_at: if status == PayoutStatus::Completed {
                Some(chrono::Utc::now())
            } else {
                None
            },
            raw_payload: data,
        })
    }

    async fn check_payout_status(&self, reference: &str) -> Result<PayoutStatus> {
        let response = self.http_client
            .get(format!(
                "{}/api/v1/transfers/{}",
                self.api_base_url, reference
            ))
            .header("X-Api-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| ramp_common::Error::External(e.to_string()))?;

        let data: serde_json::Value = response.json().await
            .map_err(|e| ramp_common::Error::Internal(e.to_string()))?;

        Ok(match data["status"].as_str().unwrap_or("") {
            "SUCCESS" | "COMPLETED" => PayoutStatus::Completed,
            "FAILED" | "REJECTED" => PayoutStatus::Failed,
            "PENDING" => PayoutStatus::Pending,
            "CANCELLED" => PayoutStatus::Cancelled,
            _ => PayoutStatus::Processing,
        })
    }

    fn verify_webhook_signature(&self, payload: &[u8], signature: &str) -> bool {
        use ramp_common::crypto::verify_webhook_signature;

        // Signature format: "t=<timestamp>,v1=<signature>"
        verify_webhook_signature(
            self.webhook_secret.as_bytes(),
            signature,
            payload,
            300, // 5 minute tolerance
        ).is_ok()
    }
}
```

### Step 3: Register the Adapter

Add your adapter to the adapter factory:

```rust
// In crates/ramp-adapter/src/factory.rs

use crate::adapters::{MockAdapter, VietcombankAdapter};
use crate::traits::RailsAdapter;
use crate::types::AdapterConfig;
use std::sync::Arc;

pub fn create_adapter(config: AdapterConfig) -> Arc<dyn RailsAdapter> {
    match config.provider_code.as_str() {
        "mock" => Arc::new(MockAdapter::new(
            config.provider_code.clone(),
            config.webhook_secret.clone(),
        )),
        "vietcombank" => Arc::new(VietcombankAdapter::new(config)),
        // Add more adapters here
        _ => panic!("Unknown adapter: {}", config.provider_code),
    }
}
```

## Optional Traits

### VirtualAccountAdapter

For banks that support persistent virtual accounts:

```rust
#[async_trait]
pub trait VirtualAccountAdapter: RailsAdapter {
    /// Create a persistent virtual account for a user
    async fn create_virtual_account(
        &self,
        request: CreateVirtualAccountRequest,
    ) -> Result<VirtualAccountInfo>;

    /// Close a virtual account
    async fn close_virtual_account(
        &self,
        account_number: &str,
    ) -> Result<()>;

    /// List virtual accounts (paginated)
    async fn list_virtual_accounts(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<VirtualAccountInfo>>;
}
```

### InstantTransferAdapter

For banks that support instant transfers:

```rust
#[async_trait]
pub trait InstantTransferAdapter: RailsAdapter {
    /// Check if instant transfer is available for a bank
    async fn check_instant_availability(
        &self,
        bank_code: &str,
    ) -> Result<bool>;

    /// Get instant transfer fee
    async fn get_instant_fee(
        &self,
        amount: Decimal,
    ) -> Result<Decimal>;
}
```

## Testing Your Adapter

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockito;

    #[tokio::test]
    async fn test_create_payin_instruction() {
        let mut server = mockito::Server::new();

        let mock = server.mock("POST", "/api/v1/virtual-accounts")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "virtual_account_number": "VA123456789",
                "account_name": "RAMPOS"
            }"#)
            .create();

        let adapter = VietcombankAdapter::new(AdapterConfig {
            provider_code: "vietcombank".to_string(),
            api_base_url: server.url(),
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
            webhook_secret: "webhook_secret".to_string(),
            timeout_secs: 30,
            extra: serde_json::json!({}),
        });

        let request = CreatePayinInstructionRequest {
            reference_code: "REF123".to_string(),
            user_id: "user_1".to_string(),
            amount_vnd: Decimal::from(1000000),
            expires_at: Utc::now() + Duration::hours(24),
            metadata: serde_json::json!({}),
        };

        let result = adapter.create_payin_instruction(request).await;
        assert!(result.is_ok());

        let instruction = result.unwrap();
        assert_eq!(instruction.account_number, "VA123456789");

        mock.assert();
    }

    #[tokio::test]
    async fn test_webhook_signature_verification() {
        let adapter = VietcombankAdapter::new(AdapterConfig {
            webhook_secret: "secret123".to_string(),
            ..Default::default()
        });

        let payload = b"test payload";
        let timestamp = Utc::now().timestamp();

        // Generate valid signature
        let signature = generate_test_signature("secret123", timestamp, payload);

        assert!(adapter.verify_webhook_signature(
            payload,
            &format!("t={},v1={}", timestamp, signature)
        ));

        // Invalid signature should fail
        assert!(!adapter.verify_webhook_signature(
            payload,
            "t=123,v1=invalid"
        ));
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Run with --ignored for integration tests
    async fn test_real_payin_flow() {
        let config = AdapterConfig::from_env("VIETCOMBANK").unwrap();
        let adapter = VietcombankAdapter::new(config);

        // 1. Create pay-in instruction
        let instruction = adapter.create_payin_instruction(
            CreatePayinInstructionRequest {
                reference_code: format!("TEST_{}", Uuid::new_v4()),
                user_id: "integration_test_user".to_string(),
                amount_vnd: Decimal::from(100000),
                expires_at: Utc::now() + Duration::hours(1),
                metadata: serde_json::json!({"test": true}),
            }
        ).await.expect("Failed to create pay-in instruction");

        println!("Created instruction: {:?}", instruction);

        // 2. Simulate webhook (in real test, wait for actual payment)
        // This would typically be done via manual testing or a test sandbox
    }
}
```

## Configuration

Configure your adapter in the environment:

```bash
# .env
ADAPTER_VIETCOMBANK_API_BASE_URL=https://api.vietcombank.com.vn
ADAPTER_VIETCOMBANK_API_KEY=your_api_key
ADAPTER_VIETCOMBANK_API_SECRET=your_api_secret
ADAPTER_VIETCOMBANK_WEBHOOK_SECRET=your_webhook_secret
ADAPTER_VIETCOMBANK_TIMEOUT_SECS=30
```

Or in the configuration file:

```yaml
# config/adapters.yaml
adapters:
  vietcombank:
    provider_code: vietcombank
    api_base_url: ${VIETCOMBANK_API_URL}
    api_key: ${VIETCOMBANK_API_KEY}
    api_secret: ${VIETCOMBANK_API_SECRET}
    webhook_secret: ${VIETCOMBANK_WEBHOOK_SECRET}
    timeout_secs: 30
    extra:
      sandbox_mode: true
```

## Best Practices

1. **Idempotency**: Use the `reference_code` to ensure idempotent operations
2. **Retry Logic**: Implement exponential backoff for API calls
3. **Logging**: Log all API requests and responses for debugging
4. **Error Handling**: Return specific error types for different failure modes
5. **Webhook Verification**: Always verify webhook signatures before processing
6. **Raw Payload Storage**: Store the raw webhook payload for audit purposes
7. **Timeout Configuration**: Set appropriate timeouts for different operations
8. **Rate Limiting**: Respect bank API rate limits

## Troubleshooting

### Common Issues

1. **Signature Mismatch**: Ensure timestamp tolerance is sufficient (300 seconds recommended)
2. **Timeout Errors**: Increase timeout for slow bank APIs
3. **Webhook Failures**: Check webhook URL is accessible and returns 200 quickly
4. **Amount Mismatch**: Verify decimal precision handling (VND has no decimal places)

### Debug Mode

Enable debug logging for adapters:

```bash
RUST_LOG=ramp_adapter=debug cargo run
```
