mod elf;
mod error;
mod instruction;
mod program;
mod transpiler;
mod vm_exe;

pub use error::{Result, RunnerError};
pub use instruction::{DebugInfo, Instruction};
pub use program::Program;
pub use vm_exe::{SparseMemoryImage, VmExe};

use std::path::Path;

/// Convenience function to load a VmExe from an ELF file.
///
/// This function is a wrapper around `VmExe::from_path` for backward compatibility.
pub fn load_vm_exe_from_elf(path: &Path) -> Result<VmExe> {
    VmExe::from_path(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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
    fn test_load_vm_exe_from_elf() {
        let temp_dir = tempfile::tempdir().unwrap();
        let elf_path = temp_dir.path().join("test.elf");

        let bytes = create_minimal_elf_with_nop();
        let mut file = std::fs::File::create(&elf_path).unwrap();
        file.write_all(&bytes).unwrap();

        let result = load_vm_exe_from_elf(&elf_path);
        assert!(result.is_ok());
        let exe = result.unwrap();
        assert!(!exe.program.is_empty());
        assert!(!exe.init_memory.is_empty());
    }

    #[test]
    fn test_load_vm_exe_from_elf_nonexistent() {
        let path = std::path::Path::new("/nonexistent/path/to/elf");
        let result = load_vm_exe_from_elf(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_public_exports() {
        // Verify all public types are accessible
        let _: Result<()> = Ok(());
        let _err = RunnerError::VaddrOverflow;
        let _di = DebugInfo::default();
        let _inst = Instruction::default();
        let _prog = Program::default();
        let _mem: SparseMemoryImage = std::collections::BTreeMap::new();
    }
}
