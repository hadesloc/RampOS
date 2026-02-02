# Compliance Rules Integration Guide

This guide explains how to implement custom AML (Anti-Money Laundering) rules for RampOS. The compliance engine supports both built-in rule types and custom rule definitions using a JSON-based DSL.

## Overview

The RampOS compliance system consists of:

- **AmlRule Trait**: Core interface for rule evaluation
- **RuleParser**: Parses JSON rule definitions into executable rules
- **RuleStore**: Manages versioned rule configurations
- **RuleCacheManager**: Redis-based caching for compiled rules
- **ComplianceEngine**: Orchestrates rule evaluation

## Architecture

```
                    ComplianceEngine
                          |
          +---------------+---------------+
          |               |               |
     RuleStore      RuleCacheManager   CaseManager
          |               |
    +-----+-----+         |
    |           |         |
RuleParser   GenericRule  Redis
    |
+---+---+
|       |
JSON   Built-in
Rules   Rules
```

## AmlRule Trait

The core trait for all compliance rules:

```rust
use async_trait::async_trait;
use ramp_common::Result;

/// Context passed to AML rules for evaluation
#[derive(Debug, Clone)]
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

/// Result of rule evaluation
#[derive(Debug, Clone)]
pub struct RuleResult {
    /// True if rule passed (no issue detected)
    pub passed: bool,
    /// Reason for failure
    pub reason: String,
    /// Risk score (0-100)
    pub risk_score: Option<RiskScore>,
    /// Severity level
    pub severity: Option<CaseSeverity>,
    /// Whether to create a compliance case
    pub create_case: bool,
}

impl RuleResult {
    pub fn pass() -> Self {
        Self {
            passed: true,
            reason: String::new(),
            risk_score: None,
            severity: None,
            create_case: false,
        }
    }

    pub fn fail(reason: impl Into<String>) -> Self {
        Self {
            passed: false,
            reason: reason.into(),
            risk_score: Some(RiskScore::new(50.0)),
            severity: Some(CaseSeverity::Medium),
            create_case: true,
        }
    }
}

/// AML Rule trait
#[async_trait]
pub trait AmlRule: Send + Sync {
    /// Rule identifier
    fn name(&self) -> &str;

    /// Case type to create if rule fails
    fn case_type(&self) -> CaseType;

    /// Evaluate the rule
    async fn evaluate(&self, context: &RuleContext) -> Result<RuleResult>;
}
```

## Rule Definition Format

Rules are defined in JSON format:

```json
{
  "version": "1.0.0",
  "rules": [
    {
      "id": "large-transaction",
      "name": "Large Transaction Detection",
      "type": "large_transaction",
      "enabled": true,
      "severity": "high",
      "parameters": {
        "threshold_vnd": 500000000
      },
      "description": "Flag transactions over 500M VND",
      "tags": ["aml", "ctr"]
    },
    {
      "id": "velocity-1h",
      "name": "Velocity Check (1 hour)",
      "type": "velocity",
      "enabled": true,
      "severity": "medium",
      "parameters": {
        "max_count": 5,
        "window_hours": 1,
        "min_total_vnd": 50000000
      }
    }
  ],
  "metadata": {
    "author": "compliance-team",
    "last_updated": "2024-01-15"
  }
}
```

## Built-in Rule Types

### 1. Large Transaction Rule

Flags transactions above a threshold:

```json
{
  "id": "large-tx-500m",
  "name": "Large Transaction (500M+)",
  "type": "large_transaction",
  "enabled": true,
  "severity": "high",
  "parameters": {
    "threshold_vnd": 500000000
  }
}
```

### 2. Velocity Rule

Detects high-frequency transactions:

```json
{
  "id": "velocity-check",
  "name": "High Velocity Detection",
  "type": "velocity",
  "enabled": true,
  "severity": "medium",
  "parameters": {
    "max_count": 10,
    "window_hours": 24,
    "min_total_vnd": 100000000
  }
}
```

### 3. Structuring Rule

Detects potential structuring (smurfing):

```json
{
  "id": "structuring-detection",
  "name": "Structuring Detection",
  "type": "structuring",
  "enabled": true,
  "severity": "high",
  "parameters": {
    "max_count": 5,
    "window_hours": 24,
    "threshold_vnd": 200000000
  }
}
```

### 4. Unusual Payout Rule

Detects unusual payout patterns:

```json
{
  "id": "rapid-payout",
  "name": "Rapid Payout Detection",
  "type": "unusual_payout",
  "enabled": true,
  "severity": "medium",
  "parameters": {
    "min_minutes_between": 5
  }
}
```

## Generic Rule DSL

For custom rules, use the conditions-based DSL:

