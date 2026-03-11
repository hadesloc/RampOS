//! Fee Calculation Engine
//!
//! Supports multiple fee types per tenant:
//! - Flat: fixed amount (e.g., 10,000 VND)
//! - Percentage: % of transaction amount (e.g., 0.5%)
//! - Tiered: different rates for different amount ranges
//! - Min/Max caps applied after calculation

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Fee type determines how the fee is calculated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeeType {
    Flat,
    Percentage,
    Tiered,
}

/// A tier defines a rate for a given amount range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeTier {
    /// Lower bound (inclusive)
    pub from: Decimal,
    /// Upper bound (exclusive), None = unlimited
    pub to: Option<Decimal>,
    /// Rate for this tier (as a decimal fraction, e.g. 0.005 = 0.5%)
    pub rate: Decimal,
}

/// Configuration for a single fee schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeConfig {
    pub fee_type: FeeType,
    /// Fixed amount for Flat fee type
    pub flat_amount: Option<Decimal>,
    /// Rate for Percentage fee type (as decimal fraction)
    pub rate: Option<Decimal>,
    /// Tiers for Tiered fee type
    pub tiers: Option<Vec<FeeTier>>,
    /// Minimum fee (floor)
    pub min_fee: Option<Decimal>,
    /// Maximum fee (cap)
    pub max_fee: Option<Decimal>,
}

/// Per-tenant fee configuration for each operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantFeeConfig {
    pub deposit: FeeConfig,
    pub withdrawal: FeeConfig,
    pub trade: FeeConfig,
}

/// Calculated fee result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Fee {
    pub amount: Decimal,
    pub fee_type: FeeType,
    pub rate: Option<Decimal>,
    pub min_fee: Option<Decimal>,
    pub max_fee: Option<Decimal>,
}

// ============================================================================
// Fee Calculator
// ============================================================================

pub struct FeeCalculator {
    /// Per-tenant fee configs. Key = tenant_id
    tenant_configs: HashMap<String, TenantFeeConfig>,
    /// Default config used when tenant has no custom config
    default_config: TenantFeeConfig,
}

impl FeeCalculator {
    /// Create a new FeeCalculator with default fee configuration
    pub fn new() -> Self {
        let default_config = TenantFeeConfig {
            deposit: FeeConfig {
                fee_type: FeeType::Flat,
                flat_amount: Some(Decimal::new(10000, 0)),
                rate: None,
                tiers: None,
                min_fee: None,
                max_fee: None,
            },
            withdrawal: FeeConfig {
                fee_type: FeeType::Percentage,
                flat_amount: None,
                rate: Some(Decimal::new(5, 3)), // 0.005 = 0.5%
                tiers: None,
                min_fee: Some(Decimal::new(10000, 0)),
                max_fee: Some(Decimal::new(500000, 0)),
            },
            trade: FeeConfig {
                fee_type: FeeType::Tiered,
                flat_amount: None,
                rate: None,
                tiers: Some(vec![
                    FeeTier {
                        from: Decimal::ZERO,
                        to: Some(Decimal::new(10_000_000, 0)),
                        rate: Decimal::new(3, 3), // 0.003 = 0.3%
                    },
                    FeeTier {
                        from: Decimal::new(10_000_000, 0),
                        to: Some(Decimal::new(100_000_000, 0)),
                        rate: Decimal::new(2, 3), // 0.002 = 0.2%
                    },
                    FeeTier {
                        from: Decimal::new(100_000_000, 0),
                        to: None,
                        rate: Decimal::new(1, 3), // 0.001 = 0.1%
                    },
                ]),
                min_fee: Some(Decimal::new(5000, 0)),
                max_fee: None,
            },
        };

        Self {
            tenant_configs: HashMap::new(),
            default_config,
        }
    }

    /// Register a custom fee config for a tenant
    pub fn set_tenant_config(&mut self, tenant_id: &str, config: TenantFeeConfig) {
        self.tenant_configs.insert(tenant_id.to_string(), config);
    }

    /// Calculate deposit fee
    pub fn calculate_deposit_fee(&self, amount: Decimal, _currency: &str, tenant_id: &str) -> Fee {
        let config = self.get_config(tenant_id);
        self.calculate(&config.deposit, amount)
    }

    /// Calculate withdrawal fee
    pub fn calculate_withdrawal_fee(
        &self,
        amount: Decimal,
        _currency: &str,
        tenant_id: &str,
    ) -> Fee {
        let config = self.get_config(tenant_id);
        self.calculate(&config.withdrawal, amount)
    }

