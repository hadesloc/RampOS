# Wallet Integration Guide (ERC-4337)

This guide explains how to integrate ERC-4337 Account Abstraction wallets with RampOS. The system supports smart accounts, session keys, and gas sponsorship through paymasters.

## Overview

RampOS provides a complete ERC-4337 integration layer:

- **SmartAccountService**: Manages smart account creation and operations
- **UserOperation**: ERC-4337 user operation builder
- **Paymaster**: Gas sponsorship with tenant limits
- **Session Keys**: Delegated signing with permissions

## Architecture

```
                        RampOS AA Layer
                              |
          +-------------------+-------------------+
          |                   |                   |
   SmartAccount         UserOperation        Paymaster
     Service              Builder            Service
          |                   |                   |
    +-----+-----+             |             +-----+-----+
    |           |             |             |           |
 Factory    Account      EntryPoint      Verifying   Limits
 Contract   Contract     v0.6/v0.7      Signature   Manager
```

## Smart Account Types

RampOS supports multiple smart account implementations:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmartAccountType {
    SimpleAccount,    // Basic ERC-4337 account
    SafeAccount,      // Safe (Gnosis) based
    KernelAccount,    // ZeroDev Kernel
    BiconomyAccount,  // Biconomy
}
```

## Chain Configuration

```rust
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub name: String,
    pub entry_point_address: Address,
    pub bundler_url: String,
    pub paymaster_address: Option<Address>,
}

impl ChainConfig {
    pub fn ethereum_mainnet() -> Self {
        Self {
            chain_id: 1,
            name: "Ethereum Mainnet".to_string(),
            entry_point_address: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                .parse()
                .unwrap(),
            bundler_url: "https://bundler.example.com".to_string(),
            paymaster_address: None,
        }
    }

    pub fn polygon_mainnet() -> Self {
        Self {
            chain_id: 137,
            name: "Polygon Mainnet".to_string(),
            entry_point_address: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                .parse()
                .unwrap(),
            bundler_url: "https://bundler.polygon.example.com".to_string(),
            paymaster_address: None,
        }
    }

    pub fn bnb_chain() -> Self {
        Self {
            chain_id: 56,
            name: "BNB Chain".to_string(),
            entry_point_address: "0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789"
                .parse()
                .unwrap(),
            bundler_url: "https://bundler.bnb.example.com".to_string(),
            paymaster_address: None,
        }
    }
}
```

## Smart Account Service

### Creating and Managing Accounts

```rust
use ethers::types::{Address, U256};
use ramp_aa::smart_account::SmartAccountService;
use ramp_common::types::{TenantId, UserId};

// Initialize service
let service = SmartAccountService::new(
    chain_config.chain_id,
    factory_address,
    entry_point_address,
);

// Get or create smart account for a user
let account = service.get_or_create_account(
    &TenantId::new("tenant_1"),
    &UserId::new("user_1"),
    owner_address, // EOA owner
).await?;

println!("Account address: {}", account.address);
println!("Is deployed: {}", account.is_deployed);
println!("Current nonce: {}", account.nonce);
```

### Smart Account Data Structure

```rust
#[derive(Debug, Clone)]
pub struct SmartAccount {
    pub address: Address,
    pub owner: Address,
    pub account_type: SmartAccountType,
    pub is_deployed: bool,
    pub nonce: U256,
}
```

### Computing Counterfactual Addresses

The account address is deterministic using CREATE2:

```rust
impl SmartAccountService {
    /// Compute deterministic salt from tenant and user
    fn compute_salt(&self, tenant_id: &TenantId, user_id: &UserId) -> U256 {
        use ethers::utils::keccak256;

        let data = format!("{}:{}", tenant_id.0, user_id.0);
        let hash = keccak256(data.as_bytes());
        U256::from_big_endian(&hash)
    }

