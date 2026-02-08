use async_trait::async_trait;
use alloy::primitives::{Address, B256, U256};
use ramp_common::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::types::{ChainConfig, GasEstimation};
use crate::user_operation::UserOperation;

/// Bundler JSON-RPC response
#[derive(Debug, Clone, Deserialize)]
pub struct BundlerResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<T>,
    pub error: Option<BundlerError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BundlerError {
    pub code: i64,
    pub message: String,
}

/// UserOperation receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserOpReceipt {
    pub user_op_hash: B256,
    pub entry_point: Address,
    pub sender: Address,
    pub nonce: U256,
    pub paymaster: Option<Address>,
    pub actual_gas_cost: U256,
    pub actual_gas_used: U256,
    pub success: bool,
    pub logs: Vec<serde_json::Value>,
    pub receipt: TransactionReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionReceipt {
    pub transaction_hash: B256,
    pub block_hash: B256,
    pub block_number: U256,
}

/// Bundler client trait
#[async_trait]
pub trait Bundler: Send + Sync {
    /// Send UserOperation to bundler
    async fn send_user_operation(
        &self,
        user_op: &UserOperation,
        entry_point: Address,
    ) -> Result<B256>;

    /// Estimate gas for UserOperation
    async fn estimate_user_operation_gas(
        &self,
        user_op: &UserOperation,
        entry_point: Address,
    ) -> Result<GasEstimation>;

    /// Get UserOperation by hash
    async fn get_user_operation_by_hash(&self, hash: B256) -> Result<Option<UserOperation>>;

    /// Get UserOperation receipt
    async fn get_user_operation_receipt(&self, hash: B256) -> Result<Option<UserOpReceipt>>;

    /// Get supported entry points
    async fn supported_entry_points(&self) -> Result<Vec<Address>>;

    /// Get chain ID
    async fn chain_id(&self) -> Result<u64>;
}

/// HTTP Bundler client
pub struct BundlerClient {
    http_client: reqwest::Client,
    bundler_url: String,
    _chain_config: ChainConfig,
}

impl BundlerClient {
    pub fn new(chain_config: ChainConfig) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            bundler_url: chain_config.bundler_url.clone(),
            _chain_config: chain_config,
        }
    }

    async fn rpc_call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let response = self
            .http_client
            .post(&self.bundler_url)
            .json(&request)
            .send()
            .await
            .map_err(|e| ramp_common::Error::ExternalService {
                service: "Bundler".into(),
                message: e.to_string(),
            })?;

        let body: BundlerResponse<T> =
            response
                .json()
                .await
                .map_err(|e| ramp_common::Error::ExternalService {
                    service: "Bundler".into(),
                    message: e.to_string(),
                })?;

        if let Some(error) = body.error {
            return Err(ramp_common::Error::ExternalService {
                service: "Bundler".into(),
                message: format!("{}: {}", error.code, error.message),
            });
        }

        body.result
            .ok_or_else(|| ramp_common::Error::ExternalService {
                service: "Bundler".into(),
                message: "Empty response".into(),
            })
    }
}

#[async_trait]
impl Bundler for BundlerClient {
    async fn send_user_operation(
        &self,
        user_op: &UserOperation,
        entry_point: Address,
    ) -> Result<B256> {
        info!(
            sender = %user_op.sender,
            nonce = %user_op.nonce,
            "Sending UserOperation to bundler"
        );

        let params = serde_json::json!([user_op, entry_point]);
        let hash: String = self.rpc_call("eth_sendUserOperation", params).await?;

        Ok(hash
            .parse()
            .map_err(|_| ramp_common::Error::Internal("Invalid hash from bundler".into()))?)
    }

    async fn estimate_user_operation_gas(
        &self,
        user_op: &UserOperation,
        entry_point: Address,
    ) -> Result<GasEstimation> {
        let params = serde_json::json!([user_op, entry_point]);
        self.rpc_call("eth_estimateUserOperationGas", params).await
    }

    async fn get_user_operation_by_hash(&self, hash: B256) -> Result<Option<UserOperation>> {
        let params = serde_json::json!([hash]);
        self.rpc_call("eth_getUserOperationByHash", params).await
    }

    async fn get_user_operation_receipt(&self, hash: B256) -> Result<Option<UserOpReceipt>> {
        let params = serde_json::json!([hash]);
        self.rpc_call("eth_getUserOperationReceipt", params).await
    }

    async fn supported_entry_points(&self) -> Result<Vec<Address>> {
        self.rpc_call("eth_supportedEntryPoints", serde_json::json!([]))
            .await
    }

    async fn chain_id(&self) -> Result<u64> {
        let result: String = self.rpc_call("eth_chainId", serde_json::json!([])).await?;
        let chain_id = u64::from_str_radix(result.trim_start_matches("0x"), 16)
            .map_err(|_| ramp_common::Error::Internal("Invalid chain ID".into()))?;
        Ok(chain_id)
    }
}
