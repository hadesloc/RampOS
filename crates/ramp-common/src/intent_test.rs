#[cfg(test)]
mod tests {
    use crate::intent::*;

    // ============================================================================
    // PayinState Tests
    // ============================================================================

    #[test]
    fn test_payin_state_happy_path() {
        // Created -> InstructionIssued -> FundsPending -> FundsConfirmed -> VndCredited -> Completed
        let mut state = PayinState::Created;

        assert!(state.can_transition_to(PayinState::InstructionIssued));
        state = PayinState::InstructionIssued;

        assert!(state.can_transition_to(PayinState::FundsPending));
        state = PayinState::FundsPending;

        assert!(state.can_transition_to(PayinState::FundsConfirmed));
        state = PayinState::FundsConfirmed;

        assert!(state.can_transition_to(PayinState::VndCredited));
        state = PayinState::VndCredited;

        assert!(state.can_transition_to(PayinState::Completed));
        state = PayinState::Completed;

        assert!(state.is_terminal());
        assert!(!state.is_error());
        assert!(state.allowed_transitions().is_empty());
    }

    #[test]
    fn test_payin_state_error_paths() {
        // Expired paths
        assert!(PayinState::InstructionIssued.can_transition_to(PayinState::Expired));
        assert!(PayinState::FundsPending.can_transition_to(PayinState::Expired));
        assert!(PayinState::Expired.is_terminal());
        assert!(PayinState::Expired.is_error());

        // Cancelled paths
        assert!(PayinState::Created.can_transition_to(PayinState::Cancelled));
        assert!(PayinState::InstructionIssued.can_transition_to(PayinState::Cancelled));
        assert!(PayinState::ManualReview.can_transition_to(PayinState::Cancelled));
        assert!(PayinState::Cancelled.is_terminal());
        assert!(!PayinState::Cancelled.is_error());

        // MismatchedAmount path
        assert!(PayinState::FundsPending.can_transition_to(PayinState::MismatchedAmount));
        assert!(PayinState::MismatchedAmount.can_transition_to(PayinState::ManualReview));
        assert!(PayinState::MismatchedAmount.can_transition_to(PayinState::VndCredited));
        assert!(PayinState::MismatchedAmount.is_error());

        // SuspectedFraud path
        assert!(PayinState::FundsConfirmed.can_transition_to(PayinState::SuspectedFraud));
        assert!(PayinState::ManualReview.can_transition_to(PayinState::SuspectedFraud));
        assert!(PayinState::SuspectedFraud.is_terminal());
        assert!(PayinState::SuspectedFraud.is_error());

        // ManualReview path
        assert!(PayinState::FundsConfirmed.can_transition_to(PayinState::ManualReview));
        assert!(PayinState::MismatchedAmount.can_transition_to(PayinState::ManualReview));
        assert!(PayinState::ManualReview.can_transition_to(PayinState::VndCredited));
    }

    #[test]
    fn test_payin_state_invalid_transitions() {
        assert!(!PayinState::Created.can_transition_to(PayinState::Completed));
        assert!(!PayinState::Completed.can_transition_to(PayinState::Created));
        assert!(!PayinState::Expired.can_transition_to(PayinState::Created));
    }

    // ============================================================================
    // PayoutState Tests
    // ============================================================================

    #[test]
    fn test_payout_state_happy_path() {
        // Created -> PolicyApproved -> Submitted -> Confirmed -> Completed
        let mut state = PayoutState::Created;

        assert!(state.can_transition_to(PayoutState::PolicyApproved));
        state = PayoutState::PolicyApproved;

        assert!(state.can_transition_to(PayoutState::Submitted));
        state = PayoutState::Submitted;

        assert!(state.can_transition_to(PayoutState::Confirmed));
        state = PayoutState::Confirmed;

        assert!(state.can_transition_to(PayoutState::Completed));
        state = PayoutState::Completed;

        assert!(state.is_terminal());
        assert!(!state.is_error());
    }

