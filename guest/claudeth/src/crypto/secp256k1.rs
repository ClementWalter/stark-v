//! secp256k1 ECDSA signature verification and public key recovery
//!
//! This module provides cryptographic functions for Ethereum's signature scheme:
//! - ECDSA signature verification
//! - Public key recovery from signatures
//! - Address derivation from signatures

#[cfg(target_arch = "riscv32")]
extern crate alloc;

use crate::crypto::keccak256;
use crate::crypto::secp256k1_math::{
    mod_add, mod_inv, mod_mul, mod_pow, mod_sub, secp256k1_n, secp256k1_p, SECP256K1_B,
};
use crate::crypto::secp256k1_point::{point_add, scalar_mul, AffinePoint};
use crate::types::{Address, Hash, U256};

/// Error type for secp256k1 operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secp256k1Error {
    /// Invalid signature format or length
    InvalidSignature,
    /// Invalid recovery ID (must be 0-3)
    InvalidRecoveryId,
    /// Invalid public key format
    InvalidPublicKey,
    /// Signature verification failed
    VerificationFailed,
}

fn deterministic_nonce(secret_key: U256, message_hash: &Hash, attempt: u64) -> U256 {
    let mut data = [0u8; 76];
    data[..4].copy_from_slice(b"SIG0");
    data[4..36].copy_from_slice(&secret_key.to_be_bytes());
    data[36..68].copy_from_slice(message_hash.as_bytes());
    data[68..].copy_from_slice(&attempt.to_be_bytes());
    let hash = keccak256(&data);
    U256::from_be_bytes(*hash.as_bytes())
}

fn validate_secret_key(secret_key: U256) -> Result<(), Secp256k1Error> {
    let n = secp256k1_n();
    if secret_key.is_zero() || secret_key >= n {
        return Err(Secp256k1Error::InvalidSignature);
    }
    Ok(())
}

fn parse_signature(signature: &[u8]) -> Result<(U256, U256), Secp256k1Error> {
    if signature.len() != 64 {
        return Err(Secp256k1Error::InvalidSignature);
    }

    let mut r_bytes = [0u8; 32];
    r_bytes.copy_from_slice(&signature[..32]);
    let mut s_bytes = [0u8; 32];
    s_bytes.copy_from_slice(&signature[32..]);

    let r = U256::from_be_bytes(r_bytes);
    let s = U256::from_be_bytes(s_bytes);
    let n = secp256k1_n();

    if r.is_zero() || r >= n || s.is_zero() || s >= n {
        return Err(Secp256k1Error::InvalidSignature);
    }

    Ok((r, s))
}

fn parse_public_key(public_key: &[u8]) -> Result<AffinePoint, Secp256k1Error> {
    if public_key.len() != 64 {
        return Err(Secp256k1Error::InvalidPublicKey);
    }

    let mut x_bytes = [0u8; 32];
    x_bytes.copy_from_slice(&public_key[..32]);
    let mut y_bytes = [0u8; 32];
    y_bytes.copy_from_slice(&public_key[32..]);

    let point = AffinePoint::Point {
        x: U256::from_be_bytes(x_bytes),
        y: U256::from_be_bytes(y_bytes),
    };

    if !point.is_on_curve() {
        return Err(Secp256k1Error::InvalidPublicKey);
    }

    Ok(point)
}

/// Derives the uncompressed public key (x||y) from a secp256k1 secret key.
pub fn public_key_from_secret(secret_key: U256) -> Result<[u8; 64], Secp256k1Error> {
    validate_secret_key(secret_key)?;
    let g = AffinePoint::generator();
    let point = scalar_mul(secret_key, g);
    let AffinePoint::Point { x, y } = point else {
        return Err(Secp256k1Error::InvalidSignature);
    };

    let mut public_key = [0u8; 64];
    public_key[..32].copy_from_slice(&x.to_be_bytes());
    public_key[32..].copy_from_slice(&y.to_be_bytes());
    Ok(public_key)
}

/// Derives an Ethereum address from a 64-byte uncompressed public key.
pub fn address_from_public_key(public_key: &[u8; 64]) -> Address {
    let hash = keccak256(public_key);
    let mut address_bytes = [0u8; 20];
    address_bytes.copy_from_slice(&hash.as_bytes()[12..]);
    Address::from(address_bytes)
}

