//! Prove and verify the recursion verifier AIR.
//!
//! Assembles the verifier-AIR components into a stwo proof: commit the
//! component tables as one trace tree, build the components against a shared
//! `TraceLocationAllocator`, and run stwo's prover/verifier. The components
//! are pure-constraint (no LogUp interaction yet), so the proof has an empty
//! preprocessed tree and no interaction tree; lookup relations binding the
//! components together arrive with the channel/Merkle milestones
//! (docs/recursion.md, M4+).

use num_traits::Zero;
use stwo::core::channel::{Channel, MerkleChannel};
use stwo::core::fields::qm31::SecureField;
use stwo::core::pcs::{CommitmentSchemeVerifier, PcsConfig};
use stwo::core::poly::circle::CanonicCoset;
use stwo::core::proof::StarkProof;
use stwo::core::vcs_lifted::blake2_merkle::Blake2sMerkleChannel;
use stwo::core::vcs_lifted::merkle_hasher::MerkleHasherLifted;
use stwo::core::verifier::VerificationError;
use stwo::core::verifier::verify;
use stwo::prover::backend::BackendForChannel;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::pcs::CommitmentSchemeProver;
use stwo::prover::poly::circle::PolyOps;
use stwo::prover::prove;
use stwo_constraint_framework::TraceLocationAllocator;

use crate::{
    CircleDoubleTable, FriFoldLineTable, LogupSumTable, Qm31InvTable, Qm31MulTable, circle_double,
    fri_fold, logup_sum, prover_columns, qm31_inv, qm31_mul,
};

/// Witness tables of the recursion AIR, one per component.
#[derive(Default)]
pub struct RecursionTraces {
    pub qm31_mul: Qm31MulTable,
    pub qm31_inv: Qm31InvTable,
    pub fri_fold_line: FriFoldLineTable,
    pub circle_double: CircleDoubleTable,
    pub logup_sum: LogupSumTable,
}

/// Proof of the recursion AIR plus the public claim (per-component sizes
/// and the LogUp claimed sum).
pub struct RecursionProof<H: MerkleHasherLifted> {
    /// Log sizes of (qm31_mul, qm31_inv, fri_fold_line, circle_double,
    /// logup_sum).
    pub log_sizes: [u32; 5],
    /// Claimed sum of the logup_sum component: Σ enabler / term.
    pub claimed_sum: SecureField,
    pub stark_proof: StarkProof<H>,
}

fn mix_claim<C: Channel>(channel: &mut C, log_sizes: &[u32; 5], claimed_sum: SecureField) {
    channel.mix_u32s(log_sizes);
    channel.mix_felts(&[claimed_sum]);
}

/// Trace-tree column log sizes in commit order.
fn column_log_sizes(log_sizes: &[u32; 5]) -> Vec<u32> {
    let widths = [
        prover_columns::Qm31MulColumns::<()>::SIZE,
        prover_columns::Qm31InvColumns::<()>::SIZE,
        prover_columns::FriFoldLineColumns::<()>::SIZE,
        prover_columns::CircleDoubleColumns::<()>::SIZE,
        prover_columns::LogupSumColumns::<()>::SIZE,
    ];
    log_sizes
        .iter()
        .zip(widths)
        .flat_map(|(&log_size, width)| std::iter::repeat_n(log_size, width))
        .collect()
}

/// Build the four components in commit order against a shared allocator.
fn components(
    location_allocator: &mut TraceLocationAllocator,
    log_sizes: &[u32; 5],
    claimed_sum: SecureField,
) -> (
    qm31_mul::Component,
    qm31_inv::Component,
    fri_fold::Component,
    circle_double::Component,
    logup_sum::Component,
) {
    (
        qm31_mul::Component::new(
            location_allocator,
            qm31_mul::Eval {
                log_size: log_sizes[0],
            },
            SecureField::zero(),
        ),
        qm31_inv::Component::new(
            location_allocator,
            qm31_inv::Eval {
                log_size: log_sizes[1],
            },
            SecureField::zero(),
        ),
        fri_fold::Component::new(
            location_allocator,
            fri_fold::Eval {
                log_size: log_sizes[2],
            },
            SecureField::zero(),
        ),
        circle_double::Component::new(
            location_allocator,
            circle_double::Eval {
                log_size: log_sizes[3],
            },
            SecureField::zero(),
        ),
        logup_sum::Component::new(
            location_allocator,
            logup_sum::Eval {
                log_size: log_sizes[4],
            },
            claimed_sum,
        ),
    )
}

