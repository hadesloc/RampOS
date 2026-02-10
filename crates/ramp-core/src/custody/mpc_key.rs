//! MPC Key Generation Service
//!
//! Implements simulated 2-of-3 Shamir Secret Sharing for key share generation,
//! storage, and refresh operations.

use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::info;

/// A single key share held by one party in the MPC scheme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyShare {
    /// Identifier for the party holding this share (1, 2, or 3)
    pub party_id: u8,
    /// Encrypted share bytes (simulated - in production this would be encrypted at rest)
    pub share_bytes: Vec<u8>,
    /// The combined public key that all shares reconstruct to
    pub public_key: Vec<u8>,
    /// When this share was created
    pub created_at: DateTime<Utc>,
    /// Generation number (increments on refresh)
    pub generation: u64,
}

/// Result of MPC key generation: 3 key shares and the combined public key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpcKeyGenResult {
    /// The three key shares (one per party)
    pub shares: Vec<KeyShare>,
    /// The combined public key derived from the secret
    pub public_key: Vec<u8>,
    /// Generation number
    pub generation: u64,
}

/// Service for MPC key management using simulated 2-of-3 Shamir Secret Sharing.
///
/// In production, this would delegate to an MPC-TSS library.
/// The simulation generates a random 32-byte secret, splits it into 3 shares
/// using polynomial evaluation over GF(256), and derives a public key via SHA-256.
pub struct MpcKeyService {
    /// Storage: user_id -> Vec<KeyShare>
    shares: Mutex<HashMap<String, Vec<KeyShare>>>,
    /// Tracks the current generation per user
    generations: Mutex<HashMap<String, u64>>,
}

impl MpcKeyService {
    pub fn new() -> Self {
        Self {
            shares: Mutex::new(HashMap::new()),
            generations: Mutex::new(HashMap::new()),
        }
    }

    /// Generate 3 key shares using simulated 2-of-3 Shamir Secret Sharing.
    ///
    /// The secret is a random 32-byte value. Shares are created by evaluating
    /// a degree-1 polynomial (threshold=2) at points x=1, x=2, x=3 over GF(256).
    /// The public key is SHA-256(secret).
    pub fn generate_key_shares(&self) -> MpcKeyGenResult {
        let mut rng = rand::thread_rng();

        // Generate random 32-byte secret
        let secret: Vec<u8> = (0..32).map(|_| rng.gen::<u8>()).collect();

        // Derive public key as SHA-256 of the secret (simulated)
        let public_key = Self::derive_public_key(&secret);

        // Generate shares using simulated Shamir's Secret Sharing
        let shares_data = Self::shamir_split(&secret, 2, 3, &mut rng);

        let now = Utc::now();
        let shares: Vec<KeyShare> = shares_data
            .into_iter()
            .enumerate()
            .map(|(i, share_bytes)| KeyShare {
                party_id: (i + 1) as u8,
                share_bytes,
                public_key: public_key.clone(),
                created_at: now,
                generation: 1,
            })
            .collect();

        info!(
            num_shares = shares.len(),
            public_key_hex = hex::encode(&public_key),
            "Generated MPC key shares"
        );

        MpcKeyGenResult {
            shares,
            public_key,
            generation: 1,
        }
    }

    /// Store a key share for a user and party.
    pub fn store_key_share(
        &self,
        user_id: &str,
        party_id: u8,
        share: KeyShare,
    ) -> ramp_common::Result<()> {
        if party_id < 1 || party_id > 3 {
            return Err(ramp_common::Error::Validation(
                "party_id must be 1, 2, or 3".into(),
            ));
        }

        let mut shares_map = self.shares.lock().unwrap();
        let user_shares = shares_map.entry(user_id.to_string()).or_default();

        // Replace existing share for this party, or add new
        if let Some(existing) = user_shares.iter_mut().find(|s| s.party_id == party_id) {
            *existing = share;
        } else {
            user_shares.push(share);
        }

        info!(user_id, party_id, "Stored key share");
        Ok(())
    }

    /// Retrieve all key shares for a user.
    pub fn get_key_shares(&self, user_id: &str) -> Vec<KeyShare> {
        let shares_map = self.shares.lock().unwrap();
        shares_map.get(user_id).cloned().unwrap_or_default()
    }

