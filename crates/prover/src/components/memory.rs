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

use crate::add_to_relation;
use crate::commitment::{RW_MEMORY_BASE, RW_TREE_HEIGHT};
use crate::relations::Relations;

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
            let clk = cols.clk.clone();
            let value0 = cols.value0.clone();
            let value1 = cols.value1.clone();
            let value2 = cols.value2.clone();
            let value3 = cols.value3.clone();
            let multiplicity = cols.multiplicity.clone();
            let root = cols.root.clone();

            let one = E::F::one();
            let two = one.clone() + one.clone();
            let base = E::F::from(M31::from(RW_MEMORY_BASE));
            let leaf_depth = E::F::from(M31::from(RW_TREE_HEIGHT - 1));
            let three = two.clone() + one.clone();

            eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));
            eval.add_constraint(
                multiplicity.clone() * (multiplicity.clone() * multiplicity.clone() - one.clone()),
            );

            let rw_as = E::F::one();
            add_to_relation!(
                eval,
                self.relations.memory_access,
                multiplicity.clone(),
                rw_as,
                addr,
                clk,
                value0,
                value1,
                value2,
                value3
            );

            let index_base = addr - base;
            add_to_relation!(
                eval,
                self.relations.merkle,
                -enabler.clone(),
                index_base.clone(),
                leaf_depth.clone(),
                value0,
                root.clone()
            );
            add_to_relation!(
                eval,
                self.relations.merkle,
                -enabler.clone(),
                index_base.clone() + one.clone(),
                leaf_depth.clone(),
                value1,
                root.clone()
            );
            add_to_relation!(
                eval,
                self.relations.merkle,
                -enabler.clone(),
                index_base.clone() + two.clone(),
                leaf_depth.clone(),
                value2,
                root.clone()
            );
            add_to_relation!(
                eval,
                self.relations.merkle,
                -enabler,
                index_base + three,
                leaf_depth,
                value3,
                root
            );
            eval.finalize_logup_in_pairs();
            eval
        }
    }
}

pub mod witness {
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

        // Column order matches MemoryCommitColumns.
        let enabler = &trace[0].data;
        let addr = &trace[1].data;
        let clk = &trace[2].data;
        let value0 = &trace[3].data;
        let value1 = &trace[4].data;
        let value2 = &trace[5].data;
        let value3 = &trace[6].data;
        let multiplicity = &trace[7].data;
        let root = &trace[8].data;

        let base = PackedM31::broadcast(M31::from(RW_MEMORY_BASE));
        let leaf_depth = PackedM31::broadcast(M31::from(RW_TREE_HEIGHT - 1));
        let one = PackedM31::broadcast(M31::one());
        let two = one + one;
        let three = two + one;

        let simd_size = enabler.len();
        let log_size = trace[0].domain.log_size();
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let rw_as = PackedM31::broadcast(M31::one());
        let rw_as_col = vec![rw_as; simd_size];
        let leaf_depth_col = vec![leaf_depth; simd_size];
        let index_base: Vec<PackedM31> = (0..simd_size).map(|i| addr[i] - base).collect();
        let index_base_plus_one: Vec<PackedM31> =
            (0..simd_size).map(|i| index_base[i] + one).collect();
        let index_base_plus_two: Vec<PackedM31> =
            (0..simd_size).map(|i| index_base[i] + two).collect();
        let index_base_plus_three: Vec<PackedM31> =
            (0..simd_size).map(|i| index_base[i] + three).collect();

        let pos_mult: Vec<PackedQM31> = (0..simd_size)
            .map(|i| PackedQM31::from(multiplicity[i]))
            .collect();
        let neg_enabler: Vec<PackedQM31> = (0..simd_size)
            .map(|i| -PackedQM31::from(enabler[i]))
            .collect();

        let memory_denom = combine!(
            relations.memory_access,
            [&rw_as_col, addr, clk, value0, value1, value2, value3]
        );
        let merkle_0_denom = combine!(
            relations.merkle,
            [&index_base, &leaf_depth_col, value0, root]
        );
        let merkle_1_denom = combine!(
            relations.merkle,
            [&index_base_plus_one, &leaf_depth_col, value1, root]
        );
        let merkle_2_denom = combine!(
            relations.merkle,
            [&index_base_plus_two, &leaf_depth_col, value2, root]
        );
        let merkle_3_denom = combine!(
            relations.merkle,
            [&index_base_plus_three, &leaf_depth_col, value3, root]
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
}
