//! Off-Ramp Fee Calculator (F16.04)
//!
//! Calculates fees for off-ramp transactions including:
//! - Network fees (gas)
//! - Platform fees (tiered 0.5% - 2%)
//! - Spread (0.1% - 0.3%)
//! - Bank transfer fees (0 for Napas 247, 3300 VND for SWIFT)

use ramp_common::types::CryptoSymbol;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Complete fee breakdown for an off-ramp transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    /// Network/gas fee in VND equivalent
    pub network_fee: Decimal,
    /// Platform fee in VND (tiered percentage)
    pub platform_fee: Decimal,
    /// Platform fee rate applied
    pub platform_fee_rate: Decimal,
    /// Spread fee in VND
    pub spread_fee: Decimal,
    /// Spread rate applied
    pub spread_rate: Decimal,
    /// Bank transfer fee in VND (0 for Napas 247, 3300 for SWIFT)
    pub bank_fee: Decimal,
    /// Total fees in VND
    pub total_fee: Decimal,
    /// Net amount user receives in VND (gross - total_fee)
    pub net_amount_vnd: Decimal,
    /// Gross amount before fees
    pub gross_amount_vnd: Decimal,
}

/// Fee tier definition
struct FeeTier {
    /// Lower bound (inclusive) in VND
    from_vnd: Decimal,
    /// Upper bound (exclusive) in VND, None = unlimited
    to_vnd: Option<Decimal>,
    /// Platform fee rate for this tier
    rate: Decimal,
}

// ============================================================================
// Off-Ramp Fee Calculator
// ============================================================================

pub struct OffRampFeeCalculator {
    /// Platform fee tiers
    tiers: Vec<FeeTier>,
}

impl OffRampFeeCalculator {
    /// Create with default fee tiers
    pub fn new() -> Self {
        Self {
            tiers: vec![
                FeeTier {
                    from_vnd: Decimal::ZERO,
                    to_vnd: Some(Decimal::new(10_000_000, 0)),     // < 10M VND
                    rate: Decimal::new(2, 2),                       // 2%
                },
                FeeTier {
                    from_vnd: Decimal::new(10_000_000, 0),
                    to_vnd: Some(Decimal::new(100_000_000, 0)),    // 10M - 100M VND
                    rate: Decimal::new(1, 2),                       // 1%
                },
                FeeTier {
                    from_vnd: Decimal::new(100_000_000, 0),
                    to_vnd: Some(Decimal::new(1_000_000_000, 0)),  // 100M - 1B VND
                    rate: Decimal::new(75, 4),                      // 0.75%
                },
                FeeTier {
                    from_vnd: Decimal::new(1_000_000_000, 0),
                    to_vnd: None,                                   // > 1B VND
                    rate: Decimal::new(5, 3),                       // 0.5%
                },
            ],
        }
    }

    /// Calculate all fees for an off-ramp transaction
    ///
    /// # Arguments
    /// * `gross_amount_vnd` - The VND equivalent of the crypto being sold (before fees)
    /// * `crypto_asset` - The crypto asset being sold (affects network fee)
    /// * `bank_type` - "domestic" for Napas 247 (free), "swift" for SWIFT (3300 VND)
    pub fn calculate_fees(
        &self,
        gross_amount_vnd: Decimal,
        crypto_asset: CryptoSymbol,
        bank_type: &str,
    ) -> FeeBreakdown {
        if gross_amount_vnd <= Decimal::ZERO {
            return FeeBreakdown {
                network_fee: Decimal::ZERO,
                platform_fee: Decimal::ZERO,
                platform_fee_rate: Decimal::ZERO,
                spread_fee: Decimal::ZERO,
                spread_rate: Decimal::ZERO,
                bank_fee: Decimal::ZERO,
                total_fee: Decimal::ZERO,
                net_amount_vnd: Decimal::ZERO,
                gross_amount_vnd,
            };
        }

        // 1. Network fee (gas cost in VND equivalent)
        let network_fee = self.get_network_fee(crypto_asset);

        // 2. Platform fee (tiered)
        let (platform_fee, platform_fee_rate) = self.calculate_platform_fee(gross_amount_vnd);

        // 3. Spread fee
        let (spread_fee, spread_rate) = self.calculate_spread(gross_amount_vnd, crypto_asset);

        // 4. Bank transfer fee
        let bank_fee = self.get_bank_fee(bank_type);

        // Total
        let total_fee = network_fee + platform_fee + spread_fee + bank_fee;
        let net_amount_vnd = if gross_amount_vnd > total_fee {
            gross_amount_vnd - total_fee
        } else {
            Decimal::ZERO
        };

        FeeBreakdown {
            network_fee,
            platform_fee,
            platform_fee_rate,
            spread_fee,
            spread_rate,
            bank_fee,
            total_fee,
            net_amount_vnd,
            gross_amount_vnd,
        }
    }

