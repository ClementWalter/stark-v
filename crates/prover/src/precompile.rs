//! Cross-proof LogUp binding: proving a computation in a separate stwo
//! instance and binding it to its caller through a shared relation.
//!
//! This is the mechanism `docs/precompiles.md` builds on, reduced to its
//! essence and proven end to end. Two independent stwo proofs share one
//! LogUp relation: the *host* proof emits `value(x, y)` tuples it used (it
//! does not prove the relationship), and the *precompile* proof consumes
//! them while proving `y = x * x`. The shared relation is drawn from both
//! proofs' trace commitments (a two-phase handshake), so neither prover can
//! choose its trace after seeing the relation; the binder then checks the
//! two claimed LogUp sums cancel. Cancellation means every pair the host
//! used was discharged by a precompile row — the host never re-proves the
//! squaring, exactly as a real hash precompile would offload Poseidon2.
//!
//! The "square" here stands in for any pure function the precompile attests
//! (`y = poseidon2(x)`); the binding shape is identical.

use num_traits::Zero;
use stwo::core::ColumnVec;
use stwo::core::channel::{Blake2sChannel, Channel};
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::SecureField;
use stwo::core::pcs::{CommitmentSchemeVerifier, PcsConfig};
use stwo::core::poly::circle::CanonicCoset;
use stwo::core::proof::StarkProof;
use stwo::core::vcs_lifted::blake2_merkle::{Blake2sMerkleChannel, Blake2sMerkleHasher};
use stwo::core::verifier::{VerificationError, verify};
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::qm31::PackedQM31;
use stwo::prover::pcs::CommitmentSchemeProver;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::{CircleEvaluation, PolyOps};
use stwo::prover::poly::twiddles::TwiddleTree;
use stwo::prover::prove;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, LogupTraceGenerator, RelationEntry,
    TraceLocationAllocator, relation,
};
use stwo_macros::{combine, define_component_tables};

// The binding table: one row per `(x, y)` pair, on either side of the shared
// relation. The host fills it with the pairs it used; the precompile fills
// it with the pairs it validated.
define_component_tables! {
    binding: {
        committed: { x, y },
    },
}

use prover_columns::BindingColumns;

// The shared relation, arity 2: `(x, y)`. A real precompile widens this to
// the hash io tuple, e.g. `poseidon2_io(in_0..15, out_0..15)`.
relation!(ValueRelation, 2);

type B = SimdBackend;
type MC = Blake2sMerkleChannel;
type H = Blake2sMerkleHasher;

/// Which side of the shared relation a proof sits on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// The caller: emits `+value(x, y)` for each pair it used, with no
    /// constraint tying `y` to `x` (the precompile owns that).
    Emit,
    /// The precompile: consumes `-value(x, y)` and proves `y = x * x`.
    ConsumeSquare,
}

/// AIR of one binding side.
#[derive(Clone)]
pub struct Eval {
    pub log_size: u32,
    pub value: ValueRelation,
    pub role: Role,
}

pub type Component = FrameworkComponent<Eval>;

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let cols = BindingColumns::from_eval(&mut eval);
        // Enabler booleanity (generated) for every row.
        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }
        // The precompile is the only side that proves the relationship.
        if self.role == Role::ConsumeSquare {
            eval.add_constraint(cols.y.clone() - cols.x.clone() * cols.x.clone());
        }
        let numerator = match self.role {
            Role::Emit => E::EF::from(cols.enabler.clone()),
            Role::ConsumeSquare => -E::EF::from(cols.enabler.clone()),
        };
        eval.add_to_relation(RelationEntry::new(
            &self.value,
            numerator,
            &[cols.x.clone(), cols.y.clone()],
        ));
        eval.finalize_logup();
        eval
    }
}

