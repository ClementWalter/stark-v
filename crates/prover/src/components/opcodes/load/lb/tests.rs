//! Tests for LB component.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

use stwo::core::pcs::TreeVec;
use stwo::core::poly::circle::CanonicCoset;
use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};

use super::*;
use crate::relations::{Counters, Relations};
use runner::run;
use runner::trace::LbTable;

static BUILD_TESTS: Once = Once::new();

fn ensure_tests_built() {
    BUILD_TESTS.call_once(|| {
        let guest_tests_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../guest/guest-tests");
        let status = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&guest_tests_dir)
            .status()
            .expect("Failed to build");
        assert!(status.success());
    });
}

fn guest_tests_bin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../guest/guest-tests/target/riscv32im-unknown-none-elf/release")
}

fn run_test_bin(name: &str) -> runner::trace::Tracer {
    ensure_tests_built();
    let elf_path = guest_tests_bin_dir().join(name);
    let elf_bytes =
        std::fs::read(&elf_path).unwrap_or_else(|e| panic!("Failed to read ELF {elf_path:?}: {e}"));
    run(&elf_bytes, 10_000)
        .unwrap_or_else(|e| panic!("Failed to run {name}: {e}"))
        .tracer
}

#[test]
fn test_lb_empty_table() {
    let table = LbTable::default();
    let trace = witness::gen_trace(table, &mut Counters::new());
    assert!(trace.is_empty());
}

#[test]
fn test_lb_interaction_trace() {
    let table = LbTable::default();
    let trace = witness::gen_trace(table, &mut Counters::new());
    let relations = Relations::dummy();
    let (_interaction_trace, claimed_sum) = witness::gen_interaction_trace(&trace, &relations);
    use num_traits::Zero;
    assert!(claimed_sum.is_zero());
}

#[test]
fn test_lb_e2e() {
    let tracer = run_test_bin("lb");
    assert!(!tracer.lb.is_empty());
    let mut counters = Counters::new();
    let trace = witness::gen_trace(tracer.lb, &mut counters);
    let log_size = trace
        .first()
        .map(|t| t.domain.log_size())
        .expect("Empty trace");
    let relations = Relations::dummy();
    let (interaction_trace, claimed_sum) = witness::gen_interaction_trace(&trace, &relations);
    let traces = TreeVec::new(vec![vec![], trace, interaction_trace]);
    let trace_polys = traces.map_cols(|c| c.interpolate());
    let eval = air::Eval {
        log_size,
        relations: relations.clone(),
    };
    assert_constraints_on_polys(
        &trace_polys,
        CanonicCoset::new(log_size),
        |assert_eval| {
            eval.evaluate(assert_eval);
        },
        claimed_sum,
    );
}
