# Handoff: Add Timelock to Paymaster Contract

## Task Summary
Added a timelock mechanism to the `RampOSPaymaster` contract to prevent immediate withdrawal of funds by the owner. This security enhancement requires a 24-hour delay between requesting and executing withdrawals.

## Changes Made

### Contract Changes (`contracts/src/RampOSPaymaster.sol`)

#### New State Variables
- `WITHDRAW_DELAY`: Constant set to 24 hours
- `pendingWithdrawAmount`: Amount pending for withdrawal
- `withdrawRequestTime`: Timestamp when withdrawal was requested
- `pendingWithdrawTo`: Recipient address for pending withdrawal

#### New Events
- `WithdrawRequested(address indexed to, uint256 amount, uint256 executeAfter)`: Emitted when withdrawal is requested
- `WithdrawExecuted(address indexed to, uint256 amount)`: Emitted when withdrawal is executed
- `WithdrawCancelled(address indexed to, uint256 amount)`: Emitted when withdrawal is cancelled

#### New Errors
- `WithdrawAlreadyPending`: Cannot request when another request is pending
- `NoWithdrawPending`: Cannot execute/cancel without pending request
- `WithdrawNotReady`: Cannot execute before timelock expires
- `WithdrawExpired`: Cannot execute after 7-day expiry window

#### New Functions
1. **`requestWithdraw(address payable to, uint256 amount)`**
   - Initiates a withdrawal request
   - Reverts if another request is already pending
   - Emits `WithdrawRequested` event

2. **`executeWithdraw()`**
   - Executes pending withdrawal after timelock expires
   - Must be called within 7 days of becoming ready (expiry window)
   - Follows CEI pattern (clears state before external call)
   - Emits `WithdrawExecuted` event

3. **`cancelWithdraw()`**
   - Cancels pending withdrawal request
   - Clears all pending state
   - Emits `WithdrawCancelled` event

4. **`getWithdrawTimeRemaining()`**
   - Returns seconds remaining until withdrawal can be executed
   - Returns 0 if ready or no pending request

5. **`isWithdrawReady()`**
   - Returns true if withdrawal can be executed now
   - Checks both timelock and expiry window

6. **`getPendingWithdraw()`**
   - Returns pending withdrawal details (to, amount, requestTime, executeAfter)

#### Deprecated Function
- `withdrawTo()` now reverts with message "Use requestWithdraw + executeWithdraw"

### Test Changes (`contracts/test/RampOSPaymaster.t.sol`)

Added 17 new tests covering:
- Request withdrawal flow
- Execute withdrawal after timelock
- Cancel withdrawal
- Revert conditions (already pending, not ready, no pending, expired)
- Owner-only access control
- Time remaining and ready status helpers
- Legacy withdrawTo revert
- Cancel and re-request flow
- WITHDRAW_DELAY constant value

## Security Considerations

1. **24-hour timelock**: Provides time to detect and respond to compromised owner keys
2. **7-day expiry window**: Prevents stale requests from being executed much later
3. **CEI pattern**: State cleared before external call to prevent reentrancy
4. **Single pending request**: Only one withdrawal request can be active at a time

## Test Results
```
Ran 19 tests for test/RampOSPaymaster.t.sol:RampOSPaymasterTest
[PASS] test_CancelWithdraw() (gas: 68642)
[PASS] test_CancelWithdraw_OnlyOwner() (gas: 87786)
[PASS] test_CancelWithdraw_RevertIfNoPending() (gas: 16266)
[PASS] test_ExecuteWithdraw_AfterTimelock() (gas: 72444)
[PASS] test_ExecuteWithdraw_OnlyOwner() (gas: 88351)
[PASS] test_ExecuteWithdraw_RevertIfExpired() (gas: 86640)
[PASS] test_ExecuteWithdraw_RevertIfNoPending() (gas: 16117)
[PASS] test_ExecuteWithdraw_RevertIfNotReady() (gas: 86072)
[PASS] test_GetWithdrawTimeRemaining() (gas: 90265)
[PASS] test_IsWithdrawReady() (gas: 92718)
[PASS] test_RequestWithdraw() (gas: 85127)
[PASS] test_RequestWithdraw_EmitsEvent() (gas: 86088)
[PASS] test_RequestWithdraw_OnlyOwner() (gas: 15768)
[PASS] test_RequestWithdraw_RevertIfAlreadyPending() (gas: 85678)
[PASS] test_RequestWithdraw_ThenCancel_ThenNewRequest() (gas: 118334)
[PASS] test_TenantLimit() (gas: 126409)
[PASS] test_ValidateUserOp() (gas: 79172)
[PASS] test_WITHDRAW_DELAY_Is24Hours() (gas: 5765)
[PASS] test_WithdrawToLegacy_Reverts() (gas: 15674)
Suite result: ok. 19 passed; 0 failed; 0 skipped
```

## Files Modified
- `C:\Users\hades\OneDrive\Desktop\New folder (6)\contracts\src\RampOSPaymaster.sol`
- `C:\Users\hades\OneDrive\Desktop\New folder (6)\contracts\test\RampOSPaymaster.t.sol`
- `C:\Users\hades\OneDrive\Desktop\New folder (6)\contracts\foundry.toml` (updated solc to 0.8.28, added via_ir)

## Usage Example

```solidity
// Step 1: Request withdrawal (starts 24-hour timelock)
paymaster.requestWithdraw(recipientAddress, 1 ether);

// Step 2: Wait 24 hours...

// Step 3: Check if ready
if (paymaster.isWithdrawReady()) {
    paymaster.executeWithdraw();
}

// Or cancel if needed
paymaster.cancelWithdraw();
```

## Completion Status
- [x] Added timelock mechanism
- [x] Added requestWithdraw function
- [x] Added executeWithdraw function
- [x] Added cancelWithdraw function
- [x] Added state variables
- [x] Added events
- [x] Updated/deprecated withdrawTo function
- [x] Added comprehensive tests
- [x] All 19 tests passing
