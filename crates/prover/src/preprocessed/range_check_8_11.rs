//! Range check table for (8, 11)-bit tuples.
//!
//! Two columns covering the cartesian product of:
//! - `limb_0 ∈ [0, 2^8)`
//! - `limb_1 ∈ [0, 2^11)`
//!
//! Total size: `2^19`.

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

/// Range check (8, 11) table.
///
/// The table enumerates all tuples in `[0, 2^8) × [0, 2^11)`.
pub struct Table;

impl PreprocessedTable for Table {
    const LOG_SIZE: u32 = 19;

    #[inline]
    fn index(values: &[u32]) -> u32 {
        values[0] + (values[1] << 8)
    }

    fn gen_columns() -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(Self::LOG_SIZE).circle_domain();
        let size = 1 << Self::LOG_SIZE;

        let mut limb_0 = aligned_vec![0u32; size];
        let mut limb_1 = aligned_vec![0u32; size];

        for i in 0..size {
            limb_0[i] = (i & 0xff) as u32;
            limb_1[i] = ((i >> 8) & 0x7ff) as u32;
        }

        vec![
            CircleEvaluation::new(domain, BaseColumn::from(limb_0)),
            CircleEvaluation::new(domain, BaseColumn::from(limb_1)),
        ]
    }

    fn column_ids() -> Vec<PreProcessedColumnId> {
        vec![
            PreProcessedColumnId {
                id: "range_check_8_11_limb_0".into(),
            },
            PreProcessedColumnId {
                id: "range_check_8_11_limb_1".into(),
            },
        ]
    }
}
