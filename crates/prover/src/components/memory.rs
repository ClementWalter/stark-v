//! Memory commitment component (initial/final values).

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

use crate::relations::Relations;
use runner::MAX_TREE_HEIGHT;

pub mod columns {
    pub use runner::trace::prover_columns::MemoryColumns;
}

pub mod air {
    use super::columns::MemoryColumns;
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
            let cols = MemoryColumns::from_eval(&mut eval);
            let enabler = cols.enabler.clone();
            let addr = cols.addr.clone();
            let clock = cols.clock.clone();
            let value_0 = cols.value_0.clone();
            let value_1 = cols.value_1.clone();
            let value_2 = cols.value_2.clone();
            let value_3 = cols.value_3.clone();
            let multiplicity = cols.multiplicity.clone();
            let root = cols.root.clone();

            let one = E::F::one();
            let two = one.clone() + one.clone();
            let leaf_depth = E::F::from(M31::from(MAX_TREE_HEIGHT - 1));
            let three = two.clone() + one.clone();

            eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));
            eval.add_constraint(
                multiplicity.clone() * (multiplicity.clone() * multiplicity.clone() - one.clone()),
            );

            add_to_relation!(
                eval,
                self.relations.range_check_8_8,
                -enabler.clone(),
                value_0.clone(),
                value_1.clone()
            );

            add_to_relation!(
                eval,
                self.relations.range_check_8_8,
                -enabler.clone(),
                value_2.clone(),
                value_3.clone()
            );

            let rw_as = E::F::one();
            add_to_relation!(
                eval,
                self.relations.memory_access,
                multiplicity.clone(),
                rw_as,
                addr,
                clock,
                value_0,
                value_1,
                value_2,
                value_3
            );

            let index_base = addr;
            add_to_relation!(
                eval,
                self.relations.merkle,
                -enabler.clone(),
                index_base.clone(),
                leaf_depth.clone(),
                value_0,
                root.clone()
            );
            add_to_relation!(
                eval,
                self.relations.merkle,
                -enabler.clone(),
                index_base.clone() + one.clone(),
                leaf_depth.clone(),
                value_1,
                root.clone()
            );
            add_to_relation!(
                eval,
                self.relations.merkle,
                -enabler.clone(),
                index_base.clone() + two.clone(),
                leaf_depth.clone(),
                value_2,
                root.clone()
            );
            add_to_relation!(
                eval,
                self.relations.merkle,
                -enabler,
                index_base + three,
                leaf_depth,
                value_3,
                root
            );
            eval.finalize_logup_in_pairs();
            eval
        }
    }
}

pub mod witness {
    use runner::trace::prover_columns::MemoryColumns;

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

        // Column order matches MemoryColumns.
        let cols = MemoryColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
        let simd_size = cols.enabler.len();

        let log_size = trace[0].domain.log_size();
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let leaf_depth = PackedM31::broadcast(M31::from(MAX_TREE_HEIGHT - 1));
        let one = PackedM31::broadcast(M31::one());
        let two = one + one;
        let three = two + one;

        let rw_as = PackedM31::broadcast(M31::one());
        let rw_as_col = vec![rw_as; simd_size];
        let leaf_depth_col = vec![leaf_depth; simd_size];
        let index_base: Vec<PackedM31> = cols.addr.to_vec();
        let index_base_plus_one: Vec<PackedM31> =
            (0..simd_size).map(|i| index_base[i] + one).collect();
        let index_base_plus_two: Vec<PackedM31> =
            (0..simd_size).map(|i| index_base[i] + two).collect();
        let index_base_plus_three: Vec<PackedM31> =
            (0..simd_size).map(|i| index_base[i] + three).collect();

        let pos_mult: Vec<PackedQM31> = (0..simd_size)
            .map(|i| PackedQM31::from(cols.multiplicity[i]))
            .collect();
        let neg_enabler: Vec<PackedQM31> = (0..simd_size)
            .map(|i| -PackedQM31::from(cols.enabler[i]))
            .collect();

        let range_check_8_8_0_denom =
            combine!(relations.range_check_8_8, [&cols.value_0, &cols.value_1]);
        let range_check_8_8_1_denom =
            combine!(relations.range_check_8_8, [&cols.value_2, &cols.value_3]);

        let memory_denom = combine!(
            relations.memory_access,
            [
                &rw_as_col,
                cols.addr,
                cols.clock,
                cols.value_0,
                cols.value_1,
                cols.value_2,
                cols.value_3
            ]
        );
        let merkle_0_denom = combine!(
            relations.merkle,
            [&index_base, &leaf_depth_col, cols.value_0, cols.root]
        );
        let merkle_1_denom = combine!(
            relations.merkle,
            [
                &index_base_plus_one,
                &leaf_depth_col,
                cols.value_1,
                cols.root
            ]
        );
        let merkle_2_denom = combine!(
            relations.merkle,
            [
                &index_base_plus_two,
                &leaf_depth_col,
                cols.value_2,
                cols.root
            ]
        );
        let merkle_3_denom = combine!(
            relations.merkle,
            [
                &index_base_plus_three,
                &leaf_depth_col,
                cols.value_3,
                cols.root
            ]
        );

        write_pair!(
            &neg_enabler,
            &range_check_8_8_0_denom,
            &neg_enabler,
            &range_check_8_8_1_denom,
            interaction_trace
        );

        write_pair!(
            &pos_mult,
            &memory_denom,
            &neg_enabler,
            &merkle_0_denom,
            interaction_trace
        );
        write_pair!(
            &neg_enabler,
            &merkle_1_denom,
            &neg_enabler,
            &merkle_2_denom,
            interaction_trace
        );
        write_col!(&neg_enabler, &merkle_3_denom, interaction_trace);

        interaction_trace.finalize_last()
    }

    /// Register multiplicities for preprocessed lookups.
    pub fn register_multiplicities(
        trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
        counters: &mut crate::relations::Counters,
    ) {
        if trace.is_empty() {
            return;
        }

        let cols = MemoryColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
        let simd_size = cols.enabler.len();

        // Numerator: negated enabler (to match gen_interaction_trace)
        let neg_enabler: Vec<PackedM31> = (0..simd_size).map(|i| -cols.enabler[i]).collect();

        // Register range_check_8_8 with negated multiplicity
        counters
            .range_check_8_8
            .register_many(&neg_enabler, &[cols.value_0, cols.value_1]);
        counters
            .range_check_8_8
            .register_many(&neg_enabler, &[cols.value_2, cols.value_3]);
    }
}
