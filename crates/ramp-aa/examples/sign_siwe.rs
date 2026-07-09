//! Dev-only SIWE signer for local smoke-testing the portal wallet-login flow.
//!
//! Mirrors the `personal_sign` (EIP-191) path that MetaMask uses and that the
//! server's `recover_personal_sign` verifies — so a signature produced here is
//! byte-for-byte acceptable to `POST /v1/portal/auth/wallet/verify`.
//!
//! Usage:
//!   cargo run -p ramp-aa --example sign_siwe -- <privkey_hex>            # prints ADDRESS
//!   cargo run -p ramp-aa --example sign_siwe -- <privkey_hex> <msgfile>  # prints ADDRESS + SIGNATURE
//!
//! The message file is read as raw bytes (no trailing newline added), exactly
//! as the server hashes `req.message.as_bytes()`.

use alloy_primitives::{keccak256, Address};
use k256::ecdsa::{signature::hazmat::PrehashSigner, SigningKey};

fn address_from_key(signing_key: &SigningKey) -> Address {
    let verifying_key = signing_key.verifying_key();
    let pub_point = verifying_key.to_encoded_point(false);
    let pub_bytes = pub_point.as_bytes();
    let hash = keccak256(&pub_bytes[1..]);
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);
    Address::from(addr)
}

/// Sign `message` with EIP-191 prefix, returning 65 bytes (r || s || v, v ∈ {27,28}).
fn sign_personal_message(message: &[u8], signing_key: &SigningKey) -> Vec<u8> {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut data = Vec::with_capacity(prefix.len() + message.len());
    data.extend_from_slice(prefix.as_bytes());
    data.extend_from_slice(message);
    let hash = keccak256(&data);

    let (sig, recovery_id): (k256::ecdsa::Signature, _) =
        signing_key.sign_prehash(hash.as_slice()).expect("sign");

    let r_s = sig.to_bytes();
    let mut out = vec![0u8; 65];
    out[..64].copy_from_slice(&r_s);
    out[64] = recovery_id.to_byte() + 27;
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: sign_siwe <privkey_hex> [msgfile]");
        std::process::exit(2);
    }

    let pk_hex = args[1].trim_start_matches("0x");
    let pk_bytes = hex::decode(pk_hex).expect("privkey must be hex");
    assert_eq!(pk_bytes.len(), 32, "privkey must be 32 bytes");
    let mut pk_arr = [0u8; 32];
    pk_arr.copy_from_slice(&pk_bytes);

    let signing_key = SigningKey::from_bytes((&pk_arr).into()).expect("valid private key");
    let address = address_from_key(&signing_key);
    // lowercase 0x-prefixed form, matching the server's msg_address comparison
    println!("ADDRESS=0x{}", hex::encode(address.as_slice()));

    if let Some(msgfile) = args.get(2) {
        let message = std::fs::read(msgfile).expect("read message file");
        let sig = sign_personal_message(&message, &signing_key);
        println!("SIGNATURE=0x{}", hex::encode(&sig));
    }
}
