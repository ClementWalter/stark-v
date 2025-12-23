use crate::trace::Tracer;
use crate::{Cpu, DecodedInst, Memory, traced};

#[traced]
pub fn sb(cpu: &mut Cpu, memory: &mut Memory, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let addr = rs1.next.wrapping_add(inst.imm as u32);
    let value = rs2.next as u8;
    let mem = memory.write_u8_traced(addr, value, tracer);
    cpu.advance_pc();
    trace_op!(rs1, rs2, mem);
}

#[traced]
pub fn sh(cpu: &mut Cpu, memory: &mut Memory, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let addr = rs1.next.wrapping_add(inst.imm as u32);
    let value = rs2.next as u16;
    let mem = memory.write_u16_traced(addr, value, tracer);
    cpu.advance_pc();
    trace_op!(rs1, rs2, mem);
}

#[traced]
pub fn sw(cpu: &mut Cpu, memory: &mut Memory, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let addr = rs1.next.wrapping_add(inst.imm as u32);
    let value = rs2.next;
    let mem = memory.write_u32_traced(addr, value, tracer);
    cpu.advance_pc();
    trace_op!(rs1, rs2, mem);
}