    /// Compute counterfactual address using CREATE2
    fn compute_address(&self, owner: Address, salt: U256) -> Result<Address> {
        use ethers::utils::keccak256;

        // CREATE2: keccak256(0xff ++ factory ++ salt ++ init_code_hash)[12:]
        let init_code_hash = keccak256(&self.get_init_code_hash());

        let mut data = Vec::with_capacity(85);
        data.push(0xff);
        data.extend_from_slice(self.factory_address.as_bytes());

        let mut salt_bytes = [0u8; 32];
        salt.to_big_endian(&mut salt_bytes);
        data.extend_from_slice(&salt_bytes);
        data.extend_from_slice(&init_code_hash);

        let hash = keccak256(&data);
        Ok(Address::from_slice(&hash[12..]))
    }
}
```

## UserOperation

### Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperation {
    /// The account making the operation
    pub sender: Address,

    /// Anti-replay parameter
    pub nonce: U256,

    /// Account init code (only for first operation)
    pub init_code: Bytes,

    /// The call data to execute on the account
    pub call_data: Bytes,

    /// Gas limit for the account's call
    pub call_gas_limit: U256,

    /// Gas limit for account verification
    pub verification_gas_limit: U256,

    /// Gas paid upfront for verification/execution overhead
    pub pre_verification_gas: U256,

    /// Maximum fee per gas (EIP-1559)
    pub max_fee_per_gas: U256,

    /// Maximum priority fee per gas (EIP-1559)
    pub max_priority_fee_per_gas: U256,

    /// Paymaster data (if sponsored)
    pub paymaster_and_data: Bytes,

    /// Account signature
    pub signature: Bytes,
}
```

### Building Operations

```rust
impl UserOperation {
    /// Create a new UserOperation
    pub fn new(sender: Address, nonce: U256, call_data: Bytes) -> Self {
        Self {
            sender,
            nonce,
            init_code: Bytes::default(),
            call_data,
            call_gas_limit: U256::from(100_000),
            verification_gas_limit: U256::from(100_000),
            pre_verification_gas: U256::from(21_000),
            max_fee_per_gas: U256::from(1_000_000_000), // 1 gwei
            max_priority_fee_per_gas: U256::from(1_000_000_000),
            paymaster_and_data: Bytes::default(),
            signature: Bytes::default(),
        }
    }

    /// Set gas parameters
    pub fn with_gas(
        mut self,
        call_gas: U256,
        verification_gas: U256,
        pre_verification: U256,
    ) -> Self {
        self.call_gas_limit = call_gas;
        self.verification_gas_limit = verification_gas;
        self.pre_verification_gas = pre_verification;
        self
    }

    /// Set fee parameters
    pub fn with_fees(mut self, max_fee: U256, max_priority_fee: U256) -> Self {
        self.max_fee_per_gas = max_fee;
        self.max_priority_fee_per_gas = max_priority_fee;
        self
    }

    /// Set paymaster data for sponsored transactions
    pub fn with_paymaster(mut self, paymaster_and_data: Bytes) -> Self {
        self.paymaster_and_data = paymaster_and_data;
        self
    }

    /// Set signature
    pub fn with_signature(mut self, signature: Bytes) -> Self {
        self.signature = signature;
        self
    }

    /// Set init code for account creation
    pub fn with_init_code(mut self, init_code: Bytes) -> Self {
        self.init_code = init_code;
        self
    }

    /// Calculate hash for signing
    pub fn hash(&self, entry_point: Address, chain_id: u64) -> H256 {
        use ethers::abi::{encode, Token};
        use ethers::utils::keccak256;

        let packed = encode(&[
            Token::Address(self.sender),
            Token::Uint(self.nonce),
            Token::Bytes(keccak256(&self.init_code).to_vec()),
            Token::Bytes(keccak256(&self.call_data).to_vec()),
            Token::Uint(self.call_gas_limit),
            Token::Uint(self.verification_gas_limit),
            Token::Uint(self.pre_verification_gas),
            Token::Uint(self.max_fee_per_gas),
            Token::Uint(self.max_priority_fee_per_gas),
            Token::Bytes(keccak256(&self.paymaster_and_data).to_vec()),
        ]);

        let user_op_hash = keccak256(&packed);

        let final_hash = encode(&[
            Token::FixedBytes(user_op_hash.to_vec()),
            Token::Address(entry_point),
            Token::Uint(U256::from(chain_id)),
        ]);

        H256::from_slice(&keccak256(&final_hash))
    }
}
```

### Building Common Operations

