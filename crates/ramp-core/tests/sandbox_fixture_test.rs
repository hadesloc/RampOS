use chrono::{TimeZone, Utc};
use ramp_common::types::{TenantId, UserId};
use ramp_core::test_utils::{
    sandbox_failure_trigger_catalog, sandbox_offramp_fixture, sandbox_payin_fixture,
    sandbox_rfq_fixture, SandboxFailureTriggerCode,
};
use rust_decimal::Decimal;

#[test]
fn sandbox_fixture_builders_are_deterministic_for_known_inputs() {
    let tenant_id = TenantId::new("tenant_sandbox");
    let user_id = UserId::new("user_sandbox");

    let payin = sandbox_payin_fixture(&tenant_id, &user_id);
    let offramp = sandbox_offramp_fixture(&tenant_id, &user_id);
    let rfq = sandbox_rfq_fixture(&tenant_id, &user_id, &offramp.intent.id);

    assert_eq!(payin.intent.id, "sandbox_payin_baseline");
    assert_eq!(payin.intent.tenant_id, tenant_id.0);
    assert_eq!(payin.intent.user_id, user_id.0);
    assert_eq!(payin.intent.amount, Decimal::from(125_000));
    assert_eq!(payin.intent.state, "FUNDS_PENDING");
    assert_eq!(payin.intent.created_at, fixed_timestamp());

    assert_eq!(offramp.intent.id, "sandbox_offramp_baseline");
    assert_eq!(offramp.intent.tenant_id, tenant_id.0);
    assert_eq!(offramp.intent.user_id, user_id.0);
    assert_eq!(offramp.intent.crypto_asset, "USDT");
    assert_eq!(offramp.intent.state, "QUOTE_LOCKED");
    assert_eq!(
        offramp.intent.quote_expires_at,
        fixed_timestamp() + chrono::Duration::minutes(15)
    );

    assert_eq!(rfq.request.id, "sandbox_rfq_baseline");
    assert_eq!(rfq.request.tenant_id, tenant_id.0);
    assert_eq!(rfq.request.user_id, user_id.0);
    assert_eq!(rfq.request.direction, "OFFRAMP");
    assert_eq!(
        rfq.request.offramp_id.as_deref(),
        Some("sandbox_offramp_baseline")
    );
    assert_eq!(rfq.request.state, "OPEN");
    assert_eq!(rfq.request.created_at, fixed_timestamp());
}

#[test]
fn sandbox_failure_trigger_catalog_covers_named_drills() {
    let catalog = sandbox_failure_trigger_catalog();
    let codes: Vec<_> = catalog.iter().map(|trigger| trigger.code).collect();

    assert!(codes.contains(&SandboxFailureTriggerCode::BankTimeout));
    assert!(codes.contains(&SandboxFailureTriggerCode::LpNoFill));

    let bank_timeout = catalog
        .iter()
        .find(|trigger| trigger.code == SandboxFailureTriggerCode::BankTimeout)
        .unwrap();
    assert_eq!(bank_timeout.code.as_str(), "BANK_TIMEOUT");
    assert_eq!(bank_timeout.applies_to, "PAYIN");

    let lp_no_fill = catalog
        .iter()
        .find(|trigger| trigger.code == SandboxFailureTriggerCode::LpNoFill)
        .unwrap();
    assert_eq!(lp_no_fill.code.as_str(), "LP_NO_FILL");
    assert_eq!(lp_no_fill.applies_to, "RFQ");
}

#[test]
fn sandbox_rfq_fixture_exposes_expected_failure_triggers() {
    let tenant_id = TenantId::new("tenant_sandbox");
    let user_id = UserId::new("user_sandbox");
    let offramp = sandbox_offramp_fixture(&tenant_id, &user_id);
    let rfq = sandbox_rfq_fixture(&tenant_id, &user_id, &offramp.intent.id);

    assert_eq!(
        rfq.failure_trigger_codes,
        vec![
            SandboxFailureTriggerCode::LpNoFill,
            SandboxFailureTriggerCode::SettlementDelay,
        ]
    );
}

fn fixed_timestamp() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 8, 9, 0, 0).single().unwrap()
}
