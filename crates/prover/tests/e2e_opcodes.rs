//! End-to-end tests for individual opcode AIR constraints.
//!
//! Each test:
//! 1. Builds a minimal RISC-V binary with inline asm for a specific opcode
//! 2. Runs it through the interpreter to get traces
//! 3. Generates witness and interaction traces
//! 4. Asserts constraints are satisfied on the trace polynomials
//!
//! Note: Currently all AIR constraints are dummy (clk - clk = 0).
//! These tests validate the trace generation infrastructure.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

use prover::components::opcodes;
use prover::relations::{Counters, Relations};
use runner::run;
use stwo::core::pcs::TreeVec;
use stwo::core::poly::circle::CanonicCoset;
use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};

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

/// Macro to generate e2e test for a single opcode.
///
/// Usage: `opcode_e2e_test!(category::opcode, tracer_field);`
/// Example: `opcode_e2e_test!(alu::add, add);`
macro_rules! opcode_e2e_test {
    ($category:ident :: $opcode:ident, $tracer_field:ident) => {
        paste::paste! {
            #[test]
            fn [<test_ $opcode _e2e>]() {
                let tracer = run_test_bin(stringify!($opcode));

                // Verify trace was captured
                assert!(
                    !tracer.$tracer_field.is_empty(),
                    concat!("Expected ", stringify!($opcode), " trace entries, got none. ",
                           "Make sure the binary executes the instruction.")
                );

                // Generate traces
                let mut counters = Counters::new();
                let trace = opcodes::$category::$opcode::witness::gen_trace(
                    tracer.$tracer_field,
                    &mut counters,
                );

                // Get log_size from trace
                let log_size = trace.first()
                    .map(|t| t.domain.log_size())
                    .expect("Empty trace after gen_trace");

                // Generate interaction trace
                let relations = Relations::dummy();
                let (interaction_trace, claimed_sum) =
                    opcodes::$category::$opcode::witness::gen_interaction_trace(&trace, &relations);

                // Build TreeVec: [preprocessed, main, interaction]
                // Currently no preprocessed columns for individual opcode tests
                let traces = TreeVec::new(vec![
                    vec![],  // No preprocessed
                    trace,
                    interaction_trace,
                ]);

                // Convert to polynomials
                let trace_polys = traces.map_cols(|c| c.interpolate());

                // Create the evaluator
                let eval = opcodes::$category::$opcode::air::Eval {
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
        }
    };
}

// =============================================================================
// ALU opcode tests (10)
// =============================================================================

opcode_e2e_test!(alu::add, add);
opcode_e2e_test!(alu::sub, sub);
opcode_e2e_test!(alu::sll, sll);
opcode_e2e_test!(alu::slt, slt);
opcode_e2e_test!(alu::sltu, sltu);
opcode_e2e_test!(alu::xor, xor);
opcode_e2e_test!(alu::srl, srl);
opcode_e2e_test!(alu::sra, sra);
opcode_e2e_test!(alu::or, or);
opcode_e2e_test!(alu::and, and);

// =============================================================================
// ALU Immediate opcode tests (9)
// =============================================================================

opcode_e2e_test!(alu_imm::addi, addi);
opcode_e2e_test!(alu_imm::slti, slti);
opcode_e2e_test!(alu_imm::sltiu, sltiu);
opcode_e2e_test!(alu_imm::xori, xori);
opcode_e2e_test!(alu_imm::ori, ori);
opcode_e2e_test!(alu_imm::andi, andi);
opcode_e2e_test!(alu_imm::slli, slli);
opcode_e2e_test!(alu_imm::srli, srli);
opcode_e2e_test!(alu_imm::srai, srai);

// =============================================================================
// Load opcode tests (5)
// =============================================================================

opcode_e2e_test!(load::lb, lb);
opcode_e2e_test!(load::lh, lh);
opcode_e2e_test!(load::lw, lw);
opcode_e2e_test!(load::lbu, lbu);
opcode_e2e_test!(load::lhu, lhu);

// =============================================================================
// Store opcode tests (3)
// =============================================================================

opcode_e2e_test!(store::sb, sb);
opcode_e2e_test!(store::sh, sh);
opcode_e2e_test!(store::sw, sw);

// =============================================================================
// Branch opcode tests (6)
// =============================================================================

opcode_e2e_test!(branch::beq, beq);
opcode_e2e_test!(branch::bne, bne);
opcode_e2e_test!(branch::blt, blt);
opcode_e2e_test!(branch::bge, bge);
opcode_e2e_test!(branch::bltu, bltu);
opcode_e2e_test!(branch::bgeu, bgeu);

// =============================================================================
// Jump opcode tests (2)
// =============================================================================

opcode_e2e_test!(jump::jal, jal);
opcode_e2e_test!(jump::jalr, jalr);

// =============================================================================
// Upper Immediate opcode tests (2)
// =============================================================================

opcode_e2e_test!(upper::lui, lui);
opcode_e2e_test!(upper::auipc, auipc);

// =============================================================================
// MulDiv opcode tests (8)
// =============================================================================

opcode_e2e_test!(muldiv::mul, mul);
opcode_e2e_test!(muldiv::mulh, mulh);
opcode_e2e_test!(muldiv::mulhsu, mulhsu);
opcode_e2e_test!(muldiv::mulhu, mulhu);
opcode_e2e_test!(muldiv::div, div);
opcode_e2e_test!(muldiv::divu, divu);
opcode_e2e_test!(muldiv::rem, rem);
opcode_e2e_test!(muldiv::remu, remu);
