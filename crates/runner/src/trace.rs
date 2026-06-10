#![allow(clippy::too_many_arguments)]
//! Trace capture for zkVM execution.
//!
//! Each opcode defines its own columnar trace table.
//! Registers and memory use a unified Access structure that gets flattened into columns.

use simd::AlignedVec;

/// Default maximum clock difference allowed between accesses.
/// Must be consistent with max range-check in the prover.
/// RangeCheck20 is an array of from 0 to u20::MAX, i.e. to 2^20 - 1.
pub const DEFAULT_MAX_CLOCK_DIFF: u32 = (1 << 20) - 1;

// =============================================================================
// Generate all trace tables, Tracer struct, and trace_op! macro
// =============================================================================

stwo_macros::define_trace_tables! {
    // ==========================================================================
    // 1. Base ALU Reg (add/sub/xor/or/and) - airs.md Section 1
    // ==========================================================================
    base_alu_reg: {
        clock, pc, rd, rs1, rs2,
        opcode_add_flag, opcode_sub_flag, opcode_xor_flag, opcode_or_flag, opcode_and_flag,
        derived: {
            expected_opcode_id: |opcode_add_flag, opcode_sub_flag, opcode_xor_flag,
                opcode_or_flag, opcode_and_flag|
                opcode_add_flag * constant(crate::decode::Opcode::Add as u32)
                + opcode_sub_flag * constant(crate::decode::Opcode::Sub as u32)
                + opcode_xor_flag * constant(crate::decode::Opcode::Xor as u32)
                + opcode_or_flag * constant(crate::decode::Opcode::Or as u32)
                + opcode_and_flag * constant(crate::decode::Opcode::And as u32),
            is_bitwise: |opcode_xor_flag, opcode_or_flag, opcode_and_flag|
                opcode_xor_flag + opcode_or_flag + opcode_and_flag,
            // Preprocessed bitwise table id: and=0, or=1, xor=2
            bitwise_id: |opcode_xor_flag, opcode_or_flag| 2 * opcode_xor_flag + opcode_or_flag,
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rs2_clock_diff: |clock, rs2_clock_prev| clock - rs2_clock_prev,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
            // Carry chains of rd = rs1 + rs2 and rs1 = rd + rs2 over 8-bit
            // limbs; each carry is 0 or 1 under the active opcode
            carry_add_0: |rs1_next_0, rs2_next_0, rd_next_0|
                (rs1_next_0 + rs2_next_0 - rd_next_0) * inv(pow2(8)),
            carry_add_1: |rs1_next_1, rs2_next_1, rd_next_1, carry_add_0|
                (rs1_next_1 + rs2_next_1 + carry_add_0 - rd_next_1) * inv(pow2(8)),
            carry_add_2: |rs1_next_2, rs2_next_2, rd_next_2, carry_add_1|
                (rs1_next_2 + rs2_next_2 + carry_add_1 - rd_next_2) * inv(pow2(8)),
            carry_add_3: |rs1_next_3, rs2_next_3, rd_next_3, carry_add_2|
                (rs1_next_3 + rs2_next_3 + carry_add_2 - rd_next_3) * inv(pow2(8)),
            carry_sub_0: |rd_next_0, rs2_next_0, rs1_next_0|
                (rd_next_0 + rs2_next_0 - rs1_next_0) * inv(pow2(8)),
            carry_sub_1: |rd_next_1, rs2_next_1, rs1_next_1, carry_sub_0|
                (rd_next_1 + rs2_next_1 - rs1_next_1 + carry_sub_0) * inv(pow2(8)),
            carry_sub_2: |rd_next_2, rs2_next_2, rs1_next_2, carry_sub_1|
                (rd_next_2 + rs2_next_2 - rs1_next_2 + carry_sub_1) * inv(pow2(8)),
            carry_sub_3: |rd_next_3, rs2_next_3, rs1_next_3, carry_sub_2|
                (rd_next_3 + rs2_next_3 - rs1_next_3 + carry_sub_2) * inv(pow2(8)),
        },
        constraints: {
            opcode_add_flag * carry_add_0 * (1 - carry_add_0),
            opcode_add_flag * carry_add_1 * (1 - carry_add_1),
            opcode_add_flag * carry_add_2 * (1 - carry_add_2),
            opcode_add_flag * carry_add_3 * (1 - carry_add_3),
            opcode_sub_flag * carry_sub_0 * (1 - carry_sub_0),
            opcode_sub_flag * carry_sub_1 * (1 - carry_sub_1),
            opcode_sub_flag * carry_sub_2 * (1 - carry_sub_2),
            opcode_sub_flag * carry_sub_3 * (1 - carry_sub_3),
        },
        lookups: {
            // Program access (R-type): Program(pc, opcode, rd_idx, rs1_idx, rs2_idx)
            program_access: -enabler => [pc, expected_opcode_id, rd_addr, rs1_addr, rs2_addr],
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // Read rs2.
            memory_access: -enabler =>
                [0, rs2_addr, rs2_clock_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3],
            memory_access: enabler =>
                [0, rs2_addr, clock, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3],
            preprocessed range_check_20: -enabler => [rs2_clock_diff],
            // Bitwise limbs (xor/or/and): Bitwise(rs1[i], rs2[i], rd[i], op id).
            preprocessed bitwise: -is_bitwise => [rs1_next_0, rs2_next_0, rd_next_0, bitwise_id],
            preprocessed bitwise: -is_bitwise => [rs1_next_1, rs2_next_1, rd_next_1, bitwise_id],
            preprocessed bitwise: -is_bitwise => [rs1_next_2, rs2_next_2, rd_next_2, bitwise_id],
            preprocessed bitwise: -is_bitwise => [rs1_next_3, rs2_next_3, rd_next_3, bitwise_id],
            // Write rd.
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler =>
                [0, rd_addr, clock, rd_next_0, rd_next_1, rd_next_2, rd_next_3],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 2. Base ALU Imm (addi/xori/ori/andi) - airs.md Section 2
    // ==========================================================================
    base_alu_imm: {
        clock, pc, rd, rs1,
        imm_0, imm_1, imm_msb,
        opcode_add_flag, opcode_xor_flag, opcode_or_flag, opcode_and_flag,
        derived: {
            // Opcode id encoded in the program segment, selected by the active flag
            expected_opcode_id: |opcode_add_flag, opcode_xor_flag, opcode_or_flag, opcode_and_flag|
                opcode_add_flag * constant(crate::decode::Opcode::Addi as u32)
                + opcode_xor_flag * constant(crate::decode::Opcode::Xori as u32)
                + opcode_or_flag * constant(crate::decode::Opcode::Ori as u32)
                + opcode_and_flag * constant(crate::decode::Opcode::Andi as u32),
            // I-type immediate: imm_0 (8 bits) + imm_1 (3 bits) + sign bit (airs.md 2.2)
            imm: |imm_0, imm_1, imm_msb| imm_0 + pow2(8) * imm_1 + pow2(11) * imm_msb,
            // Sign-extended immediate limbs; limb 0 is imm_0 and limb 3 equals limb 2
            sext_imm_1: |imm_1, imm_msb| imm_1 + ((1 << 3) * ((1 << 5) - 1)) * imm_msb,
            sext_imm_2: |imm_msb| ((1 << 8) - 1) * imm_msb,
            is_bitwise: |opcode_xor_flag, opcode_or_flag, opcode_and_flag|
                opcode_xor_flag + opcode_or_flag + opcode_and_flag,
            // Preprocessed bitwise table id: and=0, or=1, xor=2
            bitwise_id: |opcode_xor_flag, opcode_or_flag| 2 * opcode_xor_flag + opcode_or_flag,
            imm_1_shifted: |imm_1| pow2(8) * imm_1,
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
            // Carry chain of rd = rs1 + sext_imm over 8-bit limbs; each carry is 0 or 1
            carry_0: |rs1_next_0, imm_0, rd_next_0|
                (rs1_next_0 + imm_0 - rd_next_0) * inv(pow2(8)),
            carry_1: |rs1_next_1, sext_imm_1, rd_next_1, carry_0|
                (rs1_next_1 + sext_imm_1 + carry_0 - rd_next_1) * inv(pow2(8)),
            carry_2: |rs1_next_2, sext_imm_2, rd_next_2, carry_1|
                (rs1_next_2 + sext_imm_2 + carry_1 - rd_next_2) * inv(pow2(8)),
            carry_3: |rs1_next_3, sext_imm_2, rd_next_3, carry_2|
                (rs1_next_3 + sext_imm_2 + carry_2 - rd_next_3) * inv(pow2(8)),
        },
        constraints: {
            imm_msb * (1 - imm_msb),
            opcode_add_flag * carry_0 * (1 - carry_0),
            opcode_add_flag * carry_1 * (1 - carry_1),
            opcode_add_flag * carry_2 * (1 - carry_2),
            opcode_add_flag * carry_3 * (1 - carry_3),
        },
        lookups: {
            // Program access (I-type): Program(pc, opcode, rd_idx, rs1_idx, imm)
            program_access: -enabler => [pc, expected_opcode_id, rd_addr, rs1_addr, imm],
            // I-type immediate limb ranges: imm_0 is 8 bits, imm_1 is 3 bits.
            preprocessed range_check_8_11: -enabler => [imm_0, imm_1_shifted],
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // Bitwise limbs (xor/or/and): Bitwise(rs1[i], sext_imm[i], rd[i], op id).
            preprocessed bitwise: -is_bitwise => [rs1_next_0, imm_0, rd_next_0, bitwise_id],
            preprocessed bitwise: -is_bitwise => [rs1_next_1, sext_imm_1, rd_next_1, bitwise_id],
            preprocessed bitwise: -is_bitwise => [rs1_next_2, sext_imm_2, rd_next_2, bitwise_id],
            preprocessed bitwise: -is_bitwise => [rs1_next_3, sext_imm_2, rd_next_3, bitwise_id],
            // rd byte ranges.
            preprocessed range_check_8_8: -enabler => [rd_next_0, rd_next_1],
            preprocessed range_check_8_8: -enabler => [rd_next_2, rd_next_3],
            // Write rd.
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler =>
                [0, rd_addr, clock, rd_next_0, rd_next_1, rd_next_2, rd_next_3],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 3. Shifts Reg (sll/srl/sra) - airs.md Section 3
    // ==========================================================================
    shifts_reg: {
        clock, pc, rd, rs1, rs2,
        rs1_sign,
        opcode_sll_flag, opcode_srl_flag, opcode_sra_flag,
        bit_multiplier_left, bit_multiplier_right,
        bit_shift_marker_0, bit_shift_marker_1, bit_shift_marker_2, bit_shift_marker_3,
        bit_shift_marker_4, bit_shift_marker_5, bit_shift_marker_6, bit_shift_marker_7,
        limb_shift_marker_0, limb_shift_marker_1, limb_shift_marker_2, limb_shift_marker_3,
        bit_shift_carry_0, bit_shift_carry_1, bit_shift_carry_2, bit_shift_carry_3,
        derived: {
            expected_opcode_id: |opcode_sll_flag, opcode_srl_flag, opcode_sra_flag|
                opcode_sll_flag * constant(crate::decode::Opcode::Sll as u32)
                + opcode_srl_flag * constant(crate::decode::Opcode::Srl as u32)
                + opcode_sra_flag * constant(crate::decode::Opcode::Sra as u32),
            right_shift: |opcode_srl_flag, opcode_sra_flag| opcode_srl_flag + opcode_sra_flag,
            // Hot-one decoded shift quantities (airs.md 3.2)
            bit_multiplier: |bit_shift_marker_0, bit_shift_marker_1, bit_shift_marker_2,
                bit_shift_marker_3, bit_shift_marker_4, bit_shift_marker_5, bit_shift_marker_6,
                bit_shift_marker_7|
                bit_shift_marker_0 + 2 * bit_shift_marker_1 + 4 * bit_shift_marker_2
                + 8 * bit_shift_marker_3 + 16 * bit_shift_marker_4 + 32 * bit_shift_marker_5
                + 64 * bit_shift_marker_6 + 128 * bit_shift_marker_7,
            bit_shift: |bit_shift_marker_1, bit_shift_marker_2, bit_shift_marker_3,
                bit_shift_marker_4, bit_shift_marker_5, bit_shift_marker_6, bit_shift_marker_7|
                bit_shift_marker_1 + 2 * bit_shift_marker_2 + 3 * bit_shift_marker_3
                + 4 * bit_shift_marker_4 + 5 * bit_shift_marker_5 + 6 * bit_shift_marker_6
                + 7 * bit_shift_marker_7,
            limb_shift: |limb_shift_marker_1, limb_shift_marker_2, limb_shift_marker_3|
                limb_shift_marker_1 + 2 * limb_shift_marker_2 + 3 * limb_shift_marker_3,
            shift_amount: |limb_shift, bit_shift| pow2(3) * limb_shift + bit_shift,
            bit_marker_sum: |bit_shift_marker_0, bit_shift_marker_1, bit_shift_marker_2,
                bit_shift_marker_3, bit_shift_marker_4, bit_shift_marker_5, bit_shift_marker_6,
                bit_shift_marker_7|
                bit_shift_marker_0 + bit_shift_marker_1 + bit_shift_marker_2 + bit_shift_marker_3
                + bit_shift_marker_4 + bit_shift_marker_5 + bit_shift_marker_6
                + bit_shift_marker_7,
            limb_marker_sum: |limb_shift_marker_0, limb_shift_marker_1, limb_shift_marker_2,
                limb_shift_marker_3|
                limb_shift_marker_0 + limb_shift_marker_1 + limb_shift_marker_2
                + limb_shift_marker_3,
            // Shift amount comes from the low 5 bits of rs2 (airs.md 3.3)
            shift_check: |rs2_next_0, shift_amount| pow2(12) * (rs2_next_0 - shift_amount),
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rs2_clock_diff: |clock, rs2_clock_prev| clock - rs2_clock_prev,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
        },
        constraints: {
            rs1_sign * (1 - rs1_sign),
            bit_shift_marker_0 * (1 - bit_shift_marker_0),
            bit_shift_marker_1 * (1 - bit_shift_marker_1),
            bit_shift_marker_2 * (1 - bit_shift_marker_2),
            bit_shift_marker_3 * (1 - bit_shift_marker_3),
            bit_shift_marker_4 * (1 - bit_shift_marker_4),
            bit_shift_marker_5 * (1 - bit_shift_marker_5),
            bit_shift_marker_6 * (1 - bit_shift_marker_6),
            bit_shift_marker_7 * (1 - bit_shift_marker_7),
            limb_shift_marker_0 * (1 - limb_shift_marker_0),
            limb_shift_marker_1 * (1 - limb_shift_marker_1),
            limb_shift_marker_2 * (1 - limb_shift_marker_2),
            limb_shift_marker_3 * (1 - limb_shift_marker_3),
            // Exactly one bit marker and one limb marker fire on enabled rows
            bit_marker_sum - enabler,
            limb_marker_sum - enabler,
            bit_multiplier_left - opcode_sll_flag * bit_multiplier,
            bit_multiplier_right - right_shift * bit_multiplier,
            // Left shift by 8*i + b: rd limbs below i vanish, limb i carries
            // out, higher limbs chain the carries (airs.md 3.3)
            opcode_sll_flag * limb_shift_marker_0 * (rd_next_0 + pow2(8) * bit_shift_carry_0)
                - limb_shift_marker_0 * rs1_next_0 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_0 * (rd_next_1 - (bit_shift_carry_0 - pow2(8) * bit_shift_carry_1))
                - limb_shift_marker_0 * rs1_next_1 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_0 * (rd_next_2 - (bit_shift_carry_1 - pow2(8) * bit_shift_carry_2))
                - limb_shift_marker_0 * rs1_next_2 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_0 * (rd_next_3 - (bit_shift_carry_2 - pow2(8) * bit_shift_carry_3))
                - limb_shift_marker_0 * rs1_next_3 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_1 * rd_next_0,
            opcode_sll_flag * limb_shift_marker_1 * (rd_next_1 + pow2(8) * bit_shift_carry_0)
                - limb_shift_marker_1 * rs1_next_0 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_1 * (rd_next_2 - (bit_shift_carry_0 - pow2(8) * bit_shift_carry_1))
                - limb_shift_marker_1 * rs1_next_1 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_1 * (rd_next_3 - (bit_shift_carry_1 - pow2(8) * bit_shift_carry_2))
                - limb_shift_marker_1 * rs1_next_2 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_2 * rd_next_0,
            opcode_sll_flag * limb_shift_marker_2 * rd_next_1,
            opcode_sll_flag * limb_shift_marker_2 * (rd_next_2 + pow2(8) * bit_shift_carry_0)
                - limb_shift_marker_2 * rs1_next_0 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_2 * (rd_next_3 - (bit_shift_carry_0 - pow2(8) * bit_shift_carry_1))
                - limb_shift_marker_2 * rs1_next_1 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_3 * rd_next_0,
            opcode_sll_flag * limb_shift_marker_3 * rd_next_1,
            opcode_sll_flag * limb_shift_marker_3 * rd_next_2,
            opcode_sll_flag * limb_shift_marker_3 * (rd_next_3 + pow2(8) * bit_shift_carry_0)
                - limb_shift_marker_3 * rs1_next_0 * bit_multiplier_left,
            // Right shift: high limbs sign-fill, limb 3-i takes the top, lower
            // limbs chain carries downward
            limb_shift_marker_0 * (bit_shift_carry_1 * right_shift * pow2(8)
                + right_shift * (rs1_next_0 - bit_shift_carry_0)
                - rd_next_0 * bit_multiplier_right),
            limb_shift_marker_0 * (bit_shift_carry_2 * right_shift * pow2(8)
                + right_shift * (rs1_next_1 - bit_shift_carry_1)
                - rd_next_1 * bit_multiplier_right),
            limb_shift_marker_0 * (bit_shift_carry_3 * right_shift * pow2(8)
                + right_shift * (rs1_next_2 - bit_shift_carry_2)
                - rd_next_2 * bit_multiplier_right),
            limb_shift_marker_0 * (rs1_sign * (bit_multiplier_right - 1) * pow2(8)
                + right_shift * (rs1_next_3 - bit_shift_carry_3)
                - rd_next_3 * bit_multiplier_right),
            limb_shift_marker_1 * (bit_shift_carry_2 * right_shift * pow2(8)
                + right_shift * (rs1_next_1 - bit_shift_carry_1)
                - rd_next_0 * bit_multiplier_right),
            limb_shift_marker_1 * (bit_shift_carry_3 * right_shift * pow2(8)
                + right_shift * (rs1_next_2 - bit_shift_carry_2)
                - rd_next_1 * bit_multiplier_right),
            limb_shift_marker_1 * (rs1_sign * (bit_multiplier_right - 1) * pow2(8)
                + right_shift * (rs1_next_3 - bit_shift_carry_3)
                - rd_next_2 * bit_multiplier_right),
            right_shift * limb_shift_marker_1 * (rd_next_3 - rs1_sign * (pow2(8) - 1)),
            limb_shift_marker_2 * (bit_shift_carry_3 * right_shift * pow2(8)
                + right_shift * (rs1_next_2 - bit_shift_carry_2)
                - rd_next_0 * bit_multiplier_right),
            limb_shift_marker_2 * (rs1_sign * (bit_multiplier_right - 1) * pow2(8)
                + right_shift * (rs1_next_3 - bit_shift_carry_3)
                - rd_next_1 * bit_multiplier_right),
            right_shift * limb_shift_marker_2 * (rd_next_2 - rs1_sign * (pow2(8) - 1)),
            right_shift * limb_shift_marker_2 * (rd_next_3 - rs1_sign * (pow2(8) - 1)),
            limb_shift_marker_3 * (rs1_sign * (bit_multiplier_right - 1) * pow2(8)
                + right_shift * (rs1_next_3 - bit_shift_carry_3)
                - rd_next_0 * bit_multiplier_right),
            right_shift * limb_shift_marker_3 * (rd_next_1 - rs1_sign * (pow2(8) - 1)),
            right_shift * limb_shift_marker_3 * (rd_next_2 - rs1_sign * (pow2(8) - 1)),
            right_shift * limb_shift_marker_3 * (rd_next_3 - rs1_sign * (pow2(8) - 1)),
        },
        lookups: {
            // Program access (R-type): Program(pc, opcode, rd_idx, rs1_idx, rs2_idx)
            program_access: -enabler => [pc, expected_opcode_id, rd_addr, rs1_addr, rs2_addr],
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // Read rs2.
            memory_access: -enabler =>
                [0, rs2_addr, rs2_clock_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3],
            memory_access: enabler =>
                [0, rs2_addr, clock, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3],
            preprocessed range_check_20: -enabler => [rs2_clock_diff],
            // The decoded shift amount matches rs2's low 5 bits.
            preprocessed range_check_20: -enabler => [shift_check],
            // rd byte ranges.
            preprocessed range_check_8_8: -enabler => [rd_next_0, rd_next_1],
            preprocessed range_check_8_8: -enabler => [rd_next_2, rd_next_3],
            // Write rd.
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler =>
                [0, rd_addr, clock, rd_next_0, rd_next_1, rd_next_2, rd_next_3],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 4. Shifts Imm (slli/srli/srai) - airs.md Section 4
    // ==========================================================================
    shifts_imm: {
        clock, pc, rd, rs1,
        rs1_sign, imm_truncated,
        opcode_sll_flag, opcode_srl_flag, opcode_sra_flag,
        bit_multiplier_left, bit_multiplier_right,
        bit_shift_marker_0, bit_shift_marker_1, bit_shift_marker_2, bit_shift_marker_3,
        bit_shift_marker_4, bit_shift_marker_5, bit_shift_marker_6, bit_shift_marker_7,
        limb_shift_marker_0, limb_shift_marker_1, limb_shift_marker_2, limb_shift_marker_3,
        bit_shift_carry_0, bit_shift_carry_1, bit_shift_carry_2, bit_shift_carry_3,
        derived: {
            expected_opcode_id: |opcode_sll_flag, opcode_srl_flag, opcode_sra_flag|
                opcode_sll_flag * constant(crate::decode::Opcode::Slli as u32)
                + opcode_srl_flag * constant(crate::decode::Opcode::Srli as u32)
                + opcode_sra_flag * constant(crate::decode::Opcode::Srai as u32),
            right_shift: |opcode_srl_flag, opcode_sra_flag| opcode_srl_flag + opcode_sra_flag,
            // Hot-one decoded shift quantities (airs.md 4.2)
            bit_multiplier: |bit_shift_marker_0, bit_shift_marker_1, bit_shift_marker_2,
                bit_shift_marker_3, bit_shift_marker_4, bit_shift_marker_5, bit_shift_marker_6,
                bit_shift_marker_7|
                bit_shift_marker_0 + 2 * bit_shift_marker_1 + 4 * bit_shift_marker_2
                + 8 * bit_shift_marker_3 + 16 * bit_shift_marker_4 + 32 * bit_shift_marker_5
                + 64 * bit_shift_marker_6 + 128 * bit_shift_marker_7,
            bit_shift: |bit_shift_marker_1, bit_shift_marker_2, bit_shift_marker_3,
                bit_shift_marker_4, bit_shift_marker_5, bit_shift_marker_6, bit_shift_marker_7|
                bit_shift_marker_1 + 2 * bit_shift_marker_2 + 3 * bit_shift_marker_3
                + 4 * bit_shift_marker_4 + 5 * bit_shift_marker_5 + 6 * bit_shift_marker_6
                + 7 * bit_shift_marker_7,
            limb_shift: |limb_shift_marker_1, limb_shift_marker_2, limb_shift_marker_3|
                limb_shift_marker_1 + 2 * limb_shift_marker_2 + 3 * limb_shift_marker_3,
            shift_amount: |limb_shift, bit_shift| pow2(3) * limb_shift + bit_shift,
            bit_marker_sum: |bit_shift_marker_0, bit_shift_marker_1, bit_shift_marker_2,
                bit_shift_marker_3, bit_shift_marker_4, bit_shift_marker_5, bit_shift_marker_6,
                bit_shift_marker_7|
                bit_shift_marker_0 + bit_shift_marker_1 + bit_shift_marker_2 + bit_shift_marker_3
                + bit_shift_marker_4 + bit_shift_marker_5 + bit_shift_marker_6
                + bit_shift_marker_7,
            limb_marker_sum: |limb_shift_marker_0, limb_shift_marker_1, limb_shift_marker_2,
                limb_shift_marker_3|
                limb_shift_marker_0 + limb_shift_marker_1 + limb_shift_marker_2
                + limb_shift_marker_3,
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
        },
        constraints: {
            rs1_sign * (1 - rs1_sign),
            bit_shift_marker_0 * (1 - bit_shift_marker_0),
            bit_shift_marker_1 * (1 - bit_shift_marker_1),
            bit_shift_marker_2 * (1 - bit_shift_marker_2),
            bit_shift_marker_3 * (1 - bit_shift_marker_3),
            bit_shift_marker_4 * (1 - bit_shift_marker_4),
            bit_shift_marker_5 * (1 - bit_shift_marker_5),
            bit_shift_marker_6 * (1 - bit_shift_marker_6),
            bit_shift_marker_7 * (1 - bit_shift_marker_7),
            limb_shift_marker_0 * (1 - limb_shift_marker_0),
            limb_shift_marker_1 * (1 - limb_shift_marker_1),
            limb_shift_marker_2 * (1 - limb_shift_marker_2),
            limb_shift_marker_3 * (1 - limb_shift_marker_3),
            bit_marker_sum - enabler,
            limb_marker_sum - enabler,
            bit_multiplier_left - opcode_sll_flag * bit_multiplier,
            bit_multiplier_right - right_shift * bit_multiplier,
            // The immediate encodes the decoded shift amount (airs.md 4.3)
            imm_truncated - shift_amount,
            // Left shift by 8*i + b (airs.md 4.3)
            opcode_sll_flag * limb_shift_marker_0 * (rd_next_0 + pow2(8) * bit_shift_carry_0)
                - limb_shift_marker_0 * rs1_next_0 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_0 * (rd_next_1 - (bit_shift_carry_0 - pow2(8) * bit_shift_carry_1))
                - limb_shift_marker_0 * rs1_next_1 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_0 * (rd_next_2 - (bit_shift_carry_1 - pow2(8) * bit_shift_carry_2))
                - limb_shift_marker_0 * rs1_next_2 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_0 * (rd_next_3 - (bit_shift_carry_2 - pow2(8) * bit_shift_carry_3))
                - limb_shift_marker_0 * rs1_next_3 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_1 * rd_next_0,
            opcode_sll_flag * limb_shift_marker_1 * (rd_next_1 + pow2(8) * bit_shift_carry_0)
                - limb_shift_marker_1 * rs1_next_0 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_1 * (rd_next_2 - (bit_shift_carry_0 - pow2(8) * bit_shift_carry_1))
                - limb_shift_marker_1 * rs1_next_1 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_1 * (rd_next_3 - (bit_shift_carry_1 - pow2(8) * bit_shift_carry_2))
                - limb_shift_marker_1 * rs1_next_2 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_2 * rd_next_0,
            opcode_sll_flag * limb_shift_marker_2 * rd_next_1,
            opcode_sll_flag * limb_shift_marker_2 * (rd_next_2 + pow2(8) * bit_shift_carry_0)
                - limb_shift_marker_2 * rs1_next_0 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_2 * (rd_next_3 - (bit_shift_carry_0 - pow2(8) * bit_shift_carry_1))
                - limb_shift_marker_2 * rs1_next_1 * bit_multiplier_left,
            opcode_sll_flag * limb_shift_marker_3 * rd_next_0,
            opcode_sll_flag * limb_shift_marker_3 * rd_next_1,
            opcode_sll_flag * limb_shift_marker_3 * rd_next_2,
            opcode_sll_flag * limb_shift_marker_3 * (rd_next_3 + pow2(8) * bit_shift_carry_0)
                - limb_shift_marker_3 * rs1_next_0 * bit_multiplier_left,
            // Right shift with sign fill
            limb_shift_marker_0 * (bit_shift_carry_1 * right_shift * pow2(8)
                + right_shift * (rs1_next_0 - bit_shift_carry_0)
                - rd_next_0 * bit_multiplier_right),
            limb_shift_marker_0 * (bit_shift_carry_2 * right_shift * pow2(8)
                + right_shift * (rs1_next_1 - bit_shift_carry_1)
                - rd_next_1 * bit_multiplier_right),
            limb_shift_marker_0 * (bit_shift_carry_3 * right_shift * pow2(8)
                + right_shift * (rs1_next_2 - bit_shift_carry_2)
                - rd_next_2 * bit_multiplier_right),
            limb_shift_marker_0 * (rs1_sign * (bit_multiplier_right - 1) * pow2(8)
                + right_shift * (rs1_next_3 - bit_shift_carry_3)
                - rd_next_3 * bit_multiplier_right),
            limb_shift_marker_1 * (bit_shift_carry_2 * right_shift * pow2(8)
                + right_shift * (rs1_next_1 - bit_shift_carry_1)
                - rd_next_0 * bit_multiplier_right),
            limb_shift_marker_1 * (bit_shift_carry_3 * right_shift * pow2(8)
                + right_shift * (rs1_next_2 - bit_shift_carry_2)
                - rd_next_1 * bit_multiplier_right),
            limb_shift_marker_1 * (rs1_sign * (bit_multiplier_right - 1) * pow2(8)
                + right_shift * (rs1_next_3 - bit_shift_carry_3)
                - rd_next_2 * bit_multiplier_right),
            right_shift * limb_shift_marker_1 * (rd_next_3 - rs1_sign * (pow2(8) - 1)),
            limb_shift_marker_2 * (bit_shift_carry_3 * right_shift * pow2(8)
                + right_shift * (rs1_next_2 - bit_shift_carry_2)
                - rd_next_0 * bit_multiplier_right),
            limb_shift_marker_2 * (rs1_sign * (bit_multiplier_right - 1) * pow2(8)
                + right_shift * (rs1_next_3 - bit_shift_carry_3)
                - rd_next_1 * bit_multiplier_right),
            right_shift * limb_shift_marker_2 * (rd_next_2 - rs1_sign * (pow2(8) - 1)),
            right_shift * limb_shift_marker_2 * (rd_next_3 - rs1_sign * (pow2(8) - 1)),
            limb_shift_marker_3 * (rs1_sign * (bit_multiplier_right - 1) * pow2(8)
                + right_shift * (rs1_next_3 - bit_shift_carry_3)
                - rd_next_0 * bit_multiplier_right),
            right_shift * limb_shift_marker_3 * (rd_next_1 - rs1_sign * (pow2(8) - 1)),
            right_shift * limb_shift_marker_3 * (rd_next_2 - rs1_sign * (pow2(8) - 1)),
            right_shift * limb_shift_marker_3 * (rd_next_3 - rs1_sign * (pow2(8) - 1)),
        },
        lookups: {
            // Program access (I-type): Program(pc, opcode, rd_idx, rs1_idx, imm)
            program_access: -enabler => [pc, expected_opcode_id, rd_addr, rs1_addr, imm_truncated],
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // rd byte ranges.
            preprocessed range_check_8_8: -enabler => [rd_next_0, rd_next_1],
            preprocessed range_check_8_8: -enabler => [rd_next_2, rd_next_3],
            // Write rd.
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler =>
                [0, rd_addr, clock, rd_next_0, rd_next_1, rd_next_2, rd_next_3],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 5. Less Than Reg (slt/sltu) - airs.md Section 5
    // ==========================================================================
    lt_reg: {
        clock, pc, rd, rs1, rs2,
        cmp_result, rs1_msl_felt, rs2_msl_felt,
        opcode_slt_flag, opcode_sltu_flag,
        diff_marker_0, diff_marker_1, diff_marker_2, diff_marker_3,
        diff_val,
        derived: {
            expected_opcode_id: |opcode_slt_flag, opcode_sltu_flag|
                opcode_slt_flag * constant(crate::decode::Opcode::Slt as u32)
                + opcode_sltu_flag * constant(crate::decode::Opcode::Sltu as u32),
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rs2_clock_diff: |clock, rs2_clock_prev| clock - rs2_clock_prev,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
            // Most-significant-limb gaps: zero for unsigned interpretation,
            // 2^8 when the sign adjustment applies (airs.md 5.2)
            rs1_msl_gap: |rs1_next_3, rs1_msl_felt| rs1_next_3 - rs1_msl_felt,
            rs2_msl_gap: |rs2_next_3, rs2_msl_felt| rs2_next_3 - rs2_msl_felt,
            // Signed-shifted most significant limbs for the range check
            rs1_msl_shifted: |rs1_msl_felt, opcode_slt_flag| rs1_msl_felt + opcode_slt_flag * pow2(7),
            rs2_msl_shifted: |rs2_msl_felt, opcode_slt_flag| rs2_msl_felt + opcode_slt_flag * pow2(7),
            // Sum of the difference markers: at most one fires
            prefix_sum_final: |diff_marker_0, diff_marker_1, diff_marker_2, diff_marker_3|
                diff_marker_0 + diff_marker_1 + diff_marker_2 + diff_marker_3,
            // Sign of the comparison: +1 if cmp_result else -1
            cmp_sign: |cmp_result| 2 * cmp_result - 1,
        },
        constraints: {
            cmp_result * (1 - cmp_result),
            diff_marker_0 * (1 - diff_marker_0),
            diff_marker_1 * (1 - diff_marker_1),
            diff_marker_2 * (1 - diff_marker_2),
            diff_marker_3 * (1 - diff_marker_3),
            rs1_msl_gap * (pow2(8) - rs1_msl_gap),
            rs2_msl_gap * (pow2(8) - rs2_msl_gap),
            // Comparison scan from the most significant limb down: limbs
            // above the first difference are equal, and the marked limb's
            // difference equals diff_val (airs.md 5.3)
            (1 - diff_marker_3) * (cmp_sign * (rs2_msl_felt - rs1_msl_felt)),
            diff_marker_3 * (diff_val - cmp_sign * (rs2_msl_felt - rs1_msl_felt)),
            (1 - diff_marker_3 - diff_marker_2) * (cmp_sign * (rs2_next_2 - rs1_next_2)),
            diff_marker_2 * (diff_val - cmp_sign * (rs2_next_2 - rs1_next_2)),
            (1 - diff_marker_3 - diff_marker_2 - diff_marker_1)
                    * (cmp_sign * (rs2_next_1 - rs1_next_1)),
            diff_marker_1 * (diff_val - cmp_sign * (rs2_next_1 - rs1_next_1)),
            (1 - prefix_sum_final) * (cmp_sign * (rs2_next_0 - rs1_next_0)),
            diff_marker_0 * (diff_val - cmp_sign * (rs2_next_0 - rs1_next_0)),
            prefix_sum_final * (1 - prefix_sum_final),
            // Equal operands compare as not-less-than
            (1 - prefix_sum_final) * cmp_result,
        },
        lookups: {
            // Program access (R-type): Program(pc, opcode, rd_idx, rs1_idx, rs2_idx)
            program_access: -enabler => [pc, expected_opcode_id, rd_addr, rs1_addr, rs2_addr],
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // Read rs2.
            memory_access: -enabler =>
                [0, rs2_addr, rs2_clock_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3],
            memory_access: enabler =>
                [0, rs2_addr, clock, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3],
            preprocessed range_check_20: -enabler => [rs2_clock_diff],
            // Most significant limbs shifted into unsigned range under the
            // signed-comparison convention.
            preprocessed range_check_8_8: -enabler => [rs1_msl_shifted, rs2_msl_shifted],
            // When the comparison scan fired, the limb difference is > 0.
            preprocessed range_check_20: -prefix_sum_final => [diff_val - 1],
            // Write rd := cmp_result (a single bit in limb 0).
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler => [0, rd_addr, clock, cmp_result, 0, 0, 0],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 6. Less Than Imm (slti/sltiu) - airs.md Section 6
    // ==========================================================================
    lt_imm: {
        clock, pc, rd, rs1,
        cmp_result, rs1_msl_felt,
        imm_0, imm_1, imm_msb,
        opcode_slti_flag, opcode_sltiu_flag,
        diff_marker_0, diff_marker_1, diff_marker_2, diff_marker_3,
        diff_val,
        derived: {
            expected_opcode_id: |opcode_slti_flag, opcode_sltiu_flag|
                opcode_slti_flag * constant(crate::decode::Opcode::Slti as u32)
                + opcode_sltiu_flag * constant(crate::decode::Opcode::Sltiu as u32),
            // I-type immediate (airs.md 6.2)
            imm: |imm_0, imm_1, imm_msb| imm_0 + pow2(8) * imm_1 + pow2(11) * imm_msb,
            // Sign-extended immediate limbs; limb 0 is imm_0, limb 3 = limb 2
            sext_imm_1: |imm_1, imm_msb| imm_1 + (pow2(8) - pow2(3)) * imm_msb,
            sext_imm_2: |imm_msb| (pow2(8) - 1) * imm_msb,
            // Most significant limb of the comparison operand under the
            // active signedness
            sext_imm_msl_felt: |opcode_sltiu_flag, sext_imm_2, opcode_slti_flag, imm_msb|
                opcode_sltiu_flag * sext_imm_2 - opcode_slti_flag * imm_msb,
            rs1_msl_gap: |rs1_next_3, rs1_msl_felt| rs1_next_3 - rs1_msl_felt,
            rs1_msl_shifted: |rs1_msl_felt, opcode_slti_flag|
                rs1_msl_felt + opcode_slti_flag * pow2(7),
            imm_1_doubled: |imm_1| 2 * imm_1,
            prefix_sum_final: |diff_marker_0, diff_marker_1, diff_marker_2, diff_marker_3|
                diff_marker_0 + diff_marker_1 + diff_marker_2 + diff_marker_3,
            cmp_sign: |cmp_result| 2 * cmp_result - 1,
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
        },
        constraints: {
            imm_msb * (1 - imm_msb),
            rs1_msl_gap * (pow2(8) - rs1_msl_gap),
            diff_marker_0 * (1 - diff_marker_0),
            diff_marker_1 * (1 - diff_marker_1),
            diff_marker_2 * (1 - diff_marker_2),
            diff_marker_3 * (1 - diff_marker_3),
            // Comparison scan from the most significant limb down (airs.md 6.3)
            (1 - diff_marker_3) * (cmp_sign * (sext_imm_msl_felt - rs1_msl_felt)),
            diff_marker_3 * (diff_val - cmp_sign * (sext_imm_msl_felt - rs1_msl_felt)),
            (1 - diff_marker_3 - diff_marker_2) * (cmp_sign * (sext_imm_2 - rs1_next_2)),
            diff_marker_2 * (diff_val - cmp_sign * (sext_imm_2 - rs1_next_2)),
            (1 - diff_marker_3 - diff_marker_2 - diff_marker_1)
                    * (cmp_sign * (sext_imm_1 - rs1_next_1)),
            diff_marker_1 * (diff_val - cmp_sign * (sext_imm_1 - rs1_next_1)),
            (1 - prefix_sum_final) * (cmp_sign * (imm_0 - rs1_next_0)),
            diff_marker_0 * (diff_val - cmp_sign * (imm_0 - rs1_next_0)),
            prefix_sum_final * (1 - prefix_sum_final),
            (1 - prefix_sum_final) * cmp_result,
            cmp_result * (1 - cmp_result),
        },
        lookups: {
            // Program access (I-type): Program(pc, opcode, rd_idx, rs1_idx, imm)
            program_access: -enabler => [pc, expected_opcode_id, rd_addr, rs1_addr, imm],
            // Immediate limb ranges and the sign-shifted most significant limb.
            preprocessed range_check_8_8_4: -enabler => [rs1_msl_shifted, imm_0, imm_1_doubled],
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // When the comparison scan fired, the limb difference is > 0.
            preprocessed range_check_20: -prefix_sum_final => [diff_val - 1],
            // Write rd := cmp_result (a single bit in limb 0).
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler => [0, rd_addr, clock, cmp_result, 0, 0, 0],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 7. Branch Equal (beq/bne) - airs.md Section 7
    // ==========================================================================
    branch_eq: {
        clock, pc, rs1, rs2,
        imm_felt, cmp_result,
        diff_inv_marker_0, diff_inv_marker_1, diff_inv_marker_2, diff_inv_marker_3,
        opcode_beq_flag, opcode_bne_flag,
        derived: {
            expected_opcode_id: |opcode_beq_flag, opcode_bne_flag|
                opcode_beq_flag * constant(crate::decode::Opcode::Beq as u32)
                + opcode_bne_flag * constant(crate::decode::Opcode::Bne as u32),
            // 1 when the operands must be equal under the active opcode
            cmp_eq: |cmp_result, opcode_beq_flag, opcode_bne_flag|
                cmp_result * opcode_beq_flag + (1 - cmp_result) * opcode_bne_flag,
            // Inverse witness sum: cmp_eq plus marked limb differences must
            // be 1 on enabled rows (proves inequality when cmp_eq = 0)
            diff_inv_sum: |cmp_eq, diff_inv_marker_0, diff_inv_marker_1, diff_inv_marker_2,
                diff_inv_marker_3, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3,
                rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3|
                cmp_eq
                + (rs1_next_0 - rs2_next_0) * diff_inv_marker_0
                + (rs1_next_1 - rs2_next_1) * diff_inv_marker_1
                + (rs1_next_2 - rs2_next_2) * diff_inv_marker_2
                + (rs1_next_3 - rs2_next_3) * diff_inv_marker_3,
            // Conditional branch target (airs.md 7.2)
            to_pc: |pc, imm_felt, cmp_result| pc + imm_felt * cmp_result + 4 * (1 - cmp_result),
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rs2_clock_diff: |clock, rs2_clock_prev| clock - rs2_clock_prev,
        },
        constraints: {
            cmp_result * (1 - cmp_result),
            // Equality forced limb-wise when cmp_eq fires
            cmp_eq * (rs1_next_0 - rs2_next_0),
            cmp_eq * (rs1_next_1 - rs2_next_1),
            cmp_eq * (rs1_next_2 - rs2_next_2),
            cmp_eq * (rs1_next_3 - rs2_next_3),
            enabler * (1 - diff_inv_sum),
        },
        lookups: {
            // Program access (B-type): Program(pc, opcode, rs1_idx, rs2_idx, imm)
            program_access: -enabler => [pc, expected_opcode_id, rs1_addr, rs2_addr, imm_felt],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // Read rs2.
            memory_access: -enabler =>
                [0, rs2_addr, rs2_clock_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3],
            memory_access: enabler =>
                [0, rs2_addr, clock, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3],
            preprocessed range_check_20: -enabler => [rs2_clock_diff],
            // Conditional branch: pc moves to the selected target.
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [to_pc, clock_next],
        },
    },

    // ==========================================================================
    // 8. Branch Less Than (blt/bltu/bge/bgeu) - airs.md Section 8
    // ==========================================================================
    branch_lt: {
        clock, pc, rs1, rs2,
        rs1_msl_felt, rs2_msl_felt,
        imm_felt, cmp_result, cmp_lt,
        diff_marker_0, diff_marker_1, diff_marker_2, diff_marker_3,
        diff_val, branch_target,
        opcode_blt_flag, opcode_bltu_flag, opcode_bge_flag, opcode_bgeu_flag,
        derived: {
            expected_opcode_id: |opcode_blt_flag, opcode_bltu_flag, opcode_bge_flag,
                opcode_bgeu_flag|
                opcode_blt_flag * constant(crate::decode::Opcode::Blt as u32)
                + opcode_bltu_flag * constant(crate::decode::Opcode::Bltu as u32)
                + opcode_bge_flag * constant(crate::decode::Opcode::Bge as u32)
                + opcode_bgeu_flag * constant(crate::decode::Opcode::Bgeu as u32),
            lt: |opcode_blt_flag, opcode_bltu_flag| opcode_blt_flag + opcode_bltu_flag,
            ge: |opcode_bge_flag, opcode_bgeu_flag| opcode_bge_flag + opcode_bgeu_flag,
            signed: |opcode_blt_flag, opcode_bge_flag| opcode_blt_flag + opcode_bge_flag,
            rs1_msl_gap: |rs1_next_3, rs1_msl_felt| rs1_next_3 - rs1_msl_felt,
            rs2_msl_gap: |rs2_next_3, rs2_msl_felt| rs2_next_3 - rs2_msl_felt,
            rs1_msl_shifted: |rs1_msl_felt, signed| rs1_msl_felt + signed * pow2(7),
            rs2_msl_shifted: |rs2_msl_felt, signed| rs2_msl_felt + signed * pow2(7),
            prefix_sum_final: |diff_marker_0, diff_marker_1, diff_marker_2, diff_marker_3|
                diff_marker_0 + diff_marker_1 + diff_marker_2 + diff_marker_3,
            lt_sign: |cmp_lt| 2 * cmp_lt - 1,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rs2_clock_diff: |clock, rs2_clock_prev| clock - rs2_clock_prev,
        },
        constraints: {
            cmp_result * (1 - cmp_result),
            diff_marker_0 * (1 - diff_marker_0),
            diff_marker_1 * (1 - diff_marker_1),
            diff_marker_2 * (1 - diff_marker_2),
            diff_marker_3 * (1 - diff_marker_3),
            // Branch target, gated by enabler (airs.md 8.2)
            enabler * (branch_target - (pc + imm_felt * cmp_result + 4 * (1 - cmp_result))),
            rs1_msl_gap * (pow2(8) - rs1_msl_gap),
            rs2_msl_gap * (pow2(8) - rs2_msl_gap),
            // Comparison scan from the most significant limb down
            (1 - diff_marker_3) * (lt_sign * (rs2_msl_felt - rs1_msl_felt)),
            diff_marker_3 * (diff_val - lt_sign * (rs2_msl_felt - rs1_msl_felt)),
            (1 - diff_marker_3 - diff_marker_2) * (lt_sign * (rs2_next_2 - rs1_next_2)),
            diff_marker_2 * (diff_val - lt_sign * (rs2_next_2 - rs1_next_2)),
            (1 - diff_marker_3 - diff_marker_2 - diff_marker_1)
                    * (lt_sign * (rs2_next_1 - rs1_next_1)),
            diff_marker_1 * (diff_val - lt_sign * (rs2_next_1 - rs1_next_1)),
            (1 - prefix_sum_final) * (lt_sign * (rs2_next_0 - rs1_next_0)),
            diff_marker_0 * (diff_val - lt_sign * (rs2_next_0 - rs1_next_0)),
            prefix_sum_final * (1 - prefix_sum_final),
            (1 - prefix_sum_final) * cmp_lt,
            // cmp_lt selects less-than under lt opcodes, not-less-than
            // under ge opcodes
            cmp_lt - (cmp_result * lt + (1 - cmp_result) * ge),
        },
        lookups: {
            // Program access (B-type): Program(pc, opcode, rs1_idx, rs2_idx, imm)
            program_access: -enabler => [pc, expected_opcode_id, rs1_addr, rs2_addr, imm_felt],
            // Conditional branch: pc moves to the selected target.
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [branch_target, clock_next],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // Read rs2.
            memory_access: -enabler =>
                [0, rs2_addr, rs2_clock_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3],
            memory_access: enabler =>
                [0, rs2_addr, clock, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3],
            preprocessed range_check_20: -enabler => [rs2_clock_diff],
            // Most significant limbs shifted into unsigned range under the
            // signed-comparison convention.
            preprocessed range_check_8_8: -enabler => [rs1_msl_shifted, rs2_msl_shifted],
            // When the comparison scan fired, the limb difference is > 0.
            preprocessed range_check_20: -prefix_sum_final => [diff_val - 1],
        },
    },

    // ==========================================================================
    // 9. LUI - airs.md Section 9
    // ==========================================================================
    lui: {
        clock, pc, rd,
        imm_0, imm_1, imm_2,
        derived: {
            // imm = imm_0 + 2^4 * imm_1 + 2^12 * imm_2 (U-type immediate, airs.md 9.2)
            imm: |imm_0, imm_1, imm_2| imm_0 + pow2(4) * imm_1 + pow2(12) * imm_2,
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            // Limb 1 of the value written to rd: imm << 12 has limbs (0, imm_0 * 2^4, imm_1, imm_2)
            rd_val_1: |imm_0| imm_0 * pow2(4),
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
        },
        lookups: {
            // Program access (U-type): Program(pc, LUI, rd_idx, imm, 0)
            program_access: -enabler =>
                [pc, constant(crate::decode::Opcode::Lui as u32), rd_addr, imm, 0],
            // Register state transition: clock advances, pc steps by 4.
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // U-type immediate limb ranges.
            preprocessed range_check_8_8_4: -enabler => [imm_1, imm_2, imm_0],
            // Write to rd (REG_AS = 0): rd := imm << 12.
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler => [0, rd_addr, clock, 0, rd_val_1, imm_1, imm_2],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 10. AUIPC - airs.md Section 10
    // ==========================================================================
    auipc: {
        clock, pc, rd,
        imm_felt,
        derived: {
            rd_felt: |rd_next_0, rd_next_1, rd_next_2, rd_next_3|
                rd_next_0 + pow2(8) * rd_next_1 + pow2(16) * rd_next_2 + pow2(24) * rd_next_3,
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
        },
        constraints: {
            // rd = pc + imm (airs.md 10.2)
            rd_felt - (pc + imm_felt),
        },
        lookups: {
            // Program access (U-type): Program(pc, AUIPC, rd_idx, imm, 0)
            program_access: -enabler =>
                [pc, constant(crate::decode::Opcode::Auipc as u32), rd_addr, imm_felt, 0],
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // rd = pc + imm is an M31: middle limbs are bytes, outer pair is
            // checked as an M31 split.
            preprocessed range_check_8_8: -enabler => [rd_next_1, rd_next_2],
            preprocessed range_check_m31: -enabler => [rd_next_0, rd_next_3],
            // Write to rd (REG_AS = 0).
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler =>
                [0, rd_addr, clock, rd_next_0, rd_next_1, rd_next_2, rd_next_3],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 11. JALR - airs.md Section 11
    // ==========================================================================
    jalr: {
        clock, pc, rd, rs1,
        to_pc_over_two, to_pc_lsb,
        imm_felt,
        derived: {
            rs1_felt: |rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3|
                rs1_next_0 + pow2(8) * rs1_next_1 + pow2(16) * rs1_next_2 + pow2(24) * rs1_next_3,
            rd_felt: |rd_next_0, rd_next_1, rd_next_2, rd_next_3|
                rd_next_0 + pow2(8) * rd_next_1 + pow2(16) * rd_next_2 + pow2(24) * rd_next_3,
            // Jump target, even-aligned (airs.md 11.2)
            jump_target: |to_pc_over_two| 2 * to_pc_over_two,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
        },
        constraints: {
            to_pc_lsb * (1 - to_pc_lsb),
            // 2 * to_pc_over_two + to_pc_lsb = rs1 + imm
            2 * to_pc_over_two + to_pc_lsb - (rs1_felt + imm_felt),
            // rd = pc + 4, gated by rd_addr (x0 writes discarded)
            enabler * rd_addr * (rd_felt - (pc + 4)),
        },
        lookups: {
            // Program access (I-type): Program(pc, JALR, rd_idx, rs1_idx, imm)
            program_access: -enabler =>
                [pc, constant(crate::decode::Opcode::Jalr as u32), rd_addr, rs1_addr, imm_felt],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // rs1 is an M31 (the jump target must be a valid pc).
            preprocessed range_check_m31: -enabler => [rs1_next_0, rs1_next_3],
            // Jump: pc moves to the even-aligned target.
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [jump_target, clock_next],
            // rd = pc + 4 is an M31.
            preprocessed range_check_8_8: -enabler => [rd_next_1, rd_next_2],
            preprocessed range_check_m31: -enabler => [rd_next_0, rd_next_3],
            // Write rd.
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler =>
                [0, rd_addr, clock, rd_next_0, rd_next_1, rd_next_2, rd_next_3],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 12. JAL - airs.md Section 12
    // ==========================================================================
    jal: {
        clock, pc, rd,
        imm_felt,
        derived: {
            rd_felt: |rd_next_0, rd_next_1, rd_next_2, rd_next_3|
                rd_next_0 + pow2(8) * rd_next_1 + pow2(16) * rd_next_2 + pow2(24) * rd_next_3,
            jump_target: |pc, imm_felt| pc + imm_felt,
            clock_next: |clock| clock + 1,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
        },
        lookups: {
            // Program access (U-type): Program(pc, JAL, rd_idx, imm, 0)
            program_access: -enabler =>
                [pc, constant(crate::decode::Opcode::Jal as u32), rd_addr, imm_felt, 0],
            // Unconditional jump: pc moves to pc + imm.
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [jump_target, clock_next],
            // rd = pc + 4 is an M31.
            preprocessed range_check_8_8: -enabler => [rd_next_1, rd_next_2],
            preprocessed range_check_m31: -enabler => [rd_next_0, rd_next_3],
            // Write to rd (REG_AS = 0).
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler =>
                [0, rd_addr, clock, rd_next_0, rd_next_1, rd_next_2, rd_next_3],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
        constraints: {
            // rd = pc + 4, gated by enabler (padding) and rd_addr (x0
            // writes are discarded, airs.md 12.2)
            enabler * rd_addr * (rd_felt - (pc + 4)),
        },
    },

    // ==========================================================================
    // 13. Load/Store (lb/lbu/lh/lhu/lw/sb/sh/sw) - airs.md Section 13
    // ==========================================================================
    load_store: {
        clock, pc, dst, rs1, src,
        r2_idx, imm_felt, src_msb,
        shift_amount,
        src_addr_selector, dst_addr_selector,
        marker_0, marker_1, marker_2, marker_3,
        opcode_lb_flag, opcode_lh_flag, opcode_lbu_flag, opcode_lhu_flag, opcode_lw_flag,
        opcode_sb_flag, opcode_sh_flag, opcode_sw_flag,
        derived: {
            expected_opcode_id: |opcode_lb_flag, opcode_lh_flag, opcode_lbu_flag, opcode_lhu_flag,
                opcode_lw_flag, opcode_sb_flag, opcode_sh_flag, opcode_sw_flag|
                opcode_lb_flag * constant(crate::decode::Opcode::Lb as u32)
                + opcode_lh_flag * constant(crate::decode::Opcode::Lh as u32)
                + opcode_lbu_flag * constant(crate::decode::Opcode::Lbu as u32)
                + opcode_lhu_flag * constant(crate::decode::Opcode::Lhu as u32)
                + opcode_lw_flag * constant(crate::decode::Opcode::Lw as u32)
                + opcode_sb_flag * constant(crate::decode::Opcode::Sb as u32)
                + opcode_sh_flag * constant(crate::decode::Opcode::Sh as u32)
                + opcode_sw_flag * constant(crate::decode::Opcode::Sw as u32),
            opcode_b_flag: |opcode_lbu_flag, opcode_lb_flag, opcode_sb_flag|
                opcode_lbu_flag + opcode_lb_flag + opcode_sb_flag,
            opcode_h_flag: |opcode_lhu_flag, opcode_lh_flag, opcode_sh_flag|
                opcode_lhu_flag + opcode_lh_flag + opcode_sh_flag,
            opcode_w_flag: |opcode_lw_flag, opcode_sw_flag| opcode_lw_flag + opcode_sw_flag,
            is_signed: |opcode_lb_flag, opcode_lh_flag| opcode_lb_flag + opcode_lh_flag,
            load_b_flag: |opcode_lb_flag, opcode_lbu_flag| opcode_lb_flag + opcode_lbu_flag,
            load_h_flag: |opcode_lh_flag, opcode_lhu_flag| opcode_lh_flag + opcode_lhu_flag,
            is_store: |opcode_sb_flag, opcode_sh_flag, opcode_sw_flag|
                opcode_sb_flag + opcode_sh_flag + opcode_sw_flag,
            is_load: |enabler, is_store| enabler - is_store,
            // Memory address space selectors: registers are 0, RW memory 1
            src_as: |is_load| is_load,
            dst_as: |is_store| is_store,
            mem_addr: |rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3, imm_felt|
                rs1_next_0 + pow2(8) * rs1_next_1 + pow2(16) * rs1_next_2
                + pow2(24) * rs1_next_3 + imm_felt,
            sum_markers: |marker_0, marker_1, marker_2, marker_3|
                marker_0 + marker_1 + marker_2 + marker_3,
            shift_id: |marker_1, marker_2, marker_3| marker_1 + 2 * marker_2 + 3 * marker_3,
            // Sign-extension fill byte for signed loads
            signed_mask: |is_signed, src_msb| is_signed * src_msb * (pow2(8) - 1),
            // Selected aligned memory address over 4, for the range check
            aligned_addr_quarter: |src_addr_selector, dst_addr_selector, r2_idx|
                (src_addr_selector + dst_addr_selector - r2_idx) * inv(4),
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            src_clock_diff: |clock, src_clock_prev| clock - src_clock_prev,
            dst_clock_diff: |clock, dst_clock_prev| clock - dst_clock_prev,
        },
        constraints: {
            marker_0 * (1 - marker_0),
            marker_1 * (1 - marker_1),
            marker_2 * (1 - marker_2),
            marker_3 * (1 - marker_3),
            // Shift amount: byte ops use shift_id, half-word ops (shift_id 1
            // or 5) use (shift_id - 1) / 2 (airs.md 13.3)
            shift_amount - (opcode_b_flag * shift_id
                + opcode_h_flag * (shift_id - 1) * inv(2)),
            // Load/store dependent source and destination addresses
            src_addr_selector
                - (is_load * (mem_addr - shift_amount) + is_store * r2_idx),
            dst_addr_selector
                - (is_load * r2_idx + is_store * (mem_addr - shift_amount)),
            opcode_b_flag * (1 - sum_markers),
            opcode_h_flag * (2 - sum_markers),
            opcode_h_flag * (1 - shift_id) * (5 - shift_id),
            // Byte loads sign-extend the upper bytes
            load_b_flag * (signed_mask - dst_next_1),
            load_b_flag * (signed_mask - dst_next_2),
            load_b_flag * (signed_mask - dst_next_3),
            // Byte selection: loads pull memory byte i into register byte 0,
            // stores push register byte 0 into memory byte i
            load_b_flag * (dst_next_0 - src_next_0) * marker_0,
            opcode_sb_flag * (dst_next_0 - src_next_0) * marker_0,
            load_b_flag * (dst_next_0 - src_next_1) * marker_1,
            opcode_sb_flag * (dst_next_1 - src_next_0) * marker_1,
            load_b_flag * (dst_next_0 - src_next_2) * marker_2,
            opcode_sb_flag * (dst_next_2 - src_next_0) * marker_2,
            load_b_flag * (dst_next_0 - src_next_3) * marker_3,
            opcode_sb_flag * (dst_next_3 - src_next_0) * marker_3,
            // Half-word loads sign-extend the upper half
            load_h_flag * (signed_mask - dst_next_2),
            load_h_flag * (signed_mask - dst_next_3),
            // Half-word selection by shift_id (1 = low half, 5 = high half)
            load_h_flag * (5 - shift_id) * inv(4) * (dst_next_0 - src_next_0),
            load_h_flag * (5 - shift_id) * inv(4) * (dst_next_1 - src_next_1),
            load_h_flag * (shift_id - 1) * inv(4) * (dst_next_0 - src_next_2),
            load_h_flag * (shift_id - 1) * inv(4) * (dst_next_1 - src_next_3),
            opcode_sh_flag * (5 - shift_id) * inv(4) * (dst_next_0 - src_next_0),
            opcode_sh_flag * (5 - shift_id) * inv(4) * (dst_next_1 - src_next_1),
            opcode_sh_flag * (shift_id - 1) * inv(4) * (dst_next_2 - src_next_0),
            opcode_sh_flag * (shift_id - 1) * inv(4) * (dst_next_3 - src_next_1),
            // Word ops copy all bytes
            opcode_w_flag * (dst_next_0 - src_next_0),
            opcode_w_flag * (dst_next_1 - src_next_1),
            opcode_w_flag * (dst_next_2 - src_next_2),
            opcode_w_flag * (dst_next_3 - src_next_3),
        },
        lookups: {
            // Program access (I-type for loads, S-type for stores):
            // Program(pc, opcode, rs1_idx, r2_idx, imm)
            program_access: -enabler => [pc, expected_opcode_id, rs1_addr, r2_idx, imm_felt],
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // Read rs1, the base address (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // The aligned address is a multiple of 4 within the address space.
            preprocessed range_check_20: -enabler => [aligned_addr_quarter],
            // The base address is an M31.
            preprocessed range_check_m31: -enabler => [rs1_next_0, rs1_next_3],
            // Read the source (memory word for loads, register for stores).
            memory_access: -enabler =>
                [src_as, src_addr_selector, src_clock_prev,
                 src_prev_0, src_prev_1, src_prev_2, src_prev_3],
            memory_access: enabler =>
                [src_as, src_addr_selector, clock,
                 src_next_0, src_next_1, src_next_2, src_next_3],
            preprocessed range_check_20: -enabler => [src_clock_diff],
            // Write the destination (register for loads, memory for stores).
            memory_access: -enabler =>
                [dst_as, dst_addr_selector, dst_clock_prev,
                 dst_prev_0, dst_prev_1, dst_prev_2, dst_prev_3],
            memory_access: enabler =>
                [dst_as, dst_addr_selector, clock,
                 dst_next_0, dst_next_1, dst_next_2, dst_next_3],
            preprocessed range_check_20: -enabler => [dst_clock_diff],
        },
    },

    // ==========================================================================
    // 14. MUL - airs.md Section 14
    // ==========================================================================
    mul: {
        clock, pc, rd, rs1, rs2,
        derived: {
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rs2_clock_diff: |clock, rs2_clock_prev| clock - rs2_clock_prev,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
            // Schoolbook carry chain of rd = (rs1 * rs2) mod 2^32 over 8-bit
            // limbs (airs.md 14.2); each carry is range-checked, not boolean
            carry_0: |rs1_next_0, rs2_next_0, rd_next_0|
                (rs1_next_0 * rs2_next_0 - rd_next_0) * inv(pow2(8)),
            carry_1: |carry_0, rs1_next_1, rs2_next_0, rs1_next_0, rs2_next_1, rd_next_1|
                (carry_0 + rs1_next_1 * rs2_next_0 + rs1_next_0 * rs2_next_1 - rd_next_1)
                    * inv(pow2(8)),
            carry_2: |carry_1, rs1_next_2, rs2_next_0, rs1_next_1, rs2_next_1, rs1_next_0,
                rs2_next_2, rd_next_2|
                (carry_1 + rs1_next_2 * rs2_next_0 + rs1_next_1 * rs2_next_1
                    + rs1_next_0 * rs2_next_2 - rd_next_2) * inv(pow2(8)),
            carry_3: |carry_2, rs1_next_3, rs2_next_0, rs1_next_2, rs2_next_1, rs1_next_1,
                rs2_next_2, rs1_next_0, rs2_next_3, rd_next_3|
                (carry_2 + rs1_next_3 * rs2_next_0 + rs1_next_2 * rs2_next_1
                    + rs1_next_1 * rs2_next_2 + rs1_next_0 * rs2_next_3 - rd_next_3)
                    * inv(pow2(8)),
        },
        lookups: {
            // Quadratic carry denominators: every fraction must stay in a
            // singleton batch to hold the constraint degree bound.
            batch: 1,
            // Program access (R-type): Program(pc, MUL, rd_idx, rs1_idx, rs2_idx)
            program_access: -enabler =>
                [pc, constant(crate::decode::Opcode::Mul as u32), rd_addr, rs1_addr, rs2_addr],
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // Read rs2.
            memory_access: -enabler =>
                [0, rs2_addr, rs2_clock_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3],
            memory_access: enabler =>
                [0, rs2_addr, clock, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3],
            preprocessed range_check_20: -enabler => [rs2_clock_diff],
            // Multiplication carries are bytes; rd limbs are bytes.
            preprocessed range_check_8_8: -enabler => [carry_0, carry_1],
            preprocessed range_check_8_8: -enabler => [carry_2, carry_3],
            preprocessed range_check_8_8: -enabler => [rd_next_0, rd_next_1],
            preprocessed range_check_8_8: -enabler => [rd_next_2, rd_next_3],
            // Write rd.
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler =>
                [0, rd_addr, clock, rd_next_0, rd_next_1, rd_next_2, rd_next_3],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 15. MULH (mulh/mulhsu/mulhu) - airs.md Section 15
    // ==========================================================================
    mulh: {
        clock, pc, rd, rs1, rs2,
        rd_high_0, rd_high_1, rd_high_2, rd_high_3,
        rs1_sign, rs2_sign,
        opcode_mulh_flag, opcode_mulhsu_flag, opcode_mulhu_flag,
        derived: {
            expected_opcode_id: |opcode_mulh_flag, opcode_mulhsu_flag, opcode_mulhu_flag|
                opcode_mulh_flag * constant(crate::decode::Opcode::Mulh as u32)
                + opcode_mulhsu_flag * constant(crate::decode::Opcode::Mulhsu as u32)
                + opcode_mulhu_flag * constant(crate::decode::Opcode::Mulhu as u32),
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rs2_clock_diff: |clock, rs2_clock_prev| clock - rs2_clock_prev,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
            // Sign-extended 64-bit operands: top limbs gain the sign bit,
            // limbs 4..7 are the sign fill (airs.md 15.2)
            rs1_top: |rs1_next_3, rs1_sign| rs1_next_3 + rs1_sign * pow2(7),
            rs2_top: |rs2_next_3, rs2_sign| rs2_next_3 + rs2_sign * pow2(7),
            rs1_fill: |rs1_sign| rs1_sign * (pow2(8) - 1),
            rs2_fill: |rs2_sign| rs2_sign * (pow2(8) - 1),
            carry_0: |rd_high_0, rs1_next_0, rs2_next_0|
                (rs1_next_0 * rs2_next_0 - rd_high_0) * inv(pow2(8)),
            carry_1: |carry_0, rd_high_1, rs1_next_0, rs1_next_1, rs2_next_0, rs2_next_1|
                (carry_0 + rs1_next_0 * rs2_next_1 + rs1_next_1 * rs2_next_0 - rd_high_1) * inv(pow2(8)),
            carry_2: |carry_1, rd_high_2, rs1_next_0, rs1_next_1, rs1_next_2, rs2_next_0, rs2_next_1, rs2_next_2|
                (carry_1 + rs1_next_0 * rs2_next_2 + rs1_next_1 * rs2_next_1 + rs1_next_2 * rs2_next_0 - rd_high_2) * inv(pow2(8)),
            carry_3: |carry_2, rd_high_3, rs1_next_0, rs1_next_1, rs1_next_2, rs1_top, rs2_next_0, rs2_next_1, rs2_next_2, rs2_top|
                (carry_2 + rs1_next_0 * rs2_top + rs1_next_1 * rs2_next_2 + rs1_next_2 * rs2_next_1 + rs1_top * rs2_next_0 - rd_high_3) * inv(pow2(8)),
            carry_4: |carry_3, rd_next_0, rs1_fill, rs1_next_0, rs1_next_1, rs1_next_2, rs1_top, rs2_fill, rs2_next_0, rs2_next_1, rs2_next_2, rs2_top|
                (carry_3 + rs1_next_0 * rs2_fill + rs1_next_1 * rs2_top + rs1_next_2 * rs2_next_2 + rs1_top * rs2_next_1 + rs1_fill * rs2_next_0 - rd_next_0) * inv(pow2(8)),
            carry_5: |carry_4, rd_next_1, rs1_fill, rs1_next_0, rs1_next_1, rs1_next_2, rs1_top, rs2_fill, rs2_next_0, rs2_next_1, rs2_next_2, rs2_top|
                (carry_4 + rs1_next_0 * rs2_fill + rs1_next_1 * rs2_fill + rs1_next_2 * rs2_top + rs1_top * rs2_next_2 + rs1_fill * rs2_next_1 + rs1_fill * rs2_next_0 - rd_next_1) * inv(pow2(8)),
            carry_6: |carry_5, rd_next_2, rs1_fill, rs1_next_0, rs1_next_1, rs1_next_2, rs1_top, rs2_fill, rs2_next_0, rs2_next_1, rs2_next_2, rs2_top|
                (carry_5 + rs1_next_0 * rs2_fill + rs1_next_1 * rs2_fill + rs1_next_2 * rs2_fill + rs1_top * rs2_top + rs1_fill * rs2_next_2 + rs1_fill * rs2_next_1 + rs1_fill * rs2_next_0 - rd_next_2) * inv(pow2(8)),
            carry_7: |carry_6, rd_next_3, rs1_fill, rs1_next_0, rs1_next_1, rs1_next_2, rs1_top, rs2_fill, rs2_next_0, rs2_next_1, rs2_next_2, rs2_top|
                (carry_6 + rs1_next_0 * rs2_fill + rs1_next_1 * rs2_fill + rs1_next_2 * rs2_fill + rs1_top * rs2_fill + rs1_fill * rs2_top + rs1_fill * rs2_next_2 + rs1_fill * rs2_next_1 + rs1_fill * rs2_next_0 - rd_next_3) * inv(pow2(8)),
        },
        constraints: {
            rs1_sign * (1 - rs1_sign),
            rs2_sign * (1 - rs2_sign),
            // Unsigned operands force their sign bits to zero
            (opcode_mulhsu_flag + opcode_mulhu_flag) * rs2_sign,
            opcode_mulhu_flag * rs1_sign,
        },
        lookups: {
            // Quadratic carry denominators: every fraction must stay in a
            // singleton batch to hold the constraint degree bound.
            batch: 1,
            // Program access (R-type): Program(pc, opcode, rd_idx, rs1_idx, rs2_idx)
            program_access: -enabler => [pc, expected_opcode_id, rd_addr, rs1_addr, rs2_addr],
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // Read rs2.
            memory_access: -enabler =>
                [0, rs2_addr, rs2_clock_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3],
            memory_access: enabler =>
                [0, rs2_addr, clock, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3],
            preprocessed range_check_20: -enabler => [rs2_clock_diff],
            // 64-bit product carries and both result halves are bytes.
            preprocessed range_check_8_8: -enabler => [carry_0, carry_1],
            preprocessed range_check_8_8: -enabler => [carry_2, carry_3],
            preprocessed range_check_8_8: -enabler => [carry_4, carry_5],
            preprocessed range_check_8_8: -enabler => [carry_6, carry_7],
            preprocessed range_check_8_8: -enabler => [rd_high_0, rd_high_1],
            preprocessed range_check_8_8: -enabler => [rd_high_2, rd_high_3],
            preprocessed range_check_8_8: -enabler => [rd_next_0, rd_next_1],
            preprocessed range_check_8_8: -enabler => [rd_next_2, rd_next_3],
            // Write rd.
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler =>
                [0, rd_addr, clock, rd_next_0, rd_next_1, rd_next_2, rd_next_3],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 16. DIV (div/divu/rem/remu) - airs.md Section 16
    // ==========================================================================
    div: {
        clock, pc, rd, rs1, rs2,
        zero_divisor, r_zero,
        q_0, q_1, q_2, q_3,
        r_0, r_1, r_2, r_3,
        b_sign, c_sign, q_sign, sign_xor,
        c_sum_inv, r_sum_inv,
        r_abs_0, r_abs_1, r_abs_2, r_abs_3,
        r_inv_0, r_inv_1, r_inv_2, r_inv_3,
        lt_marker_0, lt_marker_1, lt_marker_2, lt_marker_3,
        lt_diff,
        opcode_div_flag, opcode_divu_flag, opcode_rem_flag, opcode_remu_flag,
        derived: {
            expected_opcode_id: |opcode_div_flag, opcode_divu_flag, opcode_rem_flag,
                opcode_remu_flag|
                opcode_div_flag * constant(crate::decode::Opcode::Div as u32)
                + opcode_divu_flag * constant(crate::decode::Opcode::Divu as u32)
                + opcode_rem_flag * constant(crate::decode::Opcode::Rem as u32)
                + opcode_remu_flag * constant(crate::decode::Opcode::Remu as u32),
            is_div: |opcode_div_flag, opcode_divu_flag| opcode_div_flag + opcode_divu_flag,
            is_signed: |opcode_div_flag, opcode_rem_flag| opcode_div_flag + opcode_rem_flag,
            special_case: |zero_divisor, r_zero| zero_divisor + r_zero,
            valid_not_zero_divisor: |enabler, zero_divisor| enabler - zero_divisor,
            valid_not_special: |enabler, special_case| enabler - special_case,
            q_sum: |q_0, q_1, q_2, q_3| q_0 + q_1 + q_2 + q_3,
            c_sum: |rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3|
                rs2_next_0 + rs2_next_1 + rs2_next_2 + rs2_next_3,
            r_sum: |r_0, r_1, r_2, r_3| r_0 + r_1 + r_2 + r_3,
            c_sign_factor: |c_sign| 1 - 2 * c_sign,
            // |r| vs |c| limb differences under the divisor sign (airs.md 16.2)
            diff_0: |c_sign_factor, rs2_next_0, r_abs_0| c_sign_factor * (rs2_next_0 - r_abs_0),
            diff_1: |c_sign_factor, rs2_next_1, r_abs_1| c_sign_factor * (rs2_next_1 - r_abs_1),
            diff_2: |c_sign_factor, rs2_next_2, r_abs_2| c_sign_factor * (rs2_next_2 - r_abs_2),
            diff_3: |c_sign_factor, rs2_next_3, r_abs_3| c_sign_factor * (rs2_next_3 - r_abs_3),
            // Result selection: quotient for div/divu, remainder for rem/remu
            a_0: |is_div, q_0, r_0| is_div * q_0 + (1 - is_div) * r_0,
            a_1: |is_div, q_1, r_1| is_div * q_1 + (1 - is_div) * r_1,
            a_2: |is_div, q_2, r_2| is_div * q_2 + (1 - is_div) * r_2,
            a_3: |is_div, q_3, r_3| is_div * q_3 + (1 - is_div) * r_3,
            // Carry chain of r + |r| = 2^32 (two's complement negation)
            carry_lt_0: |r_0, r_abs_0| (r_0 + r_abs_0) * inv(pow2(8)),
            carry_lt_1: |carry_lt_0, r_1, r_abs_1| (carry_lt_0 + r_1 + r_abs_1) * inv(pow2(8)),
            carry_lt_2: |carry_lt_1, r_2, r_abs_2| (carry_lt_1 + r_2 + r_abs_2) * inv(pow2(8)),
            carry_lt_3: |carry_lt_2, r_3, r_abs_3| (carry_lt_2 + r_3 + r_abs_3) * inv(pow2(8)),
            // Comparison scan prefixes, seeded by the special cases
            prefix_3: |special_case, lt_marker_3| special_case + lt_marker_3,
            prefix_2: |prefix_3, lt_marker_2| prefix_3 + lt_marker_2,
            prefix_1: |prefix_2, lt_marker_1| prefix_2 + lt_marker_1,
            prefix_0: |prefix_1, lt_marker_0| prefix_1 + lt_marker_0,
            lt_diff_minus_1: |lt_diff| lt_diff - 1,
            pc_next: |pc| pc + 4,
            clock_next: |clock| clock + 1,
            rs1_clock_diff: |clock, rs1_clock_prev| clock - rs1_clock_prev,
            rs2_clock_diff: |clock, rs2_clock_prev| clock - rs2_clock_prev,
            rd_clock_diff: |clock, rd_clock_prev| clock - rd_clock_prev,
        },
        constraints: {
            zero_divisor * (1 - zero_divisor),
            r_zero * (1 - r_zero),
            b_sign * (1 - b_sign),
            c_sign * (1 - c_sign),
            q_sign * (1 - q_sign),
            sign_xor * (1 - sign_xor),
            lt_marker_0 * (1 - lt_marker_0),
            lt_marker_1 * (1 - lt_marker_1),
            lt_marker_2 * (1 - lt_marker_2),
            lt_marker_3 * (1 - lt_marker_3),
            special_case * (1 - special_case),
            valid_not_zero_divisor * (1 - valid_not_zero_divisor),
            valid_not_special * (1 - valid_not_special),
            // Zero divisor: all-one quotient, zero divisor limbs (airs.md 16.3)
            zero_divisor * rs2_next_0,
            zero_divisor * rs2_next_1,
            zero_divisor * rs2_next_2,
            zero_divisor * rs2_next_3,
            zero_divisor * (q_0 - (pow2(8) - 1)),
            zero_divisor * (q_1 - (pow2(8) - 1)),
            zero_divisor * (q_2 - (pow2(8) - 1)),
            zero_divisor * (q_3 - (pow2(8) - 1)),
            valid_not_zero_divisor * (c_sum * c_sum_inv - 1),
            // Zero remainder detection
            r_zero * r_0,
            r_zero * r_1,
            r_zero * r_2,
            r_zero * r_3,
            valid_not_special * (r_sum * r_sum_inv - 1),
            // Signs only under signed opcodes; sign_xor = b_sign XOR c_sign
            (1 - is_signed) * b_sign,
            (1 - is_signed) * c_sign,
            enabler * (sign_xor - b_sign - c_sign + 2 * b_sign * c_sign),
            // Quotient sign selection
            (1 - zero_divisor) * q_sum * (q_sign - sign_xor),
            (1 - zero_divisor) * (q_sign - sign_xor) * q_sign,
            // Absolute remainder: identity without sign flip, two's
            // complement otherwise
            (1 - sign_xor) * (r_abs_0 - r_0),
            sign_xor * carry_lt_0 * (carry_lt_0 - 1),
            sign_xor * (1 - carry_lt_0) * r_abs_0,
            sign_xor * ((r_abs_0 - pow2(8)) * r_inv_0 - 1),
            (1 - sign_xor) * (r_abs_1 - r_1),
            sign_xor * (carry_lt_1 - carry_lt_0) * (carry_lt_1 - 1),
            sign_xor * (1 - carry_lt_1) * r_abs_1,
            sign_xor * ((r_abs_1 - pow2(8)) * r_inv_1 - 1),
            (1 - sign_xor) * (r_abs_2 - r_2),
            sign_xor * (carry_lt_2 - carry_lt_1) * (carry_lt_2 - 1),
            sign_xor * (1 - carry_lt_2) * r_abs_2,
            sign_xor * ((r_abs_2 - pow2(8)) * r_inv_2 - 1),
            (1 - sign_xor) * (r_abs_3 - r_3),
            sign_xor * (carry_lt_3 - carry_lt_2) * (carry_lt_3 - 1),
            sign_xor * (1 - carry_lt_3) * r_abs_3,
            sign_xor * ((r_abs_3 - pow2(8)) * r_inv_3 - 1),
            // < scan from the most significant limb
            enabler * (1 - prefix_3) * diff_3,
            enabler * lt_marker_3 * (lt_diff - diff_3),
            enabler * (1 - prefix_2) * diff_2,
            enabler * lt_marker_2 * (lt_diff - diff_2),
            enabler * (1 - prefix_1) * diff_1,
            enabler * lt_marker_1 * (lt_diff - diff_1),
            enabler * (1 - prefix_0) * diff_0,
            enabler * lt_marker_0 * (lt_diff - diff_0),
            enabler * (1 - prefix_0),
        },
        lookups: {
            // Program access (R-type): Program(pc, opcode, rd_idx, rs1_idx, rs2_idx)
            program_access: -enabler => [pc, expected_opcode_id, rd_addr, rs1_addr, rs2_addr],
            registers_state: -enabler => [pc, clock],
            registers_state: enabler => [pc_next, clock_next],
            // Read rs1 (REG_AS = 0).
            memory_access: -enabler =>
                [0, rs1_addr, rs1_clock_prev, rs1_prev_0, rs1_prev_1, rs1_prev_2, rs1_prev_3],
            memory_access: enabler =>
                [0, rs1_addr, clock, rs1_next_0, rs1_next_1, rs1_next_2, rs1_next_3],
            preprocessed range_check_20: -enabler => [rs1_clock_diff],
            // Read rs2.
            memory_access: -enabler =>
                [0, rs2_addr, rs2_clock_prev, rs2_prev_0, rs2_prev_1, rs2_prev_2, rs2_prev_3],
            memory_access: enabler =>
                [0, rs2_addr, clock, rs2_next_0, rs2_next_1, rs2_next_2, rs2_next_3],
            preprocessed range_check_20: -enabler => [rs2_clock_diff],
            // Quotient and remainder limbs are bytes.
            preprocessed range_check_8_8: -enabler => [q_0, q_1],
            preprocessed range_check_8_8: -enabler => [q_2, q_3],
            preprocessed range_check_8_8: -enabler => [r_0, r_1],
            preprocessed range_check_8_8: -enabler => [r_2, r_3],
            // |r| < |c| on regular divisions: the comparison scan difference
            // is > 0.
            preprocessed range_check_20: -valid_not_special => [lt_diff_minus_1],
            // Write rd := the division result under the special-case rules.
            memory_access: -enabler =>
                [0, rd_addr, rd_clock_prev, rd_prev_0, rd_prev_1, rd_prev_2, rd_prev_3],
            memory_access: enabler => [0, rd_addr, clock, a_0, a_1, a_2, a_3],
            preprocessed range_check_20: -enabler => [rd_clock_diff],
        },
    },

    // ==========================================================================
    // 17. Program commitment table
    // ==========================================================================
    program: {
        addr, value_0, value_1, value_2, value_3, multiplicity, root
    },

    // ==========================================================================
    // 18. Memory commitment table (initial/final)
    // ==========================================================================
    memory: {
        addr, clock,
        value_0, value_1, value_2, value_3,
        multiplicity, root
    },

    // ==========================================================================
    // 19. Merkle tree nodes
    // ==========================================================================
    merkle: {
        index, depth,
        lhs, rhs, cur,
        lhs_mult, rhs_mult, cur_mult,
        root
    },

    // ==========================================================================
    // 20. Poseidon2 hash trace
    // ==========================================================================
    poseidon2: {
        state0, state1, state2, state3, state4, state5, state6, state7, state8, state9, state10,
        state11, state12, state13, state14, state15, full0_sq1_0, full0_sq1_1, full0_sq1_2,
        full0_sq1_3, full0_sq1_4, full0_sq1_5, full0_sq1_6, full0_sq1_7, full0_sq1_8, full0_sq1_9,
        full0_sq1_10, full0_sq1_11, full0_sq1_12, full0_sq1_13, full0_sq1_14, full0_sq1_15,
        full0_sq2_0, full0_sq2_1, full0_sq2_2, full0_sq2_3, full0_sq2_4, full0_sq2_5, full0_sq2_6,
        full0_sq2_7, full0_sq2_8, full0_sq2_9, full0_sq2_10, full0_sq2_11, full0_sq2_12,
        full0_sq2_13, full0_sq2_14, full0_sq2_15, full0_mix_0, full0_mix_1, full0_mix_2,
        full0_mix_3, full0_mix_4, full0_mix_5, full0_mix_6, full0_mix_7, full0_mix_8, full0_mix_9,
        full0_mix_10, full0_mix_11, full0_mix_12, full0_mix_13, full0_mix_14, full0_mix_15,
        full1_sq1_0, full1_sq1_1, full1_sq1_2, full1_sq1_3, full1_sq1_4, full1_sq1_5, full1_sq1_6,
        full1_sq1_7, full1_sq1_8, full1_sq1_9, full1_sq1_10, full1_sq1_11, full1_sq1_12,
        full1_sq1_13, full1_sq1_14, full1_sq1_15, full1_sq2_0, full1_sq2_1, full1_sq2_2,
        full1_sq2_3, full1_sq2_4, full1_sq2_5, full1_sq2_6, full1_sq2_7, full1_sq2_8, full1_sq2_9,
        full1_sq2_10, full1_sq2_11, full1_sq2_12, full1_sq2_13, full1_sq2_14, full1_sq2_15,
        full1_mix_0, full1_mix_1, full1_mix_2, full1_mix_3, full1_mix_4, full1_mix_5, full1_mix_6,
        full1_mix_7, full1_mix_8, full1_mix_9, full1_mix_10, full1_mix_11, full1_mix_12,
        full1_mix_13, full1_mix_14, full1_mix_15, full2_sq1_0, full2_sq1_1, full2_sq1_2,
        full2_sq1_3, full2_sq1_4, full2_sq1_5, full2_sq1_6, full2_sq1_7, full2_sq1_8, full2_sq1_9,
        full2_sq1_10, full2_sq1_11, full2_sq1_12, full2_sq1_13, full2_sq1_14, full2_sq1_15,
        full2_sq2_0, full2_sq2_1, full2_sq2_2, full2_sq2_3, full2_sq2_4, full2_sq2_5, full2_sq2_6,
        full2_sq2_7, full2_sq2_8, full2_sq2_9, full2_sq2_10, full2_sq2_11, full2_sq2_12,
        full2_sq2_13, full2_sq2_14, full2_sq2_15, full2_mix_0, full2_mix_1, full2_mix_2,
        full2_mix_3, full2_mix_4, full2_mix_5, full2_mix_6, full2_mix_7, full2_mix_8, full2_mix_9,
        full2_mix_10, full2_mix_11, full2_mix_12, full2_mix_13, full2_mix_14, full2_mix_15,
        full3_sq1_0, full3_sq1_1, full3_sq1_2, full3_sq1_3, full3_sq1_4, full3_sq1_5, full3_sq1_6,
        full3_sq1_7, full3_sq1_8, full3_sq1_9, full3_sq1_10, full3_sq1_11, full3_sq1_12,
        full3_sq1_13, full3_sq1_14, full3_sq1_15, full3_sq2_0, full3_sq2_1, full3_sq2_2,
        full3_sq2_3, full3_sq2_4, full3_sq2_5, full3_sq2_6, full3_sq2_7, full3_sq2_8, full3_sq2_9,
        full3_sq2_10, full3_sq2_11, full3_sq2_12, full3_sq2_13, full3_sq2_14, full3_sq2_15,
        full3_mix_0, full3_mix_1, full3_mix_2, full3_mix_3, full3_mix_4, full3_mix_5, full3_mix_6,
        full3_mix_7, full3_mix_8, full3_mix_9, full3_mix_10, full3_mix_11, full3_mix_12,
        full3_mix_13, full3_mix_14, full3_mix_15, partial0_sq1, partial0_sq2, partial0_mul,
        partial1_sq1, partial1_sq2, partial1_mul, partial2_sq1, partial2_sq2, partial2_mul,
        partial3_sq1, partial3_sq2, partial3_mul, partial4_sq1, partial4_sq2, partial4_mul,
        partial5_sq1, partial5_sq2, partial5_mul, partial6_sq1, partial6_sq2, partial6_mul,
        partial7_sq1, partial7_sq2, partial7_mul, partial8_sq1, partial8_sq2, partial8_mul,
        partial9_sq1, partial9_sq2, partial9_mul, partial10_sq1, partial10_sq2, partial10_mul,
        partial11_sq1, partial11_sq2, partial11_mul, partial12_sq1, partial12_sq2, partial12_mul,
        partial13_sq1, partial13_sq2, partial13_mul, full4_sq1_0, full4_sq1_1, full4_sq1_2,
        full4_sq1_3, full4_sq1_4, full4_sq1_5, full4_sq1_6, full4_sq1_7, full4_sq1_8, full4_sq1_9,
        full4_sq1_10, full4_sq1_11, full4_sq1_12, full4_sq1_13, full4_sq1_14, full4_sq1_15,
        full4_sq2_0, full4_sq2_1, full4_sq2_2, full4_sq2_3, full4_sq2_4, full4_sq2_5, full4_sq2_6,
        full4_sq2_7, full4_sq2_8, full4_sq2_9, full4_sq2_10, full4_sq2_11, full4_sq2_12,
        full4_sq2_13, full4_sq2_14, full4_sq2_15, full4_mix_0, full4_mix_1, full4_mix_2,
        full4_mix_3, full4_mix_4, full4_mix_5, full4_mix_6, full4_mix_7, full4_mix_8, full4_mix_9,
        full4_mix_10, full4_mix_11, full4_mix_12, full4_mix_13, full4_mix_14, full4_mix_15,
        full5_sq1_0, full5_sq1_1, full5_sq1_2, full5_sq1_3, full5_sq1_4, full5_sq1_5, full5_sq1_6,
        full5_sq1_7, full5_sq1_8, full5_sq1_9, full5_sq1_10, full5_sq1_11, full5_sq1_12,
        full5_sq1_13, full5_sq1_14, full5_sq1_15, full5_sq2_0, full5_sq2_1, full5_sq2_2,
        full5_sq2_3, full5_sq2_4, full5_sq2_5, full5_sq2_6, full5_sq2_7, full5_sq2_8, full5_sq2_9,
        full5_sq2_10, full5_sq2_11, full5_sq2_12, full5_sq2_13, full5_sq2_14, full5_sq2_15,
        full5_mix_0, full5_mix_1, full5_mix_2, full5_mix_3, full5_mix_4, full5_mix_5, full5_mix_6,
        full5_mix_7, full5_mix_8, full5_mix_9, full5_mix_10, full5_mix_11, full5_mix_12,
        full5_mix_13, full5_mix_14, full5_mix_15, full6_sq1_0, full6_sq1_1, full6_sq1_2,
        full6_sq1_3, full6_sq1_4, full6_sq1_5, full6_sq1_6, full6_sq1_7, full6_sq1_8, full6_sq1_9,
        full6_sq1_10, full6_sq1_11, full6_sq1_12, full6_sq1_13, full6_sq1_14, full6_sq1_15,
        full6_sq2_0, full6_sq2_1, full6_sq2_2, full6_sq2_3, full6_sq2_4, full6_sq2_5, full6_sq2_6,
        full6_sq2_7, full6_sq2_8, full6_sq2_9, full6_sq2_10, full6_sq2_11, full6_sq2_12,
        full6_sq2_13, full6_sq2_14, full6_sq2_15, full6_mix_0, full6_mix_1, full6_mix_2,
        full6_mix_3, full6_mix_4, full6_mix_5, full6_mix_6, full6_mix_7, full6_mix_8, full6_mix_9,
        full6_mix_10, full6_mix_11, full6_mix_12, full6_mix_13, full6_mix_14, full6_mix_15,
        full7_sq1_0, full7_sq1_1, full7_sq1_2, full7_sq1_3, full7_sq1_4, full7_sq1_5, full7_sq1_6,
        full7_sq1_7, full7_sq1_8, full7_sq1_9, full7_sq1_10, full7_sq1_11, full7_sq1_12,
        full7_sq1_13, full7_sq1_14, full7_sq1_15, full7_sq2_0, full7_sq2_1, full7_sq2_2,
        full7_sq2_3, full7_sq2_4, full7_sq2_5, full7_sq2_6, full7_sq2_7, full7_sq2_8, full7_sq2_9,
        full7_sq2_10, full7_sq2_11, full7_sq2_12, full7_sq2_13, full7_sq2_14, full7_sq2_15,
        full7_mix_0, full7_mix_1, full7_mix_2, full7_mix_3, full7_mix_4, full7_mix_5, full7_mix_6,
        full7_mix_7, full7_mix_8, full7_mix_9, full7_mix_10, full7_mix_11, full7_mix_12,
        full7_mix_13, full7_mix_14, full7_mix_15, wide, io,
    },
}

// =============================================================================
// Tracer memory access methods and utils
// =============================================================================

/// Unified access record for both registers and memory.
///
/// - For registers: `addr` is the register index (0-31)
/// - For memory: `addr` is the byte address
/// - Values stored as `[u8; 4]` little-endian limbs (1-4 bytes meaningful)
///
/// Note: The current clock (`clock`) is not stored here because it's redundant
/// with the VM's `tracer.clock` at the time of the access.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Access {
    pub addr: u32,
    pub prev: u32,
    pub clock_prev: u32,
    pub next: u32,
}

impl std::fmt::Debug for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Access")
            .field("addr", &format_args!("{:#x}", self.addr))
            .field("prev", &format_args!("{:#x}", self.prev))
            .field("clock_prev", &self.clock_prev)
            .field("next", &format_args!("{:#x}", self.next))
            .finish()
    }
}

// =============================================================================
// Columnar AccessTable (for clock update)
// =============================================================================

/// Columnar storage for Access records.
///
/// Simplified storage since for clock catch-up:
/// - `prev == next` (value unchanged)
/// - `clock == clock_prev + max_clock_diff` (fixed increment)
#[derive(Clone)]
pub struct AccessTable {
    pub addr: AlignedVec<u32>,
    pub value: AlignedVec<u32>,
    pub clock_prev: AlignedVec<u32>,
    pub max_clock_diff: u32,
}

impl Default for AccessTable {
    fn default() -> Self {
        Self {
            addr: AlignedVec::new(),
            value: AlignedVec::new(),
            clock_prev: AlignedVec::new(),
            max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
        }
    }
}

impl std::fmt::Debug for AccessTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for i in 0..self.len() {
            list.entry(&Access {
                addr: self.addr[i],
                prev: self.value[i],
                clock_prev: self.clock_prev[i],
                next: self.value[i],
            });
        }
        list.finish()
    }
}

impl AccessTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            addr: AlignedVec::with_capacity(cap),
            value: AlignedVec::with_capacity(cap),
            clock_prev: AlignedVec::with_capacity(cap),
            max_clock_diff: DEFAULT_MAX_CLOCK_DIFF,
        }
    }

    pub fn with_max_clock_diff(max_clock_diff: u32) -> Self {
        Self {
            max_clock_diff,
            ..Default::default()
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.addr.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.addr.is_empty()
    }

    #[inline]
    pub fn push(&mut self, access: Access) {
        debug_assert_eq!(
            access.prev, access.next,
            "clock catch-up must not change value"
        );
        self.addr.push(access.addr);
        self.value.push(access.prev);
        self.clock_prev.push(access.clock_prev);
    }

    /// Consumes the table and returns columns in canonical order.
    /// Order matches the ClockUpdateColumns layout in the prover.
    pub fn into_columns(self) -> Vec<AlignedVec<u32>> {
        let len = self.len();
        let mut enabler = AlignedVec::with_capacity(len);
        for _ in 0..len {
            enabler.push(1);
        }

        let mut value_0 = AlignedVec::with_capacity(len);
        let mut value_1 = AlignedVec::with_capacity(len);
        let mut value_2 = AlignedVec::with_capacity(len);
        let mut value_3 = AlignedVec::with_capacity(len);
        for val in self.value.iter() {
            let val = *val;
            value_0.push(val & 0xFF);
            value_1.push((val >> 8) & 0xFF);
            value_2.push((val >> 16) & 0xFF);
            value_3.push((val >> 24) & 0xFF);
        }

        vec![
            enabler,
            self.addr,
            self.clock_prev,
            value_0,
            value_1,
            value_2,
            value_3,
        ]
    }

    /// Convert table to trace columns, padding to power of 2.
    /// Always produces columns with minimum log_size of 4 (16 rows),
    /// even for empty tables.
    pub fn into_witness(
        self,
    ) -> Vec<
        stwo::prover::poly::circle::CircleEvaluation<
            stwo::prover::backend::simd::SimdBackend,
            stwo::core::fields::m31::BaseField,
            stwo::prover::poly::BitReversedOrder,
        >,
    > {
        use stwo::core::poly::circle::CanonicCoset;
        use stwo::prover::backend::simd::column::BaseColumn;
        use stwo::prover::poly::circle::CircleEvaluation;

        let len = self.len() as u32;
        let log_size = len.next_power_of_two().ilog2().max(4);
        let padded_len = 1 << log_size;
        let columns = self.into_columns();
        let domain = CanonicCoset::new(log_size).circle_domain();

        columns
            .into_iter()
            .map(|mut col| {
                col.resize(padded_len, 0);
                let base_col: BaseColumn = col.into();
                CircleEvaluation::new(domain, base_col)
            })
            .collect()
    }

    pub fn to_witness(
        &self,
    ) -> Vec<
        stwo::prover::poly::circle::CircleEvaluation<
            stwo::prover::backend::simd::SimdBackend,
            stwo::core::fields::m31::BaseField,
            stwo::prover::poly::BitReversedOrder,
        >,
    > {
        self.clone().into_witness()
    }
}

impl Tracer {
    /// Generate and store intermediate accesses for clock catch-up.
    fn fill_gap(
        &mut self,
        table: GapTable,
        addr: u32,
        value: u32,
        clock_prev: u32,
        target_clock: u32,
    ) -> u32 {
        let mut current_clock = clock_prev;

        while target_clock.saturating_sub(current_clock) > self.max_clock_diff {
            let next_clock = current_clock.saturating_add(self.max_clock_diff);
            let access = Access {
                addr,
                prev: value,
                clock_prev: current_clock,
                next: value,
            };
            match table {
                GapTable::Reg => self.reg_clock_update.push(access),
                GapTable::Mem => self.mem_clock_update.push(access),
            }
            current_clock = next_clock;
        }

        current_clock
    }

    /// Trace a register access with gap-filling.
    /// Intermediate accesses are pushed to `reg_clock_update`.
    /// Returns only the final access.
    pub fn trace_reg_access(&mut self, idx: u8, prev: u32, next: u32) -> Access {
        let clock_prev = self.reg_clock[idx as usize];
        let addr = idx as u32;

        // Generate intermediate catch-up accesses and get final clock_prev
        let final_clock_prev = self.fill_gap(GapTable::Reg, addr, prev, clock_prev, self.clock);

        // Update the register's clock after gap-filling
        if final_clock_prev != clock_prev {
            self.reg_clock[idx as usize] = final_clock_prev;
        }

        // Create the final access (clock is available from tracer.clock at call site)
        let final_access = Access {
            addr,
            prev,
            clock_prev: final_clock_prev,
            next,
        };

        // Update the register's clock
        self.reg_clock[idx as usize] = self.clock;

        final_access
    }

    /// Trace a memory access with gap-filling.
    /// All memory accesses are traced at 4-byte aligned addresses.
    /// Intermediate accesses are pushed to `mem_clock_update`.
    /// Returns only the final access.
    pub fn trace_mem_access(&mut self, addr: u32, prev: u32, next: u32) -> Access {
        // Always use 4-byte aligned address
        let aligned_addr = addr & !3;

        self.mem_initial.entry(aligned_addr).or_insert(prev);

        let clock_prev = self.mem_clock.get(&aligned_addr).copied().unwrap_or(0);

        // Generate intermediate catch-up accesses and get final clock_prev
        let final_clock_prev =
            self.fill_gap(GapTable::Mem, aligned_addr, prev, clock_prev, self.clock);

        // Update mem_clock after gap-filling
        if final_clock_prev != clock_prev {
            self.mem_clock.insert(aligned_addr, final_clock_prev);
        }

        // Create the final access (clock is available from tracer.clock at call site)
        let final_access = Access {
            addr: aligned_addr,
            prev,
            clock_prev: final_clock_prev,
            next,
        };

        // Update the memory word's clock
        self.mem_clock.insert(aligned_addr, self.clock);

        final_access
    }

    pub fn trace_instr_access(&mut self, pc: u32) {
        *self.program_reads.entry(pc).or_insert(0) += 1;
    }
}

/// Helper enum for gap-filling table selection.
enum GapTable {
    Reg,
    Mem,
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    const MEM_ADDR: u32 = 0x2000;

    impl AccessTable {
        /// Returns an iterator over Access values (for backward compatibility).
        pub fn iter(&self) -> AccessTableIter<'_> {
            AccessTableIter {
                table: self,
                idx: 0,
            }
        }
    }

    /// Iterator over AccessTable that yields Access values.
    pub struct AccessTableIter<'a> {
        table: &'a AccessTable,
        idx: usize,
    }

    impl Iterator for AccessTableIter<'_> {
        type Item = Access;

        fn next(&mut self) -> Option<Self::Item> {
            if self.idx >= self.table.len() {
                None
            } else {
                let clock_prev = self.table.clock_prev[self.idx];
                let value = self.table.value[self.idx];
                let access = Access {
                    addr: self.table.addr[self.idx],
                    prev: value,
                    clock_prev,
                    next: value, // For gap-filling, prev == next
                };
                self.idx += 1;
                Some(access)
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let remaining = self.table.len() - self.idx;
            (remaining, Some(remaining))
        }
    }

    impl ExactSizeIterator for AccessTableIter<'_> {}

    impl<'a> IntoIterator for &'a AccessTable {
        type Item = Access;
        type IntoIter = AccessTableIter<'a>;

        fn into_iter(self) -> Self::IntoIter {
            self.iter()
        }
    }

    // =========================================================================
    // Tracer Construction
    // =========================================================================

    #[test]
    fn test_default_tracer() {
        let tracer = Tracer::default();
        assert_eq!(tracer.clock, 0);
        assert_eq!(tracer.max_clock_diff, DEFAULT_MAX_CLOCK_DIFF);
        assert_eq!(tracer.reg_clock, [0; 32]);
        assert!(tracer.mem_clock.is_empty());
        assert!(tracer.mem_initial.is_empty());
        assert!(tracer.program_reads.is_empty());
    }

    #[test]
    fn test_with_max_clock_diff() {
        let tracer = Tracer::with_max_clock_diff(100);
        assert_eq!(tracer.max_clock_diff, 100);
        assert_eq!(tracer.clock, 0);
    }

    // =========================================================================
    // Memory Access Tracing
    // =========================================================================

    #[test]
    fn test_trace_mem_access_first_access() {
        let mut tracer = Tracer::default();
        tracer.clock = 10;

        let access = tracer.trace_mem_access(MEM_ADDR, 0x42, 0x42);

        assert_eq!(access.addr, MEM_ADDR);
        assert_eq!(access.prev, 0x42);
        assert_eq!(access.next, 0x42);
        assert_eq!(access.clock_prev, 0);
        // Note: access.clock is no longer stored; use tracer.clock at call site
        assert!(tracer.mem_clock_update.is_empty());
    }

    #[test]
    fn test_trace_mem_access_consecutive() {
        let mut tracer = Tracer::default();

        tracer.clock = 1;
        tracer.trace_mem_access(MEM_ADDR, 0x11, 0x11);

        tracer.clock = 2;
        let access = tracer.trace_mem_access(MEM_ADDR, 0x11, 0x22);

        assert_eq!(access.clock_prev, 1);
        // Note: access.clock is no longer stored; current clock is tracer.clock=2
        assert_eq!(access.prev, 0x11);
        assert_eq!(access.next, 0x22);
        assert!(tracer.mem_clock_update.is_empty());
    }

    #[test]
    fn test_trace_mem_access_gap_filling() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clock = 0;
        tracer.trace_mem_access(MEM_ADDR, 0x42, 0x42);

        tracer.clock = 350;
        let access = tracer.trace_mem_access(MEM_ADDR, 0x42, 0x42);

        // Gap of 350 with max_diff 100 needs 3 intermediates
        assert_eq!(
            tracer.mem_clock_update.len(),
            3,
            "Expected 3 intermediates, got {}",
            tracer.mem_clock_update.len()
        );

        // Verify intermediates have correct clock_prev progression
        // Each intermediate's clock was clock_prev + max_clock_diff (now implicit)
        // Sequence: 0 -> 100 -> 200 -> 300 -> 350 (final)
        assert_eq!(tracer.mem_clock_update.clock_prev[0], 0);
        assert_eq!(tracer.mem_clock_update.clock_prev[1], 100);
        assert_eq!(tracer.mem_clock_update.clock_prev[2], 200);

        // Final access's clock_prev should be 300 (after 3 intermediates)
        assert_eq!(access.clock_prev, 300);
        // Final access's clock is tracer.clock=350, diff is 50 which is <= 100
    }

    #[test]
    fn test_trace_mem_access_exact_max_diff() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clock = 0;
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        tracer.clock = 100;
        let access = tracer.trace_mem_access(MEM_ADDR, 0, 0);

        // Exactly at max_clock_diff - no intermediate needed
        assert!(tracer.mem_clock_update.is_empty());
        assert_eq!(access.clock_prev, 0);
        // Note: access.clock is no longer stored; current clock is tracer.clock=100
    }

    #[test]
    fn test_trace_mem_access_preserves_value() {
        let mut tracer = Tracer::with_max_clock_diff(50);

        tracer.clock = 0;
        tracer.trace_mem_access(MEM_ADDR, 0xAB, 0xAB);

        tracer.clock = 200;
        let access = tracer.trace_mem_access(MEM_ADDR, 0xAB, 0xAB);

        // All intermediate accesses should preserve the value
        for intermediate in &tracer.mem_clock_update {
            assert_eq!(intermediate.prev, 0xAB);
            assert_eq!(intermediate.next, 0xAB);
        }
        // Final access should also preserve value
        assert_eq!(access.prev, 0xAB);
        assert_eq!(access.next, 0xAB);
    }

    #[test]
    fn test_trace_mem_access_updates_mem_clock() {
        let mut tracer = Tracer::default();
        tracer.clock = 10;

        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        assert_eq!(tracer.mem_clock.get(&MEM_ADDR), Some(&10));
    }

    // =========================================================================
    // Register Access Tracing
    // =========================================================================

    #[test]
    fn test_trace_reg_access_first_access() {
        let mut tracer = Tracer::default();
        tracer.clock = 10;

        let access = tracer.trace_reg_access(5, 0x42, 0x42);

        assert_eq!(access.addr, 5);
        assert_eq!(access.prev, 0x42);
        assert_eq!(access.next, 0x42);
        assert_eq!(access.clock_prev, 0);
        // Note: access.clock is no longer stored; use tracer.clock at call site
        assert!(tracer.reg_clock_update.is_empty());
    }

    #[test]
    fn test_trace_reg_access_consecutive() {
        let mut tracer = Tracer::default();

        tracer.clock = 1;
        tracer.trace_reg_access(5, 0x11, 0x11);

        tracer.clock = 2;
        let access = tracer.trace_reg_access(5, 0x11, 0x22);

        assert_eq!(access.clock_prev, 1);
        // Note: access.clock is no longer stored; current clock is tracer.clock=2
        assert_eq!(access.prev, 0x11);
        assert_eq!(access.next, 0x22);
        assert!(tracer.reg_clock_update.is_empty());
    }

    #[test]
    fn test_trace_reg_access_gap_filling() {
        let mut tracer = Tracer::with_max_clock_diff(100);

        tracer.clock = 0;
        tracer.trace_reg_access(5, 0x42, 0x42);

        tracer.clock = 350;
        let access = tracer.trace_reg_access(5, 0x42, 0x42);

        // Gap of 350 with max_diff 100 needs 3 intermediates
        assert_eq!(
            tracer.reg_clock_update.len(),
            3,
            "Expected 3 intermediates, got {}",
            tracer.reg_clock_update.len()
        );

        // Verify intermediates have correct clock_prev progression
        // Each intermediate's clock was clock_prev + max_clock_diff (now implicit)
        // Sequence: 0 -> 100 -> 200 -> 300 -> 350 (final)
        assert_eq!(tracer.reg_clock_update.clock_prev[0], 0);
        assert_eq!(tracer.reg_clock_update.clock_prev[1], 100);
        assert_eq!(tracer.reg_clock_update.clock_prev[2], 200);

        // Final access's clock_prev should be 300 (after 3 intermediates)
        assert_eq!(access.clock_prev, 300);
        // Final access's clock is tracer.clock=350, diff is 50 which is <= 100
    }

    #[test]
    fn test_trace_reg_access_x0() {
        let mut tracer = Tracer::default();
        tracer.clock = 10;

        // x0 can still be traced - the caller handles x0 semantics
        let access = tracer.trace_reg_access(0, 0, 0);

        assert_eq!(access.addr, 0);
        assert_eq!(access.prev, 0);
        assert_eq!(access.next, 0);
        assert!(tracer.reg_clock_update.is_empty());
    }

    #[test]
    fn test_trace_reg_access_updates_reg_clock() {
        let mut tracer = Tracer::default();
        tracer.clock = 10;

        tracer.trace_reg_access(5, 0, 0);

        assert_eq!(tracer.reg_clock[5], 10);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_max_clock_diff_one() {
        let mut tracer = Tracer::with_max_clock_diff(1);

        tracer.clock = 0;
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        tracer.clock = 5;
        let access = tracer.trace_mem_access(MEM_ADDR, 0, 0);

        // With max_clock_diff=1, gap of 5 needs 4 intermediates + 1 final
        assert_eq!(tracer.mem_clock_update.len(), 4);

        // Verify intermediates have correct clock_prev progression: 0, 1, 2, 3
        // Each intermediate's clock was clock_prev + 1 (now implicit)
        assert_eq!(tracer.mem_clock_update.clock_prev[0], 0);
        assert_eq!(tracer.mem_clock_update.clock_prev[1], 1);
        assert_eq!(tracer.mem_clock_update.clock_prev[2], 2);
        assert_eq!(tracer.mem_clock_update.clock_prev[3], 3);

        // Final access's clock_prev is 4, and tracer.clock=5, so diff is 1
        assert_eq!(access.clock_prev, 4);
    }

    #[test]
    fn test_max_clock_diff_max() {
        let mut tracer = Tracer::with_max_clock_diff(u32::MAX);

        tracer.clock = 0;
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        tracer.clock = u32::MAX - 1;
        tracer.trace_mem_access(MEM_ADDR, 0, 0);

        // No intermediate ever needed
        assert!(tracer.mem_clock_update.is_empty());
    }

    // =========================================================================
    // Columnar Table Tests
    // =========================================================================

    #[test]
    fn test_base_alu_reg_table_push() {
        let mut table = BaseAluRegTable::new();

        let rd = Access {
            addr: 1,
            prev: 0,
            clock_prev: 0,
            next: 10,
        };
        let rs1 = Access {
            addr: 2,
            prev: 5,
            clock_prev: 0,
            next: 5,
        };
        let rs2 = Access {
            addr: 3,
            prev: 5,
            clock_prev: 0,
            next: 5,
        };

        // Push with opcode flags: add=1, sub=0, xor=0, or=0, and=0
        table.push(1, 0x1000, rd, rs1, rs2, 1, 0, 0, 0, 0);

        assert_eq!(table.len(), 1);
        assert_eq!(table.clock[0], 1);
        assert_eq!(table.pc[0], 0x1000);
        assert_eq!(table.rd_addr[0], 1);
        assert_eq!(table.rd_next[0], 10);
        assert_eq!(table.rs1_addr[0], 2);
        assert_eq!(table.rs2_addr[0], 3);
        assert_eq!(table.opcode_add_flag[0], 1);
        assert_eq!(table.opcode_sub_flag[0], 0);
    }

    #[test]
    fn test_access_table_push() {
        let mut table = AccessTable::with_max_clock_diff(100);

        // AccessTable is for gap-filling: prev == next
        let value = 42u32;
        let access = Access {
            addr: 100,
            prev: value,
            clock_prev: 0,
            next: value,
        };
        table.push(access);

        assert_eq!(table.len(), 1);
        assert_eq!(table.addr[0], 100);
        assert_eq!(table.value[0], value);
    }

    #[test]
    fn test_total_traces() {
        let mut tracer = Tracer::default();

        // Push some traces
        let rd = Access::default();
        let rs1 = Access::default();
        let rs2 = Access::default();

        // base_alu_reg with add flag
        tracer.base_alu_reg.push(0, 0, rd, rs1, rs2, 1, 0, 0, 0, 0);
        tracer.base_alu_reg.push(1, 4, rd, rs1, rs2, 1, 0, 0, 0, 0);
        // base_alu_reg with sub flag
        tracer.base_alu_reg.push(2, 8, rd, rs1, rs2, 0, 1, 0, 0, 0);

        assert_eq!(tracer.total_traces(), 3);
    }

    #[test]
    fn test_trace_op_macro() {
        let mut tracer = Tracer::default();
        tracer.clock = 1;

        let rd = Access::default();
        let rs1 = Access::default();
        let rs2 = Access::default();

        trace_op!(base_alu_reg: tracer, 0x1000, rd, rs1, rs2, 1, 0, 0, 0, 0);

        assert_eq!(tracer.base_alu_reg.len(), 1);
        assert_eq!(tracer.base_alu_reg.clock[0], 1);
        assert_eq!(tracer.base_alu_reg.pc[0], 0x1000);
    }

    // Test prover column generation for new family tables
    mod prover_column_tests {
        use super::prover_columns::*;

        #[test]
        fn test_base_alu_reg_columns_size() {
            // base_alu_reg: clock, pc, rd (10), rs1 (10), rs2 (10),
            // + 5 opcode flags = 37 total (no enabler - has flags)
            assert_eq!(BaseAluRegColumns::<()>::SIZE, 37);
        }

        #[test]
        fn test_base_alu_imm_columns_size() {
            // base_alu_imm: clock, pc, rd (10), rs1 (10),
            // + imm_0, imm_1, imm_msb (3) + 4 opcode flags = 29 total (no enabler - has flags)
            assert_eq!(BaseAluImmColumns::<()>::SIZE, 29);
        }

        #[test]
        fn test_lui_columns_size() {
            // LUI: enabler (1), clock, pc, rd (10), imm_0, imm_1, imm_2 = 16 total
            assert_eq!(LuiColumns::<()>::SIZE, 16);
        }

        #[test]
        fn test_load_store_columns_size() {
            // load_store: clock (1), pc (1), dst (10), rs1 (10), src (10),
            // + r2_idx, imm_felt, src_msb, shift_amount (4)
            // + src_addr_selector, dst_addr_selector (2)
            // + marker_0..3 (4) + 8 opcode flags = 50 total (no enabler - has flags)
            assert_eq!(LoadStoreColumns::<()>::SIZE, 50);
        }

        #[test]
        fn test_branch_eq_columns_size() {
            // branch_eq: clock (1), pc (1), rs1 (10), rs2 (10),
            // + imm_felt (1), cmp_result (1) + diff_inv_marker_0..3 (4) + 2 opcode flags = 30 total (no enabler - has flags)
            assert_eq!(BranchEqColumns::<()>::SIZE, 30);
        }

        #[test]
        fn test_jal_columns_size() {
            // JAL: enabler (1), clock, pc, rd (10), imm_felt = 14 total
            assert_eq!(JalColumns::<()>::SIZE, 14);
        }

        #[test]
        fn test_mul_columns_size() {
            // MUL: enabler (1), clock, pc, rd (10), rs1 (10), rs2 (10) = 33 total
            assert_eq!(MulColumns::<()>::SIZE, 33);
        }
    }

    // Test derived columns and constraints declared in define_trace_tables!
    mod derived_column_tests {
        use super::prover_columns::*;
        use stwo::core::fields::m31::BaseField;

        fn f(v: u32) -> BaseField {
            BaseField::from_u32_unchecked(v)
        }

        /// All-zero LUI columns, mutated per test.
        fn zero_lui_cols() -> LuiColumns<BaseField> {
            LuiColumns::from_iter(std::iter::repeat_n(f(0), LuiColumns::<()>::SIZE))
        }

        /// All-zero Base ALU Imm columns, mutated per test.
        fn zero_base_alu_imm_cols() -> BaseAluImmColumns<BaseField> {
            BaseAluImmColumns::from_iter(std::iter::repeat_n(f(0), BaseAluImmColumns::<()>::SIZE))
        }

        #[test]
        fn test_lui_imm_combines_limbs() {
            let mut cols = zero_lui_cols();
            cols.imm_0 = f(3);
            cols.imm_1 = f(5);
            cols.imm_2 = f(7);
            assert_eq!(cols.imm(), f(3 + 5 * (1 << 4) + 7 * (1 << 12)));
        }

        #[test]
        fn test_lui_pc_next_adds_four() {
            let mut cols = zero_lui_cols();
            cols.pc = f(0x1000);
            assert_eq!(cols.pc_next(), f(0x1004));
        }

        #[test]
        fn test_lui_rd_clock_diff() {
            let mut cols = zero_lui_cols();
            cols.clock = f(10);
            cols.rd_clock_prev = f(4);
            assert_eq!(cols.rd_clock_diff(), f(6));
        }

        #[test]
        fn test_lui_enabler_booleanity_holds_for_one() {
            let mut cols = zero_lui_cols();
            cols.enabler = f(1);
            assert_eq!(cols.constraints()[0], f(0));
        }

        #[test]
        fn test_lui_enabler_booleanity_fails_for_two() {
            let mut cols = zero_lui_cols();
            cols.enabler = f(2);
            assert_ne!(cols.constraints()[0], f(0));
        }

        #[test]
        fn test_base_alu_imm_enabler_sums_flags() {
            let mut cols = zero_base_alu_imm_cols();
            cols.opcode_add_flag = f(1);
            cols.opcode_or_flag = f(1);
            assert_eq!(cols.enabler(), f(2));
        }

        #[test]
        fn test_base_alu_imm_expected_opcode_id_selects_active_flag() {
            let mut cols = zero_base_alu_imm_cols();
            cols.opcode_xor_flag = f(1);
            assert_eq!(
                cols.expected_opcode_id(),
                f(crate::decode::Opcode::Xori as u32)
            );
        }

        #[test]
        fn test_base_alu_imm_carry_0_detects_limb_overflow() {
            let mut cols = zero_base_alu_imm_cols();
            // 255 + 1 = 256 = 0 with carry 1 over an 8-bit limb
            cols.rs1_next_0 = f(255);
            cols.imm_0 = f(1);
            cols.rd_next_0 = f(0);
            assert_eq!(cols.carry_0(), f(1));
        }

        #[test]
        fn test_base_alu_imm_carry_1_chains_carry_0() {
            let mut cols = zero_base_alu_imm_cols();
            // Limb 0 overflows; limb 1 receives the carry and overflows too
            cols.rs1_next_0 = f(255);
            cols.imm_0 = f(1);
            cols.rd_next_0 = f(0);
            cols.rs1_next_1 = f(255);
            cols.rd_next_1 = f(0);
            assert_eq!(cols.carry_1(), f(1));
        }

        #[test]
        fn test_base_alu_imm_carry_booleanity_holds_for_valid_add() {
            let mut cols = zero_base_alu_imm_cols();
            cols.opcode_add_flag = f(1);
            // rs1 = 255, imm = 1: rd = 256, i.e. limb 0 wraps to 0 and limb 1 is 1
            cols.rs1_next_0 = f(255);
            cols.imm_0 = f(1);
            cols.rd_next_0 = f(0);
            cols.rd_next_1 = f(1);
            assert!(cols.constraints().iter().all(|c| *c == f(0)));
        }

        #[test]
        fn test_at_extracts_row_values() {
            // Column c holds [c, c + 100]; pc is the third column (index 2)
            let data: Vec<Vec<BaseField>> = (0..LuiColumns::<()>::SIZE as u32)
                .map(|c| vec![f(c), f(c + 100)])
                .collect();
            let cols = LuiColumns::from_iter(data.iter());
            assert_eq!(cols.at(1).pc, f(102));
        }
    }

    // =========================================================================
    // Table Debug Tests
    // =========================================================================

    mod debug_table_tests {
        use super::*;

        #[test]
        fn test_base_alu_reg_table_to_table() {
            let mut table = BaseAluRegTable::new();

            let rd = Access {
                addr: 1,
                prev: 0,
                clock_prev: 0,
                next: 10,
            };
            let rs1 = Access {
                addr: 2,
                prev: 5,
                clock_prev: 1,
                next: 5,
            };
            let rs2 = Access {
                addr: 3,
                prev: 7,
                clock_prev: 2,
                next: 7,
            };

            // Push two rows
            table.push(1, 0x1000, rd, rs1, rs2, 1, 0, 0, 0, 0);
            table.push(2, 0x1004, rd, rs1, rs2, 0, 1, 0, 0, 0);

            table.to_table().to_string();
        }

        #[test]
        fn test_lui_table_to_table_with_enabler() {
            // LUI has an enabler column (no opcode flags)
            let mut table = LuiTable::new();

            let rd = Access {
                addr: 10,
                prev: 0,
                clock_prev: 0,
                next: 0x12345000,
            };

            table.push(1, 0x1000, rd, 0x12, 0x34, 0x50);

            let output = table.to_table().to_string();

            // Check enabler column exists
            assert!(output.contains("enabler"));
        }

        #[test]
        fn test_empty_table_to_table() {
            let table = BaseAluRegTable::new();
            let output = table.to_table().to_string();

            // Empty table should still have headers
            assert!(output.contains("clock"));
        }

        #[test]
        fn test_tracer_print_tables() {
            let mut tracer = Tracer::default();

            // Add some traces
            let rd = Access::default();
            let rs1 = Access::default();
            let rs2 = Access::default();

            tracer.base_alu_reg.push(0, 0, rd, rs1, rs2, 1, 0, 0, 0, 0);
            tracer.base_alu_reg.push(1, 4, rd, rs1, rs2, 1, 0, 0, 0, 0);

            // This should not panic
            tracer.print_tables(Some(10), Some(10));
        }

        #[test]
        fn test_tracer_print_tables_empty() {
            let tracer = Tracer::default();

            // Empty tracer should not panic
            tracer.print_tables(None, None);
        }

        #[test]
        fn test_multiple_tables_to_table() {
            let mut tracer = Tracer::default();

            // Add traces to different tables
            let rd = Access::default();
            let rs1 = Access::default();
            let rs2 = Access::default();

            tracer.base_alu_reg.push(0, 0, rd, rs1, rs2, 1, 0, 0, 0, 0);
            tracer.lui.push(1, 4, rd, 0, 0, 0);
            tracer.jal.push(2, 8, rd, 100);

            // Each table should produce valid output
            let base_alu_output = tracer.base_alu_reg.to_table().to_string();
            let lui_output = tracer.lui.to_table().to_string();
            let jal_output = tracer.jal.to_table().to_string();

            // LUI and JAL have enabler columns, BaseAluReg doesn't
            assert!(lui_output.contains("enabler"));
            assert!(jal_output.contains("enabler"));
            assert!(!base_alu_output.contains("enabler"));
        }
    }
}
