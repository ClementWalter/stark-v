//! Witness generation for jal component.

use num_traits::Zero;
use stwo::core::ColumnVec;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::QM31;
use stwo::core::poly::circle::CanonicCoset;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::column::BaseColumn;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;

use runner::trace::JalTable;

/// Generate trace columns from the jal table.
pub fn gen_trace(
    table: JalTable,
    _counters: &mut crate::relations::Counters,
) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
    if table.is_empty() {
        return vec![];
    }

    // Pad to power of 2 (minimum 2^4 = 16)
    let len = table.len() as u32;
    let log_size = if len.is_power_of_two() {
        len.trailing_zeros()
    } else {
        len.next_power_of_two().trailing_zeros()
    };
    let log_size = log_size.max(4); // Minimum size
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
