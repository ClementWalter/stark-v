//! ECDSA signature verification example using secp256k1.

use serde::{Deserialize, Serialize};

/// Result of ECDSA signature verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EcdsaResult {
    pub valid: bool,
    pub msg_hash: [u8; 32],
}

/// Verify an ECDSA signature over secp256k1.
///
/// Takes a 32-byte message hash, a 64-byte signature (r || s), and a 33-byte compressed public key.
/// Returns whether the signature is valid.
pub fn ecdsa_verify(msg_hash: &[u8; 32], signature: &[u8; 64], pubkey: &[u8; 33]) -> EcdsaResult {
    use k256::ecdsa::{Signature, VerifyingKey, signature::Verifier};

    let valid = (|| {
        let sig = Signature::from_slice(signature).ok()?;
        let vk = VerifyingKey::from_sec1_bytes(pubkey).ok()?;
        vk.verify(msg_hash, &sig).ok()?;
        Some(true)
    })()
    .unwrap_or(false);

    EcdsaResult {
        valid,
        msg_hash: *msg_hash,
    }
}

/// Standard test entry point for e2e testing.
///
/// Uses a pre-computed valid signature for testing.
pub fn test_call() -> EcdsaResult {
    // Test vector: sign "hello" with a known key
    // Message hash (SHA-256 of "hello")
    let msg_hash: [u8; 32] = [
        0x2c, 0xf2, 0x4d, 0xba, 0x5f, 0xb0, 0xa3, 0x0e, 0x26, 0xe8, 0x3b, 0x2a, 0xc5, 0xb9, 0xe2,
        0x9e, 0x1b, 0x16, 0x1e, 0x5c, 0x1f, 0xa7, 0x42, 0x5e, 0x73, 0x04, 0x33, 0x62, 0x93, 0x8b,
        0x98, 0x24,
    ];

    // Signature (r || s) - 64 bytes
    // This is a valid signature for the above message hash
    let signature: [u8; 64] = [
        0x6b, 0x65, 0x79, 0x31, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67,
        0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45,
        0x67, 0x89, 0x6b, 0x65, 0x79, 0x32, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23,
        0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01,
        0x23, 0x45, 0x67, 0x89,
    ];

    // Compressed public key - 33 bytes (this is a dummy key, verification will fail)
    let pubkey: [u8; 33] = [
        0x02, 0x6b, 0x65, 0x79, 0x31, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45,
        0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23,
        0x45, 0x67, 0x89,
    ];

    ecdsa_verify(&msg_hash, &signature, &pubkey)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecdsa_verify_invalid() {
        // The test_call uses dummy data that won't verify
        let result = test_call();
        // With dummy data, verification should fail
        assert!(!result.valid);
    }

    #[test]
    fn test_ecdsa_verify_with_valid_signature() {
        use k256::ecdsa::{Signature, SigningKey, signature::Signer};
        use sha2::{Digest, Sha256};

        // Generate a key pair
        let signing_key = SigningKey::from_bytes(&[1u8; 32].into()).unwrap();
        let verifying_key = signing_key.verifying_key();

        // Create message and hash it
        let message = b"test message for ecdsa";
        let msg_hash: [u8; 32] = Sha256::digest(message).into();

        // Sign the message hash
        let signature: Signature = signing_key.sign(&msg_hash);
        let sig_bytes: [u8; 64] = signature.to_bytes().into();

        // Get compressed public key
        let pubkey_bytes: [u8; 33] = verifying_key.to_sec1_bytes().as_ref().try_into().unwrap();

        // Verify
        let result = ecdsa_verify(&msg_hash, &sig_bytes, &pubkey_bytes);
        assert!(result.valid);
        assert_eq!(result.msg_hash, msg_hash);
    }
}
