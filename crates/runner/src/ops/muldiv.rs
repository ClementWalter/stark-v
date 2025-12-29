//! Multiply/Divide operations (M extension).
//!
//! This file contains:
//! - mul family: mul (airs.md Section 14)
//! - mulh family: mulh, mulhsu, mulhu (airs.md Section 15)
//! - div family: div, divu, rem, remu (airs.md Section 16)

use crate::trace::Tracer;
use crate::{Cpu, DecodedInst};

// =============================================================================
// MUL - airs.md Section 14
// =============================================================================

pub fn mul(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let rs1_val = rs1.next as i32 as i64;
    let rs2_val = rs2.next as i32 as i64;
    let result = rs1_val.wrapping_mul(rs2_val) as u32;
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();
    trace_op!(mul: tracer, cpu.pc, rd, rs1, rs2);
}

// =============================================================================
// MULH (mulh/mulhsu/mulhu) - airs.md Section 15
// =============================================================================

/// Helper to compute 64-bit multiplication high word and witness columns
fn compute_mulh_witness(
    rs1_val: u32,
    rs2_val: u32,
    rs1_signed: bool,
    rs2_signed: bool,
) -> MulhWitness {
    let a = if rs1_signed {
        rs1_val as i32 as i64
    } else {
        rs1_val as u64 as i64
    };
    let b = if rs2_signed {
        rs2_val as i32 as i64
    } else {
        rs2_val as u64 as i64
    };
    let product = a.wrapping_mul(b);
    let lo = product as u32;
    let hi = (product >> 32) as u32;

    // rd_high is the full 64-bit result split into 8 bytes
    let rd_high = [
        (lo & 0xFF) as u32,
        ((lo >> 8) & 0xFF) as u32,
        ((lo >> 16) & 0xFF) as u32,
        ((lo >> 24) & 0xFF) as u32,
    ];

    let rs1_sign = if rs1_signed { (rs1_val >> 31) & 1 } else { 0 };
    let rs2_sign = if rs2_signed { (rs2_val >> 31) & 1 } else { 0 };

    MulhWitness {
        hi,
        rd_high,
        rs1_sign,
        rs2_sign,
    }
}

struct MulhWitness {
    hi: u32,
    rd_high: [u32; 4],
    rs1_sign: u32,
    rs2_sign: u32,
}

pub fn mulh(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let w = compute_mulh_witness(rs1.next, rs2.next, true, true);
    let rd = cpu.write_reg(inst.rd, w.hi, tracer);
    cpu.advance_pc();

    // opcode flags: mulh=1, mulhsu=0, mulhu=0
    trace_op!(mulh: tracer, cpu.pc, rd, rs1, rs2,
        w.rd_high[0], w.rd_high[1], w.rd_high[2], w.rd_high[3],
        w.rs1_sign, w.rs2_sign,
        1, 0, 0
    );
}

pub fn mulhsu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let w = compute_mulh_witness(rs1.next, rs2.next, true, false);
    let rd = cpu.write_reg(inst.rd, w.hi, tracer);
    cpu.advance_pc();

    // opcode flags: mulh=0, mulhsu=1, mulhu=0
    trace_op!(mulh: tracer, cpu.pc, rd, rs1, rs2,
        w.rd_high[0], w.rd_high[1], w.rd_high[2], w.rd_high[3],
        w.rs1_sign, w.rs2_sign,
        0, 1, 0
    );
}

pub fn mulhu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let w = compute_mulh_witness(rs1.next, rs2.next, false, false);
    let rd = cpu.write_reg(inst.rd, w.hi, tracer);
    cpu.advance_pc();

    // opcode flags: mulh=0, mulhsu=0, mulhu=1
    trace_op!(mulh: tracer, cpu.pc, rd, rs1, rs2,
        w.rd_high[0], w.rd_high[1], w.rd_high[2], w.rd_high[3],
        w.rs1_sign, w.rs2_sign,
        0, 0, 1
    );
}

// =============================================================================
// DIV (div/divu/rem/remu) - airs.md Section 16
// =============================================================================

