use simd::AlignedVec;
use stwo::{
    core::{fields::m31::BaseField, poly::circle::CanonicCoset},
    prover::{
        backend::{CpuBackend, simd::SimdBackend},
        poly::{BitReversedOrder, circle::CircleEvaluation},
    },
};

use crate::{preprocessed::PreProcessedColumns, trace_columns};

const LOG_TABLE_SIZE: u32 = 20;

trace_columns!(RangeCheckColumns, value,);

impl PreProcessedColumns<CpuBackend> for RangeCheckColumns<'static, ()> {
    fn gen_columns() -> Vec<CircleEvaluation<CpuBackend, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(LOG_TABLE_SIZE).circle_domain();

        let value_col: Vec<BaseField> = (0..1 << LOG_TABLE_SIZE)
            .map(BaseField::from_u32_unchecked)
            .collect();

        vec![CircleEvaluation::new(domain, value_col)]
    }
}

impl PreProcessedColumns<SimdBackend> for RangeCheckColumns<'static, ()> {
    fn gen_columns() -> Vec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(LOG_TABLE_SIZE).circle_domain();

        let mut value_col = AlignedVec::with_capacity(1 << LOG_TABLE_SIZE);
        for i in 0..(1u32 << LOG_TABLE_SIZE) {
            value_col.push(i);
        }

        vec![CircleEvaluation::new(domain, value_col.into())]
    }
}
