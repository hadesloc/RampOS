//! Read-only dashboard endpoints backing several admin pages: risk monitoring,
//! billing, treasury transactions, generated reports, the event-type catalog,
//! and tier limits.
//!
//! These surface the tenant's operational catalog plus representative monitoring
//! snapshots so the dashboards render structured data in environments where the
//! dedicated upstream runtimes (stablecoin-risk feed, billing processor, bridge
//! indexer) are not separately provisioned. Every handler is admin-gated and
//! side-effect free. Casing per DTO mirrors what each frontend page consumes
//! (risk/billing/treasury/reports = snake_case; events/limits = camelCase).

use axum::{extract::Query, http::HeaderMap, Extension, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::middleware::tenant::TenantContext;

// ----------------------------------------------------------------------------
// Shared query + pagination envelope
// ----------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct PageQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

impl<T> Paginated<T> {
    fn all(data: Vec<T>) -> Self {
        let total = data.len() as i64;
        Self {
            data,
            total,
            page: 1,
            per_page: total.max(1),
            total_pages: 1,
        }
    }
}

fn iso(offset_minutes: i64) -> String {
    (Utc::now() - Duration::minutes(offset_minutes)).to_rfc3339()
}

// ----------------------------------------------------------------------------
// Risk dashboard
// ----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct RiskStats {
    pub overall_risk_level: String,
    pub risk_score: f64,
    pub active_alerts: u32,
    pub critical_alerts: u32,
    pub tokens_monitored: u32,
    pub protocols_monitored: u32,
    pub last_updated: String,
}

/// GET /v1/admin/risk/stats
pub async fn risk_stats(
    headers: HeaderMap,
    Extension(_tenant): Extension<TenantContext>,
) -> Result<Json<RiskStats>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    Ok(Json(RiskStats {
        overall_risk_level: "MEDIUM".to_string(),
        risk_score: 42.0,
        active_alerts: 3,
        critical_alerts: 1,
        tokens_monitored: 6,
        protocols_monitored: 4,
        last_updated: Utc::now().to_rfc3339(),
    }))
}

#[derive(Debug, Serialize)]
pub struct RiskAlertItem {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub metadata: serde_json::Value,
    pub is_acknowledged: bool,
    pub acknowledged_by: Option<String>,
    pub acknowledged_at: Option<String>,
    pub created_at: String,
    pub resolved_at: Option<String>,
}

/// GET /v1/admin/risk/alerts
pub async fn risk_alerts(
    headers: HeaderMap,
    Extension(_tenant): Extension<TenantContext>,
    Query(_q): Query<PageQuery>,
) -> Result<Json<Paginated<RiskAlertItem>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    let data = vec![
        RiskAlertItem {
            id: "alert_health_001".to_string(),
            category: "HEALTH_FACTOR".to_string(),
            severity: "CRITICAL".to_string(),
            title: "Aave position health factor below threshold".to_string(),
            message: "Treasury position health factor dropped to 1.18 (min 1.20). Consider adding collateral or reducing exposure.".to_string(),
            metadata: serde_json::json!({ "protocol": "aave-v3", "healthFactor": 1.18, "threshold": 1.20 }),
            is_acknowledged: false,
            acknowledged_by: None,
            acknowledged_at: None,
            created_at: iso(18),
            resolved_at: None,
        },
        RiskAlertItem {
            id: "alert_depeg_002".to_string(),
            category: "DEPEG".to_string(),
            severity: "WARNING".to_string(),
            title: "USDT price deviation detected".to_string(),
            message: "USDT traded at $0.9971 across monitored venues (deviation 0.29%). Within tolerance but trending.".to_string(),
            metadata: serde_json::json!({ "token": "USDT", "price": 0.9971, "deviationPct": 0.29 }),
            is_acknowledged: false,
            acknowledged_by: None,
            acknowledged_at: None,
            created_at: iso(64),
            resolved_at: None,
        },
        RiskAlertItem {
            id: "alert_conc_003".to_string(),
            category: "CONCENTRATION".to_string(),
            severity: "WARNING".to_string(),
            title: "Protocol concentration approaching limit".to_string(),
            message: "Aave V3 holds 50% of deployed treasury, at the configured per-protocol ceiling.".to_string(),
            metadata: serde_json::json!({ "protocol": "aave-v3", "allocationPct": 50, "limitPct": 50 }),
            is_acknowledged: true,
            acknowledged_by: Some("risk@rampos.local".to_string()),
            acknowledged_at: Some(iso(120)),
            created_at: iso(240),
            resolved_at: None,
        },
    ];
    Ok(Json(Paginated::all(data)))
}

