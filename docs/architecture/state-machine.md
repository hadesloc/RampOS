# RampOS State Machine Documentation

## Overview

RampOS uses explicit state machines to track the lifecycle of every intent (transaction). Each intent type has its own state machine with defined states, transitions, and terminal conditions.

This design ensures:
- **Auditability**: Every state change is recorded
- **Reliability**: Invalid transitions are rejected at compile time
- **Visibility**: Current state is always known and queryable
- **Recovery**: Failed operations can be retried from known states

## Intent Types

RampOS supports five primary intent types:

| Intent Type | Prefix | Description |
|-------------|--------|-------------|
| `PayinVnd` | `pi_` | User deposits VND via bank transfer |
| `PayoutVnd` | `po_` | User withdraws VND to bank account |
| `TradeExecuted` | `tr_` | Crypto/VND trade execution |
| `DepositOnchain` | `dp_` | User deposits crypto from external wallet |
| `WithdrawOnchain` | `wd_` | User withdraws crypto to external wallet |

## Pay-in VND State Machine

### State Diagram

```
                                    +-------------+
                                    |   CREATED   |
                                    +------+------+
                                           |
                            +--------------+--------------+
                            |                             |
                            v                             v
                 +-------------------+             +-------------+
                 | INSTRUCTION_ISSUED|             |  CANCELLED  |
                 +--------+----------+             +-------------+
                          |
            +-------------+-------------+
            |             |             |
            v             v             v
     +-------------+ +------------+ +----------+
     |FUNDS_PENDING| |  EXPIRED   | | CANCELLED|
     +------+------+ +------------+ +----------+
            |
    +-------+-------+
    |               |
    v               v
+---------------+ +------------------+
|FUNDS_CONFIRMED| | MISMATCHED_AMOUNT|
+-------+-------+ +--------+---------+
        |                  |
  +-----+-----+    +-------+-------+
  |     |     |    |               |
  v     v     v    v               v
+---+ +----+ +--------+     +-------------+
|VND| |SUSP| |MANUAL  |     | MANUAL      |
|CRD| |FRAU| |REVIEW  |     | REVIEW      |
+---+ +----+ +---+----+     +------+------+
  |               |                |
  v               |    +-----------+-----------+
+----------+      |    |           |           |
| COMPLETED|<-----+----+           v           v
+----------+           |    +-------------+ +----------+
                       +<---|  VND_CREDITED| |CANCELLED |
                            +------+------+ +----------+
                                   |
                                   v
                            +----------+
                            | COMPLETED|
                            +----------+

Terminal States: COMPLETED, EXPIRED, SUSPECTED_FRAUD, CANCELLED
Error States: EXPIRED, MISMATCHED_AMOUNT, SUSPECTED_FRAUD
```

### State Definitions

| State | Description | Actions |
|-------|-------------|---------|
| `Created` | Intent created, awaiting instruction | Generate reference code |
| `InstructionIssued` | Virtual account or QR code generated | Wait for bank confirmation |
| `FundsPending` | Bank reports incoming transfer | Verify amount and sender |
| `FundsConfirmed` | Transfer verified and settled | Run compliance checks |
| `VndCredited` | User balance updated | Prepare completion |
| `Completed` | Success - all done | Send webhook |
| `Expired` | Timeout before confirmation | Notify user |
| `MismatchedAmount` | Received amount differs from expected | Escalate to review |
| `SuspectedFraud` | Compliance flags raised | Block and investigate |
| `ManualReview` | Requires human decision | Assign to analyst |
| `Cancelled` | Cancelled by user or system | Refund if needed |

### Valid Transitions

```rust
pub fn allowed_transitions(&self) -> Vec<PayinState> {
    match self {
        Created => vec![InstructionIssued, Cancelled],
        InstructionIssued => vec![FundsPending, Expired, Cancelled],
        FundsPending => vec![FundsConfirmed, MismatchedAmount, Expired],
        FundsConfirmed => vec![VndCredited, SuspectedFraud, ManualReview],
        VndCredited => vec![Completed],
        MismatchedAmount => vec![ManualReview, VndCredited],
        ManualReview => vec![VndCredited, SuspectedFraud, Cancelled],
        // Terminal states have no transitions
        Completed | Expired | SuspectedFraud | Cancelled => vec![],
    }
}
```

