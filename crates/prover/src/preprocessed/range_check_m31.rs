//! Range check table for M31 limb endpoints.
//!
//! Two columns containing all possible pairs of:
//! - least significant limb (8 bits)
//! - most significant limb (7 bits, since M31 < 2^31)
//! for a total size of `2^15`.

use std::marker::PhantomData;

use stwo::core::ColumnVec;
use stwo::core::fields::m31::BaseField;
use stwo::core::poly::circle::CanonicCoset;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::{Col, Column};
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

        let mut lsl = Col::<SimdBackend, BaseField>::zeros(size);
        let mut msl = Col::<SimdBackend, BaseField>::zeros(size);

        for i in 0..size {
            let idx = i as u32;
            let lsl_val = idx & 0xff;
            let msl_val = (idx >> 8) & 0x7f;

            lsl.set(i, BaseField::from(lsl_val));
            msl.set(i, BaseField::from(msl_val));
        }

        vec![
            CircleEvaluation::new(domain, lsl),
            CircleEvaluation::new(domain, msl),
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
