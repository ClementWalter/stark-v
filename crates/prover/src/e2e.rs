//! E2E test infrastructure for guest programs and opcode tests.
//!
//! Provides utilities to build and run guest-bin binaries (both high-level programs
//! and opcode tests) and validate AIR constraints.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

use runner::run;

static BUILD_GUEST: Once = Once::new();

/// Build all guest-bin binaries once (includes opcode tests + high-level programs).
pub fn ensure_guest_built() {
    BUILD_GUEST.call_once(|| {
        let guest_bin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("guest")
            .join("guest-bin");

        let status = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&guest_bin_dir)
            .status()
            .expect("Failed to execute cargo build for guest-bin");

        assert!(status.success(), "Failed to build guest binaries");
    });
}

/// Path to compiled guest binaries.
pub fn guest_bin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("guest")
        .join("guest-bin")
        .join("target")
        .join("riscv32im-unknown-none-elf")
        .join("release")
}

/// Run a guest binary and return the tracer (for opcode tests).
pub fn run_test_bin(name: &str) -> runner::trace::Tracer {
    ensure_guest_built();

    let elf_path = guest_bin_dir().join(name);
    let elf_bytes =
        std::fs::read(&elf_path).unwrap_or_else(|e| panic!("Failed to read ELF {elf_path:?}: {e}"));

    let result = run(&elf_bytes, 10_000).unwrap_or_else(|e| panic!("Failed to run {name}: {e}"));

    result.tracer
}

/// Run a guest binary and return raw output bytes (for program tests).
pub fn run_guest_raw(name: &str) -> Vec<u8> {
    ensure_guest_built();

    let elf_path = guest_bin_dir().join(name);
    let elf_bytes =
        std::fs::read(&elf_path).unwrap_or_else(|e| panic!("Failed to read ELF {elf_path:?}: {e}"));

    let result =
        run(&elf_bytes, 10_000_000).unwrap_or_else(|e| panic!("Failed to run {name}: {e}"));

    result
        .output
        .unwrap_or_else(|| panic!("No output from {name}"))
}

// =============================================================================
// E2E test macro for opcode components
// =============================================================================

/// E2E test macro for opcode components.
///
/// Generates a test that:
/// 1. Runs the opcode test binary
/// 2. Validates the trace is non-empty for the expected component
/// 3. Generates witness and interaction traces
/// 4. Asserts AIR constraints hold
///
/// # Usage
/// ```ignore
/// test_bin_e2e!(base_alu_imm, addi);
/// test_bin_e2e!(branch_eq, beq);
/// ```
#[macro_export]
macro_rules! test_bin_e2e {
    ($component:ident, $opcode:ident) => {
        paste::paste! {
            #[test]
            fn [<test_ $opcode _e2e>]() {
                use stwo::core::pcs::TreeVec;
                use stwo::core::poly::circle::CanonicCoset;
                use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};

                let tracer = $crate::e2e::run_test_bin(stringify!($opcode));

                assert!(
                    !tracer.$component.is_empty(),
                    concat!("Expected ", stringify!($opcode), " trace entries in ", stringify!($component), ", got none.")
                );

                let mut counters = $crate::relations::Counters::new();
                let trace = tracer.$component.into_witness(&mut counters);

                let log_size = trace.first()
                    .map(|t| t.domain.log_size())
                    .expect("Empty trace after gen_trace");

                let relations = $crate::relations::Relations::dummy();
                let (interaction_trace, claimed_sum) =
                    witness::gen_interaction_trace(&trace, &relations);

                let traces = TreeVec::new(vec![
                    vec![],
                    trace,
                    interaction_trace,
                ]);

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
        }
    };
}
