use alloy::dyn_abi::DynSolValue;
use alloy::primitives::{keccak256, Address, Bytes, B256, U256};
use serde::{Deserialize, Serialize};

/// ERC-4337 UserOperation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOperation {
    /// The account making the operation
    pub sender: Address,

    /// Anti-replay parameter
    pub nonce: U256,

    /// Account init code (only for first operation)
    #[serde(default)]
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
    #[serde(default)]
    pub paymaster_and_data: Bytes,

    /// Account signature
    #[serde(default)]
    pub signature: Bytes,
}

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

    /// Calculate the hash of the UserOperation for signing
    pub fn hash(&self, entry_point: Address, chain_id: u64) -> B256 {
        // Pack the UserOperation fields
        let packed = DynSolValue::Tuple(vec![
            DynSolValue::Address(self.sender),
            DynSolValue::Uint(self.nonce, 256),
            DynSolValue::Bytes(keccak256(&self.init_code).as_slice().to_vec()),
            DynSolValue::Bytes(keccak256(&self.call_data).as_slice().to_vec()),
            DynSolValue::Uint(self.call_gas_limit, 256),
            DynSolValue::Uint(self.verification_gas_limit, 256),
            DynSolValue::Uint(self.pre_verification_gas, 256),
            DynSolValue::Uint(self.max_fee_per_gas, 256),
            DynSolValue::Uint(self.max_priority_fee_per_gas, 256),
            DynSolValue::Bytes(keccak256(&self.paymaster_and_data).as_slice().to_vec()),
        ])
        .abi_encode();

        let user_op_hash = keccak256(&packed);

        // Encode with entry point and chain ID
        let final_hash = DynSolValue::Tuple(vec![
            DynSolValue::FixedBytes(user_op_hash, 32),
            DynSolValue::Address(entry_point),
            DynSolValue::Uint(U256::from(chain_id), 256),
        ])
        .abi_encode();

        keccak256(&final_hash)
    }

    /// Check if this is an account creation operation
    pub fn is_account_creation(&self) -> bool {
        !self.init_code.is_empty()
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

    /// Set paymaster data
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
}

/// Packed UserOperation for v0.7 EntryPoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackedUserOperation {
    pub sender: Address,
    pub nonce: U256,
    pub init_code: Bytes,
    pub call_data: Bytes,
    pub account_gas_limits: [u8; 32], // packed callGasLimit and verificationGasLimit
    pub pre_verification_gas: U256,
    pub gas_fees: [u8; 32], // packed maxFeePerGas and maxPriorityFeePerGas
    pub paymaster_and_data: Bytes,
    pub signature: Bytes,
}

// impl From<UserOperation> for PackedUserOperation { // Unused