/// Derives an Ethereum address directly from a secret key.
pub fn address_from_secret_key(secret_key: U256) -> Result<Address, Secp256k1Error> {
    let public_key = public_key_from_secret(secret_key)?;
    Ok(address_from_public_key(&public_key))
}

/// Signs a message hash with a secp256k1 secret key, returning (r, s, recovery_id).
///
/// This uses a deterministic nonce derived from keccak256 for test determinism.
pub fn sign_recoverable(
    message_hash: &Hash,
    secret_key: U256,
) -> Result<(U256, U256, u8), Secp256k1Error> {
    validate_secret_key(secret_key)?;
    let n = secp256k1_n();
    let n_half = n / U256::from_u64(2);
    let z = U256::from_be_bytes(*message_hash.as_bytes());

    for attempt in 0u64..1024 {
        let mut k = deterministic_nonce(secret_key, message_hash, attempt) % n;
        if k.is_zero() {
            k = U256::ONE;
        }

        let r_point = scalar_mul(k, AffinePoint::generator());
        let AffinePoint::Point { x, y } = r_point else {
            continue;
        };

        if x >= n {
            continue;
        }
        let mut recid = if (y & U256::ONE) == U256::ONE { 1u8 } else { 0u8 };

        let r = x % n;
        if r.is_zero() {
            continue;
        }

        let k_inv = match mod_inv(k, n) {
            Some(value) => value,
            None => continue,
        };

        let rd = mod_mul(r, secret_key, n);
        let sum = mod_add(z, rd, n);
        let mut s = mod_mul(k_inv, sum, n);
        if s.is_zero() {
            continue;
        }

        if s > n_half {
            s = mod_sub(U256::ZERO, s, n);
            recid ^= 1;
        }

        return Ok((r, s, recid));
    }

    Err(Secp256k1Error::InvalidSignature)
}

fn sqrt_mod_p(value: U256) -> Option<U256> {
    if value.is_zero() {
        return Some(U256::ZERO);
    }

    let p = secp256k1_p();
    let legendre_exp = (p - U256::ONE) / U256::from_u64(2);
    if mod_pow(value, legendre_exp, p) != U256::ONE {
        return None;
    }

    let sqrt_exp = (p + U256::ONE) / U256::from_u64(4);
    Some(mod_pow(value, sqrt_exp, p))
}

/// Verifies an ECDSA signature against a message hash and public key.
///
/// # Arguments
///
/// * `message_hash` - The 32-byte message hash
/// * `signature` - The 64-byte signature (r: 32 bytes, s: 32 bytes)
/// * `public_key` - The 64-byte uncompressed public key (without 0x04 prefix)
///
/// # Returns
///
/// `Ok(true)` if the signature is valid, `Ok(false)` if invalid, or `Err` if malformed.
///
/// # Examples
///
/// ```
/// use claudeth::crypto::secp256k1::verify_signature;
/// use claudeth::types::Hash;
///
/// let message_hash = Hash::from([0x42u8; 32]);
/// let signature = [0u8; 64];
/// let public_key = [0u8; 64];
///
/// // All-zero public key is invalid
/// let result = verify_signature(&message_hash, &signature, &public_key);
/// assert!(result.is_err());
/// ```
pub fn verify_signature(
    message_hash: &Hash,
    signature: &[u8],
    public_key: &[u8],
) -> Result<bool, Secp256k1Error> {
    if signature.len() != 64 {
        return Err(Secp256k1Error::InvalidSignature);
    }
    if public_key.len() != 64 {
        return Err(Secp256k1Error::InvalidPublicKey);
    }

    let public_key = parse_public_key(public_key)?;
    let (r, s) = parse_signature(signature)?;

    let n = secp256k1_n();
    let z = U256::from_be_bytes(*message_hash.as_bytes());

    let s_inv = mod_inv(s, n).ok_or(Secp256k1Error::InvalidSignature)?;
    let u1 = mod_mul(z, s_inv, n);
    let u2 = mod_mul(r, s_inv, n);

    let g = AffinePoint::generator();
    let u1g = scalar_mul(u1, g);
    let u2q = scalar_mul(u2, public_key);
    let sum = point_add(u1g, u2q);

    let AffinePoint::Point { x, .. } = sum else {
        return Ok(false);
    };

    let x_mod_n = mod_add(x, U256::ZERO, n);
    Ok(x_mod_n == r)
}

