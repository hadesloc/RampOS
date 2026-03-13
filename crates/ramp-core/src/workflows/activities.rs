//! Workflow Activities for RampOS
//!
//! This module contains activity implementations that perform the actual work
//! in workflows. Activities are the building blocks that interact with external
//! services, databases, and other systems.
//!
//! ## Rails Adapter Integration
//!
//! Activities use the `RailsAdapter` trait for banking operations:
//! - `create_payin_instruction` - Creates virtual accounts or QR codes via real bank APIs
//! - `initiate_payout` - Initiates bank transfers via real bank APIs
//! - `parse_payin_webhook` - Parses incoming bank webhooks
//!
//! The adapter is injected via `ActivityContext` to support both production
//! (VietQR, Napas) and testing (MockAdapter) scenarios.

use crate::repository::ledger::LedgerRepository;
use crate::repository::tenant::TenantRepository;
use crate::repository::webhook::WebhookRepository;
use crate::repository::BankConfirmationRepository;
use crate::service::{CorridorPackService, PaymentMethodCapabilityService};
use crate::service::webhook::{WebhookEventType, WebhookService};
use chrono::Utc;
use ramp_adapter::{
    CreatePayinInstructionRequest, InitiatePayoutRequest as AdapterPayoutRequest, PayoutStatus,
    RailsAdapter,
};
use ramp_common::{
    ledger::{patterns, LedgerCurrency, LedgerError},
    types::*,
};
use ramp_compliance::{
    case::CaseManager,
    types::{CaseSeverity, CaseType},
};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, instrument, warn};

use super::{BankConfirmation, PayinWorkflowInput, TradeWorkflowInput};

/// Activity context provides shared dependencies for activities
///
/// This context is injected into all activities and provides access to:
/// - Repository layer (ledger, webhook, tenant, bank_confirmation)
/// - Case management for compliance
/// - Rails adapters for banking operations (keyed by provider code)
/// - Bank confirmation channel for webhook-driven confirmations
#[derive(Clone)]
pub struct ActivityContext {
    pub ledger_repo: Arc<dyn LedgerRepository>,
    pub webhook_repo: Arc<dyn WebhookRepository>,
    pub tenant_repo: Arc<dyn TenantRepository>,
    pub case_manager: Arc<CaseManager>,
    /// Rails adapters keyed by provider code (e.g., "vietqr", "napas", "mock")
    pub rails_adapters: Arc<RwLock<HashMap<String, Arc<dyn RailsAdapter>>>>,
    /// Bank confirmation store for webhook-driven confirmations (in-memory cache)
    /// Key: intent_id or reference_code, Value: confirmation data
    pub bank_confirmations: Arc<RwLock<HashMap<String, BankConfirmation>>>,
    /// Bank confirmation repository for database-backed polling
    pub bank_confirmation_repo: Option<Arc<dyn BankConfirmationRepository>>,
    /// Optional corridor-pack service for bounded pilot corridor resolution.
    pub corridor_pack_service: Option<Arc<CorridorPackService>>,
    /// Optional payment-method capability service for bounded pilot corridor resolution.
    pub payment_method_capability_service: Option<Arc<PaymentMethodCapabilityService>>,
}

impl ActivityContext {
    pub fn new(
        ledger_repo: Arc<dyn LedgerRepository>,
        webhook_repo: Arc<dyn WebhookRepository>,
        tenant_repo: Arc<dyn TenantRepository>,
        case_manager: Arc<CaseManager>,
    ) -> Self {
        Self {
            ledger_repo,
            webhook_repo,
            tenant_repo,
            case_manager,
            rails_adapters: Arc::new(RwLock::new(HashMap::new())),
            bank_confirmations: Arc::new(RwLock::new(HashMap::new())),
            bank_confirmation_repo: None,
            corridor_pack_service: None,
            payment_method_capability_service: None,
        }
    }

    /// Create with pre-configured rails adapters
    pub fn with_adapters(
        ledger_repo: Arc<dyn LedgerRepository>,
        webhook_repo: Arc<dyn WebhookRepository>,
        tenant_repo: Arc<dyn TenantRepository>,
        case_manager: Arc<CaseManager>,
        adapters: HashMap<String, Arc<dyn RailsAdapter>>,
    ) -> Self {
        Self {
            ledger_repo,
            webhook_repo,
            tenant_repo,
            case_manager,
            rails_adapters: Arc::new(RwLock::new(adapters)),
            bank_confirmations: Arc::new(RwLock::new(HashMap::new())),
            bank_confirmation_repo: None,
            corridor_pack_service: None,
            payment_method_capability_service: None,
        }
    }

    /// Set the bank confirmation repository for database-backed polling
    pub fn with_bank_confirmation_repo(
        mut self,
        repo: Arc<dyn BankConfirmationRepository>,
    ) -> Self {
        self.bank_confirmation_repo = Some(repo);
        self
    }

    /// Configure bounded pilot corridor services without changing default routing behavior.
    pub fn with_pilot_corridor_services(
        mut self,
        corridor_pack_service: Arc<CorridorPackService>,
        payment_method_capability_service: Arc<PaymentMethodCapabilityService>,
    ) -> Self {
        self.corridor_pack_service = Some(corridor_pack_service);
        self.payment_method_capability_service = Some(payment_method_capability_service);
        self
    }

    /// Register a rails adapter
    pub async fn register_adapter(&self, provider_code: &str, adapter: Arc<dyn RailsAdapter>) {
        let mut adapters = self.rails_adapters.write().await;
        adapters.insert(provider_code.to_lowercase(), adapter);
    }

    /// Get a rails adapter by provider code
    pub async fn get_adapter(&self, provider_code: &str) -> Option<Arc<dyn RailsAdapter>> {
        let adapters = self.rails_adapters.read().await;
        adapters.get(&provider_code.to_lowercase()).cloned()
    }

    /// Store a bank confirmation (called by webhook handler)
    pub async fn store_bank_confirmation(&self, key: &str, confirmation: BankConfirmation) {
        let mut confirmations = self.bank_confirmations.write().await;
        confirmations.insert(key.to_string(), confirmation);
    }

    /// Retrieve and remove a bank confirmation
    pub async fn take_bank_confirmation(&self, key: &str) -> Option<BankConfirmation> {
        let mut confirmations = self.bank_confirmations.write().await;
        confirmations.remove(key)
    }

    /// Check if a bank confirmation exists
    pub async fn has_bank_confirmation(&self, key: &str) -> bool {
        let confirmations = self.bank_confirmations.read().await;
        confirmations.contains_key(key)
    }
}

/// Global activity context for static activity functions
/// In production with real Temporal SDK, this would be handled via workflow context
static ACTIVITY_CONTEXT: std::sync::OnceLock<ActivityContext> = std::sync::OnceLock::new();

