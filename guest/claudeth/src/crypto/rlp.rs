//! RLP (Recursive Length Prefix) encoding/decoding
//!
//! This module implements the Ethereum RLP specification for encoding and decoding
//! arbitrary nested arrays of binary data.
//!
//! ## RLP Encoding Rules
//!
//! 1. Single byte [0x00, 0x7f]: encoded as itself
//! 2. Byte string [0, 55] bytes: 0x80 + len, then bytes
//! 3. Byte string >55 bytes: 0xb7 + len_of_len, then len (big-endian), then bytes
//! 4. List [0, 55] bytes total: 0xc0 + len, then items
//! 5. List >55 bytes total: 0xf7 + len_of_len, then len (big-endian), then items
//!
//! See: https://ethereum.org/en/developers/docs/data-structures-and-encoding/rlp/

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::{vec, vec::Vec};

#[cfg(target_arch = "riscv32")]
use alloc::{vec, vec::Vec};

use core::fmt;

use crate::types::{Address, Hash, U256, U512};

// =============================================================================
// Error Types
// =============================================================================

/// RLP encoding/decoding errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlpError {
    /// Invalid encoding (e.g., malformed header)
    InvalidEncoding,
    /// Unexpected end of input
    UnexpectedEnd,
    /// Invalid length encoding
    InvalidLength,
    /// Input too short for expected data
    InputTooShort,
    /// Leading zero in multi-byte length encoding
    LeadingZero,
    /// Non-canonical encoding (e.g., could have used shorter form)
    NonCanonical,
}

impl fmt::Display for RlpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RlpError::InvalidEncoding => write!(f, "invalid RLP encoding"),
            RlpError::UnexpectedEnd => write!(f, "unexpected end of RLP input"),
            RlpError::InvalidLength => write!(f, "invalid length encoding"),
            RlpError::InputTooShort => write!(f, "input too short for expected data"),
            RlpError::LeadingZero => write!(f, "leading zero in multi-byte length"),
            RlpError::NonCanonical => write!(f, "non-canonical RLP encoding"),
        }
    }
}

pub type Result<T> = core::result::Result<T, RlpError>;

// =============================================================================
// Encoding Functions
// =============================================================================

/// Encodes a single byte
pub fn encode_byte(byte: u8) -> Vec<u8> {
    if byte < 0x80 {
        vec![byte]
    } else {
        vec![0x81, byte]
    }
}

/// Encodes a byte slice
pub fn encode_bytes(bytes: &[u8]) -> Vec<u8> {
    // Empty bytes special case
    if bytes.is_empty() {
        return vec![0x80];
    }

    // Single byte [0x00, 0x7f] encoded as itself
    if bytes.len() == 1 && bytes[0] < 0x80 {
        return vec![bytes[0]];
    }

    // String with length [0, 55]
    if bytes.len() <= 55 {
        let mut result = Vec::with_capacity(1 + bytes.len());
        result.push(0x80 + bytes.len() as u8);
        result.extend_from_slice(bytes);
        return result;
    }

    // String with length > 55
    let len_bytes = encode_length(bytes.len());
    let mut result = Vec::with_capacity(1 + len_bytes.len() + bytes.len());
    result.push(0xb7 + len_bytes.len() as u8);
    result.extend_from_slice(&len_bytes);
    result.extend_from_slice(bytes);
    result
}

/// Encodes a u64
pub fn encode_u64(n: u64) -> Vec<u8> {
    if n == 0 {
        vec![0x80]
    } else {
        // Remove leading zeros
        let bytes = n.to_be_bytes();
        let start = bytes.iter().position(|&b| b != 0).unwrap_or(8);
        encode_bytes(&bytes[start..])
    }
}

/// Encodes a U256
pub fn encode_u256(n: &U256) -> Vec<u8> {
    if n.is_zero() {
        vec![0x80]
    } else {
        // Convert to big-endian bytes and remove leading zeros
        let bytes = n.to_be_bytes();
        let start = bytes.iter().position(|&b| b != 0).unwrap_or(32);
        encode_bytes(&bytes[start..])
    }
}

/// Encodes a U512
pub fn encode_u512(n: &U512) -> Vec<u8> {
    if n.is_zero() {
        vec![0x80]
    } else {
        // Convert to big-endian bytes and remove leading zeros
        let bytes = n.to_be_bytes();
        let start = bytes.iter().position(|&b| b != 0).unwrap_or(64);
        encode_bytes(&bytes[start..])
    }
}