## Pay-out VND State Machine

### State Diagram

```
                          +----------+
                          | CREATED  |
                          +----+-----+
                               |
             +-----------------+-----------------+
             |                 |                 |
             v                 v                 v
    +----------------+  +----------------+  +-------------+
    | POLICY_APPROVED|  |REJECTED_BY_POLICY| |MANUAL_REVIEW|
    +-------+--------+  +----------------+  +------+------+
            |                                      |
    +-------+-------+              +---------------+---------------+
    |               |              |               |               |
    v               v              v               v               v
+-----------+  +----------+  +----------------+ +---------+ +----------+
| SUBMITTED |  | CANCELLED|  | POLICY_APPROVED| |REJECTED | | CANCELLED|
+-----+-----+  +----------+  +----------------+ +---------+ +----------+
      |
+-----+-----+-----+
|           |     |
v           v     v
+--------+ +------+ +-------+
|CONFIRMED| |BANK  | |TIMEOUT|
+----+----+ |REJECT| +---+---+
     |      +------+     |
     v                   +-------+--------+
+---------+              |                |
|COMPLETED|         +----------+    +-------------+
+---------+         | SUBMITTED|    | MANUAL_REVIEW|
                    +----------+    +-------------+

Terminal States: COMPLETED, REJECTED_BY_POLICY, BANK_REJECTED, CANCELLED
Error States: REJECTED_BY_POLICY, BANK_REJECTED, TIMEOUT
```

### State Definitions

| State | Description | Actions |
|-------|-------------|---------|
| `Created` | Payout request received | Validate balance, run policy |
| `PolicyApproved` | Passed AML/limit checks | Reserve funds, submit to bank |
| `Submitted` | Sent to bank for processing | Wait for confirmation |
| `Confirmed` | Bank confirms transfer | Finalize ledger |
| `Completed` | Success - funds delivered | Send webhook |
| `RejectedByPolicy` | Failed compliance checks | Notify user |
| `BankRejected` | Bank refused transfer | Return funds to balance |
| `Timeout` | Bank didn't respond in time | Retry or escalate |
| `ManualReview` | Requires human decision | Assign to analyst |
| `Cancelled` | Cancelled by user or system | Release reserved funds |

### Valid Transitions

```rust
pub fn allowed_transitions(&self) -> Vec<PayoutState> {
    match self {
        Created => vec![PolicyApproved, RejectedByPolicy, ManualReview],
        PolicyApproved => vec![Submitted, Cancelled],
        Submitted => vec![Confirmed, BankRejected, Timeout],
        Confirmed => vec![Completed],
        Timeout => vec![Submitted, ManualReview],
        ManualReview => vec![PolicyApproved, RejectedByPolicy, Cancelled],
        // Terminal states
        Completed | RejectedByPolicy | BankRejected | Cancelled => vec![],
    }
}
```

## Trade State Machine

### State Diagram

```
              +----------+
              | RECORDED |
              +----+-----+
                   |
         +---------+---------+
         |                   |
         v                   v
+----------------+    +--------------+
|POST_TRADE_CHECK|    |COMPLIANCE_HOLD|
+-------+--------+    +-------+------+
        |                     |
   +----+----+        +-------+-------+
   |         |        |               |
   v         v        v               v
+--------+ +------+ +--------+   +--------+
|SETTLED | |MANUAL| | MANUAL |   |REJECTED|
|LEDGER  | |REVIEW| | REVIEW |   +--------+
+---+----+ +--+---+ +---+----+
    |         |         |
    v         |         |
+--------+    |         |
|COMPLETED<---+---------+
+--------+

Terminal States: COMPLETED, REJECTED
Error States: COMPLIANCE_HOLD, REJECTED
```

### State Definitions

| State | Description | Actions |
|-------|-------------|---------|
| `Recorded` | Trade executed by matching engine | Record in database |
| `PostTradeChecked` | Post-trade compliance passed | Prepare ledger entries |
| `SettledLedger` | Ledger entries posted | Complete trade |
| `Completed` | Success - trade finalized | Send webhook |
| `ComplianceHold` | Trade flagged for review | Hold settlement |
| `ManualReview` | Requires analyst decision | Assign to analyst |
| `Rejected` | Trade cancelled due to compliance | Reverse positions |

