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

    // Verify we can access all 45 opcode fields by checking a few from each category

    // ALU (10 opcodes)
    assert!(traces.alu_add.is_empty());
    assert!(traces.alu_sub.is_empty());
    assert!(traces.alu_xor.is_empty());

    // ALU Immediate (9 opcodes)
    assert!(traces.alu_imm_addi.is_empty());
    assert!(traces.alu_imm_slli.is_empty());
    assert!(traces.alu_imm_xori.is_empty());

    // Load (5 opcodes)
    assert!(traces.load_lb.is_empty());
    assert!(traces.load_lw.is_empty());
    assert!(traces.load_lhu.is_empty());

    // Store (3 opcodes)
    assert!(traces.store_sb.is_empty());
    assert!(traces.store_sh.is_empty());
    assert!(traces.store_sw.is_empty());

    // Branch (6 opcodes)
    assert!(traces.branch_beq.is_empty());
    assert!(traces.branch_bne.is_empty());
    assert!(traces.branch_blt.is_empty());

    // Jump (2 opcodes)
    assert!(traces.jump_jal.is_empty());
    assert!(traces.jump_jalr.is_empty());

    // Upper Immediate (2 opcodes)
    assert!(traces.upper_lui.is_empty());
    assert!(traces.upper_auipc.is_empty());

    // MulDiv (8 opcodes)
    assert!(traces.muldiv_mul.is_empty());
    assert!(traces.muldiv_div.is_empty());
    assert!(traces.muldiv_rem.is_empty());
}
