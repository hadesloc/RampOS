//! Withdraw Policy Engine
//!
//! Comprehensive policy checking for cryptocurrency withdrawals including:
//! - KYC tier-based limits
//! - Daily/monthly velocity limits
//! - AML velocity checks
//! - Sanctions screening

use async_trait::async_trait;
use chrono::{Duration, Utc};
use ramp_common::{
    types::{IntentId, TenantId, UserId, WalletAddress},
    Result,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

use crate::{
    case::CaseManager,
    sanctions::SanctionsProvider,
    transaction_history::TransactionHistoryStore,
    types::{CaseSeverity, CaseType, KycTier},
};

/// Result of a policy check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyResult {
    /// Withdrawal is approved
    Approved,
    /// Withdrawal is denied with a reason
    Denied { reason: String, code: DenialCode },
    /// Withdrawal requires manual review
    ManualReview {
        reason: String,
        case_id: Option<String>,
    },
}

impl PolicyResult {
    pub fn is_approved(&self) -> bool {
        matches!(self, PolicyResult::Approved)
    }

    pub fn is_denied(&self) -> bool {
        matches!(self, PolicyResult::Denied { .. })
    }

    pub fn is_manual_review(&self) -> bool {
        matches!(self, PolicyResult::ManualReview { .. })
    }
}

/// Denial codes for policy rejections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenialCode {
    /// User KYC tier is insufficient
    InsufficientKycTier,
    /// User KYC is not verified
    KycNotVerified,
    /// Single transaction limit exceeded
    SingleTransactionLimitExceeded,
    /// Daily withdrawal limit exceeded
    DailyLimitExceeded,
    /// Monthly withdrawal limit exceeded
    MonthlyLimitExceeded,
    /// Velocity check failed
    VelocityCheckFailed,
    /// Sanctions match found
    SanctionsMatch,
    /// Destination address is blacklisted
    BlacklistedAddress,
    /// Cooling off period not met for new address
    CoolingOffPeriod,
    /// User is blocked
    UserBlocked,
    /// General policy violation
    PolicyViolation,
}

/// Withdrawal policy configuration per tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierWithdrawLimits {
    /// Maximum single withdrawal amount (in crypto units, converted to VND for comparison)
    pub single_transaction_limit_vnd: Decimal,
    /// Daily withdrawal limit
    pub daily_limit_vnd: Decimal,
    /// Monthly withdrawal limit
    pub monthly_limit_vnd: Decimal,
    /// Maximum number of withdrawals per day
    pub max_daily_count: u32,
    /// Maximum number of withdrawals per hour
    pub max_hourly_count: u32,
    /// Cooling off period for new addresses (in hours)
    pub new_address_cooling_hours: u32,
    /// Whether manual review is required for amounts above this threshold
    pub manual_review_threshold_vnd: Option<Decimal>,
}

impl TierWithdrawLimits {
    /// Get default limits for a KYC tier
    pub fn for_tier(tier: KycTier) -> Self {
        match tier {
            KycTier::Tier0 => Self {
                single_transaction_limit_vnd: Decimal::ZERO, // Cannot withdraw
                daily_limit_vnd: Decimal::ZERO,
                monthly_limit_vnd: Decimal::ZERO,
                max_daily_count: 0,
                max_hourly_count: 0,
                new_address_cooling_hours: 0,
                manual_review_threshold_vnd: None,
            },
            KycTier::Tier1 => Self {
                single_transaction_limit_vnd: Decimal::from(10_000_000), // 10M VND
                daily_limit_vnd: Decimal::from(20_000_000),              // 20M VND
                monthly_limit_vnd: Decimal::from(200_000_000),           // 200M VND
                max_daily_count: 5,
                max_hourly_count: 2,
                new_address_cooling_hours: 24,
                manual_review_threshold_vnd: Some(Decimal::from(15_000_000)), // 15M VND
            },
            KycTier::Tier2 => Self {
                single_transaction_limit_vnd: Decimal::from(100_000_000), // 100M VND
                daily_limit_vnd: Decimal::from(200_000_000),              // 200M VND
                monthly_limit_vnd: Decimal::from(2_000_000_000),          // 2B VND
                max_daily_count: 10,
                max_hourly_count: 5,
                new_address_cooling_hours: 12,
                manual_review_threshold_vnd: Some(Decimal::from(150_000_000)), // 150M VND
            },
            KycTier::Tier3 => Self {
                single_transaction_limit_vnd: Decimal::from(1_000_000_000), // 1B VND
                daily_limit_vnd: Decimal::MAX,                              // Unlimited
                monthly_limit_vnd: Decimal::MAX,                            // Unlimited
                max_daily_count: 100,
                max_hourly_count: 20,
                new_address_cooling_hours: 0,
                manual_review_threshold_vnd: Some(Decimal::from(500_000_000)), // 500M VND
            },
        }
    }
}