    /// Refresh key shares for a user: generates new shares that reconstruct
    /// to the same public key (proactive secret sharing).
    ///
    /// In a real MPC system, this would use a proactive share refresh protocol.
    /// Here we simulate it by re-splitting the reconstructed secret.
    pub fn refresh_key_shares(
        &self,
        user_id: &str,
    ) -> ramp_common::Result<MpcKeyGenResult> {
        let existing_shares = self.get_key_shares(user_id);
        if existing_shares.len() < 2 {
            return Err(ramp_common::Error::Validation(
                "Need at least 2 shares to refresh (threshold requirement)".into(),
            ));
        }

        // Extract the public key from existing shares
        let public_key = existing_shares[0].public_key.clone();

        // Reconstruct the secret from any 2 shares (simulated)
        let share_data: Vec<(u8, Vec<u8>)> = existing_shares
            .iter()
            .take(2)
            .map(|s| (s.party_id, s.share_bytes.clone()))
            .collect();
        let secret = Self::shamir_reconstruct(&share_data);

        // Verify reconstruction produces the same public key
        let reconstructed_pk = Self::derive_public_key(&secret);
        if reconstructed_pk != public_key {
            return Err(ramp_common::Error::Internal(
                "Share reconstruction produced different public key".into(),
            ));
        }

        // Increment generation
        let new_generation = {
            let mut gens = self.generations.lock().unwrap();
            let gen = gens.entry(user_id.to_string()).or_insert(1);
            *gen += 1;
            *gen
        };

        // Re-split with new random polynomial
        let mut rng = rand::thread_rng();
        let new_shares_data = Self::shamir_split(&secret, 2, 3, &mut rng);

        let now = Utc::now();
        let new_shares: Vec<KeyShare> = new_shares_data
            .into_iter()
            .enumerate()
            .map(|(i, share_bytes)| KeyShare {
                party_id: (i + 1) as u8,
                share_bytes,
                public_key: public_key.clone(),
                created_at: now,
                generation: new_generation,
            })
            .collect();

        // Store the refreshed shares
        {
            let mut shares_map = self.shares.lock().unwrap();
            shares_map.insert(user_id.to_string(), new_shares.clone());
        }

        info!(
            user_id,
            generation = new_generation,
            "Refreshed key shares"
        );

        Ok(MpcKeyGenResult {
            shares: new_shares,
            public_key,
            generation: new_generation,
        })
    }

    /// Derive a simulated public key from a secret (SHA-256 hash).
    fn derive_public_key(secret: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(secret);
        hasher.finalize().to_vec()
    }

    /// Simulated Shamir Secret Sharing: split a secret into `n` shares with threshold `t`.
    ///
    /// For each byte of the secret, we construct a random polynomial of degree (t-1)
    /// with the secret byte as the constant term, then evaluate at points 1..=n.
    /// Operations are in GF(256) using the irreducible polynomial x^8 + x^4 + x^3 + x + 1.
    fn shamir_split(
        secret: &[u8],
        threshold: usize,
        num_shares: usize,
        rng: &mut impl Rng,
    ) -> Vec<Vec<u8>> {
        let mut shares: Vec<Vec<u8>> = (0..num_shares).map(|_| Vec::with_capacity(secret.len())).collect();

        for &secret_byte in secret {
            // Random coefficients for the polynomial: a_0 = secret_byte, a_1..a_{t-1} = random
            let mut coefficients = vec![secret_byte];
            for _ in 1..threshold {
                coefficients.push(rng.gen::<u8>());
            }

            // Evaluate polynomial at x = 1, 2, ..., num_shares
            for (i, share) in shares.iter_mut().enumerate() {
                let x = (i + 1) as u8;
                let y = Self::gf256_poly_eval(&coefficients, x);
                share.push(y);
            }
        }

        shares
    }

    /// Reconstruct the secret from shares using Lagrange interpolation in GF(256).
    fn shamir_reconstruct(shares: &[(u8, Vec<u8>)]) -> Vec<u8> {
        if shares.is_empty() {
            return Vec::new();
        }

        let secret_len = shares[0].1.len();
        let mut secret = Vec::with_capacity(secret_len);

        for byte_idx in 0..secret_len {
            // Lagrange interpolation at x = 0
            let mut value: u8 = 0;

            for (i, (xi, share_i)) in shares.iter().enumerate() {
                let yi = share_i[byte_idx];

                // Compute Lagrange basis polynomial L_i(0)
                let mut basis: u8 = 1;
                for (j, (xj, _)) in shares.iter().enumerate() {
                    if i != j {
                        // basis *= (0 - xj) / (xi - xj) in GF(256)
                        // (0 - xj) = xj in GF(256) (since -x = x in GF(2^n))
                        let num = *xj;
                        let denom = Self::gf256_sub(*xi, *xj);
                        let inv_denom = Self::gf256_inv(denom);
                        basis = Self::gf256_mul(basis, Self::gf256_mul(num, inv_denom));
                    }
                }

                value = Self::gf256_add(value, Self::gf256_mul(yi, basis));
            }

            secret.push(value);
        }

        secret
    }

