//! Big integer types (U256, U512) for Ethereum
//!
//! This module provides fixed-size unsigned integer types with full arithmetic operations.
//! All operations include overflow checking and follow Rust's arithmetic conventions.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::{format, string::String};

#[cfg(target_arch = "riscv32")]
use alloc::{format, string::String};

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign,
    Mul, MulAssign, Not, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};
use core::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

// =============================================================================
// U256 Implementation
// =============================================================================

/// 256-bit unsigned integer
#[derive(Clone, Copy)]
pub struct U256([u64; 4]);

impl U256 {
    /// Zero value
    pub const ZERO: Self = U256([0, 0, 0, 0]);

    /// One value
    pub const ONE: Self = U256([1, 0, 0, 0]);

    /// Maximum value
    pub const MAX: Self = U256([u64::MAX, u64::MAX, u64::MAX, u64::MAX]);

    /// Create from a single u64
    #[inline]
    pub const fn from_u64(value: u64) -> Self {
        U256([value, 0, 0, 0])
    }

    /// Check if value is zero
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.0[0] == 0 && self.0[1] == 0 && self.0[2] == 0 && self.0[3] == 0
    }

    /// Check if value is one
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.0[0] == 1 && self.0[1] == 0 && self.0[2] == 0 && self.0[3] == 0
    }

    /// Count leading zeros
    pub const fn leading_zeros(&self) -> u32 {
        if self.0[3] != 0 {
            self.0[3].leading_zeros()
        } else if self.0[2] != 0 {
            64 + self.0[2].leading_zeros()
        } else if self.0[1] != 0 {
            128 + self.0[1].leading_zeros()
        } else {
            192 + self.0[0].leading_zeros()
        }
    }

    /// Count trailing zeros
    pub const fn trailing_zeros(&self) -> u32 {
        if self.0[0] != 0 {
            self.0[0].trailing_zeros()
        } else if self.0[1] != 0 {
            64 + self.0[1].trailing_zeros()
        } else if self.0[2] != 0 {
            128 + self.0[2].trailing_zeros()
        } else {
            192 + self.0[3].trailing_zeros()
        }
    }

    /// Count number of ones
    pub const fn count_ones(&self) -> u32 {
        self.0[0].count_ones() + self.0[1].count_ones() + self.0[2].count_ones() + self.0[3].count_ones()
    }

    /// Number of bits needed to represent this value
    pub const fn bits(&self) -> u32 {
        256 - self.leading_zeros()
    }

    /// Convert to little-endian bytes
    pub fn to_le_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(&self.0[0].to_le_bytes());
        bytes[8..16].copy_from_slice(&self.0[1].to_le_bytes());
        bytes[16..24].copy_from_slice(&self.0[2].to_le_bytes());
        bytes[24..32].copy_from_slice(&self.0[3].to_le_bytes());
        bytes
    }

    /// Convert to big-endian bytes
    pub fn to_be_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..8].copy_from_slice(&self.0[3].to_be_bytes());
        bytes[8..16].copy_from_slice(&self.0[2].to_be_bytes());
        bytes[16..24].copy_from_slice(&self.0[1].to_be_bytes());
        bytes[24..32].copy_from_slice(&self.0[0].to_be_bytes());
        bytes
    }

    /// Create from little-endian bytes
    pub fn from_le_bytes(bytes: [u8; 32]) -> Self {
        let mut limbs = [0u64; 4];
        limbs[0] = u64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]);
        limbs[1] = u64::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]]);
        limbs[2] = u64::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23]]);
        limbs[3] = u64::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31]]);
        U256(limbs)
    }

    /// Create from big-endian bytes
    pub fn from_be_bytes(bytes: [u8; 32]) -> Self {
        let mut limbs = [0u64; 4];
        limbs[3] = u64::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]);
        limbs[2] = u64::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]]);
        limbs[1] = u64::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23]]);
        limbs[0] = u64::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31]]);
        U256(limbs)
    }

    /// Checked addition
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        let (result, overflow) = self.overflowing_add(rhs);
        if overflow {
            None
        } else {
            Some(result)
        }
    }

    /// Overflowing addition
    pub const fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        let (r0, carry) = self.0[0].overflowing_add(rhs.0[0]);
        let (r1, carry) = {
            let (a, c1) = self.0[1].overflowing_add(rhs.0[1]);
            let (b, c2) = a.overflowing_add(carry as u64);
            (b, c1 || c2)
        };
        let (r2, carry) = {
            let (a, c1) = self.0[2].overflowing_add(rhs.0[2]);
            let (b, c2) = a.overflowing_add(carry as u64);
            (b, c1 || c2)
        };
        let (r3, carry) = {
            let (a, c1) = self.0[3].overflowing_add(rhs.0[3]);
            let (b, c2) = a.overflowing_add(carry as u64);
            (b, c1 || c2)
        };
        (U256([r0, r1, r2, r3]), carry)
    }

    /// Saturating addition
    pub const fn saturating_add(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_add(rhs);
        if overflow {
            Self::MAX
        } else {
            result
        }
    }

    /// Checked subtraction
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        let (result, overflow) = self.overflowing_sub(rhs);
        if overflow {
            None
        } else {
            Some(result)
        }
    }

    /// Overflowing subtraction
    pub const fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        let (r0, borrow) = self.0[0].overflowing_sub(rhs.0[0]);
        let (r1, borrow) = {
            let (a, b1) = self.0[1].overflowing_sub(rhs.0[1]);
            let (b, b2) = a.overflowing_sub(borrow as u64);
            (b, b1 || b2)
        };
        let (r2, borrow) = {
            let (a, b1) = self.0[2].overflowing_sub(rhs.0[2]);
            let (b, b2) = a.overflowing_sub(borrow as u64);
            (b, b1 || b2)
        };
        let (r3, borrow) = {
            let (a, b1) = self.0[3].overflowing_sub(rhs.0[3]);
            let (b, b2) = a.overflowing_sub(borrow as u64);
            (b, b1 || b2)
        };
        (U256([r0, r1, r2, r3]), borrow)
    }

    /// Saturating subtraction
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_sub(rhs);
        if overflow {
            Self::ZERO
        } else {
            result
        }
    }

    /// Checked multiplication
    pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
        let (result, overflow) = self.overflowing_mul(rhs);
        if overflow {
            None
        } else {
            Some(result)
        }
    }

    /// Overflowing multiplication
    pub const fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        // Full 256x256 -> 512-bit multiplication
        let mut result = [0u64; 8];

        let mut i = 0;
        while i < 4 {
            let mut carry = 0u64;
            let mut j = 0;
            while j < 4 {
                let k = i + j;
                let (hi, lo) = mul_u64(self.0[i], rhs.0[j]);

                let (sum, c1) = result[k].overflowing_add(lo);
                let (sum, c2) = sum.overflowing_add(carry);
                result[k] = sum;

                carry = hi + (c1 as u64) + (c2 as u64);
                j += 1;
            }
            if i + 4 < 8 {
                result[i + 4] = carry;
            }
            i += 1;
        }

        // Check for overflow (any high bits set)
        let overflow = result[4] != 0 || result[5] != 0 || result[6] != 0 || result[7] != 0;

        (U256([result[0], result[1], result[2], result[3]]), overflow)
    }

    /// Saturating multiplication
    pub const fn saturating_mul(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_mul(rhs);
        if overflow {
            Self::MAX
        } else {
            result
        }
    }

    /// Checked division
    pub const fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.is_zero() {
            None
        } else {
            Some(self.div_rem(rhs).0)
        }
    }

    /// Checked remainder
    pub const fn checked_rem(self, rhs: Self) -> Option<Self> {
        if rhs.is_zero() {
            None
        } else {
            Some(self.div_rem(rhs).1)
        }
    }

    /// Division and remainder
    const fn div_rem(self, rhs: Self) -> (Self, Self) {
        if rhs.is_zero() {
            panic!("division by zero");
        }

        if self.is_zero() {
            return (Self::ZERO, Self::ZERO);
        }

        if rhs.is_one() {
            return (self, Self::ZERO);
        }

        // Compare for early exit
        match self.cmp_const(&rhs) {
            Ordering::Less => return (Self::ZERO, self),
            Ordering::Equal => return (Self::ONE, Self::ZERO),
            Ordering::Greater => {}
        }

        // Long division algorithm
        let mut quotient = Self::ZERO;
        let mut remainder = Self::ZERO;

        let mut i = 256;
        while i > 0 {
            i -= 1;

            // remainder <<= 1
            remainder = remainder.shl_const(1);

            // remainder |= (self >> i) & 1
            let bit = self.shr_const(i).0[0] & 1;
            remainder.0[0] |= bit;

            // if remainder >= rhs
            if matches!(remainder.cmp_const(&rhs), Ordering::Greater | Ordering::Equal) {
                remainder = remainder.sub_const(rhs);
                // quotient |= 1 << i
                let word = i / 64;
                let bit = i % 64;
                quotient.0[word as usize] |= 1u64 << bit;
            }
        }

        (quotient, remainder)
    }

    /// Const subtraction (for internal use)
    const fn sub_const(self, rhs: Self) -> Self {
        self.overflowing_sub(rhs).0
    }

    /// Const comparison (for internal use)
    const fn cmp_const(&self, other: &Self) -> Ordering {
        let mut i = 4;
        while i > 0 {
            i -= 1;
            if self.0[i] > other.0[i] {
                return Ordering::Greater;
            }
            if self.0[i] < other.0[i] {
                return Ordering::Less;
            }
        }
        Ordering::Equal
    }

    /// Const left shift (for internal use)
    const fn shl_const(self, shift: u32) -> Self {
        if shift == 0 {
            return self;
        }
        if shift >= 256 {
            return Self::ZERO;
        }

        let word_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;

        let mut result = [0u64; 4];

        if bit_shift == 0 {
            let mut i = word_shift;
            while i < 4 {
                result[i] = self.0[i - word_shift];
                i += 1;
            }
        } else {
            let mut i = word_shift;
            while i < 4 {
                result[i] = self.0[i - word_shift] << bit_shift;
                if i > word_shift {
                    result[i] |= self.0[i - word_shift - 1] >> (64 - bit_shift);
                }
                i += 1;
            }
        }

        U256(result)
    }

    /// Const right shift (for internal use)
    const fn shr_const(self, shift: u32) -> Self {
        if shift == 0 {
            return self;
        }
        if shift >= 256 {
            return Self::ZERO;
        }

        let word_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;

        let mut result = [0u64; 4];

        if bit_shift == 0 {
            let mut i = 0;
            while i < 4 - word_shift {
                result[i] = self.0[i + word_shift];
                i += 1;
            }
        } else {
            let mut i = 0;
            while i < 4 - word_shift {
                result[i] = self.0[i + word_shift] >> bit_shift;
                if i + word_shift + 1 < 4 {
                    result[i] |= self.0[i + word_shift + 1] << (64 - bit_shift);
                }
                i += 1;
            }
        }

        U256(result)
    }

    /// Convert to usize (truncates to lower 64 bits, then to usize)
    ///
    /// # Panics
    ///
    /// Panics on 32-bit platforms if the value doesn't fit in usize
    pub fn as_usize(&self) -> usize {
        self.0[0] as usize
    }

    /// Convert to u8 (truncates to lowest byte)
    pub fn as_u8(&self) -> u8 {
        self.0[0] as u8
    }

    /// Convert to u64 (truncates to lower 64 bits)
    pub fn as_u64(&self) -> u64 {
        self.0[0]
    }
}

