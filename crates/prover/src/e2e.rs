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
pub fn run_test_bin(name: &str) -> air::trace::Tracer {
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

                let log_size = trace.first()
                    .map(|t| t.domain.log_size())
                    .expect("Empty trace after gen_trace");

                let relations = $crate::relations::Relations::dummy();
                let (interaction_trace, claimed_sum) =
                    $crate::components::$component::witness::gen_interaction_trace(
                        trace.as_slice(),
                        &relations,
                    );

                let traces = TreeVec::new(vec![
                    vec![],
                    trace.clone(),
                    interaction_trace,
                ]);

                let trace_polys = traces.map_cols(|c| c.interpolate());

                let eval = $crate::components::$component::air::Eval {
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
                // Lookup multiplicity components expose the balancing side of preprocessed table relations.
                #[cfg(feature = "track-relations")]
                {
                    use stwo_constraint_framework::{FrameworkComponent, TraceLocationAllocator};
                    use stwo_constraint_framework::relation_tracker::{
                        add_to_relation_entries, RelationSummary, RelationTrackerEntry,
                    };
                    use $crate::preprocessed::PreprocessedTable;

                    // Counters produce the lookup multiplicity traces consumed by generated components.
                    let mut counters = $crate::relations::Counters::new();
                    witness::register_multiplicities(trace.as_slice(), &mut counters);

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

                    // Lookup entries are added separately because they are generated from counters.
                    macro_rules! add_lookup_entries {
                        ($lookup:ident) => {{
                            use $crate::components::lookups::$lookup::{air as lookup_air, witness as lookup_witness};

                            let multiplicity_trace = counters.$lookup.into_trace();
                            if !multiplicity_trace.is_empty() {
                                let lookup_log_size = $crate::preprocessed::$lookup::Table::LOG_SIZE;
                                let preprocessed_columns = $crate::preprocessed::$lookup::Table::gen_columns();

                                let (_lookup_interaction, lookup_claimed) =
                                    lookup_witness::gen_interaction_trace(&multiplicity_trace, &relations);

                                let lookup_eval = lookup_air::Eval {
                                    log_size: lookup_log_size,
                                    relations: relations.clone(),
                                };

                                let mut allocator = TraceLocationAllocator::default();
                                let component = FrameworkComponent::new(&mut allocator, lookup_eval, lookup_claimed);

                                let trace_values: TreeVec<Vec<Vec<stwo::core::fields::m31::BaseField>>> = TreeVec::new(vec![
                                    preprocessed_columns.iter().map(|col| col.to_cpu().values).collect(),
                                    multiplicity_trace.iter().map(|col| col.to_cpu().values).collect(),
                                ]);
                                let trace_refs = trace_values.as_cols_ref();

                                all_entries.extend(add_to_relation_entries(&component, &trace_refs));
                            }
                        }};
                    }

                    add_lookup_entries!(bitwise);
                    add_lookup_entries!(range_check_20);
                    add_lookup_entries!(range_check_8_8);
                    add_lookup_entries!(range_check_8_11);
                    add_lookup_entries!(range_check_8_8_4);
                    add_lookup_entries!(range_check_m31);

                    let summary = RelationSummary::summarize_relations(&all_entries).cleaned();

                    println!("\n=== Relation Summary for {} (with lookups) ===", stringify!($opcode));
                    println!("{:?}", summary);
                }
            }
        }
    };
}

// =============================================================================
// E2E test macro for lookup multiplicity components
// =============================================================================

/// E2E test macro for lookup multiplicity components.
///
/// Tests a lookup component by:
/// 1. Running a guest binary that exercises an opcode
/// 2. Getting the opcode's witness trace
/// 3. Calling the opcode's register_multiplicities to populate counters
/// 4. Converting counters to a lookup multiplicity trace
/// 5. Testing the lookup component's AIR constraints
///
/// # Arguments
/// - `$opcode_component`: The opcode component module (e.g., `base_alu_reg`)
/// - `$lookup`: The lookup component to test (e.g., `bitwise`)
/// - `$opcode`: The guest binary name (e.g., `and`)
///
/// # Usage
/// ```ignore
/// test_lookup_e2e!(base_alu_reg, bitwise, and);
/// test_lookup_e2e!(base_alu_imm, range_check_8_8, addi);
/// ```
#[macro_export]
macro_rules! test_lookup_e2e {
    ($opcode_component:ident, $lookup:ident, $opcode:ident) => {
        paste::paste! {
            #[test]
            fn [<test_ $lookup _via_ $opcode _e2e>]() {
                use stwo::core::pcs::TreeVec;
                use stwo::core::poly::circle::CanonicCoset;
                use stwo_constraint_framework::{FrameworkEval, assert_constraints_on_polys};

                use $crate::components::$opcode_component::witness as opcode_witness;
                use $crate::components::lookups::$lookup::{air, witness};
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
                let multiplicity_trace = counters.$lookup.into_trace();

                assert!(
                    !multiplicity_trace.is_empty(),
                    concat!(
                        "Expected lookup trace for ", stringify!($lookup),
                        " when running ", stringify!($opcode), ", got empty trace."
                    )
                );

                // Constant preprocessed columns define the lookup table checked by this component.
                let preprocessed_columns = $crate::preprocessed::$lookup::Table::gen_columns();

                // The lookup AIR domain matches the full constant table domain.
                let log_size = $crate::preprocessed::$lookup::Table::LOG_SIZE;

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

                    println!("\n=== Relation Summary for {} via {} ===", stringify!($lookup), stringify!($opcode));
                    println!("{:?}", summary);
                }
            }
        }
    };
}

