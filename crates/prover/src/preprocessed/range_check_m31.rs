//! Range check table for safe u32 → M31 casting.
//!
//! # Purpose
//!
//! Validates that a u32 value can be safely cast to M31 by checking the
//! boundary condition on the least and most significant limbs.
//!
//! # M31 Field Bounds
//!
//! - M31 modulus: p = 2³¹ - 1 = 0x7FFFFFFF
//! - Valid range: [0, 2³¹ - 2] (since 2³¹ - 1 ≡ 0 mod p)
//!
//! # Limb Decomposition
//!
//! A u32 is decomposed into 4 limbs (8-8-8-7 bits, little-endian):
//! ```text
//! v = lsl + (limb1 << 8) + (limb2 << 16) + (msl << 24)
//!     [0,255]  [0,255]      [0,255]        [0,127]
//! ```
//!
//! # Boundary Analysis
//!
//! ```text
//! max(M31)     = 2³¹ - 2 = 0x7FFFFFFE → limbs: (254, 255, 255, 127) ✓ valid
//! invalid      = 2³¹ - 1 = 0x7FFFFFFF → limbs: (255, 255, 255, 127) ✗ equals modulus
//! ```
//!
//! When msl = 127 and all middle limbs are 255, lsl must be ≤ 254.
//! This table uses a conservative check: reject (lsl=255, msl=127) regardless
//! of middle limbs, which is safe but may reject some technically valid values.
//!
//! # Table Contents
//!
//! Contains 2¹⁵ entries covering all valid (lsl, msl) pairs:
//! - All pairs where msl ∈ [0, 126]: 127 × 256 = 32,512 entries
//! - Pairs where msl = 127 and lsl ∈ [0, 254]: 255 entries
//! - Entry at index 32767 is a duplicate (0, 0) to exclude (255, 127)

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

/// Range check table for safe u32 → M31 casting.
///
/// Enumerates all valid `(lsl, msl)` pairs excluding `(255, 127)` which
/// would allow the invalid value 2³¹ - 1 (the M31 modulus).
pub struct Table;

impl PreprocessedTable for Table {
    const LOG_SIZE: u32 = 15;

    #[inline]
    fn index(values: &[PackedM31]) -> [u32; 16] {
        let v0 = values[0].to_array();
        let v1 = values[1].to_array();
        std::array::from_fn(|i| v0[i].0 + (v1[i].0 << 8))
    }

    fn gen_columns() -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(Self::LOG_SIZE).circle_domain();
        let size = 1 << Self::LOG_SIZE;

        let mut lsl = aligned_vec![0u32; size];
        let mut msl = aligned_vec![0u32; size];

        for i in 0..size {
            let lsl_val = (i & 0xff) as u32;
            let msl_val = ((i >> 8) & 0x7f) as u32;

            // Exclude (lsl=255, msl=127) which corresponds to 2^31-1 (invalid M31).
            // Use a duplicate entry (0, 0) at index 32767 instead.
            if lsl_val == 255 && msl_val == 127 {
                lsl[i] = 0;
                msl[i] = 0;
            } else {
                lsl[i] = lsl_val;
                msl[i] = msl_val;
            }
        }

        vec![
            CircleEvaluation::new(domain, BaseColumn::from(lsl)),
            CircleEvaluation::new(domain, BaseColumn::from(msl)),
        ]
    }

    fn column_ids() -> Vec<PreProcessedColumnId> {
        vec![
            PreProcessedColumnId {
                id: "range_check_m31_lsl".into(),
            },
            PreProcessedColumnId {
                id: "range_check_m31_msl".into(),
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
        let col_lsl = columns[0].values.to_cpu();
        let col_msl = columns[1].values.to_cpu();

        // Last index is a duplicate (0, 0) to exclude (255, 127).
        for index in 0..(col_lsl.len() - 1) {
            let v0 = col_lsl[index];
            let v1 = col_msl[index];

            let packed_v0 = PackedM31::broadcast(v0);
            let packed_v1 = PackedM31::broadcast(v1);
            let values = [packed_v0, packed_v1];

            let computed_indices = Table::index(&values);

            assert_eq!(
                computed_indices[0], index as u32,
                "index mismatch at idx {index}"
            );
        }
    }
}
