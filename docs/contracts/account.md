# RampOSAccount - Smart Account Documentation

## Overview

`RampOSAccount` is an ERC-4337 compliant smart contract wallet that serves as the core account abstraction implementation for RampOS. It enables users to interact with the blockchain through a smart contract instead of a traditional EOA (Externally Owned Account).

## Contract Details

```solidity
contract RampOSAccount is BaseAccount, Initializable, UUPSUpgradeable {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    address public owner;
    IEntryPoint private immutable _entryPoint;
    mapping(address => SessionKey) public sessionKeys;
}
```

### Inheritance

- **BaseAccount** (ERC-4337): Provides core account abstraction functionality
- **Initializable** (OpenZeppelin): Enables proxy pattern initialization
- **UUPSUpgradeable** (OpenZeppelin): Allows contract upgrades

### Dependencies

- `@account-abstraction/contracts` - ERC-4337 interfaces and base contracts
- `@openzeppelin/contracts` - Security utilities (ECDSA, MessageHashUtils)

## Features

### 1. Single Owner ECDSA Signatures

The account is controlled by a single owner address that can sign transactions using standard ECDSA signatures.

```solidity
// Owner is set during initialization
function initialize(address anOwner) public virtual initializer {
    owner = anOwner;
    emit AccountInitialized(anOwner);
}
```

### 2. Transaction Execution

#### Single Transaction

Execute a single call to any address:

```solidity
function execute(
    address dest,
    uint256 value,
    bytes calldata data
) external onlyOwnerOrEntryPoint;
```

**Example: ETH Transfer**
```solidity
// Transfer 1 ETH to recipient
account.execute(recipientAddress, 1 ether, "");
```

**Example: ERC20 Transfer**
```solidity
bytes memory data = abi.encodeCall(
    IERC20.transfer,
    (recipientAddress, 100 * 10**18)
);
account.execute(tokenAddress, 0, data);
```

#### Batch Transactions

Execute multiple calls atomically in a single transaction:

```solidity
function executeBatch(
    address[] calldata dests,
    uint256[] calldata values,
    bytes[] calldata datas
) external onlyOwnerOrEntryPoint;
```

**Example: Multiple Token Transfers**
```solidity
address[] memory dests = new address[](3);
uint256[] memory values = new uint256[](3);
bytes[] memory datas = new bytes[](3);

// Transfer Token A
dests[0] = tokenA;
values[0] = 0;
datas[0] = abi.encodeCall(IERC20.transfer, (recipient, amount1));

// Transfer Token B
dests[1] = tokenB;
values[1] = 0;
datas[1] = abi.encodeCall(IERC20.transfer, (recipient, amount2));

// Approve Token C
dests[2] = tokenC;
values[2] = 0;
datas[2] = abi.encodeCall(IERC20.approve, (spender, amount3));

account.executeBatch(dests, values, datas);
```

## Session Keys

Session keys allow the account owner to delegate limited signing authority to other addresses for a specific time period.

### Session Key Structure

```solidity
struct SessionKey {
    address key;           // The session key address
    uint48 validAfter;     // Unix timestamp when key becomes valid
    uint48 validUntil;     // Unix timestamp when key expires
    bytes32 permissionsHash; // Reserved for future permission scoping
}
```

### Adding a Session Key

```solidity
function addSessionKey(
    address key,
    uint48 validAfter,
    uint48 validUntil,
    bytes32 permissionsHash
) external onlyOwner;
```

**Example: Create 1-hour Session Key**
```solidity
address sessionKey = 0x1234...;

account.addSessionKey(
    sessionKey,
    uint48(block.timestamp),           // Valid immediately
    uint48(block.timestamp + 1 hours), // Expires in 1 hour
    bytes32(0)                         // No permission restrictions (reserved)
);
```

### Removing a Session Key

```solidity
function removeSessionKey(address key) external onlyOwner;
```

**Example:**
```solidity
account.removeSessionKey(sessionKeyAddress);
```

### Checking Session Key Validity

```solidity
function isValidSessionKey(address key) public view returns (bool);
```

**Example:**
```solidity
if (account.isValidSessionKey(sessionKeyAddress)) {
    // Session key is valid
}
```

### Session Key Validation Logic

When validating a UserOperation signature, the contract checks:

1. Is the signer the owner? -> Valid
2. Is the signer a registered session key?
   - Is current time >= `validAfter`?
   - Is current time <= `validUntil`?
   -> Valid with time bounds

```solidity
function _validateSignature(
    PackedUserOperation calldata userOp,
    bytes32 userOpHash
) internal virtual override returns (uint256 validationData) {
    bytes32 hash = userOpHash.toEthSignedMessageHash();
    address signer = hash.recover(userOp.signature);

    // Check if signer is owner
    if (signer == owner) {
        return 0; // Valid
    }

    // Check if signer is a valid session key
    SessionKey memory session = sessionKeys[signer];
    if (session.key != address(0)) {
        if (block.timestamp < session.validAfter) {
            return SIG_VALIDATION_FAILED;
        }
        if (block.timestamp > session.validUntil) {
            return SIG_VALIDATION_FAILED;
        }
        return _packValidationData(false, session.validUntil, session.validAfter);
    }

    return SIG_VALIDATION_FAILED;
}
```