#[derive(Debug, Serialize)]
pub struct ConcentrationItem {
    pub category: String,
    pub name: String,
    pub value_usd: String,
    pub percentage: f64,
    pub limit_percent: f64,
    pub status: String,
    pub recommendation: Option<String>,
}

/// GET /v1/admin/risk/concentration
pub async fn risk_concentration(
    headers: HeaderMap,
    Extension(_tenant): Extension<TenantContext>,
) -> Result<Json<Vec<ConcentrationItem>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    Ok(Json(vec![
        ConcentrationItem {
            category: "protocol".to_string(),
            name: "Aave V3".to_string(),
            value_usd: "1250000".to_string(),
            percentage: 50.0,
            limit_percent: 50.0,
            status: "WARNING".to_string(),
            recommendation: Some("At per-protocol ceiling; route new deposits to Compound V3.".to_string()),
        },
        ConcentrationItem {
            category: "protocol".to_string(),
            name: "Compound V3".to_string(),
            value_usd: "750000".to_string(),
            percentage: 30.0,
            limit_percent: 50.0,
            status: "OK".to_string(),
            recommendation: None,
        },
        ConcentrationItem {
            category: "token".to_string(),
            name: "USDC".to_string(),
            value_usd: "1500000".to_string(),
            percentage: 60.0,
            limit_percent: 70.0,
            status: "OK".to_string(),
            recommendation: None,
        },
        ConcentrationItem {
            category: "token".to_string(),
            name: "USDT".to_string(),
            value_usd: "1000000".to_string(),
            percentage: 40.0,
            limit_percent: 70.0,
            status: "OK".to_string(),
            recommendation: None,
        },
        ConcentrationItem {
            category: "chain".to_string(),
            name: "Ethereum".to_string(),
            value_usd: "1800000".to_string(),
            percentage: 72.0,
            limit_percent: 70.0,
            status: "EXCEEDED".to_string(),
            recommendation: Some("Ethereum exposure exceeds 70% ceiling; bridge a tranche to Arbitrum.".to_string()),
        },
    ]))
}

// ----------------------------------------------------------------------------
// Billing
// ----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SubscriptionUsage {
    pub api_calls: i64,
    pub api_limit: i64,
    pub transaction_volume: i64,
    pub volume_limit: i64,
    pub reset_date: String,
}

#[derive(Debug, Serialize)]
pub struct PaymentMethod {
    pub brand: String,
    pub last4: String,
}

#[derive(Debug, Serialize)]
pub struct Subscription {
    pub plan: String,
    pub status: String,
    pub amount: String,
    pub currency: String,
    pub interval: String,
    pub next_invoice_date: String,
    pub payment_method: Option<PaymentMethod>,
    pub billing_email: Option<String>,
    pub usage: SubscriptionUsage,
}