/// Encodes an Address
pub fn encode_address(addr: &Address) -> Vec<u8> {
    encode_bytes(addr.as_bytes())
}

/// Encodes a Hash
pub fn encode_hash(hash: &Hash) -> Vec<u8> {
    encode_bytes(hash.as_bytes())
}

/// Encodes a list of RLP-encoded items
pub fn encode_list(items: &[Vec<u8>]) -> Vec<u8> {
    // Calculate total payload length
    let payload_len: usize = items.iter().map(|item| item.len()).sum();

    // Empty list
    if payload_len == 0 {
        return vec![0xc0];
    }

    // Assemble payload
    let mut payload = Vec::with_capacity(payload_len);
    for item in items {
        payload.extend_from_slice(item);
    }

    // List with length [0, 55]
    if payload_len <= 55 {
        let mut result = Vec::with_capacity(1 + payload_len);
        result.push(0xc0 + payload_len as u8);
        result.extend(payload);
        return result;
    }

    // List with length > 55
    let len_bytes = encode_length(payload_len);
    let mut result = Vec::with_capacity(1 + len_bytes.len() + payload_len);
    result.push(0xf7 + len_bytes.len() as u8);
    result.extend_from_slice(&len_bytes);
    result.extend(payload);
    result
}

// Helper: encode length as big-endian bytes (no leading zeros)
fn encode_length(len: usize) -> Vec<u8> {
    if len < 256 {
        vec![len as u8]
    } else if len < 65536 {
        vec![(len >> 8) as u8, len as u8]
    } else if len < 16777216 {
        vec![(len >> 16) as u8, (len >> 8) as u8, len as u8]
    } else {
        vec![
            (len >> 24) as u8,
            (len >> 16) as u8,
            (len >> 8) as u8,
            len as u8,
        ]
    }
}

// =============================================================================
// Decoding Functions
// =============================================================================

/// Decodes a single byte, returning the byte and remaining input
pub fn decode_byte(input: &[u8]) -> Result<(u8, &[u8])> {
    if input.is_empty() {
        return Err(RlpError::UnexpectedEnd);
    }

    let first = input[0];

    if first < 0x80 {
        // Single byte [0x00, 0x7f]
        Ok((first, &input[1..]))
    } else if first == 0x81 {
        // Byte with 0x81 prefix
        if input.len() < 2 {
            return Err(RlpError::InputTooShort);
        }
        Ok((input[1], &input[2..]))
    } else {
        Err(RlpError::InvalidEncoding)
    }
}

/// Decodes a byte array, returning the bytes and remaining input
pub fn decode_bytes(input: &[u8]) -> Result<(Vec<u8>, &[u8])> {
    if input.is_empty() {
        return Err(RlpError::UnexpectedEnd);
    }

    let first = input[0];

    if first < 0x80 {
        // Single byte [0x00, 0x7f]
        Ok((vec![first], &input[1..]))
    } else if first <= 0xb7 {
        // String of length [0, 55]
        let len = (first - 0x80) as usize;

        // Check for non-canonical encoding
        if len == 1 && input.len() >= 2 && input[1] < 0x80 {
            return Err(RlpError::NonCanonical);
        }

        if input.len() < 1 + len {
            return Err(RlpError::InputTooShort);
        }

        Ok((input[1..1 + len].to_vec(), &input[1 + len..]))
    } else if first <= 0xbf {
        // String of length > 55
        let len_of_len = (first - 0xb7) as usize;

        if input.len() < 1 + len_of_len {
            return Err(RlpError::InputTooShort);
        }

        // Check for leading zero
        if len_of_len > 0 && input[1] == 0 {
            return Err(RlpError::LeadingZero);
        }

        let len = decode_length(&input[1..1 + len_of_len])?;

        // Check for non-canonical encoding
        if len <= 55 {
            return Err(RlpError::NonCanonical);
        }

        if input.len() < 1 + len_of_len + len {
            return Err(RlpError::InputTooShort);
        }

        Ok((
            input[1 + len_of_len..1 + len_of_len + len].to_vec(),
            &input[1 + len_of_len + len..],
        ))
    } else {
        // This is a list, not a byte string
        Err(RlpError::InvalidEncoding)
    }
}

