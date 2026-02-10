//! EVM precompile implementations.
//!
//! Currently implements:
//! - 0x01: ECRECOVER
//! - 0x02: SHA256
//! - 0x03: RIPEMD160
//! - 0x04: IDENTITY
//! - 0x05: MODEXP (Cancun gas formula / EIP-2565)
//! - 0x06: ALT_BN128 ECADD
//! - 0x07: ALT_BN128 ECMUL
//! - 0x09: BLAKE2F (EIP-152)

#[cfg(target_arch = "riscv32")]
extern crate alloc;

use core::cmp::Ordering;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::crypto::secp256k1::recover_address;
use crate::crypto::secp256k1_math::secp256k1_n;
use crate::crypto::secp256k1_math::{mod_add, mod_inv, mod_mul, mod_sub};
use crate::crypto::{ripemd160, sha256};
use crate::evm::gas::{
    GAS_BLAKE2F_ROUND, GAS_BN256_ADD, GAS_BN256_MUL, GAS_ECRECOVER, GAS_IDENTITY_BASE,
    GAS_IDENTITY_WORD, GAS_MODEXP_BASE,
};
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
        1 => Some(meter_precompile_result(
            ecrecover_precompile(input),
            available_gas,
        )),
        2 => Some(meter_precompile_result(
            sha256_precompile(input),
            available_gas,
        )),
        3 => Some(meter_precompile_result(
            ripemd160_precompile(input),
            available_gas,
        )),
        4 => Some(meter_precompile_result(
            identity_precompile(input),
            available_gas,
        )),
        5 => Some(modexp_precompile(input, available_gas)),
        6 => Some(ecadd_precompile(input, available_gas)),
        7 => Some(ecmul_precompile(input, available_gas)),
        9 => Some(blake2f_precompile(input, available_gas)),
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

