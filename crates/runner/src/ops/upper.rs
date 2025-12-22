use crate::{Cpu, DecodedInst};

pub fn lui(cpu: &mut Cpu, inst: &DecodedInst) {
    cpu.set_reg(inst.rd, inst.imm as u32);
}

pub fn auipc(cpu: &mut Cpu, inst: &DecodedInst) {
    let result = cpu.pc.wrapping_add(inst.imm as u32);
    cpu.set_reg(inst.rd, result);
}
