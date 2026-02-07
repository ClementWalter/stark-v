//! Integration tests for segmentation functionality.

#[cfg(test)]
mod tests {
    use crate::segment::*;
    use crate::{BoundaryStrategy, SegmentConfig};

    // Note: Removed test_single_segment_short_program because fib example doesn't exist yet

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

    #[test]
    fn test_memory_snapshot_with_merkle_root() {
        use crate::{Cpu, Memory, Tracer};

        let mut mem = Memory::new();
        // Write some data to memory
        mem.write_u32(0x1000, 0xDEADBEEF);
        mem.write_u32(0x2000, 0xCAFEBABE);
        mem.write_u32(0x3000, 0x12345678);

        let cpu = Cpu::new(0x1000, 0, 0);
        let tracer = Tracer::default();

        let state = capture_state(&cpu, &mem, &tracer);

        // Verify that Merkle root is computed (non-zero)
        assert_ne!(
            state.memory.root, 0,
            "Merkle root should be computed (non-zero)"
        );

        // Verify determinism - same memory should produce same root
        let state2 = capture_state(&cpu, &mem, &tracer);
        assert_eq!(
            state.memory.root, state2.memory.root,
            "Merkle root should be deterministic"
        );

        // Verify that different memory produces different root
        mem.write_u32(0x4000, 0xABCDEF00);
        let state3 = capture_state(&cpu, &mem, &tracer);
        assert_ne!(
            state.memory.root, state3.memory.root,
            "Different memory should produce different Merkle root"
        );
    }

    #[test]
    fn test_dirty_page_restoration() {
        use crate::{Cpu, Memory, Tracer};

        let mut cpu = Cpu::new(0x1000, 0x2000, 0x3000);
        let mut mem = Memory::new();
        let mut tracer = Tracer::default();

        // Write data across multiple pages
        mem.write_u32(0x0000, 0x11111111); // Page 0
        mem.write_u32(0x1000, 0x22222222); // Page 1
        mem.write_u32(0x2000, 0x33333333); // Page 2
        mem.write_u32(0x3000, 0x44444444); // Page 3

        // Capture state
        let state = capture_state(&cpu, &mem, &tracer);

        // Clear memory
        let mut new_mem = Memory::new();

        // Restore from snapshot
        restore_state(&state, &mut cpu, &mut new_mem, &mut tracer).unwrap();

        // Verify all data is restored
        assert_eq!(new_mem.read_u32(0x0000), 0x11111111);
        assert_eq!(new_mem.read_u32(0x1000), 0x22222222);
        assert_eq!(new_mem.read_u32(0x2000), 0x33333333);
        assert_eq!(new_mem.read_u32(0x3000), 0x44444444);
    }

    #[test]
    fn test_page_based_memory_snapshot() {
        use crate::{Memory, Tracer};

        let mut mem = Memory::new();
        let tracer = Tracer::default();

        // Write data to different pages
        mem.write_u8(0x0000, 0xAA); // Page 0, offset 0
        mem.write_u8(0x0FFF, 0xBB); // Page 0, offset 4095
        mem.write_u8(0x1000, 0xCC); // Page 1, offset 0
        mem.write_u8(0x5000, 0xDD); // Page 5, offset 0

        let snapshot = capture_memory_snapshot(&mem, &tracer);

        // Verify pages are captured
        assert!(!snapshot.dirty_pages.is_empty(), "Should have dirty pages");
        assert_eq!(
            snapshot.dirty_pages.len(),
            3,
            "Should have 3 pages (0, 1, and 5)"
        );

        // Verify page 0 contains correct data
        if let Some(page0) = snapshot.dirty_pages.get(&0) {
            assert_eq!(page0.data[0], 0xAA);
            assert_eq!(page0.data[4095], 0xBB);
            assert_ne!(page0.root, 0, "Page root should be computed");
        } else {
            panic!("Page 0 should exist in dirty_pages");
        }

        // Verify page 1 contains correct data
        if let Some(page1) = snapshot.dirty_pages.get(&1) {
            assert_eq!(page1.data[0], 0xCC);
            assert_ne!(page1.root, 0, "Page root should be computed");
        } else {
            panic!("Page 1 should exist in dirty_pages");
        }
    }

    #[test]
    fn test_empty_memory_snapshot() {
        use crate::{Memory, Tracer};

        let mem = Memory::new();
        let tracer = Tracer::default();

        let snapshot = capture_memory_snapshot(&mem, &tracer);

        // Empty memory should have root = 0
        assert_eq!(snapshot.root, 0, "Empty memory should have root = 0");
        assert!(
            snapshot.dirty_pages.is_empty(),
            "Empty memory should have no dirty pages"
        );
    }
}