    #[test]
    fn test_payout_state_error_paths() {
        // RejectedByPolicy
        assert!(PayoutState::Created.can_transition_to(PayoutState::RejectedByPolicy));
        assert!(PayoutState::ManualReview.can_transition_to(PayoutState::RejectedByPolicy));
        assert!(PayoutState::RejectedByPolicy.is_terminal());
        assert!(PayoutState::RejectedByPolicy.is_error());

        // BankRejected
        assert!(PayoutState::Submitted.can_transition_to(PayoutState::BankRejected));
        assert!(PayoutState::BankRejected.is_terminal());
        assert!(PayoutState::BankRejected.is_error());

        // Timeout
        assert!(PayoutState::Submitted.can_transition_to(PayoutState::Timeout));
        assert!(PayoutState::Timeout.can_transition_to(PayoutState::Submitted)); // Retry
        assert!(PayoutState::Timeout.can_transition_to(PayoutState::ManualReview));
        assert!(PayoutState::Timeout.is_error());
        assert!(!PayoutState::Timeout.is_terminal());

        // ManualReview
        assert!(PayoutState::Created.can_transition_to(PayoutState::ManualReview));
        assert!(PayoutState::Timeout.can_transition_to(PayoutState::ManualReview));
        assert!(PayoutState::ManualReview.can_transition_to(PayoutState::PolicyApproved));
        assert!(PayoutState::ManualReview.can_transition_to(PayoutState::Cancelled));

        // Cancelled
        assert!(PayoutState::PolicyApproved.can_transition_to(PayoutState::Cancelled));
        assert!(PayoutState::Cancelled.is_terminal());
        assert!(!PayoutState::Cancelled.is_error());
    }

    #[test]
    fn test_payout_state_invalid_transitions() {
        assert!(!PayoutState::Created.can_transition_to(PayoutState::Completed));
        assert!(!PayoutState::Completed.can_transition_to(PayoutState::Created));
    }

    // ============================================================================
    // TradeState Tests
    // ============================================================================

    #[test]
    fn test_trade_state_happy_path() {
        // Recorded -> PostTradeChecked -> SettledLedger -> Completed
        let mut state = TradeState::Recorded;

        assert!(state.can_transition_to(TradeState::PostTradeChecked));
        state = TradeState::PostTradeChecked;

        assert!(state.can_transition_to(TradeState::SettledLedger));
        state = TradeState::SettledLedger;

        assert!(state.can_transition_to(TradeState::Completed));
        state = TradeState::Completed;

        assert!(state.is_terminal());
    }

    #[test]
    fn test_trade_state_error_paths() {
        // ComplianceHold
        assert!(TradeState::Recorded.can_transition_to(TradeState::ComplianceHold));
        assert!(TradeState::ComplianceHold.can_transition_to(TradeState::ManualReview));
        assert!(TradeState::ComplianceHold.can_transition_to(TradeState::Rejected));
        assert!(TradeState::ComplianceHold.is_error());

        // ManualReview
        assert!(TradeState::PostTradeChecked.can_transition_to(TradeState::ManualReview));
        assert!(TradeState::ManualReview.can_transition_to(TradeState::PostTradeChecked));
        assert!(TradeState::ManualReview.can_transition_to(TradeState::Rejected));
        assert!(!TradeState::ManualReview.is_error());

        // Rejected
        assert!(TradeState::Rejected.is_terminal());
        assert!(TradeState::Rejected.is_error());
    }

    #[test]
    fn test_trade_state_invalid_transitions() {
        assert!(!TradeState::Recorded.can_transition_to(TradeState::Completed));
        assert!(!TradeState::Completed.can_transition_to(TradeState::Recorded));
    }

    // ============================================================================
    // DepositState Tests
    // ============================================================================

    #[test]
    fn test_deposit_state_happy_path() {
        // Detected -> Confirming -> Confirmed -> KytChecked -> Credited -> Completed
        let mut state = DepositState::Detected;

        assert!(state.can_transition_to(DepositState::Confirming));
        state = DepositState::Confirming;

        assert!(state.can_transition_to(DepositState::Confirmed));
        state = DepositState::Confirmed;

        assert!(state.can_transition_to(DepositState::KytChecked));
        state = DepositState::KytChecked;

        assert!(state.can_transition_to(DepositState::Credited));
        state = DepositState::Credited;

        assert!(state.can_transition_to(DepositState::Completed));
        state = DepositState::Completed;

        assert!(state.is_terminal());
    }

    #[test]
    fn test_deposit_state_error_paths() {
        // KytFlagged
        assert!(DepositState::Confirmed.can_transition_to(DepositState::KytFlagged));
        assert!(DepositState::KytFlagged.can_transition_to(DepositState::ManualReview));
        assert!(DepositState::KytFlagged.can_transition_to(DepositState::Rejected));
        assert!(DepositState::KytFlagged.is_error());

        // ManualReview
        assert!(DepositState::ManualReview.can_transition_to(DepositState::Credited));
        assert!(DepositState::ManualReview.can_transition_to(DepositState::Rejected));
        assert!(!DepositState::ManualReview.is_error());

        // Rejected
        assert!(DepositState::Rejected.is_terminal());
        assert!(DepositState::Rejected.is_error());
    }

