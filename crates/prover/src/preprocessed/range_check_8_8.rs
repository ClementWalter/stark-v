//! Range check table for (8, 8)-bit tuples.
//!
//! Two columns covering the cartesian product of:
//! - `limb_0 ∈ [0, 2^8)`
//! - `limb_1 ∈ [0, 2^8)`
//! for a total size of `2^16`.

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

/// Range check (8, 8) table.
///
/// The table enumerates all tuples in `[0, 2^8) × [0, 2^8)`.
pub struct Table<const N: usize>(PhantomData<[(); N]>);

impl<const N: usize> PreprocessedTable<N> for Table<N> {
    const LOG_SIZE: u32 = 16;

    /// Index packs the two limbs into 16 bits.
    #[inline]
    fn index(values: [u32; N]) -> u32 {
        values[0] + (values[1] << 8)
    }

    fn gen_columns() -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(Self::LOG_SIZE).circle_domain();
        let size = 1 << Self::LOG_SIZE;

        let mut limb_0 = Col::<SimdBackend, BaseField>::zeros(size);
        let mut limb_1 = Col::<SimdBackend, BaseField>::zeros(size);

        for i in 0..size {
            let idx = i as u32;
            let val_0 = idx & 0xff;
            let val_1 = (idx >> 8) & 0xff;

            limb_0.set(i, BaseField::from(val_0));
            limb_1.set(i, BaseField::from(val_1));
        }

        vec![
            CircleEvaluation::new(domain, limb_0),
            CircleEvaluation::new(domain, limb_1),
        ]
    }

    fn column_ids() -> Vec<PreProcessedColumnId> {
        vec![
            PreProcessedColumnId {
                id: "range_check_8_8_limb_0".into(),
            },
            PreProcessedColumnId {
                id: "range_check_8_8_limb_1".into(),
            },
        ]
    }
}
