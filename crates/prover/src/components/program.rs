//! Program component for decoded instruction rows.

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
use crate::commitment::{PROGRAM_BASE, PROGRAM_TREE_HEIGHT};
use crate::relations::Relations;

pub mod columns {
    pub use runner::trace::prover_columns::ProgramColumns;
}

pub mod air {
    use super::columns::ProgramColumns;
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
            let cols = ProgramColumns::from_eval(&mut eval);
            let enabler = cols.enabler.clone();
            let addr = cols.addr.clone();
            let value0 = cols.value0.clone();
            let value1 = cols.value1.clone();
            let value2 = cols.value2.clone();
            let value3 = cols.value3.clone();
            let multiplicity = cols.multiplicity.clone();
            let root = cols.root.clone();

            let base = E::F::from(M31::from(PROGRAM_BASE));
            let leaf_depth = E::F::from(M31::from(PROGRAM_TREE_HEIGHT - 1));
            let one = E::F::one();
            let two = one.clone() + one.clone();
            let three = two.clone() + one.clone();

            eval.add_constraint(enabler.clone() * (one.clone() - enabler.clone()));

            add_to_relation!(
                eval,
                self.relations.program_access,
                multiplicity.clone(),
                addr,
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
    use super::columns::ProgramColumns;

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

        let cols = ProgramColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
        let simd_size = cols.addr.len();

        let log_size = trace[0].domain.log_size();
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        // Constants
        let base = PackedM31::broadcast(M31::from(PROGRAM_BASE));
        let leaf_depth = PackedM31::broadcast(M31::from(PROGRAM_TREE_HEIGHT - 1));
        let one = PackedM31::broadcast(M31::one());
        let two = one + one;
        let three = two + one;

        // Compute derived columns
        let leaf_depth_col = vec![leaf_depth; simd_size];
        let index_base: Vec<PackedM31> = (0..simd_size).map(|i| cols.addr[i] - base).collect();
        let index_base_plus_one: Vec<PackedM31> =
            (0..simd_size).map(|i| index_base[i] + one).collect();
        let index_base_plus_two: Vec<PackedM31> =
            (0..simd_size).map(|i| index_base[i] + two).collect();
        let index_base_plus_three: Vec<PackedM31> =
            (0..simd_size).map(|i| index_base[i] + three).collect();

        // =====================================================================
        // LogUp entries (same order as AIR)
        // =====================================================================

        // 1. program_access: + multiplicity * (addr, value0, value1, value2, value3)
        let pos_mult: Vec<PackedQM31> = (0..simd_size)
            .map(|i| PackedQM31::from(cols.multiplicity[i]))
            .collect();

        let program_denom = combine!(
            relations.program_access,
            [
                cols.addr,
                cols.value0,
                cols.value1,
                cols.value2,
                cols.value3
            ]
        );

        // 2. merkle: -enabler * (index_base, leaf_depth, value0, root)
        let neg_enabler: Vec<PackedQM31> = (0..simd_size)
            .map(|i| -PackedQM31::from(cols.enabler[i]))
            .collect();

        let merkle_0_denom = combine!(
            relations.merkle,
            [&index_base, &leaf_depth_col, cols.value0, cols.root]
        );

        write_pair!(
            &pos_mult,
            &program_denom,
            &neg_enabler,
            &merkle_0_denom,
            interaction_trace
        );

        // 3. merkle: -enabler * (index_base + 1, leaf_depth, value1, root)
        let merkle_1_denom = combine!(
            relations.merkle,
            [
                &index_base_plus_one,
                &leaf_depth_col,
                cols.value1,
                cols.root
            ]
        );

        // 4. merkle: -enabler * (index_base + 2, leaf_depth, value2, root)
        let merkle_2_denom = combine!(
            relations.merkle,
            [
                &index_base_plus_two,
                &leaf_depth_col,
                cols.value2,
                cols.root
            ]
        );

        write_pair!(
            &neg_enabler,
            &merkle_1_denom,
            &neg_enabler,
            &merkle_2_denom,
            interaction_trace
        );

        // 5. merkle: -enabler * (index_base + 3, leaf_depth, value3, root)
        let merkle_3_denom = combine!(
            relations.merkle,
            [
                &index_base_plus_three,
                &leaf_depth_col,
                cols.value3,
                cols.root
            ]
        );

        write_col!(&neg_enabler, &merkle_3_denom, interaction_trace);

        interaction_trace.finalize_last()
    }
}
