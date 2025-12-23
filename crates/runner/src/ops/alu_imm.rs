use crate::trace::Tracer;
use crate::{Cpu, DecodedInst, traced};

#[traced]
pub fn addi(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let result = rs1.next.wrapping_add(inst.imm as u32);
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1);
}

#[traced]
pub fn slti(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let result = if (rs1.next as i32) < inst.imm { 1 } else { 0 };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1);
}

#[traced]
pub fn sltiu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let result = if rs1.next < (inst.imm as u32) { 1 } else { 0 };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1);
}

#[traced]
pub fn xori(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let result = rs1.next ^ (inst.imm as u32);
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1);
}

#[traced]
pub fn ori(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let result = rs1.next | (inst.imm as u32);
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1);
}

#[traced]
pub fn andi(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let result = rs1.next & (inst.imm as u32);
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1);
}

#[traced]
pub fn slli(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let shamt = inst.imm as u32 & 0x1F;
    let result = rs1.next << shamt;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1);
}

#[traced]
pub fn srli(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let shamt = inst.imm as u32 & 0x1F;
    let result = rs1.next >> shamt;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1);
}

#[traced]
pub fn srai(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let shamt = inst.imm as u32 & 0x1F;
    let result = ((rs1.next as i32) >> shamt) as u32;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1);
}
