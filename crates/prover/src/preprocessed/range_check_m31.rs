//! Range check table for M31 limb endpoints.
//!
//! Two columns containing all possible pairs of:
//! - least significant limb (8 bits)
//! - most significant limb (7 bits, since M31 < 2^31)
//! bin(2**31 - 1) = 01111111 11111111 11111111 11111111
//! max(BaseField) = 2**31 - 2 = 01111111 11111111 11111111 11111110
//! for a total size of `2^15`.

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

/// Range check table for M31 limb endpoints.
///
/// Enumerates all `(lsl, msl)` pairs where `lsl ∈ [0, 2^8)` and `msl ∈ [0, 2^7)`.
pub struct Table<const N: usize>(PhantomData<[(); N]>);

impl<const N: usize> PreprocessedTable<N> for Table<N> {
    const LOG_SIZE: u32 = 15;

    /// Index packs `(lsl, msl)` into 15 bits.
    #[inline]
    fn index(values: [u32; N]) -> u32 {
        values[0] + (values[1] << 8)
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