    #[test]
    fn test_deposit_state_invalid_transitions() {
        assert!(!DepositState::Detected.can_transition_to(DepositState::Completed));
        assert!(!DepositState::Completed.can_transition_to(DepositState::Detected));
    }

    // ============================================================================
    // WithdrawState Tests
    // ============================================================================

    #[test]
    fn test_withdraw_state_happy_path() {
        // Created -> PolicyApproved -> KytChecked -> Signed -> Broadcasted -> Confirming -> Confirmed -> Completed
        let mut state = WithdrawState::Created;

        assert!(state.can_transition_to(WithdrawState::PolicyApproved));
        state = WithdrawState::PolicyApproved;

        assert!(state.can_transition_to(WithdrawState::KytChecked));
        state = WithdrawState::KytChecked;

        assert!(state.can_transition_to(WithdrawState::Signed));
        state = WithdrawState::Signed;

        assert!(state.can_transition_to(WithdrawState::Broadcasted));
        state = WithdrawState::Broadcasted;

        assert!(state.can_transition_to(WithdrawState::Confirming));
        state = WithdrawState::Confirming;

        assert!(state.can_transition_to(WithdrawState::Confirmed));
        state = WithdrawState::Confirmed;

        assert!(state.can_transition_to(WithdrawState::Completed));
        state = WithdrawState::Completed;

        assert!(state.is_terminal());
    }

    #[test]
    fn test_withdraw_state_error_paths() {
        // RejectedByPolicy
        assert!(WithdrawState::Created.can_transition_to(WithdrawState::RejectedByPolicy));
        assert!(WithdrawState::RejectedByPolicy.is_terminal());
        assert!(WithdrawState::RejectedByPolicy.is_error());

        // KytFlagged
        assert!(WithdrawState::PolicyApproved.can_transition_to(WithdrawState::KytFlagged));
        assert!(WithdrawState::KytFlagged.can_transition_to(WithdrawState::ManualReview));
        assert!(WithdrawState::KytFlagged.can_transition_to(WithdrawState::Cancelled));
        assert!(WithdrawState::KytFlagged.is_error());

        // BroadcastFailed
        assert!(WithdrawState::Signed.can_transition_to(WithdrawState::BroadcastFailed));
        assert!(WithdrawState::BroadcastFailed.can_transition_to(WithdrawState::Signed)); // Retry
        assert!(WithdrawState::BroadcastFailed.can_transition_to(WithdrawState::ManualReview));
        assert!(WithdrawState::BroadcastFailed.is_error());

        // ManualReview
        assert!(WithdrawState::Confirming.can_transition_to(WithdrawState::ManualReview));
        assert!(WithdrawState::ManualReview.can_transition_to(WithdrawState::PolicyApproved));
        assert!(WithdrawState::ManualReview.can_transition_to(WithdrawState::Cancelled));
        assert!(!WithdrawState::ManualReview.is_error());
    }

    #[test]
    fn test_withdraw_state_invalid_transitions() {
        assert!(!WithdrawState::Created.can_transition_to(WithdrawState::Completed));
        assert!(!WithdrawState::Completed.can_transition_to(WithdrawState::Created));
    }

    // ============================================================================
    // Unified IntentState Tests
    // ============================================================================

    #[test]
    fn test_unified_intent_state() {
        let payin = IntentState::Payin(PayinState::Completed);
        assert!(payin.is_terminal());
        assert!(!payin.is_error());
        assert_eq!(payin.as_string(), "COMPLETED");

        let payout = IntentState::Payout(PayoutState::Created);
        assert!(!payout.is_terminal());
        assert!(!payout.is_error());
        assert_eq!(payout.as_string(), "PAYOUT_CREATED");

        let trade = IntentState::Trade(TradeState::Completed);
        assert!(trade.is_terminal());
        assert!(!trade.is_error());

        let deposit = IntentState::Deposit(DepositState::Completed);
        assert!(deposit.is_terminal());
        assert!(!deposit.is_error());

        let withdraw = IntentState::Withdraw(WithdrawState::Completed);
        assert!(withdraw.is_terminal());
        assert!(!withdraw.is_error());

        // Test error delegation
        let error_payin = IntentState::Payin(PayinState::Expired);
        assert!(error_payin.is_error());

        let error_trade = IntentState::Trade(TradeState::Rejected);
        assert!(error_trade.is_error());
    }
}