/// Generate the interaction trace and claimed LogUp sum for one side.
///
/// The numerator is `±enabler`, so padding rows (enabler 0) contribute
/// `0 / value(0, 0)` and drop out cleanly.
fn gen_interaction_trace(
    trace: &[CircleEvaluation<B, BaseField, BitReversedOrder>],
    value: &ValueRelation,
    role: Role,
) -> (
    ColumnVec<CircleEvaluation<B, BaseField, BitReversedOrder>>,
    SecureField,
) {
    let cols = BindingColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.enabler.len();
    let log_size = trace[0].domain.log_size();
    let denom = combine!(value, [cols.x, cols.y]);

    debug_assert_eq!(denom.len(), simd_size);
    let mut logup_gen = LogupTraceGenerator::new(log_size);
    let mut col_gen = logup_gen.new_col();
    for (vec_row, &denominator) in denom.iter().enumerate() {
        let enabler = PackedQM31::from(cols.enabler[vec_row]);
        let numerator = match role {
            Role::Emit => enabler,
            Role::ConsumeSquare => -enabler,
        };
        col_gen.write_frac(vec_row, numerator, denominator);
    }
    col_gen.finalize_col();
    logup_gen.finalize_last()
}

/// One side's proof: its trace size, its claimed LogUp sum, and the stwo
/// proof. The trace-tree commitment inside `stark_proof` seeds the shared
/// relation.
pub struct SystemProof {
    pub log_size: u32,
    pub claimed_sum: SecureField,
    pub stark_proof: StarkProof<H>,
}

/// A bound pair of proofs: the host that used the pairs and the precompile
/// that validated them.
pub struct PrecompileBindingProof {
    pub host: SystemProof,
    pub precompile: SystemProof,
}

/// Draw the shared relation from both trace commitments' channel seeds.
///
/// Deterministic in both prover and verifier: the relation is a public
/// function of both proofs' trace roots, so neither prover commits its trace
/// after learning the relation.
fn draw_shared_relation(seed_host: SecureField, seed_precompile: SecureField) -> ValueRelation {
    let mut channel = Blake2sChannel::default();
    channel.mix_felts(&[seed_host, seed_precompile]);
    ValueRelation::draw(&mut channel)
}

/// Commit one side's preprocessed (empty) and trace trees, leaving the
/// channel at the post-commit state from which the shared seed is drawn.
fn commit_system<'a>(
    trace: &[CircleEvaluation<B, BaseField, BitReversedOrder>],
    config: PcsConfig,
    twiddles: &'a TwiddleTree<B>,
) -> (CommitmentSchemeProver<'a, B, MC>, Blake2sChannel, u32) {
    let log_size = trace[0].domain.log_size();
    let mut channel = Blake2sChannel::default();
    let mut commitment_scheme = CommitmentSchemeProver::<B, MC>::new(config, twiddles);

    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(vec![]);
    tree_builder.commit(&mut channel);

    channel.mix_u32s(&[log_size]);

    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(trace.to_vec());
    tree_builder.commit(&mut channel);

    (commitment_scheme, channel, log_size)
}

/// Finish one side: interaction trace, claimed sum, component, stwo proof.
/// The channel must already have the shared relation bound in.
fn finish_system(
    mut commitment_scheme: CommitmentSchemeProver<'_, B, MC>,
    channel: &mut Blake2sChannel,
    trace: &[CircleEvaluation<B, BaseField, BitReversedOrder>],
    value: &ValueRelation,
    role: Role,
) -> SystemProof {
    let log_size = trace[0].domain.log_size();
    let (interaction, claimed_sum) = gen_interaction_trace(trace, value, role);
    channel.mix_felts(&[claimed_sum]);

    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(interaction);
    tree_builder.commit(channel);

    let mut location_allocator = TraceLocationAllocator::default();
    let component = Component::new(
        &mut location_allocator,
        Eval {
            log_size,
            value: value.clone(),
            role,
        },
        claimed_sum,
    );
    let stark_proof =
        prove(&[&component], channel, commitment_scheme).expect("binding proof generation failed");

    SystemProof {
        log_size,
        claimed_sum,
        stark_proof,
    }
}

