//! A collection of preprocessed columns, whose values are publicly acknowledged, and independent of
//! the proof.
//!
//! Each column type defines its own trait (extending `Backend`) with backend-specific
//! implementations. This allows columns to be added independently in separate files.

use stwo::{
    core::fields::m31::BaseField,
    prover::{
        backend::Backend,
        poly::{BitReversedOrder, circle::CircleEvaluation},
    },
};
use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;

pub mod range_check;

/// Trait for generating preprocessed columns.
///
/// Each trait implements this with its own optimized column generation.
pub trait PreProcessedColumns<B: Backend> {
    fn gen_columns() -> Vec<CircleEvaluation<B, BaseField, BitReversedOrder>>;
}

/// A collection of preprocessed columns with their identifiers.
pub struct PreProcessedTrace<B: Backend> {
    pub columns: Vec<CircleEvaluation<B, BaseField, BitReversedOrder>>,
    pub ids: Vec<PreProcessedColumnId>,
}

impl<B: Backend> Default for PreProcessedTrace<B>
where
    range_check::RangeCheckColumns<'static, ()>: PreProcessedColumns<B>,
{
    fn default() -> Self {
        let mut columns = Vec::new();
        let mut ids = Vec::new();

        // Range check columns
        let range_check_cols = range_check::RangeCheckColumns::gen_columns();
        let range_check_ids = range_check::RangeCheckColumns::to_ids(None);

        for (id, col) in range_check_ids.into_iter().zip(range_check_cols) {
            columns.push(col);
            ids.push(id);
        }

        // Future columns would be added here with their own trait bounds

        Self { columns, ids }
    }
}