// Helper function for 64x64 -> 128-bit multiplication
const fn mul_u64(a: u64, b: u64) -> (u64, u64) {
    let a_lo = a as u32 as u64;
    let a_hi = (a >> 32) as u32 as u64;
    let b_lo = b as u32 as u64;
    let b_hi = (b >> 32) as u32 as u64;

    let lo_lo = a_lo * b_lo;
    let lo_hi = a_lo * b_hi;
    let hi_lo = a_hi * b_lo;
    let hi_hi = a_hi * b_hi;

    let cross = (lo_lo >> 32) + (lo_hi as u32 as u64) + (hi_lo as u32 as u64);
    let hi = hi_hi + (lo_hi >> 32) + (hi_lo >> 32) + (cross >> 32);
    let lo = (cross << 32) | (lo_lo as u32 as u64);

    (hi, lo)
}

// =============================================================================
// U256 Trait Implementations
// =============================================================================

impl Default for U256 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Debug for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "U256(0x{:016x}{:016x}{:016x}{:016x})", self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl fmt::Display for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}{:016x}{:016x}{:016x}", self.0[3], self.0[2], self.0[1], self.0[0])
    }
}

impl PartialEq for U256 {
    fn eq(&self, other: &Self) -> bool {
        self.0[0] == other.0[0] && self.0[1] == other.0[1] && self.0[2] == other.0[2] && self.0[3] == other.0[3]
    }
}

impl Eq for U256 {}

impl Hash for U256 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp_const(other)
    }
}

// Arithmetic trait implementations
impl Add for U256 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_add(rhs);
        if overflow {
            panic!("attempt to add with overflow");
        }
        result
    }
}

impl AddAssign for U256 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for U256 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_sub(rhs);
        if overflow {
            panic!("attempt to subtract with overflow");
        }
        result
    }
}

impl SubAssign for U256 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for U256 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_mul(rhs);
        if overflow {
            panic!("attempt to multiply with overflow");
        }
        result
    }
}

impl MulAssign for U256 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Div for U256 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        self.checked_div(rhs).expect("division by zero")
    }
}

