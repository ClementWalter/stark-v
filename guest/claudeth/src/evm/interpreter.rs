//! EVM Bytecode Interpreter
//!
//! This module implements the Ethereum Virtual Machine bytecode interpreter.
//! It wires together Stack, Memory, Gas metering, and all 119 opcodes into
//! a complete bytecode execution engine.
//!
//! ## Usage
//!
//! ```no_run
//! use claudeth::evm::interpreter::{execute_bytecode, ExecutionResult};
//! use claudeth::state::InMemoryState;
//!
//! // Execute simple bytecode: PUSH1 0x42 PUSH1 0x00 MSTORE STOP
//! let code = vec![0x60, 0x42, 0x60, 0x00, 0x52, 0x00];
//! let result = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
//! assert!(result.success);
//! ```

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::evm::host::{CallKind, CallMessage, CreateMessage, Host, NullHost};
use crate::evm::memory::{Memory, MemoryError};
use crate::evm::opcodes::arithmetic::EvmError as OpcodeError;
use crate::evm::stack::{Stack, StackError};
use crate::evm::{
    create2_hash_cost, init_code_gas_cost, memory_expansion_cost, opcode_gas_cost,
    GAS_CALL_NEW_ACCOUNT, GAS_CALL_STIPEND, GAS_CALL_VALUE_TRANSFER,
};
use crate::state::State;
use crate::types::{Address, Hash, U256};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during EVM execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvmError {
    /// Stack error (overflow, underflow, invalid index)
    StackError(StackError),
    /// Memory error (invalid offset, overflow)
    MemoryError(MemoryError),
    /// Insufficient gas for operation
    OutOfGas,
    /// Invalid opcode encountered
    InvalidOpcode(u8),
    /// Invalid JUMP destination
    InvalidJump,
    /// Execution reverted
    Revert(Vec<u8>),
    /// PC out of bounds
    PcOutOfBounds,
    /// Invalid PUSH data (not enough bytes)
    InvalidPush,
}

impl From<StackError> for EvmError {
    fn from(err: StackError) -> Self {
        EvmError::StackError(err)
    }
}

impl From<MemoryError> for EvmError {
    fn from(err: MemoryError) -> Self {
        EvmError::MemoryError(err)
    }
}

impl From<OpcodeError> for EvmError {
    fn from(err: OpcodeError) -> Self {
        match err {
            OpcodeError::Stack(e) => EvmError::StackError(e),
            OpcodeError::Memory(e) => EvmError::MemoryError(e),
        }
    }
}

// =============================================================================
// Execution Result
// =============================================================================

/// Result of bytecode execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    /// Gas used during execution
    pub gas_used: u64,
    /// Return data (from RETURN or REVERT)
    pub return_data: Vec<u8>,
    /// Final stack state (for debugging)
    pub stack: Stack,
    /// Final memory state (for debugging)
    pub memory: Memory,
}

// =============================================================================
// Context Structures (Simplified)
// =============================================================================

/// Block context for environment opcodes
#[derive(Debug, Clone)]
pub struct BlockContext {
    pub number: U256,
    pub timestamp: U256,
    pub coinbase: Address,
    pub difficulty: U256,
    pub gas_limit: U256,
    pub chain_id: U256,
    pub base_fee: U256,
}

impl Default for BlockContext {
    fn default() -> Self {
        BlockContext {
            number: U256::ZERO,
            timestamp: U256::ZERO,
            coinbase: Address::ZERO,
            difficulty: U256::ZERO,
            gas_limit: U256::from_u64(30_000_000),
            chain_id: U256::ONE,
            base_fee: U256::ZERO,
        }
    }
}

/// Transaction context for environment opcodes
#[derive(Debug, Clone)]
pub struct TxContext {
    pub origin: Address,
    pub gas_price: U256,
}

impl Default for TxContext {
    fn default() -> Self {
        TxContext {
            origin: Address::ZERO,
            gas_price: U256::ZERO,
        }
    }
}

/// Call context for contract execution
#[derive(Debug, Clone)]
pub struct CallContext {
    pub address: Address,
    pub caller: Address,
    pub call_value: U256,
    pub call_data: Vec<u8>,
}

impl Default for CallContext {
    fn default() -> Self {
        CallContext {
            address: Address::ZERO,
            caller: Address::ZERO,
            call_value: U256::ZERO,
            call_data: Vec::new(),
        }
    }
}

// =============================================================================
// EVM State
// =============================================================================

/// EVM execution state
pub struct Evm<S, H> {
    stack: Stack,
    memory: Memory,
    gas_remaining: u64,
    pc: usize,
    code: Vec<u8>,
    stopped: bool,
    return_data: Vec<u8>,
    block_ctx: BlockContext,
    tx_ctx: TxContext,
    call_ctx: CallContext,
    jumpdests: Vec<bool>, // Valid JUMPDEST positions
    state: S,             // State interface for account/storage access
    host: H,              // Host interface for calls/creates
}

// =============================================================================
// Helper Functions
// =============================================================================

fn address_to_u256(address: &Address) -> U256 {
    let mut bytes = [0u8; 32];
    bytes[12..].copy_from_slice(&address.to_bytes());
    U256::from_be_bytes(bytes)
}

fn u256_to_address(u256: &U256) -> Address {
    let bytes = u256.to_be_bytes();
    // Address is the last 20 bytes
    let mut addr_bytes = [0u8; 20];
    addr_bytes.copy_from_slice(&bytes[12..]);
    Address::from_slice(&addr_bytes).expect("20 bytes should always create an address")
}

fn hash_to_u256(hash: &Hash) -> U256 {
    U256::from_be_bytes(*hash.as_bytes())
}

impl<S: State, H: Host<S>> Evm<S, H> {
    /// Create a new EVM instance
    pub fn new(code: Vec<u8>, gas_limit: u64, state: S, host: H) -> Self {
        let jumpdests = Self::analyze_jumpdests(&code);
        Evm {
            stack: Stack::new(),
            memory: Memory::new(),
            gas_remaining: gas_limit,
            pc: 0,
            code,
            stopped: false,
            return_data: Vec::new(),
            block_ctx: BlockContext::default(),
            tx_ctx: TxContext::default(),
            call_ctx: CallContext::default(),
            jumpdests,
            state,
            host,
        }
    }

    /// Analyze code to find valid JUMPDEST positions
    fn analyze_jumpdests(code: &[u8]) -> Vec<bool> {
        let mut jumpdests = vec![false; code.len()];
        let mut i = 0;
        while i < code.len() {
            let opcode = code[i];
            if opcode == 0x5B {
                // JUMPDEST
                jumpdests[i] = true;
                i += 1;
            } else if (0x60..=0x7F).contains(&opcode) {
                // PUSH1-PUSH32: skip push data
                let n = (opcode - 0x5F) as usize;
                i += 1 + n;
            } else {
                i += 1;
            }
        }
        jumpdests
    }

    /// Consume gas
    fn consume_gas(&mut self, amount: u64) -> Result<(), EvmError> {
        if self.gas_remaining < amount {
            return Err(EvmError::OutOfGas);
        }
        self.gas_remaining -= amount;
        Ok(())
    }

    fn read_memory_bytes(&mut self, offset: usize, size: usize) -> Result<Vec<u8>, EvmError> {
        let mut out = Vec::with_capacity(size);
        for i in 0..size {
            if offset + i < self.memory.msize() {
                let value = self.memory.mload((offset + i) & !31)?;
                let byte_offset = (offset + i) % 32;
                let bytes = value.to_be_bytes();
                out.push(bytes[byte_offset]);
            } else {
                out.push(0);
            }
        }
        Ok(out)
    }

    fn write_memory_bytes(
        &mut self,
        offset: usize,
        data: &[u8],
        size: usize,
    ) -> Result<(), EvmError> {
        for i in 0..size {
            let byte = if i < data.len() { data[i] } else { 0 };
            self.memory.mstore8(offset + i, byte)?;
        }
        Ok(())
    }

    /// Get current opcode
    fn current_opcode(&self) -> Result<u8, EvmError> {
        if self.pc >= self.code.len() {
            return Err(EvmError::PcOutOfBounds);
        }
        Ok(self.code[self.pc])
    }

    /// Read immediate bytes for PUSH
    fn read_push_data(&mut self, n: usize) -> Result<Vec<u8>, EvmError> {
        let start = self.pc + 1;
        let end = start + n;
        if end > self.code.len() {
            return Err(EvmError::InvalidPush);
        }
        Ok(self.code[start..end].to_vec())
    }

