use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// KYC Tier levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum KycTier {
    Tier0 = 0, // View only, no transactions
    Tier1 = 1, // Basic eKYC, low limits
    Tier2 = 2, // Enhanced KYC, higher limits
    Tier3 = 3, // KYB/Business, highest limits
}

impl KycTier {
    pub fn from_i16(value: i16) -> Self {
        match value {
            0 => KycTier::Tier0,
            1 => KycTier::Tier1,
            2 => KycTier::Tier2,
            3 => KycTier::Tier3,
            _ => KycTier::Tier0,
        }
    }

    pub fn from(value: i16) -> Self {
        Self::from_i16(value)
    }
}

impl From<i16> for KycTier {
    fn from(value: i16) -> Self {
        Self::from_i16(value)
    }
}

impl KycTier {
    pub fn daily_payin_limit_vnd(&self) -> Decimal {
        match self {
            KycTier::Tier0 => Decimal::from(5_000_000),   // 5M VND
            KycTier::Tier1 => Decimal::from(50_000_000),  // 50M VND
            KycTier::Tier2 => Decimal::from(500_000_000), // 500M VND
            KycTier::Tier3 => Decimal::MAX,               // Unlimited
        }
    }

    pub fn daily_payout_limit_vnd(&self) -> Decimal {
        match self {
            KycTier::Tier0 => Decimal::from(2_000_000),   // 2M VND
            KycTier::Tier1 => Decimal::from(20_000_000),  // 20M VND
            KycTier::Tier2 => Decimal::from(200_000_000), // 200M VND
            KycTier::Tier3 => Decimal::MAX,               // Unlimited
        }
    }

    pub fn single_transaction_limit_vnd(&self) -> Decimal {
        match self {
            KycTier::Tier0 => Decimal::ZERO,
            KycTier::Tier1 => Decimal::from(10_000_000), // 10M VND
            KycTier::Tier2 => Decimal::from(100_000_000), // 100M VND
            KycTier::Tier3 => Decimal::from(1_000_000_000), // 1B VND
        }
    }
}

/// KYC verification status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KycStatus {
    Pending,
    InProgress,
    Approved,
    Rejected,
    Expired,
}

/// AML case severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CaseSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// AML case status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseStatus {
    Open,
    Review,
    Hold,
    Released,
    Reported,
    Closed,
}

/// AML case type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseType {
    Velocity,         // Too many transactions in short time
    Structuring,      // Breaking up transactions to avoid limits
    NameMismatch,     // Bank account name doesn't match user
    UnusualPayout,    // Withdrawal immediately after deposit
    LargeTransaction, // Single large transaction
    SanctionsMatch,   // Match against sanctions list
    PepMatch,         // Politically Exposed Person
    AdverseMedia,     // Negative news about person
    KytHighRisk,      // High risk address in crypto transaction
    DeviceAnomaly,    // Suspicious device/IP patterns
    Other(String),
}

/// Risk score (0-100)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RiskScore(pub f64);

impl RiskScore {
    pub fn new(score: f64) -> Self {
        Self(score.clamp(0.0, 100.0))
    }

    pub fn is_high_risk(&self) -> bool {
        self.0 >= 70.0
    }

    pub fn is_medium_risk(&self) -> bool {
        self.0 >= 40.0 && self.0 < 70.0
    }

    pub fn is_low_risk(&self) -> bool {
        self.0 < 40.0
    }
}

/// KYT (Know Your Transaction) check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KytResult {
    pub address: String,
    pub risk_score: RiskScore,
    pub risk_signals: Vec<String>,
    pub is_sanctioned: bool,
    pub checked_at: DateTime<Utc>,
}

/// Compliance check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheckResult {
    pub passed: bool,
    pub risk_score: RiskScore,
    pub flags: Vec<String>,
    pub requires_review: bool,
    pub cases_created: Vec<String>,
}

impl ComplianceCheckResult {
    pub fn pass() -> Self {
        Self {
            passed: true,
            risk_score: RiskScore::new(0.0),
            flags: vec![],
            requires_review: false,
            cases_created: vec![],
        }
    }

    pub fn fail(reason: impl Into<String>) -> Self {
        Self {
            passed: false,
            risk_score: RiskScore::new(100.0),
            flags: vec![reason.into()],
            requires_review: true,
            cases_created: vec![],
        }
    }
}
