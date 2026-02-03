# RampOSAccount
**Inherits:**
BaseAccount, Initializable, UUPSUpgradeable

**Title:**
RampOSAccount

**Author:**
RampOS Team

ERC-4337 compatible smart account for RampOS on/off-ramp operations

Implements Account Abstraction (ERC-4337) with extended session key support.
Features:
- Single owner ECDSA signature validation
- Batch transaction execution for gas efficiency
- Session keys with granular permissions (target, selector, spending limits)
- Gasless transactions via paymaster integration
- UUPS upgradeable pattern for future improvements
Security considerations:
- Only owner or EntryPoint can execute transactions
- Session keys have time-bounded validity
- Per-transaction and daily spending limits for session keys
- Target and function selector restrictions for session keys


## State Variables
### owner
Account owner address


```solidity
address public owner
```


### _ENTRY_POINT
ERC-4337 Entry Point contract reference (immutable for gas savings)


```solidity
IEntryPoint private immutable _ENTRY_POINT
```


### sessionKeys
Active session keys (legacy mapping for compatibility)


```solidity
mapping(address => SessionKey) public sessionKeys
```


### _sessionKeyStorage
Full session key storage with permissions


```solidity
mapping(address => SessionKeyStorage) internal _sessionKeyStorage
```


### _pendingSessionKey
Track pending session key for validation


```solidity
address internal _pendingSessionKey
```


## Functions
### onlyOwner

Modifier for owner-only functions


```solidity
modifier onlyOwner() ;
```

### onlyOwnerOrEntryPoint

Modifier for owner or entry point access control


```solidity
modifier onlyOwnerOrEntryPoint() ;
```

### checkSessionKeyPermissions

Modifier to check session key permissions on execute


```solidity
modifier checkSessionKeyPermissions(address dest, uint256 value, bytes calldata data) ;
```

### constructor

Constructor - sets immutable entry point and disables initializers


```solidity
constructor(IEntryPoint anEntryPoint) ;
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`anEntryPoint`|`IEntryPoint`|The ERC-4337 EntryPoint contract address|


### initialize

Initialize the account with an owner


```solidity
function initialize(address anOwner) public virtual initializer;
```

### entryPoint

Get the ERC-4337 entry point contract


```solidity
function entryPoint() public view virtual override returns (IEntryPoint);
```
**Returns**

|Name|Type|Description|
|----|----|-----------|
|`<none>`|`IEntryPoint`|The IEntryPoint interface of the entry point|


### execute

Execute a single transaction


```solidity
function execute(address dest, uint256 value, bytes calldata data)
    external
    override
    onlyOwnerOrEntryPoint
    checkSessionKeyPermissions(dest, value, data);
```

### executeBatch

Execute a batch of transactions


```solidity
function executeBatch(
    address[] calldata dests,
    uint256[] calldata values,
    bytes[] calldata datas
) external onlyOwnerOrEntryPoint;
```

### addSessionKey

Add a session key with permissions


```solidity
function addSessionKey(
    address key,
    uint48 validAfter,
    uint48 validUntil,
    SessionKeyPermissions calldata permissions
) external onlyOwner;
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`key`|`address`|The session key address|
|`validAfter`|`uint48`|Timestamp after which key is valid|
|`validUntil`|`uint48`|Timestamp until which key is valid|
|`permissions`|`SessionKeyPermissions`|The permissions for this session key|


### addSessionKeyLegacy

Add a session key with raw permissionsHash (legacy compatibility)

This creates a session key with unlimited permissions


```solidity
function addSessionKeyLegacy(
    address key,
    uint48 validAfter,
    uint48 validUntil,
    bytes32 permissionsHash
) external onlyOwner;
```

### removeSessionKey

Remove a session key


```solidity
function removeSessionKey(address key) external onlyOwner;
```

### updateSessionKeyPermissions

Update session key permissions


```solidity
function updateSessionKeyPermissions(address key, SessionKeyPermissions calldata permissions)
    external
    onlyOwner;
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`key`|`address`|The session key address|
|`permissions`|`SessionKeyPermissions`|The new permissions|


### getSessionKeyPermissions

Get session key permissions


```solidity
function getSessionKeyPermissions(address key)
    external
    view
    returns (SessionKeyPermissions memory permissions);
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`key`|`address`|The session key address|

**Returns**

