//! Merkle component for partial tree nodes.

use num_traits::{One, Zero};
use stwo::core::ColumnVec;
use stwo::core::fields::m31::{BaseField, M31};
use stwo::core::fields::qm31::QM31;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::m31::PackedM31;
use stwo::prover::backend::simd::qm31::PackedQM31;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::LogupTraceGenerator;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::add_to_relation;
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

            add_to_relation!(
                eval,
                self.relations.merkle,
                left_multiplicity,
                index.clone(),
                depth.clone(),
                left_value.clone(),
                root.clone()
            );
            add_to_relation!(
                eval,
                self.relations.merkle,
                right_multiplicity,
                index.clone() + one.clone(),
                depth.clone(),
                right_value.clone(),
                root.clone()
            );
            add_to_relation!(
                eval,
                self.relations.merkle,
                -parent_multiplicity,
                index * inv2,
                depth - one.clone(),
                parent_value.clone(),
                root
            );

            add_to_relation!(
                eval,
                self.relations.poseidon2,
                enabler.clone(),
                left_value,
                right_value
            );
            add_to_relation!(eval, self.relations.poseidon2, -enabler, parent_value);
            eval.finalize_logup_in_pairs();
            eval
        }
    }
}

pub mod witness {
    use runner::trace::prover_columns::MerkleColumns;

    use super::*;
    use crate::{combine, write_col, write_pair};

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

        // Column order matches MerkleColumns.
        let cols = MerkleColumns::from_iter(trace.iter().map(|eval| &eval.values.data));

        let simd_size = cols.enabler.len();
        let log_size = trace[0].domain.log_size();
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let one = PackedM31::broadcast(M31::one());
        let inv2 = PackedM31::broadcast(M31::inverse(&M31::from(2)));

        let index_plus_one: Vec<PackedM31> = (0..simd_size).map(|i| cols.index[i] + one).collect();
        let index_div2: Vec<PackedM31> = (0..simd_size).map(|i| cols.index[i] * inv2).collect();
        let depth_minus_one: Vec<PackedM31> = (0..simd_size).map(|i| cols.depth[i] - one).collect();

        let left_mult: Vec<PackedQM31> = (0..simd_size)
            .map(|i| PackedQM31::from(cols.left_multiplicity[i]))
            .collect();
        let right_mult: Vec<PackedQM31> = (0..simd_size)
            .map(|i| PackedQM31::from(cols.right_multiplicity[i]))
            .collect();
        let neg_parent_mult: Vec<PackedQM31> = (0..simd_size)
            .map(|i| -PackedQM31::from(cols.parent_multiplicity[i]))
            .collect();
        let pos_enabler: Vec<PackedQM31> = (0..simd_size)
            .map(|i| PackedQM31::from(cols.enabler[i]))
            .collect();
        let neg_enabler: Vec<PackedQM31> = (0..simd_size)
            .map(|i| -PackedQM31::from(cols.enabler[i]))
            .collect();

        let left_denom = combine!(
            relations.merkle,
            [cols.index, cols.depth, cols.left_value, cols.root]
        );
        let right_denom = combine!(
            relations.merkle,
            [&index_plus_one, cols.depth, cols.right_value, cols.root]
        );
        let parent_denom = combine!(
            relations.merkle,
            [&index_div2, &depth_minus_one, cols.parent_value, cols.root]
        );
        let poseidon_in_denom = combine!(relations.poseidon2, [cols.left_value, cols.right_value]);
        let poseidon_out_denom = combine!(relations.poseidon2, [cols.parent_value]);

        write_pair!(
            &left_mult,
            &left_denom,
            &right_mult,
            &right_denom,
            interaction_trace
        );
        write_pair!(
            &neg_parent_mult,
            &parent_denom,
            &pos_enabler,
            &poseidon_in_denom,
            interaction_trace
        );
        write_col!(&neg_enabler, &poseidon_out_denom, interaction_trace);

        interaction_trace.finalize_last()
    }
}
