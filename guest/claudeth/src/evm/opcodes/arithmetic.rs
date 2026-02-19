//! EVM Arithmetic and Logic Opcodes
//!
//! This module implements all arithmetic, comparison, and bitwise EVM opcodes
//! following the Ethereum Yellow Paper specification.
//!
//! ## Arithmetic Opcodes (0x01-0x0B)
//! - ADD, MUL, SUB, DIV, SDIV, MOD, SMOD, ADDMOD, MULMOD, EXP, SIGNEXTEND
//!
//! ## Comparison Opcodes (0x10-0x15)
//! - LT, GT, SLT, SGT, EQ, ISZERO
//!
//! ## Bitwise Opcodes (0x16-0x1D)
//! - AND, OR, XOR, NOT, BYTE, SHL, SHR, SAR
//!
//! All arithmetic operations use wrapping semantics (modulo 2^256).

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec;

use crate::evm::stack::{Stack, StackError};
use crate::types::U256;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during opcode execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvmError {
    /// Stack error (overflow, underflow, invalid index)
    Stack(StackError),
    /// Memory error (invalid offset, overflow)
    Memory(crate::evm::memory::MemoryError),
}

impl From<StackError> for EvmError {
    fn from(err: StackError) -> Self {
        EvmError::Stack(err)
    }
}

impl From<crate::evm::memory::MemoryError> for EvmError {
    fn from(err: crate::evm::memory::MemoryError) -> Self {
        EvmError::Memory(err)
    }
}

// =============================================================================
// Arithmetic Opcodes
// =============================================================================

/// ADD (0x01): Addition modulo 2^256
///
/// Pops two values from the stack, adds them with wrapping, and pushes the result.
pub fn add(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;
    let (result, _overflow) = a.overflowing_add(b);
    stack.push(result)?;
    Ok(())
}

/// MUL (0x02): Multiplication modulo 2^256
///
/// Pops two values from the stack, multiplies them with wrapping, and pushes the result.
pub fn mul(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;
    let (result, _overflow) = a.overflowing_mul(b);
    stack.push(result)?;
    Ok(())
}

/// SUB (0x03): Subtraction modulo 2^256
///
/// Pops two values from the stack: a (top), b (second).
/// Computes a - b with wrapping and pushes the result.
pub fn sub(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;
    let (result, _overflow) = a.overflowing_sub(b);
    stack.push(result)?;
    Ok(())
}

/// DIV (0x04): Unsigned integer division
///
/// Pops two values from the stack: a (top, dividend), b (second, divisor).
/// If b is zero, pushes 0. Otherwise, computes a / b and pushes the result.
pub fn div(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;
    let result = if b.is_zero() { U256::ZERO } else { a / b };
    stack.push(result)?;
    Ok(())
}

/// SDIV (0x05): Signed integer division
///
/// Pops two values from the stack: a (top, dividend), b (second, divisor).
/// Interprets them as two's complement signed integers,
/// performs signed division (a / b), and pushes the result.
/// Division by zero returns 0. Special case: MIN / -1 returns MIN (overflow).
pub fn sdiv(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;

    if b.is_zero() {
        stack.push(U256::ZERO)?;
        return Ok(());
    }

    // Convert to signed
    let a_negative = is_negative(a);
    let b_negative = is_negative(b);

    let a_abs = if a_negative { twos_complement(a) } else { a };
    let b_abs = if b_negative { twos_complement(b) } else { b };

    // Check for MIN / -1 overflow case
    // MIN is 2^255, which when negated stays as 2^255
    if a == sign_bit() && b == U256::MAX {
        stack.push(sign_bit())?;
        return Ok(());
    }

    let result = a_abs / b_abs;

    // Apply sign
    let result = if a_negative != b_negative && !result.is_zero() {
        twos_complement(result)
    } else {
        result
    };

    stack.push(result)?;
    Ok(())
}

/// MOD (0x06): Unsigned modulo
///
/// Pops two values from the stack: a (top, dividend), b (second, modulus).
/// If b is zero, pushes 0. Otherwise, computes a % b and pushes the result.
pub fn modulo(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;
    let result = if b.is_zero() { U256::ZERO } else { a % b };
    stack.push(result)?;
    Ok(())
}

/// SMOD (0x07): Signed modulo
///
/// Pops two values from the stack: a (top, dividend), b (second, modulus).
/// Interprets them as two's complement signed integers,
/// computes a smod b, and pushes the result. The result takes the sign of a.
pub fn smod(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;

    if b.is_zero() {
        stack.push(U256::ZERO)?;
        return Ok(());
    }

    let a_negative = is_negative(a);
    let b_negative = is_negative(b);

    let a_abs = if a_negative { twos_complement(a) } else { a };
    let b_abs = if b_negative { twos_complement(b) } else { b };

    let result = a_abs % b_abs;

    // Result takes sign of dividend (a)
    let result = if a_negative && !result.is_zero() {
        twos_complement(result)
    } else {
        result
    };

    stack.push(result)?;
    Ok(())
}

