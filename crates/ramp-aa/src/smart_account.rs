use alloy::primitives::{keccak256, Address, Bytes, U256};
use async_trait::async_trait;
use ramp_common::{
    onchain_gate::{
        experimental_onchain_execution_disabled_message, experimental_onchain_execution_enabled,
    },
    types::{TenantId, UserId},
    Error, Result,
};
use tracing::info;

use crate::types::SmartAccountType;
use crate::user_operation::UserOperation;

/// Smart account data
#[derive(Debug, Clone)]
pub struct SmartAccount {
    pub address: Address,
    pub owner: Address,
    pub account_type: SmartAccountType,
    pub is_deployed: bool,
    pub nonce: U256,
}

/// Smart account factory trait
#[async_trait]
pub trait SmartAccountFactory: Send + Sync {
    /// Compute counterfactual address
    async fn get_address(&self, owner: Address, salt: U256) -> Result<Address>;

    /// Generate init code for account creation
    async fn get_init_code(&self, owner: Address, salt: U256) -> Result<Bytes>;

    /// Check if account is deployed
    async fn is_deployed(&self, address: Address) -> Result<bool>;
}

/// Smart account service
pub struct SmartAccountService {
    _chain_id: u64,
    factory_address: Address,
    _entry_point: Address,
}

impl SmartAccountService {
    fn abi_encode_address(addr: Address) -> [u8; 32] {
        let mut word = [0u8; 32];
        word[12..32].copy_from_slice(addr.as_slice());
        word
    }

    fn abi_encode_uint(value: U256) -> [u8; 32] {
        value.to_be_bytes::<32>()
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

    fn encode_create_account_args(owner: Address, salt: U256) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(64);
        encoded.extend_from_slice(&Self::abi_encode_address(owner));
        encoded.extend_from_slice(&Self::abi_encode_uint(salt));
        encoded
    }

    fn encode_execute_args(to: Address, value: U256, data: &[u8]) -> Vec<u8> {
        let bytes_tail = Self::abi_encode_bytes(data);
        let mut encoded = Vec::with_capacity(96 + bytes_tail.len());
        encoded.extend_from_slice(&Self::abi_encode_address(to));
        encoded.extend_from_slice(&Self::abi_encode_uint(value));
        encoded.extend_from_slice(&Self::abi_encode_uint(U256::from(96u64)));
        encoded.extend_from_slice(&bytes_tail);
        encoded
    }

