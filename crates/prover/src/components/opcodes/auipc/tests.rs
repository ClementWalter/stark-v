//! Tests for auipc component.

use super::*;

#[test]
fn test_auipc_witness_gen_empty_table() {
    let table = runner::trace::AuipcTable::new();
    let trace = table.into_witness();
    // Empty table produces minimal log_size = 4 (16 rows)
    assert!(!trace.is_empty());
    assert_eq!(
        trace.first().expect("trace has columns").domain.log_size(),
        4
    );
}

#[test]
fn test_auipc_interaction_trace_empty_table() {
    let table = runner::trace::AuipcTable::new();
    let trace = table.into_witness();
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

crate::test_bin_e2e!(auipc, auipc);
