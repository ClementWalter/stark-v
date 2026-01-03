//! Verifier for RV32IM proofs.

use num_traits::Zero;
use stwo::core::channel::{Blake2sChannel, Channel};
use stwo::core::pcs::{CommitmentSchemeVerifier, PcsConfig};
use stwo::core::vcs::blake2_merkle::{Blake2sMerkleChannel, Blake2sMerkleHasher};
use stwo::core::verifier::verify;
use stwo_constraint_framework::TraceLocationAllocator;

use crate::Proof;
use crate::components::Components;
use crate::errors::VerificationError;
use crate::relations::{INTERACTION_POW_BITS, PreProcessedTrace, Relations};

pub fn verify_rv32im(
    proof: Proof<Blake2sMerkleHasher>,
    config: PcsConfig,
) -> Result<(), VerificationError> {
    let channel = &mut Blake2sChannel::default();
    let mut commitment_scheme = CommitmentSchemeVerifier::<Blake2sMerkleChannel>::new(config);

    // Public data.
    proof.public_data.mix_into(channel);

    // Preprocessed trace.
    let preprocessed_trace = PreProcessedTrace::new();
    let preprocessed_log_sizes: Vec<u32> = preprocessed_trace
        .trace
        .iter()
        .map(|col| col.domain.log_size())
        .collect();

    let mut commitment_index = 0usize;
    {
        let commitments = &proof.stark_proof.commitments;
        commitment_scheme.commit(
            commitments[commitment_index],
            &preprocessed_log_sizes,
            channel,
        );
        commitment_index += 1;

        // Main execution trace.
        let main_log_sizes = proof.claim.main_trace_log_sizes();
        commitment_scheme.commit(commitments[commitment_index], &main_log_sizes, channel);
        commitment_index += 1;
    }
    proof.claim.mix_into(channel);

    // Interaction proof of work.
    if !channel.verify_pow_nonce(INTERACTION_POW_BITS, proof.interaction_pow) {
        return Err(VerificationError::InteractionProofOfWork);
    }
    channel.mix_u64(proof.interaction_pow);

    // Draw lookup elements.
    let relations = Relations::draw(channel);

    // Verify LogUp sum (components + public data).
    let total_sum =
        proof.interaction_claim.claimed_sum.total() + proof.public_data.logup_sum(&relations);
    if !total_sum.is_zero() {
        return Err(VerificationError::InvalidLogupSum);
    }

    // Mix interaction claim and commit interaction trace.
    proof.interaction_claim.mix_into(channel);
    if !proof.interaction_claim.log_sizes.is_empty() {
        let commitments = &proof.stark_proof.commitments;
        commitment_scheme.commit(
            commitments[commitment_index],
            &proof.interaction_claim.log_sizes,
            channel,
        );
    }

    // Verify STARK proof.
    let mut location_allocator =
        TraceLocationAllocator::new_with_preprocessed_columns(&preprocessed_trace.ids);
    let components = Components::new(
        &proof.claim,
        &mut location_allocator,
        relations,
        &proof.interaction_claim.claimed_sum,
    );

    verify(
        &components.verifiers(),
        channel,
        &mut commitment_scheme,
        proof.stark_proof,
    )
    .map_err(VerificationError::from)
}
