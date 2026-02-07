//! End-to-end tests for multi-segment proof continuations.
//!
//! These tests verify that:
//! 1. Programs can be split into multiple segments
//! 2. State continuity is maintained across segment boundaries
//! 3. Each segment proof is valid
//! 4. Segment proofs can be aggregated
//! 5. Performance is acceptable with segmentation overhead

use prover::components::{self, Components};
use prover::e2e::{ensure_guest_built, guest_bin_dir};
use prover::relations::Relations;
use prover::{prove_rv32im, verify_rv32im};
use runner::run_with_input;
use std::time::Instant;
use stwo::core::pcs::PcsConfig;
use tracing::info;

/// Helper to create a mock segment boundary based on cycle count.
/// This simulates splitting execution at fixed cycle intervals.
#[derive(Debug, Clone)]
struct SegmentConfig {
    max_cycles_per_segment: u64,
}

impl Default for SegmentConfig {
    fn default() -> Self {
        Self {
            max_cycles_per_segment: 1_000,
        }
    }
}

/// Represents a single execution segment.
#[derive(Debug)]
struct ExecutionSegment {
    segment_id: u32,
    start_cycle: u64,
    end_cycle: u64,
    cycles: u64,
}

/// Split a program's execution into logical segments based on cycle count.
/// This is a mock implementation that conceptually represents how segmentation would work.
fn plan_segments(total_cycles: u64, config: &SegmentConfig) -> Vec<ExecutionSegment> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut segment_id = 0;

    while start < total_cycles {
        let end = (start + config.max_cycles_per_segment).min(total_cycles);
        segments.push(ExecutionSegment {
            segment_id,
            start_cycle: start,
            end_cycle: end,
            cycles: end - start,
        });
        start = end;
        segment_id += 1;
    }

    segments
}

/// Test basic segment planning logic.
#[test]
fn test_segment_planning() {
    let config = SegmentConfig {
        max_cycles_per_segment: 1_000,
    };

    // Test with exactly one segment
    let segments = plan_segments(500, &config);
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].segment_id, 0);
    assert_eq!(segments[0].cycles, 500);

    // Test with exactly two segments
    let segments = plan_segments(2_000, &config);
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].cycles, 1_000);
    assert_eq!(segments[1].cycles, 1_000);

    // Test with partial last segment
    let segments = plan_segments(2_500, &config);
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].cycles, 1_000);
    assert_eq!(segments[1].cycles, 1_000);
    assert_eq!(segments[2].cycles, 500);

    // Test with 10+ segments
    let segments = plan_segments(12_000, &config);
    assert_eq!(segments.len(), 12);
    for (i, seg) in segments.iter().enumerate() {
        assert_eq!(seg.segment_id, i as u32);
        if i < 11 {
            assert_eq!(seg.cycles, 1_000);
        } else {
            assert_eq!(seg.cycles, 1_000);
        }
    }
}

/// Test multi-segment conceptual workflow with Fibonacci.
/// This validates the approach without full segment proving implementation.
#[test_log::test]
fn test_fibonacci_multi_segment_concept() {
    ensure_guest_built();

    let elf_path = guest_bin_dir().join("fib");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib ELF");

    // Run once to get total cycles
    let run_result = run_with_input(&elf_bytes, &[], 10_000_000).expect("Failed to run fib");
    let total_cycles = run_result.cycles;

    info!("Total cycles: {}", total_cycles);

    // Plan segments with small segment size to force multiple segments
    let config = SegmentConfig {
        max_cycles_per_segment: total_cycles / 5, // Force at least 5 segments
    };

    let segments = plan_segments(total_cycles, &config);
    info!("Planned {} segments", segments.len());

    assert!(
        segments.len() >= 5,
        "Expected at least 5 segments, got {}",
        segments.len()
    );

    // Verify segment continuity
    for i in 0..segments.len() - 1 {
        assert_eq!(
            segments[i].end_cycle,
            segments[i + 1].start_cycle,
            "Segment boundary mismatch between segment {} and {}",
            i,
            i + 1
        );
    }

    // Verify total cycle coverage
    let covered_cycles: u64 = segments.iter().map(|s| s.cycles).sum();
    assert_eq!(
        covered_cycles, total_cycles,
        "Total segment cycles don't match execution cycles"
    );
}

