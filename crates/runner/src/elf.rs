use std::{cmp::min, collections::BTreeMap, fs, path::Path};

use elf::{
    abi::{EM_RISCV, ET_EXEC, PF_X, PT_LOAD},
    endian::LittleEndian,
    file::Class,
    ElfBytes,
};

use crate::error::{Result, RunnerError};

/// Size of a word in bytes for the RV32 (RISC-V 32-bit) architecture.
/// In RV32, a word is 32 bits (4 bytes), which is the native integer size.
/// See: https://riscv.org/technical/specifications/
const WORD_SIZE: usize = 4;

/// Minimal ELF decoder for RV32IM guests.
///
/// Implementation inspired by https://github.com/openvm-org/openvm/blob/main/crates/toolchain/transpiler/src/elf.rs
/// which is licensed under MIT/Apache-2.0.
#[derive(Debug, Clone)]
pub struct Elf {
    pub instructions: Vec<u32>,
    pub pc_start: u32,
    pub pc_base: u32,
    pub memory_image: BTreeMap<u32, u32>,
}

impl Elf {
    pub fn from_path(path: &Path, max_mem: u32) -> Result<Self> {
        let bytes = fs::read(path).map_err(|source| RunnerError::ReadElfFile {
            path: path.display().to_string(),
            source,
        })?;
        Self::decode(&bytes, max_mem)
    }

