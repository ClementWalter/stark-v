use crate::{Cpu, DecodedInst, Memory};

pub fn sb(cpu: &mut Cpu, mem: &mut Memory, inst: &DecodedInst) {
    let base = cpu.reg(inst.rs1);
    let addr = base.wrapping_add(inst.imm as u32);
    let value = cpu.reg(inst.rs2) as u8;
    mem.write_u8(addr, value);
    cpu.advance_pc();
}

pub fn sh(cpu: &mut Cpu, mem: &mut Memory, inst: &DecodedInst) {
    let base = cpu.reg(inst.rs1);
    let addr = base.wrapping_add(inst.imm as u32);
    let value = cpu.reg(inst.rs2) as u16;
    mem.write_u16(addr, value);
    cpu.advance_pc();
}

pub fn sw(cpu: &mut Cpu, mem: &mut Memory, inst: &DecodedInst) {
    let base = cpu.reg(inst.rs1);
    let addr = base.wrapping_add(inst.imm as u32);
    let value = cpu.reg(inst.rs2);
    mem.write_u32(addr, value);
    cpu.advance_pc();
}
