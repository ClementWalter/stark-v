//! Tests for base_alu_imm component.

use super::*;
use crate::witness::IntoWitness;
use num_traits::Zero;
use stwo::core::fields::qm31::QM31;

#[test]
fn test_base_alu_imm_witness_gen_empty_table() {
    let table = runner::trace::BaseAluImmTable::new();
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
fn test_base_alu_imm_interaction_trace_empty_table() {
    let table = runner::trace::BaseAluImmTable::new();
    let mut counters = crate::relations::Counters::new();
    let trace = table.into_witness(&mut counters);
    let relations = crate::relations::Relations::dummy();
    let (interaction_trace, claimed_sum) = witness::gen_interaction_trace(&trace, &relations);
    assert!(interaction_trace.is_empty());
    assert_eq!(claimed_sum, QM31::zero());
}

// =============================================================================
// E2E tests using test binaries
// =============================================================================

crate::test_bin_e2e!(base_alu_imm, addi);
crate::test_bin_e2e!(base_alu_imm, xori);
crate::test_bin_e2e!(base_alu_imm, ori);
crate::test_bin_e2e!(base_alu_imm, andi);
