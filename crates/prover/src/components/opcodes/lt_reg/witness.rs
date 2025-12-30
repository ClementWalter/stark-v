//! Witness generation for lt_reg component.

use num_traits::Zero;
use stwo::core::ColumnVec;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::QM31;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;

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
