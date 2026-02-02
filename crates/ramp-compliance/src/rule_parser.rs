use chrono::Duration;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

use crate::aml::{LargeTransactionRule, StructuringRule, UnusualPayoutRule, VelocityRule};
use crate::rules::{AmlRule, CompiledRule, RuleContext, RuleResult};
use crate::transaction_history::MockTransactionHistoryStore;
use crate::types::{CaseSeverity, CaseType, RiskScore};

#[derive(Debug, Error)]
pub enum RuleParseError {
    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("Unknown rule type: {0}")]
    UnknownRuleType(String),
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),
    #[error("Invalid parameter value: {0}")]
    InvalidParameter(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleOperator {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
    Neq,
    In,
    NotIn,
    Contains,
    StartsWith,
    EndsWith,
    Between,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleCondition {
    pub field: String,
    pub operator: RuleOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeCondition {
    #[serde(rename = "AND")]
    pub and: Option<Vec<Condition>>,
    #[serde(rename = "OR")]
    pub or: Option<Vec<Condition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    Simple(SimpleCondition),
    Composite(CompositeCondition),
}

/// Rule configuration from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleDefinition {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub rule_type: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub severity: SeverityLevel,
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub version: String,

    // New fields for generic rules
    #[serde(default)]
    pub conditions: Vec<Condition>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub score_impact: Option<i32>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeverityLevel {
    Low,
    #[default]
    Medium,
    High,
    Critical,
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

/// Rules configuration file format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesConfig {
    #[serde(default)]
    pub version: String,
    pub rules: Vec<RuleDefinition>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
}

/// Generic AML Rule Implementation
#[derive(Debug)]
pub struct GenericRule {
    definition: RuleDefinition,
}

impl GenericRule {
    pub fn new(definition: RuleDefinition) -> Self {
        Self { definition }
    }

    fn evaluate_condition(&self, condition: &Condition, context: &RuleContext) -> bool {
        match condition {
            Condition::Simple(simple) => self.evaluate_simple(simple, context),
            Condition::Composite(composite) => self.evaluate_composite(composite, context),
        }
    }

    fn evaluate_composite(&self, composite: &CompositeCondition, context: &RuleContext) -> bool {
        if let Some(and_conditions) = &composite.and {
            for cond in and_conditions {
                if !self.evaluate_condition(cond, context) {
                    return false;
                }
            }
            true
        } else if let Some(or_conditions) = &composite.or {
            for cond in or_conditions {
                if self.evaluate_condition(cond, context) {
                    return true;
                }
            }
            false
        } else {
            // Empty composite is true? Or false? Assuming true (vacuously true)
            true
        }
    }

    fn evaluate_simple(&self, condition: &SimpleCondition, context: &RuleContext) -> bool {
        let field_value = self.get_field_value(&condition.field, context);

        match &condition.operator {
            RuleOperator::Gt => self.compare_numeric(&field_value, &condition.value, |a, b| a > b),
            RuleOperator::Gte => {
                self.compare_numeric(&field_value, &condition.value, |a, b| a >= b)
            }
            RuleOperator::Lt => self.compare_numeric(&field_value, &condition.value, |a, b| a < b),
            RuleOperator::Lte => {
                self.compare_numeric(&field_value, &condition.value, |a, b| a <= b)
            }
            RuleOperator::Eq => field_value == condition.value,
            RuleOperator::Neq => field_value != condition.value,
            RuleOperator::In => self.check_in(&field_value, &condition.value),
            RuleOperator::NotIn => !self.check_in(&field_value, &condition.value),
            RuleOperator::Contains => {
                self.check_string(&field_value, &condition.value, |a, b| a.contains(b))
            }
            RuleOperator::StartsWith => {
                self.check_string(&field_value, &condition.value, |a, b| a.starts_with(b))
            }
            RuleOperator::EndsWith => {
                self.check_string(&field_value, &condition.value, |a, b| a.ends_with(b))
            }
            RuleOperator::Between => self.check_between(&field_value, &condition.value),
        }
    }

    fn get_field_value(&self, field: &str, context: &RuleContext) -> serde_json::Value {
        // Try to get from metadata first
        if let Some(val) = context.metadata.get(field) {
            return val.clone();
        }

        // Try to resolve common fields from context
        match field {
            "amount" | "current_amount" => serde_json::json!(context.current_amount),
            "tenant_id" => serde_json::json!(context.tenant_id),
            "user_id" => serde_json::json!(context.user_id),
            "transaction_type" => serde_json::json!(context.transaction_type),
            // Add more field resolvers as needed
            _ => serde_json::Value::Null,
        }
    }

    fn compare_numeric<F>(
        &self,
        actual: &serde_json::Value,
        target: &serde_json::Value,
        op: F,
    ) -> bool
    where
        F: Fn(f64, f64) -> bool,
    {
        let v1 = match actual {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => s.parse::<f64>().ok(), // Handle numeric strings
            _ => None,
        };

        let v2 = match target {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => s.parse::<f64>().ok(),
            _ => None,
        };

        match (v1, v2) {
            (Some(n1), Some(n2)) => op(n1, n2),
            _ => false,
        }
    }

    fn check_in(&self, actual: &serde_json::Value, target_list: &serde_json::Value) -> bool {
        if let serde_json::Value::Array(list) = target_list {
            list.contains(actual)
        } else {
            false
        }
    }

    fn check_string<F>(&self, actual: &serde_json::Value, target: &serde_json::Value, op: F) -> bool
    where
        F: Fn(&str, &str) -> bool,
    {
        match (actual.as_str(), target.as_str()) {
            (Some(s1), Some(s2)) => op(s1, s2),
            _ => false,
        }
    }

    fn check_between(&self, actual: &serde_json::Value, range: &serde_json::Value) -> bool {
        if let serde_json::Value::Array(arr) = range {
            if arr.len() == 2 {
                let min = &arr[0];
                let max = &arr[1];
                return self.compare_numeric(actual, min, |a, b| a >= b)
                    && self.compare_numeric(actual, max, |a, b| a <= b);
            }
        }
        false
    }
}

#[async_trait::async_trait]
impl AmlRule for GenericRule {
    fn name(&self) -> &str {
        &self.definition.name
    }

    fn case_type(&self) -> CaseType {
        // Default to Other if not specified, or infer from rule type
        // This might need refinement
        CaseType::Other(
            self.definition
                .rule_type
                .clone()
                .unwrap_or_else(|| "generic".to_string()),
        )
    }

    async fn evaluate(&self, context: &RuleContext) -> Result<RuleResult, ramp_common::Error> {
        // Evaluate all top-level conditions (implicit AND)
        for condition in &self.definition.conditions {
            if !self.evaluate_condition(condition, context) {
                return Ok(RuleResult::pass());
            }
        }

        // If we get here, all conditions passed
        Ok(RuleResult {
            passed: false,
            reason: format!("Rule '{}' triggered", self.definition.name),
            risk_score: self
                .definition
                .score_impact
                .map(|s| RiskScore::new(s as f64)),
            severity: Some(self.definition.severity.into()),
            create_case: true,
        })
    }
}

/// Rule parser that converts JSON definitions to AML rules
pub struct RuleParser;

impl RuleParser {
    /// Parse rules from JSON string
    pub fn parse_json(json: &str) -> Result<Vec<CompiledRule>, RuleParseError> {
        let config: RulesConfig = serde_json::from_str(json)?;
        Self::parse_config(&config)
    }

    /// Parse a single rule from JSON string
    pub fn parse(json: &str) -> Result<CompiledRule, RuleParseError> {
        let def: RuleDefinition = serde_json::from_str(json)?;
        Self::validate(&def)?;
        Self::create_rule(&def)
    }

    /// Parse all rules from JSON string (alias for parse_json but matching requirements)
    pub fn parse_all(json: &str) -> Result<Vec<CompiledRule>, RuleParseError> {
        Self::parse_json(json)
    }

    /// Validate a rule definition
    pub fn validate(rule: &RuleDefinition) -> Result<(), RuleParseError> {
        if rule.id.is_empty() {
            return Err(RuleParseError::ValidationError(
                "Rule ID is required".to_string(),
            ));
        }
        if rule.name.is_empty() {
            return Err(RuleParseError::ValidationError(
                "Rule name is required".to_string(),
            ));
        }

        // Validate conditions if present
        for condition in &rule.conditions {
            Self::validate_condition(condition)?;
        }

        Ok(())
    }

    fn validate_condition(condition: &Condition) -> Result<(), RuleParseError> {
        match condition {
            Condition::Simple(simple) => {
                if simple.field.is_empty() {
                    return Err(RuleParseError::ValidationError(
                        "Condition field is required".to_string(),
                    ));
                }
                match simple.operator {
                    RuleOperator::Between => {
                        if !simple.value.is_array()
                            || simple.value.as_array().map(|a| a.len()).unwrap_or(0) != 2
                        {
                            return Err(RuleParseError::ValidationError(
                                "Between operator requires an array of 2 values".to_string(),
                            ));
                        }
                    }
                    RuleOperator::In | RuleOperator::NotIn => {
                        if !simple.value.is_array() {
                            return Err(RuleParseError::ValidationError(
                                "In/NotIn operator requires an array value".to_string(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
            Condition::Composite(comp) => {
                if comp.and.is_none() && comp.or.is_none() {
                    return Err(RuleParseError::ValidationError(
                        "Composite condition must have AND or OR".to_string(),
                    ));
                }
                if let Some(list) = &comp.and {
                    for c in list {
                        Self::validate_condition(c)?;
                    }
                }
                if let Some(list) = &comp.or {
                    for c in list {
                        Self::validate_condition(c)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Parse rules from config struct
    pub fn parse_config(config: &RulesConfig) -> Result<Vec<CompiledRule>, RuleParseError> {
        let mut rules: Vec<CompiledRule> = Vec::new();

        for def in &config.rules {
            if !def.enabled {
                info!(rule_id = %def.id, "Skipping disabled rule");
                continue;
            }

            match Self::create_rule(def) {
                Ok(rule) => {
                    info!(
                        rule_id = %def.id,
                        rule_type = ?def.rule_type,
                        "Loaded rule"
                    );
                    rules.push(rule);
                }
                Err(e) => {
                    warn!(
                        rule_id = %def.id,
                        error = %e,
                        "Failed to parse rule, skipping"
                    );
                }
            }
        }

        Ok(rules)
    }

    /// Create a rule from definition
    fn create_rule(def: &RuleDefinition) -> Result<CompiledRule, RuleParseError> {
        // If it has conditions, treat as generic rule
        if !def.conditions.is_empty() {
            return Ok(Box::new(GenericRule::new(def.clone())));
        }

        // Fallback to legacy type-based dispatch if type is present
        if let Some(rule_type) = &def.rule_type {
            match rule_type.as_str() {
                "velocity" => Self::create_velocity_rule(def),
                "structuring" => Self::create_structuring_rule(def),
                "large_transaction" => Self::create_large_transaction_rule(def),
                "unusual_payout" => Self::create_unusual_payout_rule(def),
                "custom" => Self::create_custom_rule(def),
                unknown => Err(RuleParseError::UnknownRuleType(unknown.to_string())),
            }
        } else {
            // No type and no conditions
            Err(RuleParseError::ValidationError(
                "Rule must have either 'type' or 'conditions'".to_string(),
            ))
        }
    }

    fn create_velocity_rule(def: &RuleDefinition) -> Result<CompiledRule, RuleParseError> {
        let max_count = Self::get_param_i64(&def.parameters, "max_count")? as u32;
        let window_hours = Self::get_param_i64(&def.parameters, "window_hours")?;
        let min_total = Self::get_param_decimal(&def.parameters, "min_total_vnd")?;

        Ok(Box::new(VelocityRule::new(
            max_count,
            Duration::hours(window_hours),
            min_total,
            Arc::new(MockTransactionHistoryStore::new()),
        )))
    }

    fn create_structuring_rule(def: &RuleDefinition) -> Result<CompiledRule, RuleParseError> {
        let max_count = Self::get_param_i64(&def.parameters, "max_count")? as u32;
        let window_hours = Self::get_param_i64(&def.parameters, "window_hours")?;
        let threshold = Self::get_param_decimal(&def.parameters, "threshold_vnd")?;

        Ok(Box::new(StructuringRule::new(
            max_count,
            Duration::hours(window_hours),
            threshold,
            Arc::new(MockTransactionHistoryStore::new()),
        )))
    }

    fn create_large_transaction_rule(def: &RuleDefinition) -> Result<CompiledRule, RuleParseError> {
        let threshold = Self::get_param_decimal(&def.parameters, "threshold_vnd")?;
        Ok(Box::new(LargeTransactionRule::new(threshold)))
    }

    fn create_unusual_payout_rule(def: &RuleDefinition) -> Result<CompiledRule, RuleParseError> {
        let min_minutes = Self::get_param_i64(&def.parameters, "min_minutes_between")?;
        Ok(Box::new(UnusualPayoutRule::new(
            Duration::minutes(min_minutes),
            Arc::new(MockTransactionHistoryStore::new()),
        )))
    }

    fn create_custom_rule(_def: &RuleDefinition) -> Result<CompiledRule, RuleParseError> {
        // Custom rules would need a more sophisticated parser
        // For now, return an error
        Err(RuleParseError::InvalidParameter(
            "Custom rules not yet supported".to_string(),
        ))
    }

    fn get_param_i64(
        params: &HashMap<String, serde_json::Value>,
        key: &str,
    ) -> Result<i64, RuleParseError> {
        params
            .get(key)
            .and_then(|v| v.as_i64())
            .ok_or_else(|| RuleParseError::MissingParameter(key.to_string()))
    }

    fn get_param_decimal(
        params: &HashMap<String, serde_json::Value>,
        key: &str,
    ) -> Result<Decimal, RuleParseError> {
        let value = params
            .get(key)
            .ok_or_else(|| RuleParseError::MissingParameter(key.to_string()))?;

        if let Some(n) = value.as_i64() {
            Ok(Decimal::from(n))
        } else if let Some(n) = value.as_f64() {
            Decimal::try_from(n).map_err(|e| RuleParseError::InvalidParameter(e.to_string()))
        } else if let Some(s) = value.as_str() {
            s.parse()
                .map_err(|e: rust_decimal::Error| RuleParseError::InvalidParameter(e.to_string()))
        } else {
            Err(RuleParseError::InvalidParameter(format!(
                "{} must be a number",
                key
            )))
        }
    }
}

/// Rule store for managing versioned rules
pub struct RuleStore {
    rules: Arc<std::sync::RwLock<Vec<CompiledRule>>>,
    version: Arc<std::sync::RwLock<String>>,
}

impl RuleStore {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(std::sync::RwLock::new(Vec::new())),
            version: Arc::new(std::sync::RwLock::new("0.0.0".to_string())),
        }
    }

    /// Load rules from JSON configuration
    pub fn load_from_json(&self, json: &str) -> Result<usize, RuleParseError> {
        let config: RulesConfig = serde_json::from_str(json)?;
        let rules = RuleParser::parse_config(&config)?;
        let count = rules.len();

        *self.rules.write().unwrap() = rules;
        *self.version.write().unwrap() = config.version.clone();

        info!(
            version = %config.version,
            count = count,
            "Loaded rules from configuration"
        );

        Ok(count)
    }

    /// Get current version
    pub fn version(&self) -> String {
        self.version.read().unwrap().clone()
    }

    /// Get rule count
    pub fn count(&self) -> usize {
        self.rules.read().unwrap().len()
    }
}

impl Default for RuleStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RULES_JSON: &str = r#"
    {
        "version": "1.0.0",
        "rules": [
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
            },
            {
                "id": "large-tx",
                "name": "Large Transaction",
                "type": "large_transaction",
                "enabled": true,
                "severity": "high",
                "parameters": {
                    "threshold_vnd": 500000000
                }
            },
            {
                "id": "disabled-rule",
                "name": "Disabled Rule",
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

    #[test]
    fn test_parse_rules() {
        let rules = RuleParser::parse_json(SAMPLE_RULES_JSON).unwrap();
        // Should have 2 rules (disabled one is skipped)
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn test_rule_store() {
        let store = RuleStore::new();
        let count = store.load_from_json(SAMPLE_RULES_JSON).unwrap();
        assert_eq!(count, 2);
        assert_eq!(store.version(), "1.0.0");
    }

    #[test]
    fn test_invalid_json() {
        let result = RuleParser::parse_json("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_rule_type() {
        let json = r#"
        {
            "rules": [
                {
                    "id": "unknown",
                    "name": "Unknown",
                    "type": "unknown_type",
                    "parameters": {}
                }
            ]
        }
        "#;
        // Should not fail, just skip the unknown rule
        let rules = RuleParser::parse_json(json).unwrap();
        assert_eq!(rules.len(), 0);
    }

    #[test]
    fn test_generic_rule_parsing() {
        let json = r#"
        {
            "id": "velocity-check",
            "name": "High Velocity Detection",
            "version": "1.0",
            "conditions": [
                {"field": "transaction_count_1h", "operator": "gt", "value": 10}
            ],
            "action": "flag",
            "score_impact": 30,
            "enabled": true
        }
        "#;
        let rule = RuleParser::parse(json).unwrap();
        assert_eq!(rule.name(), "High Velocity Detection");
    }

    #[test]
    fn test_generic_rule_evaluation() {
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

        let mut context = RuleContext {
            tenant_id: ramp_common::types::TenantId::new(uuid::Uuid::new_v4().to_string()),
            user_id: ramp_common::types::UserId::new(uuid::Uuid::new_v4().to_string()),
            current_amount: Decimal::from(1500),
            transaction_type: crate::aml::TransactionType::Payin,
            timestamp: chrono::Utc::now(),
            metadata: serde_json::json!({}),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        // Should match (1500 > 1000)
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(rule.evaluate(&context)).unwrap();
        assert!(!result.passed); // Failed rule means "matched" and flagged
        assert!(result.create_case);
        assert_eq!(result.risk_score.map(|s| s.0), Some(50.0));

        // Should not match (500 < 1000)
        context.current_amount = Decimal::from(500);
        let result = rt.block_on(rule.evaluate(&context)).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_generic_rule_composition() {
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

        let context = RuleContext {
            tenant_id: ramp_common::types::TenantId::new(uuid::Uuid::new_v4().to_string()),
            user_id: ramp_common::types::UserId::new(uuid::Uuid::new_v4().to_string()),
            current_amount: Decimal::from(1500),
            transaction_type: crate::aml::TransactionType::Payin,
            timestamp: chrono::Utc::now(),
            metadata: serde_json::json!({
                "country": "VN"
            }),
            user_full_name: None,
            user_country: None,
            user_address: None,
        };

        // Should match
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(rule.evaluate(&context)).unwrap();
        assert!(!result.passed);

        // Fail first condition
        let mut context2 = context.clone();
        context2.current_amount = Decimal::from(500);
        let result = rt.block_on(rule.evaluate(&context2)).unwrap();
        assert!(result.passed);

        // Fail second condition
        let mut context3 = context.clone();
        context3.metadata = serde_json::json!({"country": "US"});
        let result = rt.block_on(rule.evaluate(&context3)).unwrap();
        assert!(result.passed);
    }
}
