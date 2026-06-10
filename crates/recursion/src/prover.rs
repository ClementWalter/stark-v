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

use prover::relations::Relations;
use runner::trace::Poseidon2Table;

use crate::relations::RecursionRelations;

use crate::{
    CircleDoubleTable, FriFoldLineTable, LogupSumTable, MerklePathTable, Qm31InvTable,
    Qm31MulTable, circle_double, fri_fold, logup_sum, merkle_path, prover_columns, qm31_inv,
    qm31_mul,
};

/// Witness tables of the recursion AIR, one per component.
#[derive(Default)]
pub struct RecursionTraces {
    pub qm31_mul: Qm31MulTable,
    pub qm31_inv: Qm31InvTable,
    pub fri_fold_line: FriFoldLineTable,
    pub circle_double: CircleDoubleTable,
    pub logup_sum: LogupSumTable,
    pub merkle_path: MerklePathTable,
    pub poseidon2: Poseidon2Table,
}

/// Proof of the recursion AIR plus the public claim (per-component sizes
/// and the LogUp claimed sum).
pub struct RecursionProof<H: MerkleHasherLifted> {
    /// Log sizes of (qm31_mul, qm31_inv, fri_fold_line, circle_double,
    /// logup_sum, merkle_path, poseidon2).
    pub log_sizes: [u32; 7],
    /// Claimed sum of the logup_sum component: Σ enabler / term.
    pub claimed_sum: SecureField,
    /// Claimed sums of the merkle_path and poseidon2 components; together
    /// with the public root terms their total must vanish (every hash claim
    /// discharged by a permutation row, every path anchored at a root).
    pub merkle_claimed_sum: SecureField,
    pub poseidon2_claimed_sum: SecureField,
    /// Merkle roots the decommitment paths anchor to.
    pub roots: Vec<RootClaim>,
    pub stark_proof: StarkProof<H>,
}

/// A public root anchor: `n_paths` decommitment paths of tree `tree_id`
/// terminate at `root`.
#[derive(Clone, Debug)]
pub struct RootClaim {
    pub tree_id: u32,
    pub root: [u32; 8],
    pub n_paths: u32,
}

fn mix_roots<C: Channel>(channel: &mut C, roots: &[RootClaim]) {
    channel.mix_u32s(&[roots.len() as u32]);
    for root in roots {
        channel.mix_u32s(&[root.tree_id, root.n_paths]);
        channel.mix_u32s(&root.root);
    }
}

/// The LogUp contribution of the public root anchors: each path's top row
/// consumes the root claim, so the public side emits it `n_paths` times.
fn public_root_terms(roots: &[RootClaim], recursion_relations: &RecursionRelations) -> SecureField {
    use stwo::core::fields::FieldExpOps;
    use stwo::core::fields::m31::M31;
    use stwo_constraint_framework::Relation;

    let mut total = SecureField::zero();
    for root in roots {
        let mut tuple = [M31::from(0u32); 11];
        tuple[0] = M31::from(root.tree_id);
        // depth = 0, index = 0
        for (slot, &word) in tuple[3..].iter_mut().zip(root.root.iter()) {
            *slot = M31::from(word);
        }
        let denom: SecureField = recursion_relations.merkle_node.combine(&tuple);
        total += denom.inverse() * SecureField::from(M31::from(root.n_paths));
    }
    total
}

fn mix_claim<C: Channel>(channel: &mut C, log_sizes: &[u32; 7], sums: [SecureField; 3]) {
    channel.mix_u32s(log_sizes);
    channel.mix_felts(&sums);
}

/// Trace-tree column log sizes in commit order.
fn column_log_sizes(log_sizes: &[u32; 7]) -> Vec<u32> {
    let widths = [
        prover_columns::Qm31MulColumns::<()>::SIZE,
        prover_columns::Qm31InvColumns::<()>::SIZE,
        prover_columns::FriFoldLineColumns::<()>::SIZE,
        prover_columns::CircleDoubleColumns::<()>::SIZE,
        prover_columns::LogupSumColumns::<()>::SIZE,
        prover_columns::MerklePathColumns::<()>::SIZE,
        runner::trace::prover_columns::Poseidon2Columns::<()>::SIZE,
    ];
    log_sizes
        .iter()
        .zip(widths)
        .flat_map(|(&log_size, width)| std::iter::repeat_n(log_size, width))
        .collect()
}