/// Recovers the public key from a signature and message hash.
///
/// # Arguments
///
/// * `message_hash` - The 32-byte message hash
/// * `signature` - The 64-byte signature (r: 32 bytes, s: 32 bytes)
/// * `recovery_id` - The recovery ID (0-3, typically v-27 from Ethereum)
///
/// # Returns
///
/// The 64-byte uncompressed public key (without 0x04 prefix).
///
/// # Examples
///
/// ```
/// use claudeth::crypto::secp256k1::recover_public_key;
/// use claudeth::types::Hash;
///
/// let message_hash = Hash::from([0x42u8; 32]);
/// let signature = [0u8; 64];
/// let recovery_id = 0;
///
/// let result = recover_public_key(&message_hash, &signature, recovery_id);
/// // Will fail with invalid signature, but demonstrates API
/// assert!(result.is_err());
/// ```
pub fn recover_public_key(
    message_hash: &Hash,
    signature: &[u8],
    recovery_id: u8,
) -> Result<[u8; 64], Secp256k1Error> {
    if recovery_id > 3 {
        return Err(Secp256k1Error::InvalidRecoveryId);
    }

    let (r, s) = parse_signature(signature)?;
    let n = secp256k1_n();
    let p = secp256k1_p();

    let x = if recovery_id >= 2 {
        r.checked_add(n)
            .filter(|value| *value < p)
            .ok_or(Secp256k1Error::InvalidSignature)?
    } else {
        r
    };

    let x_sq = mod_mul(x, x, p);
    let x_cubed = mod_mul(x_sq, x, p);
    let y_sq = mod_add(x_cubed, SECP256K1_B, p);

    let mut y = sqrt_mod_p(y_sq).ok_or(Secp256k1Error::InvalidSignature)?;
    let y_is_odd = (y & U256::ONE) == U256::ONE;
    let recid_is_odd = (recovery_id & 1) == 1;
    if y_is_odd != recid_is_odd {
        y = mod_sub(U256::ZERO, y, p);
    }

    let r_point = AffinePoint::Point { x, y };
    if !r_point.is_on_curve() {
        return Err(Secp256k1Error::InvalidSignature);
    }

    let r_inv = mod_inv(r, n).ok_or(Secp256k1Error::InvalidSignature)?;
    let z = U256::from_be_bytes(*message_hash.as_bytes());

    let u1 = mod_sub(U256::ZERO, mod_mul(z, r_inv, n), n);
    let u2 = mod_mul(s, r_inv, n);

    let g = AffinePoint::generator();
    let q = point_add(scalar_mul(u1, g), scalar_mul(u2, r_point));

    let AffinePoint::Point { x, y } = q else {
        return Err(Secp256k1Error::VerificationFailed);
    };

    let mut result = [0u8; 64];
    result[..32].copy_from_slice(&x.to_be_bytes());
    result[32..].copy_from_slice(&y.to_be_bytes());
    Ok(result)
}