### Condition Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `gt` | Greater than | `{"field": "amount", "operator": "gt", "value": 1000000}` |
| `gte` | Greater than or equal | `{"field": "amount", "operator": "gte", "value": 1000000}` |
| `lt` | Less than | `{"field": "amount", "operator": "lt", "value": 100}` |
| `lte` | Less than or equal | `{"field": "amount", "operator": "lte", "value": 100}` |
| `eq` | Equal | `{"field": "country", "operator": "eq", "value": "VN"}` |
| `neq` | Not equal | `{"field": "status", "operator": "neq", "value": "approved"}` |
| `in` | In array | `{"field": "country", "operator": "in", "value": ["VN", "TH", "SG"]}` |
| `not_in` | Not in array | `{"field": "country", "operator": "not_in", "value": ["KP", "IR"]}` |
| `contains` | String contains | `{"field": "name", "operator": "contains", "value": "test"}` |
| `starts_with` | String starts with | `{"field": "account", "operator": "starts_with", "value": "VN"}` |
| `ends_with` | String ends with | `{"field": "email", "operator": "ends_with", "value": "@test.com"}` |
| `between` | Between range | `{"field": "amount", "operator": "between", "value": [1000, 5000]}` |

### Simple Condition Rule

```json
{
  "id": "high-value-rule",
  "name": "High Value Transaction",
  "conditions": [
    {"field": "amount", "operator": "gt", "value": 1000000000}
  ],
  "scoreImpact": 30,
  "enabled": true
}
```

### Composite Conditions (AND/OR)

```json
{
  "id": "suspicious-pattern",
  "name": "Suspicious Transaction Pattern",
  "conditions": [
    {
      "AND": [
        {"field": "amount", "operator": "gt", "value": 50000000},
        {"field": "transaction_type", "operator": "eq", "value": "payout"},
        {
          "OR": [
            {"field": "country", "operator": "in", "value": ["XX", "YY"]},
            {"field": "user_tier", "operator": "lt", "value": 2}
          ]
        }
      ]
    }
  ],
  "severity": "high",
  "scoreImpact": 50,
  "enabled": true
}
```

## Implementing Custom Rules

### Step 1: Define the Rule

```rust
use async_trait::async_trait;
use ramp_compliance::rules::{AmlRule, RuleContext, RuleResult};
use ramp_compliance::types::{CaseSeverity, CaseType, RiskScore};
use ramp_common::Result;
use rust_decimal::Decimal;

/// Custom rule for detecting round-amount transactions
pub struct RoundAmountRule {
    /// Minimum amount to check
    min_amount: Decimal,
    /// Score impact when rule triggers
    score_impact: f64,
}

impl RoundAmountRule {
    pub fn new(min_amount: Decimal, score_impact: f64) -> Self {
        Self {
            min_amount,
            score_impact,
        }
    }

    fn is_round_amount(&self, amount: Decimal) -> bool {
        // Check if amount is a round number (ends in 000000)
        let millions = amount / Decimal::from(1_000_000);
        millions.fract().is_zero()
    }
}

#[async_trait]
impl AmlRule for RoundAmountRule {
    fn name(&self) -> &str {
        "round_amount_detection"
    }

    fn case_type(&self) -> CaseType {
        CaseType::SuspiciousPattern
    }

    async fn evaluate(&self, context: &RuleContext) -> Result<RuleResult> {
        // Skip small amounts
        if context.current_amount < self.min_amount {
            return Ok(RuleResult::pass());
        }

        if self.is_round_amount(context.current_amount) {
            Ok(RuleResult {
                passed: false,
                reason: format!(
                    "Transaction amount {} VND is a round number",
                    context.current_amount
                ),
                risk_score: Some(RiskScore::new(self.score_impact)),
                severity: Some(CaseSeverity::Low),
                create_case: true,
            })
        } else {
            Ok(RuleResult::pass())
        }
    }
}
```

### Step 2: Register with RuleParser (Optional)

For JSON-configurable custom rules:

```rust
impl RuleParser {
    fn create_rule(def: &RuleDefinition) -> Result<CompiledRule, RuleParseError> {
        // Check for generic conditions first
        if !def.conditions.is_empty() {
            return Ok(Box::new(GenericRule::new(def.clone())));
        }

        // Dispatch by type
        if let Some(rule_type) = &def.rule_type {
            match rule_type.as_str() {
                "velocity" => Self::create_velocity_rule(def),
                "structuring" => Self::create_structuring_rule(def),
                "large_transaction" => Self::create_large_transaction_rule(def),
                "unusual_payout" => Self::create_unusual_payout_rule(def),
                "round_amount" => Self::create_round_amount_rule(def),
                "custom" => Self::create_custom_rule(def),
                unknown => Err(RuleParseError::UnknownRuleType(unknown.to_string())),
            }
        } else {
            Err(RuleParseError::ValidationError(
                "Rule must have either 'type' or 'conditions'".to_string(),
            ))
        }
    }

    fn create_round_amount_rule(def: &RuleDefinition) -> Result<CompiledRule, RuleParseError> {
        let min_amount = Self::get_param_decimal(&def.parameters, "min_amount_vnd")?;
        let score_impact = def.score_impact.unwrap_or(20) as f64;

        Ok(Box::new(RoundAmountRule::new(min_amount, score_impact)))
    }
}
```

