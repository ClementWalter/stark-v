//! EVM Control Flow, Memory, Storage, and Stack Opcodes
//!
//! This module implements memory, storage, stack manipulation, and control flow opcodes
//! following the Ethereum Yellow Paper specification.
//!
//! ## Opcode Categories
//!
//! - **Memory**: MLOAD, MSTORE, MSTORE8, MSIZE
//! - **Storage**: SLOAD, SSTORE
//! - **Stack**: POP, PUSH1-PUSH32, DUP1-DUP16, SWAP1-SWAP16
//! - **Control Flow**: JUMP, JUMPI, PC, JUMPDEST, GAS

#![cfg_attr(target_arch = "riscv32", no_std)]

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::collections::HashSet;
#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::collections::BTreeSet as HashSet;
#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::evm::error::EvmError;
use crate::evm::{GAS_SLOAD_COLD, GAS_SLOAD_WARM, GAS_SSTORE_SENTRY, sstore_gas_cost};
use crate::evm::{Memory, MemoryError, Stack, StackError};
use crate::state::{State, Storage};
use crate::types::{Address, U256};

// =============================================================================
// Helper Functions
// =============================================================================

/// Convert U256 to usize, saturating to usize::MAX if the value is too large
#[inline]
fn u256_to_usize(value: &U256) -> usize {
    let bytes = value.to_le_bytes();
    let low_u64 = u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]);

    #[cfg(target_pointer_width = "64")]
    {
        low_u64 as usize
    }

    #[cfg(target_pointer_width = "32")]
    {
        if low_u64 > usize::MAX as u64 {
            usize::MAX
        } else {
            low_u64 as usize
        }
    }
}

/// Get the least significant byte from U256
#[inline]
fn u256_low_byte(value: &U256) -> u8 {
    let bytes = value.to_le_bytes();
    bytes[0]
}

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during opcode execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpcodeError {
    /// Stack error (overflow/underflow)
    Stack(StackError),
    /// Memory error (invalid offset/overflow)
    Memory(MemoryError),
    /// Invalid jump destination
    InvalidJumpDestination,
    /// Program counter out of bounds
    InvalidProgramCounter,
    /// Invalid offset or size
    InvalidOffset,
    /// Invalid PUSH (not enough bytes in code)
    InvalidPush,
    /// Operation not implemented yet
    NotImplemented,
}

impl From<StackError> for OpcodeError {
    fn from(err: StackError) -> Self {
        OpcodeError::Stack(err)
    }
}

impl From<MemoryError> for OpcodeError {
    fn from(err: MemoryError) -> Self {
        OpcodeError::Memory(err)
    }
}

// =============================================================================
// Memory Opcodes
// =============================================================================

/// MLOAD (0x51) - Load word from memory
///
/// Pops offset from stack, loads 32-byte word from memory at that offset,
/// pushes result onto stack.
pub fn mload(stack: &mut Stack, memory: &mut Memory) -> Result<(), OpcodeError> {
    let offset = stack.pop()?;
    let offset_usize = u256_to_usize(&offset);
    let value = memory.mload(offset_usize)?;
    stack.push(value)?;
    Ok(())
}

/// MSTORE (0x52) - Save word to memory
///
/// Pops offset and value from stack, stores 32-byte word to memory at that offset.
pub fn mstore(stack: &mut Stack, memory: &mut Memory) -> Result<(), OpcodeError> {
    let offset = stack.pop()?;
    let value = stack.pop()?;
    let offset_usize = u256_to_usize(&offset);
    memory.mstore(offset_usize, value)?;
    Ok(())
}

/// MSTORE8 (0x53) - Save byte to memory
///
/// Pops offset and value from stack, stores least significant byte to memory at that offset.
pub fn mstore8(stack: &mut Stack, memory: &mut Memory) -> Result<(), OpcodeError> {
    let offset = stack.pop()?;
    let value = stack.pop()?;
    let offset_usize = u256_to_usize(&offset);
    let byte = u256_low_byte(&value);
    memory.mstore8(offset_usize, byte)?;
    Ok(())
}

