# RampOS Compliance Documentation

## Overview

RampOS includes a comprehensive compliance engine that handles:
- **KYC (Know Your Customer)**: Identity verification and tier management
- **AML (Anti-Money Laundering)**: Transaction monitoring and suspicious activity detection
- **KYT (Know Your Transaction)**: Blockchain address risk assessment

This system ensures regulatory compliance while maintaining a smooth user experience.

## Architecture

```
+------------------+     +------------------+     +------------------+
|   Transaction    |---->|   AML Engine     |---->|   Case Manager   |
|   (Intent)       |     | (Rule Evaluation)|     | (Investigation)  |
+------------------+     +--------+---------+     +------------------+
                                  |
                    +-------------+-------------+
                    |             |             |
                    v             v             v
             +----------+  +-----------+  +------------+
             | Velocity |  |Structuring|  | Sanctions  |
             |  Rules   |  |   Rules   |  | Screening  |
             +----------+  +-----------+  +------------+
```

## KYC Tier System

RampOS implements a tiered KYC system that controls transaction limits based on verification level.

### Tier Definitions

| Tier | Name | Description | Requirements |
|------|------|-------------|--------------|
| Tier 0 | View Only | No transactions allowed | Email/phone verification |
| Tier 1 | Basic | Low limits | Basic eKYC (ID document) |
| Tier 2 | Enhanced | Higher limits | Enhanced KYC + Address proof |
| Tier 3 | Business | Highest limits | KYB + Source of funds |

### Tier Limits (VND)

| Tier | Daily Pay-in | Daily Pay-out | Single Transaction |
|------|-------------|---------------|-------------------|
| Tier 0 | 5,000,000 | 2,000,000 | 0 (blocked) |
| Tier 1 | 50,000,000 | 20,000,000 | 10,000,000 |
| Tier 2 | 500,000,000 | 200,000,000 | 100,000,000 |
| Tier 3 | Unlimited | Unlimited | 1,000,000,000 |

```rust
impl KycTier {
    pub fn daily_payin_limit_vnd(&self) -> Decimal {
        match self {
            KycTier::Tier0 => Decimal::from(5_000_000),
            KycTier::Tier1 => Decimal::from(50_000_000),
            KycTier::Tier2 => Decimal::from(500_000_000),
            KycTier::Tier3 => Decimal::MAX,
        }
    }

    pub fn daily_payout_limit_vnd(&self) -> Decimal {
        match self {
            KycTier::Tier0 => Decimal::from(2_000_000),
            KycTier::Tier1 => Decimal::from(20_000_000),
            KycTier::Tier2 => Decimal::from(200_000_000),
            KycTier::Tier3 => Decimal::MAX,
        }
    }

    pub fn single_transaction_limit_vnd(&self) -> Decimal {
        match self {
            KycTier::Tier0 => Decimal::ZERO,
            KycTier::Tier1 => Decimal::from(10_000_000),
            KycTier::Tier2 => Decimal::from(100_000_000),
            KycTier::Tier3 => Decimal::from(1_000_000_000),
        }
    }
}
```

### KYC Status Flow

```
+----------+     +-----------+     +----------+
| PENDING  |---->|IN_PROGRESS|---->| APPROVED |
+----------+     +-----+-----+     +----------+
                       |
                       +---------->+----------+
                       |           | REJECTED |
                       |           +----------+
                       |
                       +---------->+----------+
                                   | EXPIRED  |
                                   +----------+
```

### Tier Upgrade Requirements

```rust
fn check_upgrade_logic(&self, info: &UserKycInfo, target_tier: KycTier) -> bool {
    // Must be in approved KYC status
    if info.kyc_status != KycStatus::Approved {
        return false;
    }

    match target_tier {
        KycTier::Tier0 => false,  // Cannot upgrade to Tier0
        KycTier::Tier1 => {
            // Requires basic ID document
            info.verified_documents.iter()
                .any(|d| d == "ID_FRONT" || d == "PASSPORT" || d == "CCCD")
        }
        KycTier::Tier2 => {
            // Requires Tier1 + address verification
            info.current_tier >= KycTier::Tier1
                && info.verified_documents.contains(&"PROOF_OF_ADDRESS".to_string())
        }
        KycTier::Tier3 => {
            // Requires Tier2 + source of funds
            info.current_tier >= KycTier::Tier2
                && info.verified_documents.contains(&"SOURCE_OF_FUNDS".to_string())
        }
    }
}
```

## AML Engine

The AML Engine evaluates every transaction against a set of rules to detect suspicious activity.

### Rule Architecture

