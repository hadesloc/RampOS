//! EIP-7702 Transaction Building
//!
//! Implements EIP-7702 transaction type (0x04) with authorization list.

use alloy::primitives::{keccak256, Address, Bytes, U256, U64};
use serde::{Deserialize, Serialize};

use super::authorization::{AuthorizationList, SignedAuthorization};
use super::{Eip7702Error, Result};

/// EIP-7702 transaction type identifier
pub const EIP7702_TX_TYPE: u8 = 0x04;

/// EIP-7702 Transaction
///
/// A new transaction type that includes an authorization list allowing
/// EOAs to temporarily delegate to smart contract code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Eip7702Transaction {
    /// Chain ID
    pub chain_id: U64,
    /// Nonce
    pub nonce: U64,
    /// Max priority fee per gas (tip)
    pub max_priority_fee_per_gas: U256,
    /// Max fee per gas
    pub max_fee_per_gas: U256,
    /// Gas limit
    pub gas_limit: U64,
    /// Destination address (None for contract creation)
    pub to: Option<Address>,
    /// Value in wei
    pub value: U256,
    /// Input data
    pub data: Bytes,
    /// Access list (EIP-2930)
    pub access_list: Vec<(Address, Vec<U256>)>,
    /// EIP-7702 authorization list
    pub authorization_list: AuthorizationList,
}

impl Eip7702Transaction {
    /// Create a new EIP-7702 transaction
    pub fn new(chain_id: u64, nonce: u64, to: Address) -> Self {
        Self {
            chain_id: U64::from(chain_id),
            nonce: U64::from(nonce),
            max_priority_fee_per_gas: U256::ZERO,
            max_fee_per_gas: U256::ZERO,
            gas_limit: U64::from(21000),
            to: Some(to),
            value: U256::ZERO,
            data: Bytes::default(),
            access_list: Vec::new(),
            authorization_list: AuthorizationList::new(),
        }
    }

    /// RLP encode the transaction for signing
    pub fn signing_hash(&self) -> [u8; 32] {
        let encoded = self.rlp_encode_for_signing();
        let mut data = Vec::with_capacity(1 + encoded.len());
        data.push(EIP7702_TX_TYPE);
        data.extend_from_slice(&encoded);
        *keccak256(&data)
    }

    /// RLP encode for signing (without signature)
    fn rlp_encode_for_signing(&self) -> Vec<u8> {
        use rlp::RlpStream;

        // EIP-7702 transaction format:
        // rlp([chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas,
        //      gas_limit, to, value, data, access_list, authorization_list])
        let mut stream = RlpStream::new_list(10);

        // chain_id
        stream.append(&self.chain_id.to::<u64>());

        // nonce
        stream.append(&self.nonce.to::<u64>());

        // max_priority_fee_per_gas
        let mpfpg = self.max_priority_fee_per_gas.to_be_bytes::<32>();
        stream.append(&trim_leading_zeros(&mpfpg).to_vec());

        // max_fee_per_gas
        let mfpg = self.max_fee_per_gas.to_be_bytes::<32>();
        stream.append(&trim_leading_zeros(&mfpg).to_vec());

        // gas_limit
        stream.append(&self.gas_limit.to::<u64>());

        // to
        if let Some(to) = self.to {
            stream.append(&to.as_slice().to_vec());
        } else {
            stream.append_empty_data();
        }

        // value
        let value_bytes = self.value.to_be_bytes::<32>();
        stream.append(&trim_leading_zeros(&value_bytes).to_vec());

        // data
        stream.append(&self.data.to_vec());

        // access_list
        let mut access_stream = RlpStream::new_list(self.access_list.len());
        for (addr, slots) in &self.access_list {
            let mut item = RlpStream::new_list(2);
            item.append(&addr.as_slice().to_vec());
            let mut slots_stream = RlpStream::new_list(slots.len());
            for slot in slots {
                let slot_bytes = slot.to_be_bytes::<32>();
                slots_stream.append(&slot_bytes.to_vec());
            }
            item.append_raw(&slots_stream.out(), 1);
            access_stream.append_raw(&item.out(), 1);
        }
        stream.append_raw(&access_stream.out(), 1);

        // authorization_list
        stream.append_raw(&self.authorization_list.rlp_encode(), 1);

        stream.out().to_vec()
    }

    /// Check if this transaction has any authorizations
    pub fn has_authorizations(&self) -> bool {
        !self.authorization_list.is_empty()
    }
}

/// Builder for EIP-7702 transactions
pub struct Eip7702TxBuilder {
    tx: Eip7702Transaction,
}

impl Eip7702TxBuilder {
    /// Create a new builder
    pub fn new(chain_id: u64, nonce: u64) -> Self {
        Self {
            tx: Eip7702Transaction {
                chain_id: U64::from(chain_id),
                nonce: U64::from(nonce),
                max_priority_fee_per_gas: U256::ZERO,
                max_fee_per_gas: U256::ZERO,
                gas_limit: U64::from(21000),
                to: None,
                value: U256::ZERO,
                data: Bytes::default(),
                access_list: Vec::new(),
                authorization_list: AuthorizationList::new(),
            },
        }
    }

