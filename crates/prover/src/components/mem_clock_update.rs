//! Clock update component derived from memory gap-filling traces.

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
use runner::trace::DEFAULT_MAX_CLOCK_DIFF;

pub mod columns {
    pub use runner::trace::prover_columns::MemClockUpdateColumns;
}

pub mod air {
    use super::columns::MemClockUpdateColumns;
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
            let cols = MemClockUpdateColumns::from_eval(&mut eval);
            let enabler = cols.enabler.clone();
            let addr = cols.addr.clone();
            let clock_prev = cols.clock_prev.clone();
            let value_0 = cols.value_0.clone();
            let value_1 = cols.value_1.clone();
            let value_2 = cols.value_2.clone();
            let value_3 = cols.value_3.clone();

            let one = E::F::one();
            let diff = E::F::from(M31::from(DEFAULT_MAX_CLOCK_DIFF));

            eval.add_constraint(enabler.clone() * (one - enabler.clone()));

            let rw_as = E::F::one();
            add_to_relation!(
                eval,
                self.relations.memory_access,
                -enabler.clone(),
                rw_as.clone(),
                addr.clone(),
                clock_prev.clone(),
                value_0.clone(),
                value_1.clone(),
                value_2.clone(),
                value_3.clone()
            );
            add_to_relation!(
                eval,
                self.relations.memory_access,
                enabler,
                rw_as,
                addr,
                clock_prev + diff,
                value_0,
                value_1,
                value_2,
                value_3
            );
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

        // Column order matches MemClockUpdateColumns.
        let enabler = &trace[0].data;
        let addr = &trace[1].data;
        let clock_prev = &trace[2].data;
        let value_0 = &trace[3].data;
        let value_1 = &trace[4].data;
        let value_2 = &trace[5].data;
        let value_3 = &trace[6].data;

        let diff = PackedM31::broadcast(M31::from(DEFAULT_MAX_CLOCK_DIFF));

        let simd_size = enabler.len();
        let log_size = trace[0].domain.log_size();
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let rw_as = PackedM31::broadcast(M31::one());
        let rw_as_col = vec![rw_as; simd_size];
        let clock_prev_plus_diff: Vec<PackedM31> =
            (0..simd_size).map(|i| clock_prev[i] + diff).collect();

        let neg_enabler: Vec<PackedQM31> = (0..simd_size)
            .map(|i| -PackedQM31::from(enabler[i]))
            .collect();
        let pos_enabler: Vec<PackedQM31> = (0..simd_size)
            .map(|i| PackedQM31::from(enabler[i]))
            .collect();

        let prev_denom = combine!(
            relations.memory_access,
            [
                &rw_as_col, addr, clock_prev, value_0, value_1, value_2, value_3
            ]
        );
        let next_denom = combine!(
            relations.memory_access,
            [
                &rw_as_col,
                addr,
                &clock_prev_plus_diff,
                value_0,
                value_1,
                value_2,
                value_3
            ]
        );

        write_pair!(
            &neg_enabler,
            &prev_denom,
            &pos_enabler,
            &next_denom,
            interaction_trace
        );

        interaction_trace.finalize_last()
    }
}
