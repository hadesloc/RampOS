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
    fn abi_encode_address(addr: Address) -> [u8; 32] {
        let mut word = [0u8; 32];
        word[12..32].copy_from_slice(addr.as_slice());
        word
    }

    fn abi_encode_uint(value: U256) -> [u8; 32] {
        value.to_be_bytes::<32>()
    }

    fn abi_encode_fixed_bytes(value: &[u8; 32]) -> [u8; 32] {
        *value
    }

    fn abi_encode_bytes(data: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(32 + data.len().div_ceil(32) * 32);
        encoded.extend_from_slice(&U256::from(data.len()).to_be_bytes::<32>());
        encoded.extend_from_slice(data);
        let padding = (32 - (data.len() % 32)) % 32;
        if padding > 0 {
            encoded.resize(encoded.len() + padding, 0);
        }
        encoded
    }

    fn encode_packed_user_op(&self) -> Vec<u8> {
        let init_code_hash = keccak256(&self.init_code);
        let call_data_hash = keccak256(&self.call_data);
        let paymaster_hash = keccak256(&self.paymaster_and_data);

        let init_tail = Self::abi_encode_bytes(init_code_hash.as_slice());
        let call_tail = Self::abi_encode_bytes(call_data_hash.as_slice());
        let paymaster_tail = Self::abi_encode_bytes(paymaster_hash.as_slice());

        let init_offset = 32u64 * 10;
        let call_offset = init_offset + init_tail.len() as u64;
        let paymaster_offset = call_offset + call_tail.len() as u64;

        let mut encoded =
            Vec::with_capacity(init_offset as usize + init_tail.len() + call_tail.len() + paymaster_tail.len());
        encoded.extend_from_slice(&Self::abi_encode_address(self.sender));
        encoded.extend_from_slice(&Self::abi_encode_uint(self.nonce));
        encoded.extend_from_slice(&Self::abi_encode_uint(U256::from(init_offset)));
        encoded.extend_from_slice(&Self::abi_encode_uint(U256::from(call_offset)));
        encoded.extend_from_slice(&Self::abi_encode_uint(self.call_gas_limit));
        encoded.extend_from_slice(&Self::abi_encode_uint(self.verification_gas_limit));
        encoded.extend_from_slice(&Self::abi_encode_uint(self.pre_verification_gas));
        encoded.extend_from_slice(&Self::abi_encode_uint(self.max_fee_per_gas));
        encoded.extend_from_slice(&Self::abi_encode_uint(self.max_priority_fee_per_gas));
        encoded.extend_from_slice(&Self::abi_encode_uint(U256::from(paymaster_offset)));
        encoded.extend_from_slice(&init_tail);
        encoded.extend_from_slice(&call_tail);
        encoded.extend_from_slice(&paymaster_tail);
        encoded
    }

    fn encode_final_hash_tuple(user_op_hash: &[u8; 32], entry_point: Address, chain_id: u64) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(96);
        encoded.extend_from_slice(&Self::abi_encode_fixed_bytes(user_op_hash));
        encoded.extend_from_slice(&Self::abi_encode_address(entry_point));
        encoded.extend_from_slice(&Self::abi_encode_uint(U256::from(chain_id)));
        encoded
    }

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
        let packed = self.encode_packed_user_op();

        let user_op_hash = keccak256(&packed);

        // Encode with entry point and chain ID
        let final_hash = Self::encode_final_hash_tuple(&user_op_hash.0, entry_point, chain_id);

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