/// Global velocity thresholds for AML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityThresholds {
    /// Maximum total amount in 24 hours before flagging
    pub daily_total_vnd: Decimal,
    /// Maximum number of transactions in 1 hour
    pub hourly_count: u32,
    /// Maximum total amount in 1 hour before flagging
    pub hourly_total_vnd: Decimal,
    /// Minimum time between withdrawals (in minutes)
    pub min_interval_minutes: u32,
}

impl Default for VelocityThresholds {
    fn default() -> Self {
        Self {
            daily_total_vnd: Decimal::from(100_000_000), // 100M VND
            hourly_count: 5,
            hourly_total_vnd: Decimal::from(50_000_000), // 50M VND
            min_interval_minutes: 5,
        }
    }
}

/// Configuration for the withdraw policy engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithdrawPolicyConfig {
    /// Per-tier limits (if None, uses defaults)
    pub tier_limits: Option<std::collections::HashMap<i16, TierWithdrawLimits>>,
    /// Velocity thresholds
    pub velocity: VelocityThresholds,
    /// Whether to enable sanctions screening
    pub enable_sanctions_screening: bool,
    /// Whether to enable AML velocity checks
    pub enable_aml_checks: bool,
    /// Blacklisted destination addresses
    pub blacklisted_addresses: Vec<String>,
    /// Whether to require cooling off for new addresses
    pub require_address_cooling: bool,
}

impl Default for WithdrawPolicyConfig {
    fn default() -> Self {
        Self {
            tier_limits: None,
            velocity: VelocityThresholds::default(),
            enable_sanctions_screening: true,
            enable_aml_checks: true,
            blacklisted_addresses: vec![],
            require_address_cooling: true,
        }
    }
}

/// Withdrawal request data for policy checking
#[derive(Debug, Clone)]
pub struct WithdrawPolicyRequest {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub intent_id: IntentId,
    /// Amount in VND equivalent
    pub amount_vnd: Decimal,
    /// Destination wallet address
    pub to_address: WalletAddress,
    /// User's current KYC tier
    pub kyc_tier: KycTier,
    /// User's KYC status
    pub kyc_status: String,
    /// User's full name for sanctions screening
    pub user_full_name: Option<String>,
    /// User's country for sanctions screening
    pub user_country: Option<String>,
    /// Whether this is a new/first-time destination address
    pub is_new_address: bool,
    /// When the address was first used (if not new)
    pub address_first_used: Option<chrono::DateTime<Utc>>,
}