## Rule Store and Caching

### Loading Rules

```rust
use ramp_compliance::rule_parser::{RuleParser, RuleStore, RulesConfig};

// Create rule store
let store = RuleStore::new();

// Load from JSON string
let rules_json = include_str!("../config/aml_rules.json");
let count = store.load_from_json(rules_json)?;
println!("Loaded {} rules", count);

// Check version
println!("Rules version: {}", store.version());
```

### Redis Caching

```rust
use ramp_compliance::rules::RuleCacheManager;
use redis::Client;

let redis_client = Client::open("redis://localhost:6379")?;
let cache_manager = RuleCacheManager::new(
    redis_client,
    3600, // TTL in seconds
);

// Get cached rules for tenant
if let Some(rules) = cache_manager.get_rules(&tenant_id).await {
    // Use cached rules
} else {
    // Load from database and cache
    let rules = load_rules_from_db(&tenant_id).await?;
    cache_manager.set_rules(&tenant_id, &rules, None).await?;
}

// Invalidate cache when rules change
cache_manager.invalidate(&tenant_id).await?;

// Invalidate all tenant caches
cache_manager.invalidate_all().await?;
```

## Severity Levels

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SeverityLevel {
    Low,      // Informational, may not need review
    Medium,   // Requires attention within 24h
    High,     // Requires immediate attention
    Critical, // Requires immediate escalation
}

impl From<SeverityLevel> for CaseSeverity {
    fn from(level: SeverityLevel) -> Self {
        match level {
            SeverityLevel::Low => CaseSeverity::Low,
            SeverityLevel::Medium => CaseSeverity::Medium,
            SeverityLevel::High => CaseSeverity::High,
            SeverityLevel::Critical => CaseSeverity::Critical,
        }
    }
}
```

## Testing Rules

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ramp_compliance::rule_parser::RuleParser;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_large_transaction_rule() {
        let json = r#"
        {
            "id": "large-tx",
            "name": "Large Transaction",
            "type": "large_transaction",
            "enabled": true,
            "parameters": {
                "threshold_vnd": 500000000
            }
        }
        "#;

        let rule = RuleParser::parse(json).unwrap();

        // Test passing case
        let context = RuleContext {
            tenant_id: TenantId::new("test"),
            user_id: UserId::new("user"),
            current_amount: Decimal::from(100000000), // 100M < 500M
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        let result = rule.evaluate(&context).await.unwrap();
        assert!(result.passed);

        // Test failing case
        let context_fail = RuleContext {
            current_amount: Decimal::from(600000000), // 600M > 500M
            ..context
        };

        let result = rule.evaluate(&context_fail).await.unwrap();
        assert!(!result.passed);
        assert!(result.create_case);
    }

    #[tokio::test]
    async fn test_generic_rule_with_conditions() {
        let json = r#"
        {
            "id": "test-rule",
            "name": "Test Rule",
            "conditions": [
                {"field": "amount", "operator": "gt", "value": 1000}
            ],
            "scoreImpact": 50,
            "enabled": true
        }
        "#;

        let rule = RuleParser::parse(json).unwrap();

        let context = RuleContext {
            tenant_id: TenantId::new("test"),
            user_id: UserId::new("user"),
            current_amount: Decimal::from(1500), // > 1000
            transaction_type: TransactionType::Payin,
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        let result = rule.evaluate(&context).await.unwrap();
        assert!(!result.passed); // Rule matched
        assert_eq!(result.risk_score.map(|s| s.0), Some(50.0));
    }

    #[tokio::test]
    async fn test_composite_conditions() {
        let json = r#"
        {
            "id": "complex-rule",
            "name": "Complex Rule",
            "conditions": [
                {
                    "AND": [
                        {"field": "amount", "operator": "gt", "value": 1000},
                        {"field": "country", "operator": "eq", "value": "VN"}
                    ]
                }
            ],
            "enabled": true
        }
        "#;

        let rule = RuleParser::parse(json).unwrap();

        // Both conditions true
        let context = RuleContext {
            current_amount: Decimal::from(1500),
            metadata: serde_json::json!({"country": "VN"}),
            ..Default::default()
        };

        let result = rule.evaluate(&context).await.unwrap();
        assert!(!result.passed); // Matched

        // One condition false
        let context2 = RuleContext {
            current_amount: Decimal::from(500), // < 1000
            metadata: serde_json::json!({"country": "VN"}),
            ..Default::default()
        };

        let result = rule.evaluate(&context2).await.unwrap();
        assert!(result.passed); // Not matched
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_rule_store_loading() {
    let json = r#"
    {
        "version": "1.0.0",
        "rules": [
            {
                "id": "rule-1",
                "name": "Test Rule 1",
                "type": "large_transaction",
                "enabled": true,
                "parameters": {"threshold_vnd": 500000000}
            },
            {
                "id": "rule-2",
                "name": "Test Rule 2",
                "type": "velocity",
                "enabled": false,
                "parameters": {
                    "max_count": 10,
                    "window_hours": 24,
                    "min_total_vnd": 100000000
                }
            }
        ]
    }
    "#;

    let store = RuleStore::new();
    let count = store.load_from_json(json).unwrap();

    // Only enabled rules are loaded
    assert_eq!(count, 1);
    assert_eq!(store.version(), "1.0.0");
}
```

