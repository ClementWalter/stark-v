//! Tests for load_store component.

use super::*;
use num_traits::Zero;
use stwo::core::fields::qm31::QM31;

#[test]
fn test_load_store_witness_gen_empty_table() {
    let table = runner::trace::LoadStoreTable::new();
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
fn test_load_store_interaction_trace_empty_table() {
    let table = runner::trace::LoadStoreTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = table.into_witness(&mut counters);
    let relations = crate::relations::Relations::dummy();
    let (interaction_trace, claimed_sum) =
        witness::gen_interaction_trace(trace.as_slice(), &relations);
    assert!(interaction_trace.is_empty());
    assert_eq!(claimed_sum, QM31::zero());
}

// =============================================================================
// E2E tests using test binaries
// =============================================================================

crate::test_bin_e2e!(load_store, lb);
crate::test_bin_e2e!(load_store, lh);
crate::test_bin_e2e!(load_store, lw);
crate::test_bin_e2e!(load_store, lbu);
crate::test_bin_e2e!(load_store, lhu);
crate::test_bin_e2e!(load_store, sb);
crate::test_bin_e2e!(load_store, sh);
crate::test_bin_e2e!(load_store, sw);