/// MSIZE (0x59) - Get size of active memory in bytes
///
/// Pushes the current memory size in bytes onto the stack.
pub fn msize(stack: &mut Stack, memory: &Memory) -> Result<(), OpcodeError> {
    let size = memory.msize();
    stack.push(U256::from(size as u64))?;
    Ok(())
}

// =============================================================================
// Storage Opcodes
// =============================================================================

/// SLOAD (0x54) - Load word from storage
///
/// Pops key from stack, loads value from contract storage, pushes result onto stack.
pub fn sload(stack: &mut Stack, storage: &Storage) -> Result<(), OpcodeError> {
    let key = stack.pop()?;
    let value = storage.get(&key);
    stack.push(value)?;
    Ok(())
}

/// SSTORE (0x55) - Save word to storage
///
/// Pops key and value from stack, stores value in contract storage at that key.
pub fn sstore(stack: &mut Stack, storage: &mut Storage) -> Result<(), OpcodeError> {
    let key = stack.pop()?;
    let value = stack.pop()?;
    storage.set(&key, value);
    Ok(())
}

// =============================================================================
// Stack Manipulation Opcodes
// =============================================================================

/// POP (0x50) - Remove item from stack
///
/// Pops the top item from the stack and discards it.
pub fn pop(stack: &mut Stack) -> Result<(), OpcodeError> {
    stack.pop()?;
    Ok(())
}

/// PUSH1-PUSH32 (0x60-0x7F) - Place n-byte item on stack
///
/// Reads n bytes from bytecode starting at pc+1 and pushes them onto the stack.
/// The value is zero-padded on the left if fewer than n bytes are available.
///
/// # Arguments
/// * `stack` - The EVM stack
/// * `bytecode` - The contract bytecode
/// * `pc` - Current program counter (pointing to the PUSH opcode)
/// * `n` - Number of bytes to push (1-32)
pub fn push_n(stack: &mut Stack, bytecode: &[u8], pc: usize, n: usize) -> Result<(), OpcodeError> {
    if n == 0 || n > 32 {
        return Err(OpcodeError::InvalidProgramCounter);
    }

    let mut value = U256::ZERO;
    for i in 0..n {
        let byte_index = pc + 1 + i;
        if byte_index < bytecode.len() {
            value = (value << 8) | U256::from(bytecode[byte_index] as u64);
        } else {
            // Zero-padding on the right (shift left by remaining bytes)
            value <<= 8 * (n - i) as u32;
            break;
        }
    }
    stack.push(value)?;
    Ok(())
}

/// PUSH with strict bounds: returns InvalidPush if not enough bytes (pc+1+n > bytecode.len()).
pub fn push_n_strict(
    stack: &mut Stack,
    bytecode: &[u8],
    pc: usize,
    n: usize,
) -> Result<(), OpcodeError> {
    if n == 0 || n > 32 {
        return Err(OpcodeError::InvalidProgramCounter);
    }
    let start = pc + 1;
    let end = start + n;
    if end > bytecode.len() {
        return Err(OpcodeError::InvalidPush);
    }
    let mut value = U256::ZERO;
    for i in 0..n {
        value = (value << 8) | U256::from(bytecode[start + i] as u64);
    }
    stack.push(value)?;
    Ok(())
}

/// PUSH0 (0x5F) - Push constant 0 onto the stack.
pub fn push0(stack: &mut Stack) -> Result<(), OpcodeError> {
    stack.push(U256::ZERO)?;
    Ok(())
}

/// DUP1-DUP16 (0x80-0x8F) - Duplicate nth stack item
///
/// Duplicates the nth item on the stack and pushes it to the top.
/// DUP1 duplicates the top item, DUP2 duplicates the second item, etc.
///
/// # Arguments
/// * `stack` - The EVM stack
/// * `n` - Position to duplicate (1-16)
pub fn dup_n(stack: &mut Stack, n: usize) -> Result<(), OpcodeError> {
    if n == 0 || n > 16 {
        return Err(OpcodeError::Stack(StackError::InvalidIndex));
    }
    stack.dup(n)?;
    Ok(())
}

