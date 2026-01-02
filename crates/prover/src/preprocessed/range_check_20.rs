//! Range check table for values in [0, 2^20).
//!
//! Single column containing values [0, 1, 2, ..., 2^20 - 1].
//! Used for range checking clock differences and other bounded values.

use simd::aligned_vec;
use stwo::core::ColumnVec;
use stwo::core::fields::m31::BaseField;
use stwo::core::poly::circle::CanonicCoset;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::column::BaseColumn;
use stwo::prover::backend::simd::m31::PackedM31;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;

use crate::preprocessed::PreprocessedTable;

/// Range check 20-bit table.
pub struct Table;

impl PreprocessedTable for Table {
    const LOG_SIZE: u32 = 20;

    #[inline]
    fn index(values: &[PackedM31]) -> [u32; 16] {
        values[0].to_array().map(|v| v.0)
    }

    fn gen_columns() -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(Self::LOG_SIZE).circle_domain();
        let size = 1 << Self::LOG_SIZE;

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

#[cfg(test)]
mod tests {
    use super::*;
    use stwo::prover::backend::Column;

    /// Test that gen_columns[index(values)] == values for all valid indices.
    #[allow(clippy::needless_range_loop)]
    #[test]
    fn test_index_roundtrip() {
        let columns = Table::gen_columns();
        let col_value = columns[0].values.to_cpu();

        for index in 0..col_value.len() {
            let v = col_value[index];

            let packed_v = PackedM31::broadcast(v);
            let values = [packed_v];

            let computed_indices = Table::index(&values);

            assert_eq!(
                computed_indices[0], index as u32,
                "index mismatch at idx {index}"
            );
        }
    }
}