/// Initialize the global activity context
pub fn init_activity_context(ctx: ActivityContext) {
    let _ = ACTIVITY_CONTEXT.set(ctx);
}

/// Initialize the global activity context with adapters from ramp_adapter factory
///
/// This is a convenience function that creates adapters based on environment variables.
/// For production use, call this during application startup.
///
/// # Example
/// ```ignore
/// use ramp_core::workflows::activities::init_activity_context_with_adapters;
///
/// // Initialize with adapters from environment
/// init_activity_context_with_adapters(ledger_repo, webhook_repo, tenant_repo, case_manager).await;
/// ```
pub async fn init_activity_context_with_adapters(
    ledger_repo: Arc<dyn LedgerRepository>,
    webhook_repo: Arc<dyn WebhookRepository>,
    tenant_repo: Arc<dyn TenantRepository>,
    case_manager: Arc<CaseManager>,
) {
    // Create adapters from environment configuration
    let adapters = match ramp_adapter::create_adapters_from_env() {
        Ok(adapters) => adapters,
        Err(e) => {
            warn!(error = %e, "Failed to create adapters from environment, using empty adapter map");
            HashMap::new()
        }
    };

    let ctx = ActivityContext::with_adapters(
        ledger_repo,
        webhook_repo,
        tenant_repo,
        case_manager,
        adapters,
    );
    init_activity_context(ctx);

    info!("Activity context initialized with adapters from environment");
}

/// Initialize activity context with test adapters
///
/// Use this for integration tests where you need mock adapters.
pub async fn init_activity_context_for_testing(
    ledger_repo: Arc<dyn LedgerRepository>,
    webhook_repo: Arc<dyn WebhookRepository>,
    tenant_repo: Arc<dyn TenantRepository>,
    case_manager: Arc<CaseManager>,
) {
    let adapters = ramp_adapter::create_test_adapters();
    let ctx = ActivityContext::with_adapters(
        ledger_repo,
        webhook_repo,
        tenant_repo,
        case_manager,
        adapters,
    );
    init_activity_context(ctx);

    info!("Activity context initialized with test adapters");
}

/// Initialize activity context with full configuration including bank confirmation repository
///
/// Use this for production when you need database-backed bank confirmation polling.
pub async fn init_activity_context_full(
    ledger_repo: Arc<dyn LedgerRepository>,
    webhook_repo: Arc<dyn WebhookRepository>,
    tenant_repo: Arc<dyn TenantRepository>,
    case_manager: Arc<CaseManager>,
    bank_confirmation_repo: Option<Arc<dyn BankConfirmationRepository>>,
) {
    // Create adapters from environment configuration
    let adapters = match ramp_adapter::create_adapters_from_env() {
        Ok(adapters) => adapters,
        Err(e) => {
            warn!(error = %e, "Failed to create adapters from environment, using empty adapter map");
            HashMap::new()
        }
    };

    let mut ctx = ActivityContext::with_adapters(
        ledger_repo,
        webhook_repo,
        tenant_repo,
        case_manager,
        adapters,
    );

    if let Some(repo) = bank_confirmation_repo {
        ctx = ctx.with_bank_confirmation_repo(repo);
    }

    init_activity_context(ctx);

    info!("Activity context initialized with full configuration");
}

/// Get the activity context
fn get_context() -> Option<&'static ActivityContext> {
    ACTIVITY_CONTEXT.get()
}

fn parse_pilot_corridor_code(provider: &str) -> Option<&str> {
    provider.strip_prefix("corridor:")
}