/// GET /v1/admin/billing/subscription
pub async fn billing_subscription(
    headers: HeaderMap,
    Extension(_tenant): Extension<TenantContext>,
) -> Result<Json<Subscription>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    Ok(Json(Subscription {
        plan: "growth".to_string(),
        status: "active".to_string(),
        amount: "499.00".to_string(),
        currency: "USD".to_string(),
        interval: "month".to_string(),
        next_invoice_date: (Utc::now() + Duration::days(12)).to_rfc3339(),
        payment_method: Some(PaymentMethod {
            brand: "visa".to_string(),
            last4: "4242".to_string(),
        }),
        billing_email: Some("billing@rampos.local".to_string()),
        usage: SubscriptionUsage {
            api_calls: 84_210,
            api_limit: 250_000,
            transaction_volume: 1_438_407_430,
            volume_limit: 5_000_000_000,
            reset_date: (Utc::now() + Duration::days(12)).to_rfc3339(),
        },
    }))
}

#[derive(Debug, Serialize)]
pub struct InvoiceItem {
    pub id: String,
    pub number: String,
    pub amount: String,
    pub currency: String,
    pub status: String,
    pub date: String,
    pub due_date: String,
    pub pdf_url: Option<String>,
}

/// GET /v1/admin/billing/invoices
pub async fn billing_invoices(
    headers: HeaderMap,
    Extension(_tenant): Extension<TenantContext>,
    Query(_q): Query<PageQuery>,
) -> Result<Json<Paginated<InvoiceItem>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    let mut data = Vec::new();
    for i in 0..4 {
        let issued = Utc::now() - Duration::days(30 * (i + 1));
        data.push(InvoiceItem {
            id: format!("in_{:06}", 1000 + i),
            number: format!("RAMP-2026-{:04}", 120 - i),
            amount: "499.00".to_string(),
            currency: "USD".to_string(),
            status: "paid".to_string(),
            date: issued.to_rfc3339(),
            due_date: (issued + Duration::days(14)).to_rfc3339(),
            pdf_url: Some(format!("/v1/admin/billing/invoices/in_{:06}/pdf", 1000 + i)),
        });
    }
    Ok(Json(Paginated::all(data)))
}

// ----------------------------------------------------------------------------
// Treasury transactions (also backs the Bridge page history via type=BRIDGE)
// ----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct TreasuryTxnItem {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub token: String,
    pub amount: String,
    pub amount_usd: String,
    pub from_chain: Option<String>,
    pub to_chain: Option<String>,
    pub protocol: Option<String>,
    pub tx_hash: String,
    pub status: String,
    pub initiated_by: String,
    pub created_at: String,
    pub confirmed_at: Option<String>,
}

fn treasury_txns() -> Vec<TreasuryTxnItem> {
    vec![
        TreasuryTxnItem {
            id: "ttx_b001".to_string(),
            kind: "BRIDGE".to_string(),
            token: "USDC".to_string(),
            amount: "250000".to_string(),
            amount_usd: "250000".to_string(),
            from_chain: Some("ethereum".to_string()),
            to_chain: Some("arbitrum".to_string()),
            protocol: None,
            tx_hash: "0x9f2c4ad1c0b8e3f5a6d7e8b9c0a1f2d3e4b5c6a7d8e9f0a1b2c3d4e5f60718293".to_string(),
            status: "CONFIRMED".to_string(),
            initiated_by: "treasury@rampos.local".to_string(),
            created_at: iso(95),
            confirmed_at: Some(iso(92)),
        },
        TreasuryTxnItem {
            id: "ttx_b002".to_string(),
            kind: "BRIDGE".to_string(),
            token: "USDT".to_string(),
            amount: "120000".to_string(),
            amount_usd: "120000".to_string(),
            from_chain: Some("arbitrum".to_string()),
            to_chain: Some("base".to_string()),
            protocol: None,
            tx_hash: "0x3b1e7c9a0d2f4685b7c8d9e0a1f2b3c4d5e6f70819a2b3c4d5e6f7081920a3b4c".to_string(),
            status: "PENDING".to_string(),
            initiated_by: "treasury@rampos.local".to_string(),
            created_at: iso(8),
            confirmed_at: None,
        },
        TreasuryTxnItem {
            id: "ttx_y001".to_string(),
            kind: "YIELD_DEPOSIT".to_string(),
            token: "USDC".to_string(),
            amount: "500000".to_string(),
            amount_usd: "500000".to_string(),
            from_chain: Some("ethereum".to_string()),
            to_chain: None,
            protocol: Some("aave-v3".to_string()),
            tx_hash: "0x7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f6071829304a5b6c".to_string(),
            status: "CONFIRMED".to_string(),
            initiated_by: "treasury@rampos.local".to_string(),
            created_at: iso(310),
            confirmed_at: Some(iso(308)),
        },
        TreasuryTxnItem {
            id: "ttx_d001".to_string(),
            kind: "DEPOSIT".to_string(),
            token: "USDC".to_string(),
            amount: "1000000".to_string(),
            amount_usd: "1000000".to_string(),
            from_chain: Some("ethereum".to_string()),
            to_chain: None,
            protocol: None,
            tx_hash: "0x1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5d6e7f809a".to_string(),
            status: "CONFIRMED".to_string(),
            initiated_by: "treasury@rampos.local".to_string(),
            created_at: iso(1440),
            confirmed_at: Some(iso(1438)),
        },
    ]
}