    /// Execute a single step
    pub fn step(&mut self) -> Result<(), EvmError> {
        if self.stopped || self.pc >= self.code.len() {
            self.stopped = true;
            return Ok(());
        }

        let opcode = self.current_opcode()?;

        // Charge base gas
        let base_gas = opcode_gas_cost(opcode);
        self.consume_gas(base_gas)?;

        // Execute opcode
        match opcode {
            // 0x00: STOP
            0x00 => {
                self.stopped = true;
            }

            // 0x01-0x0B: Arithmetic
            0x01 => crate::evm::opcodes::arithmetic::add(&mut self.stack)?,
            0x02 => crate::evm::opcodes::arithmetic::mul(&mut self.stack)?,
            0x03 => crate::evm::opcodes::arithmetic::sub(&mut self.stack)?,
            0x04 => crate::evm::opcodes::arithmetic::div(&mut self.stack)?,
            0x05 => crate::evm::opcodes::arithmetic::sdiv(&mut self.stack)?,
            0x06 => crate::evm::opcodes::arithmetic::modulo(&mut self.stack)?,
            0x07 => crate::evm::opcodes::arithmetic::smod(&mut self.stack)?,
            0x08 => crate::evm::opcodes::arithmetic::addmod(&mut self.stack)?,
            0x09 => crate::evm::opcodes::arithmetic::mulmod(&mut self.stack)?,
            0x0A => {
                let exponent = *self.stack.peek(1)?;
                let exp_bytes = exponent.bits().div_ceil(8);
                self.consume_gas(50 * exp_bytes as u64)?;
                crate::evm::opcodes::arithmetic::exp(&mut self.stack)?;
            }
            0x0B => crate::evm::opcodes::arithmetic::signextend(&mut self.stack)?,

            // 0x10-0x15: Comparison
            0x10 => crate::evm::opcodes::arithmetic::lt(&mut self.stack)?,
            0x11 => crate::evm::opcodes::arithmetic::gt(&mut self.stack)?,
            0x12 => crate::evm::opcodes::arithmetic::slt(&mut self.stack)?,
            0x13 => crate::evm::opcodes::arithmetic::sgt(&mut self.stack)?,
            0x14 => crate::evm::opcodes::arithmetic::eq(&mut self.stack)?,
            0x15 => crate::evm::opcodes::arithmetic::iszero(&mut self.stack)?,

            // 0x16-0x1D: Bitwise
            0x16 => crate::evm::opcodes::arithmetic::and(&mut self.stack)?,
            0x17 => crate::evm::opcodes::arithmetic::or(&mut self.stack)?,
            0x18 => crate::evm::opcodes::arithmetic::xor(&mut self.stack)?,
            0x19 => crate::evm::opcodes::arithmetic::not(&mut self.stack)?,
            0x1A => crate::evm::opcodes::arithmetic::byte(&mut self.stack)?,
            0x1B => crate::evm::opcodes::arithmetic::shl(&mut self.stack)?,
            0x1C => crate::evm::opcodes::arithmetic::shr(&mut self.stack)?,
            0x1D => crate::evm::opcodes::arithmetic::sar(&mut self.stack)?,

            // 0x20: KECCAK256
            0x20 => {
                let offset = self.stack.peek(0)?.as_usize();
                let size = self.stack.peek(1)?.as_usize();

                // Memory expansion
                let mem_cost = memory_expansion_cost(self.memory.msize(), offset + size);
                self.consume_gas(mem_cost)?;

                // Hash cost
                let words = size.div_ceil(32);
                self.consume_gas(6 * words as u64)?;

                crate::evm::opcodes::arithmetic::keccak256(&mut self.stack, &mut self.memory)?;
            }

            // 0x30-0x3F: Environment
            0x30 => {
                // ADDRESS
                let address_u256 = address_to_u256(&self.call_ctx.address);
                self.stack.push(address_u256)?;
            }
            0x31 => {
                // BALANCE: get balance of account
                let address_u256 = self.stack.pop()?;
                let address = u256_to_address(&address_u256);
                let balance = self.state.get_balance(&address);
                self.stack.push(balance)?;
            }
            0x32 => {
                // ORIGIN
                let origin_u256 = address_to_u256(&self.tx_ctx.origin);
                self.stack.push(origin_u256)?;
            }
            0x33 => {
                // CALLER
                let caller_u256 = address_to_u256(&self.call_ctx.caller);
                self.stack.push(caller_u256)?;
            }
            0x34 => {
                // CALLVALUE
                self.stack.push(self.call_ctx.call_value)?;
            }
            0x35 => {
                // CALLDATALOAD
                let offset = self.stack.pop()?.as_usize();
                let mut data = [0u8; 32];

                let call_data_len = self.call_ctx.call_data.len();
                if offset < call_data_len {
                    let end = (offset + 32).min(call_data_len);
                    let copy_len = end - offset;
                    data[..copy_len].copy_from_slice(&self.call_ctx.call_data[offset..end]);
                }

                self.stack.push(U256::from_be_bytes(data))?;
            }
            0x36 => {
                // CALLDATASIZE
                self.stack
                    .push(U256::from_u64(self.call_ctx.call_data.len() as u64))?;
            }
            0x37 => {
                // CALLDATACOPY
                let dest = self.stack.pop()?.as_usize();
                let offset = self.stack.pop()?.as_usize();
                let size = self.stack.pop()?.as_usize();

                // Memory expansion
                let mem_cost = memory_expansion_cost(self.memory.msize(), dest + size);
                self.consume_gas(mem_cost)?;

                // Copy cost
                let words = size.div_ceil(32);
                self.consume_gas(3 * words as u64)?;

                // Copy call data to memory (zero-padded)
                for i in 0..size {
                    let byte = if offset + i < self.call_ctx.call_data.len() {
                        self.call_ctx.call_data[offset + i]
                    } else {
                        0
                    };
                    self.memory.mstore8(dest + i, byte)?;
                }
            }
            0x38 => {
                // CODESIZE
                self.stack.push(U256::from_u64(self.code.len() as u64))?;
            }
            0x39 => {
                // CODECOPY
                let dest = self.stack.pop()?.as_usize();
                let offset = self.stack.pop()?.as_usize();
                let size = self.stack.pop()?.as_usize();

                // Memory expansion
                let mem_cost = memory_expansion_cost(self.memory.msize(), dest + size);
                self.consume_gas(mem_cost)?;

                // Copy cost
                let words = size.div_ceil(32);
                self.consume_gas(3 * words as u64)?;

                // Copy code to memory
                for i in 0..size {
                    let byte = if offset + i < self.code.len() {
                        self.code[offset + i]
                    } else {
                        0
                    };
                    self.memory.mstore8(dest + i, byte)?;
                }
            }
            0x3A => {
                // GASPRICE
                self.stack.push(self.tx_ctx.gas_price)?;
            }
            0x3B => {
                // EXTCODESIZE: get code size of account
                let address_u256 = self.stack.pop()?;
                let address = u256_to_address(&address_u256);
                let code = self.state.get_code(&address);
                self.stack.push(U256::from_u64(code.len() as u64))?;
            }
            0x3C => {
                // EXTCODECOPY: copy code from external account
                let address_u256 = self.stack.pop()?;
                let dest = self.stack.pop()?.as_usize();
                let offset = self.stack.pop()?.as_usize();
                let size = self.stack.pop()?.as_usize();

                let address = u256_to_address(&address_u256);
                // Clone the code to avoid borrow checker issues
                #[cfg(not(target_arch = "riscv32"))]
                let code = self.state.get_code(&address).to_vec();
                #[cfg(target_arch = "riscv32")]
                let code = {
                    let slice = self.state.get_code(&address);
                    let mut vec = alloc::vec::Vec::new();
                    vec.extend_from_slice(slice);
                    vec
                };

                // Memory expansion
                let mem_cost = memory_expansion_cost(self.memory.msize(), dest + size);
                self.consume_gas(mem_cost)?;

                // Copy cost
                let words = size.div_ceil(32);
                self.consume_gas(3 * words as u64)?;

                // Copy code to memory
                for i in 0..size {
                    let byte = if offset + i < code.len() {
                        code[offset + i]
                    } else {
                        0
                    };
                    self.memory.mstore8(dest + i, byte)?;
                }
            }
            0x3D => {
                // RETURNDATASIZE
                self.stack.push(U256::from_u64(self.return_data.len() as u64))?;
            }
            0x3E => {
                // RETURNDATACOPY
                let dest = self.stack.pop()?.as_usize();
                let offset = self.stack.pop()?.as_usize();
                let size = self.stack.pop()?.as_usize();

                let end = offset
                    .checked_add(size)
                    .ok_or(EvmError::MemoryError(MemoryError::InvalidOffset))?;
                if end > self.return_data.len() {
                    return Err(EvmError::MemoryError(MemoryError::InvalidOffset));
                }

                // Memory expansion
                let mem_cost = memory_expansion_cost(self.memory.msize(), dest + size);
                self.consume_gas(mem_cost)?;

                // Copy cost
                let words = size.div_ceil(32);
                self.consume_gas(3 * words as u64)?;

                for i in 0..size {
                    self.memory.mstore8(dest + i, self.return_data[offset + i])?;
                }
            }
            0x3F => {
                // EXTCODEHASH: get code hash of account
                let address_u256 = self.stack.pop()?;
                let address = u256_to_address(&address_u256);
                let code_hash = self.state.get_code_hash(&address);
                let hash_u256 = hash_to_u256(&code_hash);
                self.stack.push(hash_u256)?;
            }

            // 0x40-0x4A: Block information
            0x40 => {
                // BLOCKHASH
                let number = self.stack.pop()?;
                let hash = self.host.blockhash(&number).unwrap_or(Hash::ZERO);
                self.stack.push(hash_to_u256(&hash))?;
            }
            0x41 => {
                // COINBASE
                let coinbase_u256 = U256::from_be_bytes({
                    let mut bytes = [0u8; 32];
                    bytes[12..].copy_from_slice(&self.block_ctx.coinbase.to_bytes());
                    bytes
                });
                self.stack.push(coinbase_u256)?;
            }
            0x42 => {
                // TIMESTAMP
                self.stack.push(self.block_ctx.timestamp)?;
            }
            0x43 => {
                // NUMBER
                self.stack.push(self.block_ctx.number)?;
            }
            0x44 => {
                // DIFFICULTY
                self.stack.push(self.block_ctx.difficulty)?;
            }
            0x45 => {
                // GASLIMIT
                self.stack.push(self.block_ctx.gas_limit)?;
            }
            0x46 => {
                // CHAINID
                self.stack.push(self.block_ctx.chain_id)?;
            }
            0x47 => {
                // SELFBALANCE: get balance of current contract
                let balance = self.state.get_balance(&self.call_ctx.address);
                self.stack.push(balance)?;
            }
            0x48 => {
                // BASEFEE
                self.stack.push(self.block_ctx.base_fee)?;
            }
            0x49 => {
                // BLOBHASH
                let index = self.stack.pop()?;
                let hash = self.host.blobhash(&index).unwrap_or(Hash::ZERO);
                self.stack.push(hash_to_u256(&hash))?;
            }
            0x4A => {
                // BLOBBASEFEE
                self.stack.push(self.host.blobbasefee())?;
            }

            // 0x50-0x5F: Stack, Memory, Storage, Flow
            0x50 => {
                // POP
                self.stack.pop()?;
            }
            0x51 => {
                // MLOAD
                let offset = self.stack.pop()?.as_usize();
                let mem_cost = memory_expansion_cost(self.memory.msize(), offset + 32);
                self.consume_gas(mem_cost)?;
                let value = self.memory.mload(offset)?;
                self.stack.push(value)?;
            }
            0x52 => {
                // MSTORE
                let offset = self.stack.pop()?.as_usize();
                let value = self.stack.pop()?;
                let mem_cost = memory_expansion_cost(self.memory.msize(), offset + 32);
                self.consume_gas(mem_cost)?;
                self.memory.mstore(offset, value)?;
            }
            0x53 => {
                // MSTORE8
                let offset = self.stack.pop()?.as_usize();
                let value = self.stack.pop()?;
                let mem_cost = memory_expansion_cost(self.memory.msize(), offset + 1);
                self.consume_gas(mem_cost)?;
                self.memory.mstore8(offset, value.as_u8())?;
            }
            0x54 => {
                // SLOAD: load from permanent storage
                let key = self.stack.pop()?;
                let value = self.state.sload(&self.call_ctx.address, &key);
                self.stack.push(value)?;
            }
            0x55 => {
                // SSTORE: store to permanent storage
                let key = self.stack.pop()?;
                let value = self.stack.pop()?;
                self.state.sstore(&self.call_ctx.address, &key, value);
            }
            0x56 => {
                // JUMP
                let dest = self.stack.pop()?.as_usize();
                if dest >= self.code.len() || !self.jumpdests[dest] {
                    return Err(EvmError::InvalidJump);
                }
                self.pc = dest;
                return Ok(()); // Don't increment PC
            }
            0x57 => {
                // JUMPI
                let dest = self.stack.pop()?.as_usize();
                let condition = self.stack.pop()?;
                if !condition.is_zero() {
                    if dest >= self.code.len() || !self.jumpdests[dest] {
                        return Err(EvmError::InvalidJump);
                    }
                    self.pc = dest;
                    return Ok(()); // Don't increment PC
                }
            }
            0x58 => {
                // PC
                self.stack.push(U256::from_u64(self.pc as u64))?;
            }
            0x59 => {
                // MSIZE
                self.stack.push(U256::from_u64(self.memory.msize() as u64))?;
            }
            0x5A => {
                // GAS
                self.stack.push(U256::from_u64(self.gas_remaining))?;
            }
            0x5B => {
                // JUMPDEST: no-op
            }
            0x5C => {
                // TLOAD: load from transient storage (EIP-1153)
                let key = self.stack.pop()?;
                let value = self.state.tload(&self.call_ctx.address, &key);
                self.stack.push(value)?;
            }
            0x5D => {
                // TSTORE: store to transient storage (EIP-1153)
                let key = self.stack.pop()?;
                let value = self.stack.pop()?;
                self.state.tstore(&self.call_ctx.address, &key, value);
            }
            0x5E => {
                // MCOPY
                let dest = self.stack.pop()?.as_usize();
                let src = self.stack.pop()?.as_usize();
                let size = self.stack.pop()?.as_usize();

                // Memory expansion
                let max_offset = dest.max(src) + size;
                let mem_cost = memory_expansion_cost(self.memory.msize(), max_offset);
                self.consume_gas(mem_cost)?;

                // Copy cost
                let words = size.div_ceil(32);
                self.consume_gas(3 * words as u64)?;

                self.memory.copy(dest, src, size)?;
            }
            0x5F => {
                // PUSH0
                self.stack.push(U256::ZERO)?;
            }

            // 0x60-0x7F: PUSH1-PUSH32
            0x60..=0x7F => {
                let n = (opcode - 0x5F) as usize;
                let data = self.read_push_data(n)?;
                let mut bytes = [0u8; 32];
                bytes[32 - n..].copy_from_slice(&data);
                let value = U256::from_be_bytes(bytes);
                self.stack.push(value)?;
                self.pc += n; // Skip push data
            }

            // 0x80-0x8F: DUP1-DUP16
            0x80..=0x8F => {
                let n = (opcode - 0x7F) as usize;
                self.stack.dup(n)?;
            }

            // 0x90-0x9F: SWAP1-SWAP16
            0x90..=0x9F => {
                let n = (opcode - 0x8F) as usize;
                self.stack.swap(n)?;
            }

            // 0xA0-0xA4: LOG0-LOG4
            0xA0..=0xA4 => {
                let num_topics = (opcode - 0xA0) as usize;
                let offset = self.stack.pop()?.as_usize();
                let size = self.stack.pop()?.as_usize();

                // Pop topics
                for _ in 0..num_topics {
                    self.stack.pop()?;
                }

                // Memory expansion
                let mem_cost = memory_expansion_cost(self.memory.msize(), offset + size);
                self.consume_gas(mem_cost)?;

                // Log cost
                self.consume_gas(375 * (num_topics as u64) + 8 * (size as u64))?;
            }

            // 0xF0: CREATE
            0xF0 => {
                let value = self.stack.pop()?;
                let offset = self.stack.pop()?.as_usize();
                let size = self.stack.pop()?.as_usize();

                let mem_cost = memory_expansion_cost(self.memory.msize(), offset + size);
                self.consume_gas(mem_cost)?;

                let init_code = self.read_memory_bytes(offset, size)?;
                let init_code_cost = init_code_gas_cost(size);
                self.consume_gas(init_code_cost)?;

                let max_gas = self.gas_remaining - (self.gas_remaining / 64);
                let msg = CreateMessage {
                    gas: max_gas,
                    caller: self.call_ctx.address,
                    value,
                    init_code,
                    salt: None,
                };
                let result = self.host.create(&mut self.state, msg);
                if result.gas_used > max_gas {
                    return Err(EvmError::OutOfGas);
                }
                self.consume_gas(result.gas_used)?;
                self.return_data = result.return_data.clone();

                if result.success {
                    let address = result.address.unwrap_or(Address::ZERO);
                    self.stack.push(address_to_u256(&address))?;
                } else {
                    self.stack.push(U256::ZERO)?;
                }
            }

            // 0xF1: CALL
            0xF1 => {
                let gas_requested = self.stack.pop()?.as_u64();
                let to_u256 = self.stack.pop()?;
                let to = u256_to_address(&to_u256);
                let value = self.stack.pop()?;
                let in_offset = self.stack.pop()?.as_usize();
                let in_size = self.stack.pop()?.as_usize();
                let out_offset = self.stack.pop()?.as_usize();
                let out_size = self.stack.pop()?.as_usize();

                let max_offset = in_offset
                    .saturating_add(in_size)
                    .max(out_offset.saturating_add(out_size));
                let mem_cost = memory_expansion_cost(self.memory.msize(), max_offset);
                self.consume_gas(mem_cost)?;

                let input = self.read_memory_bytes(in_offset, in_size)?;

                let is_value_transfer = !value.is_zero();
                if is_value_transfer {
                    self.consume_gas(GAS_CALL_VALUE_TRANSFER)?;
                    if !self.state.account_exists(&to) {
                        self.consume_gas(GAS_CALL_NEW_ACCOUNT)?;
                    }
                }

                let mut gas_to_forward = gas_requested;
                let max_forward = self.gas_remaining - (self.gas_remaining / 64);
                if gas_to_forward > max_forward {
                    gas_to_forward = max_forward;
                }
                if is_value_transfer {
                    gas_to_forward = gas_to_forward.saturating_add(GAS_CALL_STIPEND);
                }

                let msg = CallMessage {
                    kind: CallKind::Call,
                    gas: gas_to_forward,
                    address: to,
                    caller: self.call_ctx.address,
                    value,
                    code_address: to,
                    input,
                    is_static: false,
                };
                let result = self.host.call(&mut self.state, msg);
                if result.gas_used > gas_to_forward {
                    return Err(EvmError::OutOfGas);
                }
                self.consume_gas(result.gas_used)?;

                self.return_data = result.return_data.clone();
                self.write_memory_bytes(out_offset, &result.return_data, out_size)?;
                self.stack.push(if result.success { U256::ONE } else { U256::ZERO })?;
            }

            // 0xF2: CALLCODE
            0xF2 => {
                let gas_requested = self.stack.pop()?.as_u64();
                let to_u256 = self.stack.pop()?;
                let to = u256_to_address(&to_u256);
                let value = self.stack.pop()?;
                let in_offset = self.stack.pop()?.as_usize();
                let in_size = self.stack.pop()?.as_usize();
                let out_offset = self.stack.pop()?.as_usize();
                let out_size = self.stack.pop()?.as_usize();

                let max_offset = in_offset
                    .saturating_add(in_size)
                    .max(out_offset.saturating_add(out_size));
                let mem_cost = memory_expansion_cost(self.memory.msize(), max_offset);
                self.consume_gas(mem_cost)?;

                let input = self.read_memory_bytes(in_offset, in_size)?;

                let is_value_transfer = !value.is_zero();
                if is_value_transfer {
                    self.consume_gas(GAS_CALL_VALUE_TRANSFER)?;
                }

                let mut gas_to_forward = gas_requested;
                let max_forward = self.gas_remaining - (self.gas_remaining / 64);
                if gas_to_forward > max_forward {
                    gas_to_forward = max_forward;
                }
                if is_value_transfer {
                    gas_to_forward = gas_to_forward.saturating_add(GAS_CALL_STIPEND);
                }

                let msg = CallMessage {
                    kind: CallKind::CallCode,
                    gas: gas_to_forward,
                    address: self.call_ctx.address,
                    caller: self.call_ctx.address,
                    value,
                    code_address: to,
                    input,
                    is_static: false,
                };
                let result = self.host.call(&mut self.state, msg);
                if result.gas_used > gas_to_forward {
                    return Err(EvmError::OutOfGas);
                }
                self.consume_gas(result.gas_used)?;

                self.return_data = result.return_data.clone();
                self.write_memory_bytes(out_offset, &result.return_data, out_size)?;
                self.stack.push(if result.success { U256::ONE } else { U256::ZERO })?;
            }

            // 0xF3: RETURN
            0xF3 => {
                let offset = self.stack.pop()?.as_usize();
                let size = self.stack.pop()?.as_usize();

                // Memory expansion
                let mem_cost = memory_expansion_cost(self.memory.msize(), offset + size);
                self.consume_gas(mem_cost)?;

                self.return_data = self.read_memory_bytes(offset, size)?;

                self.stopped = true;
            }

            // 0xF4: DELEGATECALL
            0xF4 => {
                let gas_requested = self.stack.pop()?.as_u64();
                let to_u256 = self.stack.pop()?;
                let to = u256_to_address(&to_u256);
                let in_offset = self.stack.pop()?.as_usize();
                let in_size = self.stack.pop()?.as_usize();
                let out_offset = self.stack.pop()?.as_usize();
                let out_size = self.stack.pop()?.as_usize();

                let max_offset = in_offset
                    .saturating_add(in_size)
                    .max(out_offset.saturating_add(out_size));
                let mem_cost = memory_expansion_cost(self.memory.msize(), max_offset);
                self.consume_gas(mem_cost)?;

                let input = self.read_memory_bytes(in_offset, in_size)?;

                let mut gas_to_forward = gas_requested;
                let max_forward = self.gas_remaining - (self.gas_remaining / 64);
                if gas_to_forward > max_forward {
                    gas_to_forward = max_forward;
                }

                let msg = CallMessage {
                    kind: CallKind::DelegateCall,
                    gas: gas_to_forward,
                    address: self.call_ctx.address,
                    caller: self.call_ctx.caller,
                    value: self.call_ctx.call_value,
                    code_address: to,
                    input,
                    is_static: false,
                };
                let result = self.host.call(&mut self.state, msg);
                if result.gas_used > gas_to_forward {
                    return Err(EvmError::OutOfGas);
                }
                self.consume_gas(result.gas_used)?;

                self.return_data = result.return_data.clone();
                self.write_memory_bytes(out_offset, &result.return_data, out_size)?;
                self.stack.push(if result.success { U256::ONE } else { U256::ZERO })?;
            }

            // 0xF5: CREATE2
            0xF5 => {
                let value = self.stack.pop()?;
                let offset = self.stack.pop()?.as_usize();
                let size = self.stack.pop()?.as_usize();
                let salt = self.stack.pop()?;

                let mem_cost = memory_expansion_cost(self.memory.msize(), offset + size);
                self.consume_gas(mem_cost)?;

                let init_code = self.read_memory_bytes(offset, size)?;
                let init_code_cost = init_code_gas_cost(size);
                let hash_cost = create2_hash_cost(size);
                self.consume_gas(init_code_cost + hash_cost)?;

                let max_gas = self.gas_remaining - (self.gas_remaining / 64);
                let msg = CreateMessage {
                    gas: max_gas,
                    caller: self.call_ctx.address,
                    value,
                    init_code,
                    salt: Some(salt),
                };
                let result = self.host.create(&mut self.state, msg);
                if result.gas_used > max_gas {
                    return Err(EvmError::OutOfGas);
                }
                self.consume_gas(result.gas_used)?;
                self.return_data = result.return_data.clone();

                if result.success {
                    let address = result.address.unwrap_or(Address::ZERO);
                    self.stack.push(address_to_u256(&address))?;
                } else {
                    self.stack.push(U256::ZERO)?;
                }
            }

            // 0xFA: STATICCALL
            0xFA => {
                let gas_requested = self.stack.pop()?.as_u64();
                let to_u256 = self.stack.pop()?;
                let to = u256_to_address(&to_u256);
                let in_offset = self.stack.pop()?.as_usize();
                let in_size = self.stack.pop()?.as_usize();
                let out_offset = self.stack.pop()?.as_usize();
                let out_size = self.stack.pop()?.as_usize();

                let max_offset = in_offset
                    .saturating_add(in_size)
                    .max(out_offset.saturating_add(out_size));
                let mem_cost = memory_expansion_cost(self.memory.msize(), max_offset);
                self.consume_gas(mem_cost)?;

                let input = self.read_memory_bytes(in_offset, in_size)?;

                let mut gas_to_forward = gas_requested;
                let max_forward = self.gas_remaining - (self.gas_remaining / 64);
                if gas_to_forward > max_forward {
                    gas_to_forward = max_forward;
                }

                let msg = CallMessage {
                    kind: CallKind::StaticCall,
                    gas: gas_to_forward,
                    address: to,
                    caller: self.call_ctx.address,
                    value: U256::ZERO,
                    code_address: to,
                    input,
                    is_static: true,
                };
                let result = self.host.call(&mut self.state, msg);
                if result.gas_used > gas_to_forward {
                    return Err(EvmError::OutOfGas);
                }
                self.consume_gas(result.gas_used)?;

                self.return_data = result.return_data.clone();
                self.write_memory_bytes(out_offset, &result.return_data, out_size)?;
                self.stack.push(if result.success { U256::ONE } else { U256::ZERO })?;
            }

            // 0xFD: REVERT
            0xFD => {
                let offset = self.stack.pop()?.as_usize();
                let size = self.stack.pop()?.as_usize();

                // Memory expansion
                let mem_cost = memory_expansion_cost(self.memory.msize(), offset + size);
                self.consume_gas(mem_cost)?;

                let revert_data = self.read_memory_bytes(offset, size)?;

                return Err(EvmError::Revert(revert_data));
            }

            // 0xFE: INVALID
            0xFE => {
                return Err(EvmError::InvalidOpcode(opcode));
            }

            // 0xFF: SELFDESTRUCT
            0xFF => {
                // SELFDESTRUCT: mark account for deletion and transfer balance
                let beneficiary_u256 = self.stack.pop()?;
                let beneficiary = u256_to_address(&beneficiary_u256);
                self.state.selfdestruct(&self.call_ctx.address, &beneficiary);
                self.stopped = true;
            }

            // Invalid opcodes
            _ => {
                return Err(EvmError::InvalidOpcode(opcode));
            }
        }

        // Increment PC for next instruction
        self.pc += 1;
        Ok(())
    }