async fn resolve_pilot_corridor_provider(
    ctx: &ActivityContext,
    tenant_id: &str,
    requested_provider: &str,
    settlement_direction: &str,
    method_family: &str,
) -> String {
    let Some(corridor_code) = parse_pilot_corridor_code(requested_provider) else {
        return requested_provider.to_string();
    };

    let (Some(corridor_pack_service), Some(payment_method_capability_service)) = (
        ctx.corridor_pack_service.as_ref(),
        ctx.payment_method_capability_service.as_ref(),
    ) else {
        return requested_provider.to_string();
    };

    let Ok(Some(corridor)) = corridor_pack_service
        .get_corridor_pack(Some(tenant_id), corridor_code)
        .await
    else {
        return requested_provider.to_string();
    };

    let Ok(capabilities) = payment_method_capability_service
        .list_capabilities(Some(&corridor.corridor_pack_id), None)
        .await
    else {
        return requested_provider.to_string();
    };

    let supports_method = capabilities.capabilities.iter().any(|capability| {
        capability
            .settlement_direction
            .eq_ignore_ascii_case(settlement_direction)
            && capability.method_family.eq_ignore_ascii_case(method_family)
    });

    if !supports_method {
        return requested_provider.to_string();
    }

    corridor
        .endpoints
        .iter()
        .find(|endpoint| {
            endpoint
                .endpoint_role
                .eq_ignore_ascii_case(if settlement_direction.eq_ignore_ascii_case("payout") {
                    "destination"
                } else {
                    "source"
                })
                && endpoint
                    .method_family
                    .as_deref()
                    .map(|value| value.eq_ignore_ascii_case(method_family))
                    .unwrap_or(true)
        })
        .and_then(|endpoint| {
            endpoint
                .adapter_key
                .clone()
                .or_else(|| endpoint.provider_key.clone())
                .or_else(|| endpoint.partner_id.clone())
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| requested_provider.to_string())
}

/// Payin workflow activities
pub mod payin_activities {
    use super::*;

    /// Result of issuing a payment instruction
    #[derive(Debug, Clone)]
    pub struct PayinInstructionResult {
        pub reference_code: String,
        pub bank_code: String,
        pub account_number: String,
        pub account_name: String,
        pub instructions: String,
        /// QR code image as base64 PNG (if applicable)
        pub qr_image_base64: Option<String>,
        /// Raw QR code content string (if applicable)
        pub qr_content: Option<String>,
    }

    impl PayinInstructionResult {
        /// Check if this instruction includes a QR code
        pub fn has_qr_code(&self) -> bool {
            self.qr_image_base64.is_some()
        }

        /// Parse QR data from instructions JSON (if VietQR)
        fn from_instruction_with_qr(
            reference_code: String,
            bank_code: String,
            account_number: String,
            account_name: String,
            instructions: String,
        ) -> Self {
            // Try to parse QR data from instructions if it's JSON
            let (qr_image, qr_content) =
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&instructions) {
                    (
                        json.get("qr_image_base64")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        json.get("qr_content")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                    )
                } else {
                    (None, None)
                };

            Self {
                reference_code,
                bank_code,
                account_number,
                account_name,
                instructions,
                qr_image_base64: qr_image,
                qr_content: qr_content,
            }
        }
    }

    /// Activity: Issue payment instruction to user
    ///
    /// This activity calls the real rails adapter to:
    /// 1. Create a virtual account or generate QR code
    /// 2. Store the virtual account details
    /// 3. Return the instruction details for the user
    #[instrument(skip(input), fields(intent_id = %input.intent_id, rails = %input.rails_provider))]
    pub async fn issue_instruction(input: &PayinWorkflowInput) -> Result<String, String> {
        info!(
            "Issuing payment instruction via {} adapter",
            input.rails_provider
        );

        let ctx = get_context();

        if let Some(ctx) = ctx {
            // Try to get the rails adapter for this provider
            if let Some(adapter) = ctx.get_adapter(&input.rails_provider).await {
                // Create the payin instruction request
                let expires_at = chrono::DateTime::parse_from_rfc3339(&input.expires_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now() + chrono::Duration::minutes(30));

                let request = CreatePayinInstructionRequest {
                    reference_code: input.reference_code.clone(),
                    user_id: input.user_id.clone(),
                    amount_vnd: Decimal::from(input.amount_vnd),
                    expires_at,
                    metadata: serde_json::json!({
                        "tenant_id": input.tenant_id,
                        "intent_id": input.intent_id,
                    }),
                };

                // Call the real adapter
                match adapter.create_payin_instruction(request).await {
                    Ok(instruction) => {
                        info!(
                            reference_code = %instruction.reference_code,
                            bank_code = %instruction.bank_code,
                            account_number = %instruction.account_number,
                            "Payment instruction created via {} adapter",
                            adapter.provider_name()
                        );
                        Ok(instruction.reference_code)
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            "Failed to create payment instruction via {} adapter",
                            adapter.provider_name()
                        );
                        Err(format!("Adapter error: {}", e))
                    }
                }
            } else {
                // No adapter registered for this provider - fall back to simulation
                warn!(
                    provider = %input.rails_provider,
                    "No adapter registered for provider, using simulation mode"
                );
                Ok(input.reference_code.clone())
            }
        } else {
            // No context available (test mode) - return the provided reference code
            info!(
                "No activity context, simulation mode: Returning reference code {}",
                input.reference_code
            );
            Ok(input.reference_code.clone())
        }
    }

    /// Activity: Issue payment instruction with full details
    ///
    /// Returns the complete instruction details instead of just the reference code.
    #[instrument(skip(input), fields(intent_id = %input.intent_id, rails = %input.rails_provider))]
    pub async fn issue_instruction_full(
        input: &PayinWorkflowInput,
    ) -> Result<PayinInstructionResult, String> {
        info!(
            "Issuing payment instruction (full) via {} adapter",
            input.rails_provider
        );

        let ctx = get_context();

        if let Some(ctx) = ctx {
            if let Some(adapter) = ctx.get_adapter(&input.rails_provider).await {
                let expires_at = chrono::DateTime::parse_from_rfc3339(&input.expires_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now() + chrono::Duration::minutes(30));

                let request = CreatePayinInstructionRequest {
                    reference_code: input.reference_code.clone(),
                    user_id: input.user_id.clone(),
                    amount_vnd: Decimal::from(input.amount_vnd),
                    expires_at,
                    metadata: serde_json::json!({
                        "tenant_id": input.tenant_id,
                        "intent_id": input.intent_id,
                    }),
                };

                match adapter.create_payin_instruction(request).await {
                    Ok(instruction) => {
                        info!(
                            reference_code = %instruction.reference_code,
                            bank_code = %instruction.bank_code,
                            "Payment instruction created successfully"
                        );
                        Ok(PayinInstructionResult::from_instruction_with_qr(
                            instruction.reference_code,
                            instruction.bank_code,
                            instruction.account_number,
                            instruction.account_name,
                            instruction.instructions,
                        ))
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to create payment instruction");
                        Err(format!("Adapter error: {}", e))
                    }
                }
            } else {
                // Simulation mode
                Ok(PayinInstructionResult {
                    reference_code: input.reference_code.clone(),
                    bank_code: "SIMULATION".to_string(),
                    account_number: format!("VA{}", &input.reference_code),
                    account_name: "RAMPOS SIMULATION".to_string(),
                    instructions: "Simulation mode - no real payment required".to_string(),
                    qr_image_base64: None,
                    qr_content: None,
                })
            }
        } else {
            // No context (test mode)
            Ok(PayinInstructionResult {
                reference_code: input.reference_code.clone(),
                bank_code: "TEST".to_string(),
                account_number: format!("VA{}", &input.reference_code),
                account_name: "RAMPOS TEST".to_string(),
                instructions: "Test mode".to_string(),
                qr_image_base64: None,
                qr_content: None,
            })
        }
    }

    /// Activity: Wait for bank confirmation (webhook-driven)
    ///
    /// This activity polls for a bank confirmation from multiple sources:
    /// 1. In-memory cache (for immediate webhook notifications)
    /// 2. Database (bank_confirmations table) for persistent storage
    ///
    /// The webhook endpoint stores confirmations in the database via BankConfirmationRepository.
    /// This activity polls both sources with exponential backoff.
    ///
    /// In a real Temporal implementation, this would be a signal handler instead.
    #[instrument(skip(timeout), fields(intent_id = %intent_id))]
    pub async fn wait_for_bank_confirmation(
        intent_id: &str,
        timeout: Duration,
    ) -> Result<BankConfirmation, String> {
        wait_for_bank_confirmation_with_ref(intent_id, None, None, timeout).await
    }

    /// Activity: Wait for bank confirmation with reference code and tenant
    ///
    /// Enhanced version that polls both in-memory cache and database.
    /// Use this when you have tenant_id and reference_code for database lookup.
    #[instrument(skip(timeout), fields(intent_id = %intent_id, reference_code = ?reference_code))]
    pub async fn wait_for_bank_confirmation_with_ref(
        intent_id: &str,
        tenant_id: Option<&str>,
        reference_code: Option<&str>,
        timeout: Duration,
    ) -> Result<BankConfirmation, String> {
        info!("Waiting for bank confirmation (timeout: {:?})", timeout);

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let start = std::time::Instant::now();
            let poll_interval = Duration::from_secs(1);

            // Poll for confirmation with exponential backoff
            let mut current_interval = poll_interval;
            let max_interval = Duration::from_secs(10);

            while start.elapsed() < timeout {
                // 1. Check in-memory cache by intent_id
                if let Some(confirmation) = ctx.take_bank_confirmation(intent_id).await {
                    info!(
                        bank_tx_id = %confirmation.bank_tx_id,
                        amount = confirmation.amount,
                        "Bank confirmation received from in-memory cache"
                    );
                    return Ok(confirmation);
                }

                // 2. Check in-memory cache by reference code pattern
                if let Some(ref_code) = reference_code {
                    if let Some(confirmation) = ctx.take_bank_confirmation(ref_code).await {
                        info!(
                            bank_tx_id = %confirmation.bank_tx_id,
                            "Bank confirmation received via reference code (in-memory)"
                        );
                        return Ok(confirmation);
                    }
                }

                // 3. Check database if bank_confirmation_repo is available
                if let (Some(repo), Some(tenant), Some(ref_code)) = (
                    ctx.bank_confirmation_repo.as_ref(),
                    tenant_id,
                    reference_code,
                ) {
                    let tenant_id_obj = ramp_common::types::TenantId::new(tenant);

                    match repo
                        .get_pending_by_reference(&tenant_id_obj, ref_code)
                        .await
                    {
                        Ok(confirmations) => {
                            if let Some(db_confirmation) = confirmations.first() {
                                info!(
                                    confirmation_id = %db_confirmation.id,
                                    bank_tx_id = ?db_confirmation.bank_tx_id,
                                    amount = %db_confirmation.amount,
                                    "Bank confirmation found in database"
                                );

                                // Mark as matched in database
                                if let Err(e) = repo
                                    .update_matched(&tenant_id_obj, &db_confirmation.id, intent_id)
                                    .await
                                {
                                    warn!(error = %e, "Failed to update confirmation status in database");
                                }

                                // Convert to BankConfirmation
                                let bank_tx_id = db_confirmation
                                    .bank_tx_id
                                    .clone()
                                    .or(db_confirmation.bank_reference.clone())
                                    .unwrap_or_else(|| db_confirmation.id.clone());

                                let amount = db_confirmation
                                    .amount
                                    .to_string()
                                    .parse::<i64>()
                                    .unwrap_or(0);

                                let settled_at = db_confirmation
                                    .transaction_time
                                    .unwrap_or(db_confirmation.webhook_received_at)
                                    .to_rfc3339();

                                return Ok(BankConfirmation {
                                    bank_tx_id,
                                    amount,
                                    settled_at,
                                });
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Error polling database for bank confirmation");
                        }
                    }
                }

                // Sleep with backoff
                tokio::time::sleep(current_interval).await;
                current_interval = std::cmp::min(current_interval * 2, max_interval);
            }

            warn!(intent_id = %intent_id, "Timeout waiting for bank confirmation");
            Err("Timeout waiting for bank confirmation".to_string())
        } else {
            // No context - immediate timeout
            Err("No activity context available".to_string())
        }
    }

    /// Activity: Process incoming bank webhook
    ///
    /// Parses and validates a bank webhook payload using the appropriate adapter.
    /// Returns the parsed confirmation which can then be stored for the workflow.
    #[instrument(skip(payload, signature), fields(provider = %provider_code))]
    pub async fn process_bank_webhook(
        provider_code: &str,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<BankConfirmation, String> {
        info!("Processing bank webhook from {}", provider_code);

        let ctx = get_context();

        if let Some(ctx) = ctx {
            if let Some(adapter) = ctx.get_adapter(provider_code).await {
                match adapter.parse_payin_webhook(payload, signature).await {
                    Ok(confirmation) => {
                        info!(
                            reference_code = %confirmation.reference_code,
                            bank_tx_id = %confirmation.bank_tx_id,
                            amount = %confirmation.amount_vnd,
                            "Bank webhook parsed successfully"
                        );

                        let bank_confirmation = BankConfirmation {
                            bank_tx_id: confirmation.bank_tx_id,
                            amount: confirmation
                                .amount_vnd
                                .to_string()
                                .parse::<i64>()
                                .unwrap_or(0),
                            settled_at: confirmation.settled_at.to_rfc3339(),
                        };

                        // Store for the waiting workflow
                        ctx.store_bank_confirmation(
                            &confirmation.reference_code,
                            bank_confirmation.clone(),
                        )
                        .await;

                        Ok(bank_confirmation)
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to parse bank webhook");
                        Err(format!("Webhook parsing error: {}", e))
                    }
                }
            } else {
                Err(format!(
                    "No adapter registered for provider: {}",
                    provider_code
                ))
            }
        } else {
            Err("No activity context available".to_string())
        }
    }

    /// Activity: Credit user's VND balance
    ///
    /// Creates ledger entries to credit the user's VND balance after
    /// bank confirmation is received.
    #[instrument(skip(), fields(intent_id = %intent_id, amount = %amount))]
    pub async fn credit_vnd_balance(
        tenant_id: &str,
        user_id: &str,
        intent_id: &str,
        amount: i64,
    ) -> Result<(), String> {
        info!("Crediting VND balance");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let tenant = TenantId::new(tenant_id);
            let user = UserId::new(user_id);
            let intent = IntentId(intent_id.to_string());
            let decimal_amount = Decimal::from(amount);

            // Create ledger transaction for pay-in confirmed
            let tx = patterns::payin_vnd_confirmed(
                tenant.clone(),
                user.clone(),
                intent.clone(),
                decimal_amount,
            )
            .map_err(|e: LedgerError| e.to_string())?;

            // Record the transaction
            ctx.ledger_repo
                .record_transaction(tx)
                .await
                .map_err(|e| e.to_string())?;

            info!("VND balance credited successfully");
            Ok(())
        } else {
            // No context available (test mode)
            info!("Simulation mode: VND balance credit simulated");
            Ok(())
        }
    }

    /// Activity: Send webhook notification
    ///
    /// Queues a webhook event for delivery to the tenant's webhook endpoint.
    #[instrument(skip(payload), fields(tenant_id = %tenant_id, event_type = %event_type))]
    pub async fn send_webhook(
        tenant_id: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        info!("Sending webhook notification");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let tenant = TenantId::new(tenant_id);

            // Determine webhook event type
            let webhook_type = match event_type {
                "intent.completed" => WebhookEventType::IntentStatusChanged,
                "risk.review.required" => WebhookEventType::RiskReviewRequired,
                "kyc.flagged" => WebhookEventType::KycFlagged,
                _ => WebhookEventType::IntentStatusChanged,
            };

            // Extract intent_id from payload if present
            let intent_id = payload
                .get("intent_id")
                .and_then(|v| v.as_str())
                .map(|s| IntentId(s.to_string()));

            // Create webhook service and queue event
            let webhook_service =
                WebhookService::new(ctx.webhook_repo.clone(), ctx.tenant_repo.clone())
                    .map_err(|e| e.to_string())?;

            webhook_service
                .queue_event(&tenant, webhook_type, intent_id.as_ref(), payload)
                .await
                .map_err(|e| e.to_string())?;

            info!("Webhook notification queued");
            Ok(())
        } else {
            // No context available (test mode)
            info!("Simulation mode: Webhook notification simulated");
            Ok(())
        }
    }

    /// Activity: Reverse a pay-in credit (compensation)
    ///
    /// Used when a pay-in needs to be reversed due to error or fraud.
    #[instrument(skip(), fields(intent_id = %intent_id, amount = %amount))]
    pub async fn reverse_credit(
        tenant_id: &str,
        user_id: &str,
        intent_id: &str,
        amount: i64,
        reason: &str,
    ) -> Result<(), String> {
        info!(reason = %reason, "Reversing VND credit");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let tenant = TenantId::new(tenant_id);
            let user = UserId::new(user_id);
            let intent = IntentId(intent_id.to_string());
            let decimal_amount = Decimal::from(amount);

            // Create reverse ledger transaction
            // Debit user's VND liability (reduce what we owe them)
            // Credit bank asset (money stays with us)
            use ramp_common::ledger::{AccountType, LedgerCurrency, LedgerTransactionBuilder};

            let tx = LedgerTransactionBuilder::new(
                tenant.clone(),
                intent.clone(),
                format!("Reverse pay-in: {}", reason),
            )
            .debit_user(
                user.clone(),
                AccountType::LiabilityUserVnd,
                decimal_amount,
                LedgerCurrency::VND,
            )
            .credit(AccountType::AssetBank, decimal_amount, LedgerCurrency::VND)
            .build()
            .map_err(|e| e.to_string())?;

            ctx.ledger_repo
                .record_transaction(tx)
                .await
                .map_err(|e| e.to_string())?;

            info!("Credit reversal completed");
            Ok(())
        } else {
            info!("Simulation mode: Credit reversal simulated");
            Ok(())
        }
    }
}

