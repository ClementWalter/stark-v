use std::io;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, RunnerError>;

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("ELF parse error: {0}")]
    ElfParse(String),

    #[error("Not a 32-bit ELF")]
    Not32BitElf,

    #[error("Invalid machine type, must be RISC-V")]
    InvalidMachineType,

    #[error("Invalid ELF type, must be executable")]
    InvalidElfType,

    #[error("entry exceeds 32 bits: {0}")]
    EntryExceeds32Bits(String),

    #[error("Invalid entrypoint: 0x{0:08x}")]
    InvalidEntrypoint(u32),

    #[error("Missing segment table")]
    MissingSegmentTable,

    #[error("Too many program headers")]
    TooManyProgramHeaders,

    #[error("segment file size exceeds memory")]
    SegmentFileSizeExceedsMemory,

    #[error("segment memory size exceeds memory")]
    SegmentMemorySizeExceedsMemory,

    #[error("unaligned segment address: 0x{0:08x}")]
    UnalignedSegmentAddress(u32),

    #[error("vaddr overflow")]
    VaddrOverflow,

    #[error("address [0x{addr:08x}] exceeds maximum address for guest programs [0x{max_mem:08x}]")]
    AddressExceedsMaximum { addr: u32, max_mem: u32 },

    #[error("invalid segment offset")]
    InvalidSegmentOffset,

    #[error("TERMINATE imm must fit in u8")]
    TerminateImmTooBig,

    #[error("unsupported instruction: 0x{0:08x}")]
    UnsupportedInstruction(u32),

    #[error("reading ELF at {path}: {source}")]
    ReadElfFile {
        path: String,
        #[source]
        source: io::Error,
    },

    #[error("integer conversion error: {0}")]
    IntConversion(#[from] std::num::TryFromIntError),
}
