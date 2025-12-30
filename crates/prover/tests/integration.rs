//! Integration tests for component aggregation.

use num_traits::Zero;
use prover::components::opcodes::{ClaimedSum, Traces, gen_interaction_trace, gen_trace};
use prover::relations::{Counters, Relations};
use runner::trace::Tracer;
use stwo::core::fri::FriConfig;
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
    use prover::components::opcodes;
    use prover::e2e::{ensure_guest_built, guest_bin_dir};
    use prover::relations::{Counters, Relations};
    use runner::run;
    use stwo::core::pcs::TreeVec;
    use stwo::core::poly::circle::CanonicCoset;
    use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};

    ensure_guest_built();

    let elf_path = guest_bin_dir().join("fib");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib ELF");

    let run_result = run(&elf_bytes, 10_000_000).expect("Failed to run fib");
    let tracer = run_result.tracer;

    // Generate opcode traces
    let mut counters = Counters::new();
    let traces = opcodes::gen_trace(tracer, &mut counters);

    let relations = Relations::dummy();

    // Helper macro to test a single component
    macro_rules! test_component {
        ($name:ident, $module:ident) => {
            if !traces.$name.is_empty() {
                let log_size = traces
                    .$name
                    .first()
                    .map(|t| t.domain.log_size())
                    .unwrap_or(0);
                if log_size > 0 {
                    let (interaction_trace, claimed_sum) =
                        opcodes::$module::witness::gen_interaction_trace(&traces.$name, &relations);

                    let trace_tree = TreeVec::new(vec![
                        vec![], // preprocessed (empty for now)
                        traces.$name.clone(),
                        interaction_trace,
                    ]);

                    let trace_polys = trace_tree.map_cols(|c| c.interpolate());

                    let eval = opcodes::$module::air::Eval {
                        log_size,
                        relations: relations.clone(),
                    };

                    println!(
                        "Testing {} constraints (log_size={})",
                        stringify!($name),
                        log_size
                    );
                    assert_constraints_on_polys(
                        &trace_polys,
                        CanonicCoset::new(log_size),
                        |assert_eval| {
                            eval.evaluate(assert_eval);
                        },
                        claimed_sum,
                    );
                    println!("{} constraints OK", stringify!($name));
                }
            }
        };
    }

    // Test each component's constraints
    test_component!(base_alu_reg, base_alu_reg);
    test_component!(base_alu_imm, base_alu_imm);
    test_component!(shifts_reg, shifts_reg);
    test_component!(shifts_imm, shifts_imm);
    test_component!(lt_reg, lt_reg);
    test_component!(lt_imm, lt_imm);
    test_component!(branch_eq, branch_eq);
    test_component!(branch_lt, branch_lt);
    test_component!(lui, lui);
    test_component!(auipc, auipc);
    test_component!(jalr, jalr);
    test_component!(jal, jal);
    test_component!(load_store, load_store);
    test_component!(mul, mul);
    test_component!(mulh, mulh);
    test_component!(div, div);

    println!("All component constraints satisfied!");
}