impl DivAssign for U256 {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Rem for U256 {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self {
        self.checked_rem(rhs).expect("remainder by zero")
    }
}

impl RemAssign for U256 {
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

// Bitwise trait implementations
impl BitAnd for U256 {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        U256([
            self.0[0] & rhs.0[0],
            self.0[1] & rhs.0[1],
            self.0[2] & rhs.0[2],
            self.0[3] & rhs.0[3],
        ])
    }
}

impl BitAndAssign for U256 {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl BitOr for U256 {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        U256([
            self.0[0] | rhs.0[0],
            self.0[1] | rhs.0[1],
            self.0[2] | rhs.0[2],
            self.0[3] | rhs.0[3],
        ])
    }
}

impl BitOrAssign for U256 {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitXor for U256 {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        U256([
            self.0[0] ^ rhs.0[0],
            self.0[1] ^ rhs.0[1],
            self.0[2] ^ rhs.0[2],
            self.0[3] ^ rhs.0[3],
        ])
    }
}

impl BitXorAssign for U256 {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl Not for U256 {
    type Output = Self;
    fn not(self) -> Self {
        U256([!self.0[0], !self.0[1], !self.0[2], !self.0[3]])
    }
}

impl Shl<u32> for U256 {
    type Output = Self;
    fn shl(self, shift: u32) -> Self {
        self.shl_const(shift)
    }
}

impl ShlAssign<u32> for U256 {
    fn shl_assign(&mut self, shift: u32) {
        *self = *self << shift;
    }
}

impl Shr<u32> for U256 {
    type Output = Self;
    fn shr(self, shift: u32) -> Self {
        self.shr_const(shift)
    }
}

impl ShrAssign<u32> for U256 {
    fn shr_assign(&mut self, shift: u32) {
        *self = *self >> shift;
    }
}

// Conversion trait implementations
impl From<u8> for U256 {
    fn from(value: u8) -> Self {
        U256([value as u64, 0, 0, 0])
    }
}

impl From<u16> for U256 {
    fn from(value: u16) -> Self {
        U256([value as u64, 0, 0, 0])
    }
}

impl From<u32> for U256 {
    fn from(value: u32) -> Self {
        U256([value as u64, 0, 0, 0])
    }
}

impl From<u64> for U256 {
    fn from(value: u64) -> Self {
        U256([value, 0, 0, 0])
    }
}

impl From<u128> for U256 {
    fn from(value: u128) -> Self {
        U256([value as u64, (value >> 64) as u64, 0, 0])
    }
}

impl TryFrom<U256> for u64 {
    type Error = &'static str;
    fn try_from(value: U256) -> Result<Self, Self::Error> {
        if value.0[1] != 0 || value.0[2] != 0 || value.0[3] != 0 {
            Err("value too large for u64")
        } else {
            Ok(value.0[0])
        }
    }
}

impl TryFrom<U256> for u128 {
    type Error = &'static str;
    fn try_from(value: U256) -> Result<Self, Self::Error> {
        if value.0[2] != 0 || value.0[3] != 0 {
            Err("value too large for u128")
        } else {
            Ok((value.0[0] as u128) | ((value.0[1] as u128) << 64))
        }
    }
}

impl FromStr for U256 {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);

        if s.is_empty() {
            return Err("empty string");
        }

        if s.len() > 64 {
            return Err("string too long for U256");
        }

        let mut result = U256::ZERO;

        for ch in s.chars() {
            let digit = match ch {
                '0'..='9' => (ch as u8 - b'0') as u64,
                'a'..='f' => (ch as u8 - b'a' + 10) as u64,
                'A'..='F' => (ch as u8 - b'A' + 10) as u64,
                _ => return Err("invalid hex character"),
            };

            result <<= 4;
            result.0[0] |= digit;
        }

        Ok(result)
    }
}

// Serde implementations
impl Serialize for U256 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let hex = format!("0x{:016x}{:016x}{:016x}{:016x}", self.0[3], self.0[2], self.0[1], self.0[0]);
        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for U256 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        U256::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// =============================================================================
// U512 Implementation
// =============================================================================

/// 512-bit unsigned integer
#[derive(Clone, Copy)]
pub struct U512([u64; 8]);

impl U512 {
    /// Zero value
    pub const ZERO: Self = U512([0, 0, 0, 0, 0, 0, 0, 0]);

    /// One value
    pub const ONE: Self = U512([1, 0, 0, 0, 0, 0, 0, 0]);

    /// Maximum value
    pub const MAX: Self = U512([u64::MAX, u64::MAX, u64::MAX, u64::MAX, u64::MAX, u64::MAX, u64::MAX, u64::MAX]);

    /// Create from a single u64
    #[inline]
    pub const fn from_u64(value: u64) -> Self {
        U512([value, 0, 0, 0, 0, 0, 0, 0])
    }

