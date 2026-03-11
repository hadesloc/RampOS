//! EIP-7702 Authorization
//!
//! Implements the authorization tuple format for EIP-7702:
//! authorization = rlp([chain_id, address, nonce, y_parity, r, s])

use alloy::primitives::{keccak256, Address, U256};
use serde::{Deserialize, Serialize};

use super::{Eip7702Error, Result};

/// Magic prefix for EIP-7702 authorization signing
/// SET_CODE_TX_TYPE (0x04) as per the spec
pub const EIP7702_AUTH_MAGIC: u8 = 0x04;

/// ECDSA Signature (r, s, v)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub r: U256,
    pub s: U256,
    pub v: u64,
}

impl Signature {
    /// Recover the signer address from the signature
    pub fn recover(&self, message_hash: [u8; 32]) -> std::result::Result<Address, String> {
        use k256::ecdsa::{RecoveryId, Signature as K256Signature, VerifyingKey};

        let r_bytes = self.r.to_be_bytes::<32>();
        let s_bytes = self.s.to_be_bytes::<32>();

        let mut sig_bytes = [0u8; 64];
        sig_bytes[..32].copy_from_slice(&r_bytes);
        sig_bytes[32..].copy_from_slice(&s_bytes);

        let signature = K256Signature::from_slice(&sig_bytes)
            .map_err(|e| format!("Invalid signature: {}", e))?;

        let recovery_id = RecoveryId::try_from((self.v % 2) as u8)
            .map_err(|e| format!("Invalid recovery id: {}", e))?;

        let verifying_key =
            VerifyingKey::recover_from_prehash(&message_hash, &signature, recovery_id)
                .map_err(|e| format!("Recovery failed: {}", e))?;

        let public_key = verifying_key.to_encoded_point(false);
        let public_key_bytes = public_key.as_bytes();

        let hash = keccak256(&public_key_bytes[1..]);
        let mut address = [0u8; 20];
        address.copy_from_slice(&hash[12..]);

        Ok(Address::from(address))
    }
}

/// Raw authorization tuple (unsigned)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authorization {
    /// Chain ID this authorization is valid for
    pub chain_id: U256,
    /// Smart contract address to delegate execution to
    pub address: Address,
    /// EOA nonce at time of signing (prevents replay)
    pub nonce: u64,
}

impl Authorization {
    /// Create a new authorization
    pub fn new(chain_id: U256, address: Address, nonce: u64) -> Self {
        Self {
            chain_id,
            address,
            nonce,
        }
    }

    /// Create authorization for a specific chain
    pub fn for_chain(chain_id: u64, delegate: Address, nonce: u64) -> Self {
        Self::new(U256::from(chain_id), delegate, nonce)
    }

    /// Compute the signing hash for this authorization
    /// MAGIC || rlp([chain_id, address, nonce])
    pub fn signing_hash(&self) -> [u8; 32] {
        let encoded = self.rlp_encode();
        let mut data = Vec::with_capacity(1 + encoded.len());
        data.push(EIP7702_AUTH_MAGIC);
        data.extend_from_slice(&encoded);
        *keccak256(&data)
    }

    /// RLP encode the authorization tuple
    pub fn rlp_encode(&self) -> Vec<u8> {
        use rlp::RlpStream;

        let mut stream = RlpStream::new_list(3);

        // chain_id as big-endian bytes
        let chain_id_bytes = self.chain_id.to_be_bytes::<32>();
        let trimmed = trim_leading_zeros(&chain_id_bytes);
        stream.append(&trimmed);

        // address
        stream.append(&self.address.as_slice());

        // nonce
        stream.append(&self.nonce);

        stream.out().to_vec()
    }

    /// Sign this authorization with a private key
    pub fn sign(self, signature: Signature) -> SignedAuthorization {
        SignedAuthorization {
            authorization: self,
            signature,
        }
    }
}

/// Signed authorization with ECDSA signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAuthorization {
    /// The authorization tuple
    pub authorization: Authorization,
    /// ECDSA signature (r, s, v)
    pub signature: Signature,
}

impl SignedAuthorization {
    /// Create a new signed authorization
    pub fn new(authorization: Authorization, signature: Signature) -> Self {
        Self {
            authorization,
            signature,
        }
    }

    /// Recover the signer address from the signature
    pub fn recover_signer(&self) -> Result<Address> {
        let hash = self.authorization.signing_hash();
        self.signature
            .recover(hash)
            .map_err(|e| Eip7702Error::SignatureError(e.to_string()))
    }

    /// Verify the signature matches the expected signer
    pub fn verify(&self, expected_signer: Address) -> Result<()> {
        let recovered = self.recover_signer()?;
        if recovered != expected_signer {
            return Err(Eip7702Error::InvalidSignature);
        }
        Ok(())
    }

    /// RLP encode the signed authorization
    /// rlp([chain_id, address, nonce, y_parity, r, s])
    pub fn rlp_encode(&self) -> Vec<u8> {
        use rlp::RlpStream;

        let mut stream = RlpStream::new_list(6);

        // chain_id
        let chain_id_bytes = self.authorization.chain_id.to_be_bytes::<32>();
        stream.append(&trim_leading_zeros(&chain_id_bytes).to_vec());

        // address
        stream.append(&self.authorization.address.as_slice().to_vec());

        // nonce
        stream.append(&self.authorization.nonce);

        // y_parity (0 or 1): v=27 -> y_parity=0, v=28 -> y_parity=1
        let y_parity = (self.signature.v - 27) as u8;
        stream.append(&y_parity);

        // r
        let r_bytes = self.signature.r.to_be_bytes::<32>();
        stream.append(&r_bytes.to_vec());

        // s
        let s_bytes = self.signature.s.to_be_bytes::<32>();
        stream.append(&s_bytes.to_vec());

        stream.out().to_vec()
    }

