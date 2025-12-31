//! Utility functions and structs for ops modules.

/// M31 prime: 2^31 - 1
pub const M31_P: u32 = 2147483647;

/// Convert a signed byte value (when MSB is set) to its M31 field representation.
/// For a negative value x (where x = byte - 256), the M31 representation is P + x.
#[inline]
pub fn byte_to_signed_felt(byte: u8) -> u32 {
    if (byte & 0x80) != 0 {
        // Negative value: byte - 256 is in range [-128, -1]
        // M31 representation: P + (byte - 256) = P - 256 + byte
        M31_P - 256 + (byte as u32)
    } else {
        byte as u32
    }
}

/// Convert a signed i32 immediate to its M31 field representation.
/// For a negative value, the M31 representation is P + imm.
/// For a non-negative value, just return imm as u32.
#[inline]
pub fn imm_to_felt(imm: i32) -> u32 {
    if imm < 0 {
        // Negative value: M31 representation is P + imm
        // Since imm is negative, this is P - |imm|
        ((M31_P as i64) + (imm as i64)) as u32
    } else {
        imm as u32
    }
}

/// Compute the multiplicative inverse of a value in M31.
/// Uses Fermat's little theorem: a^(p-2) ≡ a^(-1) (mod p)
/// Returns 0 if the input is 0 (no inverse exists).
#[inline]
pub fn m31_inverse(a: u32) -> u32 {
    if a == 0 {
        return 0;
    }
    // a^(p-2) mod p where p = 2^31 - 1
    mod_pow(a as u64, (M31_P - 2) as u64, M31_P as u64) as u32
}

/// Modular exponentiation: base^exp mod modulus
fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    let mut result = 1u64;
    base %= modulus;
    while exp > 0 {
        if exp & 1 == 1 {
            result = (result * base) % modulus;
        }
        exp >>= 1;
        base = (base * base) % modulus;
    }
    result
}

// =============================================================================
// Shift Witness
// =============================================================================

/// Witness columns for shift operations (both register and immediate variants)
pub struct ShiftWitness {
    pub rs1_sign: u32,
    pub bit_shift_marker: [u32; 8],
    pub limb_shift_marker: [u32; 4],
    pub bit_shift_carry: [u32; 4],
}

/// Compute shift witness columns for both shifts_reg and shifts_imm families
pub fn compute_shift_witness(
    rs1_val: u32,
    shamt: u32,
    is_left: bool,
    _is_sra: bool,
) -> ShiftWitness {
    let limb_shift = (shamt / 8) as usize;
    let bit_shift = (shamt % 8) as usize;

    // rs1_sign is the MSB of rs1 (bit 31), provided separately from the top limb.
    let rs1_sign = (rs1_val >> 31) & 1;

    // Compute bit_shift_carry for each limb
    let rs1_bytes = rs1_val.to_le_bytes();
    let mut bit_shift_carry = [0u32; 4];
    for i in 0..4 {
        bit_shift_carry[i] = if bit_shift == 0 {
            0
        } else if is_left {
            // For left shifts, carry is the upper bits that overflow into the next byte
            (rs1_bytes[i] as u32) >> (8 - bit_shift)
        } else {
            // For right shifts, carry is the lower bits that are shifted out
            (rs1_bytes[i] as u32) & ((1 << bit_shift) - 1)
        };
    }

    ShiftWitness {
        rs1_sign,
        bit_shift_marker: create_one_hot_8(bit_shift),
        limb_shift_marker: create_one_hot_4(limb_shift),
        bit_shift_carry,
    }
}

// =============================================================================
// Less Than Witness (register-register comparisons)
// =============================================================================

/// Witness columns for less-than comparisons between two registers.
/// Used by lt_reg (slt/sltu) and branch_lt (blt/bltu/bge/bgeu) families.
pub struct LtRegWitness {
    pub rs1_msl_felt: u32,
    pub rs2_msl_felt: u32,
    pub diff_marker: [u32; 4],
    pub diff_val: u32,
}