### Session Key Use Cases

| Use Case | Duration | Description |
|----------|----------|-------------|
| dApp Session | 1-24 hours | Allow a dApp to sign transactions temporarily |
| Gaming | 1-4 hours | Enable in-game transactions without wallet popups |
| Automation | Days/Weeks | Allow bots to execute specific operations |
| Temporary Access | Minutes | Quick one-time delegation |

### Future: Permission-Scoped Session Keys

The `permissionsHash` field is reserved for future implementation of scoped permissions:

```solidity
// Future API (not yet implemented)
bytes32 permissionsHash = keccak256(abi.encode(
    allowedTargets,      // Contracts the session key can call
    allowedFunctions,    // Function selectors allowed
    spendingLimit,       // Max ETH/tokens that can be spent
    allowedAssets        // Specific tokens allowed
));
```

## Social Recovery

**Note:** Social recovery is not currently implemented in RampOSAccount. Future versions may include:

- Guardian-based recovery
- Time-locked recovery
- Multi-signature recovery threshold

## Access Control

### Modifiers

```solidity
modifier onlyOwner() {
    if (msg.sender != owner) revert NotOwner();
    _;
}

modifier onlyOwnerOrEntryPoint() {
    if (msg.sender != owner && msg.sender != address(_entryPoint)) {
        revert NotOwnerOrEntryPoint();
    }
    _;
}
```

### Function Access Matrix

| Function | Owner | EntryPoint | Anyone |
|----------|-------|------------|--------|
| `execute` | Yes | Yes | No |
| `executeBatch` | Yes | Yes | No |
| `addSessionKey` | Yes | No | No |
| `removeSessionKey` | Yes | No | No |
| `isValidSessionKey` | Yes | Yes | Yes |

## Upgradeability

RampOSAccount uses the UUPS (Universal Upgradeable Proxy Standard) pattern for upgrades.

### How Upgrades Work

1. Deploy new implementation contract
2. Call `upgradeTo(newImplementation)` from owner
3. Proxy delegates all calls to new implementation

```solidity
function _authorizeUpgrade(
    address newImplementation
) internal override onlyOwner {}
```

### Upgrade Considerations

- Only the owner can authorize upgrades
- Storage layout must be preserved across versions
- New implementations must inherit from UUPSUpgradeable

## Events

```solidity
event AccountInitialized(address indexed owner);
event SessionKeyAdded(address indexed key, uint48 validUntil);
event SessionKeyRemoved(address indexed key);
```

## Errors

```solidity
error NotOwner();           // Caller is not the owner
error NotOwnerOrEntryPoint(); // Caller is not owner or EntryPoint
error InvalidSessionKey();    // Session key is invalid
error SessionKeyExpired();    // Session key has expired
```

## Integration Examples

### TypeScript/Ethers.js

```typescript
import { ethers } from "ethers";

const accountAbi = [/* RampOSAccount ABI */];
const account = new ethers.Contract(accountAddress, accountAbi, signer);

// Execute a transaction
await account.execute(
    recipientAddress,
    ethers.parseEther("1.0"),
    "0x"
);

// Add a session key
const now = Math.floor(Date.now() / 1000);
await account.addSessionKey(
    sessionKeyAddress,
    now,
    now + 3600, // 1 hour
    ethers.ZeroHash
);

// Check if session key is valid
const isValid = await account.isValidSessionKey(sessionKeyAddress);
console.log("Session key valid:", isValid);
```

### Viem

```typescript
import { createPublicClient, createWalletClient, http } from "viem";
import { mainnet } from "viem/chains";

const client = createPublicClient({
    chain: mainnet,
    transport: http()
});

// Read session key
const sessionKey = await client.readContract({
    address: accountAddress,
    abi: accountAbi,
    functionName: "sessionKeys",
    args: [sessionKeyAddress]
});

console.log("Session key valid until:", sessionKey.validUntil);
```

## Testing

See [RampOSAccount.t.sol](../../contracts/test/RampOSAccount.t.sol) for comprehensive test examples:

```solidity
function test_CreateAccount() public {
    uint256 salt = 12345;
    address predicted = factory.getAddress(owner, salt);
    RampOSAccount account = factory.createAccount(owner, salt);

    assertEq(address(account), predicted);
    assertEq(account.owner(), owner);
}

function test_SessionKey() public {
    // Add session key
    vm.prank(owner);
    account.addSessionKey(sessionKey, validAfter, validUntil, bytes32(0));

    assertTrue(account.isValidSessionKey(sessionKey));

    // Remove session key
    vm.prank(owner);
    account.removeSessionKey(sessionKey);

    assertFalse(account.isValidSessionKey(sessionKey));
}
```

## Related Documentation

- [Overview](./overview.md) - Contract architecture overview
- [Paymaster](./paymaster.md) - Gas sponsorship
- [Security](./security.md) - Security considerations