    /// Calculate trade fee
    pub fn calculate_trade_fee(&self, amount: Decimal, _currency: &str, tenant_id: &str) -> Fee {
        let config = self.get_config(tenant_id);
        self.calculate(&config.trade, amount)
    }

    fn get_config(&self, tenant_id: &str) -> &TenantFeeConfig {
        self.tenant_configs
            .get(tenant_id)
            .unwrap_or(&self.default_config)
    }

    fn calculate(&self, config: &FeeConfig, amount: Decimal) -> Fee {
        if amount <= Decimal::ZERO {
            return Fee {
                amount: Decimal::ZERO,
                fee_type: config.fee_type.clone(),
                rate: config.rate,
                min_fee: config.min_fee,
                max_fee: config.max_fee,
            };
        }

        let raw_fee = match config.fee_type {
            FeeType::Flat => config.flat_amount.unwrap_or(Decimal::ZERO),
            FeeType::Percentage => {
                let rate = config.rate.unwrap_or(Decimal::ZERO);
                amount * rate
            }
            FeeType::Tiered => self.calculate_tiered(config, amount),
        };

        // Apply min/max caps
        let mut fee = raw_fee;
        if let Some(min) = config.min_fee {
            if fee < min {
                fee = min;
            }
        }
        if let Some(max) = config.max_fee {
            if fee > max {
                fee = max;
            }
        }

        Fee {
            amount: fee,
            fee_type: config.fee_type.clone(),
            rate: config.rate,
            min_fee: config.min_fee,
            max_fee: config.max_fee,
        }
    }

    fn calculate_tiered(&self, config: &FeeConfig, amount: Decimal) -> Decimal {
        let tiers = match &config.tiers {
            Some(t) => t,
            None => return Decimal::ZERO,
        };

        // Find the matching tier based on the amount
        for tier in tiers {
            let in_range = amount >= tier.from && tier.to.map_or(true, |to| amount < to);
            if in_range {
                return amount * tier.rate;
            }
        }

        // Fallback: use the last tier if amount exceeds all ranges
        tiers
            .last()
            .map(|t| amount * t.rate)
            .unwrap_or(Decimal::ZERO)
    }
}

impl Default for FeeCalculator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// VND Formatting Helper
// ============================================================================