    /// Run execution until stopped
    pub fn run(&mut self) -> Result<ExecutionResult, EvmError> {
        let initial_gas = self.gas_remaining;

        while !self.stopped && self.pc < self.code.len() {
            self.step()?;
        }

        Ok(ExecutionResult {
            success: true,
            gas_used: initial_gas - self.gas_remaining,
            return_data: self.return_data.clone(),
            stack: self.stack.clone(),
            memory: self.memory.clone(),
        })
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Execute bytecode with a gas limit and state
///
/// This is the main entry point for executing EVM bytecode.
///
/// # Arguments
///
/// * `code` - The bytecode to execute
/// * `gas_limit` - Maximum gas allowed for execution
/// * `state` - State interface for account/storage access
///
/// # Returns
///
/// Returns an `ExecutionResult` on success, or an `EvmError` if execution fails.
///
/// # Examples
///
/// ```
/// use claudeth::evm::interpreter::execute_bytecode;
/// use claudeth::state::InMemoryState;
///
/// // PUSH1 0x42 PUSH1 0x00 MSTORE STOP
/// let code = vec![0x60, 0x42, 0x60, 0x00, 0x52, 0x00];
/// let state = InMemoryState::new();
/// let result = execute_bytecode(&code, 100000, state).unwrap();
/// assert!(result.success);
/// ```
pub fn execute_bytecode<S: State>(
    code: &[u8],
    gas_limit: u64,
    state: S,
) -> Result<ExecutionResult, EvmError> {
    execute_bytecode_with_host(code, gas_limit, state, NullHost)
}

/// Execute bytecode with a custom host implementation.
pub fn execute_bytecode_with_host<S: State, H: Host<S>>(
    code: &[u8],
    gas_limit: u64,
    state: S,
    host: H,
) -> Result<ExecutionResult, EvmError> {
    let mut evm = Evm::new(code.to_vec(), gas_limit, state, host);
    evm.run()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evm::host::{CallResult, CreateResult};
    use crate::state::InMemoryState;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Debug, Default)]
    struct TestHost {
        last_call: Option<CallMessage>,
        last_create: Option<CreateMessage>,
        call_result: CallResult,
        create_result: CreateResult,
        blockhash: Option<Hash>,
        blobhash: Option<Hash>,
        blobbasefee: U256,
    }