/// Prove the recursion AIR over the given witness tables (Blake2s channel).
pub fn prove_recursion(
    traces: RecursionTraces,
    config: PcsConfig,
) -> RecursionProof<<Blake2sMerkleChannel as MerkleChannel>::H> {
    prove_recursion_with_channel::<Blake2sMerkleChannel>(traces, config)
}

/// Verify a recursion AIR proof (Blake2s channel).
pub fn verify_recursion(
    proof: RecursionProof<<Blake2sMerkleChannel as MerkleChannel>::H>,
    config: PcsConfig,
) -> Result<(), VerificationError> {
    verify_recursion_with_channel::<Blake2sMerkleChannel>(proof, config)
}

/// Prove the recursion AIR over the given witness tables with any Merkle
/// channel — in particular the Poseidon2-M31 channel whose hash the
/// recursion AIR itself proves.
pub fn prove_recursion_with_channel<MC: MerkleChannel>(
    traces: RecursionTraces,
    config: PcsConfig,
) -> RecursionProof<MC::H>
where
    SimdBackend: BackendForChannel<MC>,
{
    let qm31_mul_trace = traces.qm31_mul.into_witness();
    let qm31_inv_trace = traces.qm31_inv.into_witness();
    let fri_fold_trace = traces.fri_fold_line.into_witness();
    let circle_double_trace = traces.circle_double.into_witness();
    let logup_sum_trace = traces.logup_sum.into_witness();

    let log_size_of = |trace: &[stwo::prover::poly::circle::CircleEvaluation<
        SimdBackend,
        stwo::core::fields::m31::BaseField,
        stwo::prover::poly::BitReversedOrder,
    >]| {
        trace
            .first()
            .map(|t| t.domain.log_size())
            .expect("component trace is never empty after padding")
    };
    let log_sizes = [
        log_size_of(&qm31_mul_trace),
        log_size_of(&qm31_inv_trace),
        log_size_of(&fri_fold_trace),
        log_size_of(&circle_double_trace),
        log_size_of(&logup_sum_trace),
    ];
    let max_log_size = *log_sizes.iter().max().expect("non-empty");

    let twiddles = SimdBackend::precompute_twiddles(
        CanonicCoset::new(max_log_size + 2 + config.fri_config.log_blowup_factor)
            .circle_domain()
            .half_coset,
    );

    // Interaction trace of the logup_sum component, generated before the
    // channel work so the claimed sum is part of the claim.
    let (interaction_trace, claimed_sum) = logup_sum::gen_interaction_trace(&logup_sum_trace);

    let channel = &mut MC::C::default();
    let mut commitment_scheme = CommitmentSchemeProver::<_, MC>::new(config, &twiddles);

    // Tree 0: empty preprocessed trace.
    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(vec![]);
    tree_builder.commit(channel);

    mix_claim(channel, &log_sizes, claimed_sum);

    // Tree 1: all component tables, in the fixed commit order.
    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(
        qm31_mul_trace
            .into_iter()
            .chain(qm31_inv_trace)
            .chain(fri_fold_trace)
            .chain(circle_double_trace)
            .chain(logup_sum_trace)
            .collect::<Vec<_>>(),
    );
    tree_builder.commit(channel);

    // Tree 2: interaction trace (LogUp cumulative sums).
    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(interaction_trace);
    tree_builder.commit(channel);

    let mut location_allocator = TraceLocationAllocator::default();
    let (mul, inv, fold, double, sum) =
        components(&mut location_allocator, &log_sizes, claimed_sum);

    let stark_proof = prove(
        &[&mul, &inv, &fold, &double, &sum],
        channel,
        commitment_scheme,
    )
    .expect("recursion proof generation failed");

    RecursionProof {
        log_sizes,
        claimed_sum,
        stark_proof,
    }
}