/// Format a VND amount with dot as thousands separator
/// e.g. 1000000 -> "1.000.000"
pub fn format_vnd(amount: Decimal) -> String {
    let is_negative = amount < Decimal::ZERO;
    let abs_amount = if is_negative { -amount } else { amount };

    // Truncate to integer part for VND (no decimal places)
    let integer = abs_amount.trunc();
    let s = integer.to_string();

    // Insert dots as thousands separators (from right to left)
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push('.');
        }
        result.push(*ch);
    }

    if is_negative {
        format!("-{}", result)
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_flat_fee() {
        let calc = FeeCalculator::new();
        let fee = calc.calculate_deposit_fee(dec!(1_000_000), "VND", "test-tenant");
        assert_eq!(fee.amount, dec!(10000));
        assert_eq!(fee.fee_type, FeeType::Flat);
    }

    #[test]
    fn test_percentage_fee() {
        let calc = FeeCalculator::new();
        // Default withdrawal is 0.5%
        let fee = calc.calculate_withdrawal_fee(dec!(1_000_000), "VND", "test-tenant");
        // 1,000,000 * 0.005 = 5,000 but min is 10,000
        assert_eq!(fee.amount, dec!(10000));
    }

    #[test]
    fn test_percentage_fee_normal() {
        let calc = FeeCalculator::new();
        // 10,000,000 * 0.005 = 50,000 (within min/max)
        let fee = calc.calculate_withdrawal_fee(dec!(10_000_000), "VND", "test-tenant");
        assert_eq!(fee.amount, dec!(50000));
    }

    #[test]
    fn test_percentage_fee_max_cap() {
        let calc = FeeCalculator::new();
        // 200,000,000 * 0.005 = 1,000,000 but max is 500,000
        let fee = calc.calculate_withdrawal_fee(dec!(200_000_000), "VND", "test-tenant");
        assert_eq!(fee.amount, dec!(500000));
    }

    #[test]
    fn test_tiered_fee_low() {
        let calc = FeeCalculator::new();
        // 5,000,000 in first tier (0-10M): 0.3%
        let fee = calc.calculate_trade_fee(dec!(5_000_000), "VND", "test-tenant");
        // 5,000,000 * 0.003 = 15,000
        assert_eq!(fee.amount, dec!(15000));
    }

    #[test]
    fn test_tiered_fee_mid() {
        let calc = FeeCalculator::new();
        // 50,000,000 in second tier (10M-100M): 0.2%
        let fee = calc.calculate_trade_fee(dec!(50_000_000), "VND", "test-tenant");
        // 50,000,000 * 0.002 = 100,000
        assert_eq!(fee.amount, dec!(100000));
    }

    #[test]
    fn test_tiered_fee_high() {
        let calc = FeeCalculator::new();
        // 200,000,000 in third tier (100M+): 0.1%
        let fee = calc.calculate_trade_fee(dec!(200_000_000), "VND", "test-tenant");
        // 200,000,000 * 0.001 = 200,000
        assert_eq!(fee.amount, dec!(200000));
    }

    #[test]
    fn test_tiered_fee_min_cap() {
        let calc = FeeCalculator::new();
        // 1,000,000 * 0.003 = 3,000 but min is 5,000
        let fee = calc.calculate_trade_fee(dec!(1_000_000), "VND", "test-tenant");
        assert_eq!(fee.amount, dec!(5000));
    }

    #[test]
    fn test_zero_amount() {
        let calc = FeeCalculator::new();
        let fee = calc.calculate_deposit_fee(dec!(0), "VND", "test-tenant");
        assert_eq!(fee.amount, dec!(0));
    }

    #[test]
    fn test_negative_amount() {
        let calc = FeeCalculator::new();
        let fee = calc.calculate_deposit_fee(dec!(-100), "VND", "test-tenant");
        assert_eq!(fee.amount, dec!(0));
    }

    #[test]
    fn test_very_large_amount() {
        let calc = FeeCalculator::new();
        // Flat fee should still be fixed regardless of amount
        let fee = calc.calculate_deposit_fee(dec!(999_999_999_999), "VND", "test-tenant");
        assert_eq!(fee.amount, dec!(10000));
    }

    #[test]
    fn test_custom_tenant_config() {
        let mut calc = FeeCalculator::new();
        calc.set_tenant_config(
            "custom-tenant",
            TenantFeeConfig {
                deposit: FeeConfig {
                    fee_type: FeeType::Flat,
                    flat_amount: Some(dec!(5000)),
                    rate: None,
                    tiers: None,
                    min_fee: None,
                    max_fee: None,
                },
                withdrawal: FeeConfig {
                    fee_type: FeeType::Percentage,
                    flat_amount: None,
                    rate: Some(dec!(0.01)),
                    tiers: None,
                    min_fee: None,
                    max_fee: None,
                },
                trade: FeeConfig {
                    fee_type: FeeType::Flat,
                    flat_amount: Some(dec!(20000)),
                    rate: None,
                    tiers: None,
                    min_fee: None,
                    max_fee: None,
                },
            },
        );

        let fee = calc.calculate_deposit_fee(dec!(1_000_000), "VND", "custom-tenant");
        assert_eq!(fee.amount, dec!(5000));

        // Default tenant should still use default config
        let fee = calc.calculate_deposit_fee(dec!(1_000_000), "VND", "other-tenant");
        assert_eq!(fee.amount, dec!(10000));
    }

    #[test]
    fn test_format_vnd_basic() {
        assert_eq!(format_vnd(dec!(1000000)), "1.000.000");
    }

    #[test]
    fn test_format_vnd_small() {
        assert_eq!(format_vnd(dec!(500)), "500");
    }

    #[test]
    fn test_format_vnd_zero() {
        assert_eq!(format_vnd(dec!(0)), "0");
    }

    #[test]
    fn test_format_vnd_thousands() {
        assert_eq!(format_vnd(dec!(10000)), "10.000");
    }

    #[test]
    fn test_format_vnd_large() {
        assert_eq!(format_vnd(dec!(1234567890)), "1.234.567.890");
    }

    #[test]
    fn test_format_vnd_negative() {
        assert_eq!(format_vnd(dec!(-1000000)), "-1.000.000");
    }

    #[test]
    fn test_format_vnd_with_decimals() {
        // VND should truncate decimals
        assert_eq!(format_vnd(dec!(1000000.75)), "1.000.000");
    }

    #[test]
    fn test_fee_serialization() {
        let fee = Fee {
            amount: dec!(10000),
            fee_type: FeeType::Flat,
            rate: None,
            min_fee: None,
            max_fee: None,
        };
        let json = serde_json::to_string(&fee).unwrap();
        assert!(json.contains("\"amount\""));
        assert!(json.contains("\"feeType\":\"flat\""));
    }
}