/// Prove the host and precompile sides as two independent stwo proofs bound
/// by the shared relation.
///
/// `pairs` are the `(x, y)` the host used; the precompile re-derives and
/// proves `y = x * x` for each. Both fill the same multiset, so the claimed
/// sums cancel.
pub fn prove_binding(pairs: &[(u32, u32)], config: PcsConfig) -> PrecompileBindingProof {
    prove_binding_sides(pairs, pairs, config)
}

/// Prove the two sides from independent pair lists — the faithful shape, in
/// which the host and the precompile build their tables separately. The
/// binding holds only when the two lists are the same multiset.
pub fn prove_binding_sides(
    host_pairs: &[(u32, u32)],
    precompile_pairs: &[(u32, u32)],
    config: PcsConfig,
) -> PrecompileBindingProof {
    let mut host_table = BindingTable::new();
    let mut precompile_table = BindingTable::new();
    for &(x, y) in host_pairs {
        host_table.push(x, y);
    }
    for &(x, y) in precompile_pairs {
        precompile_table.push(x, y);
    }
    let host_trace = host_table.into_witness();
    let precompile_trace = precompile_table.into_witness();

    let max_log_size = host_trace[0]
        .domain
        .log_size()
        .max(precompile_trace[0].domain.log_size());
    let twiddles = B::precompute_twiddles(
        CanonicCoset::new(max_log_size + 2 + config.fri_config.log_blowup_factor)
            .circle_domain()
            .half_coset,
    );

    let (host_scheme, mut host_channel, _) = commit_system(&host_trace, config, &twiddles);
    let (precompile_scheme, mut precompile_channel, _) =
        commit_system(&precompile_trace, config, &twiddles);

    // Two-phase draw: the relation depends on both committed traces.
    let seed_host = host_channel.draw_secure_felt();
    let seed_precompile = precompile_channel.draw_secure_felt();
    let value = draw_shared_relation(seed_host, seed_precompile);

    // Bind the shared relation into each transcript.
    host_channel.mix_felts(&[seed_host, seed_precompile]);
    precompile_channel.mix_felts(&[seed_host, seed_precompile]);

    let host = finish_system(
        host_scheme,
        &mut host_channel,
        &host_trace,
        &value,
        Role::Emit,
    );
    let precompile = finish_system(
        precompile_scheme,
        &mut precompile_channel,
        &precompile_trace,
        &value,
        Role::ConsumeSquare,
    );

    PrecompileBindingProof { host, precompile }
}

/// Replay one side's commitment phase, leaving the verifier channel at the
/// post-commit state. Mirrors [`commit_system`].
fn replay_commit(
    proof: &SystemProof,
    config: PcsConfig,
) -> (CommitmentSchemeVerifier<MC>, Blake2sChannel) {
    let mut channel = Blake2sChannel::default();
    let mut commitment_scheme = CommitmentSchemeVerifier::<MC>::new(config);
    let commitments = &proof.stark_proof.commitments;

    commitment_scheme.commit(commitments[0], &[], &mut channel);
    channel.mix_u32s(&[proof.log_size]);
    let trace_log_sizes = vec![proof.log_size; BindingColumns::<()>::SIZE];
    commitment_scheme.commit(commitments[1], &trace_log_sizes, &mut channel);

    (commitment_scheme, channel)
}

/// Finish verifying one side: bind the shared relation, commit the
/// interaction tree, and run stwo verification against the matching role.
fn verify_system(
    proof: SystemProof,
    mut commitment_scheme: CommitmentSchemeVerifier<MC>,
    channel: &mut Blake2sChannel,
    value: &ValueRelation,
    role: Role,
    seeds: [SecureField; 2],
) -> Result<(), VerificationError> {
    channel.mix_felts(&seeds);
    channel.mix_felts(&[proof.claimed_sum]);

    // One secure column (4 base columns) of LogUp fractions.
    let interaction_log_sizes = vec![proof.log_size; 4];
    commitment_scheme.commit(
        proof.stark_proof.commitments[2],
        &interaction_log_sizes,
        channel,
    );

    let mut location_allocator = TraceLocationAllocator::default();
    let component = Component::new(
        &mut location_allocator,
        Eval {
            log_size: proof.log_size,
            value: value.clone(),
            role,
        },
        proof.claimed_sum,
    );
    verify(
        &[&component],
        channel,
        &mut commitment_scheme,
        proof.stark_proof,
    )
}