/// ADDMOD (0x08): Addition modulo N
///
/// Pops three values from the stack: a, b, N.
/// If N is zero, pushes 0. Otherwise, computes (a + b) mod N and pushes the result.
/// The addition is performed modulo 2^512 before the modulo N operation.
pub fn addmod(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?; // Top (first operand)
    let b = stack.pop()?; // Second (second operand)
    let n = stack.pop()?; // Third (modulus)

    if n.is_zero() {
        stack.push(U256::ZERO)?;
        return Ok(());
    }

    // Perform addition in U512 to avoid overflow
    let a_wide = U512::from(a);
    let b_wide = U512::from(b);
    let n_wide = U512::from(n);

    let sum = a_wide + b_wide;
    let result = sum % n_wide;

    // Convert back to U256 (safe because result < n < 2^256)
    stack.push(u512_to_u256(result))?;
    Ok(())
}

/// MULMOD (0x09): Multiplication modulo N
///
/// Pops three values from the stack: a, b, N.
/// If N is zero, pushes 0. Otherwise, computes (a * b) mod N and pushes the result.
/// The multiplication is performed modulo 2^512 before the modulo N operation.
pub fn mulmod(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?; // Top (first operand)
    let b = stack.pop()?; // Second (second operand)
    let n = stack.pop()?; // Third (modulus)

    if n.is_zero() {
        stack.push(U256::ZERO)?;
        return Ok(());
    }

    // Perform multiplication in U512 to avoid overflow
    let a_wide = U512::from(a);
    let b_wide = U512::from(b);
    let n_wide = U512::from(n);

    let product = a_wide * b_wide;
    let result = product % n_wide;

    // Convert back to U256 (safe because result < n < 2^256)
    stack.push(u512_to_u256(result))?;
    Ok(())
}

/// EXP (0x0A): Exponentiation modulo 2^256
///
/// Pops two values from the stack: base (top) and exponent (second).
/// Computes base^exponent mod 2^256 and pushes the result.
pub fn exp(stack: &mut Stack) -> Result<(), EvmError> {
    let base = stack.pop()?; // Top
    let exponent = stack.pop()?; // Second

    // Fast path for common cases
    if exponent.is_zero() {
        stack.push(U256::ONE)?;
        return Ok(());
    }

    if exponent.is_one() {
        stack.push(base)?;
        return Ok(());
    }

    if base.is_zero() {
        stack.push(U256::ZERO)?;
        return Ok(());
    }

    if base.is_one() {
        stack.push(U256::ONE)?;
        return Ok(());
    }

    // Binary exponentiation
    let result = pow_mod_256(base, exponent);
    stack.push(result)?;
    Ok(())
}

/// SIGNEXTEND (0x0B): Sign extension
///
/// Pops two values from the stack: byte position b and value x.
/// Extends the sign bit at position (b + 1) * 8 - 1 to fill all higher bits.
/// If b >= 31, the value is unchanged.
pub fn signextend(stack: &mut Stack) -> Result<(), EvmError> {
    let b = stack.pop()?; // Top (byte position)
    let x = stack.pop()?; // Second (value)

    // If b >= 31, no extension needed (already full 256 bits)
    if b >= U256::from(31u64) {
        stack.push(x)?;
        return Ok(());
    }

    // Get byte position (safe because b < 31)
    let byte_pos = b.to_le_bytes()[0] as usize;
    let bit_pos = (byte_pos * 8 + 7) as u32; // Sign bit position

    // Check if sign bit is set
    let sign_bit_set = (x >> bit_pos) & U256::ONE == U256::ONE;

    if sign_bit_set {
        // Create mask: all 1s above bit_pos
        let mask = U256::MAX << (bit_pos + 1);
        stack.push(x | mask)?;
    } else {
        // Create mask: all 0s above bit_pos
        let (mask, _) = (U256::ONE << (bit_pos + 1)).overflowing_sub(U256::ONE);
        stack.push(x & mask)?;
    }

    Ok(())
}

// =============================================================================
// Comparison Opcodes
// =============================================================================

/// LT (0x10): Less than (unsigned)
///
/// Pops two values from the stack. Pushes 1 if the first is less than the second, 0 otherwise.
pub fn lt(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;
    stack.push(if a < b { U256::ONE } else { U256::ZERO })?;
    Ok(())
}

