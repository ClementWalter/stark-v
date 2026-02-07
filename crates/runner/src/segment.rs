//! Execution segmentation for bounded memory usage and parallel proving.
//!
//! This module implements the segmentation infrastructure described in
//! docs/continuations.md. It allows splitting long-running programs into
//! multiple segments, each with its own proof.

use crate::{Cpu, Memory, Tracer};
use std::collections::BTreeMap;
use thiserror::Error;

/// Complete VM state at an instruction boundary.
#[derive(Clone, Debug)]
pub struct VmState {
    /// Program counter (4-byte aligned).
    pub pc: u32,

    /// General-purpose registers x0-x31.
    pub registers: [u32; 32],

    /// Current cycle/clock value.
    pub clock: u32,

    /// Memory state snapshot.
    pub memory: MemorySnapshot,

    /// Last access clock per register (for continuity of memory model).
    pub reg_last_clk: [u32; 32],

    /// Last access clock per memory address (sparse map).
    pub mem_last_clk: BTreeMap<u32, u32>,
}

/// Memory snapshot using sparse page-based representation.
#[derive(Clone, Debug)]
pub struct MemorySnapshot {
    /// Merkle root of memory state at this point.
    pub root: u32,

    /// Dirty pages since last snapshot (for efficient delta encoding).
    /// Maps 4KB page index to page contents.
    pub dirty_pages: BTreeMap<u32, PageData>,

    /// Full memory state (used for first segment or after threshold).
    /// Only one of `dirty_pages` or `full_state` is populated.
    pub full_state: Option<BTreeMap<u32, u8>>,
}

/// 4KB page of memory.
#[derive(Clone, Debug)]
pub struct PageData {
    /// Page contents (4096 bytes).
    pub data: [u8; 4096],

    /// Merkle root of this page.
    pub root: u32,
}

/// Configuration for segment boundaries.
#[derive(Clone, Debug)]
pub struct SegmentConfig {
    /// Maximum cycles per segment.
    pub max_cycles_per_segment: u64,

    /// Strategy for determining segment boundaries.
    pub boundary_strategy: BoundaryStrategy,

    /// Maximum memory delta before forcing full snapshot.
    pub max_memory_delta_bytes: usize,
}

impl Default for SegmentConfig {
    fn default() -> Self {
        Self {
            max_cycles_per_segment: 1_000_000,
            boundary_strategy: BoundaryStrategy::FixedCycles,
            max_memory_delta_bytes: 1_024_000, // 1MB
        }
    }
}

/// Strategy for determining segment boundaries.
#[derive(Clone, Debug)]
pub enum BoundaryStrategy {
    /// Split at exact cycle count boundaries.
    FixedCycles,

    /// Split at function boundaries (requires DWARF info).
    FunctionBoundary,

    /// Split at branch points (slightly more flexible).
    BranchPoint,
}

/// Execution result for a single segment.
#[derive(Debug)]
pub struct SegmentExecution {
    /// Segment index (0-based).
    pub segment_id: u32,

    /// Initial VM state.
    pub initial_state: VmState,

    /// Final VM state.
    pub final_state: VmState,

    /// Execution trace for this segment.
    pub tracer: Tracer,

    /// Cycles executed in this segment.
    pub cycles: u64,
}

/// Commitment to complete VM state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateCommitment {
    /// Hash of (PC, registers, clock).
    pub register_state_hash: u32,

    /// Merkle root of memory.
    pub memory_root: u32,

    /// Hash of last-access clocks (for memory model continuity).
    pub clock_state_hash: u32,
}

/// Error types for segmentation operations.
#[derive(Debug, Error)]
pub enum SegmentationError {
    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error("Snapshot error: {0}")]
    SnapshotError(String),

    #[error("Restore error: {0}")]
    RestoreError(String),
}

/// Capture VM state at current execution point.
pub fn capture_state(cpu: &Cpu, mem: &Memory, tracer: &Tracer) -> VmState {
    VmState {
        pc: cpu.pc,
        registers: cpu.regs(),
        clock: tracer.clk,
        memory: capture_memory_snapshot(mem, tracer),
        reg_last_clk: tracer.reg_clk,
        mem_last_clk: tracer.mem_clk.clone(),
    }
}

/// Capture memory snapshot (with delta encoding).
fn capture_memory_snapshot(mem: &Memory, _tracer: &Tracer) -> MemorySnapshot {
    // For now, capture full memory state
    // TODO: Implement delta encoding for efficiency
    let mut full_state = BTreeMap::new();
    for addr in mem.keys() {
        full_state.insert(addr, mem.read_u8(addr));
    }

    MemorySnapshot {
        root: 0, // TODO: Compute Merkle root
        dirty_pages: BTreeMap::new(),
        full_state: Some(full_state),
    }
}