/// Verify a bound pair: both stwo proofs hold, and their claimed LogUp sums
/// cancel under the shared relation.
///
/// Cancellation is the binding: every `value(x, y)` the host emitted was
/// consumed by a precompile row that proved `y = x * x`.
pub fn verify_binding(
    proof: PrecompileBindingProof,
    config: PcsConfig,
) -> Result<(), VerificationError> {
    let PrecompileBindingProof { host, precompile } = proof;

    let (host_scheme, mut host_channel) = replay_commit(&host, config);
    let (precompile_scheme, mut precompile_channel) = replay_commit(&precompile, config);

    let seed_host = host_channel.draw_secure_felt();
    let seed_precompile = precompile_channel.draw_secure_felt();
    let value = draw_shared_relation(seed_host, seed_precompile);

    // The cross-proof binding check.
    if !(host.claimed_sum + precompile.claimed_sum).is_zero() {
        return Err(VerificationError::InvalidStructure(
            "precompile binding: host and precompile claimed sums do not cancel".to_string(),
        ));
    }

    let seeds = [seed_host, seed_precompile];
    verify_system(
        host,
        host_scheme,
        &mut host_channel,
        &value,
        Role::Emit,
        seeds,
    )?;
    verify_system(
        precompile,
        precompile_scheme,
        &mut precompile_channel,
        &value,
        Role::ConsumeSquare,
        seeds,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PcsConfig {
        PcsConfig::default()
    }

    /// Squares of 0..6 — the honest host/precompile multiset.
    fn square_pairs() -> Vec<(u32, u32)> {
        (0u32..6).map(|x| (x, x * x)).collect()
    }

    #[test]
    fn test_binding_roundtrip_verifies() {
        let proof = prove_binding(&square_pairs(), config());
        assert!(verify_binding(proof, config()).is_ok());
    }

    #[test]
    fn test_binding_sums_cancel() {
        let proof = prove_binding(&square_pairs(), config());
        assert!((proof.host.claimed_sum + proof.precompile.claimed_sum).is_zero());
    }

    #[test]
    fn test_host_sum_alone_is_nonzero() {
        // The host's emissions do not balance on their own — the precompile
        // proof is what discharges them.
        let proof = prove_binding(&square_pairs(), config());
        assert!(!proof.host.claimed_sum.is_zero());
    }

    #[test]
    fn test_host_uses_pair_precompile_never_validated_is_rejected() {
        // The host emits a pair the precompile never proved: the multiset
        // does not close, the sums do not cancel, the binding is rejected.
        let mut host_pairs = square_pairs();
        host_pairs.push((7, 49));
        let proof = prove_binding_sides(&host_pairs, &square_pairs(), config());
        assert!(verify_binding(proof, config()).is_err());
    }

    #[test]
    fn test_precompile_validates_pair_host_never_used_is_rejected() {
        // Symmetric: an extra validated pair the host did not emit also
        // fails to cancel.
        let mut precompile_pairs = square_pairs();
        precompile_pairs.push((7, 49));
        let proof = prove_binding_sides(&square_pairs(), &precompile_pairs, config());
        assert!(verify_binding(proof, config()).is_err());
    }

    #[test]
    fn test_forged_claimed_sum_fails_stwo_verification() {
        // Forging the host's claimed sum to force the binder's cancellation
        // check to pass cannot help: the sum is bound to the committed
        // interaction trace, so stwo verification rejects it.
        let mut proof = prove_binding(&square_pairs(), config());
        let one = SecureField::from(BaseField::from(1));
        proof.host.claimed_sum = -proof.precompile.claimed_sum + one;
        proof.precompile.claimed_sum = -proof.host.claimed_sum;
        assert!(verify_binding(proof, config()).is_err());
    }
}
