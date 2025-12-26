//! Tests for MULHU component.

use super::*;
use crate::relations::Relations;
use runner::trace::MulhuTable;

#[test]
fn test_mulhu_empty_table() {
    let table = MulhuTable::default();
    let trace = witness::gen_trace(table);
    // Empty table returns empty trace
    assert!(trace.is_empty());
}

#[test]
fn test_mulhu_interaction_trace() {
    let table = MulhuTable::default();
    let trace = witness::gen_trace(table);
    let relations = Relations::dummy();

    let (_interaction_trace, claimed_sum) = witness::gen_interaction_trace(&trace, &relations);

    use num_traits::Zero;
    assert!(claimed_sum.is_zero());
}
