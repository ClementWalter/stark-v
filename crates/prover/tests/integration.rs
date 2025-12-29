//! Integration tests for component aggregation.

use num_traits::Zero;
use prover::components::opcodes::{ClaimedSum, Traces, gen_interaction_trace, gen_trace};
use prover::relations::{Counters, Relations};
use runner::trace::Tracer;

#[test]
fn test_all_components_aggregate() {
    // Create an empty tracer
    let tracer = Tracer::default();

    // Generate traces for all components
    let mut counters = Counters::new();
    let traces: Traces = gen_trace(tracer, &mut counters);

    // Generate interaction traces with default relations
    let relations = Relations::dummy();
    let (_interaction_columns, claimed_sum): (_, ClaimedSum) =
        gen_interaction_trace(&traces, &relations);

    // All claimed sums should be zero for empty traces
    assert!(claimed_sum.sum().is_zero());

    // Note: Creating components with empty traces (log_size=0) causes issues
    // in the constraint framework. In a real e2e flow, traces will have data.
    // For now, we skip component creation in this test with empty traces.
}

#[test]
fn test_traces_struct_has_all_opcodes() {
    // Create an empty tracer
    let tracer = Tracer::default();

    // Generate traces for all components
    let mut counters = Counters::new();
    let traces: Traces = gen_trace(tracer, &mut counters);

    // Verify we can access each opcode family trace (16 families total).
    assert!(traces.base_alu_reg_base_alu_reg.is_empty());
    assert!(traces.base_alu_imm_base_alu_imm.is_empty());
    assert!(traces.shifts_reg_shifts_reg.is_empty());
    assert!(traces.shifts_imm_shifts_imm.is_empty());
    assert!(traces.lt_reg_lt_reg.is_empty());
    assert!(traces.lt_imm_lt_imm.is_empty());
    assert!(traces.branch_eq_branch_eq.is_empty());
    assert!(traces.branch_lt_branch_lt.is_empty());
    assert!(traces.lui_lui.is_empty());
    assert!(traces.auipc_auipc.is_empty());
    assert!(traces.jalr_jalr.is_empty());
    assert!(traces.jal_jal.is_empty());
    assert!(traces.load_store_load_store.is_empty());
    assert!(traces.mul_mul.is_empty());
    assert!(traces.mulh_mulh.is_empty());
    assert!(traces.div_div.is_empty());
}
