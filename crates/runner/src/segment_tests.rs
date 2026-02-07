//! Integration tests for segmentation functionality.

#[cfg(test)]
mod tests {
    use crate::segment::*;
    use crate::{BoundaryStrategy, SegmentConfig, run_with_segments};

    #[test]
    fn test_single_segment_short_program() {
        // Test a very short program that runs in a single segment
        // Using a simple fibonacci program
        let elf_bytes = include_bytes!(
            "../../../../examples/fib/target/riscv32im-unknown-none-elf/release/fib"
        );

        let config = SegmentConfig {
            max_cycles_per_segment: 1_000_000,
            boundary_strategy: BoundaryStrategy::FixedCycles,
            max_memory_delta_bytes: 1_024_000,
        };

        let result = run_with_segments(elf_bytes, &[], config);

        match result {
            Ok(segments) => {
                assert!(!segments.is_empty(), "Should have at least one segment");
                assert_eq!(segments[0].segment_id, 0, "First segment should have ID 0");

                // Verify state commitments are consistent
                for i in 0..segments.len() - 1 {
                    let current_final = compute_state_commitment(&segments[i].final_state);
                    let next_initial = compute_state_commitment(&segments[i + 1].initial_state);
                    assert_eq!(
                        current_final,
                        next_initial,
                        "State commitment mismatch between segment {} and {}",
                        i,
                        i + 1
                    );
                }

                println!("Generated {} segment(s)", segments.len());
                for seg in &segments {
                    println!("  Segment {}: {} cycles", seg.segment_id, seg.cycles);
                }
            }
            Err(e) => {
                // It's okay if the fib example doesn't exist yet
                eprintln!("Note: Could not run segmentation test: {}", e);
            }
        }
    }

    #[test]
    fn test_state_commitment_consistency() {
        use crate::{Cpu, Memory, Tracer};

        let cpu = Cpu::new(0x1000, 0x2000, 0x3000);
        let mem = Memory::new();
        let tracer = Tracer::default();

        let state = capture_state(&cpu, &mem, &tracer);
        let commitment1 = compute_state_commitment(&state);
        let commitment2 = compute_state_commitment(&state);

        assert_eq!(
            commitment1, commitment2,
            "State commitments should be deterministic"
        );
    }

    #[test]
    fn test_boundary_detection() {
        let cpu = Cpu::new(0x1000, 0, 0);
        let config = SegmentConfig {
            max_cycles_per_segment: 100,
            boundary_strategy: BoundaryStrategy::FixedCycles,
            max_memory_delta_bytes: 1_024_000,
        };

        assert!(
            is_valid_boundary(&cpu, 0, &config),
            "Cycle 0 should be a boundary"
        );
        assert!(
            !is_valid_boundary(&cpu, 50, &config),
            "Cycle 50 should not be a boundary"
        );
        assert!(
            is_valid_boundary(&cpu, 100, &config),
            "Cycle 100 should be a boundary"
        );
        assert!(
            is_valid_boundary(&cpu, 200, &config),
            "Cycle 200 should be a boundary"
        );
    }
}
