# RampOSPaymaster - Gas Sponsorship Documentation

## Overview

`RampOSPaymaster` is a verifying paymaster that enables gasless transactions for RampOS users. It validates sponsorship requests using cryptographic signatures and enforces spending limits per tenant and rate limits per user.

## Contract Details

```solidity
contract RampOSPaymaster is IPaymaster, Ownable {
    using ECDSA for bytes32;
    using MessageHashUtils for bytes32;

    IEntryPoint public immutable entryPoint;
    address public verifyingSigner;

    // Tenant limits
    mapping(bytes32 => uint256) public tenantDailySpent;
    mapping(bytes32 => uint256) public tenantDailyLimit;
    mapping(bytes32 => uint256) public tenantLastResetDay;

    // User rate limits
    mapping(address => uint256) public userDailyOps;
    mapping(address => uint256) public userLastResetDay;
    uint256 public maxOpsPerUserPerDay = 100;
}
```

## How Gas Sponsorship Works

### Flow Diagram

```
+--------+     +----------+     +------------+     +---------+     +----------+
| User   |---->| Backend  |---->| Bundler    |---->| Entry   |---->| Paymaster|
| Wallet |     | (Signs)  |     |            |     | Point   |     |          |
+--------+     +----------+     +------------+     +---------+     +----------+
                    |                                                    |
                    |  1. Request sponsorship                            |
                    |                                                    |
                    |  2. Validate tenant/user                           |
                    |                                                    |
                    |  3. Sign paymaster data                            |
                    |                                                    |
                    |  4. Return signed UserOp                           |
                    |                                                    |
                    +----------------------------------------------------+
                                             |
                                             v
                               5. Bundler submits to EntryPoint
                                             |
                                             v
                               6. Paymaster validates & pays gas
```

### Step-by-Step Process

1. **User creates UserOperation**: User wants to execute a transaction
2. **Backend validates request**: Backend checks if user is eligible for sponsorship
3. **Backend signs paymaster data**: Creates cryptographic signature authorizing gas payment
4. **User submits to bundler**: Complete UserOperation with paymaster data
5. **Bundler submits to EntryPoint**: Bundles multiple UserOps together
6. **Paymaster validates**: Verifies signature, limits, and pays gas

## Verification Flow

### Paymaster Data Format

The `paymasterAndData` field contains:

```
| Offset | Length | Field       | Description                    |
|--------|--------|-------------|--------------------------------|
| 0      | 20     | paymaster   | Paymaster contract address     |
| 20     | 32     | tenantId    | Unique identifier for tenant   |
| 52     | 6      | validUntil  | Expiration timestamp           |
| 58     | 6      | validAfter  | Start validity timestamp       |
| 64     | 65     | signature   | Backend signature (r,s,v)      |
```

Total length: 129 bytes (20 + 32 + 6 + 6 + 65)

### Signature Generation

The backend signs a hash of the UserOperation and sponsorship parameters:

```solidity
bytes32 hash = keccak256(
    abi.encodePacked(
        userOpHash,   // Hash of the UserOperation
        tenantId,     // Tenant identifier
        validUntil,   // Expiration time
        validAfter    // Start time
    )
).toEthSignedMessageHash();

// Backend signs this hash with verifyingSigner private key
(v, r, s) = sign(hash, signerPrivateKey);
```

### Validation in Contract

```solidity
function validatePaymasterUserOp(
    PackedUserOperation calldata userOp,
    bytes32 userOpHash,
    uint256 maxCost
) external override returns (bytes memory context, uint256 validationData) {
    require(msg.sender == address(entryPoint), "Only entry point");

    // 1. Parse paymaster data
    bytes calldata paymasterData = userOp.paymasterAndData[20:];
    bytes32 tenantId = bytes32(paymasterData[0:32]);
    uint48 validUntil = uint48(bytes6(paymasterData[32:38]));
    uint48 validAfter = uint48(bytes6(paymasterData[38:44]));
    bytes calldata signature = paymasterData[44:109];

    // 2. Verify signature
    bytes32 hash = keccak256(
        abi.encodePacked(userOpHash, tenantId, validUntil, validAfter)
    ).toEthSignedMessageHash();

    if (hash.recover(signature) != verifyingSigner) {
        revert InvalidSignature();
    }

    // 3. Check tenant daily limit
    _checkAndUpdateTenantLimit(tenantId, maxCost);

    // 4. Check user rate limit
    _checkAndUpdateUserRateLimit(userOp.sender);

    // 5. Return context and validation data
    context = abi.encode(userOp.sender, tenantId, maxCost);
    validationData = _packValidationData(false, validUntil, validAfter);
}
```