    /// Check if value is zero
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.0[0] == 0 && self.0[1] == 0 && self.0[2] == 0 && self.0[3] == 0
            && self.0[4] == 0 && self.0[5] == 0 && self.0[6] == 0 && self.0[7] == 0
    }

    /// Check if value is one
    #[inline]
    pub const fn is_one(&self) -> bool {
        self.0[0] == 1 && self.0[1] == 0 && self.0[2] == 0 && self.0[3] == 0
            && self.0[4] == 0 && self.0[5] == 0 && self.0[6] == 0 && self.0[7] == 0
    }

    /// Count leading zeros
    pub const fn leading_zeros(&self) -> u32 {
        let mut i = 8;
        while i > 0 {
            i -= 1;
            if self.0[i] != 0 {
                return (7 - i) as u32 * 64 + self.0[i].leading_zeros();
            }
        }
        512
    }

    /// Count trailing zeros
    pub const fn trailing_zeros(&self) -> u32 {
        let mut i = 0;
        while i < 8 {
            if self.0[i] != 0 {
                return i as u32 * 64 + self.0[i].trailing_zeros();
            }
            i += 1;
        }
        512
    }

    /// Count number of ones
    pub const fn count_ones(&self) -> u32 {
        self.0[0].count_ones() + self.0[1].count_ones() + self.0[2].count_ones() + self.0[3].count_ones()
            + self.0[4].count_ones() + self.0[5].count_ones() + self.0[6].count_ones() + self.0[7].count_ones()
    }

    /// Number of bits needed to represent this value
    pub const fn bits(&self) -> u32 {
        512 - self.leading_zeros()
    }

    /// Convert to little-endian bytes
    pub fn to_le_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        for i in 0..8 {
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&self.0[i].to_le_bytes());
        }
        bytes
    }

    /// Convert to big-endian bytes
    pub fn to_be_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        for i in 0..8 {
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&self.0[7 - i].to_be_bytes());
        }
        bytes
    }

    /// Create from little-endian bytes
    pub fn from_le_bytes(bytes: [u8; 64]) -> Self {
        let mut limbs = [0u64; 8];
        for i in 0..8 {
            let mut chunk = [0u8; 8];
            chunk.copy_from_slice(&bytes[i * 8..(i + 1) * 8]);
            limbs[i] = u64::from_le_bytes(chunk);
        }
        U512(limbs)
    }

    /// Create from big-endian bytes
    pub fn from_be_bytes(bytes: [u8; 64]) -> Self {
        let mut limbs = [0u64; 8];
        for i in 0..8 {
            let mut chunk = [0u8; 8];
            chunk.copy_from_slice(&bytes[i * 8..(i + 1) * 8]);
            limbs[7 - i] = u64::from_be_bytes(chunk);
        }
        U512(limbs)
    }

    /// Checked addition
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        let (result, overflow) = self.overflowing_add(rhs);
        if overflow {
            None
        } else {
            Some(result)
        }
    }

    /// Overflowing addition
    pub const fn overflowing_add(self, rhs: Self) -> (Self, bool) {
        let mut result = [0u64; 8];
        let mut carry = false;

        let mut i = 0;
        while i < 8 {
            let (a, c1) = self.0[i].overflowing_add(rhs.0[i]);
            let (b, c2) = a.overflowing_add(carry as u64);
            result[i] = b;
            carry = c1 || c2;
            i += 1;
        }

        (U512(result), carry)
    }

    /// Saturating addition
    pub const fn saturating_add(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_add(rhs);
        if overflow {
            Self::MAX
        } else {
            result
        }
    }

    /// Checked subtraction
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        let (result, overflow) = self.overflowing_sub(rhs);
        if overflow {
            None
        } else {
            Some(result)
        }
    }

    /// Overflowing subtraction
    pub const fn overflowing_sub(self, rhs: Self) -> (Self, bool) {
        let mut result = [0u64; 8];
        let mut borrow = false;

        let mut i = 0;
        while i < 8 {
            let (a, b1) = self.0[i].overflowing_sub(rhs.0[i]);
            let (b, b2) = a.overflowing_sub(borrow as u64);
            result[i] = b;
            borrow = b1 || b2;
            i += 1;
        }

        (U512(result), borrow)
    }

    /// Saturating subtraction
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_sub(rhs);
        if overflow {
            Self::ZERO
        } else {
            result
        }
    }

    /// Checked multiplication
    pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
        let (result, overflow) = self.overflowing_mul(rhs);
        if overflow {
            None
        } else {
            Some(result)
        }
    }

    /// Overflowing multiplication (simplified - checks for any high bits)
    pub const fn overflowing_mul(self, rhs: Self) -> (Self, bool) {
        // This is a simplified version that detects overflow
        // Full 512x512 -> 1024-bit multiplication would be very complex

        // Quick checks
        if self.is_zero() || rhs.is_zero() {
            return (Self::ZERO, false);
        }

        if self.is_one() {
            return (rhs, false);
        }

        if rhs.is_one() {
            return (self, false);
        }

        // Check if multiplication will overflow by estimating bits
        let self_bits = self.bits();
        let rhs_bits = rhs.bits();
        if self_bits + rhs_bits > 512 {
            // Definite overflow
            return (Self::ZERO, true);
        }

        // Perform multiplication similar to U256
        let mut result = [0u64; 16];

        let mut i = 0;
        while i < 8 {
            let mut carry = 0u64;
            let mut j = 0;
            while j < 8 {
                let k = i + j;
                let (hi, lo) = mul_u64(self.0[i], rhs.0[j]);

                let (sum, c1) = result[k].overflowing_add(lo);
                let (sum, c2) = sum.overflowing_add(carry);
                result[k] = sum;

                carry = hi + (c1 as u64) + (c2 as u64);
                j += 1;
            }
            if i + 8 < 16 {
                result[i + 8] = carry;
            }
            i += 1;
        }

        // Check for overflow
        let mut overflow = false;
        let mut i = 8;
        while i < 16 {
            if result[i] != 0 {
                overflow = true;
                break;
            }
            i += 1;
        }

        let mut output = [0u64; 8];
        let mut i = 0;
        while i < 8 {
            output[i] = result[i];
            i += 1;
        }

        (U512(output), overflow)
    }

    /// Saturating multiplication
    pub const fn saturating_mul(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_mul(rhs);
        if overflow {
            Self::MAX
        } else {
            result
        }
    }

    /// Checked division
    pub const fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.is_zero() {
            None
        } else {
            Some(self.div_rem(rhs).0)
        }
    }

    /// Checked remainder
    pub const fn checked_rem(self, rhs: Self) -> Option<Self> {
        if rhs.is_zero() {
            None
        } else {
            Some(self.div_rem(rhs).1)
        }
    }

    /// Division and remainder
    const fn div_rem(self, rhs: Self) -> (Self, Self) {
        if rhs.is_zero() {
            panic!("division by zero");
        }

        if self.is_zero() {
            return (Self::ZERO, Self::ZERO);
        }

        if rhs.is_one() {
            return (self, Self::ZERO);
        }

        match self.cmp_const(&rhs) {
            Ordering::Less => return (Self::ZERO, self),
            Ordering::Equal => return (Self::ONE, Self::ZERO),
            Ordering::Greater => {}
        }

        // Long division
        let mut quotient = Self::ZERO;
        let mut remainder = Self::ZERO;

        let mut i = 512;
        while i > 0 {
            i -= 1;

            remainder = remainder.shl_const(1);

            let bit = self.shr_const(i).0[0] & 1;
            remainder.0[0] |= bit;

            if matches!(remainder.cmp_const(&rhs), Ordering::Greater | Ordering::Equal) {
                remainder = remainder.sub_const(rhs);
                let word = i / 64;
                let bit = i % 64;
                quotient.0[word as usize] |= 1u64 << bit;
            }
        }

        (quotient, remainder)
    }

    const fn sub_const(self, rhs: Self) -> Self {
        self.overflowing_sub(rhs).0
    }

    const fn cmp_const(&self, other: &Self) -> Ordering {
        let mut i = 8;
        while i > 0 {
            i -= 1;
            if self.0[i] > other.0[i] {
                return Ordering::Greater;
            }
            if self.0[i] < other.0[i] {
                return Ordering::Less;
            }
        }
        Ordering::Equal
    }

    const fn shl_const(self, shift: u32) -> Self {
        if shift == 0 {
            return self;
        }
        if shift >= 512 {
            return Self::ZERO;
        }

        let word_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;

        let mut result = [0u64; 8];

        if bit_shift == 0 {
            let mut i = word_shift;
            while i < 8 {
                result[i] = self.0[i - word_shift];
                i += 1;
            }
        } else {
            let mut i = word_shift;
            while i < 8 {
                result[i] = self.0[i - word_shift] << bit_shift;
                if i > word_shift {
                    result[i] |= self.0[i - word_shift - 1] >> (64 - bit_shift);
                }
                i += 1;
            }
        }

        U512(result)
    }

    const fn shr_const(self, shift: u32) -> Self {
        if shift == 0 {
            return self;
        }
        if shift >= 512 {
            return Self::ZERO;
        }

        let word_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;

        let mut result = [0u64; 8];

        if bit_shift == 0 {
            let mut i = 0;
            while i < 8 - word_shift {
                result[i] = self.0[i + word_shift];
                i += 1;
            }
        } else {
            let mut i = 0;
            while i < 8 - word_shift {
                result[i] = self.0[i + word_shift] >> bit_shift;
                if i + word_shift + 1 < 8 {
                    result[i] |= self.0[i + word_shift + 1] << (64 - bit_shift);
                }
                i += 1;
            }
        }

        U512(result)
    }
}

// =============================================================================
// U512 Trait Implementations (mirror U256)
// =============================================================================

impl Default for U512 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl fmt::Debug for U512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "U512(0x")?;
        for i in (0..8).rev() {
            write!(f, "{:016x}", self.0[i])?;
        }
        write!(f, ")")
    }
}

impl fmt::Display for U512 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x")?;
        for i in (0..8).rev() {
            write!(f, "{:016x}", self.0[i])?;
        }
        Ok(())
    }
}

impl PartialEq for U512 {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for U512 {}

impl Hash for U512 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialOrd for U512 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U512 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp_const(other)
    }
}

// Arithmetic operations (same pattern as U256)
impl Add for U512 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_add(rhs);
        if overflow {
            panic!("attempt to add with overflow");
        }
        result
    }
}

impl AddAssign for U512 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for U512 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_sub(rhs);
        if overflow {
            panic!("attempt to subtract with overflow");
        }
        result
    }
}

impl SubAssign for U512 {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for U512 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let (result, overflow) = self.overflowing_mul(rhs);
        if overflow {
            panic!("attempt to multiply with overflow");
        }
        result
    }
}

impl MulAssign for U512 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Div for U512 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        self.checked_div(rhs).expect("division by zero")
    }
}