/// Decodes a u64, returning the value and remaining input
pub fn decode_u64(input: &[u8]) -> Result<(u64, &[u8])> {
    let (bytes, rest) = decode_bytes(input)?;

    // Empty bytes means zero
    if bytes.is_empty() {
        return Ok((0, rest));
    }

    // Check for leading zeros (non-canonical)
    if bytes.len() > 1 && bytes[0] == 0 {
        return Err(RlpError::NonCanonical);
    }

    // u64 can be at most 8 bytes
    if bytes.len() > 8 {
        return Err(RlpError::InvalidLength);
    }

    let mut value = 0u64;
    for &byte in &bytes {
        value = (value << 8) | byte as u64;
    }

    Ok((value, rest))
}

/// Decodes a U256, returning the value and remaining input
pub fn decode_u256(input: &[u8]) -> Result<(U256, &[u8])> {
    let (bytes, rest) = decode_bytes(input)?;

    // Empty bytes means zero
    if bytes.is_empty() {
        return Ok((U256::ZERO, rest));
    }

    // Check for leading zeros (non-canonical)
    if bytes.len() > 1 && bytes[0] == 0 {
        return Err(RlpError::NonCanonical);
    }

    // U256 can be at most 32 bytes
    if bytes.len() > 32 {
        return Err(RlpError::InvalidLength);
    }

    // Pad to 32 bytes (big-endian)
    let mut padded = [0u8; 32];
    padded[32 - bytes.len()..].copy_from_slice(&bytes);

    Ok((U256::from_be_bytes(padded), rest))
}

/// Decodes a U512, returning the value and remaining input
pub fn decode_u512(input: &[u8]) -> Result<(U512, &[u8])> {
    let (bytes, rest) = decode_bytes(input)?;

    // Empty bytes means zero
    if bytes.is_empty() {
        return Ok((U512::ZERO, rest));
    }

    // Check for leading zeros (non-canonical)
    if bytes.len() > 1 && bytes[0] == 0 {
        return Err(RlpError::NonCanonical);
    }

    // U512 can be at most 64 bytes
    if bytes.len() > 64 {
        return Err(RlpError::InvalidLength);
    }

    // Pad to 64 bytes (big-endian)
    let mut padded = [0u8; 64];
    padded[64 - bytes.len()..].copy_from_slice(&bytes);

    Ok((U512::from_be_bytes(padded), rest))
}

/// Decodes an Address, returning the address and remaining input
pub fn decode_address(input: &[u8]) -> Result<(Address, &[u8])> {
    let (bytes, rest) = decode_bytes(input)?;

    if bytes.len() != 20 {
        return Err(RlpError::InvalidLength);
    }

    let mut addr_bytes = [0u8; 20];
    addr_bytes.copy_from_slice(&bytes);

    Ok((Address::from(addr_bytes), rest))
}

/// Decodes a Hash, returning the hash and remaining input
pub fn decode_hash(input: &[u8]) -> Result<(Hash, &[u8])> {
    let (bytes, rest) = decode_bytes(input)?;

    if bytes.len() != 32 {
        return Err(RlpError::InvalidLength);
    }

    let mut hash_bytes = [0u8; 32];
    hash_bytes.copy_from_slice(&bytes);

    Ok((Hash::from(hash_bytes), rest))
}

/// Decodes a list, returning the list items and remaining input
pub fn decode_list(input: &[u8]) -> Result<(Vec<Vec<u8>>, &[u8])> {
    if input.is_empty() {
        return Err(RlpError::UnexpectedEnd);
    }

    let first = input[0];

    if first < 0xc0 {
        // Not a list
        Err(RlpError::InvalidEncoding)
    } else if first <= 0xf7 {
        // List of length [0, 55]
        let len = (first - 0xc0) as usize;

        if input.len() < 1 + len {
            return Err(RlpError::InputTooShort);
        }

        let payload = &input[1..1 + len];
        let items = decode_list_payload(payload)?;

        Ok((items, &input[1 + len..]))
    } else {
        // List of length > 55
        let len_of_len = (first - 0xf7) as usize;

        if input.len() < 1 + len_of_len {
            return Err(RlpError::InputTooShort);
        }

        // Check for leading zero
        if len_of_len > 0 && input[1] == 0 {
            return Err(RlpError::LeadingZero);
        }

        let len = decode_length(&input[1..1 + len_of_len])?;

        // Check for non-canonical encoding
        if len <= 55 {
            return Err(RlpError::NonCanonical);
        }

        if input.len() < 1 + len_of_len + len {
            return Err(RlpError::InputTooShort);
        }

        let payload = &input[1 + len_of_len..1 + len_of_len + len];
        let items = decode_list_payload(payload)?;

        Ok((items, &input[1 + len_of_len + len..]))
    }
}

