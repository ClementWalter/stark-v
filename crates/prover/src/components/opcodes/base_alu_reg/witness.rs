//! Witness generation for base_alu_reg component.

use num_traits::Zero;
use stwo::core::ColumnVec;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::QM31;
use stwo::core::poly::circle::CanonicCoset;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::column::BaseColumn;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;

use runner::trace::BaseAluRegTable;

/// Generate trace columns from the base_alu_reg table.
/// Always produces columns with minimum log_size of 4, even for empty tables.
pub fn gen_trace(
    table: BaseAluRegTable,
    _counters: &mut crate::relations::Counters,
) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
    // Pad to power of 2 (minimum 2^4 = 16)
    let len = table.len() as u32;
    let log_size = len.next_power_of_two().ilog2().max(4);
    let padded_len = 1 << log_size;

    let columns = table.into_columns();
    let domain = CanonicCoset::new(log_size).circle_domain();

    columns
        .into_iter()
        .map(|mut col| {
            // Pad with zeros
            col.resize(padded_len, 0);
            let base_col: BaseColumn = col.into();
            CircleEvaluation::new(domain, base_col)
        })
        .collect()
}

/// Generate interaction trace for LogUp.
pub fn gen_interaction_trace(
    _trace: &ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    _relations: &crate::relations::Relations,
) -> (
    ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    QM31,
) {
    // TODO: Implement LogUp interaction trace
    (vec![], QM31::zero())
}