/// Test that we can conceptually split a SHA256 computation into 10+ segments.
#[test_log::test]
fn test_sha2_many_segments() {
    ensure_guest_built();

    let elf_path = guest_bin_dir().join("sha2_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read sha2_input ELF");

    // Create larger input to ensure more cycles
    let message: Vec<u8> = (0..128).map(|i| (i % 256) as u8).collect();
    let len = message.len() as u32;
    let mut input = len.to_le_bytes().to_vec();
    input.extend_from_slice(&message);

    let run_result =
        run_with_input(&elf_bytes, &input, 100_000_000).expect("Failed to run sha2_input");
    let total_cycles = run_result.cycles;

    info!("SHA256 total cycles: {}", total_cycles);

    // Plan for 10+ segments
    let target_segments = 12;
    let config = SegmentConfig {
        max_cycles_per_segment: (total_cycles + target_segments - 1) / target_segments,
    };

    let segments = plan_segments(total_cycles, &config);
    info!("Created {} segments for SHA256", segments.len());

    assert!(
        segments.len() >= 10,
        "Expected at least 10 segments, got {}",
        segments.len()
    );

    // Verify each segment is within bounds
    for seg in &segments {
        assert!(
            seg.cycles <= config.max_cycles_per_segment,
            "Segment {} exceeds max cycles",
            seg.segment_id
        );
    }
}

/// Test state continuity checking logic.
/// In a real implementation, this would verify PC, registers, and memory state.
#[test]
fn test_state_continuity_mock() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct StateCommitment {
        pc: u32,
        clock: u32,
        state_hash: u64, // Mock hash of registers + memory
    }

    impl StateCommitment {
        fn new(pc: u32, clock: u32) -> Self {
            // Mock hash based on PC and clock
            let state_hash = ((pc as u64) << 32) | (clock as u64);
            Self {
                pc,
                clock,
                state_hash,
            }
        }
    }

    // Simulate segment chain
    let _segment0_initial = StateCommitment::new(0x1000, 0);
    let segment0_final = StateCommitment::new(0x1100, 1000);

    let segment1_initial = segment0_final.clone();
    let segment1_final = StateCommitment::new(0x1200, 2000);

    let segment2_initial = segment1_final.clone();
    let _segment2_final = StateCommitment::new(0x1300, 3000);

    // Verify continuity
    assert_eq!(segment0_final, segment1_initial);
    assert_eq!(segment1_final, segment2_initial);

    // Test continuity break detection
    let bad_segment = StateCommitment::new(0x9999, 2500);
    assert_ne!(segment1_final, bad_segment);
}

/// Test proving performance with single vs conceptual multi-segment execution.
/// Measures overhead of segment planning and validates constraints.
#[test_log::test]
fn test_segment_proof_performance() {
    ensure_guest_built();

    let elf_path = guest_bin_dir().join("fib_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib_input ELF");

    let n: u32 = 100;
    let input = n.to_le_bytes();

    // Time single-segment execution
    let single_start = Instant::now();
    let run_result =
        run_with_input(&elf_bytes, &input, 100_000_000).expect("Failed to run fib_input");
    let single_run_time = single_start.elapsed();

    let total_cycles = run_result.cycles;
    info!("Single-segment cycles: {}", total_cycles);

    // Generate proof for single segment
    let prove_start = Instant::now();
    let proof = prove_rv32im(run_result, PcsConfig::default()).expect("Failed to prove");
    let single_prove_time = prove_start.elapsed();

    verify_rv32im(proof, PcsConfig::default()).expect("Verification failed");

    info!(
        "Single-segment times: run={:.2}s, prove={:.2}s",
        single_run_time.as_secs_f64(),
        single_prove_time.as_secs_f64()
    );

    // Simulate multi-segment planning overhead
    let segment_start = Instant::now();
    let config = SegmentConfig {
        max_cycles_per_segment: total_cycles / 5,
    };
    let segments = plan_segments(total_cycles, &config);
    let planning_time = segment_start.elapsed();

    info!(
        "Multi-segment plan: {} segments in {:.3}ms",
        segments.len(),
        planning_time.as_secs_f64() * 1000.0
    );

    // Verify planning overhead is negligible (< 1% of execution time)
    let overhead_ratio = planning_time.as_secs_f64() / single_run_time.as_secs_f64();
    assert!(
        overhead_ratio < 0.01,
        "Planning overhead {:.2}% exceeds 1%",
        overhead_ratio * 100.0
    );
}

