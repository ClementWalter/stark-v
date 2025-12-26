//! Tests for SLTIU component.

use super::*;
use crate::relations::Relations;
use runner::trace::SltiuTable;

#[test]
fn test_sltiu_empty_table() {
    let table = SltiuTable::default();
    let trace = witness::gen_trace(table);
    // Empty table returns empty trace
    assert!(trace.is_empty());
}

#[test]
fn test_sltiu_interaction_trace() {
    let table = SltiuTable::default();
    let trace = witness::gen_trace(table);
    let relations = Relations::dummy();

    let (_interaction_trace, claimed_sum) = witness::gen_interaction_trace(&trace, &relations);

    use num_traits::Zero;
    assert!(claimed_sum.is_zero());
}