/// GET /v1/admin/treasury/transactions
pub async fn treasury_transactions(
    headers: HeaderMap,
    Extension(_tenant): Extension<TenantContext>,
    Query(q): Query<PageQuery>,
) -> Result<Json<Paginated<TreasuryTxnItem>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    let kind = q.kind.as_deref().filter(|s| !s.is_empty());
    let data: Vec<TreasuryTxnItem> = treasury_txns()
        .into_iter()
        .filter(|t| kind.map_or(true, |k| t.kind == k))
        .collect();
    Ok(Json(Paginated::all(data)))
}

// ----------------------------------------------------------------------------
// Generated reports
// ----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Serialize)]
pub struct ReportItem {
    pub id: String,
    pub tenant_id: String,
    pub report_type: String,
    pub date_range: DateRange,
    pub status: String,
    pub download_url: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// GET /v1/admin/reports
pub async fn list_reports(
    headers: HeaderMap,
    Extension(tenant): Extension<TenantContext>,
) -> Result<Json<Paginated<ReportItem>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    let tenant_id = tenant.tenant_id.0.clone();
    let day = |d: i64| (Utc::now() - Duration::days(d)).to_rfc3339();
    let data = vec![
        ReportItem {
            id: "rpt_0003".to_string(),
            tenant_id: tenant_id.clone(),
            report_type: "DAILY_TRANSACTIONS".to_string(),
            date_range: DateRange { start: day(1), end: day(0) },
            status: "COMPLETED".to_string(),
            download_url: Some("/v1/admin/reports/rpt_0003/download".to_string()),
            created_at: iso(30),
            completed_at: Some(iso(28)),
        },
        ReportItem {
            id: "rpt_0002".to_string(),
            tenant_id: tenant_id.clone(),
            report_type: "AML_SUMMARY".to_string(),
            date_range: DateRange { start: day(7), end: day(0) },
            status: "COMPLETED".to_string(),
            download_url: Some("/v1/admin/reports/rpt_0002/download".to_string()),
            created_at: iso(180),
            completed_at: Some(iso(176)),
        },
        ReportItem {
            id: "rpt_0001".to_string(),
            tenant_id,
            report_type: "USER_GROWTH".to_string(),
            date_range: DateRange { start: day(30), end: day(0) },
            status: "GENERATING".to_string(),
            download_url: None,
            created_at: iso(4),
            completed_at: None,
        },
    ];
    Ok(Json(Paginated::all(data)))
}

// ----------------------------------------------------------------------------
// Event-type catalog (camelCase) — backs the Events page
// ----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventCatalogItem {
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: String,
    pub schema: serde_json::Value,
    pub deprecated: bool,
    pub published_by: Vec<String>,
    pub subscribed_by: Vec<String>,
    pub last_published: Option<String>,
}

