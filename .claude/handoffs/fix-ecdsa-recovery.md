# Handoff: Fix ECDSA Recovery ID in Paymaster

## Summary

Fixed the ECDSA signing implementation in `crates/ramp-aa/src/paymaster.rs` to correctly compute the recovery ID (v) instead of hardcoding it to 27.

## Changes Made

### 1. Fixed `sign_paymaster_data` function

**Before:**
```rust
// Sign using ECDSA
let signature: Signature = self.signing_key.sign(&eth_signed_hash);

// Calculate recovery id (v)
// For Ethereum: v = recovery_id + 27
// We use a simple approach here - in production you'd compute this properly
let v: u8 = 27; // Simplified - real implementation needs recovery computation
```

**After:**
```rust
// Sign using ECDSA with recoverable signature
// sign_prehash_recoverable returns (Signature, RecoveryId)
let (signature, recovery_id): (Signature, RecoveryId) = self
    .signing_key
    .sign_prehash_recoverable(&eth_signed_hash)
    .expect("signing should not fail with valid key");

// Calculate v: For Ethereum legacy format, v = 27 + recovery_id (0 or 1)
// recovery_id.to_byte() returns 0 or 1
let v: u8 = 27 + recovery_id.to_byte();
```

### 2. Added `recover_signer` function

New function that can recover the Ethereum address from a signature. This is useful for:
- Testing that signatures can be verified on-chain via `ECDSA.recover()`
- Verifying signature validity in off-chain scenarios

```rust
pub fn recover_signer(
    message_hash: &[u8; 32],
    signature: &[u8; 65],
) -> std::result::Result<Address, String>
```

### 3. Added `signer_address` function

Returns the Ethereum address derived from the signing key:

```rust
pub fn signer_address(&self) -> Address
```

### 4. Updated imports

Added necessary imports:
```rust
use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use k256::elliptic_curve::sec1::ToEncodedPoint;
```

## Files Modified

- `crates/ramp-aa/src/paymaster.rs` - Main implementation
- `crates/ramp-aa/Cargo.toml` - Added dev-dependencies for tests

## Tests Added

8 comprehensive unit tests were added:

1. `test_sign_and_recover_address` - Verifies signature can be used to recover signer address
2. `test_recovery_id_is_correct` - Tests multiple signatures to ensure v is always 27 or 28
3. `test_signer_address_derivation` - Verifies address derivation is deterministic
4. `test_different_keys_produce_different_addresses` - Ensures different keys produce different addresses
5. `test_signature_format_compatibility` - Validates 65-byte signature format (r || s || v)
6. `test_recover_signer_with_invalid_v` - Tests error handling for invalid v values
7. `test_recover_signer_with_invalid_signature` - Tests error handling for invalid signatures
8. `test_sponsor_produces_valid_signature` - Integration test with `sponsor()` function

## Test Results

```
running 8 tests
test paymaster::tests::test_recover_signer_with_invalid_v ... ok
test paymaster::tests::test_recover_signer_with_invalid_signature ... ok
test paymaster::tests::test_different_keys_produce_different_addresses ... ok
test paymaster::tests::test_signer_address_derivation ... ok
test paymaster::tests::test_signature_format_compatibility ... ok
test paymaster::tests::test_sponsor_produces_valid_signature ... ok
test paymaster::tests::test_sign_and_recover_address ... ok
test paymaster::tests::test_recovery_id_is_correct ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out
```

## Technical Details

### ECDSA Recovery ID

The recovery ID is a value (0 or 1) that indicates which of the two possible public keys should be recovered from an ECDSA signature. In Ethereum:

- **Legacy format**: `v = 27 + recovery_id`
- **EIP-155 format**: `v = 35 + chain_id * 2 + recovery_id`

This implementation uses the legacy format (v = 27 or 28) which is compatible with ERC-4337 paymaster signatures.

### On-Chain Verification

The signature format produced is compatible with Solidity's `ECDSA.recover()`:

```solidity
address signer = ECDSA.recover(messageHash, signature);
```

Where `signature` is the 65-byte value `r || s || v`.

## Acceptance Criteria Met

- [x] Recovery ID (v) is correctly computed from signature (0 or 1)
- [x] v = 27 + recovery_id for legacy Ethereum format
- [x] Uses k256 crate with `sign_prehash_recoverable` method
- [x] Signature can be verified by on-chain ECDSA.recover
- [x] Unit tests added for signing logic

## Next Steps

For production deployment:
1. Consider adding EIP-155 support if chain-specific signatures are needed
2. Add integration tests with actual on-chain paymaster contract
3. Consider S-value normalization for strict EIP-2 compliance (low-S form)