/// Data provider for withdraw policy engine
#[async_trait]
pub trait WithdrawPolicyDataProvider: Send + Sync {
    /// Get total withdrawal amount for a user today
    async fn get_daily_withdraw_amount(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Decimal>;

    /// Get total withdrawal amount for a user this month
    async fn get_monthly_withdraw_amount(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Decimal>;

    /// Get count of withdrawals in the last hour
    async fn get_hourly_withdraw_count(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<u32>;

    /// Get count of withdrawals today
    async fn get_daily_withdraw_count(&self, tenant_id: &TenantId, user_id: &UserId)
        -> Result<u32>;

    /// Get the last withdrawal timestamp
    async fn get_last_withdraw_time(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Option<chrono::DateTime<Utc>>>;
}

/// Withdraw Policy Engine
pub struct WithdrawPolicyEngine {
    config: WithdrawPolicyConfig,
    case_manager: Arc<CaseManager>,
    sanctions_provider: Option<Arc<dyn SanctionsProvider>>,
    transaction_store: Arc<dyn TransactionHistoryStore>,
    data_provider: Option<Arc<dyn WithdrawPolicyDataProvider>>,
}

impl WithdrawPolicyEngine {
    pub fn new(
        config: WithdrawPolicyConfig,
        case_manager: Arc<CaseManager>,
        sanctions_provider: Option<Arc<dyn SanctionsProvider>>,
        transaction_store: Arc<dyn TransactionHistoryStore>,
    ) -> Self {
        Self {
            config,
            case_manager,
            sanctions_provider,
            transaction_store,
            data_provider: None,
        }
    }

    pub fn with_data_provider(mut self, provider: Arc<dyn WithdrawPolicyDataProvider>) -> Self {
        self.data_provider = Some(provider);
        self
    }

    /// Get tier limits for a given KYC tier
    fn get_tier_limits(&self, tier: KycTier) -> TierWithdrawLimits {
        if let Some(ref custom_limits) = self.config.tier_limits {
            if let Some(limits) = custom_limits.get(&(tier as i16)) {
                return limits.clone();
            }
        }
        TierWithdrawLimits::for_tier(tier)
    }

    /// Check withdrawal policy
    pub async fn check_policy(&self, req: &WithdrawPolicyRequest) -> Result<PolicyResult> {
        info!(
            intent_id = %req.intent_id,
            user_id = %req.user_id,
            amount_vnd = %req.amount_vnd,
            kyc_tier = ?req.kyc_tier,
            "Starting withdraw policy check"
        );

        // 1. Check KYC status
        if req.kyc_status != "VERIFIED" && req.kyc_status != "APPROVED" {
            warn!(
                intent_id = %req.intent_id,
                kyc_status = %req.kyc_status,
                "Withdrawal denied: KYC not verified"
            );
            return Ok(PolicyResult::Denied {
                reason: "User KYC is not verified".to_string(),
                code: DenialCode::KycNotVerified,
            });
        }

        // 2. Check KYC tier allows withdrawals
        let tier_limits = self.get_tier_limits(req.kyc_tier);
        if tier_limits.single_transaction_limit_vnd.is_zero() {
            warn!(
                intent_id = %req.intent_id,
                kyc_tier = ?req.kyc_tier,
                "Withdrawal denied: KYC tier insufficient"
            );
            return Ok(PolicyResult::Denied {
                reason: format!("KYC tier {:?} does not allow withdrawals", req.kyc_tier),
                code: DenialCode::InsufficientKycTier,
            });
        }

        // 3. Check single transaction limit
        if req.amount_vnd > tier_limits.single_transaction_limit_vnd {
            warn!(
                intent_id = %req.intent_id,
                amount = %req.amount_vnd,
                limit = %tier_limits.single_transaction_limit_vnd,
                "Withdrawal denied: Single transaction limit exceeded"
            );
            return Ok(PolicyResult::Denied {
                reason: format!(
                    "Amount {} VND exceeds single transaction limit of {} VND for tier {:?}",
                    req.amount_vnd, tier_limits.single_transaction_limit_vnd, req.kyc_tier
                ),
                code: DenialCode::SingleTransactionLimitExceeded,
            });
        }

        // 4. Check blacklisted addresses
        let to_addr_lower = req.to_address.0.to_lowercase();
        if self
            .config
            .blacklisted_addresses
            .iter()
            .any(|a| a.to_lowercase() == to_addr_lower)
        {
            warn!(
                intent_id = %req.intent_id,
                to_address = %req.to_address,
                "Withdrawal denied: Destination address is blacklisted"
            );
            return Ok(PolicyResult::Denied {
                reason: "Destination address is blacklisted".to_string(),
                code: DenialCode::BlacklistedAddress,
            });
        }

        // 5. Check cooling off period for new addresses
        if self.config.require_address_cooling
            && tier_limits.new_address_cooling_hours > 0
            && req.is_new_address
        {
            warn!(
                intent_id = %req.intent_id,
                to_address = %req.to_address,
                cooling_hours = tier_limits.new_address_cooling_hours,
                "Withdrawal requires manual review: New address cooling period"
            );

            let case_id = self
                .case_manager
                .create_case(
                    &req.tenant_id,
                    Some(&req.user_id),
                    Some(&req.intent_id),
                    CaseType::Other("new_address_withdrawal".to_string()),
                    CaseSeverity::Medium,
                    serde_json::json!({
                        "reason": "new_address_cooling_period",
                        "to_address": req.to_address.0,
                        "amount_vnd": req.amount_vnd.to_string(),
                        "required_cooling_hours": tier_limits.new_address_cooling_hours
                    }),
                )
                .await?;

            return Ok(PolicyResult::ManualReview {
                reason: format!(
                    "New destination address requires {} hour cooling period",
                    tier_limits.new_address_cooling_hours
                ),
                case_id: Some(case_id),
            });
        }

        // 6. Check velocity limits if data provider is available
        if let Some(ref data_provider) = self.data_provider {
            // Check daily limit
            let daily_amount = data_provider
                .get_daily_withdraw_amount(&req.tenant_id, &req.user_id)
                .await?;

            if daily_amount + req.amount_vnd > tier_limits.daily_limit_vnd {
                warn!(
                    intent_id = %req.intent_id,
                    daily_amount = %daily_amount,
                    requested = %req.amount_vnd,
                    limit = %tier_limits.daily_limit_vnd,
                    "Withdrawal denied: Daily limit exceeded"
                );
                return Ok(PolicyResult::Denied {
                    reason: format!(
                        "Daily withdrawal limit exceeded. Current: {} VND, Requested: {} VND, Limit: {} VND",
                        daily_amount, req.amount_vnd, tier_limits.daily_limit_vnd
                    ),
                    code: DenialCode::DailyLimitExceeded,
                });
            }

            // Check monthly limit
            let monthly_amount = data_provider
                .get_monthly_withdraw_amount(&req.tenant_id, &req.user_id)
                .await?;

            if monthly_amount + req.amount_vnd > tier_limits.monthly_limit_vnd {
                warn!(
                    intent_id = %req.intent_id,
                    monthly_amount = %monthly_amount,
                    requested = %req.amount_vnd,
                    limit = %tier_limits.monthly_limit_vnd,
                    "Withdrawal denied: Monthly limit exceeded"
                );
                return Ok(PolicyResult::Denied {
                    reason: format!(
                        "Monthly withdrawal limit exceeded. Current: {} VND, Requested: {} VND, Limit: {} VND",
                        monthly_amount, req.amount_vnd, tier_limits.monthly_limit_vnd
                    ),
                    code: DenialCode::MonthlyLimitExceeded,
                });
            }

            // Check hourly count
            let hourly_count = data_provider
                .get_hourly_withdraw_count(&req.tenant_id, &req.user_id)
                .await?;

            if hourly_count >= tier_limits.max_hourly_count {
                warn!(
                    intent_id = %req.intent_id,
                    hourly_count = hourly_count,
                    limit = tier_limits.max_hourly_count,
                    "Withdrawal denied: Hourly count limit exceeded"
                );
                return Ok(PolicyResult::Denied {
                    reason: format!(
                        "Too many withdrawals this hour. Count: {}, Limit: {}",
                        hourly_count, tier_limits.max_hourly_count
                    ),
                    code: DenialCode::VelocityCheckFailed,
                });
            }

            // Check daily count
            let daily_count = data_provider
                .get_daily_withdraw_count(&req.tenant_id, &req.user_id)
                .await?;

            if daily_count >= tier_limits.max_daily_count {
                warn!(
                    intent_id = %req.intent_id,
                    daily_count = daily_count,
                    limit = tier_limits.max_daily_count,
                    "Withdrawal denied: Daily count limit exceeded"
                );
                return Ok(PolicyResult::Denied {
                    reason: format!(
                        "Too many withdrawals today. Count: {}, Limit: {}",
                        daily_count, tier_limits.max_daily_count
                    ),
                    code: DenialCode::VelocityCheckFailed,
                });
            }

            // Check minimum interval
            if self.config.velocity.min_interval_minutes > 0 {
                if let Some(last_time) = data_provider
                    .get_last_withdraw_time(&req.tenant_id, &req.user_id)
                    .await?
                {
                    let min_interval =
                        Duration::minutes(self.config.velocity.min_interval_minutes as i64);
                    let time_since_last = Utc::now() - last_time;

                    if time_since_last < min_interval {
                        warn!(
                            intent_id = %req.intent_id,
                            minutes_since_last = time_since_last.num_minutes(),
                            min_interval = self.config.velocity.min_interval_minutes,
                            "Withdrawal denied: Minimum interval not met"
                        );
                        return Ok(PolicyResult::Denied {
                            reason: format!(
                                "Please wait {} minutes between withdrawals",
                                self.config.velocity.min_interval_minutes
                            ),
                            code: DenialCode::VelocityCheckFailed,
                        });
                    }
                }
            }
        }

        // 7. Run AML velocity check
        if self.config.enable_aml_checks {
            let now = Utc::now();
            let hour_ago = now - Duration::hours(1);

            let (hourly_count, hourly_total) = self
                .transaction_store
                .stats_since(&req.tenant_id, &req.user_id, hour_ago)
                .await?;

            if hourly_count >= self.config.velocity.hourly_count
                || hourly_total >= self.config.velocity.hourly_total_vnd
            {
                warn!(
                    intent_id = %req.intent_id,
                    hourly_count = hourly_count,
                    hourly_total = %hourly_total,
                    "Withdrawal requires manual review: AML velocity threshold exceeded"
                );

                let case_id = self
                    .case_manager
                    .create_case(
                        &req.tenant_id,
                        Some(&req.user_id),
                        Some(&req.intent_id),
                        CaseType::Velocity,
                        CaseSeverity::High,
                        serde_json::json!({
                            "reason": "velocity_threshold_exceeded",
                            "hourly_count": hourly_count,
                            "hourly_total_vnd": hourly_total.to_string(),
                            "requested_amount_vnd": req.amount_vnd.to_string(),
                            "thresholds": {
                                "max_hourly_count": self.config.velocity.hourly_count,
                                "max_hourly_total_vnd": self.config.velocity.hourly_total_vnd.to_string()
                            }
                        }),
                    )
                    .await?;

                return Ok(PolicyResult::ManualReview {
                    reason: "Velocity threshold exceeded - requires manual review".to_string(),
                    case_id: Some(case_id),
                });
            }
        }

        // 8. Run sanctions screening
        if self.config.enable_sanctions_screening {
            if let Some(ref provider) = self.sanctions_provider {
                // Screen destination address
                match provider.check_address(&req.to_address.0).await {
                    Ok(result) if result.matched => {
                        warn!(
                            intent_id = %req.intent_id,
                            to_address = %req.to_address,
                            "Withdrawal denied: Destination address sanctions match"
                        );

                        let _case_id = self
                            .case_manager
                            .create_case(
                                &req.tenant_id,
                                Some(&req.user_id),
                                Some(&req.intent_id),
                                CaseType::SanctionsMatch,
                                CaseSeverity::Critical,
                                serde_json::json!({
                                    "reason": "sanctions_match",
                                    "type": "address",
                                    "address": "***REDACTED***",
                                    "list_name": result.list_name,
                                    "score": result.score
                                }),
                            )
                            .await?;

                        return Ok(PolicyResult::Denied {
                            reason: "Destination address matches sanctions list".to_string(),
                            code: DenialCode::SanctionsMatch,
                        });
                    }
                    Ok(_) => {} // Clean
                    Err(e) => {
                        warn!(
                            intent_id = %req.intent_id,
                            error = %e,
                            "Sanctions screening failed, requiring manual review"
                        );

                        let case_id = self
                            .case_manager
                            .create_case(
                                &req.tenant_id,
                                Some(&req.user_id),
                                Some(&req.intent_id),
                                CaseType::Other("sanctions_screening_failed".to_string()),
                                CaseSeverity::High,
                                serde_json::json!({
                                    "reason": "sanctions_screening_failed",
                                    "error": e.to_string()
                                }),
                            )
                            .await?;

                        return Ok(PolicyResult::ManualReview {
                            reason: "Sanctions screening failed - requires manual review"
                                .to_string(),
                            case_id: Some(case_id),
                        });
                    }
                }

                // Screen user name if available
                if let Some(ref name) = req.user_full_name {
                    match provider
                        .check_individual(name, None, req.user_country.as_deref())
                        .await
                    {
                        Ok(result) if result.matched => {
                            warn!(
                                intent_id = %req.intent_id,
                                "Withdrawal denied: User name sanctions match"
                            );

                            let _case_id = self
                                .case_manager
                                .create_case(
                                    &req.tenant_id,
                                    Some(&req.user_id),
                                    Some(&req.intent_id),
                                    CaseType::SanctionsMatch,
                                    CaseSeverity::Critical,
                                    serde_json::json!({
                                        "reason": "sanctions_match",
                                        "type": "individual",
                                        "name": "***REDACTED***",
                                        "list_name": result.list_name,
                                        "score": result.score
                                    }),
                                )
                                .await?;

                            return Ok(PolicyResult::Denied {
                                reason: "User name matches sanctions list".to_string(),
                                code: DenialCode::SanctionsMatch,
                            });
                        }
                        Ok(_) => {} // Clean
                        Err(e) => {
                            warn!(
                                intent_id = %req.intent_id,
                                error = %e,
                                "User sanctions screening failed, requiring manual review"
                            );

                            let case_id = self
                                .case_manager
                                .create_case(
                                    &req.tenant_id,
                                    Some(&req.user_id),
                                    Some(&req.intent_id),
                                    CaseType::Other("sanctions_screening_failed".to_string()),
                                    CaseSeverity::High,
                                    serde_json::json!({
                                        "reason": "user_sanctions_screening_failed",
                                        "error": e.to_string()
                                    }),
                                )
                                .await?;

                            return Ok(PolicyResult::ManualReview {
                                reason: "User sanctions screening failed - requires manual review"
                                    .to_string(),
                                case_id: Some(case_id),
                            });
                        }
                    }
                }
            }
        }

        // 9. Check if amount exceeds manual review threshold
        if let Some(threshold) = tier_limits.manual_review_threshold_vnd {
            if req.amount_vnd > threshold {
                warn!(
                    intent_id = %req.intent_id,
                    amount = %req.amount_vnd,
                    threshold = %threshold,
                    "Withdrawal requires manual review: Large amount"
                );

                let case_id = self
                    .case_manager
                    .create_case(
                        &req.tenant_id,
                        Some(&req.user_id),
                        Some(&req.intent_id),
                        CaseType::LargeTransaction,
                        CaseSeverity::Medium,
                        serde_json::json!({
                            "reason": "large_withdrawal",
                            "amount_vnd": req.amount_vnd.to_string(),
                            "threshold_vnd": threshold.to_string(),
                            "kyc_tier": format!("{:?}", req.kyc_tier)
                        }),
                    )
                    .await?;

                return Ok(PolicyResult::ManualReview {
                    reason: format!(
                        "Withdrawal amount {} VND exceeds manual review threshold of {} VND",
                        req.amount_vnd, threshold
                    ),
                    case_id: Some(case_id),
                });
            }
        }

        // All checks passed
        info!(
            intent_id = %req.intent_id,
            "Withdrawal policy check passed"
        );

        Ok(PolicyResult::Approved)
    }
}

/// Mock data provider for testing
#[cfg(any(test, feature = "testing"))]
pub struct MockWithdrawPolicyDataProvider {
    pub daily_amount: std::sync::Mutex<Decimal>,
    pub monthly_amount: std::sync::Mutex<Decimal>,
    pub hourly_count: std::sync::Mutex<u32>,
    pub daily_count: std::sync::Mutex<u32>,
    pub last_withdraw_time: std::sync::Mutex<Option<chrono::DateTime<Utc>>>,
}

#[cfg(any(test, feature = "testing"))]
impl MockWithdrawPolicyDataProvider {
    pub fn new() -> Self {
        Self {
            daily_amount: std::sync::Mutex::new(Decimal::ZERO),
            monthly_amount: std::sync::Mutex::new(Decimal::ZERO),
            hourly_count: std::sync::Mutex::new(0),
            daily_count: std::sync::Mutex::new(0),
            last_withdraw_time: std::sync::Mutex::new(None),
        }
    }

    pub fn with_daily_amount(self, amount: Decimal) -> Self {
        *self.daily_amount.lock().unwrap() = amount;
        self
    }

    pub fn with_monthly_amount(self, amount: Decimal) -> Self {
        *self.monthly_amount.lock().unwrap() = amount;
        self
    }

    pub fn with_hourly_count(self, count: u32) -> Self {
        *self.hourly_count.lock().unwrap() = count;
        self
    }

    pub fn with_daily_count(self, count: u32) -> Self {
        *self.daily_count.lock().unwrap() = count;
        self
    }

    pub fn with_last_withdraw_time(self, time: chrono::DateTime<Utc>) -> Self {
        *self.last_withdraw_time.lock().unwrap() = Some(time);
        self
    }
}

#[cfg(any(test, feature = "testing"))]
impl Default for MockWithdrawPolicyDataProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "testing"))]
#[async_trait]
impl WithdrawPolicyDataProvider for MockWithdrawPolicyDataProvider {
    async fn get_daily_withdraw_amount(
        &self,
        _tenant_id: &TenantId,
        _user_id: &UserId,
    ) -> Result<Decimal> {
        Ok(*self.daily_amount.lock().unwrap())
    }