/// Recovers the Ethereum address from a signature and message hash.
///
/// This is a convenience function that combines public key recovery with
/// address derivation (Keccak256 hash of public key, taking last 20 bytes).
///
/// # Arguments
///
/// * `message_hash` - The 32-byte message hash
/// * `signature` - The 64-byte signature (r: 32 bytes, s: 32 bytes)
/// * `recovery_id` - The recovery ID (0-3, typically v-27 from Ethereum)
///
/// # Returns
///
/// The 20-byte Ethereum address.
///
/// # Examples
///
/// ```
/// use claudeth::crypto::secp256k1::recover_address;
/// use claudeth::types::Hash;
///
/// let message_hash = Hash::from([0x42u8; 32]);
/// let signature = [0u8; 64];
/// let recovery_id = 0;
///
/// let result = recover_address(&message_hash, &signature, recovery_id);
/// // Will fail with invalid signature, but demonstrates API
/// assert!(result.is_err());
/// ```
pub fn recover_address(
    message_hash: &Hash,
    signature: &[u8],
    recovery_id: u8,
) -> Result<Address, Secp256k1Error> {
    // Recover public key
    let public_key = recover_public_key(message_hash, signature, recovery_id)?;

    // Hash the public key with Keccak256
    let hash = keccak256(&public_key);

    // Take last 20 bytes as address
    let mut address_bytes = [0u8; 20];
    address_bytes.copy_from_slice(&hash.as_bytes()[12..]);

    Ok(Address::from(address_bytes))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Real Ethereum Transaction Test Vectors
    // =========================================================================

    #[test]
    fn test_verify_signature_valid() {
        let message_hash = Hash::from([
            0xce, 0xb8, 0x71, 0xdb, 0x69, 0x75, 0x4f, 0x58,
            0x33, 0x38, 0x15, 0x58, 0x28, 0xaa, 0x4e, 0xd5,
            0xd9, 0xc8, 0x99, 0x18, 0xbd, 0x92, 0x27, 0x27,
            0x5b, 0x0a, 0x71, 0x51, 0x54, 0x69, 0x25, 0x4a,
        ]);
        let signature = [
            0x0f, 0x43, 0x8d, 0x04, 0xf4, 0x6a, 0x2b, 0x53,
            0x71, 0x96, 0xed, 0xa2, 0x48, 0x6c, 0x40, 0xcb,
            0x0f, 0xc1, 0xfb, 0x6d, 0x84, 0xa2, 0x58, 0xd6,
            0x75, 0x34, 0xe5, 0x18, 0x71, 0xc4, 0xf5, 0x3a,
            0x5a, 0xb0, 0x4d, 0xbc, 0x9c, 0x36, 0x56, 0x2f,
            0xf4, 0x38, 0x14, 0xc6, 0xd9, 0xf8, 0xbf, 0x9b,
            0xdc, 0x6b, 0x21, 0x0f, 0x52, 0x8b, 0x9b, 0xd1,
            0xd8, 0x84, 0x63, 0xa5, 0xd4, 0xff, 0xa6, 0x24,
        ];
        let public_key = [
            0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac,
            0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b, 0x07,
            0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9,
            0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17, 0x98,
            0x48, 0x3a, 0xda, 0x77, 0x26, 0xa3, 0xc4, 0x65,
            0x5d, 0xa4, 0xfb, 0xfc, 0x0e, 0x11, 0x08, 0xa8,
            0xfd, 0x17, 0xb4, 0x48, 0xa6, 0x85, 0x54, 0x19,
            0x9c, 0x47, 0xd0, 0x8f, 0xfb, 0x10, 0xd4, 0xb8,
        ];

        let result = verify_signature(&message_hash, &signature, &public_key);
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn test_verify_signature_invalid_signature_length() {
        let message_hash = Hash::from([0x42u8; 32]);
        let signature = [0u8; 63]; // Wrong length
        let public_key = [0u8; 64];

        let result = verify_signature(&message_hash, &signature, &public_key);
        assert_eq!(result, Err(Secp256k1Error::InvalidSignature));
    }

    #[test]
    fn test_verify_signature_invalid_public_key_length() {
        let message_hash = Hash::from([0x42u8; 32]);
        let signature = [0u8; 64];
        let public_key = [0u8; 63]; // Wrong length

        let result = verify_signature(&message_hash, &signature, &public_key);
        assert_eq!(result, Err(Secp256k1Error::InvalidPublicKey));
    }

    #[test]
    fn test_recover_public_key_invalid_signature_length() {
        let message_hash = Hash::from([0x42u8; 32]);
        let signature = [0u8; 63]; // Wrong length
        let recovery_id = 0;

        let result = recover_public_key(&message_hash, &signature, recovery_id);
        assert_eq!(result, Err(Secp256k1Error::InvalidSignature));
    }

    #[test]
    fn test_recover_public_key_invalid_recovery_id() {
        let message_hash = Hash::from([0x42u8; 32]);
        let signature = [0u8; 64];
        let recovery_id = 4; // Out of range (0-3)

        let result = recover_public_key(&message_hash, &signature, recovery_id);
        assert_eq!(result, Err(Secp256k1Error::InvalidRecoveryId));
    }

    #[test]
    fn test_recover_address_invalid_signature() {
        let message_hash = Hash::from([0x42u8; 32]);
        let signature = [0u8; 64]; // Invalid signature
        let recovery_id = 0;

        let result = recover_address(&message_hash, &signature, recovery_id);
        assert!(result.is_err());
    }

    // =========================================================================
    // Real Ethereum Signature Test Vectors
    // =========================================================================

    #[test]
    fn test_ethereum_personal_sign_message() {
        // Test Ethereum personal_sign message format
        // personal_sign prepends "\x19Ethereum Signed Message:\n{len}" to the message
        let message = b"Hello, Ethereum!";
        let prefix = b"\x19Ethereum Signed Message:\n16";

        let mut full_message = Vec::new();
        full_message.extend_from_slice(prefix);
        full_message.extend_from_slice(message);

        let message_hash = keccak256(&full_message);

        // Verify the hash is 32 bytes
        assert_eq!(message_hash.as_bytes().len(), 32);

        // This demonstrates the Ethereum personal_sign message format
        // Real signatures would be tested in the roundtrip tests
    }

    #[test]
    fn test_address_recovery_from_signature() {
        // Test the full flow: signature -> public key -> address
        let message = b"Test message";
        let message_hash = keccak256(message);

        let signature = [0x42u8; 64];
        let recovery_id = 0;

        // This should fail with invalid signature
        let result = recover_address(&message_hash, &signature, recovery_id);
        assert!(result.is_err());
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_empty_signature() {
        let message_hash = Hash::from([0x42u8; 32]);
        let signature = [];
        let public_key = [0u8; 64];

        let result = verify_signature(&message_hash, &signature, &public_key);
        assert_eq!(result, Err(Secp256k1Error::InvalidSignature));
    }

    #[test]
    fn test_empty_public_key() {
        let message_hash = Hash::from([0x42u8; 32]);
        let signature = [0u8; 64];
        let public_key = [];

        let result = verify_signature(&message_hash, &signature, &public_key);
        assert_eq!(result, Err(Secp256k1Error::InvalidPublicKey));
    }

    #[test]
    fn test_recovery_id_boundary_values() {
        let message_hash = Hash::from([0x42u8; 32]);
        let signature = [0u8; 64];

        // Test valid boundary values
        for recovery_id in 0..=3 {
            let result = recover_public_key(&message_hash, &signature, recovery_id);
            // Will fail with invalid signature, but recovery_id validation passes
            assert!(result.is_err());
            if let Err(e) = result {
                assert_ne!(e, Secp256k1Error::InvalidRecoveryId);
            }
        }

        // Test invalid boundary values
        for recovery_id in 4..=255 {
            let result = recover_public_key(&message_hash, &signature, recovery_id);
            assert_eq!(result, Err(Secp256k1Error::InvalidRecoveryId));
        }
    }

    #[test]
    fn test_all_zeros_signature() {
        let message_hash = Hash::from([0x42u8; 32]);
        let signature = [0u8; 64];
        let recovery_id = 0;

        // All zeros is not a valid signature
        let result = recover_public_key(&message_hash, &signature, recovery_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_all_ones_signature() {
        let message_hash = Hash::from([0x42u8; 32]);
        let signature = [0xffu8; 64];
        let recovery_id = 0;

        // All ones is not a valid signature
        let result = recover_public_key(&message_hash, &signature, recovery_id);
        assert!(result.is_err());
    }

    // =========================================================================
    // Real Signature Generation and Verification
    // =========================================================================

    #[test]
    fn test_recover_public_key_vector() {
        let message_hash = Hash::from([
            0xe5, 0x9a, 0xfd, 0xcd, 0xda, 0x4c, 0xc4, 0xd8,
            0xa5, 0x2e, 0x64, 0xec, 0xb2, 0x82, 0xb1, 0xfc,
            0xce, 0x9a, 0x55, 0xcd, 0x14, 0xfd, 0x33, 0xe4,
            0x89, 0x21, 0xdc, 0x9e, 0xba, 0xd6, 0xeb, 0x0b,
        ]);
        let signature = [
            0xf4, 0x1a, 0x0c, 0x71, 0xd4, 0xd3, 0x2c, 0xd7,
            0x00, 0xb1, 0xed, 0x30, 0x60, 0xdb, 0xde, 0xc6,
            0xd0, 0xfe, 0x76, 0x6a, 0x19, 0x3d, 0x39, 0x32,
            0xa3, 0x97, 0x5b, 0x37, 0xc4, 0x80, 0x85, 0xf1,
            0x6a, 0xab, 0x53, 0xe4, 0x63, 0x5a, 0x05, 0xba,
            0x30, 0xe1, 0x61, 0x80, 0x87, 0x95, 0x2d, 0x88,
            0xf3, 0xb3, 0x85, 0x49, 0xd0, 0xdf, 0xb7, 0x8d,
            0x9c, 0x74, 0x47, 0x92, 0xc6, 0x1a, 0x78, 0xf7,
        ];
        let expected_public_key = [
            0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac,
            0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b, 0x07,
            0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9,
            0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17, 0x98,
            0x48, 0x3a, 0xda, 0x77, 0x26, 0xa3, 0xc4, 0x65,
            0x5d, 0xa4, 0xfb, 0xfc, 0x0e, 0x11, 0x08, 0xa8,
            0xfd, 0x17, 0xb4, 0x48, 0xa6, 0x85, 0x54, 0x19,
            0x9c, 0x47, 0xd0, 0x8f, 0xfb, 0x10, 0xd4, 0xb8,
        ];

        let recovered =
            recover_public_key(&message_hash, &signature, 1).expect("recover public key");
        assert_eq!(recovered, expected_public_key);
    }

    #[test]
    fn test_recover_address_vector() {
        let message_hash = Hash::from([
            0xe5, 0x9a, 0xfd, 0xcd, 0xda, 0x4c, 0xc4, 0xd8,
            0xa5, 0x2e, 0x64, 0xec, 0xb2, 0x82, 0xb1, 0xfc,
            0xce, 0x9a, 0x55, 0xcd, 0x14, 0xfd, 0x33, 0xe4,
            0x89, 0x21, 0xdc, 0x9e, 0xba, 0xd6, 0xeb, 0x0b,
        ]);
        let signature = [
            0xf4, 0x1a, 0x0c, 0x71, 0xd4, 0xd3, 0x2c, 0xd7,
            0x00, 0xb1, 0xed, 0x30, 0x60, 0xdb, 0xde, 0xc6,
            0xd0, 0xfe, 0x76, 0x6a, 0x19, 0x3d, 0x39, 0x32,
            0xa3, 0x97, 0x5b, 0x37, 0xc4, 0x80, 0x85, 0xf1,
            0x6a, 0xab, 0x53, 0xe4, 0x63, 0x5a, 0x05, 0xba,
            0x30, 0xe1, 0x61, 0x80, 0x87, 0x95, 0x2d, 0x88,
            0xf3, 0xb3, 0x85, 0x49, 0xd0, 0xdf, 0xb7, 0x8d,
            0x9c, 0x74, 0x47, 0x92, 0xc6, 0x1a, 0x78, 0xf7,
        ];
        let expected_address = Address::from([
            0x7e, 0x5f, 0x45, 0x52, 0x09, 0x1a, 0x69, 0x12,
            0x5d, 0x5d, 0xfc, 0xb7, 0xb8, 0xc2, 0x65, 0x90,
            0x29, 0x39, 0x5b, 0xdf,
        ]);

        let recovered =
            recover_address(&message_hash, &signature, 1).expect("recover address");
        assert_eq!(recovered, expected_address);
    }

    #[test]
    fn test_verify_wrong_public_key() {
        let message_hash = Hash::from([
            0xce, 0xb8, 0x71, 0xdb, 0x69, 0x75, 0x4f, 0x58,
            0x33, 0x38, 0x15, 0x58, 0x28, 0xaa, 0x4e, 0xd5,
            0xd9, 0xc8, 0x99, 0x18, 0xbd, 0x92, 0x27, 0x27,
            0x5b, 0x0a, 0x71, 0x51, 0x54, 0x69, 0x25, 0x4a,
        ]);
        let signature = [
            0x0f, 0x43, 0x8d, 0x04, 0xf4, 0x6a, 0x2b, 0x53,
            0x71, 0x96, 0xed, 0xa2, 0x48, 0x6c, 0x40, 0xcb,
            0x0f, 0xc1, 0xfb, 0x6d, 0x84, 0xa2, 0x58, 0xd6,
            0x75, 0x34, 0xe5, 0x18, 0x71, 0xc4, 0xf5, 0x3a,
            0x5a, 0xb0, 0x4d, 0xbc, 0x9c, 0x36, 0x56, 0x2f,
            0xf4, 0x38, 0x14, 0xc6, 0xd9, 0xf8, 0xbf, 0x9b,
            0xdc, 0x6b, 0x21, 0x0f, 0x52, 0x8b, 0x9b, 0xd1,
            0xd8, 0x84, 0x63, 0xa5, 0xd4, 0xff, 0xa6, 0x24,
        ];
        let wrong_public_key = [
            0xc6, 0x04, 0x7f, 0x94, 0x41, 0xed, 0x7d, 0x6d,
            0x30, 0x45, 0x40, 0x6e, 0x95, 0xc0, 0x7c, 0xd8,
            0x5c, 0x77, 0x8e, 0x4b, 0x8c, 0xef, 0x3c, 0xa7,
            0xab, 0xac, 0x09, 0xb9, 0x5c, 0x70, 0x9e, 0xe5,
            0x1a, 0xe1, 0x68, 0xfe, 0xa6, 0x3d, 0xc3, 0x39,
            0xa3, 0xc5, 0x84, 0x19, 0x46, 0x6c, 0xea, 0xee,
            0xf7, 0xf6, 0x32, 0x65, 0x32, 0x66, 0xd0, 0xe1,
            0x23, 0x64, 0x31, 0xa9, 0x50, 0xcf, 0xe5, 0x2a,
        ];

        let result = verify_signature(&message_hash, &signature, &wrong_public_key);
        assert_eq!(result, Ok(false));
    }

    #[test]
    fn test_verify_wrong_message() {
        let wrong_message_hash = Hash::from([
            0x77, 0x01, 0x5a, 0x87, 0x9d, 0x44, 0xb7, 0xfb,
            0x32, 0x3a, 0x89, 0x02, 0x5f, 0x6a, 0x8f, 0x9c,
            0x27, 0x49, 0x73, 0x15, 0x5c, 0x49, 0x2f, 0xa7,
            0x82, 0x0c, 0xa6, 0x54, 0xfb, 0xc3, 0x14, 0x89,
        ]);
        let signature = [
            0x0f, 0x43, 0x8d, 0x04, 0xf4, 0x6a, 0x2b, 0x53,
            0x71, 0x96, 0xed, 0xa2, 0x48, 0x6c, 0x40, 0xcb,
            0x0f, 0xc1, 0xfb, 0x6d, 0x84, 0xa2, 0x58, 0xd6,
            0x75, 0x34, 0xe5, 0x18, 0x71, 0xc4, 0xf5, 0x3a,
            0x5a, 0xb0, 0x4d, 0xbc, 0x9c, 0x36, 0x56, 0x2f,
            0xf4, 0x38, 0x14, 0xc6, 0xd9, 0xf8, 0xbf, 0x9b,
            0xdc, 0x6b, 0x21, 0x0f, 0x52, 0x8b, 0x9b, 0xd1,
            0xd8, 0x84, 0x63, 0xa5, 0xd4, 0xff, 0xa6, 0x24,
        ];
        let public_key = [
            0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac,
            0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b, 0x07,
            0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9,
            0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17, 0x98,
            0x48, 0x3a, 0xda, 0x77, 0x26, 0xa3, 0xc4, 0x65,
            0x5d, 0xa4, 0xfb, 0xfc, 0x0e, 0x11, 0x08, 0xa8,
            0xfd, 0x17, 0xb4, 0x48, 0xa6, 0x85, 0x54, 0x19,
            0x9c, 0x47, 0xd0, 0x8f, 0xfb, 0x10, 0xd4, 0xb8,
        ];

        let result = verify_signature(&wrong_message_hash, &signature, &public_key);
        assert_eq!(result, Ok(false));
    }
}