    /// Decode from RLP bytes
    pub fn rlp_decode(data: &[u8]) -> Result<Self> {
        use rlp::Rlp;

        let rlp = Rlp::new(data);

        if rlp
            .item_count()
            .map_err(|e| Eip7702Error::EncodingError(e.to_string()))?
            != 6
        {
            return Err(Eip7702Error::EncodingError(
                "Invalid authorization RLP: expected 6 items".to_string(),
            ));
        }

        let chain_id_bytes: Vec<u8> = rlp
            .val_at(0)
            .map_err(|e| Eip7702Error::EncodingError(e.to_string()))?;
        let chain_id = U256::from_be_slice(&chain_id_bytes);

        let address_bytes: Vec<u8> = rlp
            .val_at(1)
            .map_err(|e| Eip7702Error::EncodingError(e.to_string()))?;
        let address = Address::from_slice(&address_bytes);

        let nonce: u64 = rlp
            .val_at(2)
            .map_err(|e| Eip7702Error::EncodingError(e.to_string()))?;

        let y_parity: u8 = rlp
            .val_at(3)
            .map_err(|e| Eip7702Error::EncodingError(e.to_string()))?;

        let r_bytes: Vec<u8> = rlp
            .val_at(4)
            .map_err(|e| Eip7702Error::EncodingError(e.to_string()))?;
        let r = U256::from_be_slice(&r_bytes);

        let s_bytes: Vec<u8> = rlp
            .val_at(5)
            .map_err(|e| Eip7702Error::EncodingError(e.to_string()))?;
        let s = U256::from_be_slice(&s_bytes);

        let v = 27 + y_parity as u64;
        let signature = Signature { r, s, v };

        Ok(Self {
            authorization: Authorization {
                chain_id,
                address,
                nonce,
            },
            signature,
        })
    }
}

/// List of authorizations for a transaction
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthorizationList {
    pub authorizations: Vec<SignedAuthorization>,
}

impl AuthorizationList {
    pub fn new() -> Self {
        Self {
            authorizations: Vec::new(),
        }
    }

    pub fn with_authorization(mut self, auth: SignedAuthorization) -> Self {
        self.authorizations.push(auth);
        self
    }

    pub fn add(&mut self, auth: SignedAuthorization) {
        self.authorizations.push(auth);
    }

    pub fn len(&self) -> usize {
        self.authorizations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.authorizations.is_empty()
    }

    /// RLP encode the authorization list
    pub fn rlp_encode(&self) -> Vec<u8> {
        use rlp::RlpStream;

        let mut stream = RlpStream::new_list(self.authorizations.len());
        for auth in &self.authorizations {
            stream.append_raw(&auth.rlp_encode(), 1);
        }
        stream.out().to_vec()
    }

    /// Validate all authorizations in the list
    pub fn validate_all(&self, chain_id: U256) -> Result<Vec<Address>> {
        let mut signers = Vec::with_capacity(self.authorizations.len());

        for auth in &self.authorizations {
            // Verify chain ID matches
            if auth.authorization.chain_id != chain_id {
                return Err(Eip7702Error::ChainIdMismatch {
                    expected: chain_id,
                    actual: auth.authorization.chain_id,
                });
            }

            // Recover signer
            let signer = auth.recover_signer()?;
            signers.push(signer);
        }

        Ok(signers)
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

    #[test]
    fn test_authorization_creation() {
        let delegate = "0x1234567890123456789012345678901234567890"
            .parse::<Address>()
            .unwrap();
        let auth = Authorization::for_chain(1, delegate, 5);

        assert_eq!(auth.chain_id, U256::from(1));
        assert_eq!(auth.address, delegate);
        assert_eq!(auth.nonce, 5);
    }

    #[test]
    fn test_authorization_signing_hash() {
        let delegate = "0x1234567890123456789012345678901234567890"
            .parse::<Address>()
            .unwrap();
        let auth = Authorization::for_chain(1, delegate, 0);

        let hash = auth.signing_hash();
        assert_eq!(hash.len(), 32);

        // Same auth should produce same hash
        let auth2 = Authorization::for_chain(1, delegate, 0);
        assert_eq!(auth.signing_hash(), auth2.signing_hash());

        // Different nonce should produce different hash
        let auth3 = Authorization::for_chain(1, delegate, 1);
        assert_ne!(auth.signing_hash(), auth3.signing_hash());
    }

    #[test]
    fn test_authorization_list() {
        let mut list = AuthorizationList::new();
        assert!(list.is_empty());

        let delegate = Address::ZERO;
        let auth = Authorization::for_chain(1, delegate, 0);
        let signed = SignedAuthorization::new(
            auth,
            Signature {
                r: U256::from(1),
                s: U256::from(2),
                v: 27,
            },
        );

        list.add(signed);
        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
    }

    #[test]
    fn test_trim_leading_zeros() {
        assert_eq!(trim_leading_zeros(&[0, 0, 1, 2]), &[1, 2]);
        assert_eq!(trim_leading_zeros(&[1, 2, 3]), &[1, 2, 3]);
        assert_eq!(trim_leading_zeros(&[0, 0, 0]), &[0]);
    }
}
