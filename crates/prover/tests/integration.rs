//! Integration tests for component aggregation.

use num_traits::Zero;
use prover::components::opcodes::{ClaimedSum, Traces, gen_interaction_trace, gen_trace};
use prover::relations::{Counters, Relations};
use runner::trace::Tracer;
use stwo::core::pcs::PcsConfig;
use tracing::info;

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
    assert!(claimed_sum.sum().is_zero());
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

/// Full end-to-end proof + verification for Fibonacci.
#[test_log::test]
fn test_prove_verify_fibonacci() {
    use prover::e2e::{ensure_guest_built, guest_bin_dir};
    use prover::{prove_rv32im, verify_rv32im};
    use runner::run;

    ensure_guest_built();

    let elf_path = guest_bin_dir().join("fib");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib ELF");

    let run_result = run(&elf_bytes, 10_000_000).expect("Failed to run fib");

    let proof = prove_rv32im(run_result, PcsConfig::default());
    verify_rv32im(proof, PcsConfig::default()).expect("Verification failed");
}

/// End-to-end benchmark for Fibonacci with input.
#[test_log::test]
fn test_e2e_fibonacci_benchmark() {
    use prover::e2e::{ensure_guest_built, guest_bin_dir};
    use prover::{prove_rv32im, verify_rv32im};
    use runner::run_with_input;
    use serde::Deserialize;
    use std::time::Instant;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct FibResult {
        n: u32,
        value: u32,
    }

    fn fib_value(n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        if n == 1 {
            return 1;
        }
        let mut a = 0u32;
        let mut b = 1u32;
        let mut i = 2u32;
        while i <= n {
            let tmp = a.wrapping_add(b);
            a = b;
            b = tmp;
            i += 1;
        }
        b
    }

    ensure_guest_built();

    let n: u32 = std::env::var("STARKV_FIB_N")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1_000);
    let input = n.to_le_bytes();

    let elf_path = guest_bin_dir().join("fib_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib_input ELF");

    let run_start = Instant::now();
    let run_result =
        run_with_input(&elf_bytes, &input, 100_000_000).expect("Failed to run fib_input");
    let run_elapsed = run_start.elapsed();

    let output_bytes = run_result
        .output
        .as_ref()
        .expect("No output from fib_input");
    let output: FibResult =
        postcard::from_bytes(output_bytes).expect("Failed to decode fib output");
    assert_eq!(output.n, n);
    assert_eq!(output.value, fib_value(n));

    let cycles = run_result.cycles;
    assert!(cycles > 0, "No cycles reported");

    let prove_start = Instant::now();
    let proof = prove_rv32im(run_result, PcsConfig::default());
    let prove_elapsed = prove_start.elapsed();

    verify_rv32im(proof, PcsConfig::default()).expect("Verification failed");

    let run_prove_elapsed = run_elapsed + prove_elapsed;
    let cycles_f = cycles as f64;
    let run_secs = run_elapsed.as_secs_f64().max(f64::MIN_POSITIVE);
    let run_hz = (cycles_f / run_secs).ceil() as u64;
    let run_khz = run_hz as f64 / 1_000.0;
    let run_prove_secs = run_prove_elapsed.as_secs_f64().max(f64::MIN_POSITIVE);
    let prove_secs = prove_elapsed.as_secs_f64().max(f64::MIN_POSITIVE);
    let run_prove_hz = (cycles_f / run_prove_secs).ceil() as u64;
    let prove_hz = (cycles_f / prove_secs).ceil() as u64;
    let run_prove_khz = run_prove_hz as f64 / 1_000.0;
    let prove_khz = prove_hz as f64 / 1_000.0;

    info!("fib_input benchmark");
    info!("  n: {n}");
    info!("  cycles: {cycles}");
    info!("  run:     {run_khz:>10.3} kHz  ({run_secs:.3}s)",);
    info!("  run+prove: {run_prove_khz:>10.3} kHz  ({run_prove_secs:.3}s)",);
    info!("  prove:     {prove_khz:>10.3} kHz  ({prove_secs:.3}s)",);
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

    let traces = components::gen_trace(run_result.tracer);
    let relations = Relations::dummy();

    Components::assert_constraints_on_polys(&traces, &relations);
}

/// Test constraint satisfaction for Fibonacci with explicit input.
#[test_log::test]
fn test_fibonacci_input_constraints() {
    use prover::components::{self, Components};
    use prover::e2e::{ensure_guest_built, guest_bin_dir};
    use prover::relations::Relations;
    use runner::run_with_input;

    ensure_guest_built();

    let elf_path = guest_bin_dir().join("fib_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib_input ELF");

    let input = 20u32.to_le_bytes();
    let run_result =
        run_with_input(&elf_bytes, &input, 10_000_000).expect("Failed to run fib_input");

    let traces = components::gen_trace(run_result.tracer);
    let relations = Relations::dummy();

    Components::assert_constraints_on_polys(&traces, &relations);
}
