//! Component system for RV32IM opcodes.

pub mod alu;
pub mod alu_imm;
pub mod branch;
pub mod jump;
pub mod load;
pub mod muldiv;
pub mod store;
pub mod upper;

// Aggregate all 45 components
crate::components! {
    // ALU (10 opcodes)
    alu::add,
    alu::sub,
    alu::sll,
    alu::slt,
    alu::sltu,
    alu::xor,
    alu::srl,
    alu::sra,
    alu::or,
    alu::and,

    // ALU Immediate (9 opcodes)
    alu_imm::addi,
    alu_imm::slti,
    alu_imm::sltiu,
    alu_imm::xori,
    alu_imm::ori,
    alu_imm::andi,
    alu_imm::slli,
    alu_imm::srli,
    alu_imm::srai,

    // Load (5 opcodes)
    load::lb,
    load::lh,
    load::lw,
    load::lbu,
    load::lhu,

    // Store (3 opcodes)
    store::sb,
    store::sh,
    store::sw,

    // Branch (6 opcodes)
    branch::beq,
    branch::bne,
    branch::blt,
    branch::bge,
    branch::bltu,
    branch::bgeu,

    // Jump (2 opcodes)
    jump::jal,
    jump::jalr,

    // Upper Immediate (2 opcodes)
    upper::lui,
    upper::auipc,

    // MulDiv (8 opcodes)
    muldiv::mul,
    muldiv::mulh,
    muldiv::mulhsu,
    muldiv::mulhu,
    muldiv::div,
    muldiv::divu,
    muldiv::rem,
    muldiv::remu,
}
