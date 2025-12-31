//! Tests for shifts_imm component.

use super::*;
use num_traits::Zero;
use stwo::core::fields::qm31::QM31;

#[test]
fn test_shifts_imm_witness_gen_empty_table() {
    let table = runner::trace::ShiftsImmTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = table.to_witness(&mut counters);
    // Empty table produces minimal log_size = 4 (16 rows)
    assert!(!trace.is_empty());
    assert_eq!(
        trace.first().expect("trace has columns").domain.log_size(),
        4
    );
}

#[test]
fn test_shifts_imm_interaction_trace_empty_table() {
    let table = runner::trace::ShiftsImmTable::new();
    let mut counters = crate::relations::Counters::new();
    let _trace = table.to_witness(&mut counters);
    let relations = crate::relations::Relations::dummy();
    let (interaction_trace, claimed_sum) = witness::gen_interaction_trace(&table, &relations);
    assert!(interaction_trace.is_empty());
    assert_eq!(claimed_sum, QM31::zero());
}

// =============================================================================
// E2E tests using test binaries
// =============================================================================

crate::test_bin_e2e!(shifts_imm, slli);
crate::test_bin_e2e!(shifts_imm, srli);
crate::test_bin_e2e!(shifts_imm, srai);