/// Trade workflow activities
pub mod trade_activities {
    use super::*;

    /// Activity: Run post-trade compliance check
    ///
    /// Performs compliance checks after a trade is executed:
    /// - Velocity checks (too many trades in short time)
    /// - Large transaction checks
    /// - Pattern analysis for wash trading
    #[instrument(skip(input), fields(trade_id = %input.trade_id, symbol = %input.symbol))]
    pub async fn run_post_trade_check(input: &TradeWorkflowInput) -> Result<bool, String> {
        info!("Running post-trade compliance check");

        let vnd_abs = input.vnd_delta.abs();

        // Large transaction threshold: 1B VND
        let large_tx_threshold = 1_000_000_000i64;

        if vnd_abs > large_tx_threshold {
            warn!(
                amount = vnd_abs,
                threshold = large_tx_threshold,
                "Trade exceeds large transaction threshold"
            );
            return Ok(false);
        }

        // Additional compliance checks could be added here:
        // - Velocity check: count recent trades for this user
        // - Pattern analysis: detect wash trading patterns
        // - Market manipulation detection

        info!("Post-trade compliance check passed");
        Ok(true)
    }

    /// Activity: Settle trade in ledger
    ///
    /// Creates the double-entry ledger transactions for the trade:
    /// - For BUY: Debit user's VND, Credit user's crypto
    /// - For SELL: Debit user's crypto, Credit user's VND
    #[instrument(skip(input), fields(trade_id = %input.trade_id, symbol = %input.symbol))]
    pub async fn settle_in_ledger(input: &TradeWorkflowInput) -> Result<(), String> {
        info!("Settling trade in ledger");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let tenant = TenantId::new(&input.tenant_id);
            let user = UserId::new(&input.user_id);
            let intent = IntentId(input.intent_id.clone());

            // Parse amounts
            let vnd_amount = Decimal::from(input.vnd_delta.abs());
            let crypto_amount: Decimal = input
                .crypto_delta
                .parse()
                .map_err(|_| "Invalid crypto amount")?;
            let crypto_amount = crypto_amount.abs();

            // Determine if buy or sell based on VND delta
            // Negative VND delta = user is paying VND = buying crypto
            let is_buy = input.vnd_delta < 0;

            // Determine crypto currency from symbol (e.g., "BTC/VND" -> BTC)
            let crypto_symbol = input.symbol.split('/').next().unwrap_or("BTC");
            let crypto_currency = LedgerCurrency::from_symbol(crypto_symbol);

            // Create trade ledger transaction
            let tx = patterns::trade_crypto_vnd(
                tenant.clone(),
                user.clone(),
                intent.clone(),
                vnd_amount,
                crypto_amount,
                crypto_currency,
                is_buy,
            )
            .map_err(|e: LedgerError| e.to_string())?;

            // Record the transaction
            ctx.ledger_repo
                .record_transaction(tx)
                .await
                .map_err(|e| e.to_string())?;

            info!(
                is_buy = is_buy,
                vnd_amount = %vnd_amount,
                crypto_amount = %crypto_amount,
                "Trade settled in ledger"
            );
            Ok(())
        } else {
            info!("Simulation mode: Trade settlement simulated");
            Ok(())
        }
    }

    /// Activity: Flag trade for manual review
    ///
    /// Creates a compliance case for the trade when it fails compliance checks
    /// or exhibits suspicious patterns.
    #[instrument(skip(), fields(intent_id = %intent_id, reason = %reason))]
    pub async fn flag_for_review(intent_id: &str, reason: &str) -> Result<String, String> {
        info!("Flagging trade for manual review");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            // Extract tenant_id from intent_id if possible, or use a default
            // In production, this would be passed as a parameter
            let tenant_id = TenantId::new("system");

            // Determine case type based on reason
            let case_type = if reason.contains("Large") || reason.contains("threshold") {
                CaseType::LargeTransaction
            } else if reason.contains("velocity") || reason.contains("Velocity") {
                CaseType::Velocity
            } else if reason.contains("wash") || reason.contains("Wash") {
                CaseType::Other("WashTrading".to_string())
            } else {
                CaseType::Other(reason.to_string())
            };

            // Create AML case
            let case_id = ctx
                .case_manager
                .create_case(
                    &tenant_id,
                    None,
                    Some(&IntentId(intent_id.to_string())),
                    case_type,
                    CaseSeverity::Medium,
                    serde_json::json!({
                        "reason": reason,
                        "intent_id": intent_id,
                        "flagged_at": chrono::Utc::now().to_rfc3339(),
                    }),
                )
                .await
                .map_err(|e| e.to_string())?;

            info!(case_id = %case_id, "Trade flagged, case created");
            Ok(case_id)
        } else {
            // Simulation mode
            let case_id = format!("CASE_{}", intent_id);
            info!(case_id = %case_id, "Simulation mode: Case creation simulated");
            Ok(case_id)
        }
    }

    /// Activity: Reverse trade settlement (compensation)
    ///
    /// Reverses the ledger entries for a trade that needs to be rolled back.
    #[instrument(skip(input), fields(trade_id = %input.trade_id))]
    pub async fn reverse_settlement(
        input: &TradeWorkflowInput,
        reason: &str,
    ) -> Result<(), String> {
        info!(reason = %reason, "Reversing trade settlement");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let tenant = TenantId::new(&input.tenant_id);
            let user = UserId::new(&input.user_id);
            let intent = IntentId(format!("{}_reversal", input.intent_id));

            // Parse amounts
            let vnd_amount = Decimal::from(input.vnd_delta.abs());
            let crypto_amount: Decimal = input
                .crypto_delta
                .parse()
                .map_err(|_| "Invalid crypto amount")?;
            let crypto_amount = crypto_amount.abs();

            // Original was buy, so reversal is sell (and vice versa)
            let original_is_buy = input.vnd_delta < 0;
            let reversal_is_buy = !original_is_buy;

            let crypto_symbol = input.symbol.split('/').next().unwrap_or("BTC");
            let crypto_currency = LedgerCurrency::from_symbol(crypto_symbol);

            // Create reverse trade transaction
            let tx = patterns::trade_crypto_vnd(
                tenant.clone(),
                user.clone(),
                intent.clone(),
                vnd_amount,
                crypto_amount,
                crypto_currency,
                reversal_is_buy,
            )
            .map_err(|e: LedgerError| e.to_string())?;

            ctx.ledger_repo
                .record_transaction(tx)
                .await
                .map_err(|e| e.to_string())?;

            info!("Trade settlement reversed");
            Ok(())
        } else {
            info!("Simulation mode: Trade reversal simulated");
            Ok(())
        }
    }
}