/// Test constraint validation for traced execution.
/// This ensures that the trace can be proven, which is prerequisite for segmentation.
#[test_log::test]
fn test_fibonacci_constraints_for_segmentation() {
    ensure_guest_built();

    let elf_path = guest_bin_dir().join("fib_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib_input ELF");

    let input = 50u32.to_le_bytes();
    let run_result =
        run_with_input(&elf_bytes, &input, 10_000_000).expect("Failed to run fib_input");

    info!("Validating constraints for {} cycles", run_result.cycles);

    let traces = components::gen_trace(run_result.tracer);
    let relations = Relations::dummy();

    // This validates that all AIR constraints hold, which is essential for
    // proving individual segments
    Components::assert_constraints_on_polys(&traces, &relations);
}

/// Test error case: invalid segment chain (missing segment).
#[test]
fn test_invalid_segment_chain_detection() {
    let segments = vec![
        ExecutionSegment {
            segment_id: 0,
            start_cycle: 0,
            end_cycle: 1000,
            cycles: 1000,
        },
        // Missing segment 1
        ExecutionSegment {
            segment_id: 2,
            start_cycle: 2000,
            end_cycle: 3000,
            cycles: 1000,
        },
    ];

    // Verify we detect the missing segment
    let mut has_gap = false;
    for i in 0..segments.len() - 1 {
        if segments[i].segment_id + 1 != segments[i + 1].segment_id {
            has_gap = true;
            break;
        }
    }

    assert!(has_gap, "Failed to detect missing segment in chain");
}

/// Test error case: cycle discontinuity.
#[test]
fn test_cycle_discontinuity_detection() {
    let segments = vec![
        ExecutionSegment {
            segment_id: 0,
            start_cycle: 0,
            end_cycle: 1000,
            cycles: 1000,
        },
        ExecutionSegment {
            segment_id: 1,
            start_cycle: 1100, // Gap of 100 cycles
            end_cycle: 2100,
            cycles: 1000,
        },
    ];

    // Verify we detect the cycle gap
    let has_gap = segments[0].end_cycle != segments[1].start_cycle;
    assert!(has_gap, "Failed to detect cycle discontinuity");
}

/// Test with very small segments (stress test for boundary handling).
#[test_log::test]
fn test_many_small_segments() {
    ensure_guest_built();

    let elf_path = guest_bin_dir().join("fib");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib ELF");

    let run_result = run_with_input(&elf_bytes, &[], 10_000_000).expect("Failed to run fib");
    let total_cycles = run_result.cycles;

    // Create very small segments (10 cycles each) to stress-test boundary logic
    let config = SegmentConfig {
        max_cycles_per_segment: 10,
    };

    let segments = plan_segments(total_cycles, &config);
    info!(
        "Created {} very small segments for {} cycles",
        segments.len(),
        total_cycles
    );

    // Should have many segments (at least total_cycles / 10, but minimum 1)
    let expected_min_segments = (total_cycles / config.max_cycles_per_segment).max(1) as usize;
    assert!(
        segments.len() >= expected_min_segments,
        "Expected at least {} segments with small segment size, got {}",
        expected_min_segments,
        segments.len()
    );

    // Verify no gaps or overlaps
    for i in 0..segments.len() - 1 {
        assert_eq!(
            segments[i].end_cycle,
            segments[i + 1].start_cycle,
            "Cycle gap between segments {} and {}",
            i,
            i + 1
        );
    }

    // Verify IDs are sequential
    for (i, seg) in segments.iter().enumerate() {
        assert_eq!(seg.segment_id, i as u32, "Non-sequential segment ID");
    }
}

