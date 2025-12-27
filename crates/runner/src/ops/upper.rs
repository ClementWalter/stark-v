use crate::trace::Tracer;
use crate::{Cpu, DecodedInst};

pub fn lui(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rd = cpu.write_reg(inst.rd, inst.imm as u32, tracer);
    cpu.advance_pc();
    trace_op!(lui: tracer, cpu.pc, rd);
}

pub fn auipc(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let result = cpu.pc.wrapping_add(inst.imm as u32);
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(auipc: tracer, cpu.pc, rd);
}