// Helper: decode list payload into items
fn decode_list_payload(mut payload: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut items = Vec::new();

    while !payload.is_empty() {
        // Decode one item (could be bytes or nested list)
        let item_bytes = decode_item(payload)?;
        let item_len = item_bytes.len();
        items.push(item_bytes);
        payload = &payload[item_len..];
    }

    Ok(items)
}

// Helper: decode a single RLP item (returns the full RLP encoding)
fn decode_item(input: &[u8]) -> Result<Vec<u8>> {
    if input.is_empty() {
        return Err(RlpError::UnexpectedEnd);
    }

    let first = input[0];

    if first < 0x80 {
        // Single byte
        Ok(vec![first])
    } else if first <= 0xb7 {
        // String [0, 55]
        let len = (first - 0x80) as usize;
        if input.len() < 1 + len {
            return Err(RlpError::InputTooShort);
        }
        Ok(input[..1 + len].to_vec())
    } else if first <= 0xbf {
        // String > 55
        let len_of_len = (first - 0xb7) as usize;
        if input.len() < 1 + len_of_len {
            return Err(RlpError::InputTooShort);
        }

        // Check for leading zero
        if len_of_len > 0 && input[1] == 0 {
            return Err(RlpError::LeadingZero);
        }

        let len = decode_length(&input[1..1 + len_of_len])?;
        if input.len() < 1 + len_of_len + len {
            return Err(RlpError::InputTooShort);
        }
        Ok(input[..1 + len_of_len + len].to_vec())
    } else if first <= 0xf7 {
        // List [0, 55]
        let len = (first - 0xc0) as usize;
        if input.len() < 1 + len {
            return Err(RlpError::InputTooShort);
        }
        Ok(input[..1 + len].to_vec())
    } else {
        // List > 55
        let len_of_len = (first - 0xf7) as usize;
        if input.len() < 1 + len_of_len {
            return Err(RlpError::InputTooShort);
        }

        // Check for leading zero
        if len_of_len > 0 && input[1] == 0 {
            return Err(RlpError::LeadingZero);
        }

        let len = decode_length(&input[1..1 + len_of_len])?;
        if input.len() < 1 + len_of_len + len {
            return Err(RlpError::InputTooShort);
        }
        Ok(input[..1 + len_of_len + len].to_vec())
    }
}

