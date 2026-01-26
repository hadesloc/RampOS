use crate::ledger::*;
use crate::types::*;
use rust_decimal::dec;

#[test]
fn test_ledger_transaction_builder_balanced() {
    let tenant_id = TenantId::new("t1");
    let intent_id = IntentId::new_payin();

    let result = LedgerTransactionBuilder::new(tenant_id, intent_id, "Test")
        .debit(AccountType::AssetBank, dec!(100), LedgerCurrency::VND)
        .credit(
            AccountType::LiabilityUserVnd,
            dec!(100),
            LedgerCurrency::VND,
        )
        .build();

    assert!(result.is_ok());
    let tx = result.unwrap();
    assert!(tx.is_balanced());
    assert_eq!(tx.entries.len(), 2);
    assert_eq!(tx.total_amount(), dec!(100));
}

#[test]
fn test_ledger_transaction_builder_imbalanced() {
    let tenant_id = TenantId::new("t1");
    let intent_id = IntentId::new_payin();

    let result = LedgerTransactionBuilder::new(tenant_id, intent_id, "Test")
        .debit(AccountType::AssetBank, dec!(100), LedgerCurrency::VND)
        .credit(AccountType::LiabilityUserVnd, dec!(90), LedgerCurrency::VND)
        .build();

    assert!(result.is_err());
    match result {
        Err(LedgerError::Imbalanced { debit, credit }) => {
            assert_eq!(debit, dec!(100));
            assert_eq!(credit, dec!(90));
        }
        _ => panic!("Wrong error type"),
    }
}

#[test]
fn test_complex_transaction() {
    let tenant_id = TenantId::new("t1");
    let intent_id = IntentId::new_trade();
    let user_id = UserId::new("u1");

    // Buy crypto: User pays VND, gets Crypto
    // 1. User Liability VND (Debit) -> Bank Asset (Credit) [User paying us]
    // 2. Crypto Asset (Debit) -> User Liability Crypto (Credit) [We giving user crypto]
    // Wait, patterns::trade_crypto_vnd implementation:
    // Debit UserLiabilityVnd, Credit AssetBank
    // Debit AssetCrypto, Credit UserLiabilityCrypto

    let result = patterns::trade_crypto_vnd(
        tenant_id,
        user_id,
        intent_id,
        dec!(1000000), // 1M VND
        dec!(0.001),   // 0.001 BTC
        LedgerCurrency::BTC,
        true, // is_buy
    );

    assert!(result.is_ok());
    let tx = result.unwrap();
    assert!(tx.is_balanced());
    assert_eq!(tx.entries.len(), 4);

    // Verify entries
    let debits: Vec<_> = tx
        .entries
        .iter()
        .filter(|e| e.direction == EntryDirection::Debit)
        .collect();
    let credits: Vec<_> = tx
        .entries
        .iter()
        .filter(|e| e.direction == EntryDirection::Credit)
        .collect();

    assert_eq!(debits.len(), 2);
    assert_eq!(credits.len(), 2);
}

#[test]
fn test_ledger_patterns_payin() {
    let tenant_id = TenantId::new("t1");
    let intent_id = IntentId::new_payin();
    let user_id = UserId::new("u1");

    let result = patterns::payin_vnd_confirmed(tenant_id, user_id, intent_id, dec!(50000));

    assert!(result.is_ok());
    let tx = result.unwrap();
    assert!(tx.is_balanced());

    // Debit AssetBank, Credit LiabilityUserVnd
    let debit = tx
        .entries
        .iter()
        .find(|e| e.direction == EntryDirection::Debit)
        .unwrap();
    assert_eq!(debit.account_type, AccountType::AssetBank);
    assert_eq!(debit.amount, dec!(50000));

    let credit = tx
        .entries
        .iter()
        .find(|e| e.direction == EntryDirection::Credit)
        .unwrap();
    assert_eq!(credit.account_type, AccountType::LiabilityUserVnd);
    assert_eq!(credit.amount, dec!(50000));
}

#[test]
fn test_ledger_patterns_payout() {
    let tenant_id = TenantId::new("t1");
    let intent_id = IntentId::new_payout();
    let user_id = UserId::new("u1");

    // Initiated: Debit User Liability, Credit Clearing Bank Pending
    let result =
        patterns::payout_vnd_initiated(tenant_id.clone(), user_id, intent_id.clone(), dec!(50000));

    assert!(result.is_ok());
    let tx = result.unwrap();
    assert!(tx.is_balanced());

    let debit = tx
        .entries
        .iter()
        .find(|e| e.direction == EntryDirection::Debit)
        .unwrap();
    assert_eq!(debit.account_type, AccountType::LiabilityUserVnd);

    let credit = tx
        .entries
        .iter()
        .find(|e| e.direction == EntryDirection::Credit)
        .unwrap();
    assert_eq!(credit.account_type, AccountType::ClearingBankPending);

    // Confirmed: Debit Clearing Bank Pending, Credit Asset Bank
    let result = patterns::payout_vnd_confirmed(tenant_id, intent_id, dec!(50000));

    assert!(result.is_ok());
    let tx = result.unwrap();
    assert!(tx.is_balanced());

    let debit = tx
        .entries
        .iter()
        .find(|e| e.direction == EntryDirection::Debit)
        .unwrap();
    assert_eq!(debit.account_type, AccountType::ClearingBankPending);

    let credit = tx
        .entries
        .iter()
        .find(|e| e.direction == EntryDirection::Credit)
        .unwrap();
    assert_eq!(credit.account_type, AccountType::AssetBank);
}