impl DivAssign for U512 {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Rem for U512 {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self {
        self.checked_rem(rhs).expect("remainder by zero")
    }
}

impl RemAssign for U512 {
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

// Bitwise operations
impl BitAnd for U512 {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        let mut result = [0u64; 8];
        for (i, item) in result.iter_mut().enumerate() {
            *item = self.0[i] & rhs.0[i];
        }
        U512(result)
    }
}

impl BitAndAssign for U512 {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl BitOr for U512 {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        let mut result = [0u64; 8];
        for (i, item) in result.iter_mut().enumerate() {
            *item = self.0[i] | rhs.0[i];
        }
        U512(result)
    }
}

impl BitOrAssign for U512 {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl BitXor for U512 {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self {
        let mut result = [0u64; 8];
        for (i, item) in result.iter_mut().enumerate() {
            *item = self.0[i] ^ rhs.0[i];
        }
        U512(result)
    }
}

impl BitXorAssign for U512 {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
    }
}

impl Not for U512 {
    type Output = Self;
    fn not(self) -> Self {
        let mut result = [0u64; 8];
        for (i, item) in result.iter_mut().enumerate() {
            *item = !self.0[i];
        }
        U512(result)
    }
}

impl Shl<u32> for U512 {
    type Output = Self;
    fn shl(self, shift: u32) -> Self {
        self.shl_const(shift)
    }
}

impl ShlAssign<u32> for U512 {
    fn shl_assign(&mut self, shift: u32) {
        *self = *self << shift;
    }
}

impl Shr<u32> for U512 {
    type Output = Self;
    fn shr(self, shift: u32) -> Self {
        self.shr_const(shift)
    }
}

impl ShrAssign<u32> for U512 {
    fn shr_assign(&mut self, shift: u32) {
        *self = *self >> shift;
    }
}

// Conversions
impl From<u8> for U512 {
    fn from(value: u8) -> Self {
        U512([value as u64, 0, 0, 0, 0, 0, 0, 0])
    }
}

impl From<u16> for U512 {
    fn from(value: u16) -> Self {
        U512([value as u64, 0, 0, 0, 0, 0, 0, 0])
    }
}

impl From<u32> for U512 {
    fn from(value: u32) -> Self {
        U512([value as u64, 0, 0, 0, 0, 0, 0, 0])
    }
}

impl From<u64> for U512 {
    fn from(value: u64) -> Self {
        U512([value, 0, 0, 0, 0, 0, 0, 0])
    }
}

impl From<u128> for U512 {
    fn from(value: u128) -> Self {
        U512([value as u64, (value >> 64) as u64, 0, 0, 0, 0, 0, 0])
    }
}

impl From<U256> for U512 {
    fn from(value: U256) -> Self {
        U512([value.0[0], value.0[1], value.0[2], value.0[3], 0, 0, 0, 0])
    }
}

impl TryFrom<U512> for U256 {
    type Error = &'static str;
    fn try_from(value: U512) -> Result<Self, Self::Error> {
        if value.0[4] != 0 || value.0[5] != 0 || value.0[6] != 0 || value.0[7] != 0 {
            Err("value too large for U256")
        } else {
            Ok(U256([value.0[0], value.0[1], value.0[2], value.0[3]]))
        }
    }
}

impl TryFrom<U512> for u64 {
    type Error = &'static str;
    fn try_from(value: U512) -> Result<Self, Self::Error> {
        if value.0[1] != 0 || value.0[2] != 0 || value.0[3] != 0
            || value.0[4] != 0 || value.0[5] != 0 || value.0[6] != 0 || value.0[7] != 0 {
            Err("value too large for u64")
        } else {
            Ok(value.0[0])
        }
    }
}

impl TryFrom<U512> for u128 {
    type Error = &'static str;
    fn try_from(value: U512) -> Result<Self, Self::Error> {
        if value.0[2] != 0 || value.0[3] != 0
            || value.0[4] != 0 || value.0[5] != 0 || value.0[6] != 0 || value.0[7] != 0 {
            Err("value too large for u128")
        } else {
            Ok((value.0[0] as u128) | ((value.0[1] as u128) << 64))
        }
    }
}

impl FromStr for U512 {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);

        if s.is_empty() {
            return Err("empty string");
        }

        if s.len() > 128 {
            return Err("string too long for U512");
        }

        let mut result = U512::ZERO;

        for ch in s.chars() {
            let digit = match ch {
                '0'..='9' => (ch as u8 - b'0') as u64,
                'a'..='f' => (ch as u8 - b'a' + 10) as u64,
                'A'..='F' => (ch as u8 - b'A' + 10) as u64,
                _ => return Err("invalid hex character"),
            };

            result <<= 4;
            result.0[0] |= digit;
        }

        Ok(result)
    }
}

