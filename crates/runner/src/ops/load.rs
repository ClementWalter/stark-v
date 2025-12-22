use crate::{Cpu, DecodedInst, Memory};

pub fn lb(cpu: &mut Cpu, mem: &Memory, inst: &DecodedInst) {
    let base = cpu.reg(inst.rs1);
    let addr = base.wrapping_add(inst.imm as u32);
    let value = mem.read_u8(addr) as i8 as i32 as u32; // Sign-extend
    cpu.set_reg(inst.rd, value);
}

pub fn lh(cpu: &mut Cpu, mem: &Memory, inst: &DecodedInst) {
    let base = cpu.reg(inst.rs1);
    let addr = base.wrapping_add(inst.imm as u32);
    let value = mem.read_u16(addr) as i16 as i32 as u32; // Sign-extend
    cpu.set_reg(inst.rd, value);
}

pub fn lw(cpu: &mut Cpu, mem: &Memory, inst: &DecodedInst) {
    let base = cpu.reg(inst.rs1);
    let addr = base.wrapping_add(inst.imm as u32);
    let value = mem.read_u32(addr);
    cpu.set_reg(inst.rd, value);
}

pub fn lbu(cpu: &mut Cpu, mem: &Memory, inst: &DecodedInst) {
    let base = cpu.reg(inst.rs1);
    let addr = base.wrapping_add(inst.imm as u32);
    let value = mem.read_u8(addr) as u32; // Zero-extend
    cpu.set_reg(inst.rd, value);
}

pub fn lhu(cpu: &mut Cpu, mem: &Memory, inst: &DecodedInst) {
    let base = cpu.reg(inst.rs1);
    let addr = base.wrapping_add(inst.imm as u32);
    let value = mem.read_u16(addr) as u32; // Zero-extend
    cpu.set_reg(inst.rd, value);
}
