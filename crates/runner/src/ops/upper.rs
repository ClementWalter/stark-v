use crate::trace::Tracer;
use crate::{trace_op, traced, Cpu, DecodedInst};

#[traced]
pub fn lui(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rd = cpu.write_reg(inst.rd, inst.imm as u32, tracer);
    cpu.advance_pc();
    trace_op!(rd);
}

#[traced]
pub fn auipc(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let result = cpu.pc.wrapping_add(inst.imm as u32);
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd);
}