## Rule Evaluation Pipeline

```rust
use ramp_compliance::engine::ComplianceEngine;

// Create compliance engine
let engine = ComplianceEngine::new(
    rule_store,
    case_repository,
    transaction_history,
);

// Evaluate all rules for a transaction
let results = engine.evaluate_transaction(&RuleContext {
    tenant_id: TenantId::new("tenant_1"),
    user_id: UserId::new("user_1"),
    current_amount: Decimal::from(100000000),
    transaction_type: TransactionType::Payin,
    timestamp: Utc::now(),
    metadata: serde_json::json!({
        "source_bank": "VCB",
        "ip_country": "VN"
    }),
    user_full_name: Some("Nguyen Van A".to_string()),
    user_country: Some("VN".to_string()),
    user_address: None,
}).await?;

// Process results
let mut total_risk_score = 0.0;
let mut failed_rules = Vec::new();

for (rule_name, result) in &results {
    if !result.passed {
        failed_rules.push(rule_name.clone());
        if let Some(score) = &result.risk_score {
            total_risk_score += score.0;
        }

        // Create compliance case if needed
        if result.create_case {
            engine.create_case(
                rule_name,
                result,
                &context,
            ).await?;
        }
    }
}

println!("Failed rules: {:?}", failed_rules);
println!("Total risk score: {}", total_risk_score);
```

## Configuration Best Practices

### Rule Versioning

```json
{
  "version": "2.1.0",
  "rules": [...],
  "metadata": {
    "author": "compliance-team",
    "approved_by": "cco@example.com",
    "effective_date": "2024-02-01",
    "changelog": "Added new structuring detection rule"
  }
}
```

### Environment-Specific Rules

```rust
// Load different rules per environment
let rules_file = match env::var("ENVIRONMENT").as_deref() {
    Ok("production") => "config/aml_rules_prod.json",
    Ok("staging") => "config/aml_rules_staging.json",
    _ => "config/aml_rules_dev.json",
};

let rules_json = std::fs::read_to_string(rules_file)?;
store.load_from_json(&rules_json)?;
```

### Tenant-Specific Rules

```rust
// Load base rules + tenant overrides
let base_rules = load_base_rules()?;
let tenant_rules = load_tenant_rules(&tenant_id)?;

// Merge rules, tenant rules take precedence
let merged = merge_rules(base_rules, tenant_rules);
store.load_from_config(&merged)?;
```

## Monitoring and Alerting

```rust
// Metrics for rule evaluation
use prometheus::{Counter, Histogram};

lazy_static! {
    static ref RULE_EVALUATIONS: Counter = Counter::new(
        "aml_rule_evaluations_total",
        "Total AML rule evaluations"
    ).unwrap();

    static ref RULE_FAILURES: Counter = Counter::new(
        "aml_rule_failures_total",
        "Total AML rule failures"
    ).unwrap();

    static ref RULE_DURATION: Histogram = Histogram::new(
        "aml_rule_duration_seconds",
        "AML rule evaluation duration"
    ).unwrap();
}

// In evaluation
let timer = RULE_DURATION.start_timer();
let result = rule.evaluate(&context).await?;
timer.observe_duration();

RULE_EVALUATIONS.inc();
if !result.passed {
    RULE_FAILURES.inc();
}
```

## Troubleshooting

### Common Issues

1. **Rule Not Loading**: Check JSON syntax and required fields
2. **Unexpected Matches**: Verify field names match context
3. **Performance Issues**: Cache compiled rules in Redis
4. **Version Mismatch**: Ensure cache invalidation on rule updates

### Debug Mode

```bash
RUST_LOG=ramp_compliance::rules=debug cargo run
```

This will show:
- Rule loading/parsing details
- Condition evaluation steps
- Cache hits/misses
