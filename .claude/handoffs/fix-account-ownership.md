# Handoff: Account Ownership Verification Implementation

## Task Summary
Implemented proper account ownership verification for the Account Abstraction (AA) API endpoints, including both tenant-level and user-level ownership verification.

## Latest Changes (User-Level Ownership Verification)

### 1. Added User-Level Ownership Verification to Repository Trait

**File:** `crates/ramp-core/src/repository/smart_account.rs`

Added new trait method `verify_user_ownership` to `SmartAccountRepository` trait:

```rust
/// Check if an account address belongs to a specific user within a tenant
/// This provides user-level access control in addition to tenant-level verification
async fn verify_user_ownership(
    &self,
    tenant_id: &TenantId,
    user_id: &str,
    address: &str,
    chain_id: u64,
) -> Result<bool>;
```

Implemented the method in `PgSmartAccountRepository` with SQL query that checks:
- Address matches (case-insensitive)
- Tenant ID matches
- User ID matches
- Chain ID matches
- Account status is 'ACTIVE'

### 2. Added User Ownership Verification Helper Function

**File:** `crates/ramp-api/src/handlers/aa.rs`

Added new function `verify_account_user_ownership` (lines 670-750) that:
- Rejects zero addresses
- Rejects empty user IDs
- Queries the database via `SmartAccountRepository::verify_user_ownership`
- Fails closed on database errors (returns false)
- Logs security warnings for failed verifications
- Returns false in production if repository not configured
- Has permissive fallback for non-production environments without repository (for migration)

### 3. Updated Mock Repository for Testing

Enhanced the mock repository in tests to:
- Track user_id in addition to tenant_id and chain_id
- Implement `verify_user_ownership` method
- Added `add_account_with_user` method for test setup
- Maintained backward compatibility with `add_account` method

### 4. Added Comprehensive Tests for User Ownership

Added 6 new tests for user ownership verification:

1. `test_verify_user_ownership_zero_address` - Rejects zero addresses
2. `test_verify_user_ownership_empty_user_id` - Rejects empty user IDs
3. `test_verify_user_ownership_with_mock_repo` - Verifies correct user has access, wrong user denied
4. `test_verify_user_ownership_different_tenant` - Denies access for same user in different tenant
5. `test_verify_user_ownership_chain_mismatch` - Denies access when chain ID doesn't match
6. `test_verify_user_ownership_unknown_address` - Denies access for unknown addresses

---

## Previous Changes

### Fixed Test Key for PaymasterService (`crates/ramp-api/src/handlers/aa.rs`)
- Changed the fallback test key from all zeros to a valid secp256k1 private key (scalar 1)
- All zeros is not a valid private key and was causing test failures

### Fixed ChainConfig Usage in Tests (`crates/ramp-api/src/handlers/aa.rs`)
- Updated 5 test functions to use the correct `ChainConfig` struct fields
- Changed `rpc_url` to `name` and `bundler_url: Option<String>` to `bundler_url: String`

### Wired Up AA Service in Main (`crates/ramp-api/src/main.rs`)
- Added imports for `AAServiceState`, `ChainConfig`, and `PgSmartAccountRepository`
- Created AA service initialization with SmartAccountRepository for ownership verification
- AA service is controlled by `AA_ENABLED=true` environment variable
- Configurable via environment variables:
  - `AA_ENABLED` - Enable/disable AA service (default: false)
  - `AA_CHAIN_ID` - Chain ID (default: 1)
  - `AA_CHAIN_NAME` - Chain name (default: "Ethereum Mainnet")
  - `AA_BUNDLER_URL` - Bundler URL (default: "http://localhost:4337")
  - `AA_ENTRY_POINT_ADDRESS` - Entry point contract (default: ERC-4337 v0.6)
  - `AA_PAYMASTER_ADDRESS` - Optional paymaster address

### Fixed Unrelated Issues
- Added missing `Datelike` import in `crates/ramp-core/src/test_utils.rs`
- Added missing trait methods to `MockIntentRepository` in `crates/ramp-api/tests/security_tests.rs`

---

## Existing Implementation (Already Complete)

### Migration File (`migrations/010_smart_accounts.sql`)
- Table `smart_accounts` with proper schema
- Indexes for efficient lookups
- RLS policies for tenant isolation
- Unique constraint on (address, chain_id)

