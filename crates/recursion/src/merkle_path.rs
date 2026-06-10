//! Merkle hash-step component: witness generation and AIR evaluation.
//!
//! Each enabled row claims `parent = permute(left || right)[..8]` through the
//! stark-v poseidon2 relation: it emits the 16-word permutation input and
//! consumes the 8-word wide output. The permutation constraints live solely
//! in the reused `prover::components::poseidon2` component, whose witness
//! table carries one wide row per hash step — no hash constraint is copied.

use prover::relations::Relations;
use runner::poseidon2::{T, poseidon2_traced_state};
use runner::trace::Poseidon2Table;
use stwo::core::ColumnVec;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::QM31;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::qm31::PackedQM31;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, LogupTraceGenerator, RelationEntry,
};

use crate::MerklePathTable;
use crate::prover_columns::MerklePathColumns;

pub type Component = FrameworkComponent<Eval>;

#[derive(Clone)]
pub struct Eval {
    pub log_size: u32,
    pub relations: Relations,
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let cols = MerklePathColumns::from_eval(&mut eval);
        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }
        // Emit the permutation input (consumed by the poseidon2 component).
        eval.add_to_relation(RelationEntry::new(
            &self.relations.poseidon2,
            E::EF::from(cols.enabler.clone()),
            &[
                cols.left_0.clone(),
                cols.left_1.clone(),
                cols.left_2.clone(),
                cols.left_3.clone(),
                cols.left_4.clone(),
                cols.left_5.clone(),
                cols.left_6.clone(),
                cols.left_7.clone(),
                cols.right_0.clone(),
                cols.right_1.clone(),
                cols.right_2.clone(),
                cols.right_3.clone(),
                cols.right_4.clone(),
                cols.right_5.clone(),
                cols.right_6.clone(),
                cols.right_7.clone(),
            ],
        ));
        // Consume the wide digest (emitted by the poseidon2 component).
        eval.add_to_relation(RelationEntry::new(
            &self.relations.poseidon2,
            -E::EF::from(cols.enabler.clone()),
            &[
                cols.parent_0.clone(),
                cols.parent_1.clone(),
                cols.parent_2.clone(),
                cols.parent_3.clone(),
                cols.parent_4.clone(),
                cols.parent_5.clone(),
                cols.parent_6.clone(),
                cols.parent_7.clone(),
            ],
        ));
        eval.finalize_logup_in_pairs();
        eval
    }
}

/// Record one hash step: push the binding row here and the wide permutation
/// row into the poseidon2 witness table, returning the parent digest.
pub fn push_hash_step(
    table: &mut MerklePathTable,
    poseidon2: &mut Poseidon2Table,
    left: [u32; 8],
    right: [u32; 8],
) -> [u32; 8] {
    let mut state = [0u32; T];
    state[..8].copy_from_slice(&left);
    state[8..].copy_from_slice(&right);
    let row = poseidon2_traced_state(state, true);
    poseidon2.push_row(&row);

    let parent: [u32; 8] = row[runner::poseidon2::POSEIDON2_FINAL_STATE_START
        ..runner::poseidon2::POSEIDON2_FINAL_STATE_START + 8]
        .try_into()
        .expect("8 words");
    table.push(
        left[0], left[1], left[2], left[3], left[4], left[5], left[6], left[7], right[0], right[1],
        right[2], right[3], right[4], right[5], right[6], right[7], parent[0], parent[1],
        parent[2], parent[3], parent[4], parent[5], parent[6], parent[7],
    );
    parent
}

/// Generate the interaction trace and the claimed sum of the two relation
/// entries (cancels against the poseidon2 component's wide rows).
pub fn gen_interaction_trace(
    trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
    relations: &Relations,
) -> (
    ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    QM31,
) {
    let cols = MerklePathColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.enabler.len();
    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    let pos_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(cols.enabler[i]))
        .collect();
    let neg_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(cols.enabler[i]))
        .collect();

    let input_denom = combine!(
        relations.poseidon2,
        [
            cols.left_0,
            cols.left_1,
            cols.left_2,
            cols.left_3,
            cols.left_4,
            cols.left_5,
            cols.left_6,
            cols.left_7,
            cols.right_0,
            cols.right_1,
            cols.right_2,
            cols.right_3,
            cols.right_4,
            cols.right_5,
            cols.right_6,
            cols.right_7
        ]
    );
    let parent_denom = combine!(
        relations.poseidon2,
        [
            cols.parent_0,
            cols.parent_1,
            cols.parent_2,
            cols.parent_3,
            cols.parent_4,
            cols.parent_5,
            cols.parent_6,
            cols.parent_7
        ]
    );

    write_pair!(
        &pos_enabler,
        &input_denom,
        &neg_enabler,
        &parent_denom,
        logup_gen
    );
    logup_gen.finalize_last()
}
