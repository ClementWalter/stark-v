use crate::{Cpu, DecodedInst};

pub fn jal(cpu: &mut Cpu, inst: &DecodedInst) {
    let return_addr = cpu.pc.wrapping_add(4);
    cpu.set_reg(inst.rd, return_addr);
    cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
}

pub fn jalr(cpu: &mut Cpu, inst: &DecodedInst) {
    let return_addr = cpu.pc.wrapping_add(4);
    let target = cpu.reg(inst.rs1).wrapping_add(inst.imm as u32) & !1; // Clear LSB
    cpu.set_reg(inst.rd, return_addr);
    cpu.pc = target;
}