    /// Get the network fee for a crypto asset (simulated gas cost in VND)
    fn get_network_fee(&self, asset: CryptoSymbol) -> Decimal {
        match asset {
            CryptoSymbol::BTC => Decimal::new(250_000, 0),   // ~$10 USD in VND
            CryptoSymbol::ETH => Decimal::new(125_000, 0),   // ~$5 USD in VND
            CryptoSymbol::USDT => Decimal::new(75_000, 0),   // ~$3 USD (TRC20/ERC20)
            CryptoSymbol::USDC => Decimal::new(75_000, 0),   // ~$3 USD
            CryptoSymbol::BNB => Decimal::new(25_000, 0),    // ~$1 USD (BSC is cheap)
            CryptoSymbol::SOL => Decimal::new(2_500, 0),     // ~$0.1 USD (Solana is very cheap)
            CryptoSymbol::Other => Decimal::new(125_000, 0), // Default
        }
    }

    /// Calculate tiered platform fee
    fn calculate_platform_fee(&self, amount_vnd: Decimal) -> (Decimal, Decimal) {
        for tier in &self.tiers {
            let in_range = amount_vnd >= tier.from_vnd
                && tier.to_vnd.map_or(true, |to| amount_vnd < to);
            if in_range {
                return (amount_vnd * tier.rate, tier.rate);
            }
        }

        // Fallback to last tier
        if let Some(last) = self.tiers.last() {
            (amount_vnd * last.rate, last.rate)
        } else {
            (Decimal::ZERO, Decimal::ZERO)
        }
    }

    /// Calculate spread based on asset volatility
    fn calculate_spread(&self, amount_vnd: Decimal, asset: CryptoSymbol) -> (Decimal, Decimal) {
        let spread_rate = match asset {
            CryptoSymbol::USDT | CryptoSymbol::USDC => Decimal::new(1, 3), // 0.1% for stablecoins
            CryptoSymbol::BTC | CryptoSymbol::ETH => Decimal::new(2, 3),   // 0.2% for major
            _ => Decimal::new(3, 3),                                        // 0.3% for altcoins
        };

        (amount_vnd * spread_rate, spread_rate)
    }

    /// Get bank transfer fee
    fn get_bank_fee(&self, bank_type: &str) -> Decimal {
        match bank_type {
            "domestic" | "napas247" => Decimal::ZERO,         // Free domestic
            "swift" | "international" => Decimal::new(3300, 0), // 3,300 VND
            _ => Decimal::ZERO,
        }
    }
}

