//! Fiat-Shamir transcript replay for a stark-v proof.
//!
//! The recursive verifier AIR (docs/recursion.md) needs every channel draw
//! of the inner proof as witness data, and its composition-check component
//! must recompute the composition polynomial value at the OODS point from
//! the proof's sampled mask values — through the same
//! `FrameworkEval::evaluate` code the prover and host verifier use.
//!
//! This module performs that replay natively: it advances the channel
//! exactly as `verify_rv32im` + `stwo::core::verifier::verify` do, then
//! evaluates the composition through `Components::eval_composition_polynomial_at_point`
//! (which instantiates each component's `evaluate()` with `PointEvaluator`).
//! No constraint is copied: an edit to `define_trace_tables!` changes the
//! replayed value in the same compilation.

use stwo::core::air::Components as CoreComponents;
use stwo::core::channel::Channel;
use stwo::core::circle::CirclePoint;
use stwo::core::fields::qm31::{SECURE_EXTENSION_DEGREE, SecureField};
use stwo::core::pcs::PcsConfig;
use stwo::core::pcs::utils::try_get_lifting_log_size;
use stwo::core::vcs_lifted::blake2_merkle::Blake2sMerkleHasher;
use stwo::core::verifier::{COMPOSITION_LOG_SPLIT, VerificationError as StwoVerificationError};
use stwo_constraint_framework::{PREPROCESSED_TRACE_IDX, TraceLocationAllocator};

use crate::components::Components;
use crate::errors::VerificationError;
use crate::verifier::replay_claim_phase;
use crate::{Preprocessing, Proof};

/// The OODS composition check of a proof, replayed outside the verifier.
#[derive(Debug, Clone, Copy)]
pub struct OodsCheck {
    /// The OODS point drawn from the replayed channel.
    pub oods_point: CirclePoint<SecureField>,
    /// The constraint-combination coefficient drawn from the replayed channel.
    pub random_coeff: SecureField,
    /// Composition value claimed by the proof (combined from the sampled
    /// composition coordinate polynomials).
    pub claimed: SecureField,
    /// Composition value recomputed from the sampled mask values through the
    /// components' `evaluate()`.
    pub replayed: SecureField,
}

impl OodsCheck {
    /// Whether the proof's claimed composition value matches the replay
    /// (the DEEP-ALI check).
    pub fn holds(&self) -> bool {
        self.claimed == self.replayed
    }
}

/// Replay the transcript of a proof up to the OODS point and recompute the
/// composition polynomial value from the sampled mask values.
pub fn replay_composition_oods(
    proof: &Proof<Blake2sMerkleHasher>,
    config: PcsConfig,
    preprocessing: &Preprocessing,
) -> Result<OodsCheck, VerificationError> {
    let (mut channel, mut commitment_scheme, relations) =
        replay_claim_phase(proof, config, preprocessing)?;

    let preprocessed_ids = preprocessing.column_ids();
    let mut location_allocator =
        TraceLocationAllocator::new_with_preprocessed_columns(&preprocessed_ids);
    let components = Components::new(
        &proof.claim,
        &mut location_allocator,
        relations,
        &proof.interaction_claim.claimed_sum,
    );
    let verifiers = components.verifiers();
    let core_components = CoreComponents {
        n_preprocessed_columns: commitment_scheme.trees[PREPROCESSED_TRACE_IDX]
            .column_log_sizes
            .len(),
        components: verifiers,
    };

    // Mirror `stwo::core::verifier::verify_ex` up to the OODS draw.
    let split_composition_log_degree_bound =
        core_components.composition_log_degree_bound() - COMPOSITION_LOG_SPLIT;
    let lifting_log_size = try_get_lifting_log_size(
        &commitment_scheme.config,
        split_composition_log_degree_bound + commitment_scheme.config.fri_config.log_blowup_factor,
    )
    .map_err(StwoVerificationError::from)?;
    let max_log_degree_bound =
        lifting_log_size - commitment_scheme.config.fri_config.log_blowup_factor;

    let random_coeff = channel.draw_secure_felt();
    commitment_scheme.commit(
        *proof
            .stark_proof
            .commitments
            .last()
            .expect("proof has a composition commitment"),
        &[max_log_degree_bound; 2 * SECURE_EXTENSION_DEGREE],
        &mut channel,
    );
    let oods_point = CirclePoint::<SecureField>::get_random_point(&mut channel);

    let claimed = extract_composition_oods_eval(proof, oods_point, max_log_degree_bound)
        .ok_or_else(|| {
            StwoVerificationError::InvalidStructure(
                "Unexpected sampled_values structure".to_string(),
            )
        })?;
    let replayed = core_components.eval_composition_polynomial_at_point(
        oods_point,
        &proof.stark_proof.sampled_values,
        random_coeff,
        max_log_degree_bound,
    );

    Ok(OodsCheck {
        oods_point,
        random_coeff,
        claimed,
        replayed,
    })
}

/// Combine the sampled composition coordinate polynomials into the claimed
/// composition value at the OODS point.
///
/// The composition polynomial is committed as two splits of
/// `SECURE_EXTENSION_DEGREE` base-field coordinate polynomials each; the
/// full value is `left + oods_point.repeated_double(max_log_degree_bound - 1).x * right`.
fn extract_composition_oods_eval(
    proof: &Proof<Blake2sMerkleHasher>,
    oods_point: CirclePoint<SecureField>,
    max_log_degree_bound: u32,
) -> Option<SecureField> {
    let [.., left_and_right_composition_mask] = &**proof.stark_proof.sampled_values else {
        return None;
    };
    let left_and_right_coordinate_evals: [SecureField; 2 * SECURE_EXTENSION_DEGREE] =
        left_and_right_composition_mask
            .iter()
            .map(|columns| {
                let &[eval] = &columns[..] else {
                    return None;
                };
                Some(eval)
            })
            .collect::<Option<Vec<_>>>()?
            .try_into()
            .ok()?;

    let (left_coordinate_evals, right_coordinate_evals) =
        left_and_right_coordinate_evals.split_at(SECURE_EXTENSION_DEGREE);

    let left_eval = SecureField::from_partial_evals(left_coordinate_evals.try_into().ok()?);
    let right_eval = SecureField::from_partial_evals(right_coordinate_evals.try_into().ok()?);
    Some(left_eval + oods_point.repeated_double(max_log_degree_bound - 1).x * right_eval)
}
