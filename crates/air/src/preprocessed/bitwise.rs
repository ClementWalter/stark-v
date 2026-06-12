//! Bitwise operation table.
//!
//! Four columns containing:
//! - limb_0 (8-bit input)
//! - limb_1 (8-bit input)
//! - result (limb_0 <op> limb_1)
//! - bitwise_id (0 = and, 1 = or, 2 = xor; 3 left unused to fill the power-of-two domain)
//!
//! The table covers all combinations of `bitwise_id ∈ [0, 3]` and `limb_0, limb_1 ∈ [0, 2^8)`.
//! Valid rows correspond to ids 0, 1, and 2; id 3 is padded with zero results to reach a `2^18`
//! domain size (3 * 2^16 meaningful entries).

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

/// Bitwise lookup table.
///
/// Enumerates `(limb_0, limb_1, result, bitwise_id)` tuples where result is derived from the
/// selected operation. Rows with `bitwise_id = 3` are included only as padding.
pub struct Table;

impl PreprocessedTable for Table {
    const LOG_SIZE: u32 = 18;

    #[inline]
    fn index(values: &[PackedM31]) -> [u32; 16] {
        let v0 = values[0].to_array();
        let v1 = values[1].to_array();
        let v3 = values[3].to_array();
        std::array::from_fn(|i| v0[i].0 + (v1[i].0 << 8) + (v3[i].0 << 16))
    }

    fn gen_columns() -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(Self::LOG_SIZE).circle_domain();
        let size = 1 << Self::LOG_SIZE;

        let mut limb_0 = aligned_vec![0u32; size];
        let mut limb_1 = aligned_vec![0u32; size];
        let mut result = aligned_vec![0u32; size];
        let mut bitwise_id = aligned_vec![0u32; size];

        for op_id in 0..4u32 {
            for lhs in 0..256u32 {
                for rhs in 0..256u32 {
                    let idx = (lhs | (rhs << 8) | (op_id << 16)) as usize;
                    let res = match op_id {
                        0 => lhs & rhs,
                        1 => lhs | rhs,
                        2 => lhs ^ rhs,
                        _ => 0,
                    };

                    limb_0[idx] = lhs;
                    limb_1[idx] = rhs;
                    result[idx] = res;
                    bitwise_id[idx] = op_id;
                }
            }
        }

        vec![
            CircleEvaluation::new(domain, BaseColumn::from(limb_0)),
            CircleEvaluation::new(domain, BaseColumn::from(limb_1)),
            CircleEvaluation::new(domain, BaseColumn::from(result)),
            CircleEvaluation::new(domain, BaseColumn::from(bitwise_id)),
        ]
    }

    fn column_ids() -> Vec<PreProcessedColumnId> {
        vec![
            PreProcessedColumnId {
                id: "bitwise_limb_0".into(),
            },
            PreProcessedColumnId {
                id: "bitwise_limb_1".into(),
            },
            PreProcessedColumnId {
                id: "bitwise_result".into(),
            },
            PreProcessedColumnId {
                id: "bitwise_id".into(),
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
        let col_result = columns[2].values.to_cpu();
        let col_bitwise_id = columns[3].values.to_cpu();

        for index in 0..col_limb_0.len() {
            let v0 = col_limb_0[index];
            let v1 = col_limb_1[index];
            let v2 = col_result[index];
            let v3 = col_bitwise_id[index];

            let packed_v0 = PackedM31::broadcast(v0);
            let packed_v1 = PackedM31::broadcast(v1);
            let packed_v2 = PackedM31::broadcast(v2);
            let packed_v3 = PackedM31::broadcast(v3);
            let values = [packed_v0, packed_v1, packed_v2, packed_v3];

            let computed_indices = Table::index(&values);

            assert_eq!(
                computed_indices[0], index as u32,
                "index mismatch at idx {index}"
            );
        }
    }
}