    fn encode_address_array(values: &[Address]) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(32 + values.len() * 32);
        encoded.extend_from_slice(&U256::from(values.len()).to_be_bytes::<32>());
        for value in values {
            encoded.extend_from_slice(&Self::abi_encode_address(*value));
        }
        encoded
    }

    fn encode_uint_array(values: &[U256]) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(32 + values.len() * 32);
        encoded.extend_from_slice(&U256::from(values.len()).to_be_bytes::<32>());
        for value in values {
            encoded.extend_from_slice(&Self::abi_encode_uint(*value));
        }
        encoded
    }

    fn encode_bytes_array(values: &[Bytes]) -> Vec<u8> {
        let mut head = Vec::with_capacity(32 + values.len() * 32);
        head.extend_from_slice(&U256::from(values.len()).to_be_bytes::<32>());

        let mut tail = Vec::new();
        let initial_offset = 32 + values.len() * 32;
        let mut current_offset = initial_offset;

        for value in values {
            head.extend_from_slice(&U256::from(current_offset).to_be_bytes::<32>());
            let encoded = Self::abi_encode_bytes(value.as_ref());
            current_offset += encoded.len();
            tail.extend_from_slice(&encoded);
        }

        head.extend_from_slice(&tail);
        head
    }

    fn encode_execute_batch_args(calls: &[(Address, U256, Bytes)]) -> Vec<u8> {
        let targets: Vec<Address> = calls.iter().map(|(target, _, _)| *target).collect();
        let values: Vec<U256> = calls.iter().map(|(_, value, _)| *value).collect();
        let datas: Vec<Bytes> = calls.iter().map(|(_, _, data)| data.clone()).collect();

        let targets_tail = Self::encode_address_array(&targets);
        let values_tail = Self::encode_uint_array(&values);
        let datas_tail = Self::encode_bytes_array(&datas);

        let targets_offset = 96u64;
        let values_offset = targets_offset + targets_tail.len() as u64;
        let datas_offset = values_offset + values_tail.len() as u64;

        let mut encoded =
            Vec::with_capacity(96 + targets_tail.len() + values_tail.len() + datas_tail.len());
        encoded.extend_from_slice(&Self::abi_encode_uint(U256::from(targets_offset)));
        encoded.extend_from_slice(&Self::abi_encode_uint(U256::from(values_offset)));
        encoded.extend_from_slice(&Self::abi_encode_uint(U256::from(datas_offset)));
        encoded.extend_from_slice(&targets_tail);
        encoded.extend_from_slice(&values_tail);
        encoded.extend_from_slice(&datas_tail);
        encoded
    }

    pub fn new(chain_id: u64, factory_address: Address, entry_point: Address) -> Self {
        Self {
            _chain_id: chain_id,
            factory_address,
            _entry_point: entry_point,
        }
    }

    /// Get or create smart account for a user
    pub async fn get_or_create_account(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
        owner: Address,
    ) -> Result<SmartAccount> {
        // Generate deterministic salt from tenant and user
        let salt = self.compute_salt(tenant_id, user_id);

        // Compute counterfactual address
        let address = self.compute_address(owner, salt)?;

        // Check if deployed (in production, would query chain)
        let is_deployed = false;

        info!(
            tenant_id = %tenant_id,
            user_id = %user_id,
            address = %address,
            is_deployed = is_deployed,
            "Smart account resolved"
        );

        Ok(SmartAccount {
            address,
            owner,
            account_type: SmartAccountType::SimpleAccount,
            is_deployed,
            nonce: U256::ZERO,
        })
    }

    /// Compute deterministic salt
    fn compute_salt(&self, tenant_id: &TenantId, user_id: &UserId) -> U256 {
        let data = format!("{}:{}", tenant_id.0, user_id.0);
        let hash = keccak256(data.as_bytes());
        U256::from_be_bytes(hash.0)
    }

    /// Compute counterfactual address using CREATE2
    fn compute_address(&self, _owner: Address, salt: U256) -> Result<Address> {
        // SimpleAccount init code hash (simplified)
        // In production, would use actual bytecode
        let init_code_hash = keccak256(&[0u8; 32]);

        // CREATE2 address = keccak256(0xff ++ factory ++ salt ++ init_code_hash)[12:]
        let mut data = Vec::with_capacity(85);
        data.push(0xff);
        data.extend_from_slice(self.factory_address.as_slice());
        let salt_bytes = salt.to_be_bytes::<32>();
        data.extend_from_slice(&salt_bytes);
        data.extend_from_slice(init_code_hash.as_slice());

        let hash = keccak256(&data);
        Ok(Address::from_slice(&hash[12..]))
    }

    /// Build UserOperation for account creation
    pub fn build_create_account_op(
        &self,
        account: &SmartAccount,
        owner: Address,
        salt: U256,
    ) -> Result<UserOperation> {
        // Build init code
        let init_code = self.build_init_code(owner, salt)?;

        // Empty call data for creation-only op
        let call_data = Bytes::default();

        let mut op = UserOperation::new(account.address, U256::ZERO, call_data);
        op = op.with_init_code(init_code);

        Ok(op)
    }

    /// Build init code
    fn build_init_code(&self, owner: Address, salt: U256) -> Result<Bytes> {
        // Factory address + createAccount(owner, salt) call
        let mut data = Vec::new();
        data.extend_from_slice(self.factory_address.as_slice());

        // Function selector for createAccount(address,uint256)
        let selector = [0x5f, 0xbf, 0xb9, 0xcf]; // keccak256("createAccount(address,uint256)")[:4]
        data.extend_from_slice(&selector);

        // Encode parameters using alloy DynSolValue
        let params = Self::encode_create_account_args(owner, salt);
        data.extend_from_slice(&params);

        Ok(Bytes::from(data))
    }

    /// EXPERIMENTAL — not launch scope (D-03). Fail-closed: execution calldata builders error unless explicitly enabled.
    pub fn build_transfer_op(
        &self,
        account: &SmartAccount,
        to: Address,
        value: U256,
        data: Option<Bytes>,
    ) -> Result<UserOperation> {
        if !experimental_onchain_execution_enabled() {
            return Err(Error::NotImplemented(
                experimental_onchain_execution_disabled_message(
                    "AA transfer UserOperation execution",
                ),
            ));
        }

        // Build execute(address,uint256,bytes) call
        let selector = [0xb6, 0x1d, 0x27, 0xf6]; // keccak256("execute(address,uint256,bytes)")[:4]

        let mut call_data = Vec::new();
        call_data.extend_from_slice(&selector);

        let params = Self::encode_execute_args(to, value, data.unwrap_or_default().as_ref());
        call_data.extend_from_slice(&params);

        Ok(UserOperation::new(
            account.address,
            account.nonce,
            Bytes::from(call_data),
        ))
    }

    /// EXPERIMENTAL — not launch scope (D-03). Fail-closed: batch execution calldata builders error unless explicitly enabled.
    pub fn build_batch_op(
        &self,
        account: &SmartAccount,
        calls: Vec<(Address, U256, Bytes)>,
    ) -> Result<UserOperation> {
        if !experimental_onchain_execution_enabled() {
            return Err(Error::NotImplemented(
                experimental_onchain_execution_disabled_message("AA batch UserOperation execution"),
            ));
        }

        // Build executeBatch(address[],uint256[],bytes[]) call
        let selector = [0x34, 0xfc, 0xd5, 0xbe]; // keccak256("executeBatch(address[],uint256[],bytes[])")[:4]

        let mut call_data = Vec::new();
        call_data.extend_from_slice(&selector);
        let params = Self::encode_execute_batch_args(&calls);
        call_data.extend_from_slice(&params);

        Ok(UserOperation::new(
            account.address,
            account.nonce,
            Bytes::from(call_data),
        ))
    }
}
