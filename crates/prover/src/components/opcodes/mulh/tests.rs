//! Tests for mulh component.

use super::*;
use num_traits::Zero;
use stwo_constraint_framework::{FrameworkEval, expr::ExprEvaluator};

#[test]
fn test_mulh_witness_gen_empty_table() {
    let table = runner::trace::MulhTable::new();
    let trace = table.into_witness();
    // Empty table produces minimal log_size = 4 (16 rows).
    assert!(!trace.is_empty());
    assert_eq!(
        trace.first().expect("trace has columns").domain.log_size(),
        4
    );
}

#[test]
fn test_mulh_interaction_trace_empty_table() {
    let table = runner::trace::MulhTable::new();
    let trace = table.into_witness();
    let relations = crate::relations::Relations::dummy();
    let (interaction_trace, claimed_sum) =
        witness::gen_interaction_trace(trace.as_slice(), &relations);
    // Interaction trace is always generated (even for padding-only tables)
    assert!(!interaction_trace.is_empty());
    assert!(claimed_sum.is_zero());
}

// =============================================================================
// E2E tests using test binaries
// =============================================================================

crate::test_bin_e2e!(mulh, mulh);
crate::test_bin_e2e!(mulh, mulhsu);
crate::test_bin_e2e!(mulh, mulhu);

#[test]
fn test_mulh_constraint_degree_bounds() {
    let eval = air::Eval {
        log_size: 6,
        relations: crate::relations::Relations::dummy(),
    };
    let expr_eval = eval.evaluate(ExprEvaluator::new());
    let degrees = expr_eval.constraint_degree_bounds();
    assert_eq!(degrees.len(), 20);
    eprintln!("mulh constraint degrees: {degrees:?}");
}
