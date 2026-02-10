//! secp256k1 finite-field utilities (mod p and mod n).
//!
//! This module provides basic modular arithmetic helpers needed for
//! implementing in-tree ECDSA verification and recovery.

#![allow(dead_code)]

use crate::types::{U256, U512};

const SECP256K1_P_BYTES: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xfe, 0xff, 0xff, 0xfc, 0x2f,
];

const SECP256K1_N_BYTES: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe,
    0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b,
    0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36, 0x41, 0x41,
];

/// secp256k1 field prime p.
pub(crate) fn secp256k1_p() -> U256 {
    U256::from_be_bytes(SECP256K1_P_BYTES)
}

/// secp256k1 curve order n.
pub(crate) fn secp256k1_n() -> U256 {
    U256::from_be_bytes(SECP256K1_N_BYTES)
}

/// secp256k1 curve parameter b (y^2 = x^3 + 7).
pub(crate) const SECP256K1_B: U256 = U256::from_u64(7);

fn mod_reduce(value: U512, modulus: U256) -> U256 {
    debug_assert!(!modulus.is_zero());
    let reduced = value % U512::from(modulus);
    U256::try_from(reduced).expect("modular reduction fits into U256")
}

pub(crate) fn mod_add(a: U256, b: U256, modulus: U256) -> U256 {
    mod_reduce(U512::from(a) + U512::from(b), modulus)
}

pub(crate) fn mod_sub(a: U256, b: U256, modulus: U256) -> U256 {
    mod_reduce(U512::from(a) + U512::from(modulus) - U512::from(b), modulus)
}

pub(crate) fn mod_mul(a: U256, b: U256, modulus: U256) -> U256 {
    mod_reduce(U512::from(a) * U512::from(b), modulus)
}

pub(crate) fn mod_pow(mut base: U256, mut exp: U256, modulus: U256) -> U256 {
    if modulus.is_zero() {
        return U256::ZERO;
    }

    let mut result = U256::ONE;
    base = mod_reduce(U512::from(base), modulus);

    while !exp.is_zero() {
        if (exp & U256::ONE) == U256::ONE {
            result = mod_mul(result, base, modulus);
        }
        base = mod_mul(base, base, modulus);
        exp >>= 1;
    }

    result
}

/// Modular inverse using Fermat's little theorem.
///
/// Returns `None` if `value` is zero or `modulus` is zero.
pub(crate) fn mod_inv(value: U256, modulus: U256) -> Option<U256> {
    if value.is_zero() || modulus.is_zero() {
        return None;
    }
    let exponent = modulus - U256::from_u64(2);
    Some(mod_pow(value, exponent, modulus))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_add_wraps() {
        let modulus = secp256k1_p();
        let result = mod_add(modulus - U256::ONE, U256::from_u64(2), modulus);
        assert_eq!(result, U256::ONE);
    }

    #[test]
    fn test_mod_sub_wraps() {
        let modulus = secp256k1_p();
        let result = mod_sub(U256::ONE, U256::from_u64(2), modulus);
        assert_eq!(result, modulus - U256::ONE);
    }

    #[test]
    fn test_mod_mul_wraps() {
        let modulus = secp256k1_p();
        let result = mod_mul(modulus - U256::ONE, modulus - U256::ONE, modulus);
        assert_eq!(result, U256::ONE);
    }

    #[test]
    fn test_mod_pow_small() {
        let modulus = U256::from_u64(17);
        let base = U256::from_u64(5);
        let exp = U256::from_u64(3);
        let result = mod_pow(base, exp, modulus);
        assert_eq!(result, U256::from_u64(6));
    }

    #[test]
    fn test_mod_inv_two() {
        let modulus = secp256k1_p();
        let inv_two = mod_inv(U256::from_u64(2), modulus).expect("inverse exists");
        let expected = (modulus + U256::ONE) / U256::from_u64(2);
        assert_eq!(inv_two, expected);
        let check = mod_mul(U256::from_u64(2), inv_two, modulus);
        assert_eq!(check, U256::ONE);
    }
}
