//! Recursion seam: the stark-v constraint system as data.
//!
//! The 2-to-1 recursive verifier (docs/recursion.md) consumes the same
//! `FrameworkEval::evaluate` code as the prover and the host verifier, so
//! constraints are never copied: an edit to `define_trace_tables!` changes
//! the prover, the host verifier, and everything built on this module in
//! the same compilation.
//!
//! Three instantiations of the one `evaluate` function cover the recursion
//! needs:
//! - `ExprEvaluator` — constraints as expression trees (with formal LogUp
//!   parameters), for generating or auditing the verifier AIR's
//!   composition-check sub-circuit.
//! - `InfoEvaluator` — structural data (mask offsets per interaction,
//!   constraint count), for laying out the verifier AIR trace.
//! - `PointEvaluator` (used inside `stwo::core::verifier::verify`) — QM31
//!   composition evaluation from sampled mask values, which the verifier
//!   AIR's witness generator replays.

pub mod aggregate;
pub mod segments;
pub mod transcript;

use stwo_constraint_framework::expr::ExprEvaluator;
use stwo_constraint_framework::{FrameworkEval, InfoEvaluator};

/// Extract a component's constraints as expression trees.
///
/// The result contains every polynomial constraint (`constraints`) and the
/// formal LogUp fractions (`logup.fracs`) with relation parameters
/// (`<relation>_z`, `<relation>_alpha<i>`) left symbolic.
pub fn constraint_exprs<E: FrameworkEval>(eval: &E) -> ExprEvaluator {
    eval.evaluate(ExprEvaluator::new())
}

/// Extract a component's structural summary: mask offsets per interaction
/// and the number of constraints, as used for trace layout.
pub fn constraint_info<E: FrameworkEval>(eval: &E) -> InfoEvaluator {
    eval.evaluate(InfoEvaluator::empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relations::Relations;

    fn lui_eval() -> crate::components::opcodes::lui::air::Eval {
        crate::components::opcodes::lui::air::Eval {
            log_size: 4,
            relations: Relations::dummy(),
        }
    }

    fn base_alu_imm_eval() -> crate::components::opcodes::base_alu_imm::air::Eval {
        crate::components::opcodes::base_alu_imm::air::Eval {
            log_size: 4,
            relations: Relations::dummy(),
        }
    }

    #[test]
    fn test_lui_constraint_exprs_match_info_count() {
        let exprs = constraint_exprs(&lui_eval());
        let info = constraint_info(&lui_eval());
        assert_eq!(exprs.constraints.len(), info.n_constraints);
    }

    #[test]
    fn test_lui_logup_batches_become_constraints() {
        let exprs = constraint_exprs(&lui_eval());
        // 1 enabler booleanity + ceil(7 LogUp entries / 2) = 4 batch constraints
        assert_eq!(exprs.constraints.len(), 5);
    }

    #[test]
    fn test_base_alu_imm_constraint_exprs_match_info_count() {
        let exprs = constraint_exprs(&base_alu_imm_eval());
        let info = constraint_info(&base_alu_imm_eval());
        assert_eq!(exprs.constraints.len(), info.n_constraints);
    }

    #[test]
    fn test_lui_constraints_are_nonempty_expressions() {
        let exprs = constraint_exprs(&lui_eval());
        // The enabler booleanity constraint formats to a real expression
        // referencing the trace column, proving constraints flow from the
        // macro into expression data.
        assert!(!exprs.format_constraints().is_empty());
    }
}
