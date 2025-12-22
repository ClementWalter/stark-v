use crate::{Cpu, DecodedInst};

pub fn beq(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    if rs1 == rs2 {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
}

pub fn bne(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    if rs1 != rs2 {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
}

pub fn blt(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as i32;
    let rs2 = cpu.reg(inst.rs2) as i32;
    if rs1 < rs2 {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
}

pub fn bge(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as i32;
    let rs2 = cpu.reg(inst.rs2) as i32;
    if rs1 >= rs2 {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
}

pub fn bltu(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    if rs1 < rs2 {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
}

pub fn bgeu(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    if rs1 >= rs2 {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
}