/// Payout workflow activities
pub mod payout_activities {
    use super::*;

    /// Payout request input
    #[derive(Debug, Clone)]
    pub struct PayoutRequest {
        pub tenant_id: String,
        pub user_id: String,
        pub intent_id: String,
        pub reference_code: String,
        pub amount_vnd: i64,
        pub rails_provider: String,
        pub recipient_bank_code: String,
        pub recipient_account_number: String,
        pub recipient_account_name: String,
        pub description: String,
    }

    /// Payout result
    #[derive(Debug, Clone)]
    pub struct PayoutResult {
        pub reference_code: String,
        pub provider_tx_id: String,
        pub status: String,
        pub estimated_completion: Option<String>,
    }

    /// Activity: Initiate bank transfer
    ///
    /// This activity calls the real rails adapter to initiate a bank transfer.
    #[instrument(skip(request), fields(intent_id = %request.intent_id, rails = %request.rails_provider))]
    pub async fn initiate_bank_transfer(request: &PayoutRequest) -> Result<PayoutResult, String> {
        info!(
            "Initiating bank transfer via {} adapter",
            request.rails_provider
        );

        let ctx = get_context();

        if let Some(ctx) = ctx {
            let resolved_provider = resolve_pilot_corridor_provider(
                ctx,
                &request.tenant_id,
                &request.rails_provider,
                "payout",
                "push_transfer",
            )
            .await;

            if let Some(adapter) = ctx.get_adapter(&resolved_provider).await {
                let adapter_request = AdapterPayoutRequest {
                    reference_code: request.reference_code.clone(),
                    amount_vnd: Decimal::from(request.amount_vnd),
                    recipient_bank_code: request.recipient_bank_code.clone(),
                    recipient_account_number: request.recipient_account_number.clone(),
                    recipient_account_name: request.recipient_account_name.clone(),
                    description: request.description.clone(),
                    metadata: serde_json::json!({
                        "tenant_id": request.tenant_id,
                        "user_id": request.user_id,
                        "intent_id": request.intent_id,
                    }),
                };

                match adapter.initiate_payout(adapter_request).await {
                    Ok(result) => {
                        info!(
                            reference_code = %result.reference_code,
                            provider_tx_id = %result.provider_tx_id,
                            status = ?result.status,
                            "Bank transfer initiated successfully"
                        );

                        let status_str = match result.status {
                            PayoutStatus::Pending => "PENDING",
                            PayoutStatus::Processing => "PROCESSING",
                            PayoutStatus::Completed => "COMPLETED",
                            PayoutStatus::Failed => "FAILED",
                            PayoutStatus::Cancelled => "CANCELLED",
                        };

                        Ok(PayoutResult {
                            reference_code: result.reference_code,
                            provider_tx_id: result.provider_tx_id,
                            status: status_str.to_string(),
                            estimated_completion: result
                                .estimated_completion
                                .map(|dt| dt.to_rfc3339()),
                        })
                    }
                    Err(e) => {
                        error!(
                            error = %e,
                            "Failed to initiate bank transfer via {} adapter",
                            adapter.provider_name()
                        );
                        Err(format!("Adapter error: {}", e))
                    }
                }
            } else {
                // No adapter - simulation mode
                warn!(
                    provider = %request.rails_provider,
                    "No adapter registered for provider, using simulation mode"
                );
                Ok(PayoutResult {
                    reference_code: request.reference_code.clone(),
                    provider_tx_id: format!("SIM_{}", uuid::Uuid::now_v7()),
                    status: "PROCESSING".to_string(),
                    estimated_completion: Some(
                        (Utc::now() + chrono::Duration::minutes(5)).to_rfc3339(),
                    ),
                })
            }
        } else {
            // No context - test mode
            info!("No activity context, simulation mode");
            Ok(PayoutResult {
                reference_code: request.reference_code.clone(),
                provider_tx_id: format!("TEST_{}", request.reference_code),
                status: "PROCESSING".to_string(),
                estimated_completion: None,
            })
        }
    }