    pub fn decode(bytes: &[u8], max_mem: u32) -> Result<Self> {
        let mut image = BTreeMap::new();
        let elf = ElfBytes::<LittleEndian>::minimal_parse(bytes)
            .map_err(|err| RunnerError::ElfParse(err.to_string()))?;

        if elf.ehdr.class != Class::ELF32 {
            return Err(RunnerError::Not32BitElf);
        } else if elf.ehdr.e_machine != EM_RISCV {
            return Err(RunnerError::InvalidMachineType);
        } else if elf.ehdr.e_type != ET_EXEC {
            return Err(RunnerError::InvalidElfType);
        }

        // SAFETY: After checking class == ELF32, e_entry is guaranteed to fit in u32
        // (the elf crate stores it as u64 but reads it as u32 for ELF32 files)
        let entry: u32 = elf.ehdr.e_entry as u32;

        if entry >= max_mem || !entry.is_multiple_of(WORD_SIZE as u32) {
            return Err(RunnerError::InvalidEntrypoint(entry));
        }

        let segments = elf.segments().ok_or(RunnerError::MissingSegmentTable)?;
        if segments.len() > 256 {
            return Err(RunnerError::TooManyProgramHeaders);
        }

        let mut instructions = Vec::new();
        let mut base_address = u32::MAX;
        for segment in segments.iter().filter(|seg| seg.p_type == PT_LOAD) {
            let file_size: u32 = segment.p_filesz.try_into()?;
            if file_size >= max_mem {
                return Err(RunnerError::SegmentFileSizeExceedsMemory);
            }
            let mem_size: u32 = segment.p_memsz.try_into()?;
            if mem_size >= max_mem {
                return Err(RunnerError::SegmentMemorySizeExceedsMemory);
            }
            let vaddr: u32 = segment.p_vaddr.try_into()?;
            if !vaddr.is_multiple_of(WORD_SIZE as u32) {
                return Err(RunnerError::UnalignedSegmentAddress(vaddr));
            }
            if (segment.p_flags & PF_X) != 0 && base_address > vaddr {
                base_address = vaddr;
            }
            let offset: u32 = segment.p_offset.try_into()?;
            for i in (0..mem_size).step_by(WORD_SIZE) {
                let addr = vaddr.checked_add(i).ok_or(RunnerError::VaddrOverflow)?;
                if addr >= max_mem {
                    return Err(RunnerError::AddressExceedsMaximum { addr, max_mem });
                }

                if i >= file_size {
                    image.insert(addr, 0);
                    continue;
                }

                let mut word = 0u32;
                let len = min(file_size - i, WORD_SIZE as u32);
                for j in 0..len {
                    let idx = (offset + i + j) as usize;
                    let byte = bytes.get(idx).ok_or(RunnerError::InvalidSegmentOffset)?;
                    word |= u32::from(*byte) << (j * 8);
                }
                image.insert(addr, word);
                if (segment.p_flags & PF_X) != 0 {
                    instructions.push(word);
                }
            }
        }

        if base_address == u32::MAX {
            base_address = entry;
        }

        Ok(Self {
            instructions,
            pc_start: entry,
            pc_base: base_address,
            memory_image: image,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_GUEST_MEMORY: u32 = 1 << 29;

    /// Helper to create a minimal valid RV32 ELF header
    fn create_minimal_elf32_header() -> Vec<u8> {
        let mut bytes = vec![0u8; 52]; // ELF32 header is 52 bytes

        // ELF magic number
        bytes[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);

        // ELF class (32-bit)
        bytes[4] = 1; // ELFCLASS32

        // Data encoding (little-endian)
        bytes[5] = 1; // ELFDATA2LSB

        // ELF version
        bytes[6] = 1;

        // OS/ABI
        bytes[7] = 0;

        // ELF type (ET_EXEC = 2)
        bytes[16..18].copy_from_slice(&2u16.to_le_bytes());

        // Machine type (EM_RISCV = 243)
        bytes[18..20].copy_from_slice(&243u16.to_le_bytes());

        // ELF version
        bytes[20..24].copy_from_slice(&1u32.to_le_bytes());

        // Entry point - aligned to 4 bytes
        bytes[24..28].copy_from_slice(&0x1000u32.to_le_bytes());

        // Program header offset (right after ELF header)
        bytes[28..32].copy_from_slice(&52u32.to_le_bytes());

        // Section header offset (none)
        bytes[32..36].copy_from_slice(&0u32.to_le_bytes());

        // Flags
        bytes[36..40].copy_from_slice(&0u32.to_le_bytes());

        // ELF header size
        bytes[40..42].copy_from_slice(&52u16.to_le_bytes());

        // Program header entry size (32 bytes for ELF32)
        bytes[42..44].copy_from_slice(&32u16.to_le_bytes());

        // Number of program headers
        bytes[44..46].copy_from_slice(&1u16.to_le_bytes());

        // Section header entry size
        bytes[46..48].copy_from_slice(&40u16.to_le_bytes());

        // Number of section headers
        bytes[48..50].copy_from_slice(&0u16.to_le_bytes());

        // Section name string table index
        bytes[50..52].copy_from_slice(&0u16.to_le_bytes());

        bytes
    }

    /// Helper to add a program header
    fn add_program_header(
        bytes: &mut Vec<u8>,
        p_type: u32,
        p_offset: u32,
        p_vaddr: u32,
        p_filesz: u32,
        p_memsz: u32,
        p_flags: u32,
    ) {
        let start = bytes.len();
        bytes.resize(start + 32, 0);

        // p_type
        bytes[start..start + 4].copy_from_slice(&p_type.to_le_bytes());
        // p_offset
        bytes[start + 4..start + 8].copy_from_slice(&p_offset.to_le_bytes());
        // p_vaddr
        bytes[start + 8..start + 12].copy_from_slice(&p_vaddr.to_le_bytes());
        // p_paddr
        bytes[start + 12..start + 16].copy_from_slice(&p_vaddr.to_le_bytes());
        // p_filesz
        bytes[start + 16..start + 20].copy_from_slice(&p_filesz.to_le_bytes());
        // p_memsz
        bytes[start + 20..start + 24].copy_from_slice(&p_memsz.to_le_bytes());
        // p_flags
        bytes[start + 24..start + 28].copy_from_slice(&p_flags.to_le_bytes());
        // p_align
        bytes[start + 28..start + 32].copy_from_slice(&4u32.to_le_bytes());
    }

    #[test]
    fn test_decode_invalid_magic() {
        let bytes = vec![0u8; 100];
        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RunnerError::ElfParse(_)));
    }

    #[test]
    fn test_decode_not_32bit_elf() {
        // Create a 64-bit ELF header
        let mut bytes = vec![0u8; 64]; // ELF64 header is 64 bytes

        // ELF magic
        bytes[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
        bytes[4] = 2; // ELFCLASS64
        bytes[5] = 1; // ELFDATA2LSB
        bytes[6] = 1; // version

        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RunnerError::Not32BitElf));
    }

    #[test]
    fn test_decode_invalid_machine_type() {
        let mut bytes = create_minimal_elf32_header();
        // Add a program header so the file is valid except for machine type
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1000, 4, 4, PF_X);
        bytes.extend_from_slice(&[0u8; 4]);

