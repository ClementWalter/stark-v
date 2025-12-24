use simd::{aligned_vec_with_capacity, into_base_column};
use stwo::{
    core::{fields::m31::BaseField, poly::circle::CanonicCoset},
    prover::{
        backend::{CpuBackend, simd::SimdBackend},
        poly::{BitReversedOrder, circle::CircleEvaluation},
    },
};

const LOG_TABLE_SIZE: u32 = 20;

use crate::{preprocessed::PreprocessedColumns, trace_columns};

trace_columns!(RangeCheckColumns, value,);

impl PreprocessedColumns for CpuBackend {
    fn range_check() -> Vec<CircleEvaluation<Self, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(LOG_TABLE_SIZE).circle_domain();

        let value_col: Vec<BaseField> = (0..1 << LOG_TABLE_SIZE)
            .map(|i| BaseField::from_u32_unchecked(i))
            .collect();

        vec![CircleEvaluation::new(domain, value_col)]
    }
}

impl PreprocessedColumns for SimdBackend {
    fn range_check() -> Vec<CircleEvaluation<Self, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(LOG_TABLE_SIZE).circle_domain();

        let mut value_col = aligned_vec_with_capacity(1 << LOG_TABLE_SIZE);
        for i in 0..(1u32 << LOG_TABLE_SIZE) {
            value_col.push(i);
        }

        vec![CircleEvaluation::new(domain, into_base_column(value_col))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ids() {
        assert_eq!(RangeCheckColumns::SIZE, 1);
    }
}