/// SWAP1-SWAP16 (0x90-0x9F) - Exchange 1st and nth stack items
///
/// Swaps the top stack item with the nth item below it.
/// SWAP1 swaps top with second, SWAP2 swaps top with third, etc.
///
/// # Arguments
/// * `stack` - The EVM stack
/// * `n` - Position to swap with (1-16)
pub fn swap_n(stack: &mut Stack, n: usize) -> Result<(), OpcodeError> {
    if n == 0 || n > 16 {
        return Err(OpcodeError::Stack(StackError::InvalidIndex));
    }
    stack.swap(n)?;
    Ok(())
}

// =============================================================================
// Control Flow Opcodes
// =============================================================================

/// JUMP (0x56) - Alter the program counter
///
/// Pops destination from stack and jumps to that location.
/// The destination must be a valid JUMPDEST.
///
/// # Returns
/// The new program counter value
pub fn jump(
    stack: &mut Stack,
    bytecode: &[u8],
    valid_jumpdests: &HashSet<usize>,
) -> Result<usize, OpcodeError> {
    let dest = stack.pop()?;
    let dest_usize = u256_to_usize(&dest);

    if dest_usize >= bytecode.len() || !valid_jumpdests.contains(&dest_usize) {
        return Err(OpcodeError::InvalidJumpDestination);
    }

    Ok(dest_usize)
}

/// JUMPI (0x57) - Conditionally alter the program counter
///
/// Pops destination and condition from stack. If condition is non-zero,
/// jumps to destination (which must be a valid JUMPDEST).
/// Otherwise, continues to next instruction.
///
/// # Returns
/// The new program counter value (either destination or current_pc + 1)
pub fn jumpi(
    stack: &mut Stack,
    bytecode: &[u8],
    valid_jumpdests: &HashSet<usize>,
    current_pc: usize,
) -> Result<usize, OpcodeError> {
    let dest = stack.pop()?;
    let condition = stack.pop()?;

    if condition != U256::ZERO {
        let dest_usize = u256_to_usize(&dest);
        if dest_usize >= bytecode.len() || !valid_jumpdests.contains(&dest_usize) {
            return Err(OpcodeError::InvalidJumpDestination);
        }
        Ok(dest_usize)
    } else {
        // No jump, continue to next instruction
        Ok(current_pc + 1)
    }
}

/// JUMP (0x56) using a bitmap of valid JUMPDEST positions (O(1) check).
pub fn jump_bitmap(
    stack: &mut Stack,
    bytecode: &[u8],
    jumpdests: &[bool],
) -> Result<usize, OpcodeError> {
    let dest = stack.pop()?;
    let dest_usize = u256_to_usize(&dest);
    if dest_usize >= bytecode.len() || dest_usize >= jumpdests.len() || !jumpdests[dest_usize] {
        return Err(OpcodeError::InvalidJumpDestination);
    }
    Ok(dest_usize)
}

/// JUMPI (0x57) using a bitmap of valid JUMPDEST positions.
pub fn jumpi_bitmap(
    stack: &mut Stack,
    bytecode: &[u8],
    jumpdests: &[bool],
    current_pc: usize,
) -> Result<usize, OpcodeError> {
    let dest = stack.pop()?;
    let condition = stack.pop()?;
    if condition != U256::ZERO {
        let dest_usize = u256_to_usize(&dest);
        if dest_usize >= bytecode.len() || dest_usize >= jumpdests.len() || !jumpdests[dest_usize] {
            return Err(OpcodeError::InvalidJumpDestination);
        }
        Ok(dest_usize)
    } else {
        Ok(current_pc + 1)
    }
}

/// PC (0x58) - Get the value of the program counter
///
/// Pushes the current program counter value onto the stack.
/// This is the value BEFORE the PC instruction is executed.
pub fn pc(stack: &mut Stack, current_pc: usize) -> Result<(), OpcodeError> {
    stack.push(U256::from(current_pc as u64))?;
    Ok(())
}

/// GAS (0x5A) - Get the amount of available gas
///
/// Pushes the current amount of available gas onto the stack.
/// This includes the gas for the GAS instruction itself.
pub fn gas(stack: &mut Stack, gas_remaining: u64) -> Result<(), OpcodeError> {
    stack.push(U256::from(gas_remaining))?;
    Ok(())
}

