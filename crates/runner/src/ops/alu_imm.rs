use crate::{Cpu, DecodedInst};

pub fn addi(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let result = rs1.wrapping_add(inst.imm as u32);
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn slti(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as i32;
    let result = if rs1 < inst.imm { 1 } else { 0 };
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn sltiu(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let result = if rs1 < (inst.imm as u32) { 1 } else { 0 };
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn xori(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let result = rs1 ^ (inst.imm as u32);
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn ori(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let result = rs1 | (inst.imm as u32);
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn andi(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let result = rs1 & (inst.imm as u32);
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn slli(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let shamt = inst.imm as u32 & 0x1F;
    let result = rs1 << shamt;
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn srli(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let shamt = inst.imm as u32 & 0x1F;
    let result = rs1 >> shamt;
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn srai(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as i32;
    let shamt = inst.imm as u32 & 0x1F;
    let result = (rs1 >> shamt) as u32;
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}