```rust
impl SmartAccountService {
    /// Build UserOperation for account creation
    pub fn build_create_account_op(
        &self,
        account: &SmartAccount,
        owner: Address,
        salt: U256,
    ) -> Result<UserOperation> {
        let init_code = self.build_init_code(owner, salt)?;
        let call_data = Bytes::default();

        let mut op = UserOperation::new(account.address, U256::zero(), call_data);
        op = op.with_init_code(init_code);

        Ok(op)
    }

    /// Build UserOperation for a token transfer
    pub fn build_transfer_op(
        &self,
        account: &SmartAccount,
        to: Address,
        value: U256,
        data: Option<Bytes>,
    ) -> Result<UserOperation> {
        use ethers::abi::{encode, Token};

        // Build execute(address,uint256,bytes) call
        let selector = [0xb6, 0x1d, 0x27, 0xf6]; // keccak256("execute(address,uint256,bytes)")[:4]

        let mut call_data = Vec::new();
        call_data.extend_from_slice(&selector);

        let params = encode(&[
            Token::Address(to),
            Token::Uint(value),
            Token::Bytes(data.unwrap_or_default().to_vec()),
        ]);
        call_data.extend_from_slice(&params);

        Ok(UserOperation::new(
            account.address,
            account.nonce,
            Bytes::from(call_data),
        ))
    }

    /// Build UserOperation for batch execution
    pub fn build_batch_op(
        &self,
        account: &SmartAccount,
        calls: Vec<(Address, U256, Bytes)>,
    ) -> Result<UserOperation> {
        use ethers::abi::{encode, Token};

        // Build executeBatch(address[],uint256[],bytes[]) call
        let selector = [0x34, 0xfc, 0xd5, 0xbe];

        let targets: Vec<Token> = calls.iter()
            .map(|(t, _, _)| Token::Address(*t))
            .collect();
        let values: Vec<Token> = calls.iter()
            .map(|(_, v, _)| Token::Uint(*v))
            .collect();
        let datas: Vec<Token> = calls.iter()
            .map(|(_, _, d)| Token::Bytes(d.to_vec()))
            .collect();

        let mut call_data = Vec::new();
        call_data.extend_from_slice(&selector);

        let params = encode(&[
            Token::Array(targets),
            Token::Array(values),
            Token::Array(datas),
        ]);
        call_data.extend_from_slice(&params);

        Ok(UserOperation::new(
            account.address,
            account.nonce,
            Bytes::from(call_data),
        ))
    }
}
```

## Session Keys

Session keys allow delegated signing with scoped permissions.

