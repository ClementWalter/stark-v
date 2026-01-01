#![feature(allocator_api)]
mod commitment;
mod cpu;
pub mod decode;
mod elf;
mod execute;
mod io;
mod memory;
mod poseidon2;
mod program;
// trace module must come before ops so trace_op! macro is available
#[macro_use]
pub mod trace;
mod ops;

use decode::get_or_decode;
use thiserror::Error;

pub use commitment::CommitmentError;
pub use cpu::Cpu;
pub use decode::{DecodedInst, InstCache, Opcode};
pub use elf::{ElfError, load_elf};
pub use execute::execute;
pub use memory::Memory;
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

    #[error("Commitment error: {0}")]
    Commitment(#[from] CommitmentError),
}

/// Result of a successful program execution.
#[derive(Debug)]
pub struct RunResult {
    /// Total number of cycles executed.
    pub cycles: u64,
    /// Final program counter (where halt was detected).
    pub final_pc: u32,
    /// Output bytes from guest (postcard-serialized data).
    pub output: Option<Vec<u8>>,
    /// Execution trace for proving.
    pub tracer: Tracer,
}

/// Run an ELF program to completion.
///
/// Executes until the guest halts or an infinite loop is detected (PC unchanged after instruction)
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
    let mut tracer = Tracer::with_memory(&mem);
    let mut cache: InstCache = InstCache::default();

    loop {
        // Check halt flag before executing next instruction
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

        let inst = get_or_decode(&mut cache, &mem, cpu.pc)
            .ok_or(RunError::InvalidInstruction { pc: cpu.pc })?;
        tracer.trace_instr_access(cpu.pc);

        // Early-exit on explicit self-loop sentinels (e.g., `jal x0, 0` used to halt tests).
        // Avoid tracing this noop instruction so the final trace doesn't contain a bogus row.
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

        // Update tracer clock before executing instruction
        tracer.clk += 1;

        execute(&mut cpu, &mut mem, &inst, &mut tracer);

        // Halt on infinite loop (PC unchanged after execution) - backup detection
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

        // Safety limit
        if tracer.clk as u64 > max_cycles {
            return Err(RunError::MaxCyclesExceeded {
                cycles: tracer.clk as u64,
                max: max_cycles,
            });
        }
    }
}