```rust
/// AML Rule trait - implement for each detection rule
#[async_trait]
pub trait AmlRule: Send + Sync {
    /// Rule identifier
    fn name(&self) -> &str;

    /// Case type to create if rule fails
    fn case_type(&self) -> CaseType;

    /// Evaluate the rule
    async fn evaluate(&self, context: &RuleContext) -> Result<RuleResult>;
}

/// Context passed to all rules
pub struct RuleContext {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub current_amount: Decimal,
    pub transaction_type: TransactionType,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
    pub user_full_name: Option<String>,
    pub user_country: Option<String>,
    pub user_address: Option<String>,
}
```

### Built-in AML Rules

#### 1. Velocity Rule

Detects unusually high transaction frequency.

```rust
pub struct VelocityRule {
    max_count: u32,          // Maximum transactions in window
    window: Duration,         // Time window (e.g., 1 hour)
    min_total: Decimal,       // Minimum total amount to trigger
}

// Default: 5 transactions totaling 50M VND in 1 hour
```

**Triggers when**: User makes too many transactions in a short period

**Example**:
- User makes 6 transactions totaling 60M VND in 45 minutes
- Rule triggers, creates Velocity case with High severity

#### 2. Structuring Rule

Detects attempts to break up transactions to avoid limits.

```rust
pub struct StructuringRule {
    max_count: u32,           // Maximum similar transactions
    window: Duration,          // Time window (e.g., 24 hours)
    threshold: Decimal,        // Amount threshold being avoided
}

// Default: 10 transactions between 80M-100M VND in 24 hours
```

**Triggers when**: Multiple transactions just below reporting threshold

**Example**:
- User makes 12 transactions of 95M VND each (just under 100M limit)
- Pattern suggests intentional structuring

#### 3. Large Transaction Rule

Flags single large transactions for review.

```rust
pub struct LargeTransactionRule {
    threshold: Decimal,  // Amount threshold
}

// Default: 500M VND
```

**Triggers when**: Single transaction exceeds threshold

**Example**:
- User initiates 600M VND payout
- Automatically flagged for enhanced review

#### 4. Unusual Payout Rule

Detects suspicious withdrawal patterns.

```rust
pub struct UnusualPayoutRule {
    min_time_between: Duration,  // Minimum time between deposit and withdrawal
}

// Default: 30 minutes
```

**Triggers when**: Withdrawal occurs shortly after deposit

**Example**:
- User deposits 100M VND
- User requests 95M VND withdrawal 15 minutes later
- Pattern may indicate money laundering

#### 5. Device Anomaly Rule

Detects suspicious device or location patterns.

```rust
pub struct DeviceAnomalyRule {
    device_store: Arc<dyn DeviceHistoryStore>,
}
```

**Triggers when**:
- Login from new device after significant transaction
- Rapid location changes (impossible travel)
- Multiple accounts from same device

#### 6. Sanctions Screening Rule

Checks against global sanctions lists.

```rust
pub struct SanctionsRule {
    provider: Arc<dyn SanctionsProvider>,
}
```

**Triggers when**: User name or address matches sanctions list entry

### Rule Evaluation Flow

```
+-------------+     +------------+     +------------+
| Transaction |---->| Record to  |---->| Build Rule |
|   Received  |     |   History  |     |  Context   |
+-------------+     +------------+     +-----+------+
                                             |
                         +-------------------+-------------------+
                         |         |         |         |         |
                         v         v         v         v         v
                    +--------+ +------+ +-------+ +-------+ +--------+
                    |Velocity| |Struct| | Large | |Unusual| |Sanctions|
                    | Check  | |Check | | Check | | Check | | Check  |
                    +---+----+ +--+---+ +---+---+ +---+---+ +---+----+
                        |         |         |         |         |
                        v         v         v         v         v
                    +----------------------------------------------------+
                    |              Aggregate Results                      |
                    |  - Calculate total risk score                       |
                    |  - Collect all flags                                |
                    |  - Determine if review required                     |
                    +---------------------------+------------------------+
                                                |
                         +----------------------+----------------------+
                         |                                             |
                         v                                             v
                  +-------------+                              +---------------+
                  |   PASSED    |                              | NEEDS REVIEW  |
                  | Risk < 40   |                              | Risk >= 40    |
                  +-------------+                              +-------+-------+
                                                                       |
                                                                       v
                                                              +---------------+
                                                              | Create AML    |
                                                              | Case(s)       |
                                                              +---------------+
```

## Risk Scoring

### Risk Score Levels

```rust
pub struct RiskScore(pub f64);  // 0-100 scale

impl RiskScore {
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
```

| Score Range | Risk Level | Action |
|-------------|------------|--------|
| 0-39 | Low | Auto-approve |
| 40-69 | Medium | Enhanced monitoring |
| 70-100 | High | Manual review required |

### Rule Risk Contributions

