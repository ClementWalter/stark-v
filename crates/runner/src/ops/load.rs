use crate::trace::Tracer;
use crate::{trace_op, traced, Cpu, DecodedInst, Memory};

#[traced]
pub fn lb(cpu: &mut Cpu, memory: &Memory, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let addr = rs1.next.wrapping_add(inst.imm as u32);
    let mem = memory.read_u8_traced(addr, tracer);
    let value = mem.next as u8 as i8 as i32 as u32; // Sign-extend
    let rd = cpu.write_reg(inst.rd, value, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, mem);
}

#[traced]
pub fn lh(cpu: &mut Cpu, memory: &Memory, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let addr = rs1.next.wrapping_add(inst.imm as u32);
    let mem = memory.read_u16_traced(addr, tracer);
    let value = mem.next as u16 as i16 as i32 as u32; // Sign-extend
    let rd = cpu.write_reg(inst.rd, value, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, mem);
}

#[traced]
pub fn lw(cpu: &mut Cpu, memory: &Memory, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let addr = rs1.next.wrapping_add(inst.imm as u32);
    let mem = memory.read_u32_traced(addr, tracer);
    let value = mem.next;
    let rd = cpu.write_reg(inst.rd, value, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, mem);
}

#[traced]
pub fn lbu(cpu: &mut Cpu, memory: &Memory, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let addr = rs1.next.wrapping_add(inst.imm as u32);
    let mem = memory.read_u8_traced(addr, tracer);
    let value = mem.next; // Zero-extend (already u32)
    let rd = cpu.write_reg(inst.rd, value, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, mem);
}

#[traced]
pub fn lhu(cpu: &mut Cpu, memory: &Memory, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let addr = rs1.next.wrapping_add(inst.imm as u32);
    let mem = memory.read_u16_traced(addr, tracer);
    let value = mem.next; // Zero-extend (already u32)
    let rd = cpu.write_reg(inst.rd, value, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, mem);
}
