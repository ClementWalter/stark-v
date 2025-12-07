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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_parse_error_display() {
        let err = RunnerError::ElfParse("invalid magic".to_string());
        assert_eq!(format!("{}", err), "ELF parse error: invalid magic");
    }

    #[test]
    fn test_not_32bit_elf_display() {
        let err = RunnerError::Not32BitElf;
        assert_eq!(format!("{}", err), "Not a 32-bit ELF");
    }

    #[test]
    fn test_invalid_machine_type_display() {
        let err = RunnerError::InvalidMachineType;
        assert_eq!(format!("{}", err), "Invalid machine type, must be RISC-V");
    }

    #[test]
    fn test_invalid_elf_type_display() {
        let err = RunnerError::InvalidElfType;
        assert_eq!(format!("{}", err), "Invalid ELF type, must be executable");
    }

    #[test]
    fn test_invalid_entrypoint_display() {
        let err = RunnerError::InvalidEntrypoint(0x12345678);
        assert_eq!(format!("{}", err), "Invalid entrypoint: 0x12345678");
    }

    #[test]
    fn test_missing_segment_table_display() {
        let err = RunnerError::MissingSegmentTable;
        assert_eq!(format!("{}", err), "Missing segment table");
    }

    #[test]
    fn test_too_many_program_headers_display() {
        let err = RunnerError::TooManyProgramHeaders;
        assert_eq!(format!("{}", err), "Too many program headers");
    }

    #[test]
    fn test_segment_file_size_exceeds_memory_display() {
        let err = RunnerError::SegmentFileSizeExceedsMemory;
        assert_eq!(format!("{}", err), "segment file size exceeds memory");
    }

    #[test]
    fn test_segment_memory_size_exceeds_memory_display() {
        let err = RunnerError::SegmentMemorySizeExceedsMemory;
        assert_eq!(format!("{}", err), "segment memory size exceeds memory");
    }

    #[test]
    fn test_unaligned_segment_address_display() {
        let err = RunnerError::UnalignedSegmentAddress(0x1001);
        assert_eq!(format!("{}", err), "unaligned segment address: 0x00001001");
    }

    #[test]
    fn test_vaddr_overflow_display() {
        let err = RunnerError::VaddrOverflow;
        assert_eq!(format!("{}", err), "vaddr overflow");
    }

    #[test]
    fn test_address_exceeds_maximum_display() {
        let err = RunnerError::AddressExceedsMaximum {
            addr: 0x20000000,
            max_mem: 0x10000000,
        };
        assert_eq!(
            format!("{}", err),
            "address [0x20000000] exceeds maximum address for guest programs [0x10000000]"
        );
    }

    #[test]
    fn test_invalid_segment_offset_display() {
        let err = RunnerError::InvalidSegmentOffset;
        assert_eq!(format!("{}", err), "invalid segment offset");
    }

    #[test]
    fn test_terminate_imm_too_big_display() {
        let err = RunnerError::TerminateImmTooBig;
        assert_eq!(format!("{}", err), "TERMINATE imm must fit in u8");
    }

    #[test]
    fn test_unsupported_instruction_display() {
        let err = RunnerError::UnsupportedInstruction(0xDEADBEEF);
        assert_eq!(format!("{}", err), "unsupported instruction: 0xdeadbeef");
    }

    #[test]
    fn test_read_elf_file_display() {
        let err = RunnerError::ReadElfFile {
            path: "/path/to/elf".to_string(),
            source: io::Error::new(io::ErrorKind::NotFound, "file not found"),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("reading ELF at /path/to/elf"));
    }

    #[test]
    fn test_int_conversion_from() {
        // Try to convert a u64 that doesn't fit in u32
        let large_value: u64 = u64::MAX;
        let conversion_result: std::result::Result<u32, _> = large_value.try_into();
        let e = conversion_result.unwrap_err();
        let runner_err: RunnerError = e.into();
        let msg = format!("{}", runner_err);
        assert!(msg.contains("integer conversion error"));
    }

    #[test]
    fn test_error_debug_impl() {
        let err = RunnerError::Not32BitElf;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Not32BitElf"));
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(RunnerError::VaddrOverflow);
        assert!(result.is_err());
    }
}
