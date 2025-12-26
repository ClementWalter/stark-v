//! Witness generation for BGEU opcode.

use num_traits::Zero;
use stwo::core::ColumnVec;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::QM31;
use stwo::core::poly::circle::CanonicCoset;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::column::BaseColumn;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;

use crate::relations::Relations;
use runner::trace::BgeuTable;

/// Generate trace from runner's BGEU table.
///
/// - Pads to next power of 2
/// - Each component has its own log_size based on trace length
pub fn gen_trace(
    table: BgeuTable,
    _counters: &mut crate::relations::Counters,
) -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
    let len = table.len();
    if len == 0 {
        return vec![];
    }

    // Pad to next power of 2
    let log_size = if len.is_power_of_two() {
        len.trailing_zeros()
    } else {
        len.next_power_of_two().trailing_zeros()
    };
    let padded_len = 1 << log_size;

    let domain = CanonicCoset::new(log_size).circle_domain();

    // Convert and pad all columns (enabler is already included as first column)
    let columns: Vec<_> = table
        .into_columns()
        .into_iter()
        .map(|mut col| {
            col.resize(padded_len, 0); // Pad with zeros
            let base_col: BaseColumn = col.into();
            CircleEvaluation::new(domain, base_col)
        })
        .collect();

    columns
}

/// Generate interaction trace (dummy for scaffolding).
pub fn gen_interaction_trace(
    _trace: &ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    _relations: &Relations,
) -> (
    ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    QM31,
) {
    (vec![], QM31::zero())
}