/// Compute division witness columns
fn compute_div_witness(rs1_val: u32, rs2_val: u32, is_signed: bool) -> DivWitness {
    let zero_divisor = if rs2_val == 0 { 1 } else { 0 };

    let (q, r, b_sign, c_sign, q_sign, sign_xor);

    if is_signed {
        let a = rs1_val as i32;
        let b = rs2_val as i32;
        b_sign = if a < 0 { 1 } else { 0 };
        c_sign = if b < 0 { 1 } else { 0 };

        if b == 0 {
            q = u32::MAX;
            r = rs1_val;
            q_sign = 1; // -1 is negative
            sign_xor = 0;
        } else if a == i32::MIN && b == -1 {
            q = a as u32; // Overflow: result is MIN
            r = 0;
            q_sign = 1;
            sign_xor = 0;
        } else {
            let quot = a.wrapping_div(b);
            let rem = a.wrapping_rem(b);
            q = quot as u32;
            r = rem as u32;
            q_sign = if quot < 0 { 1 } else { 0 };
            sign_xor = b_sign ^ c_sign;
        }
    } else {
        b_sign = 0;
        c_sign = 0;
        if rs2_val == 0 {
            q = u32::MAX;
            r = rs1_val;
        } else {
            q = rs1_val.wrapping_div(rs2_val);
            r = rs1_val.wrapping_rem(rs2_val);
        }
        q_sign = 0;
        sign_xor = 0;
    };

    let r_zero = if r == 0 { 1 } else { 0 };

    // Compute limbs for q and r
    let q_limbs = [
        (q & 0xFF) as u32,
        ((q >> 8) & 0xFF) as u32,
        ((q >> 16) & 0xFF) as u32,
        ((q >> 24) & 0xFF) as u32,
    ];
    let r_limbs = [
        (r & 0xFF) as u32,
        ((r >> 8) & 0xFF) as u32,
        ((r >> 16) & 0xFF) as u32,
        ((r >> 24) & 0xFF) as u32,
    ];

    // For the less-than check: r < c (divisor)
    // We need to find the first differing byte
    let c = rs2_val;
    let c_bytes = c.to_le_bytes();
    let r_bytes = r.to_le_bytes();

    let mut lt_marker = [0u32; 4];
    let mut lt_diff = 0u32;
    for i in (0..4).rev() {
        if r_bytes[i] != c_bytes[i] {
            lt_marker[i] = 1;
            lt_diff = if c_bytes[i] > r_bytes[i] {
                c_bytes[i] as u32 - r_bytes[i] as u32
            } else {
                r_bytes[i] as u32 - c_bytes[i] as u32
            };
            break;
        }
    }

    // Inverses for non-zero checks
    let c_sum: u32 = c_bytes.iter().map(|&x| x as u32).sum();
    let c_sum_inv = if c_sum == 0 { 0 } else { 1 }; // Simplified witness

    let r_sum: u32 = r_bytes.iter().map(|&x| x as u32).sum();
    let r_sum_inv = if r_sum == 0 { 0 } else { 1 }; // Simplified witness

    // r_abs and r_inv for signed remainder
    let r_abs = r_limbs; // For unsigned, r_abs = r
    let r_inv = [0u32; 4]; // Placeholder

    DivWitness {
        zero_divisor,
        r_zero,
        q: q_limbs,
        r: r_limbs,
        b_sign,
        c_sign,
        q_sign,
        sign_xor,
        c_sum_inv,
        r_sum_inv,
        r_abs,
        r_inv,
        lt_marker,
        lt_diff,
    }
}

struct DivWitness {
    zero_divisor: u32,
    r_zero: u32,
    q: [u32; 4],
    r: [u32; 4],
    b_sign: u32,
    c_sign: u32,
    q_sign: u32,
    sign_xor: u32,
    c_sum_inv: u32,
    r_sum_inv: u32,
    r_abs: [u32; 4],
    r_inv: [u32; 4],
    lt_marker: [u32; 4],
    lt_diff: u32,
}