| Rule | Risk Score | Severity |
|------|------------|----------|
| Velocity | 60 | High |
| Structuring | 75 | High |
| Large Transaction | 50 | Medium |
| Unusual Payout | 55 | Medium |
| Device Anomaly | 65 | High |
| Sanctions Match | 100 | Critical |

## Case Management

When rules trigger, the system creates investigation cases.

### Case Types

```rust
pub enum CaseType {
    Velocity,          // Too many transactions
    Structuring,       // Breaking up transactions
    NameMismatch,      // Bank account name doesn't match
    UnusualPayout,     // Suspicious withdrawal pattern
    LargeTransaction,  // Single large transaction
    SanctionsMatch,    // Sanctions list match
    PepMatch,          // Politically Exposed Person
    AdverseMedia,      // Negative news
    KytHighRisk,       // High risk blockchain address
    DeviceAnomaly,     // Suspicious device/IP
    Other(String),     // Custom case types
}
```

### Case Severity Levels

```rust
pub enum CaseSeverity {
    Low,       // Informational, may not need action
    Medium,    // Requires review within 48 hours
    High,      // Requires review within 24 hours
    Critical,  // Requires immediate action
}
```

### Case Status Flow

```
+--------+     +--------+     +--------+
|  OPEN  |---->| REVIEW |---->| CLOSED |
+---+----+     +---+----+     +--------+
    |              |
    |              +-------->+----------+
    |              |         | RELEASED |
    |              |         +----------+
    |              |
    |              +-------->+----------+
    |              |         | REPORTED |
    |              |         +----------+
    |              |
    v              v
+--------+     +--------+
|  HOLD  |<--->| REVIEW |
+--------+     +--------+
```

### Case Data Structure

```rust
pub struct AmlCase {
    pub id: String,                      // case_xxx
    pub tenant_id: TenantId,
    pub user_id: Option<UserId>,
    pub intent_id: Option<IntentId>,
    pub case_type: CaseType,
    pub severity: CaseSeverity,
    pub status: CaseStatus,
    pub detection_data: serde_json::Value,  // Rule-specific details
    pub assigned_to: Option<String>,         // Analyst ID
    pub resolution: Option<String>,          // Resolution notes
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}
```

### Case Manager Operations

```rust
impl CaseManager {
    /// Create a new case
    pub async fn create_case(
        &self,
        tenant_id: &TenantId,
        user_id: Option<&UserId>,
        intent_id: Option<&IntentId>,
        case_type: CaseType,
        severity: CaseSeverity,
        detection_data: serde_json::Value,
    ) -> Result<String>;

    /// Assign case to analyst
    pub async fn assign_case(
        &self,
        case_id: &str,
        analyst_id: &str,
    ) -> Result<()>;

    /// Update case status
    pub async fn update_status(
        &self,
        case_id: &str,
        new_status: CaseStatus,
    ) -> Result<()>;

    /// Resolve case
    pub async fn resolve_case(
        &self,
        case_id: &str,
        resolution: &str,
        new_status: CaseStatus,
    ) -> Result<()>;

    /// Get open cases for a tenant
    pub async fn get_open_cases(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<AmlCase>>;
}
```

## KYT (Know Your Transaction)

KYT checks blockchain addresses against risk databases.

### KYT Result Structure

```rust
pub struct KytResult {
    pub address: String,
    pub risk_score: RiskScore,
    pub risk_signals: Vec<String>,
    pub is_sanctioned: bool,
    pub checked_at: DateTime<Utc>,
}
```

### Risk Signals

Common risk signals detected by KYT:
- **Mixer/Tumbler**: Address associated with mixing services
- **Darknet Market**: Connection to illicit marketplaces
- **Ransomware**: Associated with ransomware payments
- **Scam**: Known scam or fraud address
- **Sanctions**: OFAC or other sanctions list
- **Gambling**: Connection to unlicensed gambling
- **High-Risk Exchange**: Unregulated exchange connection

### KYT Integration Flow

```
+------------------+     +-------------+     +---------------+
| Deposit/Withdraw |---->| KYT Service |---->| Risk Decision |
|    Intent        |     |   (Check)   |     |               |
+------------------+     +------+------+     +-------+-------+
                                |                    |
                    +-----------+-----------+        |
                    |           |           |        |
                    v           v           v        |
               +---------+ +---------+ +--------+    |
               |Chainalys| |Elliptic | | Custom |    |
               |Provider | |Provider | |Provider|    |
               +---------+ +---------+ +--------+    |
                    |           |           |        |
                    +-----------+-----------+        |
                                |                    |
                                v                    v
                         +-------------+      +-----------+
                         | Aggregate   |----->| Allow/    |
                         | Risk Score  |      | Block/    |
                         +-------------+      | Review    |
                                              +-----------+
```

