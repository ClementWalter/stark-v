use crate::trace::Tracer;
use crate::{trace_op, traced, Cpu, DecodedInst};

#[traced]
pub fn jal(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let return_addr = cpu.pc.wrapping_add(4);
    let rd = cpu.write_reg(inst.rd, return_addr, tracer);
    cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    trace_op!(rd);
}

#[traced]
pub fn jalr(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let return_addr = cpu.pc.wrapping_add(4);
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let target = rs1.next.wrapping_add(inst.imm as u32) & !1; // Clear LSB
    let rd = cpu.write_reg(inst.rd, return_addr, tracer);
    cpu.pc = target;
    trace_op!(rd, rs1);
}
