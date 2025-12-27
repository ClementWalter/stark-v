//! Tests for BGEU component.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

use stwo::core::pcs::TreeVec;
use stwo::core::poly::circle::CanonicCoset;
use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};

use super::*;
use crate::relations::{Counters, Relations};
use runner::run;
use runner::trace::BgeuTable;

static BUILD_TESTS: Once = Once::new();

/// Build guest-tests binaries once before running tests.
fn ensure_tests_built() {
    BUILD_TESTS.call_once(|| {
        let guest_tests_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../guest/guest-tests");

        let status = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&guest_tests_dir)
            .status()
            .expect("Failed to execute cargo build for guest-tests");

        assert!(status.success(), "Failed to build guest-tests binaries");
    });
}

/// Path to the guest-tests binary directory.
fn guest_tests_bin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../guest/guest-tests/target/riscv32im-unknown-none-elf/release")
}

/// Run a guest-tests binary and return the tracer.
fn run_test_bin(name: &str) -> runner::trace::Tracer {
    ensure_tests_built();

    let elf_path = guest_tests_bin_dir().join(name);
    let elf_bytes =
        std::fs::read(&elf_path).unwrap_or_else(|e| panic!("Failed to read ELF {elf_path:?}: {e}"));

    let result = run(&elf_bytes, 10_000).unwrap_or_else(|e| panic!("Failed to run {name}: {e}"));

    result.tracer
}

#[test]
fn test_bgeu_empty_table() {
    let table = BgeuTable::default();
    let trace = witness::gen_trace(table, &mut Counters::new());
    // Empty table returns empty trace
    assert!(trace.is_empty());
}

#[test]
fn test_bgeu_interaction_trace() {
    let table = BgeuTable::default();
    let trace = witness::gen_trace(table, &mut Counters::new());
    let relations = Relations::dummy();

    let (_interaction_trace, claimed_sum) = witness::gen_interaction_trace(&trace, &relations);

    use num_traits::Zero;
    assert!(claimed_sum.is_zero());
}

#[test]
fn test_bgeu_e2e() {
    let tracer = run_test_bin("bgeu");

    // Verify trace was captured
    assert!(
        !tracer.bgeu.is_empty(),
        "Expected BGEU trace entries, got none. Make sure the binary executes the instruction."
    );

    // Generate traces
    let mut counters = Counters::new();
    let trace = witness::gen_trace(tracer.bgeu, &mut counters);

    // Get log_size from trace
    let log_size = trace
        .first()
        .map(|t| t.domain.log_size())
        .expect("Empty trace after gen_trace");

    // Generate interaction trace
    let relations = Relations::dummy();
    let (interaction_trace, claimed_sum) = witness::gen_interaction_trace(&trace, &relations);

    // Build TreeVec: [preprocessed, main, interaction]
    let traces = TreeVec::new(vec![
        vec![], // No preprocessed
        trace,
        interaction_trace,
    ]);

    // Convert to polynomials
    let trace_polys = traces.map_cols(|c| c.interpolate());

    // Create the evaluator
    let eval = air::Eval {
        log_size,
        relations: relations.clone(),
    };

    // Assert constraints
    assert_constraints_on_polys(
        &trace_polys,
        CanonicCoset::new(log_size),
        |assert_eval| {
            eval.evaluate(assert_eval);
        },
        claimed_sum,
    );
}
