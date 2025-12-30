//! Tests for div component.

use super::*;
use num_traits::Zero;
use stwo::core::fields::qm31::QM31;

#[test]
fn test_div_witness_gen_empty() {
    let table = runner::trace::DivTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = witness::gen_trace(table, &mut counters);
    assert!(trace.is_empty());
}

#[test]
fn test_div_interaction_trace() {
    let table = runner::trace::DivTable::new();
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

crate::test_bin_e2e!(div, div);
crate::test_bin_e2e!(div, divu);
crate::test_bin_e2e!(div, rem);
crate::test_bin_e2e!(div, remu);
