//! Integration tests for component aggregation.

use num_traits::Zero;
use prover::components::opcodes::{ClaimedSum, Traces, gen_interaction_trace, gen_trace};
use prover::relations::{Counters, Relations};
use runner::trace::Tracer;
use stwo::core::pcs::PcsConfig;

#[test]
fn test_all_components_aggregate() {
    // Create an empty tracer
    let tracer = Tracer::default();

    // Generate traces for all components
    let mut counters = Counters::new();
    let traces: Traces = gen_trace(tracer, &mut counters);

    // Generate interaction traces with default relations
    let relations = Relations::dummy();
    let (interaction_columns, claimed_sum): (_, ClaimedSum) =
        gen_interaction_trace(&traces, &relations);

    assert!(!interaction_columns.is_empty());
    assert!(!claimed_sum.sum().is_zero());
}

#[test]
fn test_traces_struct_has_all_opcodes() {
    // Create an empty tracer
    let tracer = Tracer::default();

    // Generate traces for all components
    let mut counters = Counters::new();
    let traces: Traces = gen_trace(tracer, &mut counters);

    // Verify we can access each opcode family trace (16 families total).
    assert!(!traces.base_alu_reg.is_empty());
    assert!(!traces.base_alu_imm.is_empty());
    assert!(!traces.shifts_reg.is_empty());
    assert!(!traces.shifts_imm.is_empty());
    assert!(!traces.lt_reg.is_empty());
    assert!(!traces.lt_imm.is_empty());
    assert!(!traces.branch_eq.is_empty());
    assert!(!traces.branch_lt.is_empty());
    assert!(!traces.lui.is_empty());
    assert!(!traces.auipc.is_empty());
    assert!(!traces.jalr.is_empty());
    assert!(!traces.jal.is_empty());
    assert!(!traces.load_store.is_empty());
    assert!(!traces.mul.is_empty());
    assert!(!traces.mulh.is_empty());
    assert!(!traces.div.is_empty());
}

/// Test proving a small example (scaffolding - no real constraints yet).
#[test_log::test]
fn test_prove_fibonacci() {
    use prover::e2e::{ensure_guest_built, guest_bin_dir};
    use prover::prove_rv32im;
    use runner::run;

    ensure_guest_built();

    let elf_path = guest_bin_dir().join("fib");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib ELF");

    let run_result = run(&elf_bytes, 10_000_000).expect("Failed to run fib");

    // Generate proof
    let _proof = prove_rv32im(run_result, PcsConfig::default());
}

/// Test constraint satisfaction using assert_constraints_on_polys for each component.
/// This helps identify which specific component's constraints are failing.
#[test_log::test]
fn test_fibonacci_constraints() {
    use prover::components::{self, Components};
    use prover::e2e::{ensure_guest_built, guest_bin_dir};
    use prover::relations::Relations;
    use runner::run;

    ensure_guest_built();

    let elf_path = guest_bin_dir().join("fib");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib ELF");

    let run_result = run(&elf_bytes, 10_000_000).expect("Failed to run fib");

    dbg!(&run_result.tracer.program);
    let traces = components::gen_trace(run_result.tracer);
    let relations = Relations::dummy();

    Components::assert_constraints_on_polys(&traces, &relations);
}
