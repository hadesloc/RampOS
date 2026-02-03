# RampOSPaymaster
**Inherits:**
IPaymaster, Ownable

**Title:**
RampOSPaymaster

**Author:**
RampOS Team

Verifying paymaster for RampOS sponsored transactions

Implements ERC-4337 paymaster interface with signature-based sponsorship.
Features:
- Signature-based sponsorship verification using ECDSA
- Per-tenant daily spending limits
- Per-user daily rate limiting
- Timelocked withdrawals for security (24h delay)
Security considerations:
- Only the verifying signer can authorize sponsorships
- Withdrawals require 24h timelock to prevent instant drains
- Rate limits prevent abuse


## State Variables
### ENTRY_POINT
ERC-4337 Entry Point contract reference


```solidity
IEntryPoint public immutable ENTRY_POINT
```


### verifyingSigner
Authorized signer for validating paymaster sponsorship data


```solidity
address public verifyingSigner
```


### tenantDailySpent
Tenant spending limits


```solidity
mapping(bytes32 => uint256) public tenantDailySpent
```


### tenantDailyLimit

```solidity
mapping(bytes32 => uint256) public tenantDailyLimit
```


### tenantLastResetDay

```solidity
mapping(bytes32 => uint256) public tenantLastResetDay
```


### userDailyOps
User rate limits


```solidity
mapping(address => uint256) public userDailyOps
```


### userLastResetDay

```solidity
mapping(address => uint256) public userLastResetDay
```


### maxOpsPerUserPerDay

```solidity
uint256 public maxOpsPerUserPerDay = 100
```


### WITHDRAW_DELAY
Timelock configuration


```solidity
uint256 public constant WITHDRAW_DELAY = 24 hours
```


### pendingWithdrawAmount
Pending withdrawal state


```solidity
uint256 public pendingWithdrawAmount
```


### withdrawRequestTime

```solidity
uint256 public withdrawRequestTime
```


### pendingWithdrawTo

```solidity
address public pendingWithdrawTo
```


## Functions
### constructor


```solidity
constructor(IEntryPoint _entryPoint, address _signer) Ownable(msg.sender);
```

### validatePaymasterUserOp

Validate a user operation for sponsorship


```solidity
function validatePaymasterUserOp(
    PackedUserOperation calldata userOp,
    bytes32 userOpHash,
    uint256 maxCost
) external override returns (bytes memory context, uint256 validationData);
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`userOp`|`PackedUserOperation`|The packed user operation to validate|
|`userOpHash`|`bytes32`|Hash of the user operation|
|`maxCost`|`uint256`|Maximum gas cost to be paid|

**Returns**

|Name|Type|Description|
|----|----|-----------|
|`context`|`bytes`|Encoded context for postOp|
|`validationData`|`uint256`|Packed validation data with time range|


### postOp

Post-operation handler called after user operation execution


```solidity
function postOp(
    IPaymaster.PostOpMode mode,
    bytes calldata context,
    uint256 actualGasCost,
    uint256 /* actualUserOpFeePerGas */
) external override;
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`mode`|`IPaymaster.PostOpMode`|The post-op mode (success, reverted, or postOpReverted)|
|`context`|`bytes`|Encoded context from validatePaymasterUserOp|
|`actualGasCost`|`uint256`|Actual gas cost of the operation|
|`<none>`|`uint256`||


### _checkAndUpdateTenantLimit

Check and update tenant daily limit


```solidity
function _checkAndUpdateTenantLimit(bytes32 tenantId, uint256 cost) internal;
```

### _checkAndUpdateUserRateLimit

Check and update user rate limit


```solidity
function _checkAndUpdateUserRateLimit(address user) internal;
```

### _packValidationData

Pack validation data


```solidity
function _packValidationData(bool sigFailed, uint48 validUntil, uint48 validAfter)
    internal
    pure
    returns (uint256);
```

### setSigner


