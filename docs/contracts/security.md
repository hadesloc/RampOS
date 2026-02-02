# RampOS Smart Contracts - Security Documentation

## Security Model Overview

RampOS smart contracts implement multiple layers of security to protect user funds and ensure system integrity.

```
+-------------------+
|   Application     |
+-------------------+
         |
+-------------------+
|   Access Control  |  <- Owner/EntryPoint/Session Keys
+-------------------+
         |
+-------------------+
|   Signature       |  <- ECDSA verification
|   Verification    |
+-------------------+
         |
+-------------------+
|   Rate Limiting   |  <- Per-tenant, per-user limits
+-------------------+
         |
+-------------------+
|   ERC-4337        |  <- EntryPoint validation
|   Compliance      |
+-------------------+
```

## Access Control

### RampOSAccount Access

| Function | Owner | EntryPoint | Session Key | Anyone |
|----------|-------|------------|-------------|--------|
| `execute` | Direct | Via UserOp | Via UserOp | No |
| `executeBatch` | Direct | Via UserOp | Via UserOp | No |
| `addSessionKey` | Yes | No | No | No |
| `removeSessionKey` | Yes | No | No | No |
| `upgradeToAndCall` | Yes | No | No | No |

### RampOSPaymaster Access

| Function | Owner | EntryPoint | Anyone |
|----------|-------|------------|--------|
| `validatePaymasterUserOp` | No | Yes | No |
| `postOp` | No | Yes | No |
| `setSigner` | Yes | No | No |
| `setTenantLimit` | Yes | No | No |
| `setMaxOpsPerUser` | Yes | No | No |
| `deposit` | Yes | Yes | Yes |
| `withdrawTo` | Yes | No | No |

### Access Control Implementation

