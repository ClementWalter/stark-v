//! A collection of preprocessed columns, whose values are publicly acknowledged, and independent of
//! the proof.
//!
//! This module provides a backend-agnostic trait for generating preprocessed columns,
//! with optimized implementations for each backend (CPU, SIMD, etc.).

use stwo::{
    core::{fields::m31::BaseField, poly::circle::CanonicCoset},
    prover::{
        backend::{Backend, Col},
        poly::{BitReversedOrder, circle::CircleEvaluation},
    },
};
use stwo_constraint_framework::preprocessed_columns::PreProcessedColumnId;

pub mod range_check;

/// Trait for generating preprocessed columns for a specific backend.
///
/// Each backend (CPU, SIMD, GPU) implements this trait with its own optimized
/// column generation strategy.
pub trait PreprocessedColumns: Backend {
    // Add required preprocessed columns here
    fn range_check() -> Vec<CircleEvaluation<Self, BaseField, BitReversedOrder>>;
}

/// A collection of preprocessed columns with their identifiers.
pub struct PreProcessedTrace<B: Backend> {
    pub columns: Vec<CircleEvaluation<B, BaseField, BitReversedOrder>>,
    pub ids: Vec<PreProcessedColumnId>,
}

impl<B: PreprocessedColumns> PreProcessedTrace<B> {
    /// Creates a new preprocessed trace using the backend's column generation.
    pub fn new() -> Self {
        let mut columns = Vec::new();
        let mut ids = Vec::new();

        // Generate preprocessed columns from backend implementation
        let range_check_cols = B::range_check();
        let range_check_ids = range_check::RangeCheckColumns::to_ids(None);

        for (id, col) in range_check_ids.into_iter().zip(range_check_cols) {
            columns.push(col);
            ids.push(id);
        }

        Self { columns, ids }
    }
}

impl<B: PreprocessedColumns> Default for PreProcessedTrace<B> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Creates a CircleEvaluation from a column of BaseField values.
pub fn column_to_circle_eval<B: Backend>(
    column: Col<B, BaseField>,
    log_size: u32,
) -> CircleEvaluation<B, BaseField, BitReversedOrder> {
    let domain = CanonicCoset::new(log_size).circle_domain();
    CircleEvaluation::new(domain, column)
}