fn modexp_precompile(
    input: &[u8],
    available_gas: u64,
) -> Result<PrecompileResult, PrecompileError> {
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

const BN254_FIELD_MODULUS_BYTES: [u8; 32] = [
    0x30, 0x64, 0x4e, 0x72, 0xe1, 0x31, 0xa0, 0x29, 0xb8, 0x50, 0x45, 0xb6, 0x81, 0x81, 0x58, 0x5d,
    0x97, 0x81, 0x6a, 0x91, 0x68, 0x71, 0xca, 0x8d, 0x3c, 0x20, 0x8c, 0x16, 0xd8, 0x7c, 0xfd, 0x47,
];

fn bn254_field_modulus() -> U256 {
    U256::from_be_bytes(BN254_FIELD_MODULUS_BYTES)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bn254Point {
    Infinity,
    Point { x: U256, y: U256 },
}

fn ecadd_precompile(input: &[u8], available_gas: u64) -> Result<PrecompileResult, PrecompileError> {
    if available_gas < GAS_BN256_ADD {
        return Err(PrecompileError::OutOfGas);
    }

    // EELS reads each point with zero-padding, so short calldata still decodes as two 64-byte points.
    let point_0 = decode_bn254_g1(read_padded_bytes(input, 0, 64).as_slice())?;
    let point_1 = decode_bn254_g1(read_padded_bytes(input, 64, 64).as_slice())?;
    let result = bn254_point_add(point_0, point_1);

    let output = match result {
        Bn254Point::Infinity => vec![0u8; 64],
        Bn254Point::Point { x, y } => {
            let mut out = vec![0u8; 64];
            out[..32].copy_from_slice(&x.to_be_bytes());
            out[32..].copy_from_slice(&y.to_be_bytes());
            out
        }
    };

    Ok(PrecompileResult {
        output,
        gas_used: GAS_BN256_ADD,
    })
}

fn ecmul_precompile(input: &[u8], available_gas: u64) -> Result<PrecompileResult, PrecompileError> {
    if available_gas < GAS_BN256_MUL {
        return Err(PrecompileError::OutOfGas);
    }

    // EELS uses buffer_read semantics: short calldata is right-padded with zeros.
    let point = decode_bn254_g1(read_padded_bytes(input, 0, 64).as_slice())?;
    let scalar = U256::from_be_bytes(read_padded_word(input, 64));
    let result = bn254_point_mul(point, scalar);

    let output = match result {
        Bn254Point::Infinity => vec![0u8; 64],
        Bn254Point::Point { x, y } => {
            let mut out = vec![0u8; 64];
            out[..32].copy_from_slice(&x.to_be_bytes());
            out[32..].copy_from_slice(&y.to_be_bytes());
            out
        }
    };

    Ok(PrecompileResult {
        output,
        gas_used: GAS_BN256_MUL,
    })
}

const BLAKE2B_IV: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

const BLAKE2B_SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

fn blake2f_precompile(input: &[u8], available_gas: u64) -> Result<PrecompileResult, PrecompileError> {
    if input.len() != 213 {
        return Err(PrecompileError::OutOfGas);
    }

    let rounds = u32::from_be_bytes([input[0], input[1], input[2], input[3]]);
    let gas_used = u64::from(rounds).saturating_mul(GAS_BLAKE2F_ROUND);
    if gas_used > available_gas {
        return Err(PrecompileError::OutOfGas);
    }

    let final_block_flag = input[212];
    if final_block_flag > 1 {
        return Err(PrecompileError::OutOfGas);
    }

    let mut h = [0u64; 8];
    for (idx, word) in h.iter_mut().enumerate() {
        *word = read_u64_le(input, 4 + idx * 8);
    }

    let mut m = [0u64; 16];
    for (idx, word) in m.iter_mut().enumerate() {
        *word = read_u64_le(input, 68 + idx * 8);
    }

    let t0 = read_u64_le(input, 196);
    let t1 = read_u64_le(input, 204);
    let output_words = blake2b_compress(rounds, h, m, t0, t1, final_block_flag == 1);

    let mut output = vec![0u8; 64];
    for (idx, word) in output_words.iter().enumerate() {
        output[idx * 8..(idx + 1) * 8].copy_from_slice(&word.to_le_bytes());
    }

    Ok(PrecompileResult { output, gas_used })
}

fn read_u64_le(input: &[u8], offset: usize) -> u64 {
    let mut word = [0u8; 8];
    word.copy_from_slice(&input[offset..offset + 8]);
    u64::from_le_bytes(word)
}

fn blake2b_mix(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    // Rotation constants and operation order follow RFC-7693 section 3.2 exactly.
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(63);
}

fn blake2b_compress(
    rounds: u32,
    h: [u64; 8],
    m: [u64; 16],
    t0: u64,
    t1: u64,
    final_block: bool,
) -> [u64; 8] {
    let mut v = [0u64; 16];
    v[..8].copy_from_slice(&h);
    v[8..].copy_from_slice(&BLAKE2B_IV);
    v[12] ^= t0;
    v[13] ^= t1;

    if final_block {
        v[14] = !v[14];
    }

    for round in 0..rounds as usize {
        let schedule = &BLAKE2B_SIGMA[round % BLAKE2B_SIGMA.len()];
        blake2b_mix(
            &mut v,
            0,
            4,
            8,
            12,
            m[schedule[0]],
            m[schedule[1]],
        );
        blake2b_mix(
            &mut v,
            1,
            5,
            9,
            13,
            m[schedule[2]],
            m[schedule[3]],
        );
        blake2b_mix(
            &mut v,
            2,
            6,
            10,
            14,
            m[schedule[4]],
            m[schedule[5]],
        );
        blake2b_mix(
            &mut v,
            3,
            7,
            11,
            15,
            m[schedule[6]],
            m[schedule[7]],
        );
        blake2b_mix(
            &mut v,
            0,
            5,
            10,
            15,
            m[schedule[8]],
            m[schedule[9]],
        );
        blake2b_mix(
            &mut v,
            1,
            6,
            11,
            12,
            m[schedule[10]],
            m[schedule[11]],
        );
        blake2b_mix(
            &mut v,
            2,
            7,
            8,
            13,
            m[schedule[12]],
            m[schedule[13]],
        );
        blake2b_mix(
            &mut v,
            3,
            4,
            9,
            14,
            m[schedule[14]],
            m[schedule[15]],
        );
    }

    let mut output = [0u64; 8];
    for (idx, word) in output.iter_mut().enumerate() {
        *word = h[idx] ^ v[idx] ^ v[idx + 8];
    }
    output
}

fn decode_bn254_g1(bytes: &[u8]) -> Result<Bn254Point, PrecompileError> {
    if bytes.len() != 64 {
        return Err(PrecompileError::OutOfGas);
    }

    let mut x_bytes = [0u8; 32];
    let mut y_bytes = [0u8; 32];
    x_bytes.copy_from_slice(&bytes[..32]);
    y_bytes.copy_from_slice(&bytes[32..64]);

    let x = U256::from_be_bytes(x_bytes);
    let y = U256::from_be_bytes(y_bytes);
    let p = bn254_field_modulus();

    if x >= p || y >= p {
        return Err(PrecompileError::OutOfGas);
    }

    if x.is_zero() && y.is_zero() {
        return Ok(Bn254Point::Infinity);
    }

    let y_sq = mod_mul(y, y, p);
    let x_sq = mod_mul(x, x, p);
    let x_cubed = mod_mul(x_sq, x, p);
    let rhs = mod_add(x_cubed, U256::from_u64(3), p);

    if y_sq != rhs {
        // execution-specs treats malformed bn254 inputs as a precompile exceptional halt.
        // We map this to OutOfGas to preserve EVM sub-call failure semantics.
        return Err(PrecompileError::OutOfGas);
    }

    Ok(Bn254Point::Point { x, y })
}

fn bn254_point_double(point: Bn254Point) -> Bn254Point {
    match point {
        Bn254Point::Infinity => Bn254Point::Infinity,
        Bn254Point::Point { x, y } => {
            if y.is_zero() {
                return Bn254Point::Infinity;
            }

            let p = bn254_field_modulus();
            let numerator = mod_mul(U256::from_u64(3), mod_mul(x, x, p), p);
            let denominator = mod_mul(U256::from_u64(2), y, p);
            let Some(inv) = mod_inv(denominator, p) else {
                return Bn254Point::Infinity;
            };

            let lambda = mod_mul(numerator, inv, p);
            let lambda_sq = mod_mul(lambda, lambda, p);
            let x_3 = mod_sub(mod_sub(lambda_sq, x, p), x, p);
            let y_3 = mod_sub(mod_mul(lambda, mod_sub(x, x_3, p), p), y, p);

            Bn254Point::Point { x: x_3, y: y_3 }
        }
    }
}

fn bn254_point_add(left: Bn254Point, right: Bn254Point) -> Bn254Point {
    match (left, right) {
        (Bn254Point::Infinity, point) | (point, Bn254Point::Infinity) => point,
        (Bn254Point::Point { x: x_0, y: y_0 }, Bn254Point::Point { x: x_1, y: y_1 }) => {
            let p = bn254_field_modulus();

            if x_0 == x_1 {
                if mod_add(y_0, y_1, p).is_zero() {
                    return Bn254Point::Infinity;
                }
                return bn254_point_double(Bn254Point::Point { x: x_0, y: y_0 });
            }

            let numerator = mod_sub(y_1, y_0, p);
            let denominator = mod_sub(x_1, x_0, p);
            let Some(inv) = mod_inv(denominator, p) else {
                return Bn254Point::Infinity;
            };
            let lambda = mod_mul(numerator, inv, p);

            let lambda_sq = mod_mul(lambda, lambda, p);
            let x_2 = mod_sub(mod_sub(lambda_sq, x_0, p), x_1, p);
            let y_2 = mod_sub(mod_mul(lambda, mod_sub(x_0, x_2, p), p), y_0, p);

            Bn254Point::Point { x: x_2, y: y_2 }
        }
    }
}

fn bn254_point_mul(point: Bn254Point, scalar: U256) -> Bn254Point {
    let mut result = Bn254Point::Infinity;
    let mut addend = point;
    let mut n = scalar;

    // LSB-first double-and-add matches scalar multiplication semantics while
    // keeping arithmetic in our existing affine helpers.
    while !n.is_zero() {
        if (n & U256::ONE) == U256::ONE {
            result = bn254_point_add(result, addend);
        }
        addend = bn254_point_double(addend);
        n >>= 1;
    }

    result
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
    use hex::decode as hex_decode;

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
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
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
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x8e, 0xb2,
            0x08, 0xf7, 0xe0, 0x5d, 0x98, 0x7a, 0x9b, 0x04, 0x4a, 0x8e, 0x98, 0xc6, 0xb0, 0x87,
            0xf1, 0x5a, 0x0b, 0xfc,
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

    fn bn254_point(x: U256, y: U256) -> Vec<u8> {
        let mut out = vec![0u8; 64];
        out[..32].copy_from_slice(&x.to_be_bytes());
        out[32..].copy_from_slice(&y.to_be_bytes());
        out
    }

    #[test]
    fn test_ecadd_precompile_adds_two_points() {
        let addr = precompile_address(6);
        let one_two = bn254_point(U256::from_u64(1), U256::from_u64(2));
        let mut input = Vec::new();
        input.extend_from_slice(&one_two);
        input.extend_from_slice(&one_two);

        let result = execute_precompile_ok(&addr, &input);
        assert_eq!(result.gas_used, GAS_BN256_ADD);
        assert_eq!(
            result.output,
            bn254_point(
                U256::from_be_bytes([
                    0x03, 0x06, 0x44, 0xe7, 0x2e, 0x13, 0x1a, 0x02, 0x9b, 0x85, 0x04, 0x5b, 0x68,
                    0x18, 0x15, 0x85, 0xd9, 0x78, 0x16, 0xa9, 0x16, 0x87, 0x1c, 0xa8, 0xd3, 0xc2,
                    0x08, 0xc1, 0x6d, 0x87, 0xcf, 0xd3,
                ]),
                U256::from_be_bytes([
                    0x15, 0xed, 0x73, 0x8c, 0x0e, 0x0a, 0x7c, 0x92, 0xe7, 0x84, 0x5f, 0x96, 0xb2,
                    0xae, 0x9c, 0x0a, 0x68, 0xa6, 0xa4, 0x49, 0xe3, 0x53, 0x8f, 0xc7, 0xff, 0x3e,
                    0xbf, 0x7a, 0x5a, 0x18, 0xa2, 0xc4,
                ]),
            )
        );
    }

    #[test]
    fn test_ecadd_precompile_matches_eip196_vector_p1_plus_q1() {
        let addr = precompile_address(6);
        let p1 = bn254_point(
            U256::from_be_bytes([
                0x17, 0xc1, 0x39, 0xdf, 0x0e, 0xfe, 0xe0, 0xf7, 0x66, 0xbc, 0x02, 0x04, 0x76, 0x2b,
                0x77, 0x43, 0x62, 0xe4, 0xde, 0xd8, 0x89, 0x53, 0xa3, 0x9c, 0xe8, 0x49, 0xa8, 0xa7,
                0xfa, 0x16, 0x3f, 0xa9,
            ]),
            U256::from_be_bytes([
                0x01, 0xe0, 0x55, 0x9b, 0xac, 0xb1, 0x60, 0x66, 0x47, 0x64, 0xa3, 0x57, 0xaf, 0x8a,
                0x9f, 0xe7, 0x0b, 0xaa, 0x92, 0x58, 0xe0, 0xb9, 0x59, 0x27, 0x3f, 0xfc, 0x57, 0x18,
                0xc6, 0xd4, 0xcc, 0x7c,
            ]),
        );
        let q1 = bn254_point(
            U256::from_be_bytes([
                0x03, 0x97, 0x30, 0xea, 0x8d, 0xff, 0x12, 0x54, 0xc0, 0xfe, 0xe9, 0xc0, 0xea, 0x77,
                0x7d, 0x29, 0xa9, 0xc7, 0x10, 0xb7, 0xe6, 0x16, 0x68, 0x3f, 0x19, 0x4f, 0x18, 0xc4,
                0x3b, 0x43, 0xb8, 0x69,
            ]),
            U256::from_be_bytes([
                0x07, 0x3a, 0x5f, 0xfc, 0xc6, 0xfc, 0x7a, 0x28, 0xc3, 0x07, 0x23, 0xd6, 0xe5, 0x8c,
                0xe5, 0x77, 0x35, 0x69, 0x82, 0xd6, 0x5b, 0x83, 0x3a, 0x5a, 0x5c, 0x15, 0xbf, 0x90,
                0x24, 0xb4, 0x3d, 0x98,
            ]),
        );

        let mut input = Vec::new();
        input.extend_from_slice(&p1);
        input.extend_from_slice(&q1);

        let result = execute_precompile_ok(&addr, &input);
        assert_eq!(result.gas_used, GAS_BN256_ADD);
        assert_eq!(
            result.output,
            bn254_point(
                U256::from_be_bytes([
                    0x15, 0xbf, 0x2b, 0xb1, 0x78, 0x80, 0x14, 0x4b, 0x5d, 0x1c, 0xd2, 0xb1, 0xf4,
                    0x6e, 0xff, 0x9d, 0x61, 0x7b, 0xff, 0xd1, 0xca, 0x57, 0xc3, 0x7f, 0xb5, 0xa4,
                    0x9b, 0xd8, 0x4e, 0x53, 0xcf, 0x66,
                ]),
                U256::from_be_bytes([
                    0x04, 0x9c, 0x79, 0x7f, 0x9c, 0xe0, 0xd1, 0x70, 0x83, 0xde, 0xb3, 0x2b, 0x5e,
                    0x36, 0xf2, 0xea, 0x2a, 0x21, 0x2e, 0xe0, 0x36, 0x59, 0x8d, 0xd7, 0x62, 0x4c,
                    0x16, 0x89, 0x93, 0xd1, 0x35, 0x5f,
                ]),
            )
        );
    }

    #[test]
    fn test_ecadd_precompile_zero_pads_short_input() {
        let addr = precompile_address(6);
        // Only one point is provided, so the second point is interpreted as infinity.
        let input = bn254_point(U256::from_u64(1), U256::from_u64(2));

        let result = execute_precompile_ok(&addr, &input);
        assert_eq!(result.gas_used, GAS_BN256_ADD);
        assert_eq!(
            result.output,
            bn254_point(U256::from_u64(1), U256::from_u64(2))
        );
    }

    #[test]
    fn test_ecadd_precompile_invalid_field_element_fails() {
        let addr = precompile_address(6);
        let mut invalid_point = vec![0u8; 64];
        invalid_point[..32].copy_from_slice(&bn254_field_modulus().to_be_bytes());

        let mut input = Vec::new();
        input.extend_from_slice(&invalid_point);
        input.extend_from_slice(&bn254_point(U256::ZERO, U256::ZERO));

        let result = execute_precompile(&addr, &input, GAS_BN256_ADD).expect("precompile exists");
        assert_eq!(result, Err(PrecompileError::OutOfGas));
    }

    #[test]
    fn test_ecadd_precompile_invalid_curve_point_fails() {
        let addr = precompile_address(6);
        let invalid_point = bn254_point(U256::from_u64(1), U256::from_u64(3));

        let mut input = Vec::new();
        input.extend_from_slice(&invalid_point);
        input.extend_from_slice(&bn254_point(U256::ZERO, U256::ZERO));

        let result = execute_precompile(&addr, &input, GAS_BN256_ADD).expect("precompile exists");
        assert_eq!(result, Err(PrecompileError::OutOfGas));
    }

    #[test]
    fn test_ecadd_precompile_oog() {
        let addr = precompile_address(6);
        let mut input = Vec::new();
        input.extend_from_slice(&bn254_point(U256::from_u64(1), U256::from_u64(2)));
        input.extend_from_slice(&bn254_point(U256::ZERO, U256::ZERO));

        let result =
            execute_precompile(&addr, &input, GAS_BN256_ADD - 1).expect("precompile exists");
        assert_eq!(result, Err(PrecompileError::OutOfGas));
    }

    #[test]
    fn test_ecmul_precompile_scalar_zero_returns_infinity() {
        let addr = precompile_address(7);
        let mut input = Vec::new();
        input.extend_from_slice(&bn254_point(U256::from_u64(1), U256::from_u64(2)));
        input.extend_from_slice(&U256::ZERO.to_be_bytes());

        let result = execute_precompile_ok(&addr, &input);
        assert_eq!(result.gas_used, GAS_BN256_MUL);
        assert_eq!(result.output, vec![0u8; 64]);
    }

    #[test]
    fn test_ecmul_precompile_scalar_one_returns_same_point() {
        let addr = precompile_address(7);
        let point = bn254_point(U256::from_u64(1), U256::from_u64(2));
        let mut input = Vec::new();
        input.extend_from_slice(&point);
        input.extend_from_slice(&U256::ONE.to_be_bytes());

        let result = execute_precompile_ok(&addr, &input);
        assert_eq!(result.gas_used, GAS_BN256_MUL);
        assert_eq!(result.output, point);
    }

    #[test]
    fn test_ecmul_precompile_scalar_two_matches_eip196_vector_g1x2() {
        let addr = precompile_address(7);
        let mut input = Vec::new();
        input.extend_from_slice(&bn254_point(U256::from_u64(1), U256::from_u64(2)));
        input.extend_from_slice(&U256::from_u64(2).to_be_bytes());

        let result = execute_precompile_ok(&addr, &input);
        assert_eq!(result.gas_used, GAS_BN256_MUL);
        assert_eq!(
            result.output,
            bn254_point(
                U256::from_be_bytes([
                    0x03, 0x06, 0x44, 0xe7, 0x2e, 0x13, 0x1a, 0x02, 0x9b, 0x85, 0x04, 0x5b, 0x68,
                    0x18, 0x15, 0x85, 0xd9, 0x78, 0x16, 0xa9, 0x16, 0x87, 0x1c, 0xa8, 0xd3, 0xc2,
                    0x08, 0xc1, 0x6d, 0x87, 0xcf, 0xd3,
                ]),
                U256::from_be_bytes([
                    0x15, 0xed, 0x73, 0x8c, 0x0e, 0x0a, 0x7c, 0x92, 0xe7, 0x84, 0x5f, 0x96, 0xb2,
                    0xae, 0x9c, 0x0a, 0x68, 0xa6, 0xa4, 0x49, 0xe3, 0x53, 0x8f, 0xc7, 0xff, 0x3e,
                    0xbf, 0x7a, 0x5a, 0x18, 0xa2, 0xc4,
                ]),
            )
        );
    }

    #[test]
    fn test_ecmul_precompile_zero_pads_missing_scalar_as_zero() {
        let addr = precompile_address(7);
        // Only the point is provided, so the scalar decodes as zero.
        let input = bn254_point(U256::from_u64(1), U256::from_u64(2));

        let result = execute_precompile_ok(&addr, &input);
        assert_eq!(result.gas_used, GAS_BN256_MUL);
        assert_eq!(result.output, vec![0u8; 64]);
    }

    #[test]
    fn test_ecmul_precompile_invalid_field_element_fails() {
        let addr = precompile_address(7);
        let mut invalid_point = vec![0u8; 64];
        invalid_point[..32].copy_from_slice(&bn254_field_modulus().to_be_bytes());

        let mut input = Vec::new();
        input.extend_from_slice(&invalid_point);
        input.extend_from_slice(&U256::ONE.to_be_bytes());

        let result = execute_precompile(&addr, &input, GAS_BN256_MUL).expect("precompile exists");
        assert_eq!(result, Err(PrecompileError::OutOfGas));
    }

    #[test]
    fn test_ecmul_precompile_invalid_curve_point_fails() {
        let addr = precompile_address(7);
        let mut input = Vec::new();
        input.extend_from_slice(&bn254_point(U256::from_u64(1), U256::from_u64(3)));
        input.extend_from_slice(&U256::from_u64(2).to_be_bytes());

        let result = execute_precompile(&addr, &input, GAS_BN256_MUL).expect("precompile exists");
        assert_eq!(result, Err(PrecompileError::OutOfGas));
    }

    #[test]
    fn test_ecmul_precompile_oog() {
        let addr = precompile_address(7);
        let mut input = Vec::new();
        input.extend_from_slice(&bn254_point(U256::from_u64(1), U256::from_u64(2)));
        input.extend_from_slice(&U256::ONE.to_be_bytes());

        let result =
            execute_precompile(&addr, &input, GAS_BN256_MUL - 1).expect("precompile exists");
        assert_eq!(result, Err(PrecompileError::OutOfGas));
    }

    #[test]
    fn test_blake2f_precompile_vector_rounds_zero() {
        let addr = precompile_address(9);
        let input = blake2_reference_input(0, 1);
        let result = execute_precompile_ok(&addr, &input);

        let expected = hex_decode(
            "08c9bcf367e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5\
             d282e6ad7f520e511f6c3e2b8c68059b9442be0454267ce079217e1319cde05b",
        )
        .expect("valid hex");
        assert_eq!(result.output, expected);
        assert_eq!(result.gas_used, 0);
    }

    #[test]
    fn test_blake2f_precompile_vector_rounds_twelve() {
        let addr = precompile_address(9);
        let input = blake2_reference_input(12, 1);
        let result = execute_precompile_ok(&addr, &input);

        let expected = hex_decode(
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1\
             7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923",
        )
        .expect("valid hex");
        assert_eq!(result.output, expected);
        assert_eq!(result.gas_used, 12 * GAS_BLAKE2F_ROUND);
    }

    #[test]
    fn test_blake2f_precompile_invalid_length_fails() {
        let addr = precompile_address(9);
        let input = vec![0u8; 212];
        let result = execute_precompile(&addr, &input, u64::MAX).expect("precompile exists");
        assert_eq!(result, Err(PrecompileError::OutOfGas));
    }

    #[test]
    fn test_blake2f_precompile_invalid_final_flag_fails() {
        let addr = precompile_address(9);
        let input = blake2_reference_input(12, 2);
        let result = execute_precompile(&addr, &input, u64::MAX).expect("precompile exists");
        assert_eq!(result, Err(PrecompileError::OutOfGas));
    }

    #[test]
    fn test_blake2f_precompile_oog() {
        let addr = precompile_address(9);
        let input = blake2_reference_input(u32::MAX, 1);
        let result = execute_precompile(&addr, &input, GAS_BLAKE2F_ROUND - 1)
            .expect("precompile exists");
        assert_eq!(result, Err(PrecompileError::OutOfGas));
    }

    fn blake2_reference_input(rounds: u32, final_block_flag: u8) -> Vec<u8> {
        // These vectors come from execution-spec EIP-152 tests and are used to
        // validate byte-level compatibility with canonical fixtures.
        let h = hex_decode(
            "48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5\
             d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b",
        )
        .expect("valid state vector");
        let m = {
            let mut message = vec![0u8; 128];
            message[..3].copy_from_slice(b"abc");
            message
        };

        let mut input = Vec::with_capacity(213);
        input.extend_from_slice(&rounds.to_be_bytes());
        input.extend_from_slice(&h);
        input.extend_from_slice(&m);
        input.extend_from_slice(&3u64.to_le_bytes());
        input.extend_from_slice(&0u64.to_le_bytes());
        input.push(final_block_flag);
        input
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
