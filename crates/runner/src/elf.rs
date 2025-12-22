use crate::Memory;
use goblin::elf::Elf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ElfError {
    #[error("Failed to parse ELF: {0}")]
    Parse(#[from] goblin::error::Error),
    #[error("Not a 32-bit ELF")]
    Not32Bit,
    #[error("Not a RISC-V ELF")]
    NotRiscV,
}

/// Loaded ELF with entry point and memory initialization.
pub struct LoadedElf {
    /// Entry point PC.
    pub entry: u32,
    /// Initial stack pointer.
    pub sp: u32,
    /// Global pointer.
    pub gp: u32,
    /// Memory initialized with loadable segments.
    pub memory: Memory,
}

/// Load an ELF file and return the entry point, initial registers, and memory.
pub fn load_elf(bytes: &[u8]) -> Result<LoadedElf, ElfError> {
    let elf = Elf::parse(bytes)?;

    // Verify it's a 32-bit RISC-V ELF
    if !elf.is_lib && elf.header.e_machine != goblin::elf::header::EM_RISCV {
        return Err(ElfError::NotRiscV);
    }

    let entry = elf.entry as u32;

    // Helper to find a symbol by name
    let find_symbol = |name: &str| -> Option<u32> {
        elf.syms
            .iter()
            .find(|s| elf.strtab.get_at(s.st_name).is_some_and(|n| n == name))
            .map(|s| s.st_value as u32)
    };

    // Find __global_pointer$ symbol
    let gp = find_symbol("__global_pointer$").unwrap_or(0x0020_0800);

    // Find __stack_top symbol
    let sp = find_symbol("__stack_top").unwrap_or(0x0020_0000);

    // Load all PT_LOAD segments into memory
    let mut mem_data = Vec::new();
    for ph in &elf.program_headers {
        if ph.p_type == goblin::elf::program_header::PT_LOAD {
            let vaddr = ph.p_vaddr as u32;
            let file_offset = ph.p_offset as usize;
            let file_size = ph.p_filesz as usize;
            let mem_size = ph.p_memsz as usize;

            // Copy file contents
            if file_size > 0 && file_offset + file_size <= bytes.len() {
                for (i, &byte) in bytes[file_offset..file_offset + file_size]
                    .iter()
                    .enumerate()
                {
                    mem_data.push((vaddr.wrapping_add(i as u32), byte));
                }
            }

            // Zero-fill BSS (memsz > filesz)
            for i in file_size..mem_size {
                mem_data.push((vaddr.wrapping_add(i as u32), 0));
            }
        }
    }

    let memory: Memory = mem_data.into_iter().collect();

    Ok(LoadedElf {
        entry,
        sp,
        gp,
        memory,
    })
}