/// GT (0x11): Greater than (unsigned)
///
/// Pops two values from the stack. Pushes 1 if the first is greater than the second, 0 otherwise.
pub fn gt(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;
    stack.push(if a > b { U256::ONE } else { U256::ZERO })?;
    Ok(())
}

/// SLT (0x12): Signed less than
///
/// Pops two values from the stack, interprets them as signed integers.
/// Pushes 1 if the first is less than the second, 0 otherwise.
pub fn slt(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;

    let a_negative = is_negative(a);
    let b_negative = is_negative(b);

    let result = if a_negative == b_negative {
        // Same sign: compare as unsigned
        if a < b { U256::ONE } else { U256::ZERO }
    } else {
        // Different signs: negative is less than positive
        if a_negative { U256::ONE } else { U256::ZERO }
    };

    stack.push(result)?;
    Ok(())
}

/// SGT (0x13): Signed greater than
///
/// Pops two values from the stack, interprets them as signed integers.
/// Pushes 1 if the first is greater than the second, 0 otherwise.
pub fn sgt(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;

    let a_negative = is_negative(a);
    let b_negative = is_negative(b);

    let result = if a_negative == b_negative {
        // Same sign: compare as unsigned
        if a > b { U256::ONE } else { U256::ZERO }
    } else {
        // Different signs: positive is greater than negative
        if !a_negative { U256::ONE } else { U256::ZERO }
    };

    stack.push(result)?;
    Ok(())
}

/// EQ (0x14): Equality
///
/// Pops two values from the stack. Pushes 1 if they are equal, 0 otherwise.
pub fn eq(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;
    stack.push(if a == b { U256::ONE } else { U256::ZERO })?;
    Ok(())
}

/// ISZERO (0x15): Is zero
///
/// Pops one value from the stack. Pushes 1 if it is zero, 0 otherwise.
pub fn iszero(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    stack.push(if a.is_zero() { U256::ONE } else { U256::ZERO })?;
    Ok(())
}

// =============================================================================
// Bitwise Opcodes
// =============================================================================

/// AND (0x16): Bitwise AND
///
/// Pops two values from the stack, performs bitwise AND, and pushes the result.
pub fn and(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;
    stack.push(a & b)?;
    Ok(())
}

/// OR (0x17): Bitwise OR
///
/// Pops two values from the stack, performs bitwise OR, and pushes the result.
pub fn or(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;
    stack.push(a | b)?;
    Ok(())
}

/// XOR (0x18): Bitwise XOR
///
/// Pops two values from the stack, performs bitwise XOR, and pushes the result.
pub fn xor(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    let b = stack.pop()?;
    stack.push(a ^ b)?;
    Ok(())
}

/// NOT (0x19): Bitwise NOT
///
/// Pops one value from the stack, performs bitwise NOT, and pushes the result.
pub fn not(stack: &mut Stack) -> Result<(), EvmError> {
    let a = stack.pop()?;
    stack.push(!a)?;
    Ok(())
}

/// BYTE (0x1A): Extract byte
///
/// Pops two values: byte index i and value x.
/// Pushes the i-th byte of x, where byte 0 is the most significant byte.
/// If i >= 32, pushes 0.
pub fn byte(stack: &mut Stack) -> Result<(), EvmError> {
    let i = stack.pop()?; // Top (byte index)
    let x = stack.pop()?; // Second (value)

    // If i >= 32, result is 0
    if i >= U256::from(32u64) {
        stack.push(U256::ZERO)?;
        return Ok(());
    }

    // Get byte at position i (big-endian)
    let byte_index = i.to_le_bytes()[0] as usize;
    let bytes = x.to_be_bytes();
    stack.push(U256::from(bytes[byte_index] as u64))?;
    Ok(())
}

/// SHL (0x1B): Shift left
///
/// Pops two values: shift amount and value.
/// Shifts the value left by the shift amount and pushes the result.
/// If shift >= 256, the result is 0.
pub fn shl(stack: &mut Stack) -> Result<(), EvmError> {
    let shift = stack.pop()?; // Top (shift amount)
    let value = stack.pop()?; // Second (value)

    // If shift >= 256, result is 0
    if shift >= U256::from(256u64) {
        stack.push(U256::ZERO)?;
        return Ok(());
    }

    // Safe to convert to u32 since shift < 256
    let shift_amount = shift.to_le_bytes()[0] as u32;
    stack.push(value << shift_amount)?;
    Ok(())
}