    // ----- GF(256) arithmetic -----

    /// Addition in GF(256) is XOR.
    fn gf256_add(a: u8, b: u8) -> u8 {
        a ^ b
    }

    /// Subtraction in GF(256) is the same as addition (XOR).
    fn gf256_sub(a: u8, b: u8) -> u8 {
        a ^ b
    }

    /// Multiplication in GF(256) using the irreducible polynomial 0x11B.
    fn gf256_mul(a: u8, b: u8) -> u8 {
        let mut result: u8 = 0;
        let mut a = a as u16;
        let mut b = b;

        while b > 0 {
            if b & 1 != 0 {
                result ^= a as u8;
            }
            a <<= 1;
            if a & 0x100 != 0 {
                a ^= 0x11B; // Reduce by x^8 + x^4 + x^3 + x + 1
            }
            b >>= 1;
        }

        result
    }

    /// Multiplicative inverse in GF(256) using extended Euclidean algorithm.
    /// Returns 0 for input 0 (which is mathematically undefined but safe for our use).
    fn gf256_inv(a: u8) -> u8 {
        if a == 0 {
            return 0;
        }
        // a^254 = a^(-1) in GF(256) by Fermat's little theorem
        let mut result = a;
        for _ in 0..6 {
            result = Self::gf256_mul(result, result);
            result = Self::gf256_mul(result, a);
        }
        // One more squaring to get a^254
        result = Self::gf256_mul(result, result);
        result
    }

    /// Evaluate a polynomial at point x in GF(256) using Horner's method.
    fn gf256_poly_eval(coefficients: &[u8], x: u8) -> u8 {
        let mut result: u8 = 0;
        for &coeff in coefficients.iter().rev() {
            result = Self::gf256_add(Self::gf256_mul(result, x), coeff);
        }
        result
    }
}

impl Default for MpcKeyService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key_shares_produces_three_shares() {
        let service = MpcKeyService::new();
        let result = service.generate_key_shares();

        assert_eq!(result.shares.len(), 3);
        assert_eq!(result.public_key.len(), 32); // SHA-256 output
        assert_eq!(result.generation, 1);