/// Compute comparison witness columns for register-register comparisons.
/// Used by lt_reg (slt/sltu) and branch_lt (blt/bltu/bge/bgeu) families.
pub fn compute_lt_reg_witness(rs1_val: u32, rs2_val: u32, is_signed: bool) -> LtRegWitness {
    let rs1_bytes = rs1_val.to_le_bytes();
    let rs2_bytes = rs2_val.to_le_bytes();

    let cmp_result = if is_signed {
        (rs1_val as i32) < (rs2_val as i32)
    } else {
        rs1_val < rs2_val
    };

    // For signed comparison, msl_felt is the signed interpretation of MSB as an M31 field element
    let rs1_msl_felt = if is_signed {
        byte_to_signed_felt(rs1_bytes[3])
    } else {
        rs1_bytes[3] as u32
    };
    let rs2_msl_felt = if is_signed {
        byte_to_signed_felt(rs2_bytes[3])
    } else {
        rs2_bytes[3] as u32
    };

    // Find the first differing byte from MSB
    let mut diff_marker = [0u32; 4];
    let mut diff_val = 0u32;
    for i in (0..4).rev() {
        let a = if i == 3 {
            rs1_msl_felt
        } else {
            rs1_bytes[i] as u32
        };
        let b = if i == 3 {
            rs2_msl_felt
        } else {
            rs2_bytes[i] as u32
        };
        if a != b {
            diff_marker[i] = 1;
            // diff_val follows cmp_result orientation to stay positive in the field.
            // Use M31 modulus so signed MSB comparisons like -1 vs 0 produce a small gap (1).
            let diff = if cmp_result {
                (b as u64 + M31_P as u64 - a as u64) % M31_P as u64
            } else {
                (a as u64 + M31_P as u64 - b as u64) % M31_P as u64
            };
            diff_val = diff as u32;
            break;
        }
    }

    LtRegWitness {
        rs1_msl_felt,
        rs2_msl_felt,
        diff_marker,
        diff_val,
    }
}

// =============================================================================
// Less Than Witness (register-immediate comparisons)
// =============================================================================

/// Witness columns for less-than comparisons between a register and an immediate.
/// Used by lt_imm (slti/sltiu) family.
pub struct LtImmWitness {
    pub rs1_msl_felt: u32,
    pub diff_marker: [u32; 4],
    pub diff_val: u32,
}

/// Compute comparison witness columns for register-immediate comparisons.
/// Used by lt_imm (slti/sltiu) family.
pub fn compute_lt_imm_witness(rs1_val: u32, imm: i32, is_signed: bool) -> LtImmWitness {
    let rs1_bytes = rs1_val.to_le_bytes();

    let cmp_result = if is_signed {
        (rs1_val as i32) < imm
    } else {
        rs1_val < imm as u32
    };

    // Sign-extend the immediate to 32 bits and get bytes
    let imm_extended = imm as u32;
    let imm_bytes = imm_extended.to_le_bytes();

    // For signed comparison, msl_felt is the signed interpretation of MSB as an M31 field element
    let rs1_msl_felt = if is_signed {
        byte_to_signed_felt(rs1_bytes[3])
    } else {
        rs1_bytes[3] as u32
    };
    let imm_msl_felt = if is_signed {
        byte_to_signed_felt(imm_bytes[3])
    } else {
        imm_bytes[3] as u32
    };

    // Find the first differing byte from MSB
    let mut diff_marker = [0u32; 4];
    let mut diff_val = 0u32;
    for i in (0..4).rev() {
        let a = if i == 3 {
            rs1_msl_felt
        } else {
            rs1_bytes[i] as u32
        };
        let b = if i == 3 {
            imm_msl_felt
        } else {
            imm_bytes[i] as u32
        };
        if a != b {
            diff_marker[i] = 1;
            let diff = if cmp_result {
                (b as u64 + M31_P as u64 - a as u64) % M31_P as u64
            } else {
                (a as u64 + M31_P as u64 - b as u64) % M31_P as u64
            };
            diff_val = diff as u32;
            break;
        }
    }

    LtImmWitness {
        rs1_msl_felt,
        diff_marker,
        diff_val,
    }
}

// =============================================================================
// Helper functions
// =============================================================================

/// Create a one-hot array of size 4 with the bit at `index` set to 1.
#[inline]
pub fn create_one_hot_4(index: usize) -> [u32; 4] {
    let mut result = [0u32; 4];
    if index < 4 {
        result[index] = 1;
    }
    result
}

