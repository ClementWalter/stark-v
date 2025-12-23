use crate::trace::Tracer;
use crate::{trace_op, traced, Cpu, DecodedInst};

#[traced]
pub fn mul(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let rs1_val = rs1.next as i32 as i64;
    let rs2_val = rs2.next as i32 as i64;
    let result = rs1_val.wrapping_mul(rs2_val) as u32;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn mulh(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let rs1_val = rs1.next as i32 as i64;
    let rs2_val = rs2.next as i32 as i64;
    let result = (rs1_val.wrapping_mul(rs2_val) >> 32) as u32;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn mulhsu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let rs1_val = rs1.next as i32 as i64;
    let rs2_val = rs2.next as u64 as i64;
    let result = (rs1_val.wrapping_mul(rs2_val) >> 32) as u32;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn mulhu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let rs1_val = rs1.next as u64;
    let rs2_val = rs2.next as u64;
    let result = (rs1_val.wrapping_mul(rs2_val) >> 32) as u32;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn div(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let rs1_val = rs1.next as i32;
    let rs2_val = rs2.next as i32;
    let result = if rs2_val == 0 {
        u32::MAX // Division by zero returns -1
    } else if rs1_val == i32::MIN && rs2_val == -1 {
        rs1_val as u32 // Overflow case
    } else {
        rs1_val.wrapping_div(rs2_val) as u32
    };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn divu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = if rs2.next == 0 {
        u32::MAX // Division by zero returns max value
    } else {
        rs1.next.wrapping_div(rs2.next)
    };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn rem(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let rs1_val = rs1.next as i32;
    let rs2_val = rs2.next as i32;
    let result = if rs2_val == 0 {
        rs1_val as u32 // Division by zero returns dividend
    } else if rs1_val == i32::MIN && rs2_val == -1 {
        0 // Overflow case
    } else {
        rs1_val.wrapping_rem(rs2_val) as u32
    };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}

#[traced]
pub fn remu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = if rs2.next == 0 {
        rs1.next // Division by zero returns dividend
    } else {
        rs1.next.wrapping_rem(rs2.next)
    };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(rd, rs1, rs2);
}