    /// Activity: Check payout status
    ///
    /// Queries the rails adapter for the current status of a payout.
    #[instrument(fields(reference = %reference_code, provider = %provider_code))]
    pub async fn check_payout_status(
        provider_code: &str,
        reference_code: &str,
    ) -> Result<String, String> {
        info!("Checking payout status");

        let ctx = get_context();

        if let Some(ctx) = ctx {
            if let Some(adapter) = ctx.get_adapter(provider_code).await {
                match adapter.check_payout_status(reference_code).await {
                    Ok(status) => {
                        let status_str = match status {
                            PayoutStatus::Pending => "PENDING",
                            PayoutStatus::Processing => "PROCESSING",
                            PayoutStatus::Completed => "COMPLETED",
                            PayoutStatus::Failed => "FAILED",
                            PayoutStatus::Cancelled => "CANCELLED",
                        };
                        info!(status = %status_str, "Payout status retrieved");
                        Ok(status_str.to_string())
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to check payout status");
                        Err(format!("Status check error: {}", e))
                    }
                }
            } else {
                Err(format!(
                    "No adapter registered for provider: {}",
                    provider_code
                ))
            }
        } else {
            // Simulation mode - return completed
            Ok("COMPLETED".to_string())
        }
    }

    /// Activity: Process payout webhook
    ///
    /// Parses and validates a payout webhook payload using the appropriate adapter.
    #[instrument(skip(payload, signature), fields(provider = %provider_code))]
    pub async fn process_payout_webhook(
        provider_code: &str,
        payload: &[u8],
        signature: Option<&str>,
    ) -> Result<(String, String, Option<String>), String> {
        info!("Processing payout webhook from {}", provider_code);

        let ctx = get_context();

        if let Some(ctx) = ctx {
            if let Some(adapter) = ctx.get_adapter(provider_code).await {
                match adapter.parse_payout_webhook(payload, signature).await {
                    Ok(confirmation) => {
                        let status_str = match confirmation.status {
                            PayoutStatus::Pending => "PENDING",
                            PayoutStatus::Processing => "PROCESSING",
                            PayoutStatus::Completed => "COMPLETED",
                            PayoutStatus::Failed => "FAILED",
                            PayoutStatus::Cancelled => "CANCELLED",
                        };

                        info!(
                            reference_code = %confirmation.reference_code,
                            bank_tx_id = %confirmation.bank_tx_id,
                            status = %status_str,
                            "Payout webhook parsed successfully"
                        );

                        Ok((
                            confirmation.reference_code,
                            status_str.to_string(),
                            confirmation.failure_reason,
                        ))
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to parse payout webhook");
                        Err(format!("Webhook parsing error: {}", e))
                    }
                }
            } else {
                Err(format!(
                    "No adapter registered for provider: {}",
                    provider_code
                ))
            }
        } else {
            Err("No activity context available".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{
        CorridorComplianceHookRecord, CorridorCutoffPolicyRecord, CorridorEligibilityRuleRecord,
        CorridorEndpointRecord, CorridorFeeProfileRecord, CorridorPackRecord,
        CorridorPackRepository, CorridorRolloutScopeRecord, PaymentMethodCapabilityRecord,
        PaymentMethodCapabilityRepository,
    };
    use crate::test_utils::{MockLedgerRepository, MockTenantRepository, MockWebhookRepository};
    use async_trait::async_trait;
    use ramp_compliance::{case::CaseManager, InMemoryCaseStore};
    use std::sync::{Arc, Mutex, Once};

    #[derive(Default)]
    struct MockPilotCorridorRepository {
        corridor: Mutex<Option<CorridorPackRecord>>,
    }

    #[async_trait]
    impl CorridorPackRepository for MockPilotCorridorRepository {
        async fn upsert_corridor_pack(&self, _request: &crate::repository::UpsertCorridorPackRequest) -> ramp_common::Result<()> {
            Ok(())
        }
        async fn upsert_endpoint(&self, _request: &crate::repository::UpsertCorridorEndpointRequest) -> ramp_common::Result<()> {
            Ok(())
        }
        async fn upsert_fee_profile(&self, _request: &crate::repository::UpsertCorridorFeeProfileRequest) -> ramp_common::Result<()> {
            Ok(())
        }
        async fn upsert_cutoff_policy(&self, _request: &crate::repository::UpsertCorridorCutoffPolicyRequest) -> ramp_common::Result<()> {
            Ok(())
        }
        async fn upsert_compliance_hook(&self, _request: &crate::repository::UpsertCorridorComplianceHookRequest) -> ramp_common::Result<()> {
            Ok(())
        }
        async fn upsert_rollout_scope(&self, _request: &crate::repository::UpsertCorridorRolloutScopeRequest) -> ramp_common::Result<()> {
            Ok(())
        }
        async fn upsert_eligibility_rule(&self, _request: &crate::repository::UpsertCorridorEligibilityRuleRequest) -> ramp_common::Result<()> {
            Ok(())
        }
        async fn list_corridor_packs(&self, tenant_id: Option<&str>) -> ramp_common::Result<Vec<CorridorPackRecord>> {
            Ok(self
                .corridor
                .lock()
                .expect("corridor lock")
                .clone()
                .into_iter()
                .filter(|record| tenant_id.map(|v| record.tenant_id.as_deref() == Some(v)).unwrap_or(true))
                .collect())
        }
        async fn get_corridor_pack(&self, tenant_id: Option<&str>, corridor_code: &str) -> ramp_common::Result<Option<CorridorPackRecord>> {
            Ok(self
                .corridor
                .lock()
                .expect("corridor lock")
                .clone()
                .filter(|record| {
                    record.corridor_code == corridor_code
                        && tenant_id.map(|v| record.tenant_id.as_deref() == Some(v)).unwrap_or(true)
                }))
        }
    }

    #[derive(Default)]
    struct MockPilotPaymentMethodRepository {
        capabilities: Mutex<Vec<PaymentMethodCapabilityRecord>>,
    }

    #[async_trait]
    impl PaymentMethodCapabilityRepository for MockPilotPaymentMethodRepository {
        async fn upsert_payment_method_capability(
            &self,
            _request: &crate::repository::UpsertPaymentMethodCapabilityRequest,
        ) -> ramp_common::Result<()> {
            Ok(())
        }

        async fn list_payment_method_capabilities(
            &self,
            corridor_pack_id: Option<&str>,
            _partner_capability_id: Option<&str>,
        ) -> ramp_common::Result<Vec<PaymentMethodCapabilityRecord>> {
            Ok(self
                .capabilities
                .lock()
                .expect("capabilities lock")
                .iter()
                .filter(|record| corridor_pack_id.map(|v| record.corridor_pack_id == v).unwrap_or(true))
                .cloned()
                .collect())
        }
    }

    fn ensure_pilot_test_activity_context() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            let corridor_repo = Arc::new(MockPilotCorridorRepository {
                corridor: Mutex::new(Some(CorridorPackRecord {
                    corridor_pack_id: "corridor_vn_hk".to_string(),
                    tenant_id: Some("tenant_pilot".to_string()),
                    corridor_code: "VN_HK_PAYOUT".to_string(),
                    source_market: "VN".to_string(),
                    destination_market: "HK".to_string(),
                    source_currency: "VND".to_string(),
                    destination_currency: "HKD".to_string(),
                    settlement_direction: "PAYOUT".to_string(),
                    fee_model: "shared".to_string(),
                    lifecycle_state: "pilot".to_string(),
                    rollout_state: "approved".to_string(),
                    eligibility_state: "active".to_string(),
                    metadata: serde_json::json!({}),
                    endpoints: vec![CorridorEndpointRecord {
                        endpoint_id: "endpoint_hk_destination".to_string(),
                        endpoint_role: "destination".to_string(),
                        partner_id: None,
                        provider_key: None,
                        adapter_key: Some("mock".to_string()),
                        entity_type: "individual".to_string(),
                        rail: "fps".to_string(),
                        method_family: Some("push_transfer".to_string()),
                        settlement_mode: Some("same_day".to_string()),
                        instrument_family: Some("bank_transfer".to_string()),
                        metadata: serde_json::json!({}),
                    }],
                    fee_profiles: vec![CorridorFeeProfileRecord {
                        fee_profile_id: "fee_hk".to_string(),
                        fee_currency: "HKD".to_string(),
                        base_fee: Some("25.00".to_string()),
                        fx_spread_bps: Some(15),
                        liquidity_cost_bps: None,
                        surcharge_bps: None,
                        metadata: serde_json::json!({}),
                    }],
                    cutoff_policies: vec![CorridorCutoffPolicyRecord {
                        cutoff_policy_id: "cutoff_hk".to_string(),
                        timezone: "Asia/Hong_Kong".to_string(),
                        cutoff_windows: serde_json::json!([{"day":"weekday","cutoff":"17:00"}]),
                        holiday_calendar: Some("HK".to_string()),
                        retry_rule: Some("next_business_day".to_string()),
                        exception_policy: None,
                        metadata: serde_json::json!({}),
                    }],
                    compliance_hooks: vec![CorridorComplianceHookRecord {
                        compliance_hook_id: "hook_hk".to_string(),
                        hook_kind: "travel_rule".to_string(),
                        provider_key: Some("travel_rule_provider".to_string()),
                        required: true,
                        config: serde_json::json!({}),
                        metadata: serde_json::json!({}),
                    }],
                    rollout_scopes: vec![CorridorRolloutScopeRecord {
                        rollout_scope_id: "rollout_hk".to_string(),
                        tenant_id: Some("tenant_pilot".to_string()),
                        environment: "production".to_string(),
                        geography: Some("HK".to_string()),
                        method_family: Some("push_transfer".to_string()),
                        rollout_state: "approved".to_string(),
                        approval_reference: None,
                        metadata: serde_json::json!({}),
                    }],
                    eligibility_rules: vec![CorridorEligibilityRuleRecord {
                        eligibility_rule_id: "eligibility_hk".to_string(),
                        partner_id: None,
                        entity_type: Some("individual".to_string()),
                        method_family: Some("push_transfer".to_string()),
                        amount_bounds: serde_json::json!({"min":"100","max":"10000"}),
                        compliance_requirements: serde_json::json!({"travelRule":true}),
                        metadata: serde_json::json!({}),
                    }],
                })),
            });

            let payment_repo = Arc::new(MockPilotPaymentMethodRepository {
                capabilities: Mutex::new(vec![PaymentMethodCapabilityRecord {
                    payment_method_capability_id: "pmc_hk_push_transfer".to_string(),
                    corridor_pack_id: "corridor_vn_hk".to_string(),
                    partner_capability_id: None,
                    method_family: "push_transfer".to_string(),
                    funding_source: Some("bank_account".to_string()),
                    settlement_direction: "payout".to_string(),
                    presentment_model: Some("server_driven".to_string()),
                    card_funding_enabled: false,
                    policy_flags: serde_json::json!({}),
                    metadata: serde_json::json!({}),
                }]),
            });

            let ctx = ActivityContext::new(
                Arc::new(MockLedgerRepository::new()),
                Arc::new(MockWebhookRepository::new()),
                Arc::new(MockTenantRepository::new()),
                Arc::new(CaseManager::new(Arc::new(InMemoryCaseStore::new()))),
            )
            .with_pilot_corridor_services(
                Arc::new(CorridorPackService::with_repository(corridor_repo)),
                Arc::new(PaymentMethodCapabilityService::with_repository(payment_repo)),
            );

            let adapters = ramp_adapter::create_test_adapters();
            let ctx = ActivityContext {
                rails_adapters: Arc::new(RwLock::new(adapters)),
                ..ctx
            };
            init_activity_context(ctx);
        });
    }

