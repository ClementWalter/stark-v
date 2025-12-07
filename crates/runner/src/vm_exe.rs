use std::collections::BTreeMap;
use std::path::Path;

use crate::elf::Elf;
use crate::error::Result;
use crate::program::Program;
use crate::transpiler;

pub type SparseMemoryImage = BTreeMap<(u32, u32), u8>;

const MAX_GUEST_MEMORY: u32 = 1 << 29;

#[derive(Clone, Debug)]
pub struct VmExe {
    pub program: Program,
    pub pc_start: u32,
    pub init_memory: SparseMemoryImage,
}

impl VmExe {
    pub fn new(program: Program, pc_start: u32, init_memory: SparseMemoryImage) -> Self {
        let res = Self {
            program,
            pc_start,
            init_memory,
        };
        tracing::debug!("VmExe: {:#?}", res);
        res
    }

    /// Load a VmExe from an ELF file at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the ELF file
    ///
    /// # Returns
    ///
    /// Returns a Result containing the VmExe or an error if the ELF file
    /// could not be loaded or transpiled.
    pub fn from_path(path: &Path) -> Result<Self> {
        let elf = Elf::from_path(path, MAX_GUEST_MEMORY)?;
        transpiler::transpile_elf(elf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{Instruction, VmOpcode};
    use std::io::Write;

    #[test]
    fn test_vm_exe_new() {
        let program = Program::from_instructions(vec![], 0x1000);
        let init_memory = SparseMemoryImage::new();
        let vm_exe = VmExe::new(program, 0x2000, init_memory);

        assert_eq!(vm_exe.pc_start, 0x2000);
        assert!(vm_exe.program.is_empty());
        assert!(vm_exe.init_memory.is_empty());
    }

    #[test]
    fn test_vm_exe_new_with_data() {
        let inst = Instruction::new(VmOpcode(0x200), 1, 2, 3, 4, 5, 6, 7);
        let program = Program::from_instructions(vec![inst], 0x1000);

        let mut init_memory = SparseMemoryImage::new();
        init_memory.insert((2, 0x1000), 0xAB);
        init_memory.insert((2, 0x1001), 0xCD);

        let vm_exe = VmExe::new(program, 0x1000, init_memory);

        assert_eq!(vm_exe.pc_start, 0x1000);
        assert_eq!(vm_exe.program.len(), 1);
        assert_eq!(vm_exe.init_memory.len(), 2);
    }

    #[test]
    fn test_vm_exe_clone() {
        let inst = Instruction::new(VmOpcode(0x200), 1, 2, 3, 4, 5, 6, 7);
        let program = Program::from_instructions(vec![inst], 0x1000);
        let mut init_memory = SparseMemoryImage::new();
        init_memory.insert((2, 0x1000), 0xAB);

        let vm_exe1 = VmExe::new(program, 0x1000, init_memory);
        let vm_exe2 = vm_exe1.clone();

        assert_eq!(vm_exe1.pc_start, vm_exe2.pc_start);
        assert_eq!(vm_exe1.program.len(), vm_exe2.program.len());
        assert_eq!(vm_exe1.init_memory.len(), vm_exe2.init_memory.len());
    }

    #[test]
    fn test_vm_exe_debug() {
        let program = Program::from_instructions(vec![], 0x1000);
        let init_memory = SparseMemoryImage::new();
        let vm_exe = VmExe::new(program, 0x1000, init_memory);

        let debug_str = format!("{:?}", vm_exe);
        assert!(debug_str.contains("VmExe"));
        assert!(debug_str.contains("pc_start"));
        assert!(debug_str.contains("program"));
        assert!(debug_str.contains("init_memory"));
    }

    #[test]
    fn test_vm_exe_from_path_nonexistent() {
        let path = std::path::Path::new("/nonexistent/path/to/elf");
        let result = VmExe::from_path(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_vm_exe_from_path_valid_elf() {
        let temp_dir = tempfile::tempdir().unwrap();
        let elf_path = temp_dir.path().join("test.elf");

        // Create a minimal valid ELF file
        let bytes = create_minimal_elf_with_nop();
        let mut file = std::fs::File::create(&elf_path).unwrap();
        file.write_all(&bytes).unwrap();

        let result = VmExe::from_path(&elf_path);
        assert!(result.is_ok());
        let vm_exe = result.unwrap();
        assert!(!vm_exe.program.is_empty());
    }

    // Helper to create minimal ELF with NOP instruction
    fn create_minimal_elf_with_nop() -> Vec<u8> {
        let mut bytes = vec![0u8; 52]; // ELF32 header

        // ELF magic
        bytes[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        bytes[4] = 1; // ELFCLASS32
        bytes[5] = 1; // ELFDATA2LSB
        bytes[6] = 1; // version

        // ELF type (ET_EXEC = 2)
        bytes[16..18].copy_from_slice(&2u16.to_le_bytes());
        // Machine type (EM_RISCV = 243)
        bytes[18..20].copy_from_slice(&243u16.to_le_bytes());
        // ELF version
        bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
        // Entry point
        bytes[24..28].copy_from_slice(&0x1000u32.to_le_bytes());
        // Program header offset
        bytes[28..32].copy_from_slice(&52u32.to_le_bytes());
        // ELF header size
        bytes[40..42].copy_from_slice(&52u16.to_le_bytes());
        // Program header entry size
        bytes[42..44].copy_from_slice(&32u16.to_le_bytes());
        // Number of program headers
        bytes[44..46].copy_from_slice(&1u16.to_le_bytes());
        // Section header entry size
        bytes[46..48].copy_from_slice(&40u16.to_le_bytes());

        // Add program header (PT_LOAD = 1, PF_X = 1)
        let ph_start = bytes.len();
        bytes.resize(ph_start + 32, 0);
        bytes[ph_start..ph_start + 4].copy_from_slice(&1u32.to_le_bytes()); // p_type = PT_LOAD
        bytes[ph_start + 4..ph_start + 8].copy_from_slice(&84u32.to_le_bytes()); // p_offset
        bytes[ph_start + 8..ph_start + 12].copy_from_slice(&0x1000u32.to_le_bytes()); // p_vaddr
        bytes[ph_start + 12..ph_start + 16].copy_from_slice(&0x1000u32.to_le_bytes()); // p_paddr
        bytes[ph_start + 16..ph_start + 20].copy_from_slice(&4u32.to_le_bytes()); // p_filesz
        bytes[ph_start + 20..ph_start + 24].copy_from_slice(&4u32.to_le_bytes()); // p_memsz
        bytes[ph_start + 24..ph_start + 28].copy_from_slice(&1u32.to_le_bytes()); // p_flags = PF_X
        bytes[ph_start + 28..ph_start + 32].copy_from_slice(&4u32.to_le_bytes()); // p_align

        // Add NOP instruction (ADDI x0, x0, 0)
        bytes.extend_from_slice(&0x00000013u32.to_le_bytes());

        bytes
    }

    #[test]
    fn test_sparse_memory_image_type() {
        let mut mem: SparseMemoryImage = BTreeMap::new();
        mem.insert((2, 0x1000), 0xFF);
        mem.insert((2, 0x1001), 0xAB);

        assert_eq!(mem.get(&(2, 0x1000)), Some(&0xFF));
        assert_eq!(mem.get(&(2, 0x1001)), Some(&0xAB));
        assert_eq!(mem.get(&(2, 0x1002)), None);
    }
}
