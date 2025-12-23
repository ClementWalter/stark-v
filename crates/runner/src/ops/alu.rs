use crate::trace::Tracer;
use crate::{trace_op, traced, Cpu, DecodedInst};

#[traced]
pub fn add(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = rs1.next.wrapping_add(rs2.next);
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn sub(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = rs1.next.wrapping_sub(rs2.next);
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn sll(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let shamt = rs2.next & 0x1F;
    let result = rs1.next << shamt;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn slt(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = if (rs1.next as i32) < (rs2.next as i32) {
        1
    } else {
        0
    };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn sltu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = if rs1.next < rs2.next { 1 } else { 0 };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn xor(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = rs1.next ^ rs2.next;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn srl(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let shamt = rs2.next & 0x1F;
    let result = rs1.next >> shamt;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn sra(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let shamt = rs2.next & 0x1F;
    let result = ((rs1.next as i32) >> shamt) as u32;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn or(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = rs1.next | rs2.next;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn and(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = rs1.next & rs2.next;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}
