//! Proof serialization utilities.
//!
//! stark-v proofs are serialized using postcard for efficient binary encoding.
//! All proof types implement Serialize/Deserialize via serde.
//!
//! The proof type is `prover::Proof<Blake2sMerkleHasher>` which contains:
//! - `claim`: Component log sizes
//! - `interaction_claim`: LogUp claimed sums
//! - `public_data`: Execution state (PC, registers, I/O)
//! - `stark_proof`: The underlying STARK proof
//! - `interaction_pow`: Proof-of-work nonce
