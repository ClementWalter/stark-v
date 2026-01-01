//! Memory commitment component (initial/final values).

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

            eval.add_to_relation(RelationEntry::new(
                &self.relations.memory,
                E::EF::from(multiplicity.clone()),
                &[
                    addr.clone(),
                    clk,
                    value0.clone(),
                    value1.clone(),
                    value2.clone(),
                    value3.clone(),
                ],
            ));

            let index_base = addr - base;
            eval.add_to_relation(RelationEntry::new(
                &self.relations.merkle,
                -E::EF::from(enabler.clone()),
                &[index_base.clone(), leaf_depth.clone(), value0, root.clone()],
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.merkle,
                -E::EF::from(enabler.clone()),
                &[
                    index_base.clone() + one.clone(),
                    leaf_depth.clone(),
                    value1,
                    root.clone(),
                ],
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.merkle,
                -E::EF::from(enabler.clone()),
                &[
                    index_base.clone() + two.clone(),
                    leaf_depth.clone(),
                    value2,
                    root.clone(),
                ],
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.merkle,
                -E::EF::from(enabler),
                &[index_base + three, leaf_depth, value3, root],
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

        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let mut col = interaction_trace.new_col();
        for row in 0..packed_len {
            let enabler_row = enabler[row];
            let addr_row = addr[row];
            let clk_row = clk[row];
            let v0 = value0[row];
            let v1 = value1[row];
            let v2 = value2[row];
            let v3 = value3[row];
            let mult_row = multiplicity[row];
            let root_row = root[row];

            let index_base = addr_row - base;

            let num0: PackedQM31 = PackedQM31::from(mult_row);
            let denom0: PackedQM31 = relations
                .memory
                .combine(&[addr_row, clk_row, v0, v1, v2, v3]);

            let num1: PackedQM31 = -PackedQM31::from(enabler_row);
            let denom1: PackedQM31 = relations
                .merkle
                .combine(&[index_base, leaf_depth, v0, root_row]);

            let numerator = num0 * denom1 + num1 * denom0;
            let denom = denom0 * denom1;
            col.write_frac(row, numerator, denom);
        }
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        for row in 0..packed_len {
            let enabler_row = enabler[row];
            let addr_row = addr[row];
            let v1 = value1[row];
            let v2 = value2[row];
            let root_row = root[row];

            let index_base = addr_row - base;

            let num0: PackedQM31 = -PackedQM31::from(enabler_row);
            let denom0: PackedQM31 =
                relations
                    .merkle
                    .combine(&[index_base + one, leaf_depth, v1, root_row]);

            let num1: PackedQM31 = -PackedQM31::from(enabler_row);
            let denom1: PackedQM31 =
                relations
                    .merkle
                    .combine(&[index_base + two, leaf_depth, v2, root_row]);

            let numerator = num0 * denom1 + num1 * denom0;
            let denom = denom0 * denom1;
            col.write_frac(row, numerator, denom);
        }
        col.finalize_col();

        let mut col = interaction_trace.new_col();
        for row in 0..packed_len {
            let enabler_row = enabler[row];
            let addr_row = addr[row];
            let v3 = value3[row];
            let root_row = root[row];

            let index_base = addr_row - base;

            let numerator: PackedQM31 = -PackedQM31::from(enabler_row);
            let denom: PackedQM31 =
                relations
                    .merkle
                    .combine(&[index_base + three, leaf_depth, v3, root_row]);
            col.write_frac(row, numerator, denom);
        }
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        (trace, claimed_sum)
    }
}