        for (i, share) in result.shares.iter().enumerate() {
            assert_eq!(share.party_id, (i + 1) as u8);
            assert_eq!(share.share_bytes.len(), 32);
            assert_eq!(share.public_key, result.public_key);
            assert_eq!(share.generation, 1);
        }
    }

    #[test]
    fn test_shares_are_different_from_each_other() {
        let service = MpcKeyService::new();
        let result = service.generate_key_shares();

        // All three shares should be different
        assert_ne!(result.shares[0].share_bytes, result.shares[1].share_bytes);
        assert_ne!(result.shares[1].share_bytes, result.shares[2].share_bytes);
        assert_ne!(result.shares[0].share_bytes, result.shares[2].share_bytes);
    }

    #[test]
    fn test_shamir_reconstruct_any_two_of_three() {
        let service = MpcKeyService::new();
        let result = service.generate_key_shares();

        // Extract share data
        let shares: Vec<(u8, Vec<u8>)> = result
            .shares
            .iter()
            .map(|s| (s.party_id, s.share_bytes.clone()))
            .collect();

        // Reconstruct from shares 1,2
        let secret_12 = MpcKeyService::shamir_reconstruct(&[shares[0].clone(), shares[1].clone()]);
        // Reconstruct from shares 1,3
        let secret_13 = MpcKeyService::shamir_reconstruct(&[shares[0].clone(), shares[2].clone()]);
        // Reconstruct from shares 2,3
        let secret_23 = MpcKeyService::shamir_reconstruct(&[shares[1].clone(), shares[2].clone()]);

        // All reconstructions should produce the same secret
        assert_eq!(secret_12, secret_13);
        assert_eq!(secret_13, secret_23);

        // Verify the public key matches
        let pk = MpcKeyService::derive_public_key(&secret_12);
        assert_eq!(pk, result.public_key);
    }

    #[test]
    fn test_store_and_get_key_shares() {
        let service = MpcKeyService::new();
        let result = service.generate_key_shares();

        // Store all shares for user
        for share in &result.shares {
            service
                .store_key_share("user-1", share.party_id, share.clone())
                .unwrap();
        }

        let stored = service.get_key_shares("user-1");
        assert_eq!(stored.len(), 3);
        assert_eq!(stored[0].party_id, 1);
        assert_eq!(stored[1].party_id, 2);
        assert_eq!(stored[2].party_id, 3);
    }

    #[test]
    fn test_store_key_share_invalid_party_id() {
        let service = MpcKeyService::new();
        let share = KeyShare {
            party_id: 0,
            share_bytes: vec![0; 32],
            public_key: vec![0; 32],
            created_at: Utc::now(),
            generation: 1,
        };

        let result = service.store_key_share("user-1", 0, share.clone());
        assert!(result.is_err());

        let share4 = KeyShare {
            party_id: 4,
            ..share
        };
        let result = service.store_key_share("user-1", 4, share4);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_key_shares_nonexistent_user() {
        let service = MpcKeyService::new();
        let shares = service.get_key_shares("nonexistent");
        assert!(shares.is_empty());
    }

    #[test]
    fn test_refresh_key_shares() {
        let service = MpcKeyService::new();
        let result = service.generate_key_shares();

        // Store all shares
        for share in &result.shares {
            service
                .store_key_share("user-1", share.party_id, share.clone())
                .unwrap();
        }

        // Refresh
        let refreshed = service.refresh_key_shares("user-1").unwrap();

        // Same public key
        assert_eq!(refreshed.public_key, result.public_key);
        // Generation incremented
        assert_eq!(refreshed.generation, 2);
        // Shares should be different (new random polynomial)
        assert_ne!(refreshed.shares[0].share_bytes, result.shares[0].share_bytes);
    }

    #[test]
    fn test_refresh_key_shares_insufficient_shares() {
        let service = MpcKeyService::new();
        let result = service.generate_key_shares();

        // Store only 1 share
        service
            .store_key_share("user-1", 1, result.shares[0].clone())
            .unwrap();

        // Should fail - need at least 2 for threshold
        let err = service.refresh_key_shares("user-1");
        assert!(err.is_err());
    }

    #[test]
    fn test_gf256_arithmetic() {
        // Addition is XOR
        assert_eq!(MpcKeyService::gf256_add(0, 0), 0);
        assert_eq!(MpcKeyService::gf256_add(1, 1), 0);
        assert_eq!(MpcKeyService::gf256_add(0xFF, 0xFF), 0);

        // Multiplication identity
        assert_eq!(MpcKeyService::gf256_mul(1, 42), 42);
        assert_eq!(MpcKeyService::gf256_mul(42, 1), 42);
        assert_eq!(MpcKeyService::gf256_mul(0, 42), 0);

        // Inverse
        for x in 1..=255u8 {
            let inv = MpcKeyService::gf256_inv(x);
            assert_eq!(
                MpcKeyService::gf256_mul(x, inv),
                1,
                "Inverse failed for x={}",
                x
            );
        }
    }

    #[test]
    fn test_store_replaces_existing_share() {
        let service = MpcKeyService::new();
        let result = service.generate_key_shares();

        // Store share for party 1
        service
            .store_key_share("user-1", 1, result.shares[0].clone())
            .unwrap();

        // Replace with a different share for party 1
        let mut new_share = result.shares[1].clone();
        new_share.party_id = 1;
        service
            .store_key_share("user-1", 1, new_share.clone())
            .unwrap();

        let stored = service.get_key_shares("user-1");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].share_bytes, new_share.share_bytes);
    }

    #[test]
    fn test_multiple_users_isolated() {
        let service = MpcKeyService::new();
        let result1 = service.generate_key_shares();
        let result2 = service.generate_key_shares();

        for share in &result1.shares {
            service
                .store_key_share("user-1", share.party_id, share.clone())
                .unwrap();
        }
        for share in &result2.shares {
            service
                .store_key_share("user-2", share.party_id, share.clone())
                .unwrap();
        }

        let u1_shares = service.get_key_shares("user-1");
        let u2_shares = service.get_key_shares("user-2");

        assert_eq!(u1_shares.len(), 3);
        assert_eq!(u2_shares.len(), 3);
        assert_ne!(u1_shares[0].public_key, u2_shares[0].public_key);
    }

    #[test]
    fn test_each_generation_unique() {
        let service = MpcKeyService::new();
        let result = service.generate_key_shares();
        assert_eq!(result.generation, 1);

        // Generate another -- independent, also generation 1
        let result2 = service.generate_key_shares();
        assert_eq!(result2.generation, 1);

        // Public keys should be different (different random secrets)
        assert_ne!(result.public_key, result2.public_key);
    }
}
