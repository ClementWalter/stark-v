use crate::ops::{alu, alu_imm, branch, jump, load, muldiv, store, upper};
use crate::{Cpu, DecodedInst, Memory, Opcode};

/// Execute a decoded instruction. Each opcode handles PC advancement internally.
pub fn execute(cpu: &mut Cpu, mem: &mut Memory, inst: &DecodedInst) {
    match inst.opcode {
        // R-type ALU
        Opcode::Add => alu::add(cpu, inst),
        Opcode::Sub => alu::sub(cpu, inst),
        Opcode::Sll => alu::sll(cpu, inst),
        Opcode::Slt => alu::slt(cpu, inst),
        Opcode::Sltu => alu::sltu(cpu, inst),
        Opcode::Xor => alu::xor(cpu, inst),
        Opcode::Srl => alu::srl(cpu, inst),
        Opcode::Sra => alu::sra(cpu, inst),
        Opcode::Or => alu::or(cpu, inst),
        Opcode::And => alu::and(cpu, inst),

        // I-type ALU
        Opcode::Addi => alu_imm::addi(cpu, inst),
        Opcode::Slti => alu_imm::slti(cpu, inst),
        Opcode::Sltiu => alu_imm::sltiu(cpu, inst),
        Opcode::Xori => alu_imm::xori(cpu, inst),
        Opcode::Ori => alu_imm::ori(cpu, inst),
        Opcode::Andi => alu_imm::andi(cpu, inst),
        Opcode::Slli => alu_imm::slli(cpu, inst),
        Opcode::Srli => alu_imm::srli(cpu, inst),
        Opcode::Srai => alu_imm::srai(cpu, inst),

        // Loads
        Opcode::Lb => load::lb(cpu, mem, inst),
        Opcode::Lh => load::lh(cpu, mem, inst),
        Opcode::Lw => load::lw(cpu, mem, inst),
        Opcode::Lbu => load::lbu(cpu, mem, inst),
        Opcode::Lhu => load::lhu(cpu, mem, inst),

        // Stores
        Opcode::Sb => store::sb(cpu, mem, inst),
        Opcode::Sh => store::sh(cpu, mem, inst),
        Opcode::Sw => store::sw(cpu, mem, inst),

        // Branches
        Opcode::Beq => branch::beq(cpu, inst),
        Opcode::Bne => branch::bne(cpu, inst),
        Opcode::Blt => branch::blt(cpu, inst),
        Opcode::Bge => branch::bge(cpu, inst),
        Opcode::Bltu => branch::bltu(cpu, inst),
        Opcode::Bgeu => branch::bgeu(cpu, inst),

        // Jumps
        Opcode::Jal => jump::jal(cpu, inst),
        Opcode::Jalr => jump::jalr(cpu, inst),

        // Upper immediates
        Opcode::Lui => upper::lui(cpu, inst),
        Opcode::Auipc => upper::auipc(cpu, inst),

        // M-extension
        Opcode::Mul => muldiv::mul(cpu, inst),
        Opcode::Mulh => muldiv::mulh(cpu, inst),
        Opcode::Mulhsu => muldiv::mulhsu(cpu, inst),
        Opcode::Mulhu => muldiv::mulhu(cpu, inst),
        Opcode::Div => muldiv::div(cpu, inst),
        Opcode::Divu => muldiv::divu(cpu, inst),
        Opcode::Rem => muldiv::rem(cpu, inst),
        Opcode::Remu => muldiv::remu(cpu, inst),
    }
}
