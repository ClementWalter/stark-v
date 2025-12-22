use crate::ops::{alu, alu_imm, branch, jump, load, muldiv, store, upper};
use crate::{Cpu, DecodedInst, Memory, Opcode};

/// Execute a decoded instruction. Returns true if PC was modified by the instruction.
pub fn execute(cpu: &mut Cpu, mem: &mut Memory, inst: &DecodedInst) -> bool {
    match inst.opcode {
        // R-type ALU
        Opcode::Add => {
            alu::add(cpu, inst);
            false
        }
        Opcode::Sub => {
            alu::sub(cpu, inst);
            false
        }
        Opcode::Sll => {
            alu::sll(cpu, inst);
            false
        }
        Opcode::Slt => {
            alu::slt(cpu, inst);
            false
        }
        Opcode::Sltu => {
            alu::sltu(cpu, inst);
            false
        }
        Opcode::Xor => {
            alu::xor(cpu, inst);
            false
        }
        Opcode::Srl => {
            alu::srl(cpu, inst);
            false
        }
        Opcode::Sra => {
            alu::sra(cpu, inst);
            false
        }
        Opcode::Or => {
            alu::or(cpu, inst);
            false
        }
        Opcode::And => {
            alu::and(cpu, inst);
            false
        }

        // I-type ALU
        Opcode::Addi => {
            alu_imm::addi(cpu, inst);
            false
        }
        Opcode::Slti => {
            alu_imm::slti(cpu, inst);
            false
        }
        Opcode::Sltiu => {
            alu_imm::sltiu(cpu, inst);
            false
        }
        Opcode::Xori => {
            alu_imm::xori(cpu, inst);
            false
        }
        Opcode::Ori => {
            alu_imm::ori(cpu, inst);
            false
        }
        Opcode::Andi => {
            alu_imm::andi(cpu, inst);
            false
        }
        Opcode::Slli => {
            alu_imm::slli(cpu, inst);
            false
        }
        Opcode::Srli => {
            alu_imm::srli(cpu, inst);
            false
        }
        Opcode::Srai => {
            alu_imm::srai(cpu, inst);
            false
        }

        // Loads
        Opcode::Lb => {
            load::lb(cpu, mem, inst);
            false
        }
        Opcode::Lh => {
            load::lh(cpu, mem, inst);
            false
        }
        Opcode::Lw => {
            load::lw(cpu, mem, inst);
            false
        }
        Opcode::Lbu => {
            load::lbu(cpu, mem, inst);
            false
        }
        Opcode::Lhu => {
            load::lhu(cpu, mem, inst);
            false
        }

        // Stores
        Opcode::Sb => {
            store::sb(cpu, mem, inst);
            false
        }
        Opcode::Sh => {
            store::sh(cpu, mem, inst);
            false
        }
        Opcode::Sw => {
            store::sw(cpu, mem, inst);
            false
        }

        // Branches (modify PC themselves)
        Opcode::Beq => {
            branch::beq(cpu, inst);
            true
        }
        Opcode::Bne => {
            branch::bne(cpu, inst);
            true
        }
        Opcode::Blt => {
            branch::blt(cpu, inst);
            true
        }
        Opcode::Bge => {
            branch::bge(cpu, inst);
            true
        }
        Opcode::Bltu => {
            branch::bltu(cpu, inst);
            true
        }
        Opcode::Bgeu => {
            branch::bgeu(cpu, inst);
            true
        }

        // Jumps (modify PC themselves)
        Opcode::Jal => {
            jump::jal(cpu, inst);
            true
        }
        Opcode::Jalr => {
            jump::jalr(cpu, inst);
            true
        }

        // Upper immediates
        Opcode::Lui => {
            upper::lui(cpu, inst);
            false
        }
        Opcode::Auipc => {
            upper::auipc(cpu, inst);
            false
        }

        // M-extension
        Opcode::Mul => {
            muldiv::mul(cpu, inst);
            false
        }
        Opcode::Mulh => {
            muldiv::mulh(cpu, inst);
            false
        }
        Opcode::Mulhsu => {
            muldiv::mulhsu(cpu, inst);
            false
        }
        Opcode::Mulhu => {
            muldiv::mulhu(cpu, inst);
            false
        }
        Opcode::Div => {
            muldiv::div(cpu, inst);
            false
        }
        Opcode::Divu => {
            muldiv::divu(cpu, inst);
            false
        }
        Opcode::Rem => {
            muldiv::rem(cpu, inst);
            false
        }
        Opcode::Remu => {
            muldiv::remu(cpu, inst);
            false
        }
    }
}