### Repository (`crates/ramp-core/src/repository/smart_account.rs`)
- `SmartAccountRepository` trait with all methods
- `PgSmartAccountRepository` PostgreSQL implementation
- `verify_ownership()` - Checks if account belongs to tenant
- `verify_user_ownership()` - Checks if account belongs to specific user within tenant
- `get_by_address()` - Get account by address and chain
- `create()` - Create new smart account record with upsert

### Handler (`crates/ramp-api/src/handlers/aa.rs`)
- `verify_account_ownership()` - Queries database to verify tenant ownership
- `verify_account_user_ownership()` - Queries database to verify user ownership
- `create_account()` - Saves smart account mapping to database
- Security: Returns 403 Forbidden if account doesn't belong to tenant
- Security: Fails closed in production if repository is unavailable

---

## Test Results

All 14 AA tests pass:
```
running 14 tests
test handlers::aa::tests::test_aa_service_state_creation ... ok
test handlers::aa::tests::test_convert_user_op_to_dto ... ok
test handlers::aa::tests::test_create_smart_account_request ... ok
test handlers::aa::tests::test_hex_to_bytes ... ok
test handlers::aa::tests::test_verify_account_ownership_chain_mismatch ... ok
test handlers::aa::tests::test_verify_account_ownership_unknown_address ... ok
test handlers::aa::tests::test_verify_account_ownership_with_mock_repo ... ok
test handlers::aa::tests::test_verify_account_ownership_zero_address ... ok
test handlers::aa::tests::test_verify_user_ownership_chain_mismatch ... ok
test handlers::aa::tests::test_verify_user_ownership_different_tenant ... ok
test handlers::aa::tests::test_verify_user_ownership_empty_user_id ... ok
test handlers::aa::tests::test_verify_user_ownership_unknown_address ... ok
test handlers::aa::tests::test_verify_user_ownership_with_mock_repo ... ok
test handlers::aa::tests::test_verify_user_ownership_zero_address ... ok

test result: ok. 14 passed; 0 failed; 0 ignored
```

All 2 smart_account repository tests pass:
- `test_address_normalization`
- `test_create_request`

---

## Security Considerations

1. **Fail Closed**: In production (`RAMPOS_ENV=production`), if the SmartAccountRepository is not configured, access is denied.

2. **Chain ID Validation**: Ownership is verified against the specific chain ID, preventing cross-chain attacks.

3. **Address Normalization**: All addresses are normalized to lowercase for consistent comparison.

4. **Logging**: Failed ownership verifications are logged with warnings for security monitoring.

5. **User-Level Isolation**: Within a tenant, users can only access their own accounts.

6. **Empty User ID Rejection**: Empty user IDs are explicitly rejected to prevent bypass attacks.

---

## Files Modified

| File | Change |
|------|--------|
| `crates/ramp-core/src/repository/smart_account.rs` | Added `verify_user_ownership` trait method and PostgreSQL implementation |
| `crates/ramp-api/src/handlers/aa.rs` | Added `verify_account_user_ownership` function, updated mock repository, added 6 new tests |
| `crates/ramp-api/src/main.rs` | Added AA service wiring with SmartAccountRepository |
| `crates/ramp-core/src/test_utils.rs` | Added Datelike import |
| `crates/ramp-api/tests/security_tests.rs` | Added missing trait methods |

---

## Recommended Next Steps

1. **Add User Context to Auth Middleware**: The `TenantContext` struct in `middleware/tenant.rs` should be extended to include `user_id` from the authentication token/claims.

2. **Integrate User Verification in Handlers**: Once user context is available, update `send_user_operation` handler to call `verify_account_user_ownership` to ensure only the account owner can send transactions.

3. **Consider API Design**: Some operations (like `get_account`) may need both tenant-level (for admins) and user-level (for users) access patterns.

---

## Environment Variables for AA Service

```bash
# Enable AA service
AA_ENABLED=true

# Chain configuration
AA_CHAIN_ID=1
AA_CHAIN_NAME="Ethereum Mainnet"
AA_BUNDLER_URL="https://your-bundler.example.com"
AA_ENTRY_POINT_ADDRESS="0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
AA_PAYMASTER_ADDRESS="0x..."  # Optional

# Paymaster signer key (required in production)
PAYMASTER_SIGNER_KEY="0x..."  # 64 hex chars
```

---

## Verification Commands

```bash
# Run AA handler tests
cargo test -p ramp-api --lib handlers::aa

# Run smart account repository tests
cargo test -p ramp-core --lib repository::smart_account

# Check full workspace compiles
cargo check --workspace
```