/// Create a one-hot array of size 8 with the bit at `index` set to 1.
#[inline]
pub fn create_one_hot_8(index: usize) -> [u32; 8] {
    let mut result = [0u32; 8];
    if index < 8 {
        result[index] = 1;
    }
    result
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_to_signed_felt_positive() {
        // Positive bytes (0x00 to 0x7F) should remain unchanged
        assert_eq!(byte_to_signed_felt(0), 0);
        assert_eq!(byte_to_signed_felt(1), 1);
        assert_eq!(byte_to_signed_felt(127), 127);
    }

    #[test]
    fn test_byte_to_signed_felt_negative() {
        // Negative bytes (0x80 to 0xFF) should map to M31 representation
        // -128 (0x80) -> P - 128 = 2147483519
        assert_eq!(byte_to_signed_felt(0x80), M31_P - 128);
        // -1 (0xFF) -> P - 1 = 2147483646
        assert_eq!(byte_to_signed_felt(0xFF), M31_P - 1);
        // -127 (0x81) -> P - 127 = 2147483520
        assert_eq!(byte_to_signed_felt(0x81), M31_P - 127);
    }

    #[test]
    fn test_create_one_hot_4() {
        assert_eq!(create_one_hot_4(0), [1, 0, 0, 0]);
        assert_eq!(create_one_hot_4(1), [0, 1, 0, 0]);
        assert_eq!(create_one_hot_4(2), [0, 0, 1, 0]);
        assert_eq!(create_one_hot_4(3), [0, 0, 0, 1]);
        assert_eq!(create_one_hot_4(4), [0, 0, 0, 0]); // out of bounds
    }

    #[test]
    fn test_create_one_hot_8() {
        assert_eq!(create_one_hot_8(0), [1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(create_one_hot_8(7), [0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(create_one_hot_8(8), [0, 0, 0, 0, 0, 0, 0, 0]); // out of bounds
    }

    #[test]
    fn test_shift_witness_left() {
        let w = compute_shift_witness(0x12345678, 5, true, false);
        assert_eq!(w.rs1_sign, 0); // msb is 0
        assert_eq!(w.bit_shift_marker, [0, 0, 0, 0, 0, 1, 0, 0]); // bit 5
        assert_eq!(w.limb_shift_marker, [1, 0, 0, 0]); // limb 0
    }

    #[test]
    fn test_shift_witness_right_arithmetic() {
        let w = compute_shift_witness(0x80000000, 8, false, true);
        assert_eq!(w.rs1_sign, 1); // sra with negative number
        assert_eq!(w.bit_shift_marker, [1, 0, 0, 0, 0, 0, 0, 0]); // bit 0 (8 % 8)
        assert_eq!(w.limb_shift_marker, [0, 1, 0, 0]); // limb 1 (8 / 8)
    }

    #[test]
    fn test_lt_reg_witness_signed() {
        // -1 < 1 (signed)
        let w = compute_lt_reg_witness(0xFFFFFFFF, 1, true);
        assert_eq!(w.rs1_msl_felt, M31_P - 1); // -1 in M31
        assert_eq!(w.rs2_msl_felt, 0); // 0x00 is positive
    }

    #[test]
    fn test_lt_reg_witness_signed_negative_vs_positive() {
        // -5 < 5 (signed), ensure diff uses signed ordering for MSB
        let w = compute_lt_reg_witness(0xFFFFFFFB, 0x5, true);
        assert_eq!(w.diff_marker, [0, 0, 0, 1]);
        assert_eq!(w.diff_val, 1); // 0 - (-1) in the field is 1
    }

    #[test]
    fn test_lt_reg_witness_unsigned() {
        // 0xFFFFFFFF > 1 (unsigned)
        let w = compute_lt_reg_witness(0xFFFFFFFF, 1, false);
        assert_eq!(w.rs1_msl_felt, 0xFF); // unsigned, just the byte value
        assert_eq!(w.rs2_msl_felt, 0);
    }

    #[test]
    fn test_lt_imm_witness_signed_negative_vs_positive() {
        // -5 < 5 (signed immediate)
        let w = compute_lt_imm_witness(0xFFFFFFFB, 5, true);
        assert_eq!(w.diff_marker, [0, 0, 0, 1]);
        assert_eq!(w.diff_val, 1);
    }
}
