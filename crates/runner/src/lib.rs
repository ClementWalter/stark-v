mod cpu;
pub mod decode;
mod elf;
mod execute;
mod memory;
mod ops;
pub mod trace;

use decode::get_or_decode;
use thiserror::Error;

pub use cpu::Cpu;
pub use decode::{DecodedInst, InstCache, Opcode};
pub use elf::{load_elf, ElfError};
pub use execute::execute;
pub use memory::Memory;
pub use runner_macros::traced;
pub use trace::{Access, Tracer};

/// Errors that can occur during program execution.
#[derive(Error, Debug)]
pub enum RunError {
    #[error("Failed to load ELF: {0}")]
    Elf(#[from] ElfError),

    #[error("Invalid instruction at PC=0x{pc:08x}")]
    InvalidInstruction { pc: u32 },

    #[error("Exceeded maximum cycles ({max})")]
    MaxCyclesExceeded { cycles: u64, max: u64 },
}

/// Result of a successful program execution.
#[derive(Debug)]
pub struct RunResult {
    /// Total number of cycles executed.
    pub cycles: u64,
    /// Final program counter (where the infinite loop was detected).
    pub final_pc: u32,
}

/// Run an ELF program to completion.
///
/// Executes until an infinite loop is detected (PC unchanged after instruction)
/// or the maximum cycle count is reached.
///
/// # Arguments
/// * `elf_bytes` - Raw bytes of the ELF file
/// * `max_cycles` - Maximum number of cycles before aborting
///
/// # Returns
/// * `Ok(RunResult)` - Program completed successfully
/// * `Err(RunError)` - Execution failed
///
/// # Example
/// ```ignore
/// let elf_bytes = std::fs::read("guest.elf")?;
/// let result = runner::run(&elf_bytes, 10_000_000)?;
/// println!("Completed in {} cycles", result.cycles);
/// ```
pub fn run(elf_bytes: &[u8], max_cycles: u64) -> Result<RunResult, RunError> {
    let loaded = load_elf(elf_bytes)?;

    let mut cpu = Cpu::new(loaded.entry, loaded.sp, loaded.gp);
    let mut mem = loaded.memory;
    let mut cache: InstCache = InstCache::default();

    let mut cycle_count: u64 = 0;

    loop {
        let prev_pc = cpu.pc;

        let inst = get_or_decode(&mut cache, &mem, cpu.pc)
            .ok_or(RunError::InvalidInstruction { pc: cpu.pc })?;

        let pc_modified = execute(&mut cpu, &mut mem, &inst);

        if !pc_modified {
            cpu.advance_pc();
        }

        cycle_count += 1;

        // Halt on infinite loop (PC unchanged after execution)
        if cpu.pc == prev_pc {
            return Ok(RunResult {
                cycles: cycle_count,
                final_pc: prev_pc,
            });
        }

        // Safety limit
        if cycle_count > max_cycles {
            return Err(RunError::MaxCyclesExceeded {
                cycles: cycle_count,
                max: max_cycles,
            });
        }
    }
}