/// JUMPDEST (0x5B) - Mark a valid destination for jumps
///
/// This is a no-op instruction that marks a valid jump destination.
/// It does nothing during execution but is used for jump validation.
pub fn jumpdest() -> Result<(), OpcodeError> {
    Ok(())
}

// =============================================================================
// Jump Destination Analysis
// =============================================================================

/// Analyzes bytecode to find all valid JUMPDEST positions
///
/// Scans through bytecode and identifies all valid JUMPDEST (0x5B) opcodes.
/// PUSH opcodes are handled specially since they contain data that should
/// not be interpreted as opcodes.
pub fn analyze_jumpdests(bytecode: &[u8]) -> HashSet<usize> {
    let mut jumpdests = HashSet::new();
    let mut pc = 0;

    while pc < bytecode.len() {
        let opcode = bytecode[pc];

        if opcode == 0x5B {
            // JUMPDEST
            jumpdests.insert(pc);
            pc += 1;
        } else if (0x60..=0x7F).contains(&opcode) {
            // PUSH1-PUSH32: skip the data bytes
            let push_size = (opcode - 0x5F) as usize;
            pc += 1 + push_size;
        } else {
            pc += 1;
        }
    }

    jumpdests
}

/// Analyzes bytecode to produce a bitmap: jumpdests[i] is true iff position i is a valid JUMPDEST.
pub fn analyze_jumpdests_bitmap(bytecode: &[u8]) -> Vec<bool> {
    let mut jumpdests = vec![false; bytecode.len()];
    let mut pc = 0;
    while pc < bytecode.len() {
        let opcode = bytecode[pc];
        if opcode == 0x5B {
            jumpdests[pc] = true;
            pc += 1;
        } else if (0x60..=0x7F).contains(&opcode) {
            let push_size = (opcode - 0x5F) as usize;
            pc += 1 + push_size;
        } else {
            pc += 1;
        }
    }
    jumpdests
}

/// MCOPY (0x5E) - Copy memory region (dest, src, size).
pub fn mcopy(stack: &mut Stack, memory: &mut Memory) -> Result<(), OpcodeError> {
    let dest = u256_to_usize(&stack.pop()?);
    let src = u256_to_usize(&stack.pop()?);
    let size = u256_to_usize(&stack.pop()?);
    memory.copy(dest, src, size).map_err(OpcodeError::Memory)?;
    Ok(())
}

// =============================================================================
// Storage opcodes with EIP-2929 / EIP-3529 (State trait)
// =============================================================================

/// SLOAD (0x54) with EIP-2929 warm/cold. Caller must charge base gas (GAS_SLOAD_COLD) first.
pub fn sload_eip2929<S: State>(
    stack: &mut Stack,
    state: &S,
    address: Address,
    is_warm: bool,
    gas_remaining: &mut u64,
) -> Result<(), EvmError> {
    let key = stack.pop().map_err(EvmError::from)?;
    if is_warm {
        *gas_remaining = (*gas_remaining).saturating_add(GAS_SLOAD_COLD - GAS_SLOAD_WARM);
    }
    let value = state.sload(&address, &key);
    stack.push(value).map_err(EvmError::from)?;
    Ok(())
}

/// SSTORE (0x55) with EIP-2929 and EIP-3529. Caller must charge base gas (0) first; this charges dynamic + access.
pub fn sstore_eip2929<S: State>(
    stack: &mut Stack,
    state: &mut S,
    address: Address,
    is_warm: bool,
    gas_remaining: &mut u64,
    gas_refund: &mut u64,
) -> Result<(), EvmError> {
    let key = stack.pop().map_err(EvmError::from)?;
    let new_value = stack.pop().map_err(EvmError::from)?;
    let current_value = state.sload(&address, &key);

    if *gas_remaining <= GAS_SSTORE_SENTRY {
        return Err(EvmError::OutOfGas);
    }

    let sstore_gas = sstore_gas_cost(current_value, new_value);
    super::utils::consume_gas(gas_remaining, sstore_gas)?;
    super::utils::consume_gas(gas_remaining, 2100)?;
    if is_warm {
        *gas_remaining = (*gas_remaining).saturating_add(2000);
    }

    if !current_value.is_zero() && new_value.is_zero() {
        *gas_refund += 4800;
    }

    state.sstore(&address, &key, new_value);
    Ok(())
}