/// Build the four components in commit order against a shared allocator.
#[allow(clippy::type_complexity)]
fn components(
    location_allocator: &mut TraceLocationAllocator,
    log_sizes: &[u32; 7],
    sums: [SecureField; 3],
    relations: &Relations,
    recursion_relations: &RecursionRelations,
) -> (
    qm31_mul::Component,
    qm31_inv::Component,
    fri_fold::Component,
    circle_double::Component,
    logup_sum::Component,
    merkle_path::Component,
    prover::components::poseidon2::air::Component,
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
            sums[0],
        ),
        merkle_path::Component::new(
            location_allocator,
            merkle_path::Eval {
                log_size: log_sizes[5],
                relations: relations.clone(),
                recursion_relations: recursion_relations.clone(),
            },
            sums[1],
        ),
        prover::components::poseidon2::air::Component::new(
            location_allocator,
            prover::components::poseidon2::air::Eval {
                log_size: log_sizes[6],
                relations: relations.clone(),
            },
            sums[2],
        ),
    )
}

/// Prove the recursion AIR over the given witness tables (Blake2s channel).
pub fn prove_recursion(
    traces: RecursionTraces,
    roots: Vec<RootClaim>,
    config: PcsConfig,
) -> RecursionProof<<Blake2sMerkleChannel as MerkleChannel>::H> {
    prove_recursion_with_channel::<Blake2sMerkleChannel>(traces, roots, config)
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
    roots: Vec<RootClaim>,
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
    let merkle_path_trace = traces.merkle_path.into_witness();
    let poseidon2_trace = traces.poseidon2.into_witness();

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
        log_size_of(&merkle_path_trace),
        log_size_of(&poseidon2_trace),
    ];
    let max_log_size = *log_sizes.iter().max().expect("non-empty");

    let twiddles = SimdBackend::precompute_twiddles(
        CanonicCoset::new(max_log_size + 2 + config.fri_config.log_blowup_factor)
            .circle_domain()
            .half_coset,
    );

    let channel = &mut MC::C::default();
    let mut commitment_scheme = CommitmentSchemeProver::<_, MC>::new(config, &twiddles);

    // Tree 0: empty preprocessed trace.
    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(vec![]);
    tree_builder.commit(channel);

    channel.mix_u32s(&log_sizes);
    mix_roots(channel, &roots);

    // Tree 1: all component tables, in the fixed commit order.
    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(
        qm31_mul_trace
            .into_iter()
            .chain(qm31_inv_trace)
            .chain(fri_fold_trace)
            .chain(circle_double_trace)
            .chain(logup_sum_trace.iter().cloned())
            .chain(merkle_path_trace.iter().cloned())
            .chain(poseidon2_trace.iter().cloned())
            .collect::<Vec<_>>(),
    );
    tree_builder.commit(channel);

    // Lookup elements are drawn after the main commitment (Fiat-Shamir).
    let relations = Relations::draw(channel);
    let recursion_relations = RecursionRelations::draw(channel);

    // Interaction traces, in component order.
    let (logup_interaction, claimed_sum) = logup_sum::gen_interaction_trace(&logup_sum_trace);
    let (merkle_interaction, merkle_claimed_sum) =
        merkle_path::gen_interaction_trace(&merkle_path_trace, &relations, &recursion_relations);
    let (poseidon2_interaction, poseidon2_claimed_sum) =
        prover::components::poseidon2::witness::gen_interaction_trace(&poseidon2_trace, &relations);

    let sums = [claimed_sum, merkle_claimed_sum, poseidon2_claimed_sum];
    mix_claim(channel, &log_sizes, sums);

    // Tree 2: interaction traces (LogUp cumulative sums), in component order.
    let mut tree_builder = commitment_scheme.tree_builder();
    tree_builder.extend_evals(
        logup_interaction
            .into_iter()
            .chain(merkle_interaction)
            .chain(poseidon2_interaction)
            .collect::<Vec<_>>(),
    );
    tree_builder.commit(channel);

    let mut location_allocator = TraceLocationAllocator::default();
    let (mul, inv, fold, double, sum, merkle, poseidon2) = components(
        &mut location_allocator,
        &log_sizes,
        sums,
        &relations,
        &recursion_relations,
    );

    let stark_proof = prove(
        &[&mul, &inv, &fold, &double, &sum, &merkle, &poseidon2],
        channel,
        commitment_scheme,
    )
    .expect("recursion proof generation failed");

    RecursionProof {
        log_sizes,
        claimed_sum,
        merkle_claimed_sum,
        poseidon2_claimed_sum,
        roots,
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
    channel.mix_u32s(&proof.log_sizes);
    mix_roots(channel, &proof.roots);
    commitment_scheme.commit(commitments[1], &column_log_sizes(&proof.log_sizes), channel);

    let relations = Relations::draw(channel);
    let recursion_relations = RecursionRelations::draw(channel);

    // Every hash claim must be discharged by a permutation row, and every
    // path must anchor at a public root.
    let total = proof.merkle_claimed_sum
        + proof.poseidon2_claimed_sum
        + public_root_terms(&proof.roots, &recursion_relations);
    if !total.is_zero() {
        return Err(VerificationError::InvalidStructure(
            "merkle/poseidon2/root logup sums do not cancel".to_string(),
        ));
    }
    let sums = [
        proof.claimed_sum,
        proof.merkle_claimed_sum,
        proof.poseidon2_claimed_sum,
    ];
    mix_claim(channel, &proof.log_sizes, sums);
    // Interaction tree: one secure column (4 base) for logup_sum, two for
    // merkle_path (hash binding + node chaining), two for poseidon2.
    let interaction_log_sizes: Vec<u32> = std::iter::repeat_n(proof.log_sizes[4], 4)
        .chain(std::iter::repeat_n(proof.log_sizes[5], 8))
        .chain(std::iter::repeat_n(proof.log_sizes[6], 8))
        .collect();
    commitment_scheme.commit(commitments[2], &interaction_log_sizes, channel);

    let mut location_allocator = TraceLocationAllocator::default();
    let (mul, inv, fold, double, sum, merkle, poseidon2) = components(
        &mut location_allocator,
        &proof.log_sizes,
        sums,
        &relations,
        &recursion_relations,
    );

    verify(
        &[&mul, &inv, &fold, &double, &sum, &merkle, &poseidon2],
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

    fn random_traces(seed: u64, rows: usize) -> (RecursionTraces, Vec<QM31>, Vec<RootClaim>) {
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

        // Two-level decommitment paths: leaf -> mid -> root, one per tree.
        let mut roots = Vec::new();
        for tree_id in 0..4u32 {
            let child: [u32; 8] = std::array::from_fn(|_| rng.gen_range(0..(1 << 30)));
            let steps = [
                crate::merkle_path::PathStep {
                    direction: rng.gen_range(0..2),
                    sibling: std::array::from_fn(|_| rng.gen_range(0..(1 << 30))),
                },
                crate::merkle_path::PathStep {
                    direction: rng.gen_range(0..2),
                    sibling: std::array::from_fn(|_| rng.gen_range(0..(1 << 30))),
                },
            ];
            // Bottom-up: hash the leaf-side step (depth 1), then the root
            // step (depth 0); indices follow the directions.
            let mid = crate::merkle_path::push_path_step(
                &mut traces.merkle_path,
                &mut traces.poseidon2,
                tree_id,
                1,
                steps[0].direction,
                child,
                steps[1],
                true,
            );
            let root = crate::merkle_path::push_path_step(
                &mut traces.merkle_path,
                &mut traces.poseidon2,
                tree_id,
                0,
                0,
                mid,
                steps[0],
                false,
            );
            roots.push(RootClaim {
                tree_id,
                root,
                n_paths: 1,
            });
        }
        (traces, terms, roots)
    }

    #[test]
    fn test_recursion_air_prove_verify_roundtrip() {
        let (traces, _, roots) = random_traces(0, 50);
        let proof = prove_recursion(traces, roots, PcsConfig::default());
        verify_recursion(proof, PcsConfig::default()).expect("verification failed");
    }

    #[test]
    fn test_recursion_air_rejects_tampered_claim() {
        let (traces, _, roots) = random_traces(1, 50);
        let mut proof = prove_recursion(traces, roots, PcsConfig::default());
        // Lying about a component size breaks the channel binding.
        proof.log_sizes[0] += 1;
        assert!(verify_recursion(proof, PcsConfig::default()).is_err());
    }

    #[test]
    fn test_recursion_air_claimed_sum_is_sum_of_inverses() {
        let (traces, terms, roots) = random_traces(2, 50);
        let proof = prove_recursion(traces, roots, PcsConfig::default());
        assert_eq!(proof.claimed_sum, crate::logup_sum::expected_sum(&terms));
    }

    #[test]
    fn test_recursion_air_prove_verify_roundtrip_poseidon2_channel() {
        use prover::poseidon2_channel::Poseidon2M31MerkleChannel;
        let (traces, _, roots) = random_traces(4, 50);
        let proof = prove_recursion_with_channel::<Poseidon2M31MerkleChannel>(
            traces,
            roots,
            PcsConfig::default(),
        );
        verify_recursion_with_channel::<Poseidon2M31MerkleChannel>(proof, PcsConfig::default())
            .expect("poseidon2-channel verification failed");
    }

    #[test]
    fn test_recursion_air_merkle_and_poseidon2_sums_cancel() {
        let (traces, _, roots) = random_traces(5, 20);
        let proof = prove_recursion(traces, roots, PcsConfig::default());
        // Hash claims cancel between merkle_path and poseidon2; the node
        // claims cancel against the public root terms checked in verify.
        verify_recursion(proof, PcsConfig::default()).expect("verification failed");
    }

    #[test]
    fn test_recursion_air_rejects_undischarged_hash_claim() {
        let (mut traces, _, roots) = random_traces(6, 20);
        // Corrupt one parent limb: the merkle_path row now consumes a tuple
        // no permutation row emits, and its node claim no longer anchors.
        traces.merkle_path.parent_0[0] += 1;
        let proof = prove_recursion(traces, roots, PcsConfig::default());
        assert!(verify_recursion(proof, PcsConfig::default()).is_err());
    }

    #[test]
    fn test_recursion_air_rejects_wrong_root_claim() {
        let (traces, _, mut roots) = random_traces(7, 20);
        roots[0].root[0] += 1;
        let proof = prove_recursion(traces, roots, PcsConfig::default());
        assert!(verify_recursion(proof, PcsConfig::default()).is_err());
    }

    #[test]
    fn test_recursion_air_rejects_broken_path_chaining() {
        let (mut traces, _, roots) = random_traces(8, 20);
        // Corrupt an index: the child claim emitted above no longer matches
        // the claim this row consumes.
        traces.merkle_path.index[0] += 1;
        let proof = prove_recursion(traces, roots, PcsConfig::default());
        assert!(verify_recursion(proof, PcsConfig::default()).is_err());
    }

    #[test]
    fn test_recursion_air_rejects_tampered_claimed_sum() {
        let (traces, _, roots) = random_traces(3, 50);
        let mut proof = prove_recursion(traces, roots, PcsConfig::default());
        proof.claimed_sum += SecureField::from_u32_unchecked(1, 0, 0, 0);
        assert!(verify_recursion(proof, PcsConfig::default()).is_err());
    }
}
