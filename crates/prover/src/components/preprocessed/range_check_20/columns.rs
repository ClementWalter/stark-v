//! Column definitions for range check 20 multiplicity component.

use stwo_constraint_framework::EvalAtRow;

/// Number of trace columns for this component.
pub const N_COLUMNS: usize = 1;

/// Column offsets.
pub const MULTIPLICITY: usize = 0;

/// Columns for range check multiplicity.
pub struct Columns<E: EvalAtRow> {
    pub multiplicity: E::F,
}

impl<E: EvalAtRow> Columns<E> {
    pub fn from_eval(eval: &mut E) -> Self {
        Self {
            multiplicity: eval.next_trace_mask(),
        }
    }
}