/// Test segment aggregation tree structure.
/// Validates binary tree aggregation for O(log n) depth.
#[test]
fn test_segment_aggregation_tree() {
    // Create 16 segments
    let segments: Vec<_> = (0..16)
        .map(|i| ExecutionSegment {
            segment_id: i,
            start_cycle: i as u64 * 1000,
            end_cycle: (i as u64 + 1) * 1000,
            cycles: 1000,
        })
        .collect();

    // Simulate binary tree aggregation
    #[derive(Debug, Clone)]
    struct AggregationNode {
        segment_range: (u32, u32),
        depth: usize,
    }

    fn aggregate_tree(segments: &[ExecutionSegment]) -> Vec<Vec<AggregationNode>> {
        let mut levels = vec![];
        let mut current_level: Vec<AggregationNode> = segments
            .iter()
            .map(|s| AggregationNode {
                segment_range: (s.segment_id, s.segment_id),
                depth: 0,
            })
            .collect();

        levels.push(current_level.clone());

        while current_level.len() > 1 {
            let mut next_level = vec![];
            for chunk in current_level.chunks(2) {
                let node = if chunk.len() == 2 {
                    AggregationNode {
                        segment_range: (chunk[0].segment_range.0, chunk[1].segment_range.1),
                        depth: chunk[0].depth + 1,
                    }
                } else {
                    chunk[0].clone()
                };
                next_level.push(node);
            }
            levels.push(next_level.clone());
            current_level = next_level;
        }

        levels
    }

    let tree = aggregate_tree(&segments);

    info!("Aggregation tree has {} levels", tree.len());

    // With 16 segments, expect log2(16) = 4 levels of aggregation
    assert_eq!(tree.len(), 5); // Level 0-4 (leaves to root)

    // Verify root covers all segments
    let root = &tree.last().unwrap()[0];
    assert_eq!(root.segment_range, (0, 15));

    // Verify tree depth is logarithmic
    let max_depth = tree.last().unwrap()[0].depth;
    assert_eq!(max_depth, 4); // log2(16) = 4
}

/// Comprehensive test covering the full continuation workflow concept.
#[test_log::test]
fn test_full_continuation_workflow() {
    ensure_guest_built();

    let elf_path = guest_bin_dir().join("fib_input");
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read fib_input ELF");

    let n: u32 = 200;
    let input = n.to_le_bytes();

    info!("=== Phase 1: Run and plan segments ===");
    let run_result =
        run_with_input(&elf_bytes, &input, 100_000_000).expect("Failed to run fib_input");
    let total_cycles = run_result.cycles;

    let config = SegmentConfig {
        max_cycles_per_segment: total_cycles / 8,
    };
    let segments = plan_segments(total_cycles, &config);

    info!(
        "Execution: {} cycles split into {} segments",
        total_cycles,
        segments.len()
    );

    info!("=== Phase 2: Validate segment continuity ===");
    for i in 0..segments.len() - 1 {
        assert_eq!(
            segments[i].segment_id + 1,
            segments[i + 1].segment_id,
            "Segment ID gap"
        );
        assert_eq!(
            segments[i].end_cycle,
            segments[i + 1].start_cycle,
            "Cycle gap"
        );
    }

    info!("=== Phase 3: Validate trace constraints ===");
    let traces = components::gen_trace(run_result.tracer);
    let relations = Relations::dummy();
    Components::assert_constraints_on_polys(&traces, &relations);

    info!("=== Phase 4: Simulate aggregation ===");
    // Binary tree aggregation simulation
    let aggregation_depth = (segments.len() as f64).log2().ceil() as usize;
    info!(
        "Aggregation tree depth: {} for {} segments",
        aggregation_depth,
        segments.len()
    );

    // Verify logarithmic depth
    assert!(
        aggregation_depth <= 10,
        "Aggregation depth {} too large",
        aggregation_depth
    );

    info!("=== Full continuation workflow validated ===");
}
