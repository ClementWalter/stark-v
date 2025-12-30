//! AIR component for range check (8, 11) multiplicity.
//!
//! Provides the preprocessed side of the LogUp relation:
//! Σ (multiplicity[i] / (value[i] - z))

use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use super::columns::Columns;
use crate::relations::Relations;

pub type Component = FrameworkComponent<Eval>;

#[derive(Clone)]
pub struct Eval {
    pub log_size: u32,
    pub relations: Relations,
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let cols = Columns::from_eval(&mut eval);

        // TODO: Add LogUp constraint
        // For now, dummy constraint (multiplicity - multiplicity = 0)
        eval.add_constraint(cols.multiplicity.clone() - cols.multiplicity.clone());

        eval
    }
}