/// GET /v1/admin/events
pub async fn events_catalog(
    headers: HeaderMap,
    Extension(_tenant): Extension<TenantContext>,
) -> Result<Json<Vec<EventCatalogItem>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    let ev = |name: &str, cat: &str, desc: &str, pubs: &[&str], subs: &[&str], mins: i64| EventCatalogItem {
        name: name.to_string(),
        version: "1.0".to_string(),
        description: desc.to_string(),
        category: cat.to_string(),
        schema: serde_json::json!({ "type": "object", "required": ["intentId", "tenantId"] }),
        deprecated: false,
        published_by: pubs.iter().map(|s| s.to_string()).collect(),
        subscribed_by: subs.iter().map(|s| s.to_string()).collect(),
        last_published: Some(iso(mins)),
    };
    Ok(Json(vec![
        ev("intent.created", "Intents", "Emitted when a new payment intent is created.", &["ramp-api"], &["webhook-dispatcher", "ledger"], 3),
        ev("payin.completed", "Payments", "Emitted when a pay-in settles to the user balance.", &["ramp-core"], &["webhook-dispatcher", "notifications"], 7),
        ev("payout.completed", "Payments", "Emitted when a pay-out is confirmed on the bank rail.", &["ramp-core"], &["webhook-dispatcher", "reconciliation"], 12),
        ev("rfq.matched", "RFQ", "Emitted when an RFQ auction is matched to a winning LP bid.", &["ramp-core"], &["settlement", "webhook-dispatcher"], 21),
        ev("offramp.crypto_received", "Off-Ramp", "Emitted when crypto for an off-ramp intent is received on-chain.", &["ramp-core"], &["compliance", "settlement"], 34),
        ev("kyc.approved", "Compliance", "Emitted when a user's KYC tier is approved.", &["ramp-compliance"], &["webhook-dispatcher", "notifications"], 56),
    ]))
}

// ----------------------------------------------------------------------------
// Licensing fallback (camelCase envelopes + snake_case submission rows)
// ----------------------------------------------------------------------------

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FallbackLicenseRequirement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub license_type: String,
    pub regulatory_body: String,
    pub deadline: Option<String>,
    pub renewal_period_days: Option<i32>,
    pub required_documents: Vec<String>,
    pub is_mandatory: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn fallback_requirements() -> Vec<FallbackLicenseRequirement> {
    vec![
        FallbackLicenseRequirement {
            id: "lic_sbv_msb_registration".to_string(),
            name: "SBV money-services registration".to_string(),
            description: "Maintain Vietnam State Bank registration for fiat ramp operations and partner-rail oversight.".to_string(),
            license_type: "VASP".to_string(),
            regulatory_body: "State Bank of Vietnam".to_string(),
            deadline: Some((Utc::now() + Duration::days(21)).to_rfc3339()),
            renewal_period_days: Some(365),
            required_documents: vec!["Corporate registration".to_string(), "AML policy".to_string(), "Bank partnership letter".to_string()],
            is_mandatory: true,
            created_at: iso(1_440),
            updated_at: iso(120),
        },
        FallbackLicenseRequirement {
            id: "lic_aml_program".to_string(),
            name: "AML / CFT program evidence".to_string(),
            description: "Document the transaction-monitoring, sanctions-screening, STR/SAR, and travel-rule control framework.".to_string(),
            license_type: "COMPLIANCE".to_string(),
            regulatory_body: "Ministry of Public Security".to_string(),
            deadline: Some((Utc::now() + Duration::days(8)).to_rfc3339()),
            renewal_period_days: Some(180),
            required_documents: vec!["AML policy".to_string(), "Risk assessment".to_string(), "Compliance officer appointment".to_string()],
            is_mandatory: true,
            created_at: iso(1_440),
            updated_at: iso(90),
        },
        FallbackLicenseRequirement {
            id: "lic_cybersecurity_filing".to_string(),
            name: "Cybersecurity and data-localization filing".to_string(),
            description: "Evidence cybersecurity controls for Vietnamese user data, audit logging, and incident response.".to_string(),
            license_type: "DATA_PROTECTION".to_string(),
            regulatory_body: "MIC Vietnam".to_string(),
            deadline: Some((Utc::now() + Duration::days(45)).to_rfc3339()),
            renewal_period_days: Some(365),
            required_documents: vec!["Security policy".to_string(), "Data-processing map".to_string()],
            is_mandatory: true,
            created_at: iso(1_440),
            updated_at: iso(60),
        },
        FallbackLicenseRequirement {
            id: "lic_liquidity_policy".to_string(),
            name: "Liquidity-provider policy attestation".to_string(),
            description: "Keep LP onboarding, risk limits, and settlement procedures auditable for exchange/ramp operations.".to_string(),
            license_type: "EXCHANGE".to_string(),
            regulatory_body: "Internal Compliance".to_string(),
            deadline: None,
            renewal_period_days: Some(90),
            required_documents: vec!["LP policy".to_string(), "Counterparty risk framework".to_string()],
            is_mandatory: false,
            created_at: iso(1_440),
            updated_at: iso(30),
        },
    ]
}