/// Restore VM state from a snapshot.
pub fn restore_state(
    state: &VmState,
    cpu: &mut Cpu,
    mem: &mut Memory,
    tracer: &mut Tracer,
) -> Result<(), SegmentationError> {
    // Restore CPU state
    cpu.pc = state.pc;
    for (idx, &val) in state.registers.iter().enumerate() {
        cpu.set_reg(idx as u8, val);
    }

    // Restore tracer state
    tracer.clk = state.clock;
    tracer.reg_clk = state.reg_last_clk;
    tracer.mem_clk = state.mem_last_clk.clone();

    // Restore memory state
    restore_memory(mem, &state.memory)?;

    Ok(())
}

/// Restore memory from snapshot.
fn restore_memory(mem: &mut Memory, snapshot: &MemorySnapshot) -> Result<(), SegmentationError> {
    if let Some(ref full_state) = snapshot.full_state {
        for (&addr, &byte) in full_state {
            mem.write_u8(addr, byte);
        }
        Ok(())
    } else {
        // TODO: Implement dirty page restoration
        Err(SegmentationError::RestoreError(
            "Dirty page restoration not yet implemented".to_string(),
        ))
    }
}

/// Check if current execution state is a valid segment boundary.
pub fn is_valid_boundary(cpu: &Cpu, cycles: u64, config: &SegmentConfig) -> bool {
    match config.boundary_strategy {
        BoundaryStrategy::FixedCycles => cycles % config.max_cycles_per_segment == 0,

        BoundaryStrategy::FunctionBoundary => {
            // Requires DWARF info to identify function entry points
            // For now, fallback to cycle-based boundaries
            cycles % config.max_cycles_per_segment == 0
        }

        BoundaryStrategy::BranchPoint => {
            // More flexible: split after any potential branch target
            // PC alignment check: valid instruction boundaries are 4-byte aligned
            cpu.pc % 4 == 0 && cycles % config.max_cycles_per_segment == 0
        }
    }
}

/// Compute state commitment from VM state.
pub fn compute_state_commitment(state: &VmState) -> StateCommitment {
    use crate::poseidon2::poseidon2_hash;

    // Hash register state: PC + all registers + clock
    let mut reg_data = Vec::with_capacity(34);
    reg_data.push(state.pc);
    reg_data.extend_from_slice(&state.registers);
    reg_data.push(state.clock);
    let register_state_hash = poseidon2_hash(&reg_data);

    // Hash clock state: all register last clocks + memory last clocks
    let mut clock_data = Vec::with_capacity(32 + state.mem_last_clk.len() * 2);
    clock_data.extend_from_slice(&state.reg_last_clk);
    for (&addr, &clk) in &state.mem_last_clk {
        clock_data.push(addr);
        clock_data.push(clk);
    }
    let clock_state_hash = poseidon2_hash(&clock_data);

    StateCommitment {
        register_state_hash,
        memory_root: state.memory.root,
        clock_state_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_config_default() {
        let config = SegmentConfig::default();
        assert_eq!(config.max_cycles_per_segment, 1_000_000);
        assert!(matches!(
            config.boundary_strategy,
            BoundaryStrategy::FixedCycles
        ));
    }

    #[test]
    fn test_is_valid_boundary_fixed_cycles() {
        let cpu = Cpu::new(0x1000, 0, 0);
        let config = SegmentConfig::default();

        assert!(is_valid_boundary(&cpu, 0, &config));
        assert!(!is_valid_boundary(&cpu, 500_000, &config));
        assert!(is_valid_boundary(&cpu, 1_000_000, &config));
        assert!(is_valid_boundary(&cpu, 2_000_000, &config));
    }

    #[test]
    fn test_capture_and_restore_state() {
        let mut cpu = Cpu::new(0x1000, 0x2000, 0x3000);
        cpu.set_reg(5, 0x42);
        let mut mem = Memory::new();
        mem.write_u32(0x8000, 0xDEADBEEF);
        let mut tracer = Tracer::default();
        tracer.clk = 100;
        tracer.reg_clk[5] = 50;

        // Capture state
        let state = capture_state(&cpu, &mem, &tracer);

        // Modify VM
        cpu.pc = 0x2000;
        cpu.set_reg(5, 0x99);
        mem.write_u32(0x8000, 0x12345678);
        tracer.clk = 200;

        // Restore state
        restore_state(&state, &mut cpu, &mut mem, &mut tracer).unwrap();

        // Verify restoration
        assert_eq!(cpu.pc, 0x1000);
        assert_eq!(cpu.reg(5), 0x42);
        assert_eq!(mem.read_u32(0x8000), 0xDEADBEEF);
        assert_eq!(tracer.clk, 100);
        assert_eq!(tracer.reg_clk[5], 50);
    }

    #[test]
    fn test_state_commitment_deterministic() {
        let cpu = Cpu::new(0x1000, 0, 0);
        let mem = Memory::new();
        let tracer = Tracer::default();

        let state1 = capture_state(&cpu, &mem, &tracer);
        let state2 = capture_state(&cpu, &mem, &tracer);

        let commitment1 = compute_state_commitment(&state1);
        let commitment2 = compute_state_commitment(&state2);

        assert_eq!(commitment1, commitment2);
    }
}

// Integration tests
#[cfg(test)]
#[path = "segment_tests.rs"]
mod segment_tests;
