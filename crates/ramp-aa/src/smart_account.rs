use async_trait::async_trait;
use alloy::primitives::{Address, Bytes, U256, keccak256};
use alloy::dyn_abi::DynSolValue;
use ramp_common::{
    types::{TenantId, UserId},
    Result,
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
        let params = DynSolValue::Tuple(vec![
            DynSolValue::Address(owner),
            DynSolValue::Uint(salt, 256),
        ]).abi_encode();
        data.extend_from_slice(&params);

        Ok(Bytes::from(data))
    }

    /// Build UserOperation for a token transfer
    pub fn build_transfer_op(
        &self,
        account: &SmartAccount,
        to: Address,
        value: U256,
        data: Option<Bytes>,
    ) -> Result<UserOperation> {
        // Build execute(address,uint256,bytes) call
        let selector = [0xb6, 0x1d, 0x27, 0xf6]; // keccak256("execute(address,uint256,bytes)")[:4]

        let mut call_data = Vec::new();
        call_data.extend_from_slice(&selector);

        let params = DynSolValue::Tuple(vec![
            DynSolValue::Address(to),
            DynSolValue::Uint(value, 256),
            DynSolValue::Bytes(data.unwrap_or_default().to_vec()),
        ]).abi_encode();
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
        // Build executeBatch(address[],uint256[],bytes[]) call
        let selector = [0x34, 0xfc, 0xd5, 0xbe]; // keccak256("executeBatch(address[],uint256[],bytes[])")[:4]

        let targets: Vec<DynSolValue> = calls.iter().map(|(t, _, _)| DynSolValue::Address(*t)).collect();
        let values: Vec<DynSolValue> = calls.iter().map(|(_, v, _)| DynSolValue::Uint(*v, 256)).collect();
        let datas: Vec<DynSolValue> = calls
            .iter()
            .map(|(_, _, d)| DynSolValue::Bytes(d.to_vec()))
            .collect();

        let mut call_data = Vec::new();
        call_data.extend_from_slice(&selector);

        let params = DynSolValue::Tuple(vec![
            DynSolValue::Array(targets),
            DynSolValue::Array(values),
            DynSolValue::Array(datas),
        ]).abi_encode();
        call_data.extend_from_slice(&params);

        Ok(UserOperation::new(
            account.address,
            account.nonce,
            Bytes::from(call_data),
        ))
    }
}