### Valid Transitions

```rust
pub fn allowed_transitions(&self) -> Vec<TradeState> {
    match self {
        Recorded => vec![PostTradeChecked, ComplianceHold],
        PostTradeChecked => vec![SettledLedger, ManualReview],
        SettledLedger => vec![Completed],
        ComplianceHold => vec![ManualReview, Rejected],
        ManualReview => vec![PostTradeChecked, Rejected],
        // Terminal states
        Completed | Rejected => vec![],
    }
}
```

## On-chain Deposit State Machine

### State Diagram

```
            +----------+
            | DETECTED |
            +----+-----+
                 |
                 v
           +------------+
           | CONFIRMING |
           +-----+------+
                 |
                 v
           +-----------+
           | CONFIRMED |
           +-----+-----+
                 |
         +-------+-------+
         |               |
         v               v
   +------------+  +------------+
   | KYT_CHECKED|  | KYT_FLAGGED|
   +-----+------+  +-----+------+
         |               |
         v         +-----+-----+
   +---------+     |           |
   | CREDITED|     v           v
   +----+----+ +---------+ +--------+
        |      |MANUAL   | |REJECTED|
        v      |REVIEW   | +--------+
  +---------+  +----+----+
  |COMPLETED|       |
  +---------+       v
               +---------+
               | CREDITED|
               +----+----+
                    |
                    v
              +---------+
              |COMPLETED|
              +---------+

Terminal States: COMPLETED, REJECTED
Error States: KYT_FLAGGED, REJECTED
```

### State Definitions

| State | Description | Actions |
|-------|-------------|---------|
| `Detected` | On-chain deposit detected | Record transaction |
| `Confirming` | Waiting for block confirmations | Monitor chain |
| `Confirmed` | Required confirmations reached | Run KYT check |
| `KytChecked` | Address risk check passed | Credit balance |
| `Credited` | User balance updated | Prepare completion |
| `Completed` | Success - deposit finalized | Send webhook |
| `KytFlagged` | High-risk address detected | Investigate |
| `ManualReview` | Requires analyst decision | Assign to analyst |
| `Rejected` | Deposit rejected due to risk | Return funds if possible |

### Valid Transitions

```rust
pub fn allowed_transitions(&self) -> Vec<DepositState> {
    match self {
        Detected => vec![Confirming],
        Confirming => vec![Confirmed],
        Confirmed => vec![KytChecked, KytFlagged],
        KytChecked => vec![Credited],
        Credited => vec![Completed],
        KytFlagged => vec![ManualReview, Rejected],
        ManualReview => vec![Credited, Rejected],
        // Terminal states
        Completed | Rejected => vec![],
    }
}
```

## On-chain Withdraw State Machine

### State Diagram

```
                        +----------+
                        | CREATED  |
                        +----+-----+
                             |
               +-------------+-------------+
               |                           |
               v                           v
      +----------------+          +------------------+
      | POLICY_APPROVED|          | REJECTED_BY_POLICY|
      +-------+--------+          +------------------+
              |
      +-------+-------+
      |               |
      v               v
+------------+  +------------+
| KYT_CHECKED|  | KYT_FLAGGED|
+-----+------+  +-----+------+
      |               |
      v         +-----+-----+
  +--------+    |           |
  | SIGNED |    v           v
  +---+----+ +--------+ +----------+
      |      |MANUAL  | | CANCELLED|
      +------+REVIEW  | +----------+
      |      +---+----+
+-----+-----+    |
|           |    v
v           v  +----------------+
+-----------+ +-----------------+ | POLICY_APPROVED|
|BROADCASTED| |BROADCAST_FAILED | +----------------+
+-----+-----+ +--------+--------+
      |                |
      v          +-----+-----+
+------------+   |           |
| CONFIRMING |   v           v
+-----+------+ +--------+ +--------+
      |        | SIGNED | |MANUAL  |
+-----+-----+  +--------+ |REVIEW  |
|           |             +--------+
v           v
+----------+ +--------+
| CONFIRMED| | MANUAL |
+-----+----+ | REVIEW |
      |      +--------+
      v
+----------+
| COMPLETED|
+----------+

Terminal States: COMPLETED, REJECTED_BY_POLICY, CANCELLED
Error States: REJECTED_BY_POLICY, KYT_FLAGGED, BROADCAST_FAILED
```