```solidity
// RampOSAccount modifiers
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

## Signature Verification

### UserOperation Signature

The account validates signatures using ECDSA with EIP-191 message hashing:

```solidity
function _validateSignature(
    PackedUserOperation calldata userOp,
    bytes32 userOpHash
) internal virtual override returns (uint256 validationData) {
    // Create EIP-191 signed message hash
    bytes32 hash = userOpHash.toEthSignedMessageHash();

    // Recover signer address
    address signer = hash.recover(userOp.signature);

    // Validate against owner or session key
    if (signer == owner) {
        return 0; // Valid, no time restrictions
    }

    SessionKey memory session = sessionKeys[signer];
    if (session.key != address(0)) {
        // Validate time bounds
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

### Paymaster Signature

The paymaster verifies backend signatures for sponsorship authorization:

```solidity
bytes32 hash = keccak256(
    abi.encodePacked(
        userOpHash,
        tenantId,
        validUntil,
        validAfter
    )
).toEthSignedMessageHash();

if (hash.recover(signature) != verifyingSigner) {
    revert InvalidSignature();
}
```

## Attack Vectors and Mitigations

### 1. Signature Replay Attack

**Risk**: Reusing a valid signature for unauthorized transactions.

**Mitigations**:
- UserOp signatures include nonce (managed by EntryPoint)
- Paymaster signatures include time bounds (`validAfter`, `validUntil`)
- Each signature is bound to specific `userOpHash`

```solidity
// Nonce prevents replay
uint256 nonce = entryPoint.getNonce(sender, key);

// Time bounds prevent indefinite reuse
uint48 validUntil = uint48(block.timestamp + 5 minutes);
```

### 2. Session Key Abuse

**Risk**: Compromised session key could drain the account.

**Mitigations**:
- Time-limited validity (`validAfter`, `validUntil`)
- Owner can revoke session keys immediately
- Future: Permission-scoped session keys

**Best Practices**:
```solidity
// Use short validity periods
uint48 validUntil = uint48(block.timestamp + 1 hours);

// Revoke immediately if compromised
account.removeSessionKey(compromisedKey);
```

### 3. Paymaster Drain Attack

**Risk**: Attacker exhausts paymaster deposit with sponsored transactions.

**Mitigations**:
- Per-tenant daily spending limits
- Per-user rate limiting (max ops/day)
- Backend signature required for each sponsorship
- Backend can implement additional checks before signing

```solidity
// Tenant limit check
if (tenantDailySpent[tenantId] + cost > tenantDailyLimit[tenantId]) {
    revert TenantLimitExceeded();
}

// User rate limit check
if (userDailyOps[user] >= maxOpsPerUserPerDay) {
    revert UserRateLimitExceeded();
}
```

### 4. Upgrade Attacks

**Risk**: Malicious upgrade replaces contract logic.

**Mitigations**:
- Only owner can authorize upgrades
- UUPS pattern requires explicit authorization
- Consider timelock for upgrades in production

```solidity
function _authorizeUpgrade(
    address newImplementation
) internal override onlyOwner {}
```

### 5. Reentrancy

**Risk**: Malicious contracts calling back during execution.

**Mitigations**:
- External calls are made via low-level `call` with proper error handling
- State changes occur before external calls where applicable
- EntryPoint handles execution atomically

```solidity
function _call(address target, uint256 value, bytes memory data) internal {
    (bool success, bytes memory result) = target.call{value: value}(data);
    if (!success) {
        assembly {
            revert(add(result, 32), mload(result))
        }
    }
}
```

### 6. Front-Running

**Risk**: Attackers observe pending transactions and front-run them.

**Mitigations**:
- UserOperations go through bundlers, not public mempool
- Bundlers implement private mempools
- Nonce ordering prevents out-of-order execution

## Audit Findings

### Audit Status

| Item | Status | Notes |
|------|--------|-------|
| Internal Review | Completed | No critical issues |
| External Audit | Pending | Recommended before mainnet |
| Formal Verification | Not started | Consider for v2 |

### Known Limitations

1. **Session Key Permissions**: Currently, session keys have full account access within time bounds. Future versions should implement permission scoping.

2. **Single Owner**: No multi-sig support currently. Owner key compromise = account compromise.

3. **No Social Recovery**: Account recovery relies solely on owner key. Consider adding guardian-based recovery.

4. **Rate Limit Bypass**: A determined attacker could create multiple accounts to bypass per-user rate limits.

## Best Practices

### For Account Owners

1. **Secure Your Owner Key**
   - Use hardware wallets for owner keys
   - Never share or expose private keys
   - Consider multi-sig for high-value accounts

2. **Session Key Hygiene**
   - Use shortest practical validity periods
   - Revoke unused session keys immediately
   - Monitor session key activity

3. **Upgrade Carefully**
   - Verify new implementation addresses
   - Test upgrades on testnet first
   - Consider timelock for production

### For Integrators

1. **Backend Security**
   - Store `verifyingSigner` key in HSM/KMS
   - Implement proper authentication before signing
   - Log all sponsorship requests
   - Set conservative tenant limits

2. **Request Validation**
   ```typescript
   async function validateSponsorshipRequest(request) {
       // Check user is authenticated
       if (!request.user.isAuthenticated) throw new Error("Unauthorized");

       // Check tenant is active
       if (!request.tenant.isActive) throw new Error("Tenant inactive");

       // Check user isn't rate limited
       const userOps = await getUserDailyOps(request.user.address);
       if (userOps >= MAX_OPS_PER_USER) throw new Error("Rate limited");

       // Check tenant has budget
       const tenantSpent = await getTenantDailySpent(request.tenant.id);
       if (tenantSpent + request.maxCost > request.tenant.limit) {
           throw new Error("Tenant limit exceeded");
       }

       return true;
   }
   ```

3. **Monitoring**
   - Alert on unusual spending patterns
   - Monitor signature validation failures
   - Track EntryPoint deposit balance
   - Set up anomaly detection

### For Operators

1. **Deployment**
   - Verify contract deployments match expected bytecode
   - Use deterministic deployment for consistent addresses
   - Document all deployment parameters

2. **Key Management**
   - Rotate `verifyingSigner` periodically
   - Implement key rotation without downtime
   - Maintain secure backup of keys

3. **Incident Response**
   ```
   If Paymaster Signer Compromised:
   1. Immediately call setSigner() with new signer
   2. All pending signatures become invalid
   3. Investigate unauthorized sponsorships
   4. Consider withdrawing paymaster deposit temporarily

   If Account Owner Key Compromised:
   1. Revoke all session keys immediately
   2. Transfer assets to new account
   3. Consider upgrade to invalidate old owner (if supported)
   ```

## Security Checklist

### Pre-Deployment

- [ ] All tests pass
- [ ] Code coverage > 90%
- [ ] Static analysis (Slither) shows no critical issues
- [ ] External audit completed
- [ ] Emergency procedures documented
- [ ] Key management procedures established

### Post-Deployment

- [ ] Deployment verified on block explorer
- [ ] Monitoring and alerting configured
- [ ] Deposit topped up in EntryPoint
- [ ] Tenant limits configured
- [ ] Rate limits configured
- [ ] Documentation updated with addresses

## Emergency Procedures

### Pause Sponsorship

If abuse is detected, operators can effectively pause sponsorship by:

1. Set all tenant limits to 0
2. Or change signer to an address with no corresponding private key

```solidity
// Option 1: Zero limits
paymaster.setTenantLimit(tenantId, 0);

// Option 2: Invalid signer (no one can sign)
paymaster.setSigner(address(0xdead));
```

### Withdraw Funds

In emergency, withdraw paymaster deposit:

```solidity
uint256 balance = paymaster.getDeposit();
paymaster.withdrawTo(safeAddress, balance);
```

## Further Reading

- [ERC-4337 Specification](https://eips.ethereum.org/EIPS/eip-4337)
- [OpenZeppelin Security](https://docs.openzeppelin.com/contracts/5.x/)
- [Account Abstraction Security Best Practices](https://docs.alchemy.com/docs/account-abstraction-overview)

## Related Documentation

- [Overview](./overview.md) - Contract architecture
- [Account](./account.md) - RampOSAccount features
- [Paymaster](./paymaster.md) - Gas sponsorship
