use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::repository::rfq::LpReliabilitySnapshotRow;
use crate::service::rfq::{lp_counterparty_pressure_label, lp_counterparty_pressure_score};
use crate::service::settlement::{Settlement, SettlementStatus};
use crate::r#yield::{
    recommended_treasury_buffer_percent, StrategyConfig, YieldAllocationConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreasuryActionMode {
    RecommendationOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryFloatSlice {
    pub segment: String,
    pub asset: String,
    pub available: String,
    pub reserved: String,
    pub utilization_pct: i64,
    pub shortage_risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryLiquidityForecast {
    pub asset: String,
    pub horizon_hours: i64,
    pub projected_available: String,
    pub projected_required: String,
    pub shortage_amount: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryExposureSummary {
    pub counterparty_type: String,
    pub counterparty_id: String,
    pub direction: String,
    pub pressure_score: String,
    pub concentration: String,
    pub reliability_score: Option<String>,
    pub p95_settlement_latency_seconds: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryStressAlert {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub summary: String,
    pub recommendation_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryRecommendation {
    pub id: String,
    pub category: String,
    pub title: String,
    pub summary: String,
    pub asset: String,
    pub amount: String,
    pub source_segment: Option<String>,
    pub destination_segment: Option<String>,
    pub confidence: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryYieldAllocation {
    pub protocol: String,
    pub principal_amount: String,
    pub current_value: String,
    pub accrued_yield: String,
    pub share_percent: String,
    pub strategy_posture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreasuryControlTowerSnapshot {
    pub generated_at: String,
    pub forecast_window_hours: i64,
    pub action_mode: String,
    pub buffer_target_percent: u8,
    pub policy_hint: String,
    pub float_slices: Vec<TreasuryFloatSlice>,
    pub forecasts: Vec<TreasuryLiquidityForecast>,
    pub exposures: Vec<TreasuryExposureSummary>,
    pub alerts: Vec<TreasuryStressAlert>,
    pub recommendations: Vec<TreasuryRecommendation>,
    pub yield_allocations: Vec<TreasuryYieldAllocation>,
}

pub struct TreasuryService;

impl TreasuryService {
    pub fn new() -> Self {
        Self
    }

    pub fn build_control_tower(
        &self,
        scenario: Option<&str>,
    ) -> TreasuryControlTowerSnapshot {
        let now = Utc::now();
        let config = YieldAllocationConfig::default();
        let strategy = StrategyConfig::default();
        let float_slices = sample_float_slices(scenario);
        let settlements = sample_settlements(scenario);
        let exposures = sample_exposures(scenario);
        let yield_allocations = sample_yield_allocations(scenario);
        let forecasts = build_forecasts(&float_slices, &settlements);
        let recommendations =
            build_recommendations(&float_slices, &forecasts, &exposures, &config, &strategy);
        let alerts = build_alerts(&settlements, &forecasts, &exposures, &recommendations);

        TreasuryControlTowerSnapshot {
            generated_at: now.to_rfc3339(),
            forecast_window_hours: 24,
            action_mode: "recommendation_only".to_string(),
            buffer_target_percent: recommended_treasury_buffer_percent(&config),
            policy_hint: strategy.treasury_policy_hint(),
            float_slices,
            forecasts,
            exposures: exposures
                .into_iter()
                .map(|snapshot| TreasuryExposureSummary {
                    counterparty_type: "liquidity_provider".to_string(),
                    counterparty_id: snapshot.lp_id.clone(),
                    direction: snapshot.direction.clone(),
                    pressure_score: decimal_to_string(lp_counterparty_pressure_score(&snapshot)),
                    concentration: lp_counterparty_pressure_label(
                        &lp_counterparty_pressure_score(&snapshot),
                    )
                    .to_string(),
                    reliability_score: snapshot.reliability_score.map(decimal_to_string),
                    p95_settlement_latency_seconds: snapshot.p95_settlement_latency_seconds,
                })
                .collect(),
            alerts,
            recommendations,
            yield_allocations,
        }
    }
}

impl Default for TreasuryService {
    fn default() -> Self {
        Self::new()
    }
}

fn build_forecasts(
    float_slices: &[TreasuryFloatSlice],
    settlements: &[Settlement],
) -> Vec<TreasuryLiquidityForecast> {
    let outstanding = settlements
        .iter()
        .filter(|settlement| settlement.status.counts_toward_liquidity_pressure())
        .fold(Decimal::ZERO, |acc, _| acc + Decimal::from(125));

    float_slices
        .iter()
        .map(|slice| {
            let available = parse_decimal(&slice.available);
            let required = if slice.segment.contains("bank") {
                outstanding
            } else {
                Decimal::from(90)
            };
            let shortage = (required - available).max(Decimal::ZERO);
            TreasuryLiquidityForecast {
                asset: slice.asset.clone(),
                horizon_hours: 24,
                projected_available: decimal_to_string(available),
                projected_required: decimal_to_string(required),
                shortage_amount: decimal_to_string(shortage),
                confidence: if shortage > Decimal::ZERO {
                    "high".to_string()
                } else {
                    "medium".to_string()
                },
            }
        })
        .collect()
}

fn build_recommendations(
    float_slices: &[TreasuryFloatSlice],
    forecasts: &[TreasuryLiquidityForecast],
    exposures: &[LpReliabilitySnapshotRow],
    config: &YieldAllocationConfig,
    strategy: &StrategyConfig,
) -> Vec<TreasuryRecommendation> {
    let mut recommendations = Vec::new();

    if let Some(bank_shortage) = forecasts
        .iter()
        .find(|forecast| parse_decimal(&forecast.shortage_amount) > Decimal::ZERO)
    {
        recommendations.push(TreasuryRecommendation {
            id: "treasury_prefund_bank_vnd".to_string(),
            category: "prefund".to_string(),
            title: "Prefund the highest-pressure bank rail".to_string(),
            summary: format!(
                "Hold an extra {} {} buffer on the primary bank rail to absorb the current settlement backlog.",
                bank_shortage.shortage_amount, bank_shortage.asset
            ),
            asset: bank_shortage.asset.clone(),
            amount: bank_shortage.shortage_amount.clone(),
            source_segment: Some("chain:ethereum/usdt".to_string()),
            destination_segment: Some("bank:vcb/vnd".to_string()),
            confidence: bank_shortage.confidence.clone(),
            mode: "recommendation_only".to_string(),
        });
    }

    if let Some(riskiest_lp) = exposures.iter().max_by(|left, right| {
        lp_counterparty_pressure_score(left).cmp(&lp_counterparty_pressure_score(right))
    }) {
        recommendations.push(TreasuryRecommendation {
            id: "treasury_reduce_lp_concentration".to_string(),
            category: "counterparty".to_string(),
            title: "Reduce LP concentration".to_string(),
            summary: format!(
                "Shift incremental routing away from {} until reliability stabilizes below the treasury stress band.",
                riskiest_lp.lp_id
            ),
            asset: "USDT".to_string(),
            amount: decimal_to_string(Decimal::from(250)),
            source_segment: Some(format!("lp:{}", riskiest_lp.lp_id)),
            destination_segment: Some("lp:secondary_pool".to_string()),
            confidence: "medium".to_string(),
            mode: "recommendation_only".to_string(),
        });
    }

    let deployable_cash = float_slices
        .iter()
        .filter(|slice| slice.segment.starts_with("chain:"))
        .map(|slice| parse_decimal(&slice.available))
        .fold(Decimal::ZERO, |acc, value| acc + value);
    let max_allocatable = deployable_cash
        * Decimal::from(config.max_allocation_percent)
        / Decimal::from(100);

    recommendations.push(TreasuryRecommendation {
        id: "treasury_yield_parking_hint".to_string(),
        category: "yield".to_string(),
        title: "Keep excess stablecoin in policy-sized parking lanes".to_string(),
        summary: format!(
            "{}. Keep at least {}% as instant-access treasury buffer before any parking decision.",
            strategy.treasury_policy_hint(),
            recommended_treasury_buffer_percent(config)
        ),
        asset: "USDC".to_string(),
        amount: decimal_to_string(max_allocatable),
        source_segment: Some("chain:arbitrum/usdc".to_string()),
        destination_segment: Some("yield:balanced".to_string()),
        confidence: "medium".to_string(),
        mode: "recommendation_only".to_string(),
    });

    recommendations
}

fn build_alerts(
    settlements: &[Settlement],
    forecasts: &[TreasuryLiquidityForecast],
    exposures: &[LpReliabilitySnapshotRow],
    recommendations: &[TreasuryRecommendation],
) -> Vec<TreasuryStressAlert> {
    let now = Utc::now();
    let backlog_count = settlements
        .iter()
        .filter(|settlement| settlement.status.counts_toward_liquidity_pressure())
        .count();
    let aging_backlog = settlements
        .iter()
        .filter(|settlement| settlement.status.counts_toward_liquidity_pressure())
        .any(|settlement| settlement.age_minutes(now) >= 45);
    let shortage = forecasts
        .iter()
        .find(|forecast| parse_decimal(&forecast.shortage_amount) > Decimal::ZERO);
    let exposure = exposures.iter().max_by(|left, right| {
        lp_counterparty_pressure_score(left).cmp(&lp_counterparty_pressure_score(right))
    });

    let mut alerts = Vec::new();
    if backlog_count > 0 {
        alerts.push(TreasuryStressAlert {
            id: "alert_settlement_backlog".to_string(),
            severity: if aging_backlog {
                "high".to_string()
            } else {
                "medium".to_string()
            },
            title: "Settlement backlog is consuming working float".to_string(),
            summary: format!(
                "{} settlements are still pending or processing across the 24h treasury window.",
                backlog_count
            ),
            recommendation_ids: vec!["treasury_prefund_bank_vnd".to_string()],
        });
    }
    if let Some(shortage) = shortage {
        alerts.push(TreasuryStressAlert {
            id: "alert_float_shortage".to_string(),
            severity: "critical".to_string(),
            title: "Bank float forecast is below requirement".to_string(),
            summary: format!(
                "Projected shortage of {} {} in the next {} hours.",
                shortage.shortage_amount, shortage.asset, shortage.horizon_hours
            ),
            recommendation_ids: vec!["treasury_prefund_bank_vnd".to_string()],
        });
    }
    if let Some(exposure) = exposure {
        alerts.push(TreasuryStressAlert {
            id: "alert_counterparty_exposure".to_string(),
            severity: "medium".to_string(),
            title: "LP concentration is elevated".to_string(),
            summary: format!(
                "{} is currently the highest-pressure settlement counterparty.",
                exposure.lp_id
            ),
            recommendation_ids: vec!["treasury_reduce_lp_concentration".to_string()],
        });
    }

    if recommendations.is_empty() {
        alerts.push(TreasuryStressAlert {
            id: "alert_no_action".to_string(),
            severity: "low".to_string(),
            title: "Treasury posture is healthy".to_string(),
            summary: "No recommendation crossed the current treasury action threshold."
                .to_string(),
            recommendation_ids: Vec::new(),
        });
    }

    alerts
}

fn sample_float_slices(scenario: Option<&str>) -> Vec<TreasuryFloatSlice> {
    if matches!(scenario, Some("stable")) {
        return vec![
            TreasuryFloatSlice {
                segment: "bank:vcb/vnd".to_string(),
                asset: "VND".to_string(),
                available: decimal_to_string(Decimal::from(650)),
                reserved: decimal_to_string(Decimal::from(120)),
                utilization_pct: 18,
                shortage_risk: "low".to_string(),
            },
            TreasuryFloatSlice {
                segment: "chain:arbitrum/usdc".to_string(),
                asset: "USDC".to_string(),
                available: decimal_to_string(Decimal::from(500)),
                reserved: decimal_to_string(Decimal::from(80)),
                utilization_pct: 16,
                shortage_risk: "low".to_string(),
            },
        ];
    }

    vec![
        TreasuryFloatSlice {
            segment: "bank:vcb/vnd".to_string(),
            asset: "VND".to_string(),
            available: decimal_to_string(Decimal::from(210)),
            reserved: decimal_to_string(Decimal::from(160)),
            utilization_pct: 76,
            shortage_risk: "high".to_string(),
        },
        TreasuryFloatSlice {
            segment: "chain:ethereum/usdt".to_string(),
            asset: "USDT".to_string(),
            available: decimal_to_string(Decimal::from(340)),
            reserved: decimal_to_string(Decimal::from(90)),
            utilization_pct: 26,
            shortage_risk: "medium".to_string(),
        },
        TreasuryFloatSlice {
            segment: "chain:arbitrum/usdc".to_string(),
            asset: "USDC".to_string(),
            available: decimal_to_string(Decimal::from(420)),
            reserved: decimal_to_string(Decimal::from(140)),
            utilization_pct: 33,
            shortage_risk: "low".to_string(),
        },
    ]
}

fn sample_settlements(scenario: Option<&str>) -> Vec<Settlement> {
    let now = Utc::now();

    if matches!(scenario, Some("stable")) {
        return vec![Settlement {
            id: "stl_treasury_stable_001".to_string(),
            offramp_intent_id: "ofr_treasury_stable_001".to_string(),
            status: SettlementStatus::Completed,
            bank_reference: Some("RAMP-STABLE".to_string()),
            error_message: None,
            created_at: now - Duration::minutes(50),
            updated_at: now - Duration::minutes(10),
        }];
    }

    vec![
        Settlement {
            id: "stl_treasury_pending_001".to_string(),
            offramp_intent_id: "ofr_treasury_pending_001".to_string(),
            status: SettlementStatus::Pending,
            bank_reference: Some("RAMP-BACKLOG".to_string()),
            error_message: None,
            created_at: now - Duration::minutes(65),
            updated_at: now - Duration::minutes(40),
        },
        Settlement {
            id: "stl_treasury_processing_001".to_string(),
            offramp_intent_id: "ofr_treasury_processing_001".to_string(),
            status: SettlementStatus::Processing,
            bank_reference: Some("RAMP-PIPELINE".to_string()),
            error_message: None,
            created_at: now - Duration::minutes(38),
            updated_at: now - Duration::minutes(12),
        },
    ]
}

fn sample_exposures(scenario: Option<&str>) -> Vec<LpReliabilitySnapshotRow> {
    let now = Utc::now();
    if matches!(scenario, Some("stable")) {
        return vec![LpReliabilitySnapshotRow {
            id: "lprs_stable_001".to_string(),
            tenant_id: "tenant_demo".to_string(),
            lp_id: "lp_primary".to_string(),
            direction: "OFFRAMP".to_string(),
            window_kind: "rolling_24h".to_string(),
            window_started_at: now - Duration::hours(24),
            window_ended_at: now,
            snapshot_version: "w9-demo".to_string(),
            quote_count: 48,
            fill_count: 45,
            reject_count: 2,
            settlement_count: 44,
            dispute_count: 0,
            fill_rate: Decimal::new(94, 2),
            reject_rate: Decimal::new(4, 2),
            dispute_rate: Decimal::new(0, 2),
            avg_slippage_bps: Decimal::from(8),
            p95_settlement_latency_seconds: 420,
            reliability_score: Some(Decimal::from(91)),
            metadata: json!({ "counterparty": "bank_vcb" }),
            created_at: now,
            updated_at: now,
        }];
    }

    vec![
        LpReliabilitySnapshotRow {
            id: "lprs_active_001".to_string(),
            tenant_id: "tenant_demo".to_string(),
            lp_id: "lp_alpha".to_string(),
            direction: "OFFRAMP".to_string(),
            window_kind: "rolling_24h".to_string(),
            window_started_at: now - Duration::hours(24),
            window_ended_at: now,
            snapshot_version: "w9-demo".to_string(),
            quote_count: 36,
            fill_count: 22,
            reject_count: 10,
            settlement_count: 19,
            dispute_count: 2,
            fill_rate: Decimal::new(61, 2),
            reject_rate: Decimal::new(28, 2),
            dispute_rate: Decimal::new(6, 2),
            avg_slippage_bps: Decimal::from(41),
            p95_settlement_latency_seconds: 1900,
            reliability_score: Some(Decimal::from(63)),
            metadata: json!({ "counterparty": "lp_alpha" }),
            created_at: now,
            updated_at: now,
        },
        LpReliabilitySnapshotRow {
            id: "lprs_active_002".to_string(),
            tenant_id: "tenant_demo".to_string(),
            lp_id: "lp_beta".to_string(),
            direction: "ONRAMP".to_string(),
            window_kind: "rolling_24h".to_string(),
            window_started_at: now - Duration::hours(24),
            window_ended_at: now,
            snapshot_version: "w9-demo".to_string(),
            quote_count: 42,
            fill_count: 35,
            reject_count: 5,
            settlement_count: 33,
            dispute_count: 1,
            fill_rate: Decimal::new(83, 2),
            reject_rate: Decimal::new(12, 2),
            dispute_rate: Decimal::new(2, 2),
            avg_slippage_bps: Decimal::from(19),
            p95_settlement_latency_seconds: 980,
            reliability_score: Some(Decimal::from(82)),
            metadata: json!({ "counterparty": "lp_beta" }),
            created_at: now,
            updated_at: now,
        },
    ]
}

fn sample_yield_allocations(scenario: Option<&str>) -> Vec<TreasuryYieldAllocation> {
    if matches!(scenario, Some("stable")) {
        return vec![TreasuryYieldAllocation {
            protocol: "aave-v3".to_string(),
            principal_amount: "500000".to_string(),
            current_value: "512000".to_string(),
            accrued_yield: "12000".to_string(),
            share_percent: "48".to_string(),
            strategy_posture: "capital_preservation".to_string(),
        }];
    }

    vec![
        TreasuryYieldAllocation {
            protocol: "aave-v3".to_string(),
            principal_amount: "800000".to_string(),
            current_value: "824000".to_string(),
            accrued_yield: "24000".to_string(),
            share_percent: "65".to_string(),
            strategy_posture: "capital_preservation".to_string(),
        },
        TreasuryYieldAllocation {
            protocol: "compound-v3".to_string(),
            principal_amount: "320000".to_string(),
            current_value: "329600".to_string(),
            accrued_yield: "9600".to_string(),
            share_percent: "35".to_string(),
            strategy_posture: "balanced_liquidity".to_string(),
        },
    ]
}

fn parse_decimal(value: &str) -> Decimal {
    value.parse::<Decimal>().unwrap_or(Decimal::ZERO)
}

fn decimal_to_string(value: Decimal) -> String {
    value.round_dp(2).normalize().to_string()
}
