//! Tests for branch_lt component.

use super::*;
use num_traits::Zero;
use stwo::core::fields::qm31::QM31;

#[test]
fn test_branch_lt_witness_gen_empty_table() {
    let table = runner::trace::BranchLtTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = witness::gen_trace(table, &mut counters);
    // Empty table produces minimal log_size = 4 (16 rows)
    assert!(!trace.is_empty());
    assert_eq!(
        trace.first().expect("trace has columns").domain.log_size(),
        4
    );
}

#[test]
fn test_branch_lt_interaction_trace_empty_table() {
    let table = runner::trace::BranchLtTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = witness::gen_trace(table, &mut counters);
    let relations = crate::relations::Relations::dummy();
    let (interaction_trace, claimed_sum) = witness::gen_interaction_trace(&trace, &relations);
    assert!(interaction_trace.is_empty());
    assert_eq!(claimed_sum, QM31::zero());
}

// =============================================================================
// E2E tests using test binaries
// =============================================================================

crate::test_bin_e2e!(branch_lt, blt);
crate::test_bin_e2e!(branch_lt, bge);
crate::test_bin_e2e!(branch_lt, bltu);
crate::test_bin_e2e!(branch_lt, bgeu);