/// SHR (0x1C): Logical shift right
///
/// Pops two values: shift amount and value.
/// Shifts the value right by the shift amount (logical) and pushes the result.
/// If shift >= 256, the result is 0.
pub fn shr(stack: &mut Stack) -> Result<(), EvmError> {
    let shift = stack.pop()?; // Top (shift amount)
    let value = stack.pop()?; // Second (value)

    // If shift >= 256, result is 0
    if shift >= U256::from(256u64) {
        stack.push(U256::ZERO)?;
        return Ok(());
    }

    // Safe to convert to u32 since shift < 256
    let shift_amount = shift.to_le_bytes()[0] as u32;
    stack.push(value >> shift_amount)?;
    Ok(())
}

/// SAR (0x1D): Arithmetic shift right
///
/// Pops two values: shift amount and value.
/// Shifts the value right by the shift amount (arithmetic, sign-extending) and pushes the result.
/// If shift >= 256, the result is 0 (for non-negative) or -1 (for negative).
pub fn sar(stack: &mut Stack) -> Result<(), EvmError> {
    let shift = stack.pop()?; // Top (shift amount)
    let value = stack.pop()?; // Second (value)

    let is_negative_val = is_negative(value);

    // If shift >= 256
    if shift >= U256::from(256u64) {
        let result = if is_negative_val {
            U256::MAX
        } else {
            U256::ZERO
        };
        stack.push(result)?;
        return Ok(());
    }

    // Safe to convert to u32 since shift < 256
    let shift_amount = shift.to_le_bytes()[0] as u32;

    // Logical shift right
    let shifted = value >> shift_amount;

    // If negative, fill high bits with 1s
    let result = if is_negative_val {
        let mask = U256::MAX << (256 - shift_amount);
        shifted | mask
    } else {
        shifted
    };

    stack.push(result)?;
    Ok(())
}

// =============================================================================
// Hashing Operations
// =============================================================================