impl Default for OffRampFeeCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_small_amount_high_fee_rate() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(5_000_000), CryptoSymbol::USDT, "domestic");

        // Should be 2% for amounts < 10M VND
        assert_eq!(fees.platform_fee_rate, dec!(0.02));
        assert_eq!(fees.platform_fee, dec!(100_000)); // 5M * 2%
    }

    #[test]
    fn test_medium_amount_fee_rate() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(50_000_000), CryptoSymbol::ETH, "domestic");

        // Should be 1% for amounts 10M-100M VND
        assert_eq!(fees.platform_fee_rate, dec!(0.01));
        assert_eq!(fees.platform_fee, dec!(500_000)); // 50M * 1%
    }

    #[test]
    fn test_large_amount_fee_rate() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(500_000_000), CryptoSymbol::BTC, "domestic");

        // Should be 0.75% for amounts 100M-1B VND
        assert_eq!(fees.platform_fee_rate, dec!(0.0075));
        assert_eq!(fees.platform_fee, dec!(3_750_000)); // 500M * 0.75%
    }

    #[test]
    fn test_very_large_amount_fee_rate() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(2_000_000_000), CryptoSymbol::BTC, "domestic");

        // Should be 0.5% for amounts > 1B VND
        assert_eq!(fees.platform_fee_rate, dec!(0.005));
        assert_eq!(fees.platform_fee, dec!(10_000_000)); // 2B * 0.5%
    }

    #[test]
    fn test_network_fee_btc() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(100_000_000), CryptoSymbol::BTC, "domestic");
        assert_eq!(fees.network_fee, dec!(250_000));
    }

    #[test]
    fn test_network_fee_sol() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(100_000_000), CryptoSymbol::SOL, "domestic");
        assert_eq!(fees.network_fee, dec!(2_500)); // SOL is cheap
    }

    #[test]
    fn test_spread_stablecoin() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(100_000_000), CryptoSymbol::USDT, "domestic");

        // Stablecoin spread should be 0.1%
        assert_eq!(fees.spread_rate, dec!(0.001));
        assert_eq!(fees.spread_fee, dec!(100_000)); // 100M * 0.1%
    }

    #[test]
    fn test_spread_volatile_asset() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(100_000_000), CryptoSymbol::BNB, "domestic");

        // Altcoin spread should be 0.3%
        assert_eq!(fees.spread_rate, dec!(0.003));
        assert_eq!(fees.spread_fee, dec!(300_000)); // 100M * 0.3%
    }

    #[test]
    fn test_bank_fee_domestic() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(50_000_000), CryptoSymbol::ETH, "domestic");
        assert_eq!(fees.bank_fee, Decimal::ZERO);
    }

    #[test]
    fn test_bank_fee_swift() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(50_000_000), CryptoSymbol::ETH, "swift");
        assert_eq!(fees.bank_fee, dec!(3300));
    }

    #[test]
    fn test_total_fee_calculation() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(50_000_000), CryptoSymbol::ETH, "domestic");

        let expected_total = fees.network_fee + fees.platform_fee + fees.spread_fee + fees.bank_fee;
        assert_eq!(fees.total_fee, expected_total);
    }

    #[test]
    fn test_net_amount() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(50_000_000), CryptoSymbol::ETH, "domestic");

        assert_eq!(fees.net_amount_vnd, fees.gross_amount_vnd - fees.total_fee);
        assert!(fees.net_amount_vnd > Decimal::ZERO);
    }

    #[test]
    fn test_zero_amount() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(Decimal::ZERO, CryptoSymbol::BTC, "domestic");

        assert_eq!(fees.total_fee, Decimal::ZERO);
        assert_eq!(fees.net_amount_vnd, Decimal::ZERO);
    }

    #[test]
    fn test_negative_amount() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(-100), CryptoSymbol::BTC, "domestic");

        assert_eq!(fees.total_fee, Decimal::ZERO);
        assert_eq!(fees.net_amount_vnd, Decimal::ZERO);
    }

    #[test]
    fn test_fee_breakdown_serializable() {
        let calc = OffRampFeeCalculator::new();
        let fees = calc.calculate_fees(dec!(50_000_000), CryptoSymbol::ETH, "domestic");

        let json = serde_json::to_string(&fees).unwrap();
        assert!(json.contains("\"network_fee\""));
        assert!(json.contains("\"platform_fee\""));
        assert!(json.contains("\"net_amount_vnd\""));
    }
}