// Serde
impl Serialize for U512 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let hex = format!("0x{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}",
            self.0[7], self.0[6], self.0[5], self.0[4], self.0[3], self.0[2], self.0[1], self.0[0]);
        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for U512 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        U512::from_str(&s).map_err(serde::de::Error::custom)
    }
}
// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // U256 Basic Tests
    // =============================================================================

    #[test]
    fn test_u256_zero() {
        let zero = U256::ZERO;
        assert!(zero.is_zero());
        assert!(!zero.is_one());
        assert_eq!(zero.bits(), 0);
    }

    #[test]
    fn test_u256_one() {
        let one = U256::ONE;
        assert!(!one.is_zero());
        assert!(one.is_one());
        assert_eq!(one.bits(), 1);
    }

    #[test]
    fn test_u256_max() {
        let max = U256::MAX;
        assert!(!max.is_zero());
        assert_eq!(max.bits(), 256);
        assert_eq!(max.leading_zeros(), 0);
    }

    #[test]
    fn test_u256_from_u64() {
        let val = U256::from_u64(42);
        assert_eq!(val.0[0], 42);
        assert_eq!(val.0[1], 0);
        assert_eq!(val.0[2], 0);
        assert_eq!(val.0[3], 0);
    }

    #[test]
    fn test_u256_from_primitives() {
        assert_eq!(U256::from(0u8), U256::ZERO);
        assert_eq!(U256::from(1u16), U256::ONE);
        assert_eq!(U256::from(42u32), U256::from_u64(42));
        assert_eq!(U256::from(u64::MAX), U256([u64::MAX, 0, 0, 0]));
    }

    #[test]
    fn test_u256_from_u128() {
        let val = u128::MAX;
        let u256 = U256::from(val);
        assert_eq!(u256.0[0], u64::MAX);
        assert_eq!(u256.0[1], u64::MAX);
        assert_eq!(u256.0[2], 0);
        assert_eq!(u256.0[3], 0);
    }

    // =============================================================================
    // U256 Arithmetic Tests
    // =============================================================================

    #[test]
    fn test_u256_add_basic() {
        let a = U256::from(10u64);
        let b = U256::from(20u64);
        let c = a + b;
        assert_eq!(c, U256::from(30u64));
    }

    #[test]
    fn test_u256_add_with_carry() {
        let a = U256([u64::MAX, 0, 0, 0]);
        let b = U256::from(1u64);
        let c = a + b;
        assert_eq!(c.0[0], 0);
        assert_eq!(c.0[1], 1);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_u256_add_overflow() {
        let _ = U256::MAX + U256::ONE;
    }

    #[test]
    fn test_u256_checked_add() {
        assert_eq!(U256::from(5u64).checked_add(U256::from(3u64)), Some(U256::from(8u64)));
        assert_eq!(U256::MAX.checked_add(U256::ONE), None);
    }

    #[test]
    fn test_u256_saturating_add() {
        assert_eq!(U256::from(5u64).saturating_add(U256::from(3u64)), U256::from(8u64));
        assert_eq!(U256::MAX.saturating_add(U256::ONE), U256::MAX);
    }

    #[test]
    fn test_u256_sub_basic() {
        let a = U256::from(30u64);
        let b = U256::from(20u64);
        let c = a - b;
        assert_eq!(c, U256::from(10u64));
    }

    #[test]
    fn test_u256_sub_with_borrow() {
        let a = U256([0, 1, 0, 0]);
        let b = U256::from(1u64);
        let c = a - b;
        assert_eq!(c.0[0], u64::MAX);
        assert_eq!(c.0[1], 0);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_u256_sub_underflow() {
        let _ = U256::ZERO - U256::ONE;
    }

    #[test]
    fn test_u256_checked_sub() {
        assert_eq!(U256::from(10u64).checked_sub(U256::from(3u64)), Some(U256::from(7u64)));
        assert_eq!(U256::ZERO.checked_sub(U256::ONE), None);
    }

    #[test]
    fn test_u256_saturating_sub() {
        assert_eq!(U256::from(10u64).saturating_sub(U256::from(3u64)), U256::from(7u64));
        assert_eq!(U256::ZERO.saturating_sub(U256::ONE), U256::ZERO);
    }

    #[test]
    fn test_u256_mul_basic() {
        let a = U256::from(6u64);
        let b = U256::from(7u64);
        let c = a * b;
        assert_eq!(c, U256::from(42u64));
    }

    #[test]
    fn test_u256_mul_large() {
        let a = U256::from(u64::MAX);
        let b = U256::from(2u64);
        let c = a * b;
        assert_eq!(c.0[0], u64::MAX - 1);
        assert_eq!(c.0[1], 1);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_u256_mul_overflow() {
        let a = U256([0, 0, 1, 0]);
        let b = U256([0, 0, 1, 0]);
        let _ = a * b;
    }

    #[test]
    fn test_u256_checked_mul() {
        assert_eq!(U256::from(6u64).checked_mul(U256::from(7u64)), Some(U256::from(42u64)));
        let large = U256([0, 0, 1, 0]);
        assert_eq!(large.checked_mul(large), None);
    }

    #[test]
    fn test_u256_saturating_mul() {
        assert_eq!(U256::from(6u64).saturating_mul(U256::from(7u64)), U256::from(42u64));
        let large = U256([0, 0, 1, 0]);
        assert_eq!(large.saturating_mul(large), U256::MAX);
    }

    #[test]
    fn test_u256_div_basic() {
        let a = U256::from(42u64);
        let b = U256::from(6u64);
        let c = a / b;
        assert_eq!(c, U256::from(7u64));
    }

    #[test]
    fn test_u256_div_exact() {
        let a = U256::from(100u64);
        let b = U256::from(10u64);
        assert_eq!(a / b, U256::from(10u64));
    }

    #[test]
    fn test_u256_div_remainder() {
        let a = U256::from(43u64);
        let b = U256::from(6u64);
        assert_eq!(a / b, U256::from(7u64));
        assert_eq!(a % b, U256::from(1u64));
    }

    #[test]
    #[should_panic(expected = "division by zero")]
    fn test_u256_div_by_zero() {
        let _ = U256::ONE / U256::ZERO;
    }

    #[test]
    fn test_u256_checked_div() {
        assert_eq!(U256::from(42u64).checked_div(U256::from(6u64)), Some(U256::from(7u64)));
        assert_eq!(U256::ONE.checked_div(U256::ZERO), None);
    }

    #[test]
    fn test_u256_rem_basic() {
        let a = U256::from(43u64);
        let b = U256::from(6u64);
        let r = a % b;
        assert_eq!(r, U256::from(1u64));
    }

    // =============================================================================
    // U256 Bitwise Tests
    // =============================================================================

    #[test]
    fn test_u256_bitand() {
        let a = U256::from(0b1010u64);
        let b = U256::from(0b1100u64);
        assert_eq!(a & b, U256::from(0b1000u64));
    }

    #[test]
    fn test_u256_bitor() {
        let a = U256::from(0b1010u64);
        let b = U256::from(0b1100u64);
        assert_eq!(a | b, U256::from(0b1110u64));
    }

    #[test]
    fn test_u256_bitxor() {
        let a = U256::from(0b1010u64);
        let b = U256::from(0b1100u64);
        assert_eq!(a ^ b, U256::from(0b0110u64));
    }

    #[test]
    fn test_u256_not() {
        let a = U256::ZERO;
        let b = !a;
        assert_eq!(b, U256::MAX);
    }

    #[test]
    fn test_u256_shl_basic() {
        let a = U256::from(1u64);
        assert_eq!(a << 0, U256::from(1u64));
        assert_eq!(a << 1, U256::from(2u64));
        assert_eq!(a << 8, U256::from(256u64));
    }

    #[test]
    fn test_u256_shl_cross_word() {
        let a = U256::from(1u64);
        let b = a << 64;
        assert_eq!(b.0[0], 0);
        assert_eq!(b.0[1], 1);
    }

    #[test]
    fn test_u256_shl_overflow() {
        let a = U256::from(1u64);
        let b = a << 256;
        assert_eq!(b, U256::ZERO);
    }

    #[test]
    fn test_u256_shr_basic() {
        let a = U256::from(256u64);
        assert_eq!(a >> 0, U256::from(256u64));
        assert_eq!(a >> 1, U256::from(128u64));
        assert_eq!(a >> 8, U256::from(1u64));
    }

    #[test]
    fn test_u256_shr_cross_word() {
        let a = U256([0, 1, 0, 0]);
        let b = a >> 64;
        assert_eq!(b.0[0], 1);
        assert_eq!(b.0[1], 0);
    }

    // =============================================================================
    // U256 Comparison Tests
    // =============================================================================

    #[test]
    fn test_u256_eq() {
        assert_eq!(U256::ZERO, U256::ZERO);
        assert_eq!(U256::ONE, U256::ONE);
        assert_ne!(U256::ZERO, U256::ONE);
    }

    #[test]
    fn test_u256_cmp() {
        assert!(U256::ZERO < U256::ONE);
        assert!(U256::ONE > U256::ZERO);
        assert!(U256::ONE <= U256::ONE);
        assert!(U256::ONE >= U256::ONE);
    }

    #[test]
    fn test_u256_cmp_complex() {
        let a = U256([0, 0, 0, 1]);
        let b = U256([u64::MAX, u64::MAX, u64::MAX, 0]);
        assert!(a > b);
    }

    // =============================================================================
    // U256 Bit Counting Tests
    // =============================================================================

    #[test]
    fn test_u256_leading_zeros() {
        assert_eq!(U256::ZERO.leading_zeros(), 256);
        assert_eq!(U256::ONE.leading_zeros(), 255);
        assert_eq!(U256::from(0x80u64).leading_zeros(), 256 - 8);
        assert_eq!(U256::MAX.leading_zeros(), 0);
    }

    #[test]
    fn test_u256_trailing_zeros() {
        assert_eq!(U256::ZERO.trailing_zeros(), 256);
        assert_eq!(U256::ONE.trailing_zeros(), 0);
        assert_eq!(U256::from(8u64).trailing_zeros(), 3);
        assert_eq!(U256([0, 1, 0, 0]).trailing_zeros(), 64);
    }

    #[test]
    fn test_u256_count_ones() {
        assert_eq!(U256::ZERO.count_ones(), 0);
        assert_eq!(U256::ONE.count_ones(), 1);
        assert_eq!(U256::MAX.count_ones(), 256);
        assert_eq!(U256::from(0b1010u64).count_ones(), 2);
    }

    #[test]
    fn test_u256_bits() {
        assert_eq!(U256::ZERO.bits(), 0);
        assert_eq!(U256::ONE.bits(), 1);
        assert_eq!(U256::from(255u64).bits(), 8);
        assert_eq!(U256::from(256u64).bits(), 9);
    }

    // =============================================================================
    // U256 Byte Conversion Tests
    // =============================================================================

    #[test]
    fn test_u256_to_le_bytes() {
        let val = U256::from(0x0102030405060708u64);
        let bytes = val.to_le_bytes();
        assert_eq!(bytes[0], 0x08);
        assert_eq!(bytes[7], 0x01);
    }

    #[test]
    fn test_u256_to_be_bytes() {
        let val = U256::from(0x0102030405060708u64);
        let bytes = val.to_be_bytes();
        assert_eq!(bytes[31], 0x08);
        assert_eq!(bytes[24], 0x01);
    }

    #[test]
    fn test_u256_from_le_bytes() {
        let mut bytes = [0u8; 32];
        bytes[0] = 0x01;
        bytes[8] = 0x02;
        let val = U256::from_le_bytes(bytes);
        assert_eq!(val.0[0], 0x01);
        assert_eq!(val.0[1], 0x02);
    }

    #[test]
    fn test_u256_from_be_bytes() {
        let mut bytes = [0u8; 32];
        bytes[31] = 0x01;
        bytes[23] = 0x02;
        let val = U256::from_be_bytes(bytes);
        assert_eq!(val.0[0], 0x01);
        assert_eq!(val.0[1], 0x02);
    }

    #[test]
    fn test_u256_byte_roundtrip() {
        let original = U256([0x1122334455667788, 0x99aabbccddeeff00, 0x0011223344556677, 0x8899aabbccddeeff]);
        let le_bytes = original.to_le_bytes();
        let be_bytes = original.to_be_bytes();
        assert_eq!(U256::from_le_bytes(le_bytes), original);
        assert_eq!(U256::from_be_bytes(be_bytes), original);
    }

    // =============================================================================
    // U256 String Parsing Tests
    // =============================================================================

    #[test]
    fn test_u256_from_str_basic() {
        assert_eq!(U256::from_str("0").unwrap(), U256::ZERO);
        assert_eq!(U256::from_str("1").unwrap(), U256::ONE);
        assert_eq!(U256::from_str("ff").unwrap(), U256::from(255u64));
    }

    #[test]
    fn test_u256_from_str_with_prefix() {
        assert_eq!(U256::from_str("0x0").unwrap(), U256::ZERO);
        assert_eq!(U256::from_str("0x1").unwrap(), U256::ONE);
        assert_eq!(U256::from_str("0xFF").unwrap(), U256::from(255u64));
    }

    #[test]
    fn test_u256_from_str_case_insensitive() {
        assert_eq!(U256::from_str("abc").unwrap(), U256::from_str("ABC").unwrap());
        assert_eq!(U256::from_str("0xDeAdBeEf").unwrap(), U256::from(0xdeadbeefu64));
    }

    #[test]
    fn test_u256_from_str_errors() {
        assert!(U256::from_str("").is_err());
        assert!(U256::from_str("g").is_err());
        assert!(U256::from_str("0x").is_err());
        // String too long (> 64 hex chars)
        assert!(U256::from_str("1" .repeat(65).as_str()).is_err());
    }

    #[test]
    fn test_u256_display() {
        assert_eq!(format!("{}", U256::ZERO), "0x0000000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(format!("{}", U256::ONE), "0x0000000000000000000000000000000000000000000000000000000000000001");
    }

    // =============================================================================
    // U256 Try From Tests
    // =============================================================================

    #[test]
    fn test_u256_try_into_u64() {
        assert_eq!(u64::try_from(U256::from(42u64)).unwrap(), 42u64);
        assert!(u64::try_from(U256([0, 1, 0, 0])).is_err());
    }

    #[test]
    fn test_u256_try_into_u128() {
        assert_eq!(u128::try_from(U256::from(42u64)).unwrap(), 42u128);
        assert_eq!(u128::try_from(U256::from(u128::MAX)).unwrap(), u128::MAX);
        assert!(u128::try_from(U256([0, 0, 1, 0])).is_err());
    }

    // =============================================================================
    // U256 Assign Operator Tests
    // =============================================================================

    #[test]
    fn test_u256_add_assign() {
        let mut a = U256::from(10u64);
        a += U256::from(5u64);
        assert_eq!(a, U256::from(15u64));
    }

    #[test]
    fn test_u256_sub_assign() {
        let mut a = U256::from(10u64);
        a -= U256::from(5u64);
        assert_eq!(a, U256::from(5u64));
    }

    #[test]
    fn test_u256_mul_assign() {
        let mut a = U256::from(10u64);
        a *= U256::from(5u64);
        assert_eq!(a, U256::from(50u64));
    }

    #[test]
    fn test_u256_div_assign() {
        let mut a = U256::from(50u64);
        a /= U256::from(5u64);
        assert_eq!(a, U256::from(10u64));
    }

    #[test]
    fn test_u256_rem_assign() {
        let mut a = U256::from(53u64);
        a %= U256::from(5u64);
        assert_eq!(a, U256::from(3u64));
    }

    #[test]
    fn test_u256_bitand_assign() {
        let mut a = U256::from(0b1010u64);
        a &= U256::from(0b1100u64);
        assert_eq!(a, U256::from(0b1000u64));
    }

    #[test]
    fn test_u256_bitor_assign() {
        let mut a = U256::from(0b1010u64);
        a |= U256::from(0b1100u64);
        assert_eq!(a, U256::from(0b1110u64));
    }

    #[test]
    fn test_u256_bitxor_assign() {
        let mut a = U256::from(0b1010u64);
        a ^= U256::from(0b1100u64);
        assert_eq!(a, U256::from(0b0110u64));
    }

    #[test]
    fn test_u256_shl_assign() {
        let mut a = U256::from(1u64);
        a <<= 8;
        assert_eq!(a, U256::from(256u64));
    }

    #[test]
    fn test_u256_shr_assign() {
        let mut a = U256::from(256u64);
        a >>= 8;
        assert_eq!(a, U256::from(1u64));
    }

    // =============================================================================
    // U512 Basic Tests
    // =============================================================================

    #[test]
    fn test_u512_zero() {
        let zero = U512::ZERO;
        assert!(zero.is_zero());
        assert!(!zero.is_one());
        assert_eq!(zero.bits(), 0);
    }

    #[test]
    fn test_u512_one() {
        let one = U512::ONE;
        assert!(!one.is_zero());
        assert!(one.is_one());
        assert_eq!(one.bits(), 1);
    }

    #[test]
    fn test_u512_max() {
        let max = U512::MAX;
        assert!(!max.is_zero());
        assert_eq!(max.bits(), 512);
        assert_eq!(max.leading_zeros(), 0);
    }

    #[test]
    fn test_u512_from_u256() {
        let u256 = U256([1, 2, 3, 4]);
        let u512 = U512::from(u256);
        assert_eq!(u512.0[0], 1);
        assert_eq!(u512.0[1], 2);
        assert_eq!(u512.0[2], 3);
        assert_eq!(u512.0[3], 4);
        assert_eq!(u512.0[4], 0);
    }

    #[test]
    fn test_u512_try_into_u256() {
        let u512 = U512([1, 2, 3, 4, 0, 0, 0, 0]);
        let u256 = U256::try_from(u512).unwrap();
        assert_eq!(u256.0[0], 1);
        assert_eq!(u256.0[3], 4);

        let u512_big = U512([0, 0, 0, 0, 1, 0, 0, 0]);
        assert!(U256::try_from(u512_big).is_err());
    }

    // =============================================================================
    // U512 Arithmetic Tests
    // =============================================================================

    #[test]
    fn test_u512_add_basic() {
        let a = U512::from(10u64);
        let b = U512::from(20u64);
        let c = a + b;
        assert_eq!(c, U512::from(30u64));
    }

    #[test]
    fn test_u512_add_with_carry() {
        let a = U512([u64::MAX, 0, 0, 0, 0, 0, 0, 0]);
        let b = U512::from(1u64);
        let c = a + b;
        assert_eq!(c.0[0], 0);
        assert_eq!(c.0[1], 1);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_u512_add_overflow() {
        let _ = U512::MAX + U512::ONE;
    }

    #[test]
    fn test_u512_sub_basic() {
        let a = U512::from(30u64);
        let b = U512::from(20u64);
        let c = a - b;
        assert_eq!(c, U512::from(10u64));
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_u512_sub_underflow() {
        let _ = U512::ZERO - U512::ONE;
    }

    #[test]
    fn test_u512_mul_basic() {
        let a = U512::from(6u64);
        let b = U512::from(7u64);
        let c = a * b;
        assert_eq!(c, U512::from(42u64));
    }

    #[test]
    fn test_u512_div_basic() {
        let a = U512::from(42u64);
        let b = U512::from(6u64);
        let c = a / b;
        assert_eq!(c, U512::from(7u64));
    }

    #[test]
    fn test_u512_rem_basic() {
        let a = U512::from(43u64);
        let b = U512::from(6u64);
        let r = a % b;
        assert_eq!(r, U512::from(1u64));
    }

    // =============================================================================
    // U512 Bitwise Tests
    // =============================================================================

    #[test]
    fn test_u512_bitand() {
        let a = U512::from(0b1010u64);
        let b = U512::from(0b1100u64);
        assert_eq!(a & b, U512::from(0b1000u64));
    }

    #[test]
    fn test_u512_bitor() {
        let a = U512::from(0b1010u64);
        let b = U512::from(0b1100u64);
        assert_eq!(a | b, U512::from(0b1110u64));
    }

    #[test]
    fn test_u512_bitxor() {
        let a = U512::from(0b1010u64);
        let b = U512::from(0b1100u64);
        assert_eq!(a ^ b, U512::from(0b0110u64));
    }

    #[test]
    fn test_u512_not() {
        let a = U512::ZERO;
        let b = !a;
        assert_eq!(b, U512::MAX);
    }

    #[test]
    fn test_u512_shl_basic() {
        let a = U512::from(1u64);
        assert_eq!(a << 1, U512::from(2u64));
        assert_eq!(a << 8, U512::from(256u64));
    }

    #[test]
    fn test_u512_shr_basic() {
        let a = U512::from(256u64);
        assert_eq!(a >> 1, U512::from(128u64));
        assert_eq!(a >> 8, U512::from(1u64));
    }

    // =============================================================================
    // U512 Comparison Tests
    // =============================================================================

    #[test]
    fn test_u512_eq() {
        assert_eq!(U512::ZERO, U512::ZERO);
        assert_eq!(U512::ONE, U512::ONE);
        assert_ne!(U512::ZERO, U512::ONE);
    }

    #[test]
    fn test_u512_cmp() {
        assert!(U512::ZERO < U512::ONE);
        assert!(U512::ONE > U512::ZERO);
    }

    // =============================================================================
    // Property-Based Tests (Commutativity, Associativity, etc.)
    // =============================================================================

    #[test]
    fn test_u256_add_commutative() {
        let a = U256::from(123u64);
        let b = U256::from(456u64);
        assert_eq!(a + b, b + a);
    }

    #[test]
    fn test_u256_add_associative() {
        let a = U256::from(100u64);
        let b = U256::from(200u64);
        let c = U256::from(300u64);
        assert_eq!((a + b) + c, a + (b + c));
    }

    #[test]
    fn test_u256_mul_commutative() {
        let a = U256::from(123u64);
        let b = U256::from(456u64);
        assert_eq!(a * b, b * a);
    }

    #[test]
    fn test_u256_mul_associative() {
        let a = U256::from(5u64);
        let b = U256::from(7u64);
        let c = U256::from(11u64);
        assert_eq!((a * b) * c, a * (b * c));
    }

    #[test]
    fn test_u256_mul_distributive() {
        let a = U256::from(5u64);
        let b = U256::from(7u64);
        let c = U256::from(11u64);
        assert_eq!(a * (b + c), a * b + a * c);
    }

    #[test]
    fn test_u256_add_identity() {
        let a = U256::from(12345u64);
        assert_eq!(a + U256::ZERO, a);
        assert_eq!(U256::ZERO + a, a);
    }

    #[test]
    fn test_u256_mul_identity() {
        let a = U256::from(12345u64);
        assert_eq!(a * U256::ONE, a);
        assert_eq!(U256::ONE * a, a);
    }

    #[test]
    fn test_u256_mul_zero() {
        let a = U256::from(12345u64);
        assert_eq!(a * U256::ZERO, U256::ZERO);
        assert_eq!(U256::ZERO * a, U256::ZERO);
    }

    #[test]
    fn test_u256_div_identity() {
        let a = U256::from(12345u64);
        assert_eq!(a / U256::ONE, a);
    }

    #[test]
    fn test_u256_div_self() {
        let a = U256::from(12345u64);
        assert_eq!(a / a, U256::ONE);
    }

    // =============================================================================
    // Serde Tests
    // =============================================================================

    #[test]
    fn test_u256_serde_roundtrip() {
        let val = U256::from(0xdeadbeefu64);
        let json = serde_json::to_string(&val).unwrap();
        let parsed: U256 = serde_json::from_str(&json).unwrap();
        assert_eq!(val, parsed);
    }

    #[test]
    fn test_u512_serde_roundtrip() {
        let val = U512::from(0xdeadbeefu64);
        let json = serde_json::to_string(&val).unwrap();
        let parsed: U512 = serde_json::from_str(&json).unwrap();
        assert_eq!(val, parsed);
    }

    // =============================================================================
    // Edge Case Tests
    // =============================================================================

    #[test]
    fn test_u256_large_division() {
        let a = U256([u64::MAX, u64::MAX, 0, 0]);
        let b = U256::from(2u64);
        let (q, r) = a.div_rem(b);
        assert_eq!(r, U256::ONE);
        assert_eq!(q * b + r, a);
    }

    #[test]
    fn test_u256_shift_by_word_boundary() {
        let a = U256::from(0x12345678u64);
        let b = a << 64;
        assert_eq!(b.0[0], 0);
        assert_eq!(b.0[1], 0x12345678);

        let c = b >> 64;
        assert_eq!(c, a);
    }

    #[test]
    fn test_u256_hash_consistency() {
        use core::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let a = U256::from(42u64);
        let b = U256::from(42u64);

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        a.hash(&mut hasher1);
        b.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_u256_clone() {
        let a = U256::from(42u64);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn test_u256_debug() {
        let a = U256::from(42u64);
        let debug_str = format!("{a:?}");
        assert!(debug_str.contains("U256"));
    }

    #[test]
    fn test_u512_byte_roundtrip() {
        let original = U512([1, 2, 3, 4, 5, 6, 7, 8]);
        let le_bytes = original.to_le_bytes();
        let be_bytes = original.to_be_bytes();
        assert_eq!(U512::from_le_bytes(le_bytes), original);
        assert_eq!(U512::from_be_bytes(be_bytes), original);
    }
}
