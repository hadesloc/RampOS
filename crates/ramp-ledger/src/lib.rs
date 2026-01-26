//! RampOS Ledger - Double-entry accounting system
//! Re-exports from ramp-common for convenience

pub use ramp_common::ledger::*;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use ramp_common::types::{TenantId, IntentId};

    #[test]
    fn test_double_entry_posting() {
        let tenant_id = TenantId::new("tenant1");
        let intent_id = IntentId::new_payin();

        let tx = LedgerTransactionBuilder::new(
            tenant_id,
            intent_id,
            "Test Posting"
        )
        .debit(AccountType::AssetBank, dec!(100), LedgerCurrency::VND)
        .credit(AccountType::LiabilityUserVnd, dec!(100), LedgerCurrency::VND)
        .build();

        assert!(tx.is_ok());
        let tx = tx.unwrap();
        assert!(tx.is_balanced());
        assert_eq!(tx.entries.len(), 2);
    }

    #[test]
    fn test_balance_queries() {
        // Since ramp-ledger currently only re-exports types and doesn't contain the repository/service logic
        // (which is in ramp-core), we can only test the data structures here.
        // Balance queries are tested in ramp-core::service::ledger::tests.

        // However, we can test that we can construct a BalanceRow-like structure or similar if it was exported here.
        // Let's just verify AccountType display

        let account = AccountType::LiabilityUserVnd;
        assert_eq!(account.to_string(), "Liability:UserVND");
    }
}
