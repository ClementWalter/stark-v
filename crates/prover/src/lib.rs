#![allow(non_camel_case_types)]
#![feature(
    allocator_api,
    portable_simd,
    array_chunks,
    iter_array_chunks,
    macro_metavar_expr_concat
)]

#[macro_use]
pub mod macros;
#[macro_use]
pub mod logup_macros;
pub mod components;
pub mod errors;
pub mod preprocessed;
pub mod prover;
pub mod public_data;
pub mod relations;
pub mod verifier;

pub use errors::VerificationError;
pub use prover::prove_rv32im;
pub use public_data::PublicData;
pub use verifier::verify_rv32im;

/// E2E test infrastructure (building and running guest binaries).
#[doc(hidden)]
pub mod e2e;

use stwo::core::channel::Channel;
use stwo::core::proof::StarkProof;
use stwo::core::vcs::MerkleHasher;

use crate::components::ClaimedSum;

/// Interaction claim for LogUp (claimed sums + interaction trace log sizes).
#[derive(Clone, Debug)]
pub struct InteractionClaim {
    pub claimed_sum: ClaimedSum,
    pub log_sizes: Vec<u32>,
}

impl InteractionClaim {
    pub fn mix_into(&self, channel: &mut impl Channel) {
        self.claimed_sum.mix_into(channel);
        channel.mix_u64(self.log_sizes.len() as u64);
        for log_size in &self.log_sizes {
            channel.mix_u64(*log_size as u64);
        }
    }
}

/// RV32IM proof bundle.
#[derive(Clone, Debug)]
pub struct Proof<H: MerkleHasher> {
    pub claim: components::Claim,
    pub interaction_claim: InteractionClaim,
    pub public_data: PublicData,
    pub stark_proof: StarkProof<H>,
    pub interaction_pow: u64,
}