### Session Key Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionKey {
    pub key_address: Address,
    pub valid_until: u64,
    pub valid_after: u64,
    pub permissions: Vec<SessionPermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPermission {
    pub target: Address,
    pub selector: [u8; 4],
    pub max_value: U256,
    pub rules: Vec<PermissionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionRule {
    MaxAmount(U256),
    AllowedRecipients(Vec<Address>),
    TimeWindow { start: u64, end: u64 },
    RateLimit { count: u32, period_secs: u64 },
}
```

### Smart Contract Session Key Implementation

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract RampOSAccount is BaseAccount, Initializable, UUPSUpgradeable {
    using ECDSA for bytes32;

    address public owner;
    IEntryPoint private immutable _entryPoint;

    struct SessionKey {
        address key;
        uint48 validAfter;
        uint48 validUntil;
        bytes32 permissionsHash;
    }

    mapping(address => SessionKey) public sessionKeys;

    event SessionKeyAdded(address indexed key, uint48 validUntil);
    event SessionKeyRemoved(address indexed key);

    /// @notice Add a session key
    function addSessionKey(
        address key,
        uint48 validAfter,
        uint48 validUntil,
        bytes32 permissionsHash
    ) external onlyOwner {
        sessionKeys[key] = SessionKey({
            key: key,
            validAfter: validAfter,
            validUntil: validUntil,
            permissionsHash: permissionsHash
        });

        emit SessionKeyAdded(key, validUntil);
    }

    /// @notice Remove a session key
    function removeSessionKey(address key) external onlyOwner {
        delete sessionKeys[key];
        emit SessionKeyRemoved(key);
    }

    /// @notice Check if a session key is valid
    function isValidSessionKey(address key) public view returns (bool) {
        SessionKey memory session = sessionKeys[key];
        if (session.key == address(0)) return false;
        if (block.timestamp < session.validAfter) return false;
        if (block.timestamp > session.validUntil) return false;
        return true;
    }

    /// @notice Validate user operation signature
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
            return _packValidationData(
                false,
                session.validUntil,
                session.validAfter
            );
        }

        return SIG_VALIDATION_FAILED;
    }
}
```

### Using Session Keys

```rust
use ethers::signers::{LocalWallet, Signer};
use std::time::{SystemTime, UNIX_EPOCH};

// Generate a new session key
let session_wallet = LocalWallet::new(&mut rand::thread_rng());
let session_address = session_wallet.address();

// Define validity period (24 hours)
let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_secs();
let valid_after = now as u48;
let valid_until = (now + 86400) as u48;

// Define permissions
let permissions = vec![
    SessionPermission {
        target: usdc_contract,
        selector: [0xa9, 0x05, 0x9c, 0xbb], // transfer(address,uint256)
        max_value: U256::zero(),
        rules: vec![
            PermissionRule::MaxAmount(U256::from(1000) * U256::exp10(6)), // 1000 USDC
            PermissionRule::RateLimit { count: 10, period_secs: 3600 },
        ],
    },
];

// Compute permissions hash
let permissions_hash = keccak256(&encode_permissions(&permissions));

// Add session key to account (owner signs this)
let add_key_op = build_add_session_key_op(
    &account,
    session_address,
    valid_after,
    valid_until,
    permissions_hash,
)?;

// Now the session key can sign UserOperations within its permissions
let user_op = service.build_transfer_op(
    &account,
    recipient,
    U256::from(100) * U256::exp10(6), // 100 USDC
    Some(usdc_transfer_data),
)?;

// Sign with session key
let hash = user_op.hash(entry_point, chain_id);
let signature = session_wallet.sign_message(hash.as_bytes()).await?;

let signed_op = user_op.with_signature(Bytes::from(signature.to_vec()));
```

## Gas Sponsorship (Paymaster)

### Paymaster Contract

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract RampOSPaymaster is IPaymaster, Ownable {
    using ECDSA for bytes32;

    IEntryPoint public immutable entryPoint;
    address public verifyingSigner;

    // Tenant spending limits
    mapping(bytes32 => uint256) public tenantDailySpent;
    mapping(bytes32 => uint256) public tenantDailyLimit;
    mapping(bytes32 => uint256) public tenantLastResetDay;

    // User rate limits
    mapping(address => uint256) public userDailyOps;
    mapping(address => uint256) public userLastResetDay;
    uint256 public maxOpsPerUserPerDay = 100;

    event Sponsored(
        address indexed sender,
        bytes32 indexed tenantId,
        uint256 gasCost
    );

    constructor(IEntryPoint _entryPoint, address _signer) Ownable(msg.sender) {
        entryPoint = _entryPoint;
        verifyingSigner = _signer;
    }

    function validatePaymasterUserOp(
        PackedUserOperation calldata userOp,
        bytes32 userOpHash,
        uint256 maxCost
    ) external override returns (bytes memory context, uint256 validationData) {
        require(msg.sender == address(entryPoint), "Only entry point");

        // Decode paymaster data: tenantId + validUntil + validAfter + signature
        bytes calldata paymasterData = userOp.paymasterAndData[20:];
        require(paymasterData.length >= 109, "Invalid paymaster data length");

        bytes32 tenantId = bytes32(paymasterData[0:32]);
        uint48 validUntil = uint48(bytes6(paymasterData[32:38]));
        uint48 validAfter = uint48(bytes6(paymasterData[38:44]));
        bytes calldata signature = paymasterData[44:109];

        // Verify signature
        bytes32 hash = keccak256(
            abi.encodePacked(userOpHash, tenantId, validUntil, validAfter)
        ).toEthSignedMessageHash();

        if (hash.recover(signature) != verifyingSigner) {
            revert InvalidSignature();
        }

        // Check tenant daily limit
        _checkAndUpdateTenantLimit(tenantId, maxCost);

        // Check user rate limit
        _checkAndUpdateUserRateLimit(userOp.sender);

        context = abi.encode(userOp.sender, tenantId, maxCost);
        validationData = _packValidationData(false, validUntil, validAfter);
    }

    function postOp(
        PostOpMode mode,
        bytes calldata context,
        uint256 actualGasCost,
        uint256 actualUserOpFeePerGas
    ) external override {
        require(msg.sender == address(entryPoint), "Only entry point");

        (address sender, bytes32 tenantId, uint256 maxCost) = abi.decode(
            context,
            (address, bytes32, uint256)
        );

        emit Sponsored(sender, tenantId, actualGasCost);

        // Refund unused gas to tenant limit
        if (maxCost > actualGasCost) {
            uint256 refund = maxCost - actualGasCost;
            if (tenantDailySpent[tenantId] >= refund) {
                tenantDailySpent[tenantId] -= refund;
            }
        }
    }

    // Admin functions
    function setTenantLimit(bytes32 tenantId, uint256 limit) external onlyOwner {
        tenantDailyLimit[tenantId] = limit;
    }

    function deposit() external payable {
        entryPoint.depositTo{value: msg.value}(address(this));
    }

    function getDeposit() external view returns (uint256) {
        return entryPoint.balanceOf(address(this));
    }
}
```

### Using Paymaster

```rust
use ethers::signers::Signer;

// Build paymaster data
fn build_paymaster_data(
    paymaster: Address,
    tenant_id: [u8; 32],
    valid_until: u64,
    valid_after: u64,
    signer: &impl Signer,
    user_op_hash: H256,
) -> Result<Bytes> {
    let mut data = Vec::new();

    // Paymaster address (20 bytes)
    data.extend_from_slice(paymaster.as_bytes());

    // Tenant ID (32 bytes)
    data.extend_from_slice(&tenant_id);

    // Valid until (6 bytes)
    data.extend_from_slice(&valid_until.to_be_bytes()[2..]);

    // Valid after (6 bytes)
    data.extend_from_slice(&valid_after.to_be_bytes()[2..]);

    // Sign the hash
    let hash = keccak256(
        encode(&[
            Token::FixedBytes(user_op_hash.as_bytes().to_vec()),
            Token::FixedBytes(tenant_id.to_vec()),
            Token::Uint(U256::from(valid_until)),
            Token::Uint(U256::from(valid_after)),
        ])
    );

    let signature = signer.sign_message(&hash).await?;
    data.extend_from_slice(&signature.to_vec());

    Ok(Bytes::from(data))
}

// Use in UserOperation
let user_op = service.build_transfer_op(&account, recipient, value, None)?;

let paymaster_data = build_paymaster_data(
    paymaster_address,
    tenant_id_bytes,
    Utc::now().timestamp() as u64 + 3600, // 1 hour
    Utc::now().timestamp() as u64,
    &paymaster_signer,
    user_op.hash(entry_point, chain_id),
)?;

let sponsored_op = user_op.with_paymaster(paymaster_data);
```

## Gas Estimation

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimation {
    pub pre_verification_gas: U256,
    pub verification_gas_limit: U256,
    pub call_gas_limit: U256,
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
}

async fn estimate_gas(
    bundler_url: &str,
    user_op: &UserOperation,
    entry_point: Address,
) -> Result<GasEstimation> {
    let client = reqwest::Client::new();

    let response = client
        .post(bundler_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_estimateUserOperationGas",
            "params": [user_op, entry_point],
            "id": 1
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;

    Ok(GasEstimation {
        pre_verification_gas: U256::from_str_radix(
            result["result"]["preVerificationGas"].as_str().unwrap_or("0x0"),
            16,
        )?,
        verification_gas_limit: U256::from_str_radix(
            result["result"]["verificationGasLimit"].as_str().unwrap_or("0x0"),
            16,
        )?,
        call_gas_limit: U256::from_str_radix(
            result["result"]["callGasLimit"].as_str().unwrap_or("0x0"),
            16,
        )?,
        max_fee_per_gas: U256::from(1_000_000_000), // Get from gas oracle
        max_priority_fee_per_gas: U256::from(1_000_000_000),
    })
}
```

## Bundler Integration

```rust
async fn send_user_operation(
    bundler_url: &str,
    user_op: &UserOperation,
    entry_point: Address,
) -> Result<H256> {
    let client = reqwest::Client::new();

    let response = client
        .post(bundler_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_sendUserOperation",
            "params": [user_op, entry_point],
            "id": 1
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;

    if let Some(error) = result.get("error") {
        return Err(Error::BundlerError(error.to_string()));
    }

    let op_hash = result["result"]
        .as_str()
        .ok_or(Error::InvalidResponse)?;

    Ok(H256::from_str(op_hash)?)
}

async fn get_user_operation_receipt(
    bundler_url: &str,
    op_hash: H256,
) -> Result<Option<UserOperationReceipt>> {
    let client = reqwest::Client::new();

    let response = client
        .post(bundler_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getUserOperationReceipt",
            "params": [op_hash],
            "id": 1
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;

    if result["result"].is_null() {
        return Ok(None);
    }

    // Parse receipt
    Ok(Some(UserOperationReceipt {
        user_op_hash: op_hash,
        sender: result["result"]["sender"].as_str().unwrap().parse()?,
        success: result["result"]["success"].as_bool().unwrap_or(false),
        actual_gas_cost: U256::from_str_radix(
            result["result"]["actualGasCost"].as_str().unwrap_or("0x0"),
            16,
        )?,
        tx_hash: result["result"]["receipt"]["transactionHash"]
            .as_str()
            .map(|s| H256::from_str(s).ok())
            .flatten(),
    }))
}
```

## Complete Integration Example

```rust
use ramp_aa::{SmartAccountService, UserOperation, ChainConfig};
use ethers::signers::{LocalWallet, Signer};

async fn execute_sponsored_transfer(
    tenant_id: &TenantId,
    user_id: &UserId,
    owner_wallet: &LocalWallet,
    recipient: Address,
    amount: U256,
    token_contract: Address,
) -> Result<H256> {
    let chain_config = ChainConfig::polygon_mainnet();

    // Initialize services
    let aa_service = SmartAccountService::new(
        chain_config.chain_id,
        factory_address,
        chain_config.entry_point_address,
    );

    // Get or create smart account
    let account = aa_service.get_or_create_account(
        tenant_id,
        user_id,
        owner_wallet.address(),
    ).await?;

    // Build ERC20 transfer calldata
    let transfer_data = encode_erc20_transfer(recipient, amount);

    // Build UserOperation
    let mut user_op = aa_service.build_transfer_op(
        &account,
        token_contract,
        U256::zero(), // No ETH value for ERC20 transfer
        Some(transfer_data),
    )?;

    // If account not deployed, include init code
    if !account.is_deployed {
        let salt = aa_service.compute_salt(tenant_id, user_id);
        let init_code = aa_service.build_init_code(owner_wallet.address(), salt)?;
        user_op = user_op.with_init_code(init_code);
    }

    // Estimate gas
    let gas = estimate_gas(
        &chain_config.bundler_url,
        &user_op,
        chain_config.entry_point_address,
    ).await?;

    user_op = user_op.with_gas(
        gas.call_gas_limit,
        gas.verification_gas_limit,
        gas.pre_verification_gas,
    ).with_fees(
        gas.max_fee_per_gas,
        gas.max_priority_fee_per_gas,
    );

    // Add paymaster for gas sponsorship
    let paymaster_data = build_paymaster_data(
        paymaster_address,
        tenant_id_to_bytes32(tenant_id),
        Utc::now().timestamp() as u64 + 3600,
        Utc::now().timestamp() as u64,
        &paymaster_signer,
        user_op.hash(chain_config.entry_point_address, chain_config.chain_id),
    )?;

    user_op = user_op.with_paymaster(paymaster_data);

    // Sign with owner
    let hash = user_op.hash(
        chain_config.entry_point_address,
        chain_config.chain_id,
    );
    let signature = owner_wallet.sign_message(hash.as_bytes()).await?;
    user_op = user_op.with_signature(Bytes::from(signature.to_vec()));

    // Send to bundler
    let op_hash = send_user_operation(
        &chain_config.bundler_url,
        &user_op,
        chain_config.entry_point_address,
    ).await?;

    // Wait for receipt
    loop {
        if let Some(receipt) = get_user_operation_receipt(
            &chain_config.bundler_url,
            op_hash,
        ).await? {
            if receipt.success {
                return Ok(receipt.tx_hash.unwrap_or(op_hash));
            } else {
                return Err(Error::UserOperationFailed);
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
```

## Configuration

```bash
# .env
# Chain configuration
AA_CHAIN_ID=137
AA_ENTRY_POINT=0x5FF137D4b0FDCD49DcA30c7CF57E578a026d2789
AA_FACTORY_ADDRESS=0x...your_factory
AA_PAYMASTER_ADDRESS=0x...your_paymaster

# Bundler
AA_BUNDLER_URL=https://api.stackup.sh/v1/bundler/...

# Signing
AA_PAYMASTER_SIGNER_KEY=0x...private_key
```

## Best Practices

1. **Gas Estimation**: Always estimate gas before sending
2. **Nonce Management**: Track nonces to avoid conflicts
3. **Signature Validation**: Validate signatures client-side before sending
4. **Error Handling**: Handle bundler rejections gracefully
5. **Session Key Expiry**: Use short-lived session keys
6. **Paymaster Limits**: Monitor tenant spending limits
7. **Audit Logging**: Log all UserOperations for debugging
