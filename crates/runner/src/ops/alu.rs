use crate::{Cpu, DecodedInst};

pub fn add(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    let result = rs1.wrapping_add(rs2);
    cpu.set_reg(inst.rd, result);
}

pub fn sub(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    let result = rs1.wrapping_sub(rs2);
    cpu.set_reg(inst.rd, result);
}

pub fn sll(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    let shamt = rs2 & 0x1F;
    let result = rs1 << shamt;
    cpu.set_reg(inst.rd, result);
}

pub fn slt(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as i32;
    let rs2 = cpu.reg(inst.rs2) as i32;
    let result = if rs1 < rs2 { 1 } else { 0 };
    cpu.set_reg(inst.rd, result);
}

pub fn sltu(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    let result = if rs1 < rs2 { 1 } else { 0 };
    cpu.set_reg(inst.rd, result);
}

pub fn xor(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    let result = rs1 ^ rs2;
    cpu.set_reg(inst.rd, result);
}

pub fn srl(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    let shamt = rs2 & 0x1F;
    let result = rs1 >> shamt;
    cpu.set_reg(inst.rd, result);
}

pub fn sra(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as i32;
    let rs2 = cpu.reg(inst.rs2);
    let shamt = rs2 & 0x1F;
    let result = (rs1 >> shamt) as u32;
    cpu.set_reg(inst.rd, result);
}

pub fn or(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    let result = rs1 | rs2;
    cpu.set_reg(inst.rd, result);
}

pub fn and(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    let result = rs1 & rs2;
    cpu.set_reg(inst.rd, result);
}
