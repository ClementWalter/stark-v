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

pub use commitment::{CommitmentError, MAX_TREE_HEIGHT};
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

    #[error("Input length {len} exceeds input capacity {capacity}")]
    InputTooLarge { len: usize, capacity: usize },

    #[error("Commitment error: {0}")]
    Commitment(#[from] CommitmentError),
}

/// Word-aligned I/O word captured from memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoWord {
    pub addr: u32,
    pub value: u32,
}

/// Result of a successful program execution.
#[derive(Debug)]
pub struct RunResult {
    /// Total number of cycles executed.
    pub cycles: u64,
    /// Entry program counter.
    pub initial_pc: u32,
    /// Final program counter (where halt was detected).
    pub final_pc: u32,
    /// Register values at start of execution.
    pub initial_regs: [u32; 32],
    /// Register values at end of execution.
    pub final_regs: [u32; 32],
    /// Output bytes from guest (postcard-serialized data).
    pub output: Option<Vec<u8>>,
    /// Raw input bytes provided to the guest.
    pub input: Vec<u8>,
    /// Input region start address.
    pub input_start: u32,
    /// Input region end address (exclusive).
    pub input_end: u32,
    /// Output length (value stored at output_len_addr).
    pub output_len: u32,
    /// Address of output length word.
    pub output_len_addr: u32,
    /// Address of output data start.
    pub output_data_addr: u32,
    /// Address of output data end (exclusive).
    pub output_end_addr: u32,
    /// Output words (length word + output data words).
    pub output_words: Vec<IoWord>,
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
    run_with_input(elf_bytes, &[], max_cycles)
}

/// Run an ELF program to completion with explicit input bytes.
pub fn run_with_input(
    elf_bytes: &[u8],
    input: &[u8],
    max_cycles: u64,
) -> Result<RunResult, RunError> {
    let loaded = load_elf(elf_bytes)?;
    let layout = commitment::MemoryLayout::from_loaded(&loaded);

    let mut cpu = Cpu::new(loaded.entry, loaded.sp, loaded.gp);
    let initial_pc = cpu.pc;
    let initial_regs = cpu.regs();
    let mut mem = loaded.memory;
    let input_start = loaded.input_start_addr;
    let input_end = loaded.input_end_addr;
    let input_capacity = input_end.saturating_sub(input_start) as usize;
    if input.len() > input_capacity {
        return Err(RunError::InputTooLarge {
            len: input.len(),
            capacity: input_capacity,
        });
    }
    for (idx, byte) in input.iter().enumerate() {
        let addr = input_start.wrapping_add(idx as u32);
        mem.write_u8(addr, *byte);
    }
    let mut cache: InstCache = InstCache::default();
    let mut tracer = Tracer::default();

    loop {
        // Check halt flag before executing next instruction
        if mem.read_u32(loaded.halt_flag_addr) != 0 {
            let output_len = mem.read_u32(loaded.output_len_addr);
            let output = io::read_output(
                &mem,
                loaded.output_len_addr,
                loaded.output_data_addr,
                loaded.output_end_addr,
            );
            let output_words = collect_output_words(
                &mem,
                loaded.output_len_addr,
                loaded.output_data_addr,
                output_len,
            );
            tracer.finalize_commitments(&mem, &layout)?;
            return Ok(RunResult {
                cycles: tracer.clk as u64,
                initial_pc,
                final_pc: cpu.pc,
                initial_regs,
                final_regs: cpu.regs(),
                output,
                input: input.to_vec(),
                input_start,
                input_end,
                output_len,
                output_len_addr: loaded.output_len_addr,
                output_data_addr: loaded.output_data_addr,
                output_end_addr: loaded.output_end_addr,
                output_words,
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
            let output_len = mem.read_u32(loaded.output_len_addr);
            let output = io::read_output(
                &mem,
                loaded.output_len_addr,
                loaded.output_data_addr,
                loaded.output_end_addr,
            );
            let output_words = collect_output_words(
                &mem,
                loaded.output_len_addr,
                loaded.output_data_addr,
                output_len,
            );
            tracer.finalize_commitments(&mem, &layout)?;
            return Ok(RunResult {
                cycles: tracer.clk as u64,
                initial_pc,
                final_pc: cpu.pc,
                initial_regs,
                final_regs: cpu.regs(),
                output,
                input: input.to_vec(),
                input_start,
                input_end,
                output_len,
                output_len_addr: loaded.output_len_addr,
                output_data_addr: loaded.output_data_addr,
                output_end_addr: loaded.output_end_addr,
                output_words,
                tracer,
            });
        }

        // Update tracer clock before executing instruction
        tracer.clk += 1;

        execute(&mut cpu, &mut mem, &inst, &mut tracer);

        // Halt on infinite loop (PC unchanged after execution) - backup detection
        if cpu.pc == prev_pc {
            let output_len = mem.read_u32(loaded.output_len_addr);
            let output = io::read_output(
                &mem,
                loaded.output_len_addr,
                loaded.output_data_addr,
                loaded.output_end_addr,
            );
            let output_words = collect_output_words(
                &mem,
                loaded.output_len_addr,
                loaded.output_data_addr,
                output_len,
            );
            tracer.finalize_commitments(&mem, &layout)?;
            return Ok(RunResult {
                cycles: tracer.clk as u64,
                initial_pc,
                final_pc: prev_pc,
                initial_regs,
                final_regs: cpu.regs(),
                output,
                input: input.to_vec(),
                input_start,
                input_end,
                output_len,
                output_len_addr: loaded.output_len_addr,
                output_data_addr: loaded.output_data_addr,
                output_end_addr: loaded.output_end_addr,
                output_words,
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

pub(crate) fn collect_output_words(
    mem: &Memory,
    output_len_addr: u32,
    output_data_addr: u32,
    output_len: u32,
) -> Vec<IoWord> {
    let mut words = Vec::new();
    let len_addr = output_len_addr & !3;
    words.push(IoWord {
        addr: len_addr,
        value: mem.read_u32(len_addr),
    });
    if output_len == 0 {
        return words;
    }
    let start = output_data_addr & !3;
    let end = output_data_addr.wrapping_add(output_len);
    let end_aligned = end.wrapping_add(3) & !3;
    let mut addr = start;
    while addr < end_aligned {
        words.push(IoWord {
            addr,
            value: mem.read_u32(addr),
        });
        addr = addr.wrapping_add(4);
    }
    words
}
