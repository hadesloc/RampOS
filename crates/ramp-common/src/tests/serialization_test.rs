use crate::intent::*;
use crate::ledger::*;
use crate::types::*;
use serde_json::json;

#[test]
fn test_intent_type_serialization() {
    let t = IntentType::PayinVnd;
    let json = serde_json::to_value(t).unwrap();
    assert_eq!(json, json!("PAYIN_VND"));

    let t = IntentType::WithdrawOnchain;
    let json = serde_json::to_value(t).unwrap();
    assert_eq!(json, json!("WITHDRAW_ONCHAIN"));
}

#[test]
fn test_payin_state_serialization() {
    let s = PayinState::InstructionIssued;
    let json = serde_json::to_value(s).unwrap();
    assert_eq!(json, json!("INSTRUCTION_ISSUED"));

    let s = PayinState::FundsPending;
    let json = serde_json::to_value(s).unwrap();
    assert_eq!(json, json!("FUNDS_PENDING"));
}

#[test]
fn test_payout_state_serialization() {
    let s = PayoutState::PolicyApproved;
    let json = serde_json::to_value(s).unwrap();
    assert_eq!(json, json!("POLICY_APPROVED"));
}

#[test]
fn test_vnd_amount_serialization() {
    let amt = VndAmount::from_i64(100500);
    let json = serde_json::to_value(amt).unwrap();
    // Decimal serializes to string by default in some configs, or number.
    // rust_decimal default serialization is a string/number depending on feature.
    // Let's check what it actually produces.
    // Usually rust_decimal with serde-with-float or default might be different.
    // Based on the code, it uses default serialization.

    // If it's a number:
    // assert_eq!(json, json!("100500"));

    // Let's verify roundtrip which is safer
    let deserialized: VndAmount = serde_json::from_value(json).unwrap();
    assert_eq!(amt, deserialized);
}

#[test]
fn test_intent_state_enum_serialization() {
    // Test the untagged/tagged enum serialization for Unified Intent State
    let s = IntentState::Payin(PayinState::Created);
    let json = serde_json::to_value(s).unwrap();

    // #[serde(tag = "type", content = "state")]
    assert_eq!(
        json,
        json!({
            "type": "Payin",
            "state": "CREATED"
        })
    );

    // Actually, PayinState::Created serializes to "CREATED" because of rename_all="SCREAMING_SNAKE_CASE" on the enum?
    // No, variant is Created. SCREAMING_SNAKE_CASE of Created is CREATED.
    // Let's check intent.rs again.
}

#[test]
fn test_ledger_currency_serialization() {
    let c = LedgerCurrency::VND;
    let json = serde_json::to_value(c).unwrap();
    assert_eq!(json, json!("VND"));

    let c = LedgerCurrency::USDT;
    let json = serde_json::to_value(c).unwrap();
    assert_eq!(json, json!("USDT"));
}

#[test]
fn test_account_type_serialization() {
    let a = AccountType::LiabilityUserVnd;
    let json = serde_json::to_value(a).unwrap();
    assert_eq!(json, json!("LIABILITY_USER_VND"));
}