## Spending Limits

### Per-Tenant Daily Limits

Each tenant (business/application) can have a daily spending limit:

```solidity
mapping(bytes32 => uint256) public tenantDailyLimit;
mapping(bytes32 => uint256) public tenantDailySpent;
mapping(bytes32 => uint256) public tenantLastResetDay;
```

**Setting a Tenant Limit:**
```solidity
// Set 10 ETH daily limit for tenant
bytes32 tenantId = keccak256("my-app");
paymaster.setTenantLimit(tenantId, 10 ether);
```

**How Limits Work:**
1. Each day (UTC) the counter resets
2. Each sponsored transaction adds to `tenantDailySpent`
3. If `spent + newCost > limit`, transaction reverts

```solidity
function _checkAndUpdateTenantLimit(bytes32 tenantId, uint256 cost) internal {
    uint256 today = block.timestamp / 1 days;

    // Reset if new day
    if (tenantLastResetDay[tenantId] < today) {
        tenantDailySpent[tenantId] = 0;
        tenantLastResetDay[tenantId] = today;
    }

    // Check limit
    uint256 limit = tenantDailyLimit[tenantId];
    if (limit > 0 && tenantDailySpent[tenantId] + cost > limit) {
        revert TenantLimitExceeded();
    }

    tenantDailySpent[tenantId] += cost;
}
```

### Per-User Rate Limits

Individual users are limited to a maximum number of operations per day:

```solidity
uint256 public maxOpsPerUserPerDay = 100; // Default
mapping(address => uint256) public userDailyOps;
mapping(address => uint256) public userLastResetDay;
```

**Adjusting User Limits:**
```solidity
paymaster.setMaxOpsPerUser(200); // Allow 200 ops/day/user
```

## Post-Operation Handling

After transaction execution, the paymaster handles gas refunds:

```solidity
function postOp(
    PostOpMode mode,
    bytes calldata context,
    uint256 actualGasCost,
    uint256 actualUserOpFeePerGas
) external override {
    require(msg.sender == address(entryPoint), "Only entry point");

    (address sender, bytes32 tenantId, uint256 maxCost) = abi.decode(
        context, (address, bytes32, uint256)
    );

    emit Sponsored(sender, tenantId, actualGasCost);

    // Refund unused gas allocation to tenant's daily allowance
    if (maxCost > actualGasCost) {
        uint256 refund = maxCost - actualGasCost;
        if (tenantDailySpent[tenantId] >= refund) {
            tenantDailySpent[tenantId] -= refund;
        }
    }
}
```

## Integration Guide

### Backend Implementation

```typescript
import { ethers } from "ethers";

class PaymasterService {
    private signer: ethers.Wallet;
    private paymasterAddress: string;

    constructor(signerPrivateKey: string, paymasterAddress: string) {
        this.signer = new ethers.Wallet(signerPrivateKey);
        this.paymasterAddress = paymasterAddress;
    }

    async sponsorUserOp(
        userOpHash: string,
        tenantId: string,
        validitySeconds: number = 300
    ): Promise<string> {
        const now = Math.floor(Date.now() / 1000);
        const validAfter = now;
        const validUntil = now + validitySeconds;

        // Create message hash
        const tenantIdBytes = ethers.id(tenantId);
        const message = ethers.solidityPacked(
            ["bytes32", "bytes32", "uint48", "uint48"],
            [userOpHash, tenantIdBytes, validUntil, validAfter]
        );
        const messageHash = ethers.hashMessage(ethers.getBytes(ethers.keccak256(message)));

        // Sign
        const signature = await this.signer.signMessage(
            ethers.getBytes(ethers.keccak256(message))
        );

        // Pack paymaster data
        const paymasterAndData = ethers.solidityPacked(
            ["address", "bytes32", "uint48", "uint48", "bytes"],
            [this.paymasterAddress, tenantIdBytes, validUntil, validAfter, signature]
        );

        return paymasterAndData;
    }
}
```