        // Change machine type to x86 (EM_386 = 3)
        bytes[18..20].copy_from_slice(&3u16.to_le_bytes());

        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RunnerError::InvalidMachineType));
    }

    #[test]
    fn test_decode_invalid_elf_type() {
        let mut bytes = create_minimal_elf32_header();
        // Add a program header so the file is valid except for elf type
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1000, 4, 4, PF_X);
        bytes.extend_from_slice(&[0u8; 4]);

        // Change to shared object (ET_DYN = 3)
        bytes[16..18].copy_from_slice(&3u16.to_le_bytes());

        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RunnerError::InvalidElfType));
    }

    #[test]
    fn test_decode_entry_exceeds_max_mem() {
        let mut bytes = create_minimal_elf32_header();
        // Set entry point beyond max memory (aligned)
        bytes[24..28].copy_from_slice(&(MAX_GUEST_MEMORY).to_le_bytes());
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1000, 4, 4, PF_X);
        bytes.extend_from_slice(&[0u8; 4]); // dummy segment data
        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RunnerError::InvalidEntrypoint(_)));
    }

    #[test]
    fn test_decode_unaligned_entry() {
        let mut bytes = create_minimal_elf32_header();
        // Set unaligned entry point
        bytes[24..28].copy_from_slice(&0x1001u32.to_le_bytes());
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1000, 4, 4, PF_X);
        bytes.extend_from_slice(&[0u8; 4]);
        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RunnerError::InvalidEntrypoint(_)));
    }

    #[test]
    fn test_decode_too_many_program_headers() {
        let mut bytes = create_minimal_elf32_header();
        // Set 257 program headers (more than the 256 limit)
        bytes[44..46].copy_from_slice(&257u16.to_le_bytes());

        // Need enough data for the elf parser to not fail early
        // Add dummy program headers
        for _ in 0..257 {
            let start = bytes.len();
            bytes.resize(start + 32, 0);
        }

        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RunnerError::TooManyProgramHeaders));
    }

    #[test]
    fn test_decode_segment_file_size_exceeds_memory() {
        let mut bytes = create_minimal_elf32_header();
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1000, MAX_GUEST_MEMORY, 4, PF_X);
        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RunnerError::SegmentFileSizeExceedsMemory
        ));
    }

    #[test]
    fn test_decode_segment_mem_size_exceeds_memory() {
        let mut bytes = create_minimal_elf32_header();
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1000, 4, MAX_GUEST_MEMORY, PF_X);
        bytes.extend_from_slice(&[0u8; 4]);
        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RunnerError::SegmentMemorySizeExceedsMemory
        ));
    }

    #[test]
    fn test_decode_unaligned_segment_address() {
        let mut bytes = create_minimal_elf32_header();
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1001, 4, 4, PF_X); // unaligned vaddr
        bytes.extend_from_slice(&[0u8; 4]);
        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RunnerError::UnalignedSegmentAddress(_)
        ));
    }

    #[test]
    fn test_decode_address_exceeds_maximum() {
        let mut bytes = create_minimal_elf32_header();
        // Large vaddr that will exceed max_mem during iteration
        let vaddr = MAX_GUEST_MEMORY - 8;
        add_program_header(&mut bytes, PT_LOAD, 84, vaddr, 4, 16, PF_X);
        bytes.extend_from_slice(&[0u8; 4]);
        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RunnerError::AddressExceedsMaximum { .. }
        ));
    }

    #[test]
    fn test_decode_invalid_segment_offset() {
        let mut bytes = create_minimal_elf32_header();
        // Set offset beyond actual file
        add_program_header(&mut bytes, PT_LOAD, 10000, 0x1000, 4, 4, PF_X);
        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RunnerError::InvalidSegmentOffset));
    }

    #[test]
    fn test_decode_valid_minimal_elf() {
        let mut bytes = create_minimal_elf32_header();
        // Add a valid executable segment
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1000, 4, 4, PF_X);
        // Add instruction data (ADDI x0, x0, 0 - NOP)
        bytes.extend_from_slice(&0x00000013u32.to_le_bytes());

        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_ok());
        let elf = result.unwrap();
        assert_eq!(elf.pc_start, 0x1000);
        assert_eq!(elf.pc_base, 0x1000);
        assert_eq!(elf.instructions.len(), 1);
        assert_eq!(elf.instructions[0], 0x00000013);
    }

    #[test]
    fn test_decode_multiple_segments() {
        let mut bytes = create_minimal_elf32_header();
        // Update to 2 program headers
        bytes[44..46].copy_from_slice(&2u16.to_le_bytes());

        // First segment (executable at 0x2000)
        add_program_header(&mut bytes, PT_LOAD, 116, 0x2000, 4, 4, PF_X);
        // Second segment (executable at 0x1000 - lower address)
        add_program_header(&mut bytes, PT_LOAD, 120, 0x1000, 4, 4, PF_X);

        // Add instruction data
        bytes.extend_from_slice(&0x00000013u32.to_le_bytes()); // for first segment
        bytes.extend_from_slice(&0x00100093u32.to_le_bytes()); // for second segment

        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_ok());
        let elf = result.unwrap();
        // pc_base should be the lowest executable address
        assert_eq!(elf.pc_base, 0x1000);
        assert_eq!(elf.instructions.len(), 2);
    }

    #[test]
    fn test_decode_bss_segment() {
        let mut bytes = create_minimal_elf32_header();
        // Add a segment with memsz > filesz (BSS-like)
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1000, 4, 8, PF_X);
        bytes.extend_from_slice(&0x00000013u32.to_le_bytes());

        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_ok());
        let elf = result.unwrap();
        // Should have memory entries for both file content and zeroed BSS
        assert!(elf.memory_image.contains_key(&0x1000));
        assert!(elf.memory_image.contains_key(&0x1004));
        assert_eq!(elf.memory_image[&0x1004], 0); // BSS is zeroed
    }

    #[test]
    fn test_decode_non_executable_segment() {
        let mut bytes = create_minimal_elf32_header();
        // Update to 2 program headers
        bytes[44..46].copy_from_slice(&2u16.to_le_bytes());

        // Data segment (non-executable)
        add_program_header(&mut bytes, PT_LOAD, 116, 0x2000, 4, 4, 0); // no PF_X
        // Code segment (executable)
        add_program_header(&mut bytes, PT_LOAD, 120, 0x1000, 4, 4, PF_X);

        bytes.extend_from_slice(&0xDEADBEEFu32.to_le_bytes()); // data
        bytes.extend_from_slice(&0x00000013u32.to_le_bytes()); // code

        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_ok());
        let elf = result.unwrap();
        // Only executable segments contribute to instructions
        assert_eq!(elf.instructions.len(), 1);
        assert_eq!(elf.instructions[0], 0x00000013);
        // But both are in memory image
        assert!(elf.memory_image.contains_key(&0x1000));
        assert!(elf.memory_image.contains_key(&0x2000));
    }

    #[test]
    fn test_decode_no_executable_segments() {
        let mut bytes = create_minimal_elf32_header();
        // Non-executable segment
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1000, 4, 4, 0);
        bytes.extend_from_slice(&0xDEADBEEFu32.to_le_bytes());

        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_ok());
        let elf = result.unwrap();
        // pc_base falls back to entry when no executable segments
        assert_eq!(elf.pc_base, 0x1000); // entry point
        assert!(elf.instructions.is_empty());
    }

    #[test]
    fn test_from_path_nonexistent_file() {
        let path = Path::new("/nonexistent/path/to/elf");
        let result = Elf::from_path(path, MAX_GUEST_MEMORY);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RunnerError::ReadElfFile { .. }));
    }

    #[test]
    fn test_from_path_with_temp_file() {
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        let elf_path = temp_dir.path().join("test.elf");

        let mut bytes = create_minimal_elf32_header();
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1000, 4, 4, PF_X);
        bytes.extend_from_slice(&0x00000013u32.to_le_bytes());

        let mut file = std::fs::File::create(&elf_path).unwrap();
        file.write_all(&bytes).unwrap();

        let result = Elf::from_path(&elf_path, MAX_GUEST_MEMORY);
        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_partial_word_at_segment_end() {
        let mut bytes = create_minimal_elf32_header();
        // Segment with filesz = 6 (not a multiple of 4)
        add_program_header(&mut bytes, PT_LOAD, 84, 0x1000, 6, 8, PF_X);
        // Add 6 bytes of data + padding
        bytes.extend_from_slice(&[0x13, 0x00, 0x00, 0x00, 0x93, 0x00]);

        let result = Elf::decode(&bytes, MAX_GUEST_MEMORY);
        assert!(result.is_ok());
        let elf = result.unwrap();
        // First word should be complete
        assert_eq!(elf.memory_image[&0x1000], 0x00000013);
        // Second word should have partial data (only 2 bytes)
        assert_eq!(elf.memory_image[&0x1004], 0x00000093);
    }
}
