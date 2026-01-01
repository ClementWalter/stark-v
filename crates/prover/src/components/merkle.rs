//! Merkle component for partial tree nodes.

use num_traits::{One, Zero};
use stwo::core::ColumnVec;
use stwo::core::fields::m31::{BaseField, M31};
use stwo::core::fields::qm31::QM31;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::m31::{LOG_N_LANES, PackedM31};
use stwo::prover::backend::simd::qm31::PackedQM31;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::LogupTraceGenerator;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval, RelationEntry};

use crate::relations::Relations;

pub mod columns {
    pub use runner::trace::prover_columns::MerkleColumns;
}

pub mod air {
    use super::columns::MerkleColumns;
    use super::*;

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
            let cols = MerkleColumns::from_eval(&mut eval);
            let enabler = cols.enabler.clone();
            let index = cols.index.clone();
            let depth = cols.depth.clone();
            let left_value = cols.left_value.clone();
            let right_value = cols.right_value.clone();
            let parent_value = cols.parent_value.clone();
            let left_multiplicity = cols.left_multiplicity.clone();
            let right_multiplicity = cols.right_multiplicity.clone();
            let parent_multiplicity = cols.parent_multiplicity.clone();
            let root = cols.root.clone();

            let one = E::F::one();
            let two = one.clone() + one.clone();
            let inv2 = E::F::from(M31::inverse(&M31::from(2)));

            eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));
            eval.add_constraint(
                left_multiplicity.clone()
                    * (left_multiplicity.clone() - one.clone())
                    * (left_multiplicity.clone() - two.clone()),
            );
            eval.add_constraint(
                right_multiplicity.clone()
                    * (right_multiplicity.clone() - one.clone())
                    * (right_multiplicity.clone() - two.clone()),
            );
            eval.add_constraint(
                parent_multiplicity.clone()
                    * (parent_multiplicity.clone() - one.clone())
                    * (parent_multiplicity.clone() - two.clone()),
            );

            eval.add_to_relation(RelationEntry::new(
                &self.relations.merkle,
                E::EF::from(left_multiplicity),
                &[
                    index.clone(),
                    depth.clone(),
                    left_value.clone(),
                    root.clone(),
                ],
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.merkle,
                E::EF::from(right_multiplicity),
                &[
                    index.clone() + one.clone(),
                    depth.clone(),
                    right_value.clone(),
                    root.clone(),
                ],
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.merkle,
                -E::EF::from(parent_multiplicity),
                &[
                    index * inv2,
                    depth - one.clone(),
                    parent_value.clone(),
                    root,
                ],
            ));

            eval.add_to_relation(RelationEntry::new(
                &self.relations.poseidon2,
                E::EF::from(enabler.clone()),
                &[left_value, right_value],
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.poseidon2,
                -E::EF::from(enabler),
                &[parent_value],
            ));
            eval.finalize_logup_in_pairs();
            eval
        }
    }
}

pub mod witness {
    use super::*;

    pub fn gen_interaction_trace(
        trace: &ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        relations: &Relations,
    ) -> (
        ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        QM31,
    ) {
        if trace.is_empty() {
            return (vec![], QM31::zero());
        }

        let log_size = trace[0].domain.log_size();
        let packed_len = 1usize << (log_size - LOG_N_LANES) as usize;

        // Column order matches MerkleColumns.
        let enabler = &trace[0].data;
        let index = &trace[1].data;
        let depth = &trace[2].data;
        let left_value = &trace[3].data;
        let right_value = &trace[4].data;
        let parent_value = &trace[5].data;
        let left_multiplicity = &trace[6].data;
        let right_multiplicity = &trace[7].data;
        let parent_multiplicity = &trace[8].data;
        let root = &trace[9].data;

        let one = PackedM31::broadcast(M31::one());
        let inv2 = PackedM31::broadcast(M31::inverse(&M31::from(2)));

        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let mut col = interaction_trace.new_col();
        for row in 0..packed_len {
            let index_row = index[row];
            let depth_row = depth[row];
            let left_row = left_value[row];
            let right_row = right_value[row];
            let left_mult = left_multiplicity[row];
            let right_mult = right_multiplicity[row];
            let root_row = root[row];

            let num0: PackedQM31 = PackedQM31::from(left_mult);
            let denom0: PackedQM31 = relations
                .merkle
                .combine(&[index_row, depth_row, left_row, root_row]);

            let num1: PackedQM31 = PackedQM31::from(right_mult);
            let denom1: PackedQM31 =
                relations
                    .merkle
                    .combine(&[index_row + one, depth_row, right_row, root_row]);

            let numerator = num0 * denom1 + num1 * denom0;
            let denom = denom0 * denom1;
            col.write_frac(row, numerator, denom);
        }
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        for row in 0..packed_len {
            let enabler_row = enabler[row];
            let index_row = index[row];
            let depth_row = depth[row];
            let left_row = left_value[row];
            let right_row = right_value[row];
            let parent_row = parent_value[row];
            let parent_mult = parent_multiplicity[row];
            let root_row = root[row];

            let num0: PackedQM31 = -PackedQM31::from(parent_mult);
            let denom0: PackedQM31 = relations.merkle.combine(&[
                index_row * inv2,
                depth_row - one,
                parent_row,
                root_row,
            ]);

            let num1: PackedQM31 = PackedQM31::from(enabler_row);
            let denom1: PackedQM31 = relations.poseidon2.combine(&[left_row, right_row]);

            let numerator = num0 * denom1 + num1 * denom0;
            let denom = denom0 * denom1;
            col.write_frac(row, numerator, denom);
        }
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        for row in 0..packed_len {
            let enabler_row = enabler[row];
            let parent_row = parent_value[row];

            let numerator: PackedQM31 = -PackedQM31::from(enabler_row);
            let denom: PackedQM31 = relations.poseidon2.combine(&[parent_row]);
            col.write_frac(row, numerator, denom);
        }
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        (trace, claimed_sum)
    }
}
