use std::{cmp::min, collections::BTreeMap, fs, path::Path};

use elf::{
    abi::{EM_RISCV, ET_EXEC, PF_X, PT_LOAD},
    endian::LittleEndian,
    file::Class,
    ElfBytes,
};
use eyre::{bail, Context, ContextCompat, Result};

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
        let bytes = fs::read(path).with_context(|| format!("reading ELF at {}", path.display()))?;
        Self::decode(&bytes, max_mem)
    }

    pub fn decode(bytes: &[u8], max_mem: u32) -> Result<Self> {
        let mut image = BTreeMap::new();
        let elf = ElfBytes::<LittleEndian>::minimal_parse(bytes)
            .map_err(|err| eyre::eyre!("Elf parse error: {err}"))?;

        if elf.ehdr.class != Class::ELF32 {
            bail!("Not a 32-bit ELF");
        } else if elf.ehdr.e_machine != EM_RISCV {
            bail!("Invalid machine type, must be RISC-V");
        } else if elf.ehdr.e_type != ET_EXEC {
            bail!("Invalid ELF type, must be executable");
        }

        let entry: u32 = elf
            .ehdr
            .e_entry
            .try_into()
            .map_err(|err| eyre::eyre!("entry exceeds 32 bits: {err}"))?;

        if entry >= max_mem || !entry.is_multiple_of(WORD_SIZE as u32) {
            bail!("Invalid entrypoint: 0x{entry:08x}");
        }

        let segments = elf
            .segments()
            .ok_or_else(|| eyre::eyre!("Missing segment table"))?;
        if segments.len() > 256 {
            bail!("Too many program headers");
        }

        let mut instructions = Vec::new();
        let mut base_address = u32::MAX;
        for segment in segments.iter().filter(|seg| seg.p_type == PT_LOAD) {
            let file_size: u32 = segment.p_filesz.try_into()?;
            if file_size >= max_mem {
                bail!("segment file size exceeds memory");
            }
            let mem_size: u32 = segment.p_memsz.try_into()?;
            if mem_size >= max_mem {
                bail!("segment memory size exceeds memory");
            }
            let vaddr: u32 = segment.p_vaddr.try_into()?;
            if !vaddr.is_multiple_of(WORD_SIZE as u32) {
                bail!("unaligned segment address: 0x{vaddr:08x}");
            }
            if (segment.p_flags & PF_X) != 0 && base_address > vaddr {
                base_address = vaddr;
            }
            let offset: u32 = segment.p_offset.try_into()?;
            for i in (0..mem_size).step_by(WORD_SIZE) {
                let addr = vaddr
                    .checked_add(i)
                    .ok_or_else(|| eyre::eyre!("vaddr overflow"))?;
                if addr >= max_mem {
                    bail!(
                        "address [0x{addr:08x}] exceeds maximum address for guest programs [0x{max_mem:08x}]"
                    );
                }

                if i >= file_size {
                    image.insert(addr, 0);
                    continue;
                }

                let mut word = 0u32;
                let len = min(file_size - i, WORD_SIZE as u32);
                for j in 0..len {
                    let idx = (offset + i + j) as usize;
                    let byte = bytes.get(idx).context("invalid segment offset")?;
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