### State Definitions

| State | Description | Actions |
|-------|-------------|---------|
| `Created` | Withdrawal request received | Validate balance, run policy |
| `PolicyApproved` | Passed compliance checks | Reserve funds, run KYT |
| `KytChecked` | Destination address approved | Prepare transaction |
| `Signed` | Transaction signed with custody key | Broadcast to chain |
| `Broadcasted` | Transaction sent to mempool | Monitor for inclusion |
| `Confirming` | Transaction in block, awaiting confirmations | Wait for finality |
| `Confirmed` | Required confirmations reached | Finalize |
| `Completed` | Success - withdrawal complete | Send webhook |
| `RejectedByPolicy` | Failed compliance checks | Return funds |
| `KytFlagged` | High-risk destination address | Hold and investigate |
| `BroadcastFailed` | Failed to broadcast transaction | Retry or escalate |
| `ManualReview` | Requires analyst decision | Assign to analyst |
| `Cancelled` | Cancelled by user or system | Return funds |

### Valid Transitions

```rust
pub fn allowed_transitions(&self) -> Vec<WithdrawState> {
    match self {
        Created => vec![PolicyApproved, RejectedByPolicy],
        PolicyApproved => vec![KytChecked, KytFlagged],
        KytChecked => vec![Signed],
        Signed => vec![Broadcasted, BroadcastFailed],
        Broadcasted => vec![Confirming],
        Confirming => vec![Confirmed, ManualReview],
        Confirmed => vec![Completed],
        KytFlagged => vec![ManualReview, Cancelled],
        BroadcastFailed => vec![Signed, ManualReview],
        ManualReview => vec![PolicyApproved, Cancelled],
        // Terminal states
        Completed | RejectedByPolicy | Cancelled => vec![],
    }
}
```

## Unified Intent State

All intent states are unified under a single enum for polymorphic handling:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "state")]
pub enum IntentState {
    Payin(PayinState),
    Payout(PayoutState),
    Trade(TradeState),
    Deposit(DepositState),
    Withdraw(WithdrawState),
}

impl IntentState {
    /// Check if this is a terminal (final) state
    pub fn is_terminal(&self) -> bool;

    /// Check if this is an error state
    pub fn is_error(&self) -> bool;

    /// Convert to string representation
    pub fn as_string(&self) -> String;
}
```

## State Transition Validation

Transitions are validated at runtime:

```rust
// Example: Validate and perform transition
pub async fn transition_intent(
    intent_id: &IntentId,
    target_state: IntentState,
) -> Result<()> {
    let current = self.get_intent_state(intent_id).await?;

    // Check if transition is allowed
    if !current.can_transition_to(&target_state) {
        return Err(Error::InvalidStateTransition {
            from: current.as_string(),
            to: target_state.as_string(),
        });
    }

    // Record state change event
    self.record_state_change(intent_id, &current, &target_state).await?;

    // Update state in database
    self.update_intent_state(intent_id, &target_state).await?;

    Ok(())
}
```

## Event Emission

Every state transition emits an event:

```rust
// Event published to NATS on state change
{
    "event_type": "intent.state_changed",
    "timestamp": "2024-01-15T10:30:00Z",
    "data": {
        "intent_id": "pi_01234567890",
        "intent_type": "PAYIN_VND",
        "tenant_id": "tenant_abc",
        "previous_state": "FUNDS_PENDING",
        "new_state": "FUNDS_CONFIRMED",
        "transition_reason": "Bank confirmation received"
    }
}
```

## Best Practices

1. **Always check `is_terminal()`** before attempting transitions
2. **Log all state changes** with full context for debugging
3. **Use transactions** when updating state and related data
4. **Emit events** after successful state transitions
5. **Handle manual review states** with clear escalation paths
6. **Set timeouts** for non-terminal states to trigger expiry
