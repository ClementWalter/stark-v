//! Range check table for values in [0, 2^20).
//!
//! Single column containing values [0, 1, 2, ..., 2^20 - 1].
//! Used for range checking clock differences and other bounded values.

use std::marker::PhantomData;

use simd::aligned_vec;
use stwo::core::ColumnVec;
use stwo::core::fields::m31::BaseField;
use stwo::core::poly::circle::CanonicCoset;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::column::BaseColumn;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;

use crate::preprocessed::PreprocessedTable;

/// Range check 20-bit table.
///
/// Generic over `N` (column count) - the actual N is determined by the
/// `preprocessed!` macro declaration.
pub struct Table<const N: usize>(PhantomData<[(); N]>);

impl<const N: usize> PreprocessedTable<N> for Table<N> {
    const LOG_SIZE: u32 = 20;

    /// Index is the first value (range check uses single value lookup).
    #[inline]
    fn index(values: [u32; N]) -> u32 {
        values[0]
    }

    fn gen_columns() -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(Self::LOG_SIZE).circle_domain();
        let size = 1 << Self::LOG_SIZE;

        // Generate values [0, 1, 2, ..., 2^20 - 1]
        let mut col = aligned_vec![0u32; size];
        for i in 0..size {
            col[i] = i as u32;
        }

        vec![CircleEvaluation::new(domain, BaseColumn::from(col))]
    }

    fn column_ids() -> Vec<PreProcessedColumnId> {
        vec![PreProcessedColumnId {
            id: "range_check_20_value".into(),
        }]
    }
}
