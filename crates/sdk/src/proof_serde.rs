//! Proof serialization utilities.
//!
//! This module provides utilities for serializing and deserializing stark-v proofs.
//!
//! **Note**: Full proof serialization is a work in progress. Currently, the VM
//! caches proofs internally for verification. Full serialization support will
//! be added once the underlying proof types support Serialize/Deserialize.

// TODO: Implement full proof serialization once prover types have Serialize derives.
// This will involve:
// 1. Adding Serialize/Deserialize to Claim, InteractionClaim, PublicData
// 2. Using postcard for efficient binary serialization
// 3. Providing conversion to/from ere_zkvm_interface::Proof::Bytes
