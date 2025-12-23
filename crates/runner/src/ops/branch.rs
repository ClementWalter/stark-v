use crate::trace::Tracer;
use crate::{trace_op, traced, Cpu, DecodedInst};

#[traced]
pub fn beq(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    if rs1.next == rs2.next {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
    trace_op!(rs1, rs2);
}

#[traced]
pub fn bne(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    if rs1.next != rs2.next {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
    trace_op!(rs1, rs2);
}

#[traced]
pub fn blt(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    if (rs1.next as i32) < (rs2.next as i32) {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
    trace_op!(rs1, rs2);
}

#[traced]
pub fn bge(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    if (rs1.next as i32) >= (rs2.next as i32) {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
    trace_op!(rs1, rs2);
}

#[traced]
pub fn bltu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    if rs1.next < rs2.next {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
    trace_op!(rs1, rs2);
}

#[traced]
pub fn bgeu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    if rs1.next >= rs2.next {
        cpu.pc = cpu.pc.wrapping_add(inst.imm as u32);
    } else {
        cpu.advance_pc();
    }
    trace_op!(rs1, rs2);
}
