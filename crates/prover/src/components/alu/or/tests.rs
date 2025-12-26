//! Tests for OR component.

use super::*;
use crate::relations::Relations;
use runner::trace::OrTable;

#[test]
fn test_or_empty_table() {
    let table = OrTable::default();
    let trace = witness::gen_trace(table);
    // Empty table returns empty trace
    assert!(trace.is_empty());
}

#[test]
fn test_or_interaction_trace() {
    let table = OrTable::default();
    let trace = witness::gen_trace(table);
    let relations = Relations::dummy();

    let (_interaction_trace, claimed_sum) = witness::gen_interaction_trace(&trace, &relations);

    use num_traits::Zero;
    assert!(claimed_sum.is_zero());
}
