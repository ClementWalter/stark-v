//! Clock update component derived from register gap-filling traces.

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
use runner::trace::DEFAULT_MAX_CLOCK_DIFF;

pub mod columns {
    use stwo_constraint_framework::EvalAtRow;

    #[derive(Debug, Clone)]
    pub struct RegClockUpdateColumns<T> {
        pub enabler: T,
        pub addr: T,
        pub clk_prev: T,
        pub value0: T,
        pub value1: T,
        pub value2: T,
        pub value3: T,
    }

    impl<T> RegClockUpdateColumns<T> {
        pub const SIZE: usize = 7;

        pub fn from_eval<E: EvalAtRow<F = T>>(eval: &mut E) -> Self {
            Self {
                enabler: eval.next_trace_mask(),
                addr: eval.next_trace_mask(),
                clk_prev: eval.next_trace_mask(),
                value0: eval.next_trace_mask(),
                value1: eval.next_trace_mask(),
                value2: eval.next_trace_mask(),
                value3: eval.next_trace_mask(),
            }
        }
    }
}

pub mod air {
    use super::columns::RegClockUpdateColumns;
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
            let cols = RegClockUpdateColumns::from_eval(&mut eval);
            let enabler = cols.enabler.clone();
            let addr = cols.addr.clone();
            let clk_prev = cols.clk_prev.clone();
            let value0 = cols.value0.clone();
            let value1 = cols.value1.clone();
            let value2 = cols.value2.clone();
            let value3 = cols.value3.clone();

            let one = E::F::one();
            let diff = E::F::from(M31::from(DEFAULT_MAX_CLOCK_DIFF));

            eval.add_constraint(enabler.clone() * (one - enabler.clone()));

            eval.add_to_relation(RelationEntry::new(
                &self.relations.memory,
                -E::EF::from(enabler.clone()),
                &[
                    addr.clone(),
                    clk_prev.clone(),
                    value0.clone(),
                    value1.clone(),
                    value2.clone(),
                    value3.clone(),
                ],
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.memory,
                E::EF::from(enabler),
                &[addr, clk_prev + diff, value0, value1, value2, value3],
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

        // Column order matches RegClockUpdateColumns.
        let enabler = &trace[0].data;
        let addr = &trace[1].data;
        let clk_prev = &trace[2].data;
        let value0 = &trace[3].data;
        let value1 = &trace[4].data;
        let value2 = &trace[5].data;
        let value3 = &trace[6].data;

        let diff = PackedM31::broadcast(M31::from(DEFAULT_MAX_CLOCK_DIFF));

        let mut interaction_trace = LogupTraceGenerator::new(log_size);
        let mut col = interaction_trace.new_col();
        for row in 0..packed_len {
            let enabler_row = enabler[row];
            let addr_row = addr[row];
            let clk_prev_row = clk_prev[row];
            let v0 = value0[row];
            let v1 = value1[row];
            let v2 = value2[row];
            let v3 = value3[row];

            let num0: PackedQM31 = -PackedQM31::from(enabler_row);
            let denom0: PackedQM31 =
                relations
                    .memory
                    .combine(&[addr_row, clk_prev_row, v0, v1, v2, v3]);

            let num1: PackedQM31 = PackedQM31::from(enabler_row);
            let denom1: PackedQM31 =
                relations
                    .memory
                    .combine(&[addr_row, clk_prev_row + diff, v0, v1, v2, v3]);

            let numerator = num0 * denom1 + num1 * denom0;
            let denom = denom0 * denom1;
            col.write_frac(row, numerator, denom);
        }
        col.finalize_col();

        let (trace, claimed_sum) = interaction_trace.finalize_last();
        (trace, claimed_sum)
    }
}
