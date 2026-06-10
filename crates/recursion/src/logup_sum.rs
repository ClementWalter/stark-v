//! LogUp sum-of-inverses component: witness generation and AIR evaluation.
//!
//! Each enabled row contributes `1 / term` to the component's claimed sum
//! through a LogUp fraction in the interaction trace. This is the in-AIR form
//! of the verifier's LogUp-sum check (`PublicData::logup_sum` and the total
//! claimed-sum-is-zero assertion), and the first interaction-trace user of
//! the recursion AIR — the same machinery later binds the components to each
//! other via relations.

use num_traits::Zero;
use stwo::core::ColumnVec;
use stwo::core::Fraction;
use stwo::core::fields::FieldExpOps;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::{QM31, SecureField};
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::cm31::PackedCM31;
use stwo::prover::backend::simd::qm31::PackedQM31;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, LogupTraceGenerator,
};

use crate::LogupSumTable;
use crate::prover_columns::LogupSumColumns;

pub type Component = FrameworkComponent<Eval>;

#[derive(Clone)]
pub struct Eval {
    pub log_size: u32,
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let cols = LogupSumColumns::from_eval(&mut eval);
        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }
        let term = E::combine_ef([
            cols.term_0.clone(),
            cols.term_1.clone(),
            cols.term_2.clone(),
            cols.term_3.clone(),
        ]);
        // Padding rows have term = 0 and enabler = 0; shifting the
        // denominator by (1 - enabler) makes them contribute 0/1 instead of
        // the undefined 0/0, without affecting enabled rows.
        let one = <E::F as num_traits::One>::one();
        let denominator = term + E::EF::from(one - cols.enabler.clone());
        eval.write_logup_frac(Fraction::new(
            E::EF::from(cols.enabler.clone()),
            denominator,
        ));
        eval.finalize_logup();
        eval
    }
}

/// Record a term whose inverse joins the claimed sum, and return `1 / term`.
///
/// Panics if `term` is zero.
pub fn push_term(table: &mut LogupSumTable, term: QM31) -> QM31 {
    let limbs = term.to_m31_array();
    table.push(limbs[0].0, limbs[1].0, limbs[2].0, limbs[3].0);
    term.inverse()
}

/// Generate the interaction trace and the claimed sum `Σ enabler / term`.
pub fn gen_interaction_trace(
    trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
) -> (
    ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    SecureField,
) {
    let cols = LogupSumColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.enabler.len();
    let log_size = trace[0].domain.log_size();

    let mut logup_gen = LogupTraceGenerator::new(log_size);
    let mut col_gen = logup_gen.new_col();
    for vec_row in 0..simd_size {
        let enabler = cols.enabler[vec_row];
        let numerator = PackedQM31::from(enabler);
        let term = PackedQM31([
            PackedCM31([cols.term_0[vec_row], cols.term_1[vec_row]]),
            PackedCM31([cols.term_2[vec_row], cols.term_3[vec_row]]),
        ]);
        // Same padding shift as the AIR: denominator = term + (1 - enabler).
        let one = stwo::prover::backend::simd::m31::PackedM31::broadcast(BaseField::from(1));
        let denominator = term + PackedQM31::from(one - enabler);
        col_gen.write_frac(vec_row, numerator, denominator);
    }
    col_gen.finalize_col();
    logup_gen.finalize_last()
}

/// Host-side oracle: the claimed sum is the sum of inverses of the terms.
pub fn expected_sum(terms: &[QM31]) -> SecureField {
    if terms.is_empty() {
        return SecureField::zero();
    }
    let inverses = SecureField::batch_inverse(terms);
    inverses.iter().copied().sum()
}
