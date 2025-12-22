use crate::{Cpu, DecodedInst};

pub fn mul(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as i32 as i64;
    let rs2 = cpu.reg(inst.rs2) as i32 as i64;
    let result = rs1.wrapping_mul(rs2) as u32;
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn mulh(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as i32 as i64;
    let rs2 = cpu.reg(inst.rs2) as i32 as i64;
    let result = (rs1.wrapping_mul(rs2) >> 32) as u32;
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn mulhsu(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as i32 as i64;
    let rs2 = cpu.reg(inst.rs2) as u64 as i64;
    let result = (rs1.wrapping_mul(rs2) >> 32) as u32;
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn mulhu(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as u64;
    let rs2 = cpu.reg(inst.rs2) as u64;
    let result = (rs1.wrapping_mul(rs2) >> 32) as u32;
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn div(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as i32;
    let rs2 = cpu.reg(inst.rs2) as i32;
    let result = if rs2 == 0 {
        u32::MAX // Division by zero returns -1
    } else if rs1 == i32::MIN && rs2 == -1 {
        rs1 as u32 // Overflow case
    } else {
        rs1.wrapping_div(rs2) as u32
    };
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn divu(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    let result = if rs2 == 0 {
        u32::MAX // Division by zero returns max value
    } else {
        rs1.wrapping_div(rs2)
    };
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn rem(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1) as i32;
    let rs2 = cpu.reg(inst.rs2) as i32;
    let result = if rs2 == 0 {
        rs1 as u32 // Division by zero returns dividend
    } else if rs1 == i32::MIN && rs2 == -1 {
        0 // Overflow case
    } else {
        rs1.wrapping_rem(rs2) as u32
    };
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}

pub fn remu(cpu: &mut Cpu, inst: &DecodedInst) {
    let rs1 = cpu.reg(inst.rs1);
    let rs2 = cpu.reg(inst.rs2);
    let result = if rs2 == 0 {
        rs1 // Division by zero returns dividend
    } else {
        rs1.wrapping_rem(rs2)
    };
    cpu.set_reg(inst.rd, result);
    cpu.advance_pc();
}
