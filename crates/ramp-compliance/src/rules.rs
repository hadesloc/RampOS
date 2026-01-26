pub mod version;
pub mod sanctions;


use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ramp_common::{
    types::{TenantId, UserId},
    Result,
};
use redis::AsyncCommands;
use rust_decimal::Decimal;
use tracing::error;

use crate::aml::TransactionType;
use crate::rule_parser::{RuleDefinition, RuleParser, RulesConfig};
use crate::types::{CaseSeverity, CaseType, RiskScore};

/// Context passed to AML rules
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
    pub passed: bool,
    pub reason: String,
    pub risk_score: Option<RiskScore>,
    pub severity: Option<CaseSeverity>,
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

pub type CompiledRule = Box<dyn AmlRule>;

/// Rule configuration (loaded from database or config)
#[derive(Debug, Clone)]
pub struct RuleConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub parameters: serde_json::Value,
    pub severity: CaseSeverity,
}

/// Manages caching of compiled rules in Redis
pub struct RuleCacheManager {
    client: redis::Client,
    ttl_seconds: u64,
}

impl RuleCacheManager {
    pub fn new(client: redis::Client, ttl_seconds: u64) -> Self {
        Self {
            client,
            ttl_seconds,
        }
    }

    fn get_key(tenant_id: &TenantId) -> String {
        format!("aml:rules:{}", tenant_id)
    }

    pub async fn get_rules(&self, tenant_id: &TenantId) -> Option<Vec<CompiledRule>> {
        let key = Self::get_key(tenant_id);
        let mut conn = match self.client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to get redis connection: {}", e);
                return None;
            }
        };

        let data: String = match conn.get(&key).await {
            Ok(data) => data,
            Err(_) => return None, // Cache miss or error
        };

        let definitions: Vec<RuleDefinition> = match serde_json::from_str(&data) {
            Ok(defs) => defs,
            Err(e) => {
                error!("Failed to deserialize rules from cache: {}", e);
                return None;
            }
        };

        // Compile rules using RuleParser
        let config = RulesConfig {
            version: "1.0".to_string(),
            rules: definitions,
            metadata: None,
        };

        match RuleParser::parse_config(&config) {
            Ok(rules) => Some(rules),
            Err(e) => {
                error!("Failed to compile rules from cache: {}", e);
                None
            }
        }
    }

    pub async fn set_rules(
        &self,
        tenant_id: &TenantId,
        rules: &[RuleDefinition],
        ttl: Option<u64>,
    ) -> Result<()> {
        let key = Self::get_key(tenant_id);
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        // Serialize rule definitions to JSON
        let data = serde_json::to_string(rules)
            .map_err(|e| ramp_common::Error::Serialization(e.to_string()))?;

        let ttl_val = ttl.unwrap_or(self.ttl_seconds);
        conn.set_ex::<_, _, ()>(key, data, ttl_val)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn invalidate(&self, tenant_id: &TenantId) -> Result<()> {
        let key = Self::get_key(tenant_id);
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        conn.del::<_, ()>(key)
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn invalidate_all(&self) -> Result<()> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ramp_common::Error::Database(e.to_string()))?;

        // Scan for keys matching pattern
        let mut keys = Vec::new();
        {
            let mut iter: redis::AsyncIter<String> = conn
                .scan_match("aml:rules:*")
                .await
                .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
            while let Some(key) = iter.next_item().await {
                keys.push(key);
            }
        } // iter is dropped here

        if !keys.is_empty() {
            conn.del::<_, ()>(keys)
                .await
                .map_err(|e| ramp_common::Error::Database(e.to_string()))?;
        }

        Ok(())
    }
}