## Sanctions Screening

### Provider Interface

```rust
#[async_trait]
pub trait SanctionsProvider: Send + Sync {
    /// Check individual against sanctions
    async fn check_individual(
        &self,
        name: &str,
        date_of_birth: Option<&str>,
        country: Option<&str>,
    ) -> Result<SanctionsResult>;

    /// Check organization against sanctions
    async fn check_organization(
        &self,
        name: &str,
        country: Option<&str>,
    ) -> Result<SanctionsResult>;

    /// Check address/location
    async fn check_address(
        &self,
        address: &str,
    ) -> Result<SanctionsResult>;
}

pub struct SanctionsResult {
    pub matched: bool,
    pub score: f64,
    pub list_name: Option<String>,
    pub matched_entries: Vec<SanctionsEntry>,
}
```

### Supported Lists

- OFAC SDN (US Treasury)
- UN Security Council
- EU Consolidated List
- UK HM Treasury
- OpenSanctions (aggregated)

### Match Handling

When a sanctions match is detected:
1. Transaction is immediately blocked
2. Critical severity case is created
3. All user transactions are frozen
4. Compliance team is notified
5. SAR filing may be required

## Compliance Check Result

```rust
pub struct ComplianceCheckResult {
    pub passed: bool,              // Overall pass/fail
    pub risk_score: RiskScore,     // Aggregate risk score
    pub flags: Vec<String>,        // All triggered flags
    pub requires_review: bool,     // Needs manual review
    pub cases_created: Vec<String>,// Case IDs created
}

impl ComplianceCheckResult {
    /// Create a passing result
    pub fn pass() -> Self {
        Self {
            passed: true,
            risk_score: RiskScore::new(0.0),
            flags: vec![],
            requires_review: false,
            cases_created: vec![],
        }
    }

    /// Create a failing result
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
```

## Rule Configuration

Rules can be configured per tenant via database or config:

```rust
pub struct RuleConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub parameters: serde_json::Value,
    pub severity: CaseSeverity,
}

// Example rule configuration
{
    "id": "velocity_check",
    "name": "Transaction Velocity Check",
    "enabled": true,
    "parameters": {
        "max_count": 5,
        "window_minutes": 60,
        "min_total_vnd": 50000000
    },
    "severity": "HIGH"
}
```

## Rule Caching

Rules are cached in Redis for performance:

```rust
pub struct RuleCacheManager {
    client: redis::Client,
    ttl_seconds: u64,
}

impl RuleCacheManager {
    /// Get cached rules for tenant
    pub async fn get_rules(&self, tenant_id: &TenantId) -> Option<Vec<CompiledRule>>;

    /// Cache rules for tenant
    pub async fn set_rules(
        &self,
        tenant_id: &TenantId,
        rules: &[RuleDefinition],
        ttl: Option<u64>,
    ) -> Result<()>;

    /// Invalidate cache for tenant
    pub async fn invalidate(&self, tenant_id: &TenantId) -> Result<()>;
}
```

## Reporting

### Report Types

```rust
pub enum ReportType {
    Sar,    // Suspicious Activity Report
    Ctr,    // Currency Transaction Report
    Daily,  // Daily compliance summary
    Kyc,    // KYC verification report
    Aml,    // AML activity report
}
```

### SAR (Suspicious Activity Report)

Required when suspicious activity is detected:

```rust
pub struct SarReport {
    pub report_id: String,
    pub filing_date: DateTime<Utc>,
    pub subject: SarSubject,
    pub suspicious_activity: SuspiciousActivity,
    pub narrative: String,
    pub supporting_docs: Vec<String>,
}
```

## Best Practices

1. **Defense in Depth**: Layer multiple rules for comprehensive coverage
2. **Tune Thresholds**: Adjust based on false positive rates
3. **Monitor Effectiveness**: Track rule hit rates and outcomes
4. **Regular Review**: Audit cases and update rules quarterly
5. **Document Decisions**: Keep detailed case resolution notes
6. **Training**: Ensure analysts understand all rule types
7. **Escalation Paths**: Define clear escalation for high-severity cases
8. **Regulatory Updates**: Stay current with regulatory changes

## Compliance API Endpoints

```
GET  /api/v1/compliance/cases              - List cases
GET  /api/v1/compliance/cases/{id}         - Get case details
POST /api/v1/compliance/cases/{id}/assign  - Assign case
POST /api/v1/compliance/cases/{id}/resolve - Resolve case
GET  /api/v1/compliance/reports            - List reports
POST /api/v1/compliance/reports/sar        - File SAR
GET  /api/v1/users/{id}/kyc                - Get KYC status
POST /api/v1/users/{id}/kyc/upgrade        - Request tier upgrade
```