/// TLOAD (0x5C) - Load from transient storage.
pub fn tload<S: State>(stack: &mut Stack, state: &S, address: Address) -> Result<(), EvmError> {
    let key = stack.pop().map_err(EvmError::from)?;
    let value = state.tload(&address, &key);
    stack.push(value).map_err(EvmError::from)?;
    Ok(())
}

/// TSTORE (0x5D) - Store to transient storage.
pub fn tstore<S: State>(
    stack: &mut Stack,
    state: &mut S,
    address: Address,
) -> Result<(), EvmError> {
    let key = stack.pop().map_err(EvmError::from)?;
    let value = stack.pop().map_err(EvmError::from)?;
    state.tstore(&address, &key, value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Memory Opcode Tests
    // =========================================================================

    #[test]
    fn test_mload_basic() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // Store a value first
        memory.mstore(0, U256::from(0x1234u64)).unwrap();

        // Load it back
        stack.push(U256::ZERO).unwrap();
        mload(&mut stack, &mut memory).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(0x1234u64));
    }

    #[test]
    fn test_mload_offset() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // Store at offset 32
        memory.mstore(32, U256::from(0xABCDu64)).unwrap();

        // Load from offset 32
        stack.push(U256::from(32u64)).unwrap();
        mload(&mut stack, &mut memory).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(0xABCDu64));
    }

    #[test]
    fn test_mstore_basic() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        stack.push(U256::from(0x9876u64)).unwrap();
        stack.push(U256::ZERO).unwrap();
        mstore(&mut stack, &mut memory).unwrap();

        let loaded = memory.mload(0).unwrap();
        assert_eq!(loaded, U256::from(0x9876u64));
    }

    #[test]
    fn test_mstore8_basic() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // Store byte 0xAB at offset 0
        stack.push(U256::from(0x1234ABu64)).unwrap(); // Only last byte should be stored
        stack.push(U256::ZERO).unwrap();
        mstore8(&mut stack, &mut memory).unwrap();

        // Load the word and check only the first byte
        let loaded = memory.mload(0).unwrap();
        let bytes = loaded.to_be_bytes();
        assert_eq!(bytes[0], 0xAB);
        // Other bytes should be zero
        assert_eq!(bytes[1], 0x00);
    }

    #[test]
    fn test_msize_empty() {
        let stack = &mut Stack::new();
        let memory = Memory::new();

        msize(stack, &memory).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_msize_after_store() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // Store at offset 0 - expands memory to 32 bytes
        memory.mstore(0, U256::from(42u64)).unwrap();

        msize(&mut stack, &memory).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(32u64));
    }

    #[test]
    fn test_msize_after_large_offset() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // Store at offset 64 - expands memory to 96 bytes
        memory.mstore(64, U256::from(123u64)).unwrap();

        msize(&mut stack, &memory).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(96u64));
    }

    // =========================================================================
    // Storage Opcode Tests
    // =========================================================================

    #[test]
    fn test_sload_empty() {
        let mut stack = Stack::new();
        let storage = Storage::new();

        stack.push(U256::from(42u64)).unwrap();
        sload(&mut stack, &storage).unwrap();

        // Empty storage returns zero
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_sstore_sload_roundtrip() {
        let mut stack = Stack::new();
        let mut storage = Storage::new();

        // Store value
        stack.push(U256::from(0x1234u64)).unwrap();
        stack.push(U256::from(5u64)).unwrap(); // key
        sstore(&mut stack, &mut storage).unwrap();

        // Load value
        stack.push(U256::from(5u64)).unwrap();
        sload(&mut stack, &storage).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(0x1234u64));
    }

    #[test]
    fn test_sstore_zero_deletes() {
        let mut stack = Stack::new();
        let mut storage = Storage::new();

        // Store non-zero value
        stack.push(U256::from(100u64)).unwrap();
        stack.push(U256::from(7u64)).unwrap();
        sstore(&mut stack, &mut storage).unwrap();

        // Store zero (delete)
        stack.push(U256::ZERO).unwrap();
        stack.push(U256::from(7u64)).unwrap();
        sstore(&mut stack, &mut storage).unwrap();

        // Load should return zero
        stack.push(U256::from(7u64)).unwrap();
        sload(&mut stack, &storage).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    // =========================================================================
    // Stack Manipulation Tests
    // =========================================================================

    #[test]
    fn test_pop_basic() {
        let mut stack = Stack::new();
        stack.push(U256::from(1u64)).unwrap();
        stack.push(U256::from(2u64)).unwrap();
        stack.push(U256::from(3u64)).unwrap();

        pop(&mut stack).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(2u64));
        assert_eq!(stack.pop().unwrap(), U256::from(1u64));
    }

    #[test]
    fn test_pop_empty_stack() {
        let mut stack = Stack::new();
        let result = pop(&mut stack);
        assert!(matches!(
            result,
            Err(OpcodeError::Stack(StackError::Underflow))
        ));
    }

    #[test]
    fn test_push1() {
        let mut stack = Stack::new();
        let bytecode = vec![0x60, 0x42]; // PUSH1 0x42

        push_n(&mut stack, &bytecode, 0, 1).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(0x42u64));
    }

    #[test]
    fn test_push2() {
        let mut stack = Stack::new();
        let bytecode = vec![0x61, 0x12, 0x34]; // PUSH2 0x1234

        push_n(&mut stack, &bytecode, 0, 2).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(0x1234u64));
    }

    #[test]
    fn test_push32_max() {
        let mut stack = Stack::new();
        let mut bytecode = vec![0x7F]; // PUSH32
        bytecode.extend_from_slice(&[0xFF; 32]);

        push_n(&mut stack, &bytecode, 0, 32).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::MAX);
    }

    #[test]
    fn test_push_partial_data() {
        let mut stack = Stack::new();
        let bytecode = vec![0x62, 0x12]; // PUSH3 with only 1 byte (missing 2 bytes)

        push_n(&mut stack, &bytecode, 0, 3).unwrap();

        // Should be left-padded: 0x12 followed by two zero bytes = 0x120000
        assert_eq!(stack.pop().unwrap(), U256::from(0x120000u64));
    }

    #[test]
    fn test_dup1() {
        let mut stack = Stack::new();
        stack.push(U256::from(42u64)).unwrap();

        dup_n(&mut stack, 1).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(42u64));
        assert_eq!(stack.pop().unwrap(), U256::from(42u64));
    }

    #[test]
    fn test_dup2() {
        let mut stack = Stack::new();
        stack.push(U256::from(1u64)).unwrap();
        stack.push(U256::from(2u64)).unwrap();

        dup_n(&mut stack, 2).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(1u64));
        assert_eq!(stack.pop().unwrap(), U256::from(2u64));
        assert_eq!(stack.pop().unwrap(), U256::from(1u64));
    }

    #[test]
    fn test_dup16() {
        let mut stack = Stack::new();
        for i in 1..=16 {
            stack.push(U256::from(i as u64)).unwrap();
        }

        dup_n(&mut stack, 16).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(1u64));
    }

    #[test]
    fn test_swap1() {
        let mut stack = Stack::new();
        stack.push(U256::from(1u64)).unwrap();
        stack.push(U256::from(2u64)).unwrap();

        swap_n(&mut stack, 1).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(1u64));
        assert_eq!(stack.pop().unwrap(), U256::from(2u64));
    }

    #[test]
    fn test_swap2() {
        let mut stack = Stack::new();
        stack.push(U256::from(1u64)).unwrap();
        stack.push(U256::from(2u64)).unwrap();
        stack.push(U256::from(3u64)).unwrap();

        swap_n(&mut stack, 2).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(1u64));
        assert_eq!(stack.pop().unwrap(), U256::from(2u64));
        assert_eq!(stack.pop().unwrap(), U256::from(3u64));
    }

    #[test]
    fn test_swap16() {
        let mut stack = Stack::new();
        for i in 1..=17 {
            stack.push(U256::from(i as u64)).unwrap();
        }

        swap_n(&mut stack, 16).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(1u64));
        for _ in 2..=16 {
            stack.pop().unwrap();
        }
        assert_eq!(stack.pop().unwrap(), U256::from(17u64));
    }

    // =========================================================================
    // Control Flow Tests
    // =========================================================================

    #[test]
    fn test_pc_basic() {
        let mut stack = Stack::new();

        pc(&mut stack, 42).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(42u64));
    }

    #[test]
    fn test_gas_basic() {
        let mut stack = Stack::new();

        gas(&mut stack, 1000).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(1000u64));
    }

    #[test]
    fn test_jumpdest_noop() {
        // JUMPDEST is a no-op
        assert!(jumpdest().is_ok());
    }

    #[test]
    fn test_analyze_jumpdests_simple() {
        let bytecode = vec![
            0x60, 0x0A, // PUSH1 10
            0x5B, // JUMPDEST at position 2
            0x56, // JUMP
        ];

        let jumpdests = analyze_jumpdests(&bytecode);

        assert_eq!(jumpdests.len(), 1);
        assert!(jumpdests.contains(&2));
    }

    #[test]
    fn test_analyze_jumpdests_with_push() {
        let bytecode = vec![
            0x60, 0x5B, // PUSH1 0x5B (not a real JUMPDEST!)
            0x5B, // JUMPDEST at position 2
            0x61, 0x5B, 0x5B, // PUSH2 0x5B5B (not real JUMPDESTs!)
            0x5B, // JUMPDEST at position 6
        ];

        let jumpdests = analyze_jumpdests(&bytecode);

        assert_eq!(jumpdests.len(), 2);
        assert!(jumpdests.contains(&2));
        assert!(jumpdests.contains(&6));
        assert!(!jumpdests.contains(&1)); // Inside PUSH1
        assert!(!jumpdests.contains(&4)); // Inside PUSH2
        assert!(!jumpdests.contains(&5)); // Inside PUSH2
    }

    #[test]
    fn test_jump_valid() {
        let mut stack = Stack::new();
        let bytecode = vec![0x5B, 0x00, 0x5B]; // JUMPDEST at 0 and 2
        let jumpdests = analyze_jumpdests(&bytecode);

        stack.push(U256::from(2u64)).unwrap();
        let new_pc = jump(&mut stack, &bytecode, &jumpdests).unwrap();

        assert_eq!(new_pc, 2);
    }

    #[test]
    fn test_jump_invalid_destination() {
        let mut stack = Stack::new();
        let bytecode = vec![0x5B, 0x00, 0x00]; // Only JUMPDEST at 0
        let jumpdests = analyze_jumpdests(&bytecode);

        stack.push(U256::from(1u64)).unwrap(); // Not a JUMPDEST
        let result = jump(&mut stack, &bytecode, &jumpdests);

        assert!(matches!(result, Err(OpcodeError::InvalidJumpDestination)));
    }

    #[test]
    fn test_jump_out_of_bounds() {
        let mut stack = Stack::new();
        let bytecode = vec![0x5B];
        let jumpdests = analyze_jumpdests(&bytecode);

        stack.push(U256::from(100u64)).unwrap();
        let result = jump(&mut stack, &bytecode, &jumpdests);

        assert!(matches!(result, Err(OpcodeError::InvalidJumpDestination)));
    }

    #[test]
    fn test_jumpi_taken() {
        let mut stack = Stack::new();
        let bytecode = vec![0x5B, 0x00, 0x5B]; // JUMPDEST at 0 and 2
        let jumpdests = analyze_jumpdests(&bytecode);

        stack.push(U256::from(1u64)).unwrap(); // condition (non-zero)
        stack.push(U256::from(2u64)).unwrap(); // destination
        let new_pc = jumpi(&mut stack, &bytecode, &jumpdests, 0).unwrap();

        assert_eq!(new_pc, 2);
    }

    #[test]
    fn test_jumpi_not_taken() {
        let mut stack = Stack::new();
        let bytecode = vec![0x5B, 0x00, 0x5B];
        let jumpdests = analyze_jumpdests(&bytecode);

        stack.push(U256::ZERO).unwrap(); // condition (zero)
        stack.push(U256::from(2u64)).unwrap(); // destination
        let new_pc = jumpi(&mut stack, &bytecode, &jumpdests, 0).unwrap();

        assert_eq!(new_pc, 1); // current_pc + 1
    }

    #[test]
    fn test_jumpi_invalid_destination() {
        let mut stack = Stack::new();
        let bytecode = vec![0x5B, 0x00, 0x00]; // Only JUMPDEST at 0
        let jumpdests = analyze_jumpdests(&bytecode);

        stack.push(U256::from(1u64)).unwrap(); // condition (non-zero)
        stack.push(U256::from(1u64)).unwrap(); // destination (not a JUMPDEST)
        let result = jumpi(&mut stack, &bytecode, &jumpdests, 0);

        assert!(matches!(result, Err(OpcodeError::InvalidJumpDestination)));
    }

    // =========================================================================
    // Edge Case Tests
    // =========================================================================

    #[test]
    fn test_mload_mstore_large_offset() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // Store at large offset
        let offset = 1024u64;
        stack.push(U256::from(0xDEADBEEFu64)).unwrap();
        stack.push(U256::from(offset)).unwrap();
        mstore(&mut stack, &mut memory).unwrap();

        // Load it back
        stack.push(U256::from(offset)).unwrap();
        mload(&mut stack, &mut memory).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from(0xDEADBEEFu64));
    }

    #[test]
    fn test_sload_multiple_keys() {
        let mut stack = Stack::new();
        let mut storage = Storage::new();

        // Store multiple values
        for i in 0u64..10 {
            stack.push(U256::from(i * 100)).unwrap();
            stack.push(U256::from(i)).unwrap();
            sstore(&mut stack, &mut storage).unwrap();
        }

        // Load them all back
        for i in 0u64..10 {
            stack.push(U256::from(i)).unwrap();
            sload(&mut stack, &storage).unwrap();
            assert_eq!(stack.pop().unwrap(), U256::from(i * 100));
        }
    }

    #[test]
    fn test_push_all_sizes() {
        for n in 1..=32 {
            let mut stack = Stack::new();
            let mut bytecode = vec![0x5F + n as u8]; // PUSH opcode
            bytecode.extend_from_slice(&vec![0xFF; n]);

            push_n(&mut stack, &bytecode, 0, n).unwrap();

            let result = stack.pop().unwrap();
            // For n bytes of 0xFF, result should be 2^(8n) - 1
            let mut expected = U256::ZERO;
            for _ in 0..n {
                expected = (expected << 8) | U256::from(0xFFu64);
            }
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_dup_invalid_n() {
        let mut stack = Stack::new();
        stack.push(U256::from(42u64)).unwrap();

        assert!(dup_n(&mut stack, 0).is_err());
        assert!(dup_n(&mut stack, 17).is_err());
    }

    #[test]
    fn test_swap_invalid_n() {
        let mut stack = Stack::new();
        stack.push(U256::from(1u64)).unwrap();
        stack.push(U256::from(2u64)).unwrap();

        assert!(swap_n(&mut stack, 0).is_err());
        assert!(swap_n(&mut stack, 17).is_err());
    }

    #[test]
    fn test_complex_jump_pattern() {
        // Test a more complex bytecode with multiple jumps
        let bytecode = vec![
            0x5B, // 0: JUMPDEST
            0x60, 0x06, // 1-2: PUSH1 6
            0x56, // 3: JUMP
            0x00, // 4: STOP (dead code)
            0x00, // 5: STOP (dead code)
            0x5B, // 6: JUMPDEST
            0x60, 0x0A, // 7-8: PUSH1 10
            0x57, // 9: JUMPI
            0x5B, // 10: JUMPDEST
        ];

        let jumpdests = analyze_jumpdests(&bytecode);
        assert_eq!(jumpdests.len(), 3);
        assert!(jumpdests.contains(&0));
        assert!(jumpdests.contains(&6));
        assert!(jumpdests.contains(&10));
    }

    #[test]
    fn test_memory_expansion_multiple_stores() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // Store at multiple offsets
        for i in 0u64..5 {
            stack.push(U256::from(i * 111)).unwrap();
            stack.push(U256::from(i * 32)).unwrap();
            mstore(&mut stack, &mut memory).unwrap();
        }

        // Verify memory size expanded correctly
        msize(&mut stack, &memory).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from(160u64)); // 5 * 32 = 160 bytes
    }
}
