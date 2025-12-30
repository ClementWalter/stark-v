//! Tests for shifts_imm component.

use super::*;
use num_traits::Zero;
use stwo::core::fields::qm31::QM31;

#[test]
fn test_shifts_imm_witness_gen_empty() {
    let table = runner::trace::Shifts_immTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = witness::gen_trace(table, &mut counters);
    assert!(trace.is_empty());
}

#[test]
fn test_shifts_imm_interaction_trace() {
    let table = runner::trace::Shifts_immTable::new();
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

crate::test_bin_e2e!(shifts_imm, slli);
crate::test_bin_e2e!(shifts_imm, srli);
crate::test_bin_e2e!(shifts_imm, srai);