/// Verify a recursion AIR proof with any Merkle channel.
pub fn verify_recursion_with_channel<MC: MerkleChannel>(
    proof: RecursionProof<MC::H>,
    config: PcsConfig,
) -> Result<(), VerificationError> {
    let channel = &mut MC::C::default();
    let mut commitment_scheme = CommitmentSchemeVerifier::<MC>::new(config);

    let commitments = &proof.stark_proof.commitments;
    commitment_scheme.commit(commitments[0], &[], channel);
    mix_claim(channel, &proof.log_sizes, proof.claimed_sum);
    commitment_scheme.commit(commitments[1], &column_log_sizes(&proof.log_sizes), channel);
    // Interaction tree: 4 base columns (one secure column) at the
    // logup_sum component's size.
    commitment_scheme.commit(commitments[2], &[proof.log_sizes[4]; 4], channel);

    let mut location_allocator = TraceLocationAllocator::default();
    let (mul, inv, fold, double, sum) =
        components(&mut location_allocator, &proof.log_sizes, proof.claimed_sum);

    verify(
        &[&mul, &inv, &fold, &double, &sum],
        channel,
        &mut commitment_scheme,
        proof.stark_proof,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};
    use stwo::core::circle::SECURE_FIELD_CIRCLE_GEN;
    use stwo::core::fields::m31::BaseField;
    use stwo::core::fields::qm31::QM31;

    fn random_qm31(rng: &mut SmallRng) -> QM31 {
        QM31::from_u32_unchecked(
            rng.gen_range(0..(1 << 30)),
            rng.gen_range(0..(1 << 30)),
            rng.gen_range(0..(1 << 30)),
            rng.gen_range(0..(1 << 30)),
        )
    }

    fn random_traces(seed: u64, rows: usize) -> (RecursionTraces, Vec<QM31>) {
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut traces = RecursionTraces::default();
        let mut terms = Vec::new();
        for _ in 0..rows {
            let a = random_qm31(&mut rng);
            let b = random_qm31(&mut rng);
            crate::qm31_mul::push_mul(&mut traces.qm31_mul, a, b);
            crate::qm31_inv::push_inv(&mut traces.qm31_inv, a);
            crate::fri_fold::push_fold_line(
                &mut traces.fri_fold_line,
                a,
                b,
                BaseField::from_u32_unchecked(rng.gen_range(1..(1 << 30))),
                random_qm31(&mut rng),
            );
            crate::circle_double::push_double(
                &mut traces.circle_double,
                SECURE_FIELD_CIRCLE_GEN.mul(rng.r#gen::<u128>()),
            );
            crate::logup_sum::push_term(&mut traces.logup_sum, b);
            terms.push(b);
        }
        (traces, terms)
    }

    #[test]
    fn test_recursion_air_prove_verify_roundtrip() {
        let (traces, _) = random_traces(0, 50);
        let proof = prove_recursion(traces, PcsConfig::default());
        verify_recursion(proof, PcsConfig::default()).expect("verification failed");
    }

    #[test]
    fn test_recursion_air_rejects_tampered_claim() {
        let (traces, _) = random_traces(1, 50);
        let mut proof = prove_recursion(traces, PcsConfig::default());
        // Lying about a component size breaks the channel binding.
        proof.log_sizes[0] += 1;
        assert!(verify_recursion(proof, PcsConfig::default()).is_err());
    }

    #[test]
    fn test_recursion_air_claimed_sum_is_sum_of_inverses() {
        let (traces, terms) = random_traces(2, 50);
        let proof = prove_recursion(traces, PcsConfig::default());
        assert_eq!(proof.claimed_sum, crate::logup_sum::expected_sum(&terms));
    }

    #[test]
    fn test_recursion_air_prove_verify_roundtrip_poseidon2_channel() {
        use prover::poseidon2_channel::Poseidon2M31MerkleChannel;
        let (traces, _) = random_traces(4, 50);
        let proof =
            prove_recursion_with_channel::<Poseidon2M31MerkleChannel>(traces, PcsConfig::default());
        verify_recursion_with_channel::<Poseidon2M31MerkleChannel>(proof, PcsConfig::default())
            .expect("poseidon2-channel verification failed");
    }

    #[test]
    fn test_recursion_air_rejects_tampered_claimed_sum() {
        let (traces, _) = random_traces(3, 50);
        let mut proof = prove_recursion(traces, PcsConfig::default());
        proof.claimed_sum += SecureField::from_u32_unchecked(1, 0, 0, 0);
        assert!(verify_recursion(proof, PcsConfig::default()).is_err());
    }
}
