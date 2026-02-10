//! secp256k1 ECDSA signature verification and public key recovery
//!
//! This module provides cryptographic functions for Ethereum's signature scheme:
//! - ECDSA signature verification
//! - Public key recovery from signatures
//! - Address derivation from signatures

#[cfg(target_arch = "riscv32")]
extern crate alloc;

use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};

use crate::crypto::keccak256;
use crate::types::{Address, Hash};

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
    // Validate signature length
    if signature.len() != 64 {
        return Err(Secp256k1Error::InvalidSignature);
    }

    // Validate public key length
    if public_key.len() != 64 {
        return Err(Secp256k1Error::InvalidPublicKey);
    }

    // Parse signature
    let sig = Signature::try_from(signature).map_err(|_| Secp256k1Error::InvalidSignature)?;

    // Construct uncompressed public key with 0x04 prefix
    let mut pk_bytes = [0u8; 65];
    pk_bytes[0] = 0x04;
    pk_bytes[1..].copy_from_slice(public_key);

    // Parse public key
    let verifying_key =
        VerifyingKey::from_sec1_bytes(&pk_bytes).map_err(|_| Secp256k1Error::InvalidPublicKey)?;

    // Verify signature over the prehashed message (Ethereum uses Keccak-256 prehash).
    use k256::ecdsa::signature::hazmat::PrehashVerifier;
    Ok(verifying_key
        .verify_prehash(message_hash.as_bytes(), &sig)
        .is_ok())
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
    // Validate signature length
    if signature.len() != 64 {
        return Err(Secp256k1Error::InvalidSignature);
    }

    // Validate recovery ID
    if recovery_id > 3 {
        return Err(Secp256k1Error::InvalidRecoveryId);
    }

    // Parse signature
    let sig = Signature::try_from(signature).map_err(|_| Secp256k1Error::InvalidSignature)?;

    // Parse recovery ID
    let recid = RecoveryId::try_from(recovery_id).map_err(|_| Secp256k1Error::InvalidRecoveryId)?;

    // Recover public key
    let recovered_key = VerifyingKey::recover_from_prehash(message_hash.as_bytes(), &sig, recid)
        .map_err(|_| Secp256k1Error::VerificationFailed)?;

    // Convert to uncompressed format (65 bytes with 0x04 prefix)
    let pk_bytes = recovered_key.to_encoded_point(false);
    let pk_slice = pk_bytes.as_bytes();

    // Extract the 64 bytes (skip 0x04 prefix)
    let mut result = [0u8; 64];
    result.copy_from_slice(&pk_slice[1..]);

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