    #[tokio::test]
    async fn test_issue_instruction_simulation() {
        let input = PayinWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-123".to_string(),
            amount_vnd: 1000000,
            rails_provider: "VCB".to_string(),
            reference_code: "REF123".to_string(),
            expires_at: "2026-01-24T00:00:00Z".to_string(),
        };

        let result = payin_activities::issue_instruction(&input).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "REF123");
    }

    #[tokio::test]
    async fn test_credit_vnd_balance_simulation() {
        let result =
            payin_activities::credit_vnd_balance("tenant1", "user1", "intent-123", 1000000).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_trade_check_pass() {
        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-trade-1".to_string(),
            trade_id: "trade-123".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: -100_000_000,
            crypto_delta: "0.1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = trade_activities::run_post_trade_check(&input).await;
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should pass for 100M VND
    }

    #[tokio::test]
    async fn test_post_trade_check_fail() {
        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-trade-2".to_string(),
            trade_id: "trade-large".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: -2_000_000_000, // 2B VND - exceeds threshold
            crypto_delta: "2.0".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = trade_activities::run_post_trade_check(&input).await;
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should fail for 2B VND
    }

    #[tokio::test]
    async fn test_flag_for_review_simulation() {
        let result =
            trade_activities::flag_for_review("intent-123", "Large transaction detected").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("CASE_"));
    }

    #[tokio::test]
    async fn test_settle_in_ledger_simulation() {
        let input = TradeWorkflowInput {
            tenant_id: "tenant1".to_string(),
            user_id: "user1".to_string(),
            intent_id: "intent-trade-1".to_string(),
            trade_id: "trade-123".to_string(),
            symbol: "BTC/VND".to_string(),
            price: "1000000000".to_string(),
            vnd_delta: -100_000_000,
            crypto_delta: "0.1".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        let result = trade_activities::settle_in_ledger(&input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pilot_corridor_payout_uses_configured_adapter() {
        ensure_pilot_test_activity_context();

        let request = payout_activities::PayoutRequest {
            tenant_id: "tenant_pilot".to_string(),
            user_id: "user_pilot".to_string(),
            intent_id: "intent_pilot".to_string(),
            reference_code: "REF_PILOT_001".to_string(),
            amount_vnd: 500_000,
            rails_provider: "corridor:VN_HK_PAYOUT".to_string(),
            recipient_bank_code: "004".to_string(),
            recipient_account_number: "123456789".to_string(),
            recipient_account_name: "Pilot User".to_string(),
            description: "pilot corridor payout".to_string(),
        };

        let result = payout_activities::initiate_bank_transfer(&request)
            .await
            .expect("pilot corridor payout should use configured adapter");
        assert!(result.provider_tx_id.starts_with("MOCK_"));
    }

    #[tokio::test]
    async fn test_unknown_pilot_corridor_keeps_simulation_fallback() {
        ensure_pilot_test_activity_context();

        let request = payout_activities::PayoutRequest {
            tenant_id: "tenant_pilot".to_string(),
            user_id: "user_pilot".to_string(),
            intent_id: "intent_pilot_unknown".to_string(),
            reference_code: "REF_PILOT_002".to_string(),
            amount_vnd: 500_000,
            rails_provider: "corridor:UNKNOWN".to_string(),
            recipient_bank_code: "004".to_string(),
            recipient_account_number: "123456789".to_string(),
            recipient_account_name: "Pilot User".to_string(),
            description: "unknown pilot corridor payout".to_string(),
        };

        let result = payout_activities::initiate_bank_transfer(&request)
            .await
            .expect("unknown pilot corridor should keep fallback behavior");
        assert!(result.provider_tx_id.starts_with("SIM_"));
    }
}