    async fn get_monthly_withdraw_amount(
        &self,
        _tenant_id: &TenantId,
        _user_id: &UserId,
    ) -> Result<Decimal> {
        Ok(*self.monthly_amount.lock().unwrap())
    }

    async fn get_hourly_withdraw_count(
        &self,
        _tenant_id: &TenantId,
        _user_id: &UserId,
    ) -> Result<u32> {
        Ok(*self.hourly_count.lock().unwrap())
    }

    async fn get_daily_withdraw_count(
        &self,
        _tenant_id: &TenantId,
        _user_id: &UserId,
    ) -> Result<u32> {
        Ok(*self.daily_count.lock().unwrap())
    }

    async fn get_last_withdraw_time(
        &self,
        _tenant_id: &TenantId,
        _user_id: &UserId,
    ) -> Result<Option<chrono::DateTime<Utc>>> {
        Ok(*self.last_withdraw_time.lock().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::mock::InMemoryCaseStore;
    use crate::transaction_history::MockTransactionHistoryStore;

    fn create_test_engine() -> WithdrawPolicyEngine {
        let case_store = Arc::new(InMemoryCaseStore::new());
        let case_manager = Arc::new(CaseManager::new(case_store));
        let transaction_store: Arc<dyn TransactionHistoryStore> =
            Arc::new(MockTransactionHistoryStore::new());

        WithdrawPolicyEngine::new(
            WithdrawPolicyConfig::default(),
            case_manager,
            None, // No sanctions provider for basic tests
            transaction_store,
        )
    }

    fn create_test_request() -> WithdrawPolicyRequest {
        WithdrawPolicyRequest {
            tenant_id: TenantId::new("tenant1"),
            user_id: UserId::new("user1"),
            intent_id: IntentId::new_withdraw(),
            amount_vnd: Decimal::from(5_000_000), // 5M VND
            to_address: WalletAddress::new("0x1234567890123456789012345678901234567890"),
            kyc_tier: KycTier::Tier1,
            kyc_status: "VERIFIED".to_string(),
            user_full_name: Some("Test User".to_string()),
            user_country: Some("VN".to_string()),
            is_new_address: false,
            address_first_used: Some(Utc::now() - Duration::days(30)),
        }
    }

    #[tokio::test]
    async fn test_policy_approved_for_valid_request() {
        let engine = create_test_engine();
        let req = create_test_request();

        let result = engine.check_policy(&req).await.unwrap();
        assert!(result.is_approved());
    }

    #[tokio::test]
    async fn test_policy_denied_for_unverified_kyc() {
        let engine = create_test_engine();
        let mut req = create_test_request();
        req.kyc_status = "PENDING".to_string();

        let result = engine.check_policy(&req).await.unwrap();
        assert!(result.is_denied());
        if let PolicyResult::Denied { code, .. } = result {
            assert_eq!(code, DenialCode::KycNotVerified);
        }
    }

    #[tokio::test]
    async fn test_policy_denied_for_tier0() {
        let engine = create_test_engine();
        let mut req = create_test_request();
        req.kyc_tier = KycTier::Tier0;

        let result = engine.check_policy(&req).await.unwrap();
        assert!(result.is_denied());
        if let PolicyResult::Denied { code, .. } = result {
            assert_eq!(code, DenialCode::InsufficientKycTier);
        }
    }

    #[tokio::test]
    async fn test_policy_denied_for_exceeding_single_limit() {
        let engine = create_test_engine();
        let mut req = create_test_request();
        req.amount_vnd = Decimal::from(50_000_000); // 50M VND, exceeds Tier1 limit of 10M

        let result = engine.check_policy(&req).await.unwrap();
        assert!(result.is_denied());
        if let PolicyResult::Denied { code, .. } = result {
            assert_eq!(code, DenialCode::SingleTransactionLimitExceeded);
        }
    }

    #[tokio::test]
    async fn test_policy_denied_for_blacklisted_address() {
        let case_store = Arc::new(InMemoryCaseStore::new());
        let case_manager = Arc::new(CaseManager::new(case_store));
        let transaction_store: Arc<dyn TransactionHistoryStore> =
            Arc::new(MockTransactionHistoryStore::new());

        let config = WithdrawPolicyConfig {
            blacklisted_addresses: vec!["0x1234567890123456789012345678901234567890".to_string()],
            ..Default::default()
        };

        let engine = WithdrawPolicyEngine::new(config, case_manager, None, transaction_store);

        let req = create_test_request();
        let result = engine.check_policy(&req).await.unwrap();
        assert!(result.is_denied());
        if let PolicyResult::Denied { code, .. } = result {
            assert_eq!(code, DenialCode::BlacklistedAddress);
        }
    }

    #[tokio::test]
    async fn test_policy_manual_review_for_new_address() {
        let case_store = Arc::new(InMemoryCaseStore::new());
        let case_manager = Arc::new(CaseManager::new(case_store));
        let transaction_store: Arc<dyn TransactionHistoryStore> =
            Arc::new(MockTransactionHistoryStore::new());

        let config = WithdrawPolicyConfig {
            require_address_cooling: true,
            ..Default::default()
        };

        let engine = WithdrawPolicyEngine::new(config, case_manager, None, transaction_store);

        let mut req = create_test_request();
        req.is_new_address = true;
        req.address_first_used = None;

        let result = engine.check_policy(&req).await.unwrap();
        assert!(result.is_manual_review());
    }

    #[tokio::test]
    async fn test_policy_denied_for_daily_limit_exceeded() {
        let case_store = Arc::new(InMemoryCaseStore::new());
        let case_manager = Arc::new(CaseManager::new(case_store));
        let transaction_store: Arc<dyn TransactionHistoryStore> =
            Arc::new(MockTransactionHistoryStore::new());

        let engine = WithdrawPolicyEngine::new(
            WithdrawPolicyConfig::default(),
            case_manager,
            None,
            transaction_store,
        );

        // Set up mock data provider with high daily amount
        let data_provider = Arc::new(
            MockWithdrawPolicyDataProvider::new().with_daily_amount(Decimal::from(19_000_000)), // 19M, close to 20M limit
        );

        let engine = engine.with_data_provider(data_provider);

        let mut req = create_test_request();
        req.amount_vnd = Decimal::from(5_000_000); // This would push over the 20M limit

        let result = engine.check_policy(&req).await.unwrap();
        assert!(result.is_denied());
        if let PolicyResult::Denied { code, .. } = result {
            assert_eq!(code, DenialCode::DailyLimitExceeded);
        }
    }

    #[tokio::test]
    async fn test_policy_denied_for_hourly_count_exceeded() {
        let case_store = Arc::new(InMemoryCaseStore::new());
        let case_manager = Arc::new(CaseManager::new(case_store));
        let transaction_store: Arc<dyn TransactionHistoryStore> =
            Arc::new(MockTransactionHistoryStore::new());

        let engine = WithdrawPolicyEngine::new(
            WithdrawPolicyConfig::default(),
            case_manager,
            None,
            transaction_store,
        );

        let data_provider = Arc::new(
            MockWithdrawPolicyDataProvider::new().with_hourly_count(3), // >= 2 (Tier1 limit)
        );

        let engine = engine.with_data_provider(data_provider);
        let req = create_test_request();

        let result = engine.check_policy(&req).await.unwrap();
        assert!(result.is_denied());
        if let PolicyResult::Denied { code, .. } = result {
            assert_eq!(code, DenialCode::VelocityCheckFailed);
        }
    }

    #[tokio::test]
    async fn test_tier2_has_higher_limits() {
        let engine = create_test_engine();
        let mut req = create_test_request();
        req.kyc_tier = KycTier::Tier2;
        req.amount_vnd = Decimal::from(50_000_000); // 50M VND, within Tier2 limit of 100M

        let result = engine.check_policy(&req).await.unwrap();
        assert!(result.is_approved());
    }

    #[tokio::test]
    async fn test_manual_review_for_large_amount() {
        let engine = create_test_engine();
        let mut req = create_test_request();
        req.kyc_tier = KycTier::Tier3; // Tier3 has 1B single tx limit, 500M manual review threshold
        req.amount_vnd = Decimal::from(600_000_000); // 600M VND, exceeds Tier3 manual review threshold of 500M
        req.is_new_address = false;

        let result = engine.check_policy(&req).await.unwrap();
        // Should be manual review due to large amount
        assert!(result.is_manual_review());
    }

    #[test]
    fn test_tier_limits() {
        let tier0 = TierWithdrawLimits::for_tier(KycTier::Tier0);
        assert!(tier0.single_transaction_limit_vnd.is_zero());

        let tier1 = TierWithdrawLimits::for_tier(KycTier::Tier1);
        assert_eq!(
            tier1.single_transaction_limit_vnd,
            Decimal::from(10_000_000)
        );

        let tier2 = TierWithdrawLimits::for_tier(KycTier::Tier2);
        assert_eq!(
            tier2.single_transaction_limit_vnd,
            Decimal::from(100_000_000)
        );

        let tier3 = TierWithdrawLimits::for_tier(KycTier::Tier3);
        assert_eq!(
            tier3.single_transaction_limit_vnd,
            Decimal::from(1_000_000_000)
        );
    }

    #[test]
    fn test_policy_result_helpers() {
        let approved = PolicyResult::Approved;
        assert!(approved.is_approved());
        assert!(!approved.is_denied());
        assert!(!approved.is_manual_review());

        let denied = PolicyResult::Denied {
            reason: "test".to_string(),
            code: DenialCode::PolicyViolation,
        };
        assert!(!denied.is_approved());
        assert!(denied.is_denied());
        assert!(!denied.is_manual_review());

        let manual_review = PolicyResult::ManualReview {
            reason: "test".to_string(),
            case_id: None,
        };
        assert!(!manual_review.is_approved());
        assert!(!manual_review.is_denied());
        assert!(manual_review.is_manual_review());
    }
}