```solidity
function setSigner(address _signer) external onlyOwner;
```

### setTenantLimit


```solidity
function setTenantLimit(bytes32 tenantId, uint256 limit) external onlyOwner;
```

### setMaxOpsPerUser


```solidity
function setMaxOpsPerUser(uint256 maxOps) external onlyOwner;
```

### deposit

Deposit ETH to EntryPoint for gas sponsorship


```solidity
function deposit() external payable;
```

### requestWithdraw

Request a withdrawal with timelock


```solidity
function requestWithdraw(address payable to, uint256 amount) external onlyOwner;
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`to`|`address payable`|The address to withdraw to|
|`amount`|`uint256`|The amount to withdraw|


### executeWithdraw

Execute a pending withdrawal after timelock expires


```solidity
function executeWithdraw() external onlyOwner;
```

### cancelWithdraw

Cancel a pending withdrawal request


```solidity
function cancelWithdraw() external onlyOwner;
```

### getWithdrawTimeRemaining

Get the time remaining until withdrawal can be executed


```solidity
function getWithdrawTimeRemaining() external view returns (uint256 timeRemaining);
```
**Returns**

|Name|Type|Description|
|----|----|-----------|
|`timeRemaining`|`uint256`|Seconds remaining, or 0 if ready/no pending|


### isWithdrawReady

Check if a withdrawal is ready to execute


```solidity
function isWithdrawReady() external view returns (bool ready);
```
**Returns**

|Name|Type|Description|
|----|----|-----------|
|`ready`|`bool`|True if withdrawal can be executed|


### getPendingWithdraw

Get pending withdrawal details


```solidity
function getPendingWithdraw()
    external
    view
    returns (address to, uint256 amount, uint256 requestTime, uint256 executeAfter);
```
**Returns**

|Name|Type|Description|
|----|----|-----------|
|`to`|`address`|Recipient address|
|`amount`|`uint256`|Pending amount|
|`requestTime`|`uint256`|Time of request|
|`executeAfter`|`uint256`|Time when execution becomes possible|


### withdrawTo

This function is kept for interface compatibility but reverts

Legacy function - deprecated, use requestWithdraw + executeWithdraw


```solidity
function withdrawTo(address payable, uint256) external view onlyOwner;
```

### getDeposit

Get current deposit balance in EntryPoint


```solidity
function getDeposit() external view returns (uint256);
```
**Returns**

|Name|Type|Description|
|----|----|-----------|
|`<none>`|`uint256`|The balance of this paymaster in the EntryPoint|


## Events
### SignerUpdated
Events


```solidity
event SignerUpdated(address indexed oldSigner, address indexed newSigner);
```

### TenantLimitSet

```solidity
event TenantLimitSet(bytes32 indexed tenantId, uint256 limit);
```

### Sponsored

```solidity
event Sponsored(address indexed sender, bytes32 indexed tenantId, uint256 gasCost);
```

### WithdrawRequested

```solidity
event WithdrawRequested(address indexed to, uint256 amount, uint256 executeAfter);
```

### WithdrawExecuted

```solidity
event WithdrawExecuted(address indexed to, uint256 amount);
```

### WithdrawCancelled

```solidity
event WithdrawCancelled(address indexed to, uint256 amount);
```

## Errors
### InvalidSignature
Errors


```solidity
error InvalidSignature();
```

### TenantLimitExceeded

```solidity
error TenantLimitExceeded();
```

### UserRateLimitExceeded

```solidity
error UserRateLimitExceeded();
```

### PaymasterDepositTooLow

```solidity
error PaymasterDepositTooLow();
```

### WithdrawAlreadyPending

```solidity
error WithdrawAlreadyPending();
```

### NoWithdrawPending

```solidity
error NoWithdrawPending();
```

### WithdrawNotReady

```solidity
error WithdrawNotReady();
```

### WithdrawExpired

```solidity
error WithdrawExpired();
```

