use rustc_hash::FxHashMap;
use thiserror::Error;

use crate::Memory;
use crate::ops::utils::M31_P;
use crate::poseidon2::{
    POSEIDON2_DEFAULT_HASHES_DEPTH_21, POSEIDON2_TRACE_COLUMNS, T, poseidon2_traced,
};
use crate::program::decode_program;
use crate::trace::{MemoryTable, MerkleTable, Poseidon2Table, ProgramTable, Tracer};

pub const RW_MEMORY_BASE: u32 = PROGRAM_BASE + PROGRAM_RANGE_SIZE;
pub const RW_TREE_LEAVES: u32 = 1 << 21;
pub const RW_TREE_HEIGHT: u32 = 22;

pub const PROGRAM_BASE: u32 = 0x0000_0400;
pub const PROGRAM_TREE_HEIGHT: u32 = 21;
pub const PROGRAM_RANGE_SIZE: u32 = 0x000F_FC00;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CommitmentError {
    #[error("Failed to decode instruction at PC=0x{pc:08x} (word=0x{word:08x})")]
    DecodeFailure { pc: u32, word: u32 },
    #[error("RW memory address out of range: 0x{addr:08x}")]
    RwAddressOutOfRange { addr: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MerkleValue {
    pub value: u32,
    pub multiplicity: u32,
}

impl MerkleValue {
    pub fn new(value: u32, multiplicity: u32) -> Self {
        Self {
            value,
            multiplicity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeData {
    pub index: u32,
    pub depth: u32,
    pub left: MerkleValue,
    pub right: MerkleValue,
    pub parent: MerkleValue,
}

pub fn leaf_index(base: u32, addr: u32, limb: u32) -> u32 {
    ((addr - base) / 4) * 4 + limb
}

fn default_hashes(leaf_depth: u32) -> &'static [u32] {
    let max_depth = (POSEIDON2_DEFAULT_HASHES_DEPTH_21.len() - 1) as u32;
    assert!(
        leaf_depth <= max_depth,
        "unsupported leaf depth {leaf_depth} (max {max_depth})"
    );
    let offset = (max_depth - leaf_depth) as usize;
    &POSEIDON2_DEFAULT_HASHES_DEPTH_21[offset..]
}

pub fn build_partial_merkle_tree(
    leaves: &FxHashMap<u32, MerkleValue>,
    leaf_depth: u32,
    poseidon2: &mut Poseidon2Table,
) -> (Vec<NodeData>, u32) {
    if leaves.is_empty() {
        let root = default_hashes(leaf_depth)[0];
        return (vec![], root);
    }

    let defaults = default_hashes(leaf_depth);
    let mut nodes = Vec::new();
    let mut current: FxHashMap<u32, MerkleValue> = leaves.clone();

    for depth in (1..=leaf_depth).rev() {
        let mut parent: FxHashMap<u32, MerkleValue> = FxHashMap::default();
        let mut indices: Vec<u32> = current.keys().copied().collect();
        indices.sort_unstable();
        let mut processed = std::collections::HashSet::new();

        for &index in &indices {
            if processed.contains(&index) {
                continue;
            }

            let sibling_index = index ^ 1;
            let (left_index, right_index) = if index.is_multiple_of(2) {
                (index, sibling_index)
            } else {
                (sibling_index, index)
            };

            let left = current
                .get(&left_index)
                .copied()
                .unwrap_or_else(|| MerkleValue::new(defaults[depth as usize], 0));
            let right = current
                .get(&right_index)
                .copied()
                .unwrap_or_else(|| MerkleValue::new(defaults[depth as usize], 0));

            let row = poseidon2_traced(left.value, right.value);
            poseidon2.push_row(&row);

            let parent_hash = row[POSEIDON2_TRACE_COLUMNS - T];
            let parent_value = MerkleValue::new(parent_hash, 1);

            nodes.push(NodeData {
                index: left_index,
                depth,
                left,
                right,
                parent: parent_value,
            });

            parent.insert(left_index >> 1, parent_value);
            processed.insert(left_index);
            processed.insert(right_index);
        }

        current = parent;
    }

    let root = current.get(&0).map(|v| v.value).unwrap_or(0);
    (nodes, root)
}

impl Tracer {
    pub fn finalize_commitments(&mut self, memory: &Memory) -> Result<(), CommitmentError> {
        let rw_range = RW_MEMORY_BASE..RW_MEMORY_BASE + RW_TREE_LEAVES;

        // Sanity check: all memory addresses should be in the RW memory range
        for &addr in self.mem_clk.keys() {
            if !rw_range.contains(&addr) {
                return Err(CommitmentError::RwAddressOutOfRange { addr });
            }
        }

        // Create trace tables
        self.program = ProgramTable::new();
        self.memory = MemoryTable::new();
        self.merkle = MerkleTable::new();
        self.poseidon2 = Poseidon2Table::new();

        // Create program leaves
        let program_rows = decode_program(memory)?;
        let mut program_leaves: FxHashMap<u32, MerkleValue> = FxHashMap::default();
        for row in &program_rows {
            let read_count = self.program_reads.get(&row.addr).copied().unwrap_or(0);
            for limb in 0..4u32 {
                let idx = leaf_index(PROGRAM_BASE, row.addr, limb);
                program_leaves.insert(idx, MerkleValue::new(row.values[limb as usize], read_count));
            }
        }

        // Create memory leaves
        let mut mem_entries: Vec<(u32, u32, u32, u32)> = Vec::new();
        let mut rw_initial_leaves: FxHashMap<u32, MerkleValue> = FxHashMap::default();
        let mut rw_final_leaves: FxHashMap<u32, MerkleValue> = FxHashMap::default();

        let mut mem_addrs: Vec<u32> = self
            .mem_initial
            .keys()
            .copied()
            .filter(|addr| rw_range.contains(addr))
            .collect();
        mem_addrs.sort_unstable();

        for addr in mem_addrs {
            let initial_word = self.mem_initial[&addr];
            let final_word = memory.read_u32(addr);

            let initial_bytes = initial_word.to_le_bytes();
            let final_bytes = final_word.to_le_bytes();
            let final_clk = self.mem_clk.get(&addr).copied().unwrap_or(0);

            mem_entries.push((addr, initial_word, final_word, final_clk));

            for limb in 0..4u32 {
                let idx = leaf_index(RW_MEMORY_BASE, addr, limb);
                rw_initial_leaves.insert(
                    idx,
                    MerkleValue::new(initial_bytes[limb as usize] as u32, 1),
                );
                rw_final_leaves.insert(idx, MerkleValue::new(final_bytes[limb as usize] as u32, 1));
            }
        }

        // Build Merkle trees and Poseidon2 trace
        let program_leaf_depth = PROGRAM_TREE_HEIGHT.saturating_sub(1);
        let rw_leaf_depth = RW_TREE_HEIGHT.saturating_sub(1);

        let (program_nodes, program_root) =
            build_partial_merkle_tree(&program_leaves, program_leaf_depth, &mut self.poseidon2);
        let (rw_initial_nodes, rw_initial_root) =
            build_partial_merkle_tree(&rw_initial_leaves, rw_leaf_depth, &mut self.poseidon2);
        let (rw_final_nodes, rw_final_root) =
            build_partial_merkle_tree(&rw_final_leaves, rw_leaf_depth, &mut self.poseidon2);

        // Create memory trace
        for (addr, initial_word, final_word, final_clk) in mem_entries {
            let initial_bytes = initial_word.to_le_bytes();
            let final_bytes = final_word.to_le_bytes();

            self.memory.push(
                addr,
                0,
                initial_bytes[0] as u32,
                initial_bytes[1] as u32,
                initial_bytes[2] as u32,
                initial_bytes[3] as u32,
                1,
                rw_final_root,
            );

            self.memory.push(
                addr,
                final_clk,
                final_bytes[0] as u32,
                final_bytes[1] as u32,
                final_bytes[2] as u32,
                final_bytes[3] as u32,
                M31_P - 1,
                rw_final_root,
            );
        }

        // Create program trace
        for row in &program_rows {
            let read_count = self.program_reads.get(&row.addr).copied().unwrap_or(0);
            self.program.push(
                row.addr,
                row.values[0],
                row.values[1],
                row.values[2],
                row.values[3],
                read_count,
                program_root,
            );
        }

        // Create Merkle tree trace
        let push_nodes = |nodes: Vec<NodeData>, root: u32, merkle: &mut MerkleTable| {
            for node in nodes {
                merkle.push(
                    node.index,
                    node.depth,
                    node.left.value,
                    node.right.value,
                    node.parent.value,
                    node.left.multiplicity,
                    node.right.multiplicity,
                    node.parent.multiplicity,
                    root,
                );
            }
        };

        push_nodes(rw_initial_nodes, rw_initial_root, &mut self.merkle);
        push_nodes(rw_final_nodes, rw_final_root, &mut self.merkle);
        push_nodes(program_nodes, program_root, &mut self.merkle);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Memory;
    use crate::commitment::{PROGRAM_BASE, RW_MEMORY_BASE};
    use crate::{InstCache, RunError, RunResult, decode, execute, io, load_elf};

    fn run_with_tracer_for_test(
        elf_bytes: &[u8],
        max_cycles: u64,
        mut tracer: Tracer,
    ) -> Result<RunResult, RunError> {
        let loaded = load_elf(elf_bytes)?;

        let mut cpu = crate::Cpu::new(loaded.entry, loaded.sp, loaded.gp);
        let mut mem = loaded.memory;
        let mut cache: InstCache = InstCache::default();

        loop {
            if mem.read_u32(loaded.halt_flag_addr) != 0 {
                let output = io::read_output(
                    &mem,
                    loaded.output_len_addr,
                    loaded.output_data_addr,
                    loaded.output_end_addr,
                );
                tracer.finalize_commitments(&mem)?;
                return Ok(RunResult {
                    cycles: tracer.clk as u64,
                    final_pc: cpu.pc,
                    output,
                    tracer,
                });
            }

            let prev_pc = cpu.pc;
            let inst = crate::decode::get_or_decode(&mut cache, &mem, cpu.pc)
                .ok_or(RunError::InvalidInstruction { pc: cpu.pc })?;
            tracer.trace_instr_access(cpu.pc);

            let is_self_loop = match inst.opcode {
                decode::Opcode::Jal if inst.rd == 0 && inst.imm == 0 => true,
                decode::Opcode::Jalr if inst.rd == 0 => {
                    let target = cpu.reg(inst.rs1).wrapping_add(inst.imm as u32) & !1;
                    target == cpu.pc
                }
                _ => false,
            };
            if is_self_loop {
                let output = io::read_output(
                    &mem,
                    loaded.output_len_addr,
                    loaded.output_data_addr,
                    loaded.output_end_addr,
                );
                tracer.finalize_commitments(&mem)?;
                return Ok(RunResult {
                    cycles: tracer.clk as u64,
                    final_pc: cpu.pc,
                    output,
                    tracer,
                });
            }

            tracer.clk += 1;
            execute(&mut cpu, &mut mem, &inst, &mut tracer);

            if cpu.pc == prev_pc {
                let output = io::read_output(
                    &mem,
                    loaded.output_len_addr,
                    loaded.output_data_addr,
                    loaded.output_end_addr,
                );
                tracer.finalize_commitments(&mem)?;
                return Ok(RunResult {
                    cycles: tracer.clk as u64,
                    final_pc: prev_pc,
                    output,
                    tracer,
                });
            }

            if tracer.clk as u64 > max_cycles {
                return Err(RunError::MaxCyclesExceeded {
                    cycles: tracer.clk as u64,
                    max: max_cycles,
                });
            }
        }
    }

    #[test]
    fn test_leaf_index_rw_and_program() {
        assert_eq!(leaf_index(RW_MEMORY_BASE, RW_MEMORY_BASE, 0), 0);
        assert_eq!(leaf_index(RW_MEMORY_BASE, RW_MEMORY_BASE, 3), 3);
        assert_eq!(leaf_index(RW_MEMORY_BASE, RW_MEMORY_BASE + 4, 0), 4);

        assert_eq!(leaf_index(PROGRAM_BASE, PROGRAM_BASE, 0), 0);
        assert_eq!(leaf_index(PROGRAM_BASE, PROGRAM_BASE + 4, 2), 6);
    }

    #[test]
    fn test_commitment_decode_failure() {
        let mut mem = Memory::new();
        mem.write_u32(PROGRAM_BASE, 0xFFFF_FFFF);
        let mut tracer = Tracer::default();
        let err = tracer.finalize_commitments(&mem).unwrap_err();
        assert_eq!(
            err,
            CommitmentError::DecodeFailure {
                pc: PROGRAM_BASE,
                word: 0xFFFF_FFFF
            }
        );
    }

    #[test]
    fn test_commitment_out_of_range_rw_access() {
        let mut tracer = Tracer::default();
        tracer.trace_mem_access(RW_MEMORY_BASE - 4, 0, 0);
        let err = tracer.finalize_commitments(&Memory::new()).unwrap_err();
        assert_eq!(
            err,
            CommitmentError::RwAddressOutOfRange {
                addr: RW_MEMORY_BASE - 4
            }
        );
    }

    #[test]
    fn test_commitment_traces_non_empty() {
        prover::e2e::ensure_guest_built();
        let elf_path = prover::e2e::guest_bin_dir().join("memory");
        let elf_bytes = std::fs::read(&elf_path)
            .unwrap_or_else(|e| panic!("Failed to read ELF {elf_path:?}: {e}"));

        let result =
            run_with_tracer_for_test(&elf_bytes, 10_000_000, Tracer::with_max_clock_diff(1))
                .expect("Run failed");
        let tracer = result.tracer;

        assert!(!tracer.program.is_empty());
        assert!(!tracer.merkle.is_empty());
        assert!(!tracer.poseidon2.is_empty());
        assert!(!tracer.mem_clk_update.is_empty());
    }
}