// Helper: decode length from bytes (big-endian)
fn decode_length(bytes: &[u8]) -> Result<usize> {
    if bytes.is_empty() {
        return Err(RlpError::InvalidLength);
    }

    let mut len = 0usize;
    for &byte in bytes {
        len = (len << 8) | byte as usize;
    }

    Ok(len)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Single Byte Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_byte_small() {
        assert_eq!(encode_byte(0x00), vec![0x00]);
        assert_eq!(encode_byte(0x7f), vec![0x7f]);
    }

    #[test]
    fn test_encode_byte_large() {
        assert_eq!(encode_byte(0x80), vec![0x81, 0x80]);
        assert_eq!(encode_byte(0xff), vec![0x81, 0xff]);
    }

    #[test]
    fn test_decode_byte_small() {
        assert_eq!(decode_byte(&[0x00]).unwrap(), (0x00, &[][..]));
        assert_eq!(decode_byte(&[0x7f]).unwrap(), (0x7f, &[][..]));
    }

    #[test]
    fn test_decode_byte_large() {
        assert_eq!(decode_byte(&[0x81, 0x80]).unwrap(), (0x80, &[][..]));
        assert_eq!(decode_byte(&[0x81, 0xff]).unwrap(), (0xff, &[][..]));
    }

    #[test]
    fn test_byte_roundtrip() {
        for byte in 0u8..=255 {
            let encoded = encode_byte(byte);
            let (decoded, rest) = decode_byte(&encoded).unwrap();
            assert_eq!(decoded, byte);
            assert!(rest.is_empty());
        }
    }

    // =========================================================================
    // Bytes Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_empty_bytes() {
        assert_eq!(encode_bytes(&[]), vec![0x80]);
    }

    #[test]
    fn test_encode_single_byte_bytes() {
        assert_eq!(encode_bytes(&[0x00]), vec![0x00]);
        assert_eq!(encode_bytes(&[0x7f]), vec![0x7f]);
        assert_eq!(encode_bytes(&[0x80]), vec![0x81, 0x80]);
    }

    #[test]
    fn test_encode_short_bytes() {
        assert_eq!(encode_bytes(&[0x01, 0x02, 0x03]), vec![0x83, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_encode_55_bytes() {
        let bytes = vec![0x42; 55];
        let mut expected = vec![0x80 + 55];
        expected.extend(&bytes);
        assert_eq!(encode_bytes(&bytes), expected);
    }

    #[test]
    fn test_encode_56_bytes() {
        let bytes = vec![0x42; 56];
        let mut expected = vec![0xb8, 56]; // 0xb7 + 1, then length
        expected.extend(&bytes);
        assert_eq!(encode_bytes(&bytes), expected);
    }

    #[test]
    fn test_encode_long_bytes() {
        let bytes = vec![0x42; 1024];
        let mut expected = vec![0xb9, 0x04, 0x00]; // 0xb7 + 2, then length (0x0400 = 1024)
        expected.extend(&bytes);
        assert_eq!(encode_bytes(&bytes), expected);
    }

    #[test]
    fn test_decode_empty_bytes() {
        assert_eq!(decode_bytes(&[0x80]).unwrap(), (vec![], &[][..]));
    }

    #[test]
    fn test_decode_single_byte_bytes() {
        assert_eq!(decode_bytes(&[0x00]).unwrap(), (vec![0x00], &[][..]));
        assert_eq!(decode_bytes(&[0x7f]).unwrap(), (vec![0x7f], &[][..]));
        assert_eq!(decode_bytes(&[0x81, 0x80]).unwrap(), (vec![0x80], &[][..]));
    }

    #[test]
    fn test_decode_short_bytes() {
        assert_eq!(
            decode_bytes(&[0x83, 0x01, 0x02, 0x03]).unwrap(),
            (vec![0x01, 0x02, 0x03], &[][..])
        );
    }

    #[test]
    fn test_decode_long_bytes() {
        let bytes = vec![0x42; 1024];
        let mut encoded = vec![0xb9, 0x04, 0x00];
        encoded.extend(&bytes);
        assert_eq!(decode_bytes(&encoded).unwrap(), (bytes, &[][..]));
    }

    #[test]
    fn test_bytes_roundtrip() {
        let test_cases = vec![
            vec![],
            vec![0x00],
            vec![0x7f],
            vec![0x80],
            vec![0x01, 0x02, 0x03],
            vec![0x42; 55],
            vec![0x42; 56],
            vec![0x42; 1024],
        ];

        for bytes in test_cases {
            let encoded = encode_bytes(&bytes);
            let (decoded, rest) = decode_bytes(&encoded).unwrap();
            assert_eq!(decoded, bytes);
            assert!(rest.is_empty());
        }
    }

    // =========================================================================
    // u64 Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_u64_zero() {
        assert_eq!(encode_u64(0), vec![0x80]);
    }

    #[test]
    fn test_encode_u64_small() {
        assert_eq!(encode_u64(1), vec![0x01]);
        assert_eq!(encode_u64(127), vec![0x7f]);
        assert_eq!(encode_u64(128), vec![0x81, 0x80]);
    }

    #[test]
    fn test_encode_u64_medium() {
        assert_eq!(encode_u64(256), vec![0x82, 0x01, 0x00]);
        assert_eq!(encode_u64(1024), vec![0x82, 0x04, 0x00]);
    }

    #[test]
    fn test_encode_u64_large() {
        assert_eq!(
            encode_u64(0x0102030405060708),
            vec![0x88, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }

    #[test]
    fn test_decode_u64_zero() {
        assert_eq!(decode_u64(&[0x80]).unwrap(), (0, &[][..]));
    }

    #[test]
    fn test_decode_u64_small() {
        assert_eq!(decode_u64(&[0x01]).unwrap(), (1, &[][..]));
        assert_eq!(decode_u64(&[0x7f]).unwrap(), (127, &[][..]));
        assert_eq!(decode_u64(&[0x81, 0x80]).unwrap(), (128, &[][..]));
    }

    #[test]
    fn test_u64_roundtrip() {
        let test_cases = vec![0, 1, 127, 128, 256, 1024, 0x0102030405060708, u64::MAX];

        for value in test_cases {
            let encoded = encode_u64(value);
            let (decoded, rest) = decode_u64(&encoded).unwrap();
            assert_eq!(decoded, value);
            assert!(rest.is_empty());
        }
    }

    // =========================================================================
    // U256 Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_u256_zero() {
        assert_eq!(encode_u256(&U256::ZERO), vec![0x80]);
    }

    #[test]
    fn test_encode_u256_one() {
        assert_eq!(encode_u256(&U256::ONE), vec![0x01]);
    }

    #[test]
    fn test_encode_u256_small() {
        assert_eq!(encode_u256(&U256::from(127u64)), vec![0x7f]);
        assert_eq!(encode_u256(&U256::from(128u64)), vec![0x81, 0x80]);
    }

    #[test]
    fn test_u256_roundtrip() {
        let test_cases = vec![
            U256::ZERO,
            U256::ONE,
            U256::from(127u64),
            U256::from(128u64),
            U256::from(256u64),
            U256::from(u64::MAX),
            U256::from(u128::MAX),
        ];

        for value in test_cases {
            let encoded = encode_u256(&value);
            let (decoded, rest) = decode_u256(&encoded).unwrap();
            assert_eq!(decoded, value);
            assert!(rest.is_empty());
        }
    }

    // =========================================================================
    // U512 Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_u512_zero() {
        assert_eq!(encode_u512(&U512::ZERO), vec![0x80]);
    }

    #[test]
    fn test_encode_u512_one() {
        assert_eq!(encode_u512(&U512::ONE), vec![0x01]);
    }

    #[test]
    fn test_u512_roundtrip() {
        let test_cases = vec![
            U512::ZERO,
            U512::ONE,
            U512::from(127u64),
            U512::from(128u64),
            U512::from(u64::MAX),
            U512::from(U256::MAX),
        ];

        for value in test_cases {
            let encoded = encode_u512(&value);
            let (decoded, rest) = decode_u512(&encoded).unwrap();
            assert_eq!(decoded, value);
            assert!(rest.is_empty());
        }
    }

    // =========================================================================
    // Address Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_address() {
        let addr = Address::from([0x42; 20]);
        let encoded = encode_address(&addr);
        assert_eq!(encoded[0], 0x80 + 20); // 0x94
        assert_eq!(encoded.len(), 21);
    }

    #[test]
    fn test_address_roundtrip() {
        let test_cases = vec![
            Address::ZERO,
            Address::from([0x42; 20]),
            Address::from([0xff; 20]),
        ];

        for addr in test_cases {
            let encoded = encode_address(&addr);
            let (decoded, rest) = decode_address(&encoded).unwrap();
            assert_eq!(decoded, addr);
            assert!(rest.is_empty());
        }
    }

    // =========================================================================
    // Hash Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_hash() {
        let hash = Hash::from([0x42; 32]);
        let encoded = encode_hash(&hash);
        assert_eq!(encoded[0], 0xa0); // 0x80 + 32
        assert_eq!(encoded.len(), 33);
    }

    #[test]
    fn test_hash_roundtrip() {
        let test_cases = vec![
            Hash::ZERO,
            Hash::from([0x42; 32]),
            Hash::from([0xff; 32]),
        ];

        for hash in test_cases {
            let encoded = encode_hash(&hash);
            let (decoded, rest) = decode_hash(&encoded).unwrap();
            assert_eq!(decoded, hash);
            assert!(rest.is_empty());
        }
    }

    // =========================================================================
    // List Encoding Tests
    // =========================================================================

    #[test]
    fn test_encode_empty_list() {
        assert_eq!(encode_list(&[]), vec![0xc0]);
    }

    #[test]
    fn test_encode_list_single_item() {
        let items = vec![vec![0x01]];
        assert_eq!(encode_list(&items), vec![0xc1, 0x01]);
    }

    #[test]
    fn test_encode_list_multiple_items() {
        let items = vec![vec![0x01], vec![0x02], vec![0x03]];
        assert_eq!(encode_list(&items), vec![0xc3, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_encode_nested_list() {
        let inner = vec![vec![0x01], vec![0x02]];
        let inner_encoded = encode_list(&inner);
        let outer = vec![inner_encoded, vec![0x03]];
        assert_eq!(encode_list(&outer), vec![0xc4, 0xc2, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_decode_empty_list() {
        assert_eq!(decode_list(&[0xc0]).unwrap(), (vec![], &[][..]));
    }

    #[test]
    fn test_decode_list_single_item() {
        assert_eq!(
            decode_list(&[0xc1, 0x01]).unwrap(),
            (vec![vec![0x01]], &[][..])
        );
    }

    #[test]
    fn test_decode_list_multiple_items() {
        assert_eq!(
            decode_list(&[0xc3, 0x01, 0x02, 0x03]).unwrap(),
            (vec![vec![0x01], vec![0x02], vec![0x03]], &[][..])
        );
    }

    #[test]
    fn test_list_roundtrip() {
        let test_cases = vec![
            vec![],
            vec![vec![0x01]],
            vec![vec![0x01], vec![0x02], vec![0x03]],
            vec![encode_bytes(&[0x01, 0x02]), encode_bytes(&[0x03, 0x04])],
        ];

        for items in test_cases {
            let encoded = encode_list(&items);
            let (decoded, rest) = decode_list(&encoded).unwrap();
            assert_eq!(decoded, items);
            assert!(rest.is_empty());
        }
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[test]
    fn test_decode_empty_input() {
        assert_eq!(decode_byte(&[]), Err(RlpError::UnexpectedEnd));
        assert_eq!(decode_bytes(&[]), Err(RlpError::UnexpectedEnd));
        assert_eq!(decode_list(&[]), Err(RlpError::UnexpectedEnd));
    }

    #[test]
    fn test_decode_input_too_short() {
        assert_eq!(decode_bytes(&[0x83, 0x01]), Err(RlpError::InputTooShort));
        assert_eq!(decode_list(&[0xc3, 0x01]), Err(RlpError::InputTooShort));
    }

    #[test]
    fn test_decode_non_canonical_single_byte() {
        // Single byte [0x00, 0x7f] should not be encoded with 0x81 prefix
        assert_eq!(decode_bytes(&[0x81, 0x7f]), Err(RlpError::NonCanonical));
    }

    #[test]
    fn test_decode_leading_zero() {
        // Multi-byte length should not have leading zeros
        let mut data = vec![0xb9, 0x00, 0x38];
        data.extend(vec![0x42; 56]);
        assert_eq!(decode_bytes(&data), Err(RlpError::LeadingZero));
    }

    #[test]
    fn test_decode_non_canonical_length() {
        // Length <= 55 should not use long form
        let mut data = vec![0xb8, 55]; // Wrong: should use short form
        data.extend(vec![0x42; 55]);
        assert_eq!(decode_bytes(&data), Err(RlpError::NonCanonical));
    }

    #[test]
    fn test_decode_invalid_address_length() {
        let encoded = encode_bytes(&[0x42; 19]); // Wrong length
        assert_eq!(decode_address(&encoded), Err(RlpError::InvalidLength));
    }

    #[test]
    fn test_decode_invalid_hash_length() {
        let encoded = encode_bytes(&[0x42; 31]); // Wrong length
        assert_eq!(decode_hash(&encoded), Err(RlpError::InvalidLength));
    }

    // =========================================================================
    // Ethereum Test Vectors
    // =========================================================================

    #[test]
    fn test_ethereum_dog() {
        // "dog" = [0x64, 0x6f, 0x67]
        let encoded = encode_bytes(&[0x64, 0x6f, 0x67]);
        assert_eq!(encoded, vec![0x83, 0x64, 0x6f, 0x67]);
    }

    #[test]
    fn test_ethereum_cat_dog_list() {
        // [ "cat", "dog" ]
        let cat = encode_bytes(&[0x63, 0x61, 0x74]);
        let dog = encode_bytes(&[0x64, 0x6f, 0x67]);
        let encoded = encode_list(&[cat, dog]);
        assert_eq!(encoded, vec![0xc8, 0x83, 0x63, 0x61, 0x74, 0x83, 0x64, 0x6f, 0x67]);
    }

    #[test]
    fn test_ethereum_empty_string() {
        assert_eq!(encode_bytes(&[]), vec![0x80]);
    }

    #[test]
    fn test_ethereum_empty_list() {
        assert_eq!(encode_list(&[]), vec![0xc0]);
    }

    #[test]
    fn test_ethereum_zero() {
        assert_eq!(encode_u64(0), vec![0x80]);
    }

    #[test]
    fn test_ethereum_small_integer() {
        assert_eq!(encode_u64(15), vec![0x0f]);
    }

    #[test]
    fn test_ethereum_medium_integer() {
        assert_eq!(encode_u64(1024), vec![0x82, 0x04, 0x00]);
    }

    #[test]
    #[allow(clippy::cloned_ref_to_slice_refs)]
    fn test_ethereum_nested_empty_lists() {
        // [ [], [[]], [ [], [[]] ] ]
        let empty = encode_list(&[]);
        let inner1 = encode_list(&[empty.clone()]);
        let inner2 = encode_list(&[empty.clone(), inner1.clone()]);
        let outer = encode_list(&[empty.clone(), inner1, inner2]);
        assert_eq!(
            outer,
            vec![0xc7, 0xc0, 0xc1, 0xc0, 0xc3, 0xc0, 0xc1, 0xc0]
        );
    }

    // =========================================================================
    // Additional Coverage Tests
    // =========================================================================

    #[test]
    fn test_encode_length_boundaries() {
        assert_eq!(encode_length(255), vec![0xff]);
        assert_eq!(encode_length(256), vec![0x01, 0x00]);
        assert_eq!(encode_length(65535), vec![0xff, 0xff]);
        assert_eq!(encode_length(65536), vec![0x01, 0x00, 0x00]);
    }

    #[test]
    fn test_remaining_input() {
        let input = [0x01, 0x02, 0x03];
        let (decoded, rest) = decode_byte(&input).unwrap();
        assert_eq!(decoded, 0x01);
        assert_eq!(rest, &[0x02, 0x03]);
    }

    #[test]
    fn test_decode_bytes_with_remaining() {
        let input = [0x83, 0x01, 0x02, 0x03, 0x04, 0x05];
        let (decoded, rest) = decode_bytes(&input).unwrap();
        assert_eq!(decoded, vec![0x01, 0x02, 0x03]);
        assert_eq!(rest, &[0x04, 0x05]);
    }

    #[test]
    fn test_long_list() {
        // Create a list with 60 bytes total payload (> 55)
        let items: Vec<Vec<u8>> = (0..60).map(|_| vec![0x01]).collect();
        let encoded = encode_list(&items);

        // Should use long form: 0xf7 + len_of_len + len + payload
        assert_eq!(encoded[0], 0xf8); // 0xf7 + 1
        assert_eq!(encoded[1], 60); // length

        let (decoded, rest) = decode_list(&encoded).unwrap();
        assert_eq!(decoded, items);
        assert!(rest.is_empty());
    }

    #[test]
    fn test_u256_max() {
        let value = U256::MAX;
        let encoded = encode_u256(&value);
        let (decoded, rest) = decode_u256(&encoded).unwrap();
        assert_eq!(decoded, value);
        assert!(rest.is_empty());
    }

    #[test]
    fn test_decode_u64_too_large() {
        // 9 bytes is too large for u64
        let encoded = vec![0x89, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
        assert_eq!(decode_u64(&encoded), Err(RlpError::InvalidLength));
    }

    #[test]
    fn test_decode_u256_too_large() {
        // 33 bytes is too large for U256
        // Use length 257 (0x0101) to avoid leading zeros
        let mut encoded = vec![0xb9, 0x01, 0x01]; // Length 257 in 2 bytes
        encoded.extend(vec![0x42; 257]);
        assert_eq!(decode_u256(&encoded), Err(RlpError::InvalidLength));
    }

    #[test]
    fn test_decode_u512_too_large() {
        // 65 bytes is too large for U512
        // Use length 257 (0x0101) to avoid leading zeros
        let mut encoded = vec![0xb9, 0x01, 0x01]; // Length 257 in 2 bytes
        encoded.extend(vec![0x42; 257]);
        assert_eq!(decode_u512(&encoded), Err(RlpError::InvalidLength));
    }

    #[test]
    fn test_decode_u64_leading_zero() {
        // Leading zeros are non-canonical
        let encoded = vec![0x82, 0x00, 0x01];
        assert_eq!(decode_u64(&encoded), Err(RlpError::NonCanonical));
    }

    #[test]
    fn test_nested_list_with_bytes() {
        let inner_items = vec![
            encode_bytes(&[0x01, 0x02]),
            encode_u64(42),
        ];
        let inner_list = encode_list(&inner_items);

        let outer_items = vec![
            encode_bytes(&[0xff]),
            inner_list,
            encode_u64(0),
        ];
        let outer_list = encode_list(&outer_items);

        let (decoded, rest) = decode_list(&outer_list).unwrap();
        assert!(rest.is_empty());
        assert_eq!(decoded.len(), 3);
    }
}
