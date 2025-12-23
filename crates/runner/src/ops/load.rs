use crate::trace::Tracer;
use crate::{Cpu, DecodedInst, Memory, traced};

/// Extract a byte from a u32 word at the given byte offset (0-3).
#[inline]
fn extract_byte(word: u32, offset: u32) -> u8 {
    (word >> (8 * (offset & 3))) as u8
}

/// Extract a half-word from a u32 word at the given byte offset (0 or 2).
#[inline]
fn extract_halfword(word: u32, offset: u32) -> u16 {
    (word >> (8 * (offset & 2))) as u16
}

#[traced]
pub fn lb(cpu: &mut Cpu, memory: &Memory, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let addr = rs1.next.wrapping_add(inst.imm as u32);
    let mem = memory.read_u8_traced(addr, tracer);
    // Extract the specific byte from the aligned word
    let byte = extract_byte(mem.next, addr);
    let value = byte as i8 as i32 as u32; // Sign-extend
    let rd = cpu.write_reg(inst.rd, value, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, mem);
}

#[traced]
pub fn lh(cpu: &mut Cpu, memory: &Memory, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let addr = rs1.next.wrapping_add(inst.imm as u32);
    let mem = memory.read_u16_traced(addr, tracer);
    // Extract the specific half-word from the aligned word
    let halfword = extract_halfword(mem.next, addr);
    let value = halfword as i16 as i32 as u32; // Sign-extend
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
    // Extract the specific byte from the aligned word
    let value = extract_byte(mem.next, addr) as u32; // Zero-extend
    let rd = cpu.write_reg(inst.rd, value, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, mem);
}

#[traced]
pub fn lhu(cpu: &mut Cpu, memory: &Memory, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let addr = rs1.next.wrapping_add(inst.imm as u32);
    let mem = memory.read_u16_traced(addr, tracer);
    // Extract the specific half-word from the aligned word
    let value = extract_halfword(mem.next, addr) as u32; // Zero-extend
    let rd = cpu.write_reg(inst.rd, value, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, mem);
}
