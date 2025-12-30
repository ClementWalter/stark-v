//! Opcode family components for RV32IM instructions.
//!
//! Each family groups related opcodes that share the same AIR structure.
//! See airs.md for detailed specifications.

pub mod auipc; // Section 10: auipc
pub mod base_alu_imm; // Section 2: addi, xori, ori, andi
pub mod base_alu_reg; // Section 1: add, sub, xor, or, and
pub mod branch_eq; // Section 7: beq, bne
pub mod branch_lt; // Section 8: blt, bltu, bge, bgeu
pub mod div;
pub mod jal; // Section 12: jal
pub mod jalr; // Section 11: jalr
pub mod load_store; // Section 13: lb, lbu, lh, lhu, lw, sb, sh, sw
pub mod lt_imm; // Section 6: slti, sltiu
pub mod lt_reg; // Section 5: slt, sltu
pub mod lui; // Section 9: lui
pub mod mul; // Section 14: mul
pub mod mulh; // Section 15: mulh, mulhsu, mulhu
pub mod shifts_imm; // Section 4: slli, srli, srai
pub mod shifts_reg; // Section 3: sll, srl, sra // Section 16: div, divu, rem, remu

crate::opcode_components! {
    // 16 opcode families
    base_alu_reg,
    base_alu_imm,
    shifts_reg,
    shifts_imm,
    lt_reg,
    lt_imm,
    branch_eq,
    branch_lt,
    lui,
    auipc,
    jalr,
    jal,
    load_store,
    mul,
    mulh,
    div,
}