    /// Set destination address
    pub fn to(mut self, address: Address) -> Self {
        self.tx.to = Some(address);
        self
    }

    /// Set value
    pub fn value(mut self, value: U256) -> Self {
        self.tx.value = value;
        self
    }

    /// Set call data
    pub fn data(mut self, data: Bytes) -> Self {
        self.tx.data = data;
        self
    }

    /// Set gas limit
    pub fn gas_limit(mut self, limit: u64) -> Self {
        self.tx.gas_limit = U64::from(limit);
        self
    }

    /// Set max fee per gas
    pub fn max_fee_per_gas(mut self, fee: U256) -> Self {
        self.tx.max_fee_per_gas = fee;
        self
    }

    /// Set max priority fee per gas
    pub fn max_priority_fee_per_gas(mut self, fee: U256) -> Self {
        self.tx.max_priority_fee_per_gas = fee;
        self
    }

    /// Add an authorization
    pub fn add_authorization(mut self, auth: SignedAuthorization) -> Self {
        self.tx.authorization_list.add(auth);
        self
    }

    /// Set the full authorization list
    pub fn authorization_list(mut self, list: AuthorizationList) -> Self {
        self.tx.authorization_list = list;
        self
    }

    /// Add to access list
    pub fn add_access_list_entry(mut self, address: Address, slots: Vec<U256>) -> Self {
        self.tx.access_list.push((address, slots));
        self
    }

    /// Build the transaction
    pub fn build(self) -> Result<Eip7702Transaction> {
        // Validate
        if self.tx.to.is_none() && self.tx.data.is_empty() {
            return Err(Eip7702Error::EncodingError(
                "Contract creation requires data".to_string(),
            ));
        }

        if self.tx.authorization_list.is_empty() {
            return Err(Eip7702Error::EncodingError(
                "EIP-7702 transaction requires at least one authorization".to_string(),
            ));
        }

        Ok(self.tx)
    }

    /// Build without validation (for testing)
    pub fn build_unchecked(self) -> Eip7702Transaction {
        self.tx
    }
}

/// Trim leading zeros from a byte slice
fn trim_leading_zeros(bytes: &[u8]) -> &[u8] {
    let first_nonzero = bytes
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(bytes.len() - 1);
    &bytes[first_nonzero..]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eip7702::authorization::{Authorization, Signature};

    fn test_address() -> Address {
        "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap()
    }

    fn test_delegate() -> Address {
        "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
            .parse()
            .unwrap()
    }

    fn test_signed_auth() -> SignedAuthorization {
        let auth = Authorization::for_chain(1, test_delegate(), 0);
        SignedAuthorization::new(
            auth,
            Signature {
                r: U256::from(1),
                s: U256::from(2),
                v: 27,
            },
        )
    }

    #[test]
    fn test_tx_creation() {
        let tx = Eip7702Transaction::new(1, 0, test_address());

        assert_eq!(tx.chain_id, U64::from(1));
        assert_eq!(tx.nonce, U64::from(0));
        assert_eq!(tx.to, Some(test_address()));
        assert!(!tx.has_authorizations());
    }

    #[test]
    fn test_builder_basic() {
        let auth = test_signed_auth();

        let tx = Eip7702TxBuilder::new(1, 5)
            .to(test_address())
            .value(U256::from(1000))
            .gas_limit(100000)
            .max_fee_per_gas(U256::from(20_000_000_000u64))
            .max_priority_fee_per_gas(U256::from(1_000_000_000u64))
            .add_authorization(auth)
            .build()
            .unwrap();

        assert_eq!(tx.chain_id, U64::from(1));
        assert_eq!(tx.nonce, U64::from(5));
        assert_eq!(tx.value, U256::from(1000));
        assert_eq!(tx.gas_limit, U64::from(100000));
        assert!(tx.has_authorizations());
        assert_eq!(tx.authorization_list.len(), 1);
    }

    #[test]
    fn test_builder_requires_authorization() {
        let result = Eip7702TxBuilder::new(1, 0).to(test_address()).build();

        assert!(result.is_err());
    }

    #[test]
    fn test_signing_hash() {
        let auth = test_signed_auth();

        let tx = Eip7702TxBuilder::new(1, 0)
            .to(test_address())
            .add_authorization(auth)
            .build_unchecked();

        let hash = tx.signing_hash();
        assert_eq!(hash.len(), 32);

        // Same tx should produce same hash
        let auth2 = test_signed_auth();
        let tx2 = Eip7702TxBuilder::new(1, 0)
            .to(test_address())
            .add_authorization(auth2)
            .build_unchecked();

        assert_eq!(tx.signing_hash(), tx2.signing_hash());
    }

    #[test]
    fn test_access_list() {
        let auth = test_signed_auth();

        let tx = Eip7702TxBuilder::new(1, 0)
            .to(test_address())
            .add_authorization(auth)
            .add_access_list_entry(test_delegate(), vec![U256::from(1), U256::from(2)])
            .build_unchecked();

        assert_eq!(tx.access_list.len(), 1);
        assert_eq!(tx.access_list[0].1.len(), 2);
    }
}
