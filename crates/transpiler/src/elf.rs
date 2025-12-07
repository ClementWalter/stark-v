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

/// ELF decoder for RV32IM guests.
///
/// Implementation inspired by https://github.com/openvm-org/openvm/blob/02d5a13556b167ccae0c3b817f612adf26d92fda/crates/toolchain/transpiler/src/elf.rs
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

        // ELF entry point
        let entry: u32 =
            elf.ehdr
                .e_entry
                .try_into()
                .map_err(|err: std::num::TryFromIntError| {
                    RunnerError::EntryExceeds32Bits(err.to_string())
                })?;

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