### Frontend Integration

```typescript
import { createSmartAccountClient } from "@account-abstraction/sdk";

const smartAccountClient = await createSmartAccountClient({
    signer: userSigner,
    entryPointAddress: ENTRY_POINT_ADDRESS,
    bundlerUrl: BUNDLER_URL,
    paymasterMiddleware: async (userOp) => {
        // Request sponsorship from backend
        const response = await fetch("/api/sponsor", {
            method: "POST",
            body: JSON.stringify({
                userOpHash: getUserOpHash(userOp),
                tenantId: "my-app"
            })
        });

        const { paymasterAndData } = await response.json();
        return { paymasterAndData };
    }
});

// Execute sponsored transaction
await smartAccountClient.sendTransaction({
    to: recipientAddress,
    value: ethers.parseEther("1.0")
});
```

## Admin Functions

### Update Signer

```solidity
function setSigner(address _signer) external onlyOwner;
```

**Example:**
```solidity
paymaster.setSigner(newSignerAddress);
// Emits: SignerUpdated(oldSigner, newSigner)
```

### Manage Deposits

The paymaster must maintain a deposit in the EntryPoint to pay for gas:

```solidity
// Deposit ETH to EntryPoint
function deposit() external payable;

// Withdraw ETH from EntryPoint
function withdrawTo(address payable to, uint256 amount) external onlyOwner;

// Check current deposit
function getDeposit() external view returns (uint256);
```

**Example:**
```solidity
// Deposit 10 ETH for gas payments
paymaster.deposit{value: 10 ether}();

// Check balance
uint256 balance = paymaster.getDeposit();

// Withdraw excess
paymaster.withdrawTo(treasuryAddress, 5 ether);
```

## Events

```solidity
event SignerUpdated(address indexed oldSigner, address indexed newSigner);
event TenantLimitSet(bytes32 indexed tenantId, uint256 limit);
event Sponsored(address indexed sender, bytes32 indexed tenantId, uint256 gasCost);
```

## Errors

```solidity
error InvalidSignature();       // Paymaster data signature invalid
error TenantLimitExceeded();    // Tenant daily limit reached
error UserRateLimitExceeded();  // User has too many ops today
error PaymasterDepositTooLow(); // Insufficient EntryPoint deposit
```

## Security Considerations

### Signature Security

- The `verifyingSigner` private key must be stored securely (HSM/KMS)
- Use short validity windows (5-15 minutes) to limit replay window
- Rotate signer keys periodically

### Limit Recommendations

| Tenant Type | Daily Limit | User Rate Limit |
|-------------|-------------|-----------------|
| Free tier | 0.1 ETH | 10 ops/day |
| Starter | 1 ETH | 50 ops/day |
| Pro | 10 ETH | 200 ops/day |
| Enterprise | Custom | Custom |

### Monitoring

Monitor these metrics:
- Daily spend per tenant
- User operation counts
- Signature validation failures
- EntryPoint deposit balance

## Testing

See [RampOSPaymaster.t.sol](../../contracts/test/RampOSPaymaster.t.sol) for test examples:

```solidity
function test_ValidateUserOp() public {
    // Create valid paymaster signature
    bytes32 hash = keccak256(
        abi.encodePacked(userOpHash, tenantId, validUntil, validAfter)
    ).toEthSignedMessageHash();

    (uint8 v, bytes32 r, bytes32 s) = vm.sign(signerKey, hash);
    bytes memory signature = abi.encodePacked(r, s, v);

    // Construct paymasterAndData
    bytes memory paymasterAndData = abi.encodePacked(
        address(paymaster),
        tenantId,
        validUntil,
        validAfter,
        signature
    );
    userOp.paymasterAndData = paymasterAndData;

    // Validate
    vm.prank(address(entryPoint));
    (bytes memory context, uint256 validationData) = paymaster.validatePaymasterUserOp(
        userOp, userOpHash, 1e18
    );

    assertEq(validationData & 1, 0); // Success
}
```

## Related Documentation

- [Overview](./overview.md) - Contract architecture
- [Account](./account.md) - RampOSAccount features
- [Security](./security.md) - Security model
