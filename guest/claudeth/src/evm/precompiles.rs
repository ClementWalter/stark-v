//! EVM precompile implementations.
//!
//! Currently implements:
//! - 0x01: ECRECOVER
//! - 0x02: SHA256
//! - 0x03: RIPEMD160
//! - 0x04: IDENTITY
//! - 0x05: MODEXP (Cancun gas formula / EIP-2565)

#[cfg(target_arch = "riscv32")]
extern crate alloc;

use core::cmp::Ordering;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::crypto::secp256k1::recover_address;
use crate::crypto::secp256k1_math::secp256k1_n;
use crate::crypto::{ripemd160, sha256};
use crate::evm::gas::{GAS_ECRECOVER, GAS_IDENTITY_BASE, GAS_IDENTITY_WORD, GAS_MODEXP_BASE};
use crate::evm::gas::{GAS_RIPEMD160_BASE, GAS_RIPEMD160_WORD, GAS_SHA256_BASE, GAS_SHA256_WORD};
use crate::types::{Address, Hash, U256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecompileResult {
    pub output: Vec<u8>,
    pub gas_used: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrecompileError {
    OutOfGas,
}

/// Execute a precompile with explicit gas metering.
///
/// We return `Some(Err(OutOfGas))` instead of bubbling an interpreter error so
/// CALL/STATICCALL semantics match the EVM: failed sub-call, not caller halt.
pub fn execute_precompile(
    address: &Address,
    input: &[u8],
    available_gas: u64,
) -> Option<Result<PrecompileResult, PrecompileError>> {
    let id = precompile_id(address)?;
    match id {
        1 => Some(meter_precompile_result(ecrecover_precompile(input), available_gas)),
        2 => Some(meter_precompile_result(sha256_precompile(input), available_gas)),
        3 => Some(meter_precompile_result(ripemd160_precompile(input), available_gas)),
        4 => Some(meter_precompile_result(identity_precompile(input), available_gas)),
        5 => Some(modexp_precompile(input, available_gas)),
        _ => None,
    }
}

fn meter_precompile_result(
    result: PrecompileResult,
    available_gas: u64,
) -> Result<PrecompileResult, PrecompileError> {
    if result.gas_used > available_gas {
        return Err(PrecompileError::OutOfGas);
    }
    Ok(result)
}

fn precompile_id(address: &Address) -> Option<u8> {
    let bytes = address.as_bytes();
    if bytes[..19].iter().any(|&b| b != 0) {
        return None;
    }
    let id = bytes[19];
    if id == 0 { None } else { Some(id) }
}

fn identity_precompile(input: &[u8]) -> PrecompileResult {
    let word_count = (input.len() as u64).div_ceil(32);
    PrecompileResult {
        output: input.to_vec(),
        gas_used: GAS_IDENTITY_BASE + GAS_IDENTITY_WORD * word_count,
    }
}

fn sha256_precompile(input: &[u8]) -> PrecompileResult {
    // Precompile gas depends on input length, so we round up to 32-byte words.
    let word_count = (input.len() as u64).div_ceil(32);
    let digest = sha256(input);
    PrecompileResult {
        output: digest.to_vec(),
        gas_used: GAS_SHA256_BASE + GAS_SHA256_WORD * word_count,
    }
}

fn ripemd160_precompile(input: &[u8]) -> PrecompileResult {
    // Precompile gas depends on input length, so we round up to 32-byte words.
    let word_count = (input.len() as u64).div_ceil(32);
    let digest = ripemd160(input);
    let mut output = vec![0u8; 32];
    // RIPEMD-160 returns 20 bytes, so we left-pad to 32 bytes to match EVM output rules.
    output[12..].copy_from_slice(&digest);
    PrecompileResult {
        output,
        gas_used: GAS_RIPEMD160_BASE + GAS_RIPEMD160_WORD * word_count,
    }
}

fn ecrecover_precompile(input: &[u8]) -> PrecompileResult {
    let message_hash = Hash::from(read_padded_word(input, 0));
    let v = U256::from_be_bytes(read_padded_word(input, 32));
    let r = U256::from_be_bytes(read_padded_word(input, 64));
    let s = U256::from_be_bytes(read_padded_word(input, 96));

    let output = if v == U256::from_u64(27) || v == U256::from_u64(28) {
        let n = secp256k1_n();
        if r.is_zero() || s.is_zero() || r >= n || s >= n {
            Vec::new()
        } else {
            let mut signature = [0u8; 64];
            signature[..32].copy_from_slice(&r.to_be_bytes());
            signature[32..].copy_from_slice(&s.to_be_bytes());
            let recovery_id = (v - U256::from_u64(27)).as_u64() as u8;
            match recover_address(&message_hash, &signature, recovery_id) {
                Ok(address) => {
                    let mut out = vec![0u8; 32];
                    out[12..].copy_from_slice(address.as_bytes());
                    out
                }
                Err(_) => Vec::new(),
            }
        }
    } else {
        Vec::new()
    };

    PrecompileResult {
        output,
        gas_used: GAS_ECRECOVER,
    }
}

fn modexp_precompile(input: &[u8], available_gas: u64) -> Result<PrecompileResult, PrecompileError> {
    let base_length = U256::from_be_bytes(read_padded_word(input, 0));
    let exponent_length = U256::from_be_bytes(read_padded_word(input, 32));
    let modulus_length = U256::from_be_bytes(read_padded_word(input, 64));

    let exp_start = U256::from_u64(96).saturating_add(base_length);
    let exp_head_len = if exponent_length > U256::from_u64(32) {
        32usize
    } else {
        exponent_length.as_u64() as usize
    };
    let exp_head_bytes = read_padded_bytes_u256(input, exp_start, exp_head_len);
    let exponent_head = u256_from_variable_be_bytes(&exp_head_bytes);

    let gas_cost = modexp_gas_cost(base_length, modulus_length, exponent_length, exponent_head);
    if gas_cost > U256::from_u64(available_gas) {
        return Err(PrecompileError::OutOfGas);
    }

    if base_length.is_zero() && modulus_length.is_zero() {
        return Ok(PrecompileResult {
            output: Vec::new(),
            gas_used: gas_cost.as_u64(),
        });
    }

    let Some(base_len) = u256_to_usize(base_length) else {
        return Err(PrecompileError::OutOfGas);
    };
    let Some(exp_len) = u256_to_usize(exponent_length) else {
        return Err(PrecompileError::OutOfGas);
    };
    let Some(mod_len) = u256_to_usize(modulus_length) else {
        return Err(PrecompileError::OutOfGas);
    };

    let exp_start = 96usize.saturating_add(base_len);
    let modulus_start = exp_start.saturating_add(exp_len);

    let base_bytes = read_padded_bytes(input, 96, base_len);
    let exponent_bytes = read_padded_bytes(input, exp_start, exp_len);
    let modulus_bytes = read_padded_bytes(input, modulus_start, mod_len);

    if mod_len == 0 {
        return Ok(PrecompileResult {
            output: Vec::new(),
            gas_used: gas_cost.as_u64(),
        });
    }

    let modulus = BigUint::from_be_bytes(&modulus_bytes);
    if modulus.is_zero() {
        return Ok(PrecompileResult {
            output: vec![0u8; mod_len],
            gas_used: gas_cost.as_u64(),
        });
    }

    // We reduce the base before exponentiation because MODEXP is defined over
    // integer values and reduction keeps intermediate arithmetic bounded.
    let base = bytes_modulo(&base_bytes, &modulus);
    let result = modular_exponentiation(&base, &exponent_bytes, &modulus);

    Ok(PrecompileResult {
        output: result.to_be_bytes_padded(mod_len),
        gas_used: gas_cost.as_u64(),
    })
}

fn modexp_gas_cost(
    base_length: U256,
    modulus_length: U256,
    exponent_length: U256,
    exponent_head: U256,
) -> U256 {
    let complexity = modexp_complexity(base_length, modulus_length);
    let iterations = modexp_iterations(exponent_length, exponent_head);
    let mut cost = complexity.saturating_mul(iterations) / U256::from_u64(3);
    if cost < U256::from_u64(GAS_MODEXP_BASE) {
        cost = U256::from_u64(GAS_MODEXP_BASE);
    }
    cost
}

fn modexp_complexity(base_length: U256, modulus_length: U256) -> U256 {
    let max_length = if base_length > modulus_length {
        base_length
    } else {
        modulus_length
    };
    let words = max_length.saturating_add(U256::from_u64(7)) / U256::from_u64(8);
    words.saturating_mul(words)
}

fn modexp_iterations(exponent_length: U256, exponent_head: U256) -> U256 {
    let mut count = if exponent_length <= U256::from_u64(32) && exponent_head.is_zero() {
        U256::ZERO
    } else if exponent_length <= U256::from_u64(32) {
        let bits = exponent_head.bits() as u64;
        U256::from_u64(bits.saturating_sub(1))
    } else {
        let length_part = exponent_length
            .saturating_sub(U256::from_u64(32))
            .saturating_mul(U256::from_u64(8));
        let bits = exponent_head.bits() as u64;
        let bits_part = U256::from_u64(bits.saturating_sub(1));
        length_part.saturating_add(bits_part)
    };

    if count.is_zero() {
        count = U256::ONE;
    }
    count
}

fn u256_from_variable_be_bytes(bytes: &[u8]) -> U256 {
    let mut padded = [0u8; 32];
    let copy_len = bytes.len().min(32);
    padded[32 - copy_len..].copy_from_slice(&bytes[bytes.len().saturating_sub(copy_len)..]);
    U256::from_be_bytes(padded)
}

fn u256_to_usize(value: U256) -> Option<usize> {
    if value > U256::from_u64(usize::MAX as u64) {
        return None;
    }
    Some(value.as_u64() as usize)
}

fn read_padded_word(input: &[u8], offset: usize) -> [u8; 32] {
    let mut word = [0u8; 32];
    if offset >= input.len() {
        return word;
    }
    let end = (offset + 32).min(input.len());
    word[..(end - offset)].copy_from_slice(&input[offset..end]);
    word
}

fn read_padded_bytes(input: &[u8], offset: usize, len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    if len == 0 || offset >= input.len() {
        return out;
    }

    let available = input.len().saturating_sub(offset);
    let copy_len = available.min(len);
    out[..copy_len].copy_from_slice(&input[offset..offset + copy_len]);
    out
}

fn read_padded_bytes_u256(input: &[u8], offset: U256, len: usize) -> Vec<u8> {
    let Some(offset) = u256_to_usize(offset) else {
        return vec![0u8; len];
    };
    read_padded_bytes(input, offset, len)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BigUint {
    limbs: Vec<u32>,
}

impl BigUint {
    fn zero() -> Self {
        Self { limbs: Vec::new() }
    }

    fn one() -> Self {
        Self { limbs: vec![1] }
    }

    fn from_be_bytes(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return Self::zero();
        }

        let mut limbs = Vec::with_capacity(bytes.len().div_ceil(4));
        let mut end = bytes.len();
        while end > 0 {
            let start = end.saturating_sub(4);
            let mut chunk = [0u8; 4];
            let chunk_len = end - start;
            chunk[4 - chunk_len..].copy_from_slice(&bytes[start..end]);
            limbs.push(u32::from_be_bytes(chunk));
            end = start;
        }

        let mut value = Self { limbs };
        value.normalize();
        value
    }

    fn to_be_bytes(&self) -> Vec<u8> {
        if self.is_zero() {
            return vec![0u8];
        }

        let mut out = Vec::with_capacity(self.limbs.len() * 4);
        for limb in self.limbs.iter().rev() {
            out.extend_from_slice(&limb.to_be_bytes());
        }

        let first_non_zero = out.iter().position(|&b| b != 0).unwrap_or(out.len() - 1);
        out[first_non_zero..].to_vec()
    }

    fn to_be_bytes_padded(&self, len: usize) -> Vec<u8> {
        if len == 0 {
            return Vec::new();
        }

        let raw = self.to_be_bytes();
        if raw.len() >= len {
            return raw[raw.len() - len..].to_vec();
        }

        let mut out = vec![0u8; len];
        out[len - raw.len()..].copy_from_slice(&raw);
        out
    }

    fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    fn normalize(&mut self) {
        while self.limbs.last().copied() == Some(0) {
            self.limbs.pop();
        }
    }

    fn cmp(&self, other: &Self) -> Ordering {
        if self.limbs.len() > other.limbs.len() {
            return Ordering::Greater;
        }
        if self.limbs.len() < other.limbs.len() {
            return Ordering::Less;
        }

        for i in (0..self.limbs.len()).rev() {
            if self.limbs[i] > other.limbs[i] {
                return Ordering::Greater;
            }
            if self.limbs[i] < other.limbs[i] {
                return Ordering::Less;
            }
        }
        Ordering::Equal
    }

    fn bit_len(&self) -> usize {
        let Some(&last) = self.limbs.last() else {
            return 0;
        };
        (self.limbs.len() - 1) * 32 + (32 - last.leading_zeros() as usize)
    }

    fn bit(&self, index: usize) -> bool {
        let limb_idx = index / 32;
        let bit_idx = index % 32;
        self.limbs
            .get(limb_idx)
            .map(|limb| ((limb >> bit_idx) & 1) == 1)
            .unwrap_or(false)
    }

    fn add(&self, other: &Self) -> Self {
        let max_len = self.limbs.len().max(other.limbs.len());
        let mut out = Vec::with_capacity(max_len + 1);
        let mut carry = 0u64;

        for i in 0..max_len {
            let a = self.limbs.get(i).copied().unwrap_or(0) as u64;
            let b = other.limbs.get(i).copied().unwrap_or(0) as u64;
            let sum = a + b + carry;
            out.push(sum as u32);
            carry = sum >> 32;
        }

        if carry != 0 {
            out.push(carry as u32);
        }

        Self { limbs: out }
    }

    fn sub(&self, other: &Self) -> Self {
        debug_assert!(self.cmp(other) != Ordering::Less);

        let mut out = Vec::with_capacity(self.limbs.len());
        let mut borrow = 0i64;

        for i in 0..self.limbs.len() {
            let a = self.limbs[i] as i64;
            let b = other.limbs.get(i).copied().unwrap_or(0) as i64;
            let mut diff = a - b - borrow;
            if diff < 0 {
                diff += 1i64 << 32;
                borrow = 1;
            } else {
                borrow = 0;
            }
            out.push(diff as u32);
        }

        let mut value = Self { limbs: out };
        value.normalize();
        value
    }

    fn shl1(&self) -> Self {
        if self.is_zero() {
            return Self::zero();
        }

        let mut out = Vec::with_capacity(self.limbs.len() + 1);
        let mut carry = 0u64;
        for limb in &self.limbs {
            let value = ((*limb as u64) << 1) | carry;
            out.push(value as u32);
            carry = value >> 32;
        }

        if carry != 0 {
            out.push(carry as u32);
        }

        Self { limbs: out }
    }
}

fn add_mod(a: &BigUint, b: &BigUint, modulus: &BigUint) -> BigUint {
    let mut sum = a.add(b);
    if sum.cmp(modulus) != Ordering::Less {
        sum = sum.sub(modulus);
    }
    sum
}

fn double_mod(value: &BigUint, modulus: &BigUint) -> BigUint {
    let mut doubled = value.shl1();
    if doubled.cmp(modulus) != Ordering::Less {
        doubled = doubled.sub(modulus);
    }
    doubled
}

fn mul_mod(a: &BigUint, b: &BigUint, modulus: &BigUint) -> BigUint {
    let mut result = BigUint::zero();
    let mut acc = a.clone();

    // Double-and-add keeps intermediates reduced, which avoids a full big-int
    // division implementation while still matching modular multiplication.
    for bit_index in 0..b.bit_len() {
        if b.bit(bit_index) {
            result = add_mod(&result, &acc, modulus);
        }
        acc = double_mod(&acc, modulus);
    }

    result
}

fn bytes_modulo(bytes: &[u8], modulus: &BigUint) -> BigUint {
    let mut result = BigUint::zero();
    let one = BigUint::one();

    // Streaming base reduction mirrors execution-specs behavior and prevents
    // allocating huge temporary integers for very large base inputs.
    for byte in bytes {
        for shift in (0..8).rev() {
            result = double_mod(&result, modulus);
            if ((byte >> shift) & 1) == 1 {
                result = add_mod(&result, &one, modulus);
            }
        }
    }

    result
}

fn modular_exponentiation(base: &BigUint, exponent_bytes: &[u8], modulus: &BigUint) -> BigUint {
    let mut result = BigUint::one();
    if result.cmp(modulus) != Ordering::Less {
        result = result.sub(modulus);
    }

    for byte in exponent_bytes {
        for shift in (0..8).rev() {
            result = mul_mod(&result, &result, modulus);
            if ((byte >> shift) & 1) == 1 {
                result = mul_mod(&result, base, modulus);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::secp256k1::{address_from_secret_key, sign_recoverable};

    fn execute_precompile_ok(address: &Address, input: &[u8]) -> PrecompileResult {
        execute_precompile(address, input, u64::MAX)
            .expect("precompile result")
            .expect("enough gas")
    }

    #[test]
    fn test_identity_precompile_output_and_gas() {
        let input = vec![0x01, 0x02, 0x03];
        let addr = precompile_address(4);
        let result = execute_precompile_ok(&addr, &input);
        assert_eq!(result.output, input);
        assert_eq!(result.gas_used, GAS_IDENTITY_BASE + GAS_IDENTITY_WORD);
    }

    #[test]
    fn test_ecrecover_precompile_success() {
        let message_hash = Hash::from([0x11u8; 32]);
        let secret = U256::from_u64(42);
        let (r, s, recid) = sign_recoverable(&message_hash, secret).expect("signature");
        let v = U256::from_u64(recid as u64 + 27);

        let mut input = vec![0u8; 128];
        input[..32].copy_from_slice(message_hash.as_bytes());
        input[32..64].copy_from_slice(&v.to_be_bytes());
        input[64..96].copy_from_slice(&r.to_be_bytes());
        input[96..128].copy_from_slice(&s.to_be_bytes());

        let addr = precompile_address(1);
        let result = execute_precompile_ok(&addr, &input);

        let expected = address_from_secret_key(secret).expect("address");
        let mut expected_output = vec![0u8; 32];
        expected_output[12..].copy_from_slice(expected.as_bytes());

        assert_eq!(result.output, expected_output);
        assert_eq!(result.gas_used, GAS_ECRECOVER);
    }

    #[test]
    fn test_ecrecover_precompile_invalid_v_returns_empty() {
        let input = vec![0u8; 128];
        let addr = precompile_address(1);
        let result = execute_precompile_ok(&addr, &input);
        assert!(result.output.is_empty());
        assert_eq!(result.gas_used, GAS_ECRECOVER);
    }

    #[test]
    fn test_sha256_precompile_output_and_gas() {
        let input = b"abc".to_vec();
        let addr = precompile_address(2);
        let result = execute_precompile_ok(&addr, &input);
        let expected = vec![
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d,
            0xae, 0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10,
            0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
        ];
        assert_eq!(result.output, expected);
        assert_eq!(result.gas_used, GAS_SHA256_BASE + GAS_SHA256_WORD);
    }

    #[test]
    fn test_ripemd160_precompile_output_and_gas() {
        let input = b"abc".to_vec();
        let addr = precompile_address(3);
        let result = execute_precompile_ok(&addr, &input);
        let expected = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8e,
            0xb2, 0x08, 0xf7, 0xe0, 0x5d, 0x98, 0x7a, 0x9b, 0x04, 0x4a, 0x8e, 0x98, 0xc6,
            0xb0, 0x87, 0xf1, 0x5a, 0x0b, 0xfc,
        ];
        assert_eq!(result.output, expected);
        assert_eq!(result.gas_used, GAS_RIPEMD160_BASE + GAS_RIPEMD160_WORD);
    }

    #[test]
    fn test_modexp_precompile_basic() {
        // 2^5 mod 13 = 6, return length is modulus length (1 byte).
        let input = modexp_input(&[0x02], &[0x05], &[0x0d]);
        let addr = precompile_address(5);
        let result = execute_precompile_ok(&addr, &input);

        assert_eq!(result.output, vec![0x06]);
        assert_eq!(result.gas_used, GAS_MODEXP_BASE);
    }

    #[test]
    fn test_modexp_precompile_modulus_zero_returns_zeros() {
        let input = modexp_input(&[0x02], &[0x05], &[0x00, 0x00]);
        let addr = precompile_address(5);
        let result = execute_precompile_ok(&addr, &input);

        assert_eq!(result.output, vec![0x00, 0x00]);
    }

    #[test]
    fn test_modexp_precompile_zero_lengths_returns_empty() {
        let mut input = vec![0u8; 96];
        input.extend_from_slice(&[0xaa, 0xbb, 0xcc]);

        let addr = precompile_address(5);
        let result = execute_precompile_ok(&addr, &input);

        assert!(result.output.is_empty());
        assert_eq!(result.gas_used, GAS_MODEXP_BASE);
    }

    #[test]
    fn test_modexp_precompile_oog() {
        // 256-byte base/modulus produces gas above 200 for exponent=1.
        let base = vec![0x01; 256];
        let exp = vec![0x01];
        let modulus = vec![0x03; 256];
        let input = modexp_input(&base, &exp, &modulus);

        let addr = precompile_address(5);
        let result = execute_precompile(&addr, &input, 300).expect("precompile exists");
        assert_eq!(result, Err(PrecompileError::OutOfGas));
    }

    fn modexp_input(base: &[u8], exponent: &[u8], modulus: &[u8]) -> Vec<u8> {
        let mut input = Vec::with_capacity(96 + base.len() + exponent.len() + modulus.len());
        input.extend_from_slice(&U256::from_u64(base.len() as u64).to_be_bytes());
        input.extend_from_slice(&U256::from_u64(exponent.len() as u64).to_be_bytes());
        input.extend_from_slice(&U256::from_u64(modulus.len() as u64).to_be_bytes());
        input.extend_from_slice(base);
        input.extend_from_slice(exponent);
        input.extend_from_slice(modulus);
        input
    }

    fn precompile_address(id: u8) -> Address {
        let mut bytes = [0u8; 20];
        bytes[19] = id;
        Address::from(bytes)
    }
}
