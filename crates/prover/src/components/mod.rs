//! Component system for tracer-backed and preprocessed AIR components.

pub mod mem_clock_update;
pub mod memory;
pub mod merkle;
pub mod opcodes;
pub mod poseidon2;
pub mod preprocessed;
pub mod program;
pub mod reg_clock_update;

stwo_macros::opcode_components! {
    preprocessed,
    opcodes::auipc,
    opcodes::base_alu_imm,
    opcodes::base_alu_reg,
    opcodes::branch_eq,
    opcodes::branch_lt,
    opcodes::div,
    opcodes::jal,
    opcodes::jalr,
    opcodes::load_store,
    opcodes::lt_imm,
    opcodes::lt_reg,
    opcodes::lui,
    opcodes::mul,
    opcodes::mulh,
    opcodes::shifts_imm,
    opcodes::shifts_reg,
    program,
    memory,
    merkle,
    poseidon2,
    mem_clock_update,
    reg_clock_update,
}