pub fn div(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let rs1_val = rs1.next as i32;
    let rs2_val = rs2.next as i32;
    let result = if rs2_val == 0 {
        u32::MAX
    } else if rs1_val == i32::MIN && rs2_val == -1 {
        rs1_val as u32
    } else {
        rs1_val.wrapping_div(rs2_val) as u32
    };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();

    let w = compute_div_witness(rs1.next, rs2.next, true);

    // opcode flags: div=1, divu=0, rem=0, remu=0
    trace_op!(div: tracer, cpu.pc, rd, rs1, rs2,
        w.zero_divisor, w.r_zero,
        w.q[0], w.q[1], w.q[2], w.q[3],
        w.r[0], w.r[1], w.r[2], w.r[3],
        w.b_sign, w.c_sign, w.q_sign, w.sign_xor,
        w.c_sum_inv, w.r_sum_inv,
        w.r_abs[0], w.r_abs[1], w.r_abs[2], w.r_abs[3],
        w.r_inv[0], w.r_inv[1], w.r_inv[2], w.r_inv[3],
        w.lt_marker[0], w.lt_marker[1], w.lt_marker[2], w.lt_marker[3],
        w.lt_diff,
        1, 0, 0, 0
    );
}

pub fn divu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = if rs2.next == 0 {
        u32::MAX
    } else {
        rs1.next.wrapping_div(rs2.next)
    };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();

    let w = compute_div_witness(rs1.next, rs2.next, false);

    // opcode flags: div=0, divu=1, rem=0, remu=0
    trace_op!(div: tracer, cpu.pc, rd, rs1, rs2,
        w.zero_divisor, w.r_zero,
        w.q[0], w.q[1], w.q[2], w.q[3],
        w.r[0], w.r[1], w.r[2], w.r[3],
        w.b_sign, w.c_sign, w.q_sign, w.sign_xor,
        w.c_sum_inv, w.r_sum_inv,
        w.r_abs[0], w.r_abs[1], w.r_abs[2], w.r_abs[3],
        w.r_inv[0], w.r_inv[1], w.r_inv[2], w.r_inv[3],
        w.lt_marker[0], w.lt_marker[1], w.lt_marker[2], w.lt_marker[3],
        w.lt_diff,
        0, 1, 0, 0
    );
}

pub fn rem(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let rs1_val = rs1.next as i32;
    let rs2_val = rs2.next as i32;
    let result = if rs2_val == 0 {
        rs1_val as u32
    } else if rs1_val == i32::MIN && rs2_val == -1 {
        0
    } else {
        rs1_val.wrapping_rem(rs2_val) as u32
    };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();

    let w = compute_div_witness(rs1.next, rs2.next, true);

    // opcode flags: div=0, divu=0, rem=1, remu=0
    trace_op!(div: tracer, cpu.pc, rd, rs1, rs2,
        w.zero_divisor, w.r_zero,
        w.q[0], w.q[1], w.q[2], w.q[3],
        w.r[0], w.r[1], w.r[2], w.r[3],
        w.b_sign, w.c_sign, w.q_sign, w.sign_xor,
        w.c_sum_inv, w.r_sum_inv,
        w.r_abs[0], w.r_abs[1], w.r_abs[2], w.r_abs[3],
        w.r_inv[0], w.r_inv[1], w.r_inv[2], w.r_inv[3],
        w.lt_marker[0], w.lt_marker[1], w.lt_marker[2], w.lt_marker[3],
        w.lt_diff,
        0, 0, 1, 0
    );
}

pub fn remu(cpu: &mut Cpu, inst: &DecodedInst, tracer: &mut Tracer) {
    let rs1 = cpu.read_reg(inst.rs1, tracer);
    let rs2 = cpu.read_reg(inst.rs2, tracer);
    let result = if rs2.next == 0 {
        rs1.next
    } else {
        rs1.next.wrapping_rem(rs2.next)
    };
    let rd = cpu.write_reg(inst.rd, result, tracer);
    cpu.advance_pc();

    let w = compute_div_witness(rs1.next, rs2.next, false);

    // opcode flags: div=0, divu=0, rem=0, remu=1
    trace_op!(div: tracer, cpu.pc, rd, rs1, rs2,
        w.zero_divisor, w.r_zero,
        w.q[0], w.q[1], w.q[2], w.q[3],
        w.r[0], w.r[1], w.r[2], w.r[3],
        w.b_sign, w.c_sign, w.q_sign, w.sign_xor,
        w.c_sum_inv, w.r_sum_inv,
        w.r_abs[0], w.r_abs[1], w.r_abs[2], w.r_abs[3],
        w.r_inv[0], w.r_inv[1], w.r_inv[2], w.r_inv[3],
        w.lt_marker[0], w.lt_marker[1], w.lt_marker[2], w.lt_marker[3],
        w.lt_diff,
        0, 0, 0, 1
    );
}
