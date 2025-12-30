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

/// Bitwise lookup table.
///
/// Enumerates `(limb_0, limb_1, result, bitwise_id)` tuples where result is derived from the
/// selected operation. Rows with `bitwise_id = 3` are included only as padding.
pub struct Table<const N: usize>(PhantomData<[(); N]>);

impl<const N: usize> PreprocessedTable<N> for Table<N> {
    const LOG_SIZE: u32 = 18;

    /// Index packs `limb_0`, `limb_1`, and `bitwise_id` into 18 bits.
    #[inline]
    fn index(values: [u32; N]) -> u32 {
        values[0] + (values[1] << 8) + (values[3] << 16)
    }

    fn gen_columns() -> ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>> {
        let domain = CanonicCoset::new(Self::LOG_SIZE).circle_domain();
        let size = 1 << Self::LOG_SIZE;

        let mut limb_0 = Col::<SimdBackend, BaseField>::zeros(size);
        let mut limb_1 = Col::<SimdBackend, BaseField>::zeros(size);
        let mut result = Col::<SimdBackend, BaseField>::zeros(size);
        let mut bitwise_id = Col::<SimdBackend, BaseField>::zeros(size);

        for op_id in 0..4u32 {
            for lhs in 0..256u32 {
                for rhs in 0..256u32 {
                    let idx = (lhs) | (rhs << 8) | (op_id << 16);
                    let res = match op_id {
                        0 => lhs & rhs,
                        1 => lhs | rhs,
                        2 => lhs ^ rhs,
                        _ => 0,
                    };

                    limb_0.set(idx as usize, BaseField::from(lhs));
                    limb_1.set(idx as usize, BaseField::from(rhs));
                    result.set(idx as usize, BaseField::from(res));
                    bitwise_id.set(idx as usize, BaseField::from(op_id));
                }
            }
        }

        vec![
            CircleEvaluation::new(domain, limb_0),
            CircleEvaluation::new(domain, limb_1),
            CircleEvaluation::new(domain, result),
            CircleEvaluation::new(domain, bitwise_id),
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
