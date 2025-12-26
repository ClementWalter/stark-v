//! Tests for SLTI component.

use super::*;
use crate::relations::Relations;
use runner::trace::SltiTable;

#[test]
fn test_slti_empty_table() {
    let table = SltiTable::default();
    let trace = witness::gen_trace(table, &mut crate::relations::Counters::new());
    // Empty table returns empty trace
    assert!(trace.is_empty());
}

#[test]
fn test_slti_interaction_trace() {
    let table = SltiTable::default();
    let trace = witness::gen_trace(table, &mut crate::relations::Counters::new());
    let relations = Relations::dummy();

    let (_interaction_trace, claimed_sum) = witness::gen_interaction_trace(&trace, &relations);

    use num_traits::Zero;
    assert!(claimed_sum.is_zero());
}
