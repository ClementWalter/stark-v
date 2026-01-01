//! Tests for lt_reg component.

use super::*;
use num_traits::Zero;
use stwo::core::fields::qm31::QM31;

#[test]
fn test_lt_reg_witness_gen_empty_table() {
    let table = runner::trace::LtRegTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = table.into_witness(&mut counters);
    // Empty table produces minimal log_size = 4 (16 rows)
    assert!(!trace.is_empty());
    assert_eq!(
        trace.first().expect("trace has columns").domain.log_size(),
        4
    );
}

#[test]
fn test_lt_reg_interaction_trace_empty_table() {
    let table = runner::trace::LtRegTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = table.into_witness(&mut counters);
    let relations = crate::relations::Relations::dummy();
    let (interaction_trace, claimed_sum) =
        witness::gen_interaction_trace(trace.as_slice(), &relations);
    // Interaction trace is always generated (even for padding-only tables)
    assert!(!interaction_trace.is_empty());
    // claimed_sum won't be zero due to range check entries with -1 numerator
    let _ = claimed_sum;
}

// =============================================================================
// E2E tests using test binaries
// =============================================================================

crate::test_bin_e2e!(lt_reg, slt);
crate::test_bin_e2e!(lt_reg, sltu);