    impl Host<InMemoryState> for Rc<RefCell<TestHost>> {
        fn call(&mut self, _state: &mut InMemoryState, msg: CallMessage) -> CallResult {
            let mut inner = self.borrow_mut();
            inner.last_call = Some(msg);
            inner.call_result.clone()
        }

        fn create(&mut self, _state: &mut InMemoryState, msg: CreateMessage) -> CreateResult {
            let mut inner = self.borrow_mut();
            inner.last_create = Some(msg);
            inner.create_result.clone()
        }

        fn blockhash(&self, number: &U256) -> Option<Hash> {
            let inner = self.borrow();
            inner
                .blockhash
                .filter(|_| *number != U256::ZERO)
        }

        fn blobhash(&self, index: &U256) -> Option<Hash> {
            let inner = self.borrow();
            inner
                .blobhash
                .filter(|_| *index != U256::ZERO)
        }

        fn blobbasefee(&self) -> U256 {
            self.borrow().blobbasefee
        }
    }

    // =============================================================================
    // Basic Execution Tests
    // =============================================================================

    #[test]
    fn test_empty_code() {
        let result = execute_bytecode(&[], 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.gas_used, 0);
    }

    #[test]
    fn test_stop() {
        let code = vec![0x00]; // STOP
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.gas_used, 0);
    }

    // =============================================================================
    // Arithmetic Tests
    // =============================================================================

    #[test]
    fn test_add() {
        // PUSH1 0x02 PUSH1 0x03 ADD STOP
        let code = vec![0x60, 0x02, 0x60, 0x03, 0x01, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(5));
    }

    #[test]
    fn test_mul() {
        // PUSH1 0x03 PUSH1 0x04 MUL STOP
        let code = vec![0x60, 0x03, 0x60, 0x04, 0x02, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(12));
    }

    #[test]
    fn test_sub() {
        // PUSH1 0x05 PUSH1 0x03 SUB STOP (5 - 3 = 2)
        let code = vec![0x60, 0x05, 0x60, 0x03, 0x03, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(2));
    }

    #[test]
    fn test_div() {
        // PUSH1 0x08 PUSH1 0x02 DIV STOP (8 / 2 = 4)
        let code = vec![0x60, 0x08, 0x60, 0x02, 0x04, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(4));
    }

    // =============================================================================
    // Stack Operations Tests
    // =============================================================================

    #[test]
    fn test_push_pop() {
        // PUSH1 0x42 POP STOP
        let code = vec![0x60, 0x42, 0x50, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.len(), 0);
    }

    #[test]
    fn test_dup1() {
        // PUSH1 0x42 DUP1 STOP
        let code = vec![0x60, 0x42, 0x80, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.len(), 2);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x42));
        assert_eq!(result.stack.peek(1).unwrap(), &U256::from_u64(0x42));
    }

    #[test]
    fn test_swap1() {
        // PUSH1 0x01 PUSH1 0x02 SWAP1 STOP
        let code = vec![0x60, 0x01, 0x60, 0x02, 0x90, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x01));
        assert_eq!(result.stack.peek(1).unwrap(), &U256::from_u64(0x02));
    }

    // =============================================================================
    // Memory Tests
    // =============================================================================

    #[test]
    fn test_mstore_mload() {
        // PUSH1 0x42 PUSH1 0x00 MSTORE PUSH1 0x00 MLOAD STOP
        let code = vec![0x60, 0x42, 0x60, 0x00, 0x52, 0x60, 0x00, 0x51, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x42));
    }

    #[test]
    fn test_mstore8() {
        // PUSH1 0xFF PUSH1 0x00 MSTORE8 PUSH1 0x00 MLOAD STOP
        let code = vec![0x60, 0xFF, 0x60, 0x00, 0x53, 0x60, 0x00, 0x51, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        let value = result.stack.peek(0).unwrap();
        let bytes = value.to_be_bytes();
        assert_eq!(bytes[0], 0xFF);
    }

    // =============================================================================
    // Control Flow Tests
    // =============================================================================

    #[test]
    fn test_jump() {
        // PUSH1 0x05 JUMP INVALID JUMPDEST PUSH1 0x42 STOP
        let code = vec![0x60, 0x05, 0x56, 0xFE, 0xFE, 0x5B, 0x60, 0x42, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x42));
    }

    #[test]
    fn test_jumpi_taken() {
        // PUSH1 0x01 PUSH1 0x06 JUMPI INVALID JUMPDEST PUSH1 0x42 STOP
        let code = vec![0x60, 0x01, 0x60, 0x06, 0x57, 0xFE, 0x5B, 0x60, 0x42, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x42));
    }

    #[test]
    fn test_jumpi_not_taken() {
        // PUSH1 0x00 PUSH1 0x06 JUMPI PUSH1 0x99 STOP
        let code = vec![0x60, 0x00, 0x60, 0x06, 0x57, 0x60, 0x99, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x99));
    }

    #[test]
    fn test_invalid_jump() {
        // PUSH1 0x03 JUMP STOP
        let code = vec![0x60, 0x03, 0x56, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new());
        assert!(matches!(result, Err(EvmError::InvalidJump)));
    }

    // =============================================================================
    // Gas Tests
    // =============================================================================

    #[test]
    fn test_out_of_gas() {
        // PUSH1 0x01 ADD (requires at least 6 gas: 3+3)
        let code = vec![0x60, 0x01, 0x01];
        let result = execute_bytecode(&code, 5, InMemoryState::new());
        assert!(matches!(result, Err(EvmError::OutOfGas)));
    }

    #[test]
    fn test_gas_tracking() {
        // PUSH1 0x01 STOP (costs 3 + 0 = 3 gas)
        let code = vec![0x60, 0x01, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.gas_used, 3);
    }

    // =============================================================================
    // Return/Revert Tests
    // =============================================================================

    #[test]
    fn test_return_empty() {
        // PUSH1 0x00 PUSH1 0x00 RETURN
        let code = vec![0x60, 0x00, 0x60, 0x00, 0xF3];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.return_data.len(), 0);
    }

    #[test]
    fn test_revert() {
        // PUSH1 0x00 PUSH1 0x00 REVERT
        let code = vec![0x60, 0x00, 0x60, 0x00, 0xFD];
        let result = execute_bytecode(&code, 1000, InMemoryState::new());
        assert!(matches!(result, Err(EvmError::Revert(_))));
    }

    // =============================================================================
    // Invalid Opcode Tests
    // =============================================================================

    #[test]
    fn test_invalid_opcode() {
        let code = vec![0xFE]; // INVALID
        let result = execute_bytecode(&code, 1000, InMemoryState::new());
        assert!(matches!(result, Err(EvmError::InvalidOpcode(0xFE))));
    }

    #[test]
    fn test_undefined_opcode() {
        let code = vec![0x0C]; // Undefined
        let result = execute_bytecode(&code, 1000, InMemoryState::new());
        assert!(matches!(result, Err(EvmError::InvalidOpcode(0x0C))));
    }

    // =============================================================================
    // Complex Integration Tests
    // =============================================================================

    #[test]
    fn test_fibonacci_like() {
        // Calculate 1+1, then add result to itself twice
        // PUSH1 0x01 PUSH1 0x01 ADD DUP1 ADD DUP1 ADD STOP
        let code = vec![
            0x60, 0x01, // PUSH1 1
            0x60, 0x01, // PUSH1 1
            0x01,       // ADD (result: 2)
            0x80,       // DUP1 (stack: 2, 2)
            0x01,       // ADD (result: 4)
            0x80,       // DUP1 (stack: 4, 4)
            0x01,       // ADD (result: 8)
            0x00,       // STOP
        ];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(8));
    }

    #[test]
    fn test_memory_expansion_cost() {
        // Store at high memory offset
        // PUSH1 0x42 PUSH1 0xFF MSTORE STOP
        let code = vec![0x60, 0x42, 0x60, 0xFF, 0x52, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert!(result.gas_used > 6); // Base gas + memory expansion
    }

    #[test]
    fn test_codecopy() {
        // Copy 3 bytes of code to memory at offset 0
        // PUSH1 0x03 PUSH1 0x00 PUSH1 0x00 CODECOPY PUSH1 0x00 MLOAD STOP
        let code = vec![0x60, 0x03, 0x60, 0x00, 0x60, 0x00, 0x39, 0x60, 0x00, 0x51, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        // Memory should contain first 3 bytes of code (0x60, 0x03, 0x60)
        let value = result.stack.peek(0).unwrap();
        let bytes = value.to_be_bytes();
        assert_eq!(bytes[0], 0x60);
        assert_eq!(bytes[1], 0x03);
        assert_eq!(bytes[2], 0x60);
    }

    // =============================================================================
    // Environment Opcode Tests
    // =============================================================================

    #[test]
    fn test_address_caller_callvalue() {
        // ADDRESS CALLER CALLVALUE STOP
        let code = vec![0x30, 0x33, 0x34, 0x00];
        let mut evm = Evm::new(code, 1000, InMemoryState::new(), NullHost);
        let address = Address::new([0x11; 20]);
        let caller = Address::new([0x22; 20]);
        let call_value = U256::from_u64(0x1234);
        evm.call_ctx = CallContext {
            address,
            caller,
            call_value,
            call_data: Vec::new(),
        };

        let result = evm.run().unwrap();
        assert!(result.success);
        assert_eq!(
            result.stack.peek(2).unwrap(),
            &address_to_u256(&address)
        );
        assert_eq!(
            result.stack.peek(1).unwrap(),
            &address_to_u256(&caller)
        );
        assert_eq!(result.stack.peek(0).unwrap(), &call_value);
    }

    #[test]
    fn test_calldata_load_and_size() {
        // PUSH1 0x00 CALLDATALOAD CALLDATASIZE STOP
        let code = vec![0x60, 0x00, 0x35, 0x36, 0x00];
        let mut evm = Evm::new(code, 1000, InMemoryState::new(), NullHost);
        evm.call_ctx.call_data = vec![0xAA, 0xBB, 0xCC];

        let result = evm.run().unwrap();
        assert!(result.success);
        let value = result.stack.peek(1).unwrap();
        let bytes = value.to_be_bytes();
        assert_eq!(bytes[0], 0xAA);
        assert_eq!(bytes[1], 0xBB);
        assert_eq!(bytes[2], 0xCC);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(3));
    }

    #[test]
    fn test_calldata_load_out_of_range() {
        // PUSH1 0x05 CALLDATALOAD STOP
        let code = vec![0x60, 0x05, 0x35, 0x00];
        let mut evm = Evm::new(code, 1000, InMemoryState::new(), NullHost);
        evm.call_ctx.call_data = vec![0xAA, 0xBB, 0xCC];

        let result = evm.run().unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_calldatacopy_zero_pad() {
        // Copy 6 bytes of calldata starting at offset 2 into memory at 0.
        // PUSH1 0x06 PUSH1 0x02 PUSH1 0x00 CALLDATACOPY PUSH1 0x00 MLOAD STOP
        let code = vec![
            0x60, 0x06, 0x60, 0x02, 0x60, 0x00, 0x37, 0x60, 0x00, 0x51, 0x00,
        ];
        let mut evm = Evm::new(code, 1000, InMemoryState::new(), NullHost);
        evm.call_ctx.call_data = vec![0xDE, 0xAD, 0xBE, 0xEF];

        let result = evm.run().unwrap();
        assert!(result.success);
        let value = result.stack.peek(0).unwrap();
        let bytes = value.to_be_bytes();
        assert_eq!(bytes[0], 0xBE);
        assert_eq!(bytes[1], 0xEF);
        assert_eq!(bytes[2], 0x00);
        assert_eq!(bytes[3], 0x00);
        assert_eq!(bytes[4], 0x00);
        assert_eq!(bytes[5], 0x00);
    }

    #[test]
    fn test_returndata_size_and_copy() {
        // Copy 4 bytes of returndata into memory at 0, then load and check size.
        // PUSH1 0x04 PUSH1 0x00 PUSH1 0x00 RETURNDATACOPY PUSH1 0x00 MLOAD RETURNDATASIZE STOP
        let code = vec![
            0x60, 0x04, 0x60, 0x00, 0x60, 0x00, 0x3E, 0x60, 0x00, 0x51, 0x3D, 0x00,
        ];
        let mut evm = Evm::new(code, 1000, InMemoryState::new(), NullHost);
        evm.return_data = vec![0x01, 0x02, 0x03, 0x04];

        let result = evm.run().unwrap();
        assert!(result.success);
        let value = result.stack.peek(1).unwrap();
        let bytes = value.to_be_bytes();
        assert_eq!(bytes[0], 0x01);
        assert_eq!(bytes[1], 0x02);
        assert_eq!(bytes[2], 0x03);
        assert_eq!(bytes[3], 0x04);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(4));
    }

    #[test]
    fn test_returndatacopy_out_of_bounds() {
        // PUSH1 0x04 PUSH1 0x02 PUSH1 0x00 RETURNDATACOPY
        let code = vec![0x60, 0x04, 0x60, 0x02, 0x60, 0x00, 0x3E];
        let mut evm = Evm::new(code, 1000, InMemoryState::new(), NullHost);
        evm.return_data = vec![0x01, 0x02, 0x03, 0x04];

        let result = evm.run();
        assert!(matches!(
            result,
            Err(EvmError::MemoryError(MemoryError::InvalidOffset))
        ));
    }

    #[test]
    fn test_pc_opcode() {
        // PC STOP
        let code = vec![0x58, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_gas_opcode() {
        // GAS STOP
        let code = vec![0x5A, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        // Gas should be less than initial (2 gas consumed)
        let remaining = result.stack.peek(0).unwrap();
        assert_eq!(*remaining, U256::from_u64(998));
    }

    #[test]
    fn test_msize() {
        // PUSH1 0x42 PUSH1 0x40 MSTORE MSIZE STOP
        let code = vec![0x60, 0x42, 0x60, 0x40, 0x52, 0x59, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        // Memory should be expanded to cover offset 0x40 + 32 = 0x60 (96 bytes)
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(96));
    }

    #[test]
    fn test_comparison_operations() {
        // PUSH1 0x03 PUSH1 0x05 LT STOP
        // Stack after PUSHes: [3, 5] (top is 5)
        // LT pops a=5, then b=3, compares b < a: 3 < 5 = true = 1
        let code = vec![0x60, 0x03, 0x60, 0x05, 0x10, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ONE);
    }

    #[test]
    fn test_bitwise_operations() {
        // PUSH1 0x0F PUSH1 0xF0 AND STOP (0xF0 & 0x0F = 0)
        let code = vec![0x60, 0x0F, 0x60, 0xF0, 0x16, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_push0() {
        // PUSH0 STOP
        let code = vec![0x5F, 0x00];
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_push32() {
        // PUSH32 0xFF..FF STOP
        let mut code = vec![0x7F];
        code.extend_from_slice(&[0xFF; 32]);
        code.push(0x00);
        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::MAX);
    }

    #[test]
    fn test_dup16() {
        // Push 16 values, then DUP16
        let mut code = Vec::new();
        for i in 1..=16 {
            code.push(0x60); // PUSH1
            code.push(i);
        }
        code.push(0x8F); // DUP16
        code.push(0x00); // STOP

        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.len(), 17);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ONE); // Should duplicate first value
    }

    #[test]
    fn test_swap16() {
        // Push 17 values, then SWAP16
        let mut code = Vec::new();
        for i in 1..=17 {
            code.push(0x60); // PUSH1
            code.push(i);
        }
        code.push(0x9F); // SWAP16
        code.push(0x00); // STOP

        let result = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ONE); // Top should be swapped with 16th
    }

    #[test]
    fn test_stack_overflow() {
        // Try to push 1025 items (exceeds 1024 limit)
        let mut code = Vec::new();
        for _ in 0..1025 {
            code.push(0x60); // PUSH1
            code.push(0x01);
        }
        code.push(0x00); // STOP

        let result = execute_bytecode(&code, 1000000, InMemoryState::new());
        assert!(matches!(result, Err(EvmError::StackError(StackError::Overflow))));
    }

    #[test]
    fn test_stack_underflow() {
        // Try to POP from empty stack
        let code = vec![0x50]; // POP
        let result = execute_bytecode(&code, 1000, InMemoryState::new());
        assert!(matches!(result, Err(EvmError::StackError(StackError::Underflow))));
    }

    // =============================================================================
    // Real Contract Bytecode Tests
    // =============================================================================

    #[test]
    fn test_simple_storage_contract() {
        // Simplified storage contract pattern:
        // Store a value to memory, then return it
        // PUSH1 0x42 PUSH1 0x00 MSTORE PUSH1 0x20 PUSH1 0x00 RETURN
        let code = vec![
            0x60, 0x42,       // PUSH1 0x42
            0x60, 0x00,       // PUSH1 0x00
            0x52,             // MSTORE
            0x60, 0x20,       // PUSH1 0x20 (size)
            0x60, 0x00,       // PUSH1 0x00 (offset)
            0xF3,             // RETURN
        ];
        let result = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.return_data.len(), 32);
        assert_eq!(result.return_data[31], 0x42);
    }

    #[test]
    fn test_conditional_logic() {
        // If-else using JUMPI: if x > 5 then push 1 else push 0
        // PUSH1 0x06 PUSH1 0x05 GT PUSH1 0x0D JUMPI PUSH1 0x00 PUSH1 0x11 JUMP JUMPDEST PUSH1 0x01 JUMPDEST STOP
        let code = vec![
            0x60, 0x06,       // PUSH1 6
            0x60, 0x05,       // PUSH1 5
            0x11,             // GT (6 > 5 = true)
            0x60, 0x0D,       // PUSH1 13 (jump target)
            0x57,             // JUMPI
            0x60, 0x00,       // PUSH1 0 (else branch)
            0x60, 0x11,       // PUSH1 17 (skip to end)
            0x56,             // JUMP
            0x5B,             // JUMPDEST (offset 13)
            0x60, 0x01,       // PUSH1 1 (then branch)
            0x5B,             // JUMPDEST (offset 17)
            0x00,             // STOP
        ];
        let result = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ONE);
    }

    #[test]
    fn test_loop_simulation() {
        // Simple counter loop: push 3 values and add them
        // PUSH1 1 PUSH1 2 PUSH1 3 ADD ADD STOP
        let code = vec![
            0x60, 0x01,       // PUSH1 1
            0x60, 0x02,       // PUSH1 2
            0x60, 0x03,       // PUSH1 3
            0x01,             // ADD (2+3=5)
            0x01,             // ADD (1+5=6)
            0x00,             // STOP
        ];
        let result = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(6));
    }

    #[test]
    fn test_mcopy_operation() {
        // MCOPY: Copy memory region
        // Store 0x42 at offset 0, then copy to offset 32
        // PUSH1 0x42 PUSH1 0x00 MSTORE PUSH1 0x20 PUSH1 0x00 PUSH1 0x20 MCOPY PUSH1 0x20 MLOAD STOP
        let code = vec![
            0x60, 0x42,       // PUSH1 0x42
            0x60, 0x00,       // PUSH1 0x00
            0x52,             // MSTORE
            0x60, 0x20,       // PUSH1 32 (size)
            0x60, 0x00,       // PUSH1 0 (src)
            0x60, 0x20,       // PUSH1 32 (dest)
            0x5E,             // MCOPY
            0x60, 0x20,       // PUSH1 32
            0x51,             // MLOAD
            0x00,             // STOP
        ];
        let result = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x42));
    }

    #[test]
    fn test_all_push_sizes() {
        // Test PUSH1, PUSH2, PUSH4, etc.
        // PUSH2 0x1234 PUSH1 0x56 ADD STOP
        let code = vec![
            0x61, 0x12, 0x34, // PUSH2 0x1234
            0x60, 0x56,       // PUSH1 0x56
            0x01,             // ADD
            0x00,             // STOP
        ];
        let result = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x1234 + 0x56));
    }

    #[test]
    fn test_complete_opcode_coverage() {
        // Test coverage of major opcode categories
        let mut code = Vec::new();

        // Arithmetic
        code.extend_from_slice(&[0x60, 0x0A]); // PUSH1 10
        code.extend_from_slice(&[0x60, 0x05]); // PUSH1 5
        code.push(0x01); // ADD
        code.push(0x60); code.push(0x02); // PUSH1 2
        code.push(0x02); // MUL
        code.push(0x60); code.push(0x03); // PUSH1 3
        code.push(0x04); // DIV

        // Bitwise
        code.push(0x60); code.push(0xFF); // PUSH1 0xFF
        code.push(0x16); // AND

        // Comparison
        code.push(0x60); code.push(0x00); // PUSH1 0
        code.push(0x14); // EQ

        // Stack manipulation
        code.push(0x80); // DUP1
        code.push(0x50); // POP

        code.push(0x00); // STOP

        let result = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
    }

    // =============================================================================
    // State Integration Tests
    // =============================================================================

    #[test]
    fn test_balance_opcode() {
        // PUSH20 <address> BALANCE STOP
        let mut code = Vec::new();
        code.push(0x73); // PUSH20
        let test_addr = Address::new([0x42; 20]);
        code.extend_from_slice(&test_addr.to_bytes());
        code.push(0x31); // BALANCE
        code.push(0x00); // STOP

        let mut state = InMemoryState::new();
        state.set_balance(&test_addr, U256::from_u64(12345));

        let result = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(12345));
    }

    #[test]
    fn test_balance_zero_account() {
        // PUSH20 <address> BALANCE STOP
        let mut code = Vec::new();
        code.push(0x73); // PUSH20
        let test_addr = Address::new([0x99; 20]);
        code.extend_from_slice(&test_addr.to_bytes());
        code.push(0x31); // BALANCE
        code.push(0x00); // STOP

        let state = InMemoryState::new(); // Empty state

        let result = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_extcodesize_opcode() {
        // PUSH20 <address> EXTCODESIZE STOP
        let mut code = Vec::new();
        code.push(0x73); // PUSH20
        let test_addr = Address::new([0x55; 20]);
        code.extend_from_slice(&test_addr.to_bytes());
        code.push(0x3B); // EXTCODESIZE
        code.push(0x00); // STOP

        let mut state = InMemoryState::new();
        state.set_code(&test_addr, vec![0x60, 0x42, 0x60, 0x00, 0x52]); // 5 bytes

        let result = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(5));
    }

    #[test]
    fn test_extcodecopy_opcode() {
        // PUSH20 <address> PUSH1 4 PUSH1 0 PUSH1 0 EXTCODECOPY PUSH1 0 MLOAD STOP
        let test_addr = Address::new([0xAB; 20]);
        let mut code = vec![
            0x60, 0x04, // PUSH1 4 (size)
            0x60, 0x00, // PUSH1 0 (offset in code)
            0x60, 0x00, // PUSH1 0 (dest in memory)
            0x73,       // PUSH20
        ];
        code.extend_from_slice(&test_addr.to_bytes());
        code.extend_from_slice(&[
            0x3C,       // EXTCODECOPY
            0x60, 0x00, // PUSH1 0
            0x51,       // MLOAD
            0x00,       // STOP
        ]);

        let mut state = InMemoryState::new();
        state.set_code(&test_addr, vec![0xDE, 0xAD, 0xBE, 0xEF, 0x99]);

        let result = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        let value = result.stack.peek(0).unwrap();
        let bytes = value.to_be_bytes();
        assert_eq!(bytes[0], 0xDE);
        assert_eq!(bytes[1], 0xAD);
        assert_eq!(bytes[2], 0xBE);
        assert_eq!(bytes[3], 0xEF);
    }

    #[test]
    fn test_extcodehash_opcode() {
        // PUSH20 <address> EXTCODEHASH STOP
        let mut code = Vec::new();
        code.push(0x73); // PUSH20
        let test_addr = Address::new([0x77; 20]);
        code.extend_from_slice(&test_addr.to_bytes());
        code.push(0x3F); // EXTCODEHASH
        code.push(0x00); // STOP

        let mut state = InMemoryState::new();
        let test_code = vec![0x60, 0x42];
        state.set_code(&test_addr, test_code.clone());

        let result = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        // Hash should not be zero (it's the keccak of the code)
        assert_ne!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_selfbalance_opcode() {
        // SELFBALANCE STOP
        let code = vec![0x47, 0x00]; // SELFBALANCE STOP

        let mut state = InMemoryState::new();
        let contract_addr = Address::new([0x11; 20]);
        state.set_balance(&contract_addr, U256::from_u64(999));

        let mut evm = Evm::new(code, 100000, state, NullHost);
        evm.call_ctx.address = contract_addr;

        let result = evm.run().unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(999));
    }

    #[test]
    fn test_sload_sstore_opcodes() {
        // PUSH1 0x99 PUSH1 0x42 SSTORE PUSH1 0x42 SLOAD STOP
        let code = vec![
            0x60, 0x99, // PUSH1 0x99 (value)
            0x60, 0x42, // PUSH1 0x42 (key)
            0x55,       // SSTORE
            0x60, 0x42, // PUSH1 0x42 (key)
            0x54,       // SLOAD
            0x00,       // STOP
        ];

        let state = InMemoryState::new();
        let result = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x99));
    }

    #[test]
    fn test_sload_uninitialized() {
        // PUSH1 0xFF SLOAD STOP
        let code = vec![0x60, 0xFF, 0x54, 0x00]; // PUSH1 0xFF SLOAD STOP

        let state = InMemoryState::new();
        let result = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_tload_tstore_opcodes() {
        // PUSH1 0x77 PUSH1 0x33 TSTORE PUSH1 0x33 TLOAD STOP
        let code = vec![
            0x60, 0x77, // PUSH1 0x77 (value)
            0x60, 0x33, // PUSH1 0x33 (key)
            0x5D,       // TSTORE
            0x60, 0x33, // PUSH1 0x33 (key)
            0x5C,       // TLOAD
            0x00,       // STOP
        ];

        let state = InMemoryState::new();
        let result = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x77));
    }

    #[test]
    fn test_tload_uninitialized() {
        // PUSH1 0xAA TLOAD STOP
        let code = vec![0x60, 0xAA, 0x5C, 0x00]; // PUSH1 0xAA TLOAD STOP

        let state = InMemoryState::new();
        let result = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_selfdestruct_opcode() {
        // PUSH20 <beneficiary> SELFDESTRUCT
        let mut code = Vec::new();
        code.push(0x73); // PUSH20
        let beneficiary = Address::new([0xBB; 20]);
        code.extend_from_slice(&beneficiary.to_bytes());
        code.push(0xFF); // SELFDESTRUCT

        let mut state = InMemoryState::new();
        let contract_addr = Address::new([0xCC; 20]);
        state.set_balance(&contract_addr, U256::from_u64(1000));

        let mut evm = Evm::new(code, 100000, state, NullHost);
        evm.call_ctx.address = contract_addr;

        let result = evm.run().unwrap();
        assert!(result.success);
        // Check that selfdestruct was recorded
        let selfdestructs = evm.state.get_selfdestructs();
        assert_eq!(selfdestructs.len(), 1);
        assert_eq!(selfdestructs[0].0, contract_addr);
        assert_eq!(selfdestructs[0].1, beneficiary);
    }

    #[test]
    fn test_sstore_multiple_keys() {
        // Store multiple values and load them back
        let code = vec![
            0x60, 0x11, // PUSH1 0x11
            0x60, 0x01, // PUSH1 0x01 (key)
            0x55,       // SSTORE
            0x60, 0x22, // PUSH1 0x22
            0x60, 0x02, // PUSH1 0x02 (key)
            0x55,       // SSTORE
            0x60, 0x01, // PUSH1 0x01 (key)
            0x54,       // SLOAD
            0x60, 0x02, // PUSH1 0x02 (key)
            0x54,       // SLOAD
            0x00,       // STOP
        ];

        let state = InMemoryState::new();
        let result = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x22));
        assert_eq!(result.stack.peek(1).unwrap(), &U256::from_u64(0x11));
    }

    #[test]
    fn test_transient_storage_isolated() {
        // Transient storage should not affect permanent storage
        let code = vec![
            0x60, 0xAA, // PUSH1 0xAA
            0x60, 0x10, // PUSH1 0x10 (key)
            0x5D,       // TSTORE (transient)
            0x60, 0xBB, // PUSH1 0xBB
            0x60, 0x10, // PUSH1 0x10 (key)
            0x55,       // SSTORE (permanent)
            0x60, 0x10, // PUSH1 0x10 (key)
            0x5C,       // TLOAD (transient)
            0x60, 0x10, // PUSH1 0x10 (key)
            0x54,       // SLOAD (permanent)
            0x00,       // STOP
        ];

        let state = InMemoryState::new();
        let result = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        // Permanent storage should have 0xBB
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0xBB));
        // Transient storage should have 0xAA
        assert_eq!(result.stack.peek(1).unwrap(), &U256::from_u64(0xAA));
    }

    #[test]
    fn test_blockhash_blobhash_blobbasefee_opcodes() {
        let mut host_state = TestHost::default();
        let block_hash = Hash::new([0x11; 32]);
        let blob_hash = Hash::new([0x22; 32]);
        host_state.blockhash = Some(block_hash);
        host_state.blobhash = Some(blob_hash);
        host_state.blobbasefee = U256::from_u64(7);

        let host = Rc::new(RefCell::new(host_state));
        let code = vec![
            0x60, 0x01, // PUSH1 0x01
            0x40,       // BLOCKHASH
            0x60, 0x02, // PUSH1 0x02
            0x49,       // BLOBHASH
            0x4A,       // BLOBBASEFEE
            0x00,       // STOP
        ];

        let result =
            execute_bytecode_with_host(&code, 100000, InMemoryState::new(), host).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(7));
        assert_eq!(result.stack.peek(1).unwrap(), &hash_to_u256(&blob_hash));
        assert_eq!(result.stack.peek(2).unwrap(), &hash_to_u256(&block_hash));
    }

    #[test]
    fn test_call_opcode_invokes_host_and_writes_output() {
        let mut host_state = TestHost::default();
        host_state.call_result = CallResult {
            success: true,
            return_data: vec![0xAA, 0xBB],
            gas_used: 5,
        };
        let host = Rc::new(RefCell::new(host_state));

        let mut to_bytes = [0u8; 20];
        to_bytes[19] = 0x01;
        let to = Address::from_slice(&to_bytes).unwrap();

        let mut code = vec![
            0x60, 0x01, // PUSH1 0x01
            0x60, 0x00, // PUSH1 0x00
            0x53,       // MSTORE8 (memory[0] = 0x01)
            0x60, 0x03, // PUSH1 0x03 (out_size)
            0x60, 0x20, // PUSH1 0x20 (out_offset)
            0x60, 0x01, // PUSH1 0x01 (in_size)
            0x60, 0x00, // PUSH1 0x00 (in_offset)
            0x60, 0x00, // PUSH1 0x00 (value)
            0x73,       // PUSH20
        ];
        code.extend_from_slice(&to.to_bytes());
        code.extend_from_slice(&[
            0x60, 0x10, // PUSH1 0x10 (gas)
            0xF1,       // CALL
            0x00,       // STOP
        ]);

        let result = execute_bytecode_with_host(&code, 100000, InMemoryState::new(), host.clone())
            .unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ONE);
        assert_eq!(result.return_data, vec![0xAA, 0xBB]);

        let mut memory = result.memory;
        let word = memory.mload(0x20).unwrap().to_be_bytes();
        assert_eq!(&word[0..3], &[0xAA, 0xBB, 0x00]);

        let recorded = host.borrow().last_call.clone().unwrap();
        assert_eq!(recorded.kind, CallKind::Call);
        assert_eq!(recorded.address, to);
        assert_eq!(recorded.caller, Address::ZERO);
        assert_eq!(recorded.value, U256::ZERO);
        assert_eq!(recorded.code_address, to);
        assert_eq!(recorded.input, vec![0x01]);
        assert_eq!(recorded.gas, 0x10);
    }

    #[test]
    fn test_create2_opcode_forwards_init_code_and_salt() {
        let mut host_state = TestHost::default();
        let created_addr = Address::new([0xAB; 20]);
        host_state.create_result = CreateResult {
            success: true,
            address: Some(created_addr),
            return_data: Vec::new(),
            gas_used: 7,
        };
        let host = Rc::new(RefCell::new(host_state));

        let code = vec![
            0x60, 0xAA, // PUSH1 0xAA
            0x60, 0x00, // PUSH1 0x00
            0x53,       // MSTORE8 (init code byte)
            0x61, 0x12, 0x34, // PUSH2 0x1234 (salt)
            0x60, 0x01, // PUSH1 0x01 (size)
            0x60, 0x00, // PUSH1 0x00 (offset)
            0x60, 0x00, // PUSH1 0x00 (value)
            0xF5,       // CREATE2
            0x00,       // STOP
        ];

        let result = execute_bytecode_with_host(&code, 100000, InMemoryState::new(), host.clone())
            .unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &address_to_u256(&created_addr));

        let recorded = host.borrow().last_create.clone().unwrap();
        assert_eq!(recorded.salt, Some(U256::from_u64(0x1234)));
        assert_eq!(recorded.init_code, vec![0xAA]);
    }

    #[test]
    fn test_delegatecall_uses_caller_and_value_from_context() {
        let mut host_state = TestHost::default();
        host_state.call_result = CallResult {
            success: true,
            return_data: Vec::new(),
            gas_used: 1,
        };
        let host = Rc::new(RefCell::new(host_state));

        let mut to_bytes = [0u8; 20];
        to_bytes[19] = 0x02;
        let to = Address::from_slice(&to_bytes).unwrap();

        let mut code = vec![
            0x60, 0x00, // PUSH1 0x00 (out_size)
            0x60, 0x00, // PUSH1 0x00 (out_offset)
            0x60, 0x00, // PUSH1 0x00 (in_size)
            0x60, 0x00, // PUSH1 0x00 (in_offset)
            0x73,       // PUSH20
        ];
        code.extend_from_slice(&to.to_bytes());
        code.extend_from_slice(&[
            0x60, 0x20, // PUSH1 0x20 (gas)
            0xF4,       // DELEGATECALL
            0x00,       // STOP
        ]);

        let mut evm = Evm::new(code, 100000, InMemoryState::new(), host.clone());
        evm.call_ctx.address = Address::new([0x11; 20]);
        evm.call_ctx.caller = Address::new([0x22; 20]);
        evm.call_ctx.call_value = U256::from_u64(77);

        let result = evm.run().unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ONE);

        let recorded = host.borrow().last_call.clone().unwrap();
        assert_eq!(recorded.kind, CallKind::DelegateCall);
        assert_eq!(recorded.address, Address::new([0x11; 20]));
        assert_eq!(recorded.caller, Address::new([0x22; 20]));
        assert_eq!(recorded.value, U256::from_u64(77));
        assert_eq!(recorded.code_address, to);
    }
}