// =============================================================================
// Segmented proving helpers
// =============================================================================
//
// A long execution is split by `runner::run_segments_with_input` into
// segments of bounded cycle count, each proven independently with the
// per-segment clock restarting at 0. Consecutive segments chain on their
// public data: the exit state of segment `k` — program counter, register
// file, and read-write memory Merkle root — must equal the entry state of
// segment `k + 1`, while the program root is common to all segments.
//
// In production the runner streams segments to a pool of provers and the
// chain checks are asserted in-AIR by the 2-to-1 aggregation (the
// `recursion` crate); these host-side helpers exist for tests and benches.

use stwo::core::channel::MerkleChannel;
use stwo::core::pcs::PcsConfig;
use stwo::core::vcs_lifted::blake2_merkle::{Blake2sMerkleChannel, Blake2sMerkleHasher};
use stwo::core::vcs_lifted::merkle_hasher::MerkleHasherLifted;
use stwo::prover::backend::simd::SimdBackend;

use crate::errors::VerificationError;
use crate::{Preprocessing, Proof, prove_rv32im_with_channel, verify_rv32im_with_channel};

/// Prove every segment of a segmented execution (Blake2s channel).
pub fn prove_segments(
    run_results: Vec<runner::RunResult>,
    config: PcsConfig,
    preprocessing: &Preprocessing,
) -> Vec<Proof<Blake2sMerkleHasher>> {
    prove_segments_with_channel::<Blake2sMerkleChannel>(run_results, config, preprocessing)
}

/// Prove every segment of a segmented execution with any Merkle channel —
/// in particular the Poseidon2-M31 channel whose hash the recursion AIR
/// proves.
///
/// Segments are independent statements, so they prove embarrassingly in
/// parallel: one single-threaded stwo prover per segment, up to the rayon
/// pool size (the number of cores). Per the fibonacci benchmark
/// (README "Parallelization Strategy"), this beats intra-proof rayon
/// parallelism in aggregate throughput — build WITHOUT the `parallel`
/// feature so each prover stays single-threaded.
pub fn prove_segments_with_channel<MC: MerkleChannel>(
    run_results: Vec<runner::RunResult>,
    config: PcsConfig,
    preprocessing: &Preprocessing<MC::H>,
) -> Vec<Proof<MC::H>>
where
    SimdBackend: stwo::prover::backend::BackendForChannel<MC>
        + stwo::prover::backend::ColumnOps<
            <MC::H as MerkleHasherLifted>::Hash,
            Column = Vec<<MC::H as MerkleHasherLifted>::Hash>,
        >,
    MC::H: Sync,
    <MC::H as MerkleHasherLifted>::Hash: Send + Sync,
{
    use rayon::iter::{IntoParallelIterator, ParallelIterator};

    run_results
        .into_par_iter()
        .map(|run_result| prove_rv32im_with_channel::<MC>(run_result, config, preprocessing))
        .collect()
}

/// Verify a chain of segment proofs (Blake2s channel).
pub fn verify_segments(
    proofs: Vec<Proof<Blake2sMerkleHasher>>,
    config: PcsConfig,
    preprocessing: &Preprocessing,
) -> Result<(), VerificationError> {
    verify_segments_with_channel::<Blake2sMerkleChannel>(proofs, config, preprocessing)
}

/// Verify a chain of segment proofs with any Merkle channel: each proof
/// individually, plus the boundary chaining between consecutive segments.
pub fn verify_segments_with_channel<MC: MerkleChannel>(
    proofs: Vec<Proof<MC::H>>,
    config: PcsConfig,
    preprocessing: &Preprocessing<MC::H>,
) -> Result<(), VerificationError> {
    for (index, pair) in proofs.windows(2).enumerate() {
        let (prev, next) = (&pair[0].public_data, &pair[1].public_data);
        let mismatch = |what| VerificationError::SegmentChainMismatch {
            prev: index,
            next: index + 1,
            what,
        };
        if prev.final_pc != next.initial_pc {
            return Err(mismatch("final_pc != initial_pc"));
        }
        if prev.final_regs != next.initial_regs {
            return Err(mismatch("final_regs != initial_regs"));
        }
        if prev.final_rw_root != next.initial_rw_root {
            return Err(mismatch("final_rw_root != initial_rw_root"));
        }
        if prev.program_root != next.program_root {
            return Err(mismatch("program_root differs"));
        }
    }

    for proof in proofs {
        verify_rv32im_with_channel::<MC>(proof, config, preprocessing)?;
    }
    Ok(())
}
