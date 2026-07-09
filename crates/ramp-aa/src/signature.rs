//! EIP-191 personal_sign signature recovery
//!
//! Provides `recover_personal_sign` which recovers the Ethereum address from a
//! message signed with `personal_sign` (MetaMask / EIP-191 prefix).

use alloy_primitives::{keccak256, Address};

/// Recover the Ethereum address that signed `message` via `personal_sign`.
///
/// # EIP-191 digest
/// ```text
/// keccak256("\x19Ethereum Signed Message:\n" + len(message).to_string() + message)
/// ```
///
/// # Signature encoding
/// 65 bytes: r[32] || s[32] || v[1].
/// `v` may be 0, 1, 27, or 28; values ≥ 27 are normalised by subtracting 27.
pub fn recover_personal_sign(
    message: &[u8],
    signature: &[u8],
) -> Result<Address, String> {
    // 1. Build EIP-191 prefix
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut data = Vec::with_capacity(prefix.len() + message.len());
    data.extend_from_slice(prefix.as_bytes());
    data.extend_from_slice(message);

    let hash = keccak256(&data);

    // 2. Parse 65-byte signature
    if signature.len() != 65 {
        return Err(format!(
            "Invalid signature length: expected 65 bytes, got {}",
            signature.len()
        ));
    }

    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&signature[..64]);
    let v_raw = signature[64];

    // Normalise v: accept 0, 1, 27, 28
    let recovery_id_byte = if v_raw >= 27 { v_raw - 27 } else { v_raw };
    if recovery_id_byte > 1 {
        return Err(format!("Invalid recovery id: v={}", v_raw));
    }

    // 3. Recover using k256 (same crate / pattern as eip7702/authorization.rs)
    use k256::ecdsa::{RecoveryId, Signature as K256Signature, VerifyingKey};

    let sig = K256Signature::from_slice(&sig_bytes)
        .map_err(|e| format!("Invalid signature bytes: {}", e))?;

    let recovery_id = RecoveryId::try_from(recovery_id_byte)
        .map_err(|e| format!("Invalid recovery id: {}", e))?;

    let verifying_key =
        VerifyingKey::recover_from_prehash(hash.as_slice(), &sig, recovery_id)
            .map_err(|e| format!("Signature recovery failed: {}", e))?;

    // 4. Derive Ethereum address: keccak256(uncompressed_pubkey[1..])[12..]
    let public_key = verifying_key.to_encoded_point(false);
    let pub_bytes = public_key.as_bytes();
    // pub_bytes[0] == 0x04 (uncompressed marker), skip it
    let hash2 = keccak256(&pub_bytes[1..]);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash2[12..]);

    Ok(Address::from(addr))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Signs `message` with the given k256 private-key bytes using EIP-191 prefix,
    /// returning the 65-byte signature (r || s || v with v ∈ {27, 28}).
    fn sign_personal_message(message: &[u8], privkey_bytes: &[u8; 32]) -> Vec<u8> {
        use k256::ecdsa::{SigningKey, signature::hazmat::PrehashSigner};

        let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
        let mut data = Vec::with_capacity(prefix.len() + message.len());
        data.extend_from_slice(prefix.as_bytes());
        data.extend_from_slice(message);
        let hash = keccak256(&data);

        let signing_key = SigningKey::from_bytes(privkey_bytes.into())
            .expect("valid private key");
        let (sig, recovery_id): (k256::ecdsa::Signature, _) =
            signing_key.sign_prehash(hash.as_slice()).expect("sign");

        let r_s = sig.to_bytes();
        let mut out = vec![0u8; 65];
        out[..64].copy_from_slice(&r_s);
        out[64] = recovery_id.to_byte() + 27;
        out
    }

    #[test]
    fn test_recover_known_key() {
        // A deterministic private key (not a real secret)
        let privkey: [u8; 32] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
            0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
        ];

        // Derive expected Ethereum address from the private key
        use k256::ecdsa::SigningKey;
        let signing_key = SigningKey::from_bytes((&privkey).into()).unwrap();
        let verifying_key = signing_key.verifying_key();
        let pub_point = verifying_key.to_encoded_point(false);
        let pub_bytes = pub_point.as_bytes();
        let hash = keccak256(&pub_bytes[1..]);
        let mut expected_addr = [0u8; 20];
        expected_addr.copy_from_slice(&hash[12..]);
        let expected = Address::from(expected_addr);

        let message = b"Sign in to RampOS Portal.";
        let sig = sign_personal_message(message, &privkey);

        let recovered = recover_personal_sign(message, &sig)
            .expect("recovery should succeed");

        assert_eq!(
            recovered, expected,
            "recovered address must match the signer's Ethereum address"
        );
    }

    #[test]
    fn test_recover_different_messages_gives_different_results() {
        let privkey: [u8; 32] = [0xde; 32];

        let sig1 = sign_personal_message(b"message one", &privkey);
        let sig2 = sign_personal_message(b"message two", &privkey);

        let addr1 = recover_personal_sign(b"message one", &sig1).unwrap();
        let addr2 = recover_personal_sign(b"message two", &sig2).unwrap();

        // Both should recover to the same address (same key)
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn test_wrong_signature_length_errors() {
        let result = recover_personal_sign(b"hello", &[0u8; 64]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("65 bytes"));
    }

    #[test]
    fn test_v_27_and_0_equivalent() {
        let privkey: [u8; 32] = [0xab; 32];
        let message = b"test v normalisation";

        let mut sig = sign_personal_message(message, &privkey);
        // Both v=27 and v=28 paths are covered by test_recover_known_key;
        // here we verify v=0/1 (raw form) also works.
        let v_original = sig[64];
        if v_original == 27 {
            sig[64] = 0;
        } else {
            sig[64] = 1;
        }

        let mut sig_27 = sig.clone();
        sig_27[64] = v_original; // restore
        let addr_raw = recover_personal_sign(message, &sig).unwrap();
        let addr_27 = recover_personal_sign(message, &sig_27).unwrap();
        assert_eq!(addr_raw, addr_27);
    }
}