/// KECCAK256 (0x20): Compute Keccak-256 hash
///
/// Pops offset and size from stack, reads data from memory,
/// computes the hash, and pushes the result.
pub fn keccak256(
    stack: &mut Stack,
    memory: &mut crate::evm::memory::Memory,
) -> Result<(), EvmError> {
    use crate::crypto::keccak256 as keccak;

    let offset = stack.pop()?;
    let size = stack.pop()?;

    // Convert to usize (will truncate if too large)
    let offset_usize = offset.as_usize();
    let size_usize = size.as_usize();

    // Read data from memory
    let mut data = vec![0u8; size_usize];
    for (i, byte) in data.iter_mut().enumerate().take(size_usize) {
        if offset_usize + i < memory.msize() {
            let word = memory.mload((offset_usize + i) & !31)?;
            let byte_offset = (offset_usize + i) % 32;
            let bytes = word.to_be_bytes();
            *byte = bytes[byte_offset];
        } else {
            *byte = 0;
        }
    }

    // Compute hash
    let hash = keccak(&data);

    // Push result (convert Hash to bytes)
    stack.push(U256::from_be_bytes(*hash.as_bytes()))?;
    Ok(())
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Check if a U256 value is negative in two's complement representation
#[inline]
fn is_negative(value: U256) -> bool {
    value >= sign_bit()
}

/// Get the sign bit (2^255)
#[inline]
fn sign_bit() -> U256 {
    U256::ONE << 255
}

/// Two's complement negation
#[inline]
fn twos_complement(value: U256) -> U256 {
    let (result, _) = (!value).overflowing_add(U256::ONE);
    result
}

/// Binary exponentiation modulo 2^256
fn pow_mod_256(mut base: U256, mut exp: U256) -> U256 {
    let mut result = U256::ONE;

    while !exp.is_zero() {
        if (exp & U256::ONE) == U256::ONE {
            let (r, _) = result.overflowing_mul(base);
            result = r;
        }
        let (b, _) = base.overflowing_mul(base);
        base = b;
        exp >>= 1;
    }

    result
}

/// Import U512 type for modulo operations
use crate::types::U512;

/// Convert U512 to U256 (assumes value fits in U256)
fn u512_to_u256(value: U512) -> U256 {
    let bytes = value.to_le_bytes();
    let mut u256_bytes = [0u8; 32];
    u256_bytes.copy_from_slice(&bytes[0..32]);
    U256::from_le_bytes(u256_bytes)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // Arithmetic Opcode Tests
    // =============================================================================

    #[test]
    fn test_add_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(5u64)).unwrap();
        stack.push(U256::from(3u64)).unwrap();
        add(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(8u64));
    }

    #[test]
    fn test_add_overflow() {
        let mut stack = Stack::new();
        stack.push(U256::MAX).unwrap();
        stack.push(U256::ONE).unwrap();
        add(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_add_large_values() {
        let mut stack = Stack::new();
        stack.push(U256::MAX - U256::from(10u64)).unwrap(); // MAX-10
        stack.push(U256::from(15u64)).unwrap(); // 15
        add(&mut stack).unwrap();
        // (MAX-10) + 15 = MAX + 5 = 2^256 + 4 ≡ 4 (mod 2^256)
        assert_eq!(stack.pop().unwrap(), U256::from(4u64));
    }

    #[test]
    fn test_mul_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(6u64)).unwrap();
        stack.push(U256::from(7u64)).unwrap();
        mul(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(42u64));
    }

    #[test]
    fn test_mul_overflow() {
        let mut stack = Stack::new();
        stack.push(U256::MAX).unwrap();
        stack.push(U256::from(2u64)).unwrap();
        mul(&mut stack).unwrap();
        // MAX * 2 = 2^256 - 2 (wraps to -2 in two's complement)
        assert_eq!(stack.pop().unwrap(), U256::MAX - U256::ONE);
    }

    #[test]
    fn test_mul_by_zero() {
        let mut stack = Stack::new();
        stack.push(U256::from(42u64)).unwrap();
        stack.push(U256::ZERO).unwrap();
        mul(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_sub_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(3u64)).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        sub(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(7u64));
    }

    #[test]
    fn test_sub_underflow() {
        let mut stack = Stack::new();
        stack.push(U256::from(5u64)).unwrap();
        stack.push(U256::from(3u64)).unwrap();
        sub(&mut stack).unwrap();
        // 3 - 5 wraps around
        assert_eq!(stack.pop().unwrap(), U256::MAX - U256::ONE);
    }

    #[test]
    fn test_div_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(3u64)).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        div(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(3u64));
    }

    #[test]
    fn test_div_by_zero() {
        let mut stack = Stack::new();
        stack.push(U256::ZERO).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        div(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_sdiv_positive() {
        let mut stack = Stack::new();
        stack.push(U256::from(3u64)).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        sdiv(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(3u64));
    }

    #[test]
    fn test_sdiv_negative_dividend() {
        let mut stack = Stack::new();
        // -10 / 3 = -3
        stack.push(U256::from(3u64)).unwrap();
        stack.push(twos_complement(U256::from(10u64))).unwrap();
        sdiv(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), twos_complement(U256::from(3u64)));
    }

    #[test]
    fn test_sdiv_negative_divisor() {
        let mut stack = Stack::new();
        // 10 / -3 = -3
        stack.push(twos_complement(U256::from(3u64))).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        sdiv(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), twos_complement(U256::from(3u64)));
    }

    #[test]
    fn test_sdiv_both_negative() {
        let mut stack = Stack::new();
        // -10 / -3 = 3
        stack.push(twos_complement(U256::from(3u64))).unwrap();
        stack.push(twos_complement(U256::from(10u64))).unwrap();
        sdiv(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(3u64));
    }

    #[test]
    fn test_sdiv_by_zero() {
        let mut stack = Stack::new();
        stack.push(U256::ZERO).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        sdiv(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_mod_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(3u64)).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        modulo(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(1u64));
    }

    #[test]
    fn test_mod_by_zero() {
        let mut stack = Stack::new();
        stack.push(U256::ZERO).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        modulo(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_smod_positive() {
        let mut stack = Stack::new();
        stack.push(U256::from(3u64)).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        smod(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(1u64));
    }

    #[test]
    fn test_smod_negative_dividend() {
        let mut stack = Stack::new();
        // -10 % 3 = -1
        stack.push(U256::from(3u64)).unwrap();
        stack.push(twos_complement(U256::from(10u64))).unwrap();
        smod(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), twos_complement(U256::from(1u64)));
    }

    #[test]
    fn test_smod_negative_divisor() {
        let mut stack = Stack::new();
        // 10 % -3 = 1 (result takes sign of dividend)
        stack.push(twos_complement(U256::from(3u64))).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        smod(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(1u64));
    }

    #[test]
    fn test_addmod_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(5u64)).unwrap(); // Will be N (modulus)
        stack.push(U256::from(3u64)).unwrap(); // Will be b
        stack.push(U256::from(8u64)).unwrap(); // Will be a (top)
        addmod(&mut stack).unwrap();
        // (8 + 3) % 5 = 11 % 5 = 1
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_addmod_overflow() {
        let mut stack = Stack::new();
        stack.push(U256::from(7u64)).unwrap(); // N (modulus)
        stack.push(U256::from(10u64)).unwrap(); // b
        stack.push(U256::MAX).unwrap(); // a (top)
        addmod(&mut stack).unwrap();
        // (MAX + 10) % 7 = (2^256 - 1 + 10) % 7 = (2^256 + 9) % 7
        // 2^256 % 7 = (2^3)^85 * 2 % 7 = 8^85 * 2 % 7 = 1^85 * 2 % 7 = 2
        // So (2^256 + 9) % 7 = (2 + 9) % 7 = 11 % 7 = 4
        assert_eq!(stack.pop().unwrap(), U256::from(4u64));
    }

    #[test]
    fn test_addmod_by_zero() {
        let mut stack = Stack::new();
        stack.push(U256::ZERO).unwrap(); // N = 0 (modulus)
        stack.push(U256::from(3u64)).unwrap(); // b
        stack.push(U256::from(5u64)).unwrap(); // a (top)
        addmod(&mut stack).unwrap();
        // ADDMOD with N=0 returns 0
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_mulmod_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(10u64)).unwrap(); // N (modulus)
        stack.push(U256::from(7u64)).unwrap(); // b
        stack.push(U256::from(6u64)).unwrap(); // a (top)
        mulmod(&mut stack).unwrap();
        // (6 * 7) % 10 = 42 % 10 = 2
        assert_eq!(stack.pop().unwrap(), U256::from(2u64));
    }

    #[test]
    fn test_mulmod_overflow() {
        let mut stack = Stack::new();
        stack.push(U256::MAX).unwrap();
        stack.push(U256::MAX).unwrap();
        stack.push(U256::from(100u64)).unwrap();
        mulmod(&mut stack).unwrap();
        // (MAX * MAX) % 100 should be computed without overflow
        assert!(stack.pop().unwrap() < U256::from(100u64));
    }

    #[test]
    fn test_mulmod_by_zero() {
        let mut stack = Stack::new();
        stack.push(U256::from(6u64)).unwrap();
        stack.push(U256::from(7u64)).unwrap();
        stack.push(U256::ZERO).unwrap();
        mulmod(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_exp_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(10u64)).unwrap(); // exponent
        stack.push(U256::from(2u64)).unwrap(); // base (top)
        exp(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(1024u64));
    }

    #[test]
    fn test_exp_zero_exponent() {
        let mut stack = Stack::new();
        stack.push(U256::ZERO).unwrap(); // exponent
        stack.push(U256::from(42u64)).unwrap(); // base (top)
        exp(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_exp_zero_base() {
        let mut stack = Stack::new();
        stack.push(U256::from(42u64)).unwrap(); // exponent
        stack.push(U256::ZERO).unwrap(); // base (top)
        exp(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_exp_one() {
        let mut stack = Stack::new();
        stack.push(U256::ONE).unwrap(); // exponent
        stack.push(U256::from(42u64)).unwrap(); // base (top)
        exp(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(42u64));
    }

    #[test]
    fn test_exp_overflow() {
        let mut stack = Stack::new();
        stack.push(U256::from(256u64)).unwrap(); // exponent
        stack.push(U256::from(2u64)).unwrap(); // base (top)
        exp(&mut stack).unwrap();
        // 2^256 wraps to 0
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_signextend_byte_0() {
        let mut stack = Stack::new();
        stack.push(U256::from(0x7Fu64)).unwrap(); // value (positive in byte 0)
        stack.push(U256::ZERO).unwrap(); // byte position 0 (top)
        signextend(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(0x7Fu64));
    }

    #[test]
    fn test_signextend_byte_0_negative() {
        let mut stack = Stack::new();
        stack.push(U256::from(0x80u64)).unwrap(); // value (negative in byte 0)
        stack.push(U256::ZERO).unwrap(); // byte position 0 (top)
        signextend(&mut stack).unwrap();
        // Should extend sign bit - 0x80 has bit 7 set, so extend to all 1s
        let expected = U256::MAX - U256::from(0x7Fu64);
        assert_eq!(stack.pop().unwrap(), expected);
    }

    #[test]
    fn test_signextend_byte_1() {
        let mut stack = Stack::new();
        stack.push(U256::from(0x7FFFu64)).unwrap(); // value (positive in bytes 0-1)
        stack.push(U256::ONE).unwrap(); // byte position 1 (top)
        signextend(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(0x7FFFu64));
    }

    #[test]
    fn test_signextend_large_index() {
        let mut stack = Stack::new();
        stack.push(U256::from(0x80u64)).unwrap(); // value
        stack.push(U256::from(100u64)).unwrap(); // byte position >= 31 (top)
        signextend(&mut stack).unwrap();
        // No change when index >= 31
        assert_eq!(stack.pop().unwrap(), U256::from(0x80u64));
    }

    // =============================================================================
    // Comparison Opcode Tests
    // =============================================================================

    #[test]
    fn test_lt_true() {
        let mut stack = Stack::new();
        stack.push(U256::from(10u64)).unwrap();
        stack.push(U256::from(5u64)).unwrap();
        lt(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_lt_false() {
        let mut stack = Stack::new();
        stack.push(U256::from(5u64)).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        lt(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_lt_equal() {
        let mut stack = Stack::new();
        stack.push(U256::from(5u64)).unwrap();
        stack.push(U256::from(5u64)).unwrap();
        lt(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_gt_true() {
        let mut stack = Stack::new();
        stack.push(U256::from(5u64)).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        gt(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_gt_false() {
        let mut stack = Stack::new();
        stack.push(U256::from(10u64)).unwrap();
        stack.push(U256::from(5u64)).unwrap();
        gt(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_slt_positive_true() {
        let mut stack = Stack::new();
        stack.push(U256::from(10u64)).unwrap();
        stack.push(U256::from(5u64)).unwrap();
        slt(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_slt_negative_comparison() {
        let mut stack = Stack::new();
        // -5 < 10 = true
        stack.push(U256::from(10u64)).unwrap();
        stack.push(twos_complement(U256::from(5u64))).unwrap();
        slt(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_slt_both_negative() {
        let mut stack = Stack::new();
        // -10 < -5 = true
        stack.push(twos_complement(U256::from(5u64))).unwrap();
        stack.push(twos_complement(U256::from(10u64))).unwrap();
        slt(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_sgt_positive_true() {
        let mut stack = Stack::new();
        stack.push(U256::from(5u64)).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        sgt(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_sgt_negative_comparison() {
        let mut stack = Stack::new();
        // 10 > -5 = true
        stack.push(twos_complement(U256::from(5u64))).unwrap();
        stack.push(U256::from(10u64)).unwrap();
        sgt(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_eq_true() {
        let mut stack = Stack::new();
        stack.push(U256::from(42u64)).unwrap();
        stack.push(U256::from(42u64)).unwrap();
        eq(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_eq_false() {
        let mut stack = Stack::new();
        stack.push(U256::from(42u64)).unwrap();
        stack.push(U256::from(43u64)).unwrap();
        eq(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_iszero_true() {
        let mut stack = Stack::new();
        stack.push(U256::ZERO).unwrap();
        iszero(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_iszero_false() {
        let mut stack = Stack::new();
        stack.push(U256::from(42u64)).unwrap();
        iszero(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    // =============================================================================
    // Bitwise Opcode Tests
    // =============================================================================

    #[test]
    fn test_and_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(0b1100u64)).unwrap();
        stack.push(U256::from(0b1010u64)).unwrap();
        and(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(0b1000u64));
    }

    #[test]
    fn test_and_all_ones() {
        let mut stack = Stack::new();
        stack.push(U256::from(42u64)).unwrap();
        stack.push(U256::MAX).unwrap();
        and(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(42u64));
    }

    #[test]
    fn test_or_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(0b1100u64)).unwrap();
        stack.push(U256::from(0b1010u64)).unwrap();
        or(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(0b1110u64));
    }

    #[test]
    fn test_or_with_zero() {
        let mut stack = Stack::new();
        stack.push(U256::from(42u64)).unwrap();
        stack.push(U256::ZERO).unwrap();
        or(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(42u64));
    }

    #[test]
    fn test_xor_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(0b1100u64)).unwrap();
        stack.push(U256::from(0b1010u64)).unwrap();
        xor(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(0b0110u64));
    }

    #[test]
    fn test_xor_same_value() {
        let mut stack = Stack::new();
        stack.push(U256::from(42u64)).unwrap();
        stack.push(U256::from(42u64)).unwrap();
        xor(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_not_basic() {
        let mut stack = Stack::new();
        stack.push(U256::ZERO).unwrap();
        not(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::MAX);
    }

    #[test]
    fn test_not_max() {
        let mut stack = Stack::new();
        stack.push(U256::MAX).unwrap();
        not(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_byte_first() {
        let mut stack = Stack::new();
        // Create value with 0xFF in the most significant byte (byte 0)
        stack
            .push(U256::from(0xFF00000000000000u64) << 192)
            .unwrap(); // value
        stack.push(U256::ZERO).unwrap(); // index 0 (top)
        byte(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(0xFFu64));
    }

    #[test]
    fn test_byte_middle() {
        let mut stack = Stack::new();
        let mut bytes = [0u8; 32];
        bytes[15] = 0xAB; // Byte 15 (big-endian)
        stack.push(U256::from_be_bytes(bytes)).unwrap(); // value
        stack.push(U256::from(15u64)).unwrap(); // index 15 (top)
        byte(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(0xABu64));
    }

    #[test]
    fn test_byte_out_of_bounds() {
        let mut stack = Stack::new();
        stack.push(U256::from(32u64)).unwrap();
        stack.push(U256::from(0xFFu64)).unwrap();
        byte(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_shl_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(0b1010u64)).unwrap(); // value
        stack.push(U256::from(1u64)).unwrap(); // shift amount (top)
        shl(&mut stack).unwrap();
        // SHL pops shift (1), then value (0b1010), computes value << shift = 0b1010 << 1 = 0b10100
        assert_eq!(stack.pop().unwrap(), U256::from(0b10100u64));
    }

    #[test]
    fn test_shl_large_shift() {
        let mut stack = Stack::new();
        stack.push(U256::from(42u64)).unwrap(); // value
        stack.push(U256::from(256u64)).unwrap(); // shift amount (top)
        shl(&mut stack).unwrap();
        // 42 << 256 = 0 (shift >= 256 results in 0)
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_shl_overflow() {
        let mut stack = Stack::new();
        stack.push(U256::from(1u64)).unwrap();
        stack.push(U256::MAX).unwrap();
        shl(&mut stack).unwrap();
        // Shifting left by 1 causes overflow, wraps
        assert!(stack.pop().unwrap() < U256::MAX);
    }

    #[test]
    fn test_shr_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(0b1010u64)).unwrap(); // value
        stack.push(U256::from(1u64)).unwrap(); // shift amount (top)
        shr(&mut stack).unwrap();
        // 0b1010 >> 1 = 0b101
        assert_eq!(stack.pop().unwrap(), U256::from(0b101u64));
    }

    #[test]
    fn test_shr_large_shift() {
        let mut stack = Stack::new();
        stack.push(U256::from(256u64)).unwrap();
        stack.push(U256::from(42u64)).unwrap();
        shr(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_sar_positive() {
        let mut stack = Stack::new();
        stack.push(U256::from(8u64)).unwrap(); // value
        stack.push(U256::from(2u64)).unwrap(); // shift amount (top)
        sar(&mut stack).unwrap();
        // 8 >> 2 = 2 (arithmetic right shift, but positive so same as logical)
        assert_eq!(stack.pop().unwrap(), U256::from(2u64));
    }

    #[test]
    fn test_sar_negative() {
        let mut stack = Stack::new();
        // -8 >> 2 should be -2 (sign-extended)
        stack.push(twos_complement(U256::from(8u64))).unwrap(); // value = -8
        stack.push(U256::from(2u64)).unwrap(); // shift = 2 (top)
        sar(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), twos_complement(U256::from(2u64)));
    }

    #[test]
    fn test_sar_negative_large_shift() {
        let mut stack = Stack::new();
        stack.push(twos_complement(U256::from(1u64))).unwrap(); // value = -1
        stack.push(U256::from(256u64)).unwrap(); // shift = 256 (top)
        sar(&mut stack).unwrap();
        // -1 >> 256 should be all 1s (sign extension fills with 1s)
        assert_eq!(stack.pop().unwrap(), U256::MAX);
    }

    // =============================================================================
    // Edge Case Tests
    // =============================================================================

    #[test]
    fn test_stack_underflow() {
        let mut stack = Stack::new();
        assert!(add(&mut stack).is_err());
    }

    #[test]
    fn test_operations_preserve_stack_size() {
        let mut stack = Stack::new();
        stack.push(U256::from(10u64)).unwrap();
        stack.push(U256::from(20u64)).unwrap();
        stack.push(U256::from(30u64)).unwrap();

        // ADD consumes 2, produces 1
        add(&mut stack).unwrap();
        assert_eq!(stack.len(), 2);

        // NOT consumes 1, produces 1
        not(&mut stack).unwrap();
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_max_value_operations() {
        let mut stack = Stack::new();

        // Test with MAX values
        stack.push(U256::MAX).unwrap();
        stack.push(U256::MAX).unwrap();
        add(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::MAX - U256::ONE);

        stack.push(U256::MAX).unwrap();
        stack.push(U256::MAX).unwrap();
        mul(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ONE);
    }

    #[test]
    fn test_zero_operations() {
        let mut stack = Stack::new();

        // ADD with zero
        stack.push(U256::from(42u64)).unwrap();
        stack.push(U256::ZERO).unwrap();
        add(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(42u64));

        // MUL with zero
        stack.push(U256::from(42u64)).unwrap();
        stack.push(U256::ZERO).unwrap();
        mul(&mut stack).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }
}
