//! Range check table for (8, 8, 4)-bit tuples.
//!
//! Three columns covering the cartesian product of:
//! - `limb_0 ∈ [0, 2^8)`
//! - `limb_1 ∈ [0, 2^8)`
//! - `limb_2 ∈ [0, 2^4)`
//!
//! Total size: `2^20`.

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

/// Range check (8, 8, 4) table.
///
/// The table enumerates all tuples in `[0, 2^8) × [0, 2^8) × [0, 2^4)`.
pub struct Table;

impl PreprocessedTable for Table {
    const LOG_SIZE: u32 = 20;

    #[inline]
    fn index(values: &[PackedM31]) -> [u32; 16] {
        let v0 = values[0].to_array();
        let v1 = values[1].to_array();
        let v2 = values[2].to_array();
        std::array::from_fn(|i| v0[i].0 + (v1[i].0 << 8) + (v2[i].0 << 16))
    }

    fn gen_columns() -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(Self::LOG_SIZE).circle_domain();
        let size = 1 << Self::LOG_SIZE;

        let mut limb_0 = aligned_vec![0u32; size];
        let mut limb_1 = aligned_vec![0u32; size];
        let mut limb_2 = aligned_vec![0u32; size];

        for i in 0..size {
            limb_0[i] = (i & 0xff) as u32;
            limb_1[i] = ((i >> 8) & 0xff) as u32;
            limb_2[i] = ((i >> 16) & 0x0f) as u32;
        }

        vec![
            CircleEvaluation::new(domain, BaseColumn::from(limb_0)),
            CircleEvaluation::new(domain, BaseColumn::from(limb_1)),
            CircleEvaluation::new(domain, BaseColumn::from(limb_2)),
        ]
    }

    fn column_ids() -> Vec<PreProcessedColumnId> {
        vec![
            PreProcessedColumnId {
                id: "range_check_8_8_4_limb_0".into(),
            },
            PreProcessedColumnId {
                id: "range_check_8_8_4_limb_1".into(),
            },
            PreProcessedColumnId {
                id: "range_check_8_8_4_limb_2".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stwo::prover::backend::Column;

    /// Test that gen_columns[index(values)] == values for all valid indices.
    #[test]
    fn test_index_roundtrip() {
        let columns = Table::gen_columns();
        let col_limb_0 = columns[0].values.to_cpu();
        let col_limb_1 = columns[1].values.to_cpu();
        let col_limb_2 = columns[2].values.to_cpu();

        for index in 0..col_limb_0.len() {
            let v0 = col_limb_0[index];
            let v1 = col_limb_1[index];
            let v2 = col_limb_2[index];

            let packed_v0 = PackedM31::broadcast(v0);
            let packed_v1 = PackedM31::broadcast(v1);
            let packed_v2 = PackedM31::broadcast(v2);
            let values = [packed_v0, packed_v1, packed_v2];

            let computed_indices = Table::index(&values);

            assert_eq!(
                computed_indices[0], index as u32,
                "index mismatch at idx {index}"
            );
        }
    }
}
