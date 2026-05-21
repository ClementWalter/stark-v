//! Component system for tracer-backed and preprocessed AIR components.

pub mod mem_clock_update;
pub mod memory;
pub mod merkle;
pub mod opcodes;
pub mod poseidon2;
pub mod preprocessed;
pub mod program;
pub mod reg_clock_update;

pub use opcodes::{
    auipc, base_alu_imm, base_alu_reg, branch_eq, branch_lt, div, jal, jalr, load_store, lt_imm,
    lt_reg, lui, mul, mulh, shifts_imm, shifts_reg,
};

stwo_macros::opcode_components! {
    preprocessed: preprocessed;
    auipc: opcodes::auipc,
    base_alu_imm: opcodes::base_alu_imm,
    base_alu_reg: opcodes::base_alu_reg,
    branch_eq: opcodes::branch_eq,
    branch_lt: opcodes::branch_lt,
    div: opcodes::div,
    jal: opcodes::jal,
    jalr: opcodes::jalr,
    load_store: opcodes::load_store,
    lt_imm: opcodes::lt_imm,
    lt_reg: opcodes::lt_reg,
    lui: opcodes::lui,
    mul: opcodes::mul,
    mulh: opcodes::mulh,
    shifts_imm: opcodes::shifts_imm,
    shifts_reg: opcodes::shifts_reg,
    program,
    memory,
    merkle,
    poseidon2,
    mem_clock_update,
    reg_clock_update,
}
