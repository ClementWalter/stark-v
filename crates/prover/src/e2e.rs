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
///
/// # Panics
///
/// Panics if the cargo build command fails to execute or if the build fails.
/// This is acceptable for test infrastructure as build failures should halt all tests.
pub fn ensure_guest_built() {
    BUILD_GUEST.call_once(|| {
        let guest_bin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("guest")
            .join("guest-bin");

        // Note: This expect() is safe in test context - build failures should halt all tests
        let status = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(&guest_bin_dir)
            .status()
            .expect(
                "Failed to execute cargo build command - cargo may not be installed or accessible",
            );

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
/// 5. Registers multiplicities and generates preprocessed traces
/// 6. (With track-relations) Tracks and prints relation summary including preprocessed
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

                let trace = tracer.$component.into_witness();

                // Note: trace is guaranteed non-empty by the assert above
                let log_size = trace.first()
                    .map(|t| t.domain.log_size())
                    .expect("Empty trace after gen_trace - this should be unreachable");

                let relations = $crate::relations::Relations::dummy();
                let (interaction_trace, claimed_sum) =
                    witness::gen_interaction_trace(trace.as_slice(), &relations);

                let traces = TreeVec::new(vec![
                    vec![],
                    trace.clone(),
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

                // Track and print relation summary for debugging LogUp imbalances
                // Includes both opcode and preprocessed components to show balanced relations
                #[cfg(feature = "track-relations")]
                {
                    use stwo_constraint_framework::{FrameworkComponent, TraceLocationAllocator};
                    use stwo_constraint_framework::relation_tracker::{
                        add_to_relation_entries, RelationSummary, RelationTrackerEntry,
                    };
                    use $crate::preprocessed::PreprocessedTable;

                    // Register multiplicities to get preprocessed traces
                    let mut counters = $crate::relations::Counters::new();
                    witness::register_multiplicities(trace.as_slice(), &mut counters);

                    // Convert counters to preprocessed traces
                    let prep_traces = $crate::components::preprocessed::Traces::from_counters(counters);

                    let mut all_entries: Vec<RelationTrackerEntry> = vec![];

                    // 1. Collect entries from the opcode component
                    {
                        let mut allocator = TraceLocationAllocator::default();
                        let component = FrameworkComponent::new(&mut allocator, eval, claimed_sum);

                        let trace_values: TreeVec<Vec<Vec<stwo::core::fields::m31::BaseField>>> = TreeVec::new(vec![
                            vec![], // preprocessed (empty for opcode)
                            trace.iter().map(|col| col.to_cpu().values).collect(),
                        ]);
                        let trace_refs = trace_values.as_cols_ref();

                        all_entries.extend(add_to_relation_entries(&component, &trace_refs));
                    }

                    // 2. Collect entries from preprocessed components
                    // Helper macro to add preprocessed entries
                    macro_rules! add_preprocessed_entries {
                        ($prep:ident) => {{
                            use $crate::components::preprocessed::$prep::{air as prep_air, witness as prep_witness};

                            let multiplicity_trace = &prep_traces.$prep;
                            if !multiplicity_trace.is_empty() {
                                let prep_log_size = $crate::preprocessed::$prep::Table::LOG_SIZE;
                                let preprocessed_columns = $crate::preprocessed::$prep::Table::gen_columns();

                                let (_prep_interaction, prep_claimed) =
                                    prep_witness::gen_interaction_trace(multiplicity_trace, &relations);

                                let prep_eval = prep_air::Eval {
                                    log_size: prep_log_size,
                                    relations: relations.clone(),
                                };

                                let mut allocator = TraceLocationAllocator::default();
                                let component = FrameworkComponent::new(&mut allocator, prep_eval, prep_claimed);

                                let trace_values: TreeVec<Vec<Vec<stwo::core::fields::m31::BaseField>>> = TreeVec::new(vec![
                                    preprocessed_columns.iter().map(|col| col.to_cpu().values).collect(),
                                    multiplicity_trace.iter().map(|col| col.to_cpu().values).collect(),
                                ]);
                                let trace_refs = trace_values.as_cols_ref();

                                all_entries.extend(add_to_relation_entries(&component, &trace_refs));
                            }
                        }};
                    }

                    add_preprocessed_entries!(bitwise);
                    add_preprocessed_entries!(range_check_20);
                    add_preprocessed_entries!(range_check_8_8);
                    add_preprocessed_entries!(range_check_8_11);
                    add_preprocessed_entries!(range_check_8_8_4);
                    add_preprocessed_entries!(range_check_m31);

                    let summary = RelationSummary::summarize_relations(&all_entries).cleaned();

                    println!("\n=== Relation Summary for {} (with preprocessed) ===", stringify!($opcode));
                    println!("{:?}", summary);
                }
            }
        }
    };
}

// =============================================================================
// E2E test macro for preprocessed components
// =============================================================================

/// E2E test macro for preprocessed components (multiplicity tables).
///
/// Tests a preprocessed component by:
/// 1. Running a guest binary that exercises an opcode
/// 2. Getting the opcode's witness trace
/// 3. Calling the opcode's register_multiplicities to populate counters
/// 4. Converting counters to preprocessed trace
/// 5. Testing the preprocessed component's AIR constraints
///
/// # Arguments
/// - `$opcode_component`: The opcode component module (e.g., `base_alu_reg`)
/// - `$preprocessed`: The preprocessed component to test (e.g., `bitwise`)
/// - `$opcode`: The guest binary name (e.g., `and`)
///
/// # Usage
/// ```ignore
/// test_preprocessed_e2e!(base_alu_reg, bitwise, and);
/// test_preprocessed_e2e!(base_alu_imm, range_check_8_8, addi);
/// ```
#[macro_export]
macro_rules! test_preprocessed_e2e {
    ($opcode_component:ident, $preprocessed:ident, $opcode:ident) => {
        paste::paste! {
            #[test]
            fn [<test_ $preprocessed _via_ $opcode _e2e>]() {
                use stwo::core::pcs::TreeVec;
                use stwo::core::poly::circle::CanonicCoset;
                use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};

                use $crate::components::opcodes::$opcode_component::witness as opcode_witness;
                use $crate::components::preprocessed::$preprocessed::{air, witness};
                use $crate::preprocessed::PreprocessedTable;

                // Run guest binary and get the opcode trace
                let tracer = $crate::e2e::run_test_bin(stringify!($opcode));

                assert!(
                    !tracer.$opcode_component.is_empty(),
                    concat!("Expected ", stringify!($opcode), " trace entries in ", stringify!($opcode_component), ", got none.")
                );

                // Convert to witness trace
                let opcode_trace = tracer.$opcode_component.into_witness();

                // Register multiplicities for this opcode into counters
                let mut counters = $crate::relations::Counters::new();
                opcode_witness::register_multiplicities(opcode_trace.as_slice(), &mut counters);

                // Convert counters to preprocessed multiplicity trace
                let multiplicity_trace = counters.$preprocessed.into_trace();

                assert!(
                    !multiplicity_trace.is_empty(),
                    concat!(
                        "Expected preprocessed trace for ", stringify!($preprocessed),
                        " when running ", stringify!($opcode), ", got empty trace."
                    )
                );

                // Get the constant preprocessed columns for this table
                let preprocessed_columns = $crate::preprocessed::$preprocessed::Table::gen_columns();

                // The preprocessed AIR has LOG_SIZE from the table, not from the multiplicity trace
                let log_size = $crate::preprocessed::$preprocessed::Table::LOG_SIZE;

                let relations = $crate::relations::Relations::dummy();
                let (interaction_trace, claimed_sum) =
                    witness::gen_interaction_trace(&multiplicity_trace, &relations);

                let traces = TreeVec::new(vec![
                    preprocessed_columns.clone(),  // Tree 0: constant lookup table columns
                    multiplicity_trace.clone(),  // Tree 1: multiplicity trace
                    interaction_trace,  // Tree 2: interaction trace
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

                // Track and print relation summary for debugging LogUp imbalances
                #[cfg(feature = "track-relations")]
                {
                    use stwo_constraint_framework::{FrameworkComponent, TraceLocationAllocator};
                    use stwo_constraint_framework::relation_tracker::{
                        add_to_relation_entries, RelationSummary,
                    };

                    let mut allocator = TraceLocationAllocator::default();
                    let component = FrameworkComponent::new(&mut allocator, eval, claimed_sum);

                    // Convert trace to the format expected by add_to_relation_entries
                    let trace_values: TreeVec<Vec<Vec<stwo::core::fields::m31::BaseField>>> = TreeVec::new(vec![
                        preprocessed_columns.iter().map(|col| col.to_cpu().values).collect(),
                        multiplicity_trace.iter().map(|col| col.to_cpu().values).collect(),
                    ]);
                    let trace_refs = trace_values.as_cols_ref();

                    let entries = add_to_relation_entries(&component, &trace_refs);
                    let summary = RelationSummary::summarize_relations(&entries).cleaned();

                    println!("\n=== Relation Summary for {} via {} ===", stringify!($preprocessed), stringify!($opcode));
                    println!("{:?}", summary);
                }
            }
        }
    };
}