/// GET /v1/admin/licensing/requirements (fallback when repository is not wired)
pub async fn licensing_requirements_fallback(
    headers: HeaderMap,
    Extension(_tenant): Extension<TenantContext>,
) -> Result<Json<serde_json::Value>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    let data = fallback_requirements();
    Ok(Json(serde_json::json!({
        "data": data,
        "total": data.len(),
        "limit": 100,
        "offset": 0,
    })))
}

/// GET /v1/admin/licensing/status (fallback when repository is not wired)
pub async fn licensing_status_fallback(
    headers: HeaderMap,
    Extension(tenant): Extension<TenantContext>,
) -> Result<Json<serde_json::Value>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    let reqs = fallback_requirements();
    let licenses: Vec<_> = reqs
        .iter()
        .enumerate()
        .map(|(idx, req)| {
            let status = match idx {
                0 => "APPROVED",
                1 => "UNDER_REVIEW",
                2 => "PENDING",
                _ => "PENDING",
            };
            serde_json::json!({
                "requirementId": req.id,
                "requirementName": req.name,
                "licenseType": req.license_type,
                "status": status,
                "licenseNumber": if status == "APPROVED" { Some("VN-RAMP-2026-001") } else { None },
                "issueDate": if status == "APPROVED" { Some(iso(60 * 24 * 30)) } else { None },
                "expiryDate": if status == "APPROVED" { Some((Utc::now() + Duration::days(305)).to_rfc3339()) } else { None },
                "lastSubmissionId": format!("lic_sub_{:03}", idx + 1),
                "notes": "Fallback catalog: licensing repository not provisioned in this environment.",
                "updatedAt": iso(45 + idx as i64 * 15),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({
        "tenantId": tenant.tenant_id.0,
        "totalRequirements": reqs.len(),
        "approvedCount": 1,
        "pendingCount": 3,
        "expiredCount": 0,
        "licenses": licenses,
    })))
}

/// GET /v1/admin/licensing/deadlines (fallback when repository is not wired)
pub async fn licensing_deadlines_fallback(
    headers: HeaderMap,
    Extension(_tenant): Extension<TenantContext>,
) -> Result<Json<serde_json::Value>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    let reqs = fallback_requirements();
    let upcoming: Vec<_> = reqs
        .into_iter()
        .filter_map(|req| {
            let deadline = req.deadline?;
            let days_remaining = ((chrono::DateTime::parse_from_rfc3339(&deadline).ok()?.with_timezone(&Utc) - Utc::now()).num_days()).max(0);
            Some(serde_json::json!({
                "requirementId": req.id,
                "requirementName": req.name,
                "licenseType": req.license_type,
                "deadline": deadline,
                "daysRemaining": days_remaining,
                "status": "PENDING",
                "isOverdue": false,
            }))
        })
        .collect();
    Ok(Json(serde_json::json!({ "upcoming": upcoming, "overdue": [] })))
}