|Name|Type|Description|
|----|----|-----------|
|`permissions`|`SessionKeyPermissions`|The permissions struct|


### getSessionKeySpendingInfo

Get session key spending info


```solidity
function getSessionKeySpendingInfo(address key)
    external
    view
    returns (uint256 dailySpent, uint256 dailyRemaining, uint256 spendingLimit);
```
**Parameters**

|Name|Type|Description|
|----|----|-----------|
|`key`|`address`|The session key address|

**Returns**

|Name|Type|Description|
|----|----|-----------|
|`dailySpent`|`uint256`|Amount spent today|
|`dailyRemaining`|`uint256`|Amount remaining for today (0 if unlimited)|
|`spendingLimit`|`uint256`|Per-transaction limit (0 if unlimited)|


### isValidSessionKey

Check if a session key is valid


```solidity
function isValidSessionKey(address key) public view returns (bool);
```

### isTargetAllowed

Check if a target is allowed for a session key


```solidity
function isTargetAllowed(address key, address target) public view returns (bool);
```

### isSelectorAllowed

Check if a selector is allowed for a session key


```solidity
function isSelectorAllowed(address key, bytes4 selector) public view returns (bool);
```

### _validateSignature

Validate user operation signature


```solidity
function _validateSignature(PackedUserOperation calldata userOp, bytes32 userOpHash)
    internal
    virtual
    override
    returns (uint256 validationData);
```

### _validateSessionKeyPermissions

Validate session key permissions for a call


```solidity
function _validateSessionKeyPermissions(
    address key,
    address target,
    uint256 value,
    bytes calldata data
) internal;
```

### _computePermissionsHash

Compute permissions hash


```solidity
function _computePermissionsHash(SessionKeyPermissions calldata permissions)
    internal
    pure
    returns (bytes32);
```

### _call

Internal call function


```solidity
function _call(address target, uint256 value, bytes memory data) internal;
```

### _authorizeUpgrade

Authorize upgrade (only owner)


```solidity
function _authorizeUpgrade(address newImplementation) internal override onlyOwner;
```

### receive

Receive ETH


```solidity
receive() external payable;
```

## Events
### AccountInitialized
Events


```solidity
event AccountInitialized(address indexed owner);
```

### SessionKeyAdded

```solidity
event SessionKeyAdded(address indexed key, uint48 validUntil);
```

### SessionKeyRemoved

```solidity
event SessionKeyRemoved(address indexed key);
```

### SessionKeyPermissionsUpdated

```solidity
event SessionKeyPermissionsUpdated(address indexed key, bytes32 permissionsHash);
```

### DailyLimitReset

```solidity
event DailyLimitReset(address indexed key, uint256 day);
```

## Errors
### NotOwner
Errors


```solidity
error NotOwner();
```

### NotOwnerOrEntryPoint

```solidity
error NotOwnerOrEntryPoint();
```

### InvalidSessionKey

```solidity
error InvalidSessionKey();
```

### SessionKeyExpired

```solidity
error SessionKeyExpired();
```

### TargetNotAllowed

```solidity
error TargetNotAllowed(address target);
```

### SelectorNotAllowed

```solidity
error SelectorNotAllowed(bytes4 selector);
```

### SpendingLimitExceeded

```solidity
error SpendingLimitExceeded(uint256 requested, uint256 limit);
```

### DailyLimitExceeded

```solidity
error DailyLimitExceeded(uint256 requested, uint256 remaining);
```

## Structs
### SessionKeyPermissions
Session key permissions structure


```solidity
struct SessionKeyPermissions {
    address[] allowedTargets; // Contracts session key can call
    bytes4[] allowedSelectors; // Function selectors session key can call
    uint256 spendingLimit; // Max ETH per transaction (0 = unlimited)
    uint256 dailyLimit; // Max ETH per day (0 = unlimited)
}
```

### SessionKey
Session key data


```solidity
struct SessionKey {
    address key;
    uint48 validAfter;
    uint48 validUntil;
    bytes32 permissionsHash;
}
```

### SessionKeyStorage
Storage for session key permissions


```solidity
struct SessionKeyStorage {
    SessionKey metadata;
    address[] allowedTargets;
    bytes4[] allowedSelectors;
    uint256 spendingLimit;
    uint256 dailyLimit;
    uint256 dailySpent;
    uint256 lastResetDay;
}
```

