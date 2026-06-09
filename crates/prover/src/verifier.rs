//! Verifier for RV32IM proofs.

use num_traits::Zero;
use stwo::core::channel::{Blake2sChannel, Channel};
use stwo::core::pcs::{CommitmentSchemeVerifier, PcsConfig};
use stwo::core::vcs_lifted::blake2_merkle::{Blake2sMerkleChannel, Blake2sMerkleHasher};
use stwo::core::verifier::verify;
use stwo_constraint_framework::TraceLocationAllocator;

use crate::Preprocessing;
use crate::Proof;
use crate::components::Components;
use crate::errors::VerificationError;
use crate::relations::{INTERACTION_POW_BITS, Relations};

/// Replay the claim phase of the Fiat-Shamir transcript: mix public data,
/// commit the preprocessed/main/interaction trees, check the interaction
/// proof of work, and draw the LogUp relations.
///
/// Returns the channel and commitment scheme advanced to the state right
/// before `stwo::core::verifier::verify` takes over (composition commitment
/// and OODS draws). Shared by host verification and the recursion transcript
/// replay (`crate::recursion`), so the protocol prefix has a single
/// implementation.
pub(crate) fn replay_claim_phase(
    proof: &Proof<Blake2sMerkleHasher>,
    config: PcsConfig,
    preprocessing: &Preprocessing,
) -> Result<
    (
        Blake2sChannel,
        CommitmentSchemeVerifier<Blake2sMerkleChannel>,
        Relations,
    ),
    VerificationError,
> {
    let mut channel = Blake2sChannel::default();
    let mut commitment_scheme = CommitmentSchemeVerifier::<Blake2sMerkleChannel>::new(config);

    // Public data.
    proof.public_data.mix_into(&mut channel);

    // Preprocessed trace — use pre-computed log sizes from preprocessing.
    let mut commitment_index = 0usize;
    {
        let commitments = &proof.stark_proof.commitments;
        commitment_scheme.commit(
            commitments[commitment_index],
            &preprocessing.log_sizes,
            &mut channel,
        );
        commitment_index += 1;

        // Main execution trace.
        let main_log_sizes = proof.claim.main_trace_log_sizes();
        commitment_scheme.commit(commitments[commitment_index], &main_log_sizes, &mut channel);
        commitment_index += 1;
    }
    proof.claim.mix_into(&mut channel);

    // Interaction proof of work.
    if !channel.verify_pow_nonce(INTERACTION_POW_BITS, proof.interaction_pow) {
        return Err(VerificationError::InteractionProofOfWork);
    }
    channel.mix_u64(proof.interaction_pow);

    // Draw lookup elements.
    let relations = Relations::draw(&mut channel);

    // Mix interaction claim and commit interaction trace.
    proof.interaction_claim.mix_into(&mut channel);
    if !proof.interaction_claim.log_sizes.is_empty() {
        let commitments = &proof.stark_proof.commitments;
        commitment_scheme.commit(
            commitments[commitment_index],
            &proof.interaction_claim.log_sizes,
            &mut channel,
        );
    }

    Ok((channel, commitment_scheme, relations))
}

pub fn verify_rv32im(
    proof: Proof<Blake2sMerkleHasher>,
    config: PcsConfig,
    preprocessing: &Preprocessing,
) -> Result<(), VerificationError> {
    let (mut channel, mut commitment_scheme, relations) =
        replay_claim_phase(&proof, config, preprocessing)?;

    // Verify LogUp sum (components + public data).
    let total_sum =
        proof.interaction_claim.claimed_sum.total() + proof.public_data.logup_sum(&relations);
    if !total_sum.is_zero() {
        return Err(VerificationError::InvalidLogupSum);
    }

    // Verify STARK proof.
    let preprocessed_ids = preprocessing.column_ids();
    let mut location_allocator =
        TraceLocationAllocator::new_with_preprocessed_columns(&preprocessed_ids);
    let components = Components::new(
        &proof.claim,
        &mut location_allocator,
        relations,
        &proof.interaction_claim.claimed_sum,
    );

    verify(
        &components.verifiers(),
        &mut channel,
        &mut commitment_scheme,
        proof.stark_proof,
    )
    .map_err(VerificationError::from)
}