/// GET /v1/admin/licensing/submissions (fallback when repository is not wired)
pub async fn licensing_submissions_fallback(
    headers: HeaderMap,
    Extension(_tenant): Extension<TenantContext>,
) -> Result<Json<serde_json::Value>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    let data = vec![
        serde_json::json!({
            "id": "lic_sub_001",
            "requirement_id": "lic_sbv_msb_registration",
            "requirement_name": "SBV money-services registration",
            "submitted_by": "compliance@rampos.local",
            "document_url": "/docs/licensing/sbv-registration.pdf",
            "document_name": "SBV registration packet.pdf",
            "status": "APPROVED",
            "reviewer_notes": "Approved for current demo environment.",
            "submitted_at": iso(180),
            "reviewed_at": iso(120),
        }),
        serde_json::json!({
            "id": "lic_sub_002",
            "requirement_id": "lic_aml_program",
            "requirement_name": "AML / CFT program evidence",
            "submitted_by": "compliance@rampos.local",
            "document_url": "/docs/licensing/aml-program.pdf",
            "document_name": "AML program evidence.pdf",
            "status": "PENDING_REVIEW",
            "reviewer_notes": null,
            "submitted_at": iso(65),
            "reviewed_at": null,
        }),
    ];
    Ok(Json(serde_json::json!({
        "data": data,
        "total": data.len(),
        "page": 1,
        "per_page": 50,
        "total_pages": 1,
    })))
}

// ----------------------------------------------------------------------------
// Tier limits catalog (camelCase) — backs the Limits page
// ----------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TierLimitItem {
    pub kyc_tier: u8,
    pub tier_name: String,
    pub daily_payin_limit_vnd: i64,
    pub daily_payout_limit_vnd: i64,
    pub monthly_payin_limit_vnd: i64,
    pub monthly_payout_limit_vnd: i64,
    pub single_transaction_max_vnd: i64,
}

/// GET /v1/admin/limits
pub async fn list_tier_limits(
    headers: HeaderMap,
    Extension(_tenant): Extension<TenantContext>,
) -> Result<Json<Vec<TierLimitItem>>, ApiError> {
    super::tier::check_admin_key(&headers)?;
    Ok(Json(vec![
        TierLimitItem {
            kyc_tier: 0,
            tier_name: "Tier 0 — Basic".to_string(),
            daily_payin_limit_vnd: 10_000_000,
            daily_payout_limit_vnd: 5_000_000,
            monthly_payin_limit_vnd: 50_000_000,
            monthly_payout_limit_vnd: 25_000_000,
            single_transaction_max_vnd: 5_000_000,
        },
        TierLimitItem {
            kyc_tier: 1,
            tier_name: "Tier 1 — Phone".to_string(),
            daily_payin_limit_vnd: 50_000_000,
            daily_payout_limit_vnd: 25_000_000,
            monthly_payin_limit_vnd: 500_000_000,
            monthly_payout_limit_vnd: 250_000_000,
            single_transaction_max_vnd: 20_000_000,
        },
        TierLimitItem {
            kyc_tier: 2,
            tier_name: "Tier 2 — ID Verified".to_string(),
            daily_payin_limit_vnd: 200_000_000,
            daily_payout_limit_vnd: 100_000_000,
            monthly_payin_limit_vnd: 2_000_000_000,
            monthly_payout_limit_vnd: 1_000_000_000,
            single_transaction_max_vnd: 100_000_000,
        },
        TierLimitItem {
            kyc_tier: 3,
            tier_name: "Tier 3 — Full KYC".to_string(),
            daily_payin_limit_vnd: 1_000_000_000,
            daily_payout_limit_vnd: 500_000_000,
            monthly_payin_limit_vnd: 10_000_000_000,
            monthly_payout_limit_vnd: 5_000_000_000,
            single_transaction_max_vnd: 500_000_000,
        },
    ]))
}
