//! EVM Environment and Block Opcodes
//!
//! This module implements opcodes that interact with the execution environment,
//! including block information, transaction context, contract state, and system operations.

#![cfg_attr(target_arch = "riscv32", no_std)]

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::crypto::keccak::keccak256;
use crate::evm::error::EvmError;
use crate::evm::host::{CallKind, CallMessage, CreateMessage, Host};
use crate::evm::{GAS_CALL_NEW_ACCOUNT, GAS_CALL_STIPEND, GAS_CALL_VALUE_TRANSFER};
use crate::evm::{Memory, Stack};
use crate::state::State;
use crate::types::{Address, Hash, U256};

use super::control::OpcodeError;
use super::utils;

// =============================================================================
// Context Structures
// =============================================================================

/// Block context information
#[derive(Debug, Clone)]
pub struct BlockContext {
    /// Coinbase address (block miner)
    pub coinbase: Address,
    /// Block timestamp
    pub timestamp: U256,
    /// Block number
    pub number: U256,
    /// Block difficulty (or prevrandao post-merge)
    pub difficulty: U256,
    /// Block gas limit
    pub gas_limit: U256,
    /// Chain ID
    pub chain_id: U256,
    /// Base fee per gas (EIP-1559)
    pub base_fee: U256,
    /// Excess blob gas (EIP-4844)
    pub excess_blob_gas: Option<U256>,
}

/// Parameters for constructing a block context.
#[derive(Debug, Clone)]
pub struct BlockContextParams {
    pub coinbase: Address,
    pub timestamp: U256,
    pub number: U256,
    pub difficulty: U256,
    pub gas_limit: U256,
    pub chain_id: U256,
    pub base_fee: U256,
    pub excess_blob_gas: Option<U256>,
}

impl BlockContext {
    /// Create a new block context
    pub fn new(params: BlockContextParams) -> Self {
        Self {
            coinbase: params.coinbase,
            timestamp: params.timestamp,
            number: params.number,
            difficulty: params.difficulty,
            gas_limit: params.gas_limit,
            chain_id: params.chain_id,
            base_fee: params.base_fee,
            excess_blob_gas: params.excess_blob_gas,
        }
    }
}

/// Transaction context information
#[derive(Debug, Clone)]
pub struct TxContext {
    /// Transaction origin (msg.sender of the original transaction)
    pub origin: Address,
    /// Gas price
    pub gas_price: U256,
}

impl TxContext {
    /// Create a new transaction context
    pub fn new(origin: Address, gas_price: U256) -> Self {
        Self { origin, gas_price }
    }
}

/// Contract execution context
#[derive(Debug, Clone)]
pub struct ContractContext {
    /// Current contract address
    pub address: Address,
    /// Caller address
    pub caller: Address,
    /// Call value (msg.value)
    pub call_value: U256,
    /// Call data
    pub call_data: Vec<u8>,
    /// Code being executed
    pub code: Vec<u8>,
    /// Return data from last call
    pub return_data: Vec<u8>,
}

impl ContractContext {
    /// Create a new contract context
    pub fn new(
        address: Address,
        caller: Address,
        call_value: U256,
        call_data: Vec<u8>,
        code: Vec<u8>,
    ) -> Self {
        Self {
            address,
            caller,
            call_value,
            call_data,
            code,
            return_data: Vec::new(),
        }
    }
}

// =============================================================================
// Error Types
// =============================================================================
// =============================================================================

/// Convert an Address (20 bytes) to U256 (32 bytes) with zero padding
fn address_to_u256(addr: &Address) -> U256 {
    let mut bytes = [0u8; 32];
    bytes[12..32].copy_from_slice(addr.as_bytes());
    U256::from_be_bytes(bytes)
}


// =============================================================================
// Block Information Opcodes
// =============================================================================

/// BLOCKHASH (0x40) - Get hash of one of the 256 most recent complete blocks (uses Host).
pub fn blockhash_with_host<S: State, H: Host<S>>(
    stack: &mut Stack,
    host: &H,
) -> Result<(), OpcodeError> {
    let number = stack.pop()?;
    let hash = host.blockhash(&number).unwrap_or(Hash::ZERO);
    stack.push(utils::hash_to_u256(&hash))?;
    Ok(())
}

/// COINBASE (0x41) - Get the block's beneficiary address
pub fn coinbase(stack: &mut Stack, block_ctx: &BlockContext) -> Result<(), OpcodeError> {
    stack.push(address_to_u256(&block_ctx.coinbase))?;
    Ok(())
}

/// TIMESTAMP (0x42) - Get the block's timestamp
pub fn timestamp(stack: &mut Stack, block_ctx: &BlockContext) -> Result<(), OpcodeError> {
    stack.push(block_ctx.timestamp)?;
    Ok(())
}

/// NUMBER (0x43) - Get the block's number
pub fn number(stack: &mut Stack, block_ctx: &BlockContext) -> Result<(), OpcodeError> {
    stack.push(block_ctx.number)?;
    Ok(())
}

/// DIFFICULTY (0x44) - Get the block's difficulty (or prevrandao post-merge)
pub fn difficulty(stack: &mut Stack, block_ctx: &BlockContext) -> Result<(), OpcodeError> {
    stack.push(block_ctx.difficulty)?;
    Ok(())
}

/// GASLIMIT (0x45) - Get the block's gas limit
pub fn gaslimit(stack: &mut Stack, block_ctx: &BlockContext) -> Result<(), OpcodeError> {
    stack.push(block_ctx.gas_limit)?;
    Ok(())
}

/// CHAINID (0x46) - Get the chain ID
pub fn chainid(stack: &mut Stack, block_ctx: &BlockContext) -> Result<(), OpcodeError> {
    stack.push(block_ctx.chain_id)?;
    Ok(())
}

/// BASEFEE (0x48) - Get the base fee
pub fn basefee(stack: &mut Stack, block_ctx: &BlockContext) -> Result<(), OpcodeError> {
    stack.push(block_ctx.base_fee)?;
    Ok(())
}

// =============================================================================
// Transaction Context Opcodes
// =============================================================================

/// ORIGIN (0x32) - Get execution origination address
pub fn origin(stack: &mut Stack, tx_ctx: &TxContext) -> Result<(), OpcodeError> {
    stack.push(address_to_u256(&tx_ctx.origin))?;
    Ok(())
}

/// GASPRICE (0x3A) - Get price of gas in current environment
pub fn gasprice(stack: &mut Stack, tx_ctx: &TxContext) -> Result<(), OpcodeError> {
    stack.push(tx_ctx.gas_price)?;
    Ok(())
}

// =============================================================================
// Contract Context Opcodes
// =============================================================================

/// ADDRESS (0x30) - Get address of currently executing account
pub fn address(stack: &mut Stack, contract_ctx: &ContractContext) -> Result<(), OpcodeError> {
    stack.push(address_to_u256(&contract_ctx.address))?;
    Ok(())
}

/// CALLER (0x33) - Get caller address
pub fn caller(stack: &mut Stack, contract_ctx: &ContractContext) -> Result<(), OpcodeError> {
    stack.push(address_to_u256(&contract_ctx.caller))?;
    Ok(())
}

/// CALLVALUE (0x34) - Get deposited value by the instruction/transaction
pub fn callvalue(stack: &mut Stack, contract_ctx: &ContractContext) -> Result<(), OpcodeError> {
    stack.push(contract_ctx.call_value)?;
    Ok(())
}

/// CALLDATALOAD (0x35) - Get input data of current environment
pub fn calldataload(stack: &mut Stack, contract_ctx: &ContractContext) -> Result<(), OpcodeError> {
    let offset = stack.pop()?.as_usize();
    let mut data = [0u8; 32];

    let call_data_len = contract_ctx.call_data.len();
    if offset < call_data_len {
        let end = (offset + 32).min(call_data_len);
        let copy_len = end - offset;
        data[..copy_len].copy_from_slice(&contract_ctx.call_data[offset..end]);
    }

    stack.push(U256::from_be_bytes(data))?;
    Ok(())
}

/// CALLDATASIZE (0x36) - Get size of input data
pub fn calldatasize(stack: &mut Stack, contract_ctx: &ContractContext) -> Result<(), OpcodeError> {
    stack.push(U256::from_u64(contract_ctx.call_data.len() as u64))?;
    Ok(())
}

/// CALLDATACOPY (0x37) - Copy input data to memory
pub fn calldatacopy(
    stack: &mut Stack,
    memory: &mut Memory,
    contract_ctx: &ContractContext,
) -> Result<(), OpcodeError> {
    let dest_offset = stack.pop()?.as_usize();
    let offset = stack.pop()?.as_usize();
    let size = stack.pop()?.as_usize();

    if size == 0 {
        return Ok(());
    }

    let dest_end = dest_offset
        .checked_add(size)
        .ok_or(OpcodeError::InvalidOffset)?;
    memory.expand(dest_end)?;

    for i in 0..size {
        let byte = if offset + i < contract_ctx.call_data.len() {
            contract_ctx.call_data[offset + i]
        } else {
            0
        };
        memory.mstore8(dest_offset + i, byte)?;
    }

    Ok(())
}

/// CODESIZE (0x38) - Get size of code running in current environment
pub fn codesize(stack: &mut Stack, contract_ctx: &ContractContext) -> Result<(), OpcodeError> {
    stack.push(U256::from_u64(contract_ctx.code.len() as u64))?;
    Ok(())
}

/// CODECOPY (0x39) - Copy code running in current environment to memory
pub fn codecopy(
    stack: &mut Stack,
    memory: &mut Memory,
    contract_ctx: &ContractContext,
) -> Result<(), OpcodeError> {
    let dest_offset = stack.pop()?.as_usize();
    let offset = stack.pop()?.as_usize();
    let size = stack.pop()?.as_usize();

    if size == 0 {
        return Ok(());
    }

    let dest_end = dest_offset
        .checked_add(size)
        .ok_or(OpcodeError::InvalidOffset)?;
    memory.expand(dest_end)?;

    for i in 0..size {
        let byte = if offset + i < contract_ctx.code.len() {
            contract_ctx.code[offset + i]
        } else {
            0
        };
        memory.mstore8(dest_offset + i, byte)?;
    }

    Ok(())
}

/// RETURNDATASIZE (0x3D) - Get size of output data from the previous call
pub fn returndatasize(
    stack: &mut Stack,
    contract_ctx: &ContractContext,
) -> Result<(), OpcodeError> {
    stack.push(U256::from_u64(contract_ctx.return_data.len() as u64))?;
    Ok(())
}

/// RETURNDATACOPY (0x3E) - Copy output data from the previous call to memory
pub fn returndatacopy(
    stack: &mut Stack,
    memory: &mut Memory,
    contract_ctx: &ContractContext,
) -> Result<(), OpcodeError> {
    let dest_offset = stack.pop()?.as_usize();
    let offset = stack.pop()?.as_usize();
    let size = stack.pop()?.as_usize();

    if size == 0 {
        return Ok(());
    }

    // Check if offset + size exceeds return data length
    let return_data_len = contract_ctx.return_data.len();
    if offset.checked_add(size).ok_or(OpcodeError::InvalidOffset)? > return_data_len {
        return Err(OpcodeError::InvalidOffset);
    }

    let dest_end = dest_offset
        .checked_add(size)
        .ok_or(OpcodeError::InvalidOffset)?;
    memory.expand(dest_end)?;

    for i in 0..size {
        memory.mstore8(dest_offset + i, contract_ctx.return_data[offset + i])?;
    }

    Ok(())
}

// =============================================================================
// External Account Opcodes (State + EIP-2929 warm/cold)
// =============================================================================

/// BALANCE (0x31) - Get balance of the given account. Caller charges base gas (GAS_BALANCE_COLD) first.
pub fn balance<S: State>(
    stack: &mut Stack,
    state: &S,
    is_warm: bool,
    gas_remaining: &mut u64,
) -> Result<(), EvmError> {
    let address = utils::u256_to_address(&stack.pop().map_err(EvmError::from)?)?;
    if is_warm {
        *gas_remaining = (*gas_remaining).saturating_add(2600 - 100); // GAS_BALANCE_COLD - GAS_BALANCE_WARM
    }
    let b = state.get_balance(&address);
    stack.push(b).map_err(EvmError::from)?;
    Ok(())
}

/// EXTCODESIZE (0x3B) - Get size of an account's code. Caller charges base gas first.
pub fn extcodesize<S: State>(
    stack: &mut Stack,
    state: &S,
    is_warm: bool,
    gas_remaining: &mut u64,
) -> Result<(), EvmError> {
    let address = utils::u256_to_address(&stack.pop().map_err(EvmError::from)?)?;
    if is_warm {
        *gas_remaining = (*gas_remaining).saturating_add(2600 - 100);
    }
    let len = state.get_code(&address).len();
    stack
        .push(U256::from_u64(len as u64))
        .map_err(EvmError::from)?;
    Ok(())
}

/// EXTCODECOPY (0x3C) - Copy an account's code to memory. Caller charges base + memory + copy gas first.
pub fn extcodecopy<S: State>(
    stack: &mut Stack,
    state: &S,
    memory: &mut Memory,
    is_warm: bool,
    gas_remaining: &mut u64,
) -> Result<(), EvmError> {
    let address = utils::u256_to_address(&stack.pop().map_err(EvmError::from)?)?;
    if is_warm {
        *gas_remaining = (*gas_remaining).saturating_add(2600 - 100);
    }
    let dest_offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let code_offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let size = stack.pop().map_err(EvmError::from)?.as_usize();

    if size == 0 {
        return Ok(());
    }

    let code = state.get_code(&address);
    for i in 0..size {
        let byte = if code_offset + i < code.len() {
            code[code_offset + i]
        } else {
            0
        };
        memory
            .mstore8(dest_offset + i, byte)
            .map_err(EvmError::from)?;
    }
    Ok(())
}

/// EXTCODEHASH (0x3F) - Get hash of an account's code. Caller charges base gas first.
pub fn extcodehash<S: State>(
    stack: &mut Stack,
    state: &S,
    is_warm: bool,
    gas_remaining: &mut u64,
) -> Result<(), EvmError> {
    let address = utils::u256_to_address(&stack.pop().map_err(EvmError::from)?)?;
    if is_warm {
        *gas_remaining = (*gas_remaining).saturating_add(2600 - 100);
    }
    let code_hash = state.get_code_hash(&address);
    stack
        .push(utils::hash_to_u256(&code_hash))
        .map_err(EvmError::from)?;
    Ok(())
}

/// SELFBALANCE (0x47) - Get balance of currently executing account
pub fn selfbalance<S: State>(
    stack: &mut Stack,
    state: &S,
    address: Address,
) -> Result<(), EvmError> {
    let b = state.get_balance(&address);
    stack.push(b).map_err(EvmError::from)?;
    Ok(())
}

/// BLOBHASH (0x49) - Get hash of blob at index (uses Host).
pub fn blobhash_with_host<S: State, H: Host<S>>(
    stack: &mut Stack,
    host: &H,
) -> Result<(), OpcodeError> {
    let index = stack.pop()?;
    let hash = host.blobhash(&index).unwrap_or(Hash::ZERO);
    stack.push(utils::hash_to_u256(&hash))?;
    Ok(())
}

/// BLOBBASEFEE (0x4A) - Get blob base fee (uses Host).
pub fn blobbasefee_with_host<S: State, H: Host<S>>(
    stack: &mut Stack,
    host: &H,
) -> Result<(), OpcodeError> {
    stack.push(host.blobbasefee())?;
    Ok(())
}

// =============================================================================
// Cryptographic Opcodes
// =============================================================================

/// SHA3 (0x20) - Compute Keccak-256 hash
pub fn sha3(stack: &mut Stack, memory: &mut Memory) -> Result<(), OpcodeError> {
    let offset = stack.pop()?.as_usize();
    let size = stack.pop()?.as_usize();

    if size == 0 {
        // Hash of empty data
        let hash = keccak256(&[]);
        stack.push(U256::from_be_bytes(*hash.as_bytes()))?;
        return Ok(());
    }

    // Expand memory and load data
    let end = offset.checked_add(size).ok_or(OpcodeError::InvalidOffset)?;
    memory.expand(end)?;

    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        let word = memory.mload(offset + (i / 32) * 32)?;
        let bytes = word.to_be_bytes();
        data.push(bytes[i % 32]);
    }

    let hash = keccak256(&data);
    stack.push(U256::from_be_bytes(*hash.as_bytes()))?;
    Ok(())
}

// =============================================================================
// Control Flow Opcodes
// =============================================================================

/// RETURN (0xF3) - Halt execution returning output data
pub fn op_return(stack: &mut Stack, memory: &mut Memory) -> Result<Vec<u8>, OpcodeError> {
    let offset = stack.pop()?.as_usize();
    let size = stack.pop()?.as_usize();

    if size == 0 {
        return Ok(Vec::new());
    }

    let end = offset.checked_add(size).ok_or(OpcodeError::InvalidOffset)?;
    memory.expand(end)?;

    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        let word = memory.mload(offset + (i / 32) * 32)?;
        let bytes = word.to_be_bytes();
        data.push(bytes[i % 32]);
    }

    Ok(data)
}

/// REVERT (0xFD) - Halt execution reverting state changes
pub fn revert(stack: &mut Stack, memory: &mut Memory) -> Result<Vec<u8>, OpcodeError> {
    let offset = stack.pop()?.as_usize();
    let size = stack.pop()?.as_usize();

    if size == 0 {
        return Ok(Vec::new());
    }

    let end = offset.checked_add(size).ok_or(OpcodeError::InvalidOffset)?;
    memory.expand(end)?;

    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        let word = memory.mload(offset + (i / 32) * 32)?;
        let bytes = word.to_be_bytes();
        data.push(bytes[i % 32]);
    }

    Ok(data)
}

// =============================================================================
// Call/Create context (minimal set for system opcodes)
// =============================================================================

/// Minimal call context for CALL/CREATE/DELEGATECALL/STATICCALL.
#[derive(Debug, Clone)]
pub struct CallEnv {
    pub self_address: Address,
    pub caller: Address,
    pub call_value: U256,
}

// =============================================================================
// Complex Opcodes (CREATE, CALL, etc.) – require State + Host
// =============================================================================

/// CREATE (0xF0). Caller must charge base gas and memory expansion + init code gas first.
pub fn execute_create<S: State, H: Host<S>>(
    state: &mut S,
    host: &mut H,
    stack: &mut Stack,
    memory: &mut Memory,
    call_env: &CallEnv,
    gas_remaining: &mut u64,
    return_data: &mut Vec<u8>,
) -> Result<(), EvmError> {
    let value = stack.pop().map_err(EvmError::from)?;
    let offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let size = stack.pop().map_err(EvmError::from)?.as_usize();

    let init_code = utils::read_memory_bytes(memory, offset, size)?;
    let max_gas = (*gas_remaining).saturating_sub((*gas_remaining) / 64);

    let msg = CreateMessage {
        gas: max_gas,
        caller: call_env.self_address,
        value,
        init_code,
        salt: None,
    };
    let result = host.create(state, msg);
    if result.gas_used > max_gas {
        return Err(EvmError::OutOfGas);
    }
    utils::consume_gas(gas_remaining, result.gas_used)?;
    *return_data = result.return_data;

    if result.success {
        let addr = result.address.unwrap_or(Address::ZERO);
        stack
            .push(utils::address_to_u256(&addr))
            .map_err(EvmError::from)?;
    } else {
        stack.push(U256::ZERO).map_err(EvmError::from)?;
    }
    Ok(())
}

/// CREATE2 (0xF5). Caller must charge base gas, memory expansion, init code gas, and hash cost first.
pub fn execute_create2<S: State, H: Host<S>>(
    state: &mut S,
    host: &mut H,
    stack: &mut Stack,
    memory: &mut Memory,
    call_env: &CallEnv,
    gas_remaining: &mut u64,
    return_data: &mut Vec<u8>,
) -> Result<(), EvmError> {
    let value = stack.pop().map_err(EvmError::from)?;
    let offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let size = stack.pop().map_err(EvmError::from)?.as_usize();
    let salt = stack.pop().map_err(EvmError::from)?;

    let init_code = utils::read_memory_bytes(memory, offset, size)?;
    let max_gas = (*gas_remaining).saturating_sub((*gas_remaining) / 64);

    let msg = CreateMessage {
        gas: max_gas,
        caller: call_env.self_address,
        value,
        init_code,
        salt: Some(salt),
    };
    let result = host.create(state, msg);
    if result.gas_used > max_gas {
        return Err(EvmError::OutOfGas);
    }
    utils::consume_gas(gas_remaining, result.gas_used)?;
    *return_data = result.return_data;

    if result.success {
        let addr = result.address.unwrap_or(Address::ZERO);
        stack
            .push(utils::address_to_u256(&addr))
            .map_err(EvmError::from)?;
    } else {
        stack.push(U256::ZERO).map_err(EvmError::from)?;
    }
    Ok(())
}

/// CALL (0xF1). Caller must charge base gas, memory expansion, value transfer, and new-account cost.
#[allow(clippy::too_many_arguments)]
pub fn execute_call<S: State, H: Host<S>>(
    state: &mut S,
    host: &mut H,
    stack: &mut Stack,
    memory: &mut Memory,
    call_env: &CallEnv,
    gas_remaining: &mut u64,
    return_data: &mut Vec<u8>,
    is_warm: bool,
) -> Result<(), EvmError> {
    let gas_requested = stack.pop().map_err(EvmError::from)?.as_u64();
    let to = utils::u256_to_address(&stack.pop().map_err(EvmError::from)?)?;
    let value = stack.pop().map_err(EvmError::from)?;
    let in_offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let in_size = stack.pop().map_err(EvmError::from)?.as_usize();
    let out_offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let out_size = stack.pop().map_err(EvmError::from)?.as_usize();

    if is_warm {
        *gas_remaining = (*gas_remaining).saturating_add(2600 - 100);
    }

    let input = utils::read_memory_bytes(memory, in_offset, in_size)?;
    let is_value_transfer = !value.is_zero();
    if is_value_transfer {
        utils::consume_gas(gas_remaining, GAS_CALL_VALUE_TRANSFER)?;
        if !state.account_exists(&to) {
            utils::consume_gas(gas_remaining, GAS_CALL_NEW_ACCOUNT)?;
        }
    }

    let mut gas_to_forward = gas_requested;
    let max_forward = (*gas_remaining).saturating_sub((*gas_remaining) / 64);
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
        caller: call_env.self_address,
        value,
        code_address: to,
        input,
        is_static: false,
    };
    let result = host.call(state, msg);
    if result.gas_used > gas_to_forward {
        return Err(EvmError::OutOfGas);
    }
    utils::consume_gas(gas_remaining, result.gas_used)?;
    *return_data = result.return_data.clone();
    utils::write_memory_bytes(memory, out_offset, &result.return_data, out_size)?;
    stack
        .push(if result.success {
            U256::ONE
        } else {
            U256::ZERO
        })
        .map_err(EvmError::from)?;
    Ok(())
}

/// CALLCODE (0xF2).
#[allow(clippy::too_many_arguments)]
pub fn execute_callcode<S: State, H: Host<S>>(
    state: &mut S,
    host: &mut H,
    stack: &mut Stack,
    memory: &mut Memory,
    call_env: &CallEnv,
    gas_remaining: &mut u64,
    return_data: &mut Vec<u8>,
    is_warm: bool,
) -> Result<(), EvmError> {
    let gas_requested = stack.pop().map_err(EvmError::from)?.as_u64();
    let to = utils::u256_to_address(&stack.pop().map_err(EvmError::from)?)?;
    let value = stack.pop().map_err(EvmError::from)?;
    let in_offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let in_size = stack.pop().map_err(EvmError::from)?.as_usize();
    let out_offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let out_size = stack.pop().map_err(EvmError::from)?.as_usize();

    if is_warm {
        *gas_remaining = (*gas_remaining).saturating_add(2600 - 100);
    }

    let input = utils::read_memory_bytes(memory, in_offset, in_size)?;
    let is_value_transfer = !value.is_zero();
    if is_value_transfer {
        utils::consume_gas(gas_remaining, GAS_CALL_VALUE_TRANSFER)?;
    }

    let mut gas_to_forward = gas_requested;
    let max_forward = (*gas_remaining).saturating_sub((*gas_remaining) / 64);
    if gas_to_forward > max_forward {
        gas_to_forward = max_forward;
    }
    if is_value_transfer {
        gas_to_forward = gas_to_forward.saturating_add(GAS_CALL_STIPEND);
    }

    let msg = CallMessage {
        kind: CallKind::CallCode,
        gas: gas_to_forward,
        address: call_env.self_address,
        caller: call_env.self_address,
        value,
        code_address: to,
        input,
        is_static: false,
    };
    let result = host.call(state, msg);
    if result.gas_used > gas_to_forward {
        return Err(EvmError::OutOfGas);
    }
    utils::consume_gas(gas_remaining, result.gas_used)?;
    *return_data = result.return_data.clone();
    utils::write_memory_bytes(memory, out_offset, &result.return_data, out_size)?;
    stack
        .push(if result.success {
            U256::ONE
        } else {
            U256::ZERO
        })
        .map_err(EvmError::from)?;
    Ok(())
}

/// DELEGATECALL (0xF4).
#[allow(clippy::too_many_arguments)]
pub fn execute_delegatecall<S: State, H: Host<S>>(
    state: &mut S,
    host: &mut H,
    stack: &mut Stack,
    memory: &mut Memory,
    call_env: &CallEnv,
    gas_remaining: &mut u64,
    return_data: &mut Vec<u8>,
    is_warm: bool,
) -> Result<(), EvmError> {
    // Stack (top to bottom): gas, to, in_offset, in_size, out_offset, out_size
    let gas_requested = stack.pop().map_err(EvmError::from)?.as_u64();
    let to = utils::u256_to_address(&stack.pop().map_err(EvmError::from)?)?;
    let in_offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let in_size = stack.pop().map_err(EvmError::from)?.as_usize();
    let out_offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let out_size = stack.pop().map_err(EvmError::from)?.as_usize();

    if is_warm {
        *gas_remaining = (*gas_remaining).saturating_add(2600 - 100);
    }

    let input = utils::read_memory_bytes(memory, in_offset, in_size)?;
    let mut gas_to_forward = gas_requested;
    let max_forward = (*gas_remaining).saturating_sub((*gas_remaining) / 64);
    if gas_to_forward > max_forward {
        gas_to_forward = max_forward;
    }

    let msg = CallMessage {
        kind: CallKind::DelegateCall,
        gas: gas_to_forward,
        address: call_env.self_address,
        caller: call_env.caller,
        value: call_env.call_value,
        code_address: to,
        input,
        is_static: false,
    };
    let result = host.call(state, msg);
    if result.gas_used > gas_to_forward {
        return Err(EvmError::OutOfGas);
    }
    utils::consume_gas(gas_remaining, result.gas_used)?;
    *return_data = result.return_data.clone();
    utils::write_memory_bytes(memory, out_offset, &result.return_data, out_size)?;
    stack
        .push(if result.success {
            U256::ONE
        } else {
            U256::ZERO
        })
        .map_err(EvmError::from)?;
    Ok(())
}

/// STATICCALL (0xFA).
#[allow(clippy::too_many_arguments)]
pub fn execute_staticcall<S: State, H: Host<S>>(
    state: &mut S,
    host: &mut H,
    stack: &mut Stack,
    memory: &mut Memory,
    call_env: &CallEnv,
    gas_remaining: &mut u64,
    return_data: &mut Vec<u8>,
    is_warm: bool,
) -> Result<(), EvmError> {
    // Stack (top to bottom): gas, to, in_offset, in_size, out_offset, out_size
    let gas_requested = stack.pop().map_err(EvmError::from)?.as_u64();
    let to = utils::u256_to_address(&stack.pop().map_err(EvmError::from)?)?;
    let in_offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let in_size = stack.pop().map_err(EvmError::from)?.as_usize();
    let out_offset = stack.pop().map_err(EvmError::from)?.as_usize();
    let out_size = stack.pop().map_err(EvmError::from)?.as_usize();

    if is_warm {
        *gas_remaining = (*gas_remaining).saturating_add(2600 - 100);
    }

    let input = utils::read_memory_bytes(memory, in_offset, in_size)?;
    let mut gas_to_forward = gas_requested;
    let max_forward = (*gas_remaining).saturating_sub((*gas_remaining) / 64);
    if gas_to_forward > max_forward {
        gas_to_forward = max_forward;
    }

    let msg = CallMessage {
        kind: CallKind::StaticCall,
        gas: gas_to_forward,
        address: to,
        caller: call_env.self_address,
        value: U256::ZERO,
        code_address: to,
        input,
        is_static: true,
    };
    let result = host.call(state, msg);
    if result.gas_used > gas_to_forward {
        return Err(EvmError::OutOfGas);
    }
    utils::consume_gas(gas_remaining, result.gas_used)?;
    *return_data = result.return_data.clone();
    utils::write_memory_bytes(memory, out_offset, &result.return_data, out_size)?;
    stack
        .push(if result.success {
            U256::ONE
        } else {
            U256::ZERO
        })
        .map_err(EvmError::from)?;
    Ok(())
}

/// SELFDESTRUCT (0xFF).
pub fn execute_selfdestruct<S: State>(
    state: &mut S,
    stack: &mut Stack,
    self_address: Address,
) -> Result<(), EvmError> {
    let beneficiary = utils::u256_to_address(&stack.pop().map_err(EvmError::from)?)?;
    state.selfdestruct(&self_address, &beneficiary);
    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create default contexts
    fn create_block_context() -> BlockContext {
        BlockContext::new(BlockContextParams {
            coinbase: Address::from([0x42; 20]),
            timestamp: U256::from_u64(1234567890),
            number: U256::from_u64(1000),
            difficulty: U256::from_u64(12345),
            gas_limit: U256::from_u64(30_000_000),
            chain_id: U256::from_u64(1),
            base_fee: U256::from_u64(100),
            excess_blob_gas: None,
        })
    }

    fn create_tx_context() -> TxContext {
        TxContext::new(Address::from([0x11; 20]), U256::from_u64(20_000_000_000))
    }

    fn create_contract_context() -> ContractContext {
        ContractContext::new(
            Address::from([0xAA; 20]),
            Address::from([0xBB; 20]),
            U256::from_u64(1_000_000),
            vec![0x12, 0x34, 0x56, 0x78],
            vec![0x60, 0x00, 0x60, 0x00],
        )
    }

    // =============================================================================
    // Block Information Opcode Tests
    // =============================================================================

    #[test]
    fn test_coinbase() {
        let mut stack = Stack::new();
        let block_ctx = create_block_context();

        coinbase(&mut stack, &block_ctx).unwrap();

        let result = stack.pop().unwrap();
        assert_eq!(result, address_to_u256(&block_ctx.coinbase));
    }

    #[test]
    fn test_timestamp() {
        let mut stack = Stack::new();
        let block_ctx = create_block_context();

        timestamp(&mut stack, &block_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from_u64(1234567890));
    }

    #[test]
    fn test_number() {
        let mut stack = Stack::new();
        let block_ctx = create_block_context();

        number(&mut stack, &block_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from_u64(1000));
    }

    #[test]
    fn test_difficulty() {
        let mut stack = Stack::new();
        let block_ctx = create_block_context();

        difficulty(&mut stack, &block_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from_u64(12345));
    }

    #[test]
    fn test_gaslimit() {
        let mut stack = Stack::new();
        let block_ctx = create_block_context();

        gaslimit(&mut stack, &block_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from_u64(30_000_000));
    }

    #[test]
    fn test_chainid() {
        let mut stack = Stack::new();
        let block_ctx = create_block_context();

        chainid(&mut stack, &block_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from_u64(1));
    }

    #[test]
    fn test_basefee() {
        let mut stack = Stack::new();
        let block_ctx = create_block_context();

        basefee(&mut stack, &block_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from_u64(100));
    }

    #[test]
    fn test_blockhash() {
        use crate::evm::host::NullHost;
        use crate::state::InMemoryState;

        let mut stack = Stack::new();
        let host = NullHost;

        stack.push(U256::from_u64(999)).unwrap();
        blockhash_with_host::<InMemoryState, _>(&mut stack, &host).unwrap();

        // NullHost returns None -> zero hash
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    // =============================================================================
    // Transaction Context Opcode Tests
    // =============================================================================

    #[test]
    fn test_origin() {
        let mut stack = Stack::new();
        let tx_ctx = create_tx_context();

        origin(&mut stack, &tx_ctx).unwrap();

        let result = stack.pop().unwrap();
        assert_eq!(result, address_to_u256(&tx_ctx.origin));
    }

    #[test]
    fn test_gasprice() {
        let mut stack = Stack::new();
        let tx_ctx = create_tx_context();

        gasprice(&mut stack, &tx_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from_u64(20_000_000_000));
    }

    // =============================================================================
    // Contract Context Opcode Tests
    // =============================================================================

    #[test]
    fn test_address() {
        let mut stack = Stack::new();
        let contract_ctx = create_contract_context();

        address(&mut stack, &contract_ctx).unwrap();

        let result = stack.pop().unwrap();
        assert_eq!(result, address_to_u256(&contract_ctx.address));
    }

    #[test]
    fn test_caller() {
        let mut stack = Stack::new();
        let contract_ctx = create_contract_context();

        caller(&mut stack, &contract_ctx).unwrap();

        let result = stack.pop().unwrap();
        assert_eq!(result, address_to_u256(&contract_ctx.caller));
    }

    #[test]
    fn test_callvalue() {
        let mut stack = Stack::new();
        let contract_ctx = create_contract_context();

        callvalue(&mut stack, &contract_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from_u64(1_000_000));
    }

    #[test]
    fn test_calldataload() {
        let mut stack = Stack::new();
        let contract_ctx = create_contract_context();

        stack.push(U256::ZERO).unwrap();
        calldataload(&mut stack, &contract_ctx).unwrap();

        let result = stack.pop().unwrap();
        let expected = {
            let mut data = [0u8; 32];
            data[0] = 0x12;
            data[1] = 0x34;
            data[2] = 0x56;
            data[3] = 0x78;
            U256::from_be_bytes(data)
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn test_calldataload_offset() {
        let mut stack = Stack::new();
        let contract_ctx = create_contract_context();

        stack.push(U256::from_u64(2)).unwrap();
        calldataload(&mut stack, &contract_ctx).unwrap();

        let result = stack.pop().unwrap();
        let expected = {
            let mut data = [0u8; 32];
            data[0] = 0x56;
            data[1] = 0x78;
            U256::from_be_bytes(data)
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn test_calldataload_beyond_end() {
        let mut stack = Stack::new();
        let contract_ctx = create_contract_context();

        stack.push(U256::from_u64(100)).unwrap();
        calldataload(&mut stack, &contract_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_calldatasize() {
        let mut stack = Stack::new();
        let contract_ctx = create_contract_context();

        calldatasize(&mut stack, &contract_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from_u64(4));
    }

    #[test]
    fn test_calldatacopy() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();
        let contract_ctx = create_contract_context();

        stack.push(U256::from_u64(4)).unwrap(); // size
        stack.push(U256::ZERO).unwrap(); // offset
        stack.push(U256::ZERO).unwrap(); // dest_offset
        calldatacopy(&mut stack, &mut memory, &contract_ctx).unwrap();

        let result = memory.mload(0).unwrap();
        let expected = {
            let mut data = [0u8; 32];
            data[0] = 0x12;
            data[1] = 0x34;
            data[2] = 0x56;
            data[3] = 0x78;
            U256::from_be_bytes(data)
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn test_calldatacopy_partial() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();
        let contract_ctx = create_contract_context();

        stack.push(U256::from_u64(2)).unwrap(); // size
        stack.push(U256::from_u64(1)).unwrap(); // offset
        stack.push(U256::from_u64(10)).unwrap(); // dest_offset
        calldatacopy(&mut stack, &mut memory, &contract_ctx).unwrap();

        let result = memory.mload(0).unwrap();
        let bytes = result.to_be_bytes();
        assert_eq!(bytes[10], 0x34);
        assert_eq!(bytes[11], 0x56);
    }

    #[test]
    fn test_calldatacopy_beyond_end() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();
        let contract_ctx = create_contract_context();

        stack.push(U256::from_u64(10)).unwrap(); // size (beyond call data)
        stack.push(U256::from_u64(2)).unwrap(); // offset
        stack.push(U256::ZERO).unwrap(); // dest_offset
        calldatacopy(&mut stack, &mut memory, &contract_ctx).unwrap();

        let result = memory.mload(0).unwrap();
        let bytes = result.to_be_bytes();
        assert_eq!(bytes[0], 0x56);
        assert_eq!(bytes[1], 0x78);
        assert_eq!(bytes[2], 0x00); // Zero padding
    }

    #[test]
    fn test_codesize() {
        let mut stack = Stack::new();
        let contract_ctx = create_contract_context();

        codesize(&mut stack, &contract_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from_u64(4));
    }

    #[test]
    fn test_codecopy() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();
        let contract_ctx = create_contract_context();

        stack.push(U256::from_u64(4)).unwrap(); // size
        stack.push(U256::ZERO).unwrap(); // offset
        stack.push(U256::ZERO).unwrap(); // dest_offset
        codecopy(&mut stack, &mut memory, &contract_ctx).unwrap();

        let result = memory.mload(0).unwrap();
        let expected = {
            let mut data = [0u8; 32];
            data[0] = 0x60;
            data[1] = 0x00;
            data[2] = 0x60;
            data[3] = 0x00;
            U256::from_be_bytes(data)
        };
        assert_eq!(result, expected);
    }

    #[test]
    fn test_returndatasize() {
        let mut stack = Stack::new();
        let mut contract_ctx = create_contract_context();
        contract_ctx.return_data = vec![0xAA, 0xBB, 0xCC];

        returndatasize(&mut stack, &contract_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::from_u64(3));
    }

    #[test]
    fn test_returndatacopy() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();
        let mut contract_ctx = create_contract_context();
        contract_ctx.return_data = vec![0xAA, 0xBB, 0xCC, 0xDD];

        stack.push(U256::from_u64(3)).unwrap(); // size
        stack.push(U256::from_u64(1)).unwrap(); // offset
        stack.push(U256::ZERO).unwrap(); // dest_offset
        returndatacopy(&mut stack, &mut memory, &contract_ctx).unwrap();

        let result = memory.mload(0).unwrap();
        let bytes = result.to_be_bytes();
        assert_eq!(bytes[0], 0xBB);
        assert_eq!(bytes[1], 0xCC);
        assert_eq!(bytes[2], 0xDD);
    }

    #[test]
    fn test_returndatacopy_out_of_bounds() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();
        let mut contract_ctx = create_contract_context();
        contract_ctx.return_data = vec![0xAA, 0xBB];

        stack.push(U256::from_u64(5)).unwrap(); // size (exceeds return data)
        stack.push(U256::ZERO).unwrap(); // offset
        stack.push(U256::ZERO).unwrap(); // dest_offset

        let result = returndatacopy(&mut stack, &mut memory, &contract_ctx);
        assert_eq!(result, Err(OpcodeError::InvalidOffset));
    }

    // =============================================================================
    // External Account Opcode Tests
    // =============================================================================

    #[test]
    fn test_balance() {
        use crate::state::InMemoryState;

        let mut stack = Stack::new();
        let state = InMemoryState::new();
        let mut gas = 10_000u64;

        stack
            .push(address_to_u256(&Address::from([0x12; 20])))
            .unwrap();
        balance::<InMemoryState>(&mut stack, &state, false, &mut gas).unwrap();

        // Empty state returns zero
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_extcodesize() {
        use crate::state::InMemoryState;

        let mut stack = Stack::new();
        let state = InMemoryState::new();
        let mut gas = 10_000u64;

        stack
            .push(address_to_u256(&Address::from([0x12; 20])))
            .unwrap();
        extcodesize::<InMemoryState>(&mut stack, &state, false, &mut gas).unwrap();

        // No code returns zero
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_extcodecopy() {
        use crate::state::InMemoryState;

        let mut stack = Stack::new();
        let mut memory = Memory::new();
        let state = InMemoryState::new();
        let mut gas = 10_000u64;

        stack.push(U256::from_u64(10)).unwrap(); // size
        stack.push(U256::ZERO).unwrap(); // offset
        stack.push(U256::ZERO).unwrap(); // dest_offset
        stack
            .push(address_to_u256(&Address::from([0x12; 20])))
            .unwrap(); // address
        extcodecopy::<InMemoryState>(&mut stack, &state, &mut memory, false, &mut gas).unwrap();

        // No code fills with zeros
        let result = memory.mload(0).unwrap();
        assert_eq!(result, U256::ZERO);
    }

    #[test]
    fn test_extcodehash() {
        use crate::state::InMemoryState;

        let mut stack = Stack::new();
        let state = InMemoryState::new();
        let mut gas = 10_000u64;

        stack
            .push(address_to_u256(&Address::from([0x12; 20])))
            .unwrap();
        extcodehash::<InMemoryState>(&mut stack, &state, false, &mut gas).unwrap();

        // Empty account returns EMPTY_CODE_HASH (keccak256 of empty)
        let empty_hash = keccak256(&[]);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(empty_hash.as_bytes());
        assert_eq!(stack.pop().unwrap(), U256::from_be_bytes(bytes));
    }

    #[test]
    fn test_selfbalance() {
        use crate::state::InMemoryState;

        let mut stack = Stack::new();
        let state = InMemoryState::new();
        let addr = Address::from([0x42; 20]);

        selfbalance::<InMemoryState>(&mut stack, &state, addr).unwrap();

        // Empty state returns zero
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    // =============================================================================
    // Cryptographic Opcode Tests
    // =============================================================================

    #[test]
    fn test_sha3_empty() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        stack.push(U256::ZERO).unwrap(); // size
        stack.push(U256::ZERO).unwrap(); // offset
        sha3(&mut stack, &mut memory).unwrap();

        let result = stack.pop().unwrap();
        let expected_hash = keccak256(&[]);
        assert_eq!(result, U256::from_be_bytes(*expected_hash.as_bytes()));
    }

    #[test]
    fn test_sha3_data() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // Store some data in memory
        let data = U256::from_be_bytes([0xFF; 32]);
        memory.mstore(0, data).unwrap();

        stack.push(U256::from_u64(32)).unwrap(); // size
        stack.push(U256::ZERO).unwrap(); // offset
        sha3(&mut stack, &mut memory).unwrap();

        let result = stack.pop().unwrap();
        // Verify it's not zero (actual hash computed)
        assert_ne!(result, U256::ZERO);
    }

    // =============================================================================
    // Control Flow Opcode Tests
    // =============================================================================

    #[test]
    fn test_return_empty() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        stack.push(U256::ZERO).unwrap(); // size
        stack.push(U256::ZERO).unwrap(); // offset
        let data = op_return(&mut stack, &mut memory).unwrap();

        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_return_data() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // Store some data in memory
        let value = U256::from_u64(0xDEADBEEF);
        memory.mstore(0, value).unwrap();

        stack.push(U256::from_u64(4)).unwrap(); // size
        stack.push(U256::from_u64(28)).unwrap(); // offset (last 4 bytes of the word)
        let data = op_return(&mut stack, &mut memory).unwrap();

        assert_eq!(data.len(), 4);
        assert_eq!(data[0], 0xDE);
        assert_eq!(data[1], 0xAD);
        assert_eq!(data[2], 0xBE);
        assert_eq!(data[3], 0xEF);
    }

    #[test]
    fn test_revert_empty() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        stack.push(U256::ZERO).unwrap(); // size
        stack.push(U256::ZERO).unwrap(); // offset
        let data = revert(&mut stack, &mut memory).unwrap();

        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_revert_data() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // Store some data in memory
        let value = U256::from_u64(0xBADC0DE);
        memory.mstore(0, value).unwrap();

        stack.push(U256::from_u64(4)).unwrap(); // size
        stack.push(U256::from_u64(28)).unwrap(); // offset
        let data = revert(&mut stack, &mut memory).unwrap();

        assert_eq!(data.len(), 4);
    }

    // =============================================================================
    // Complex Opcode Tests (execute_* with NullHost / InMemoryState)
    // =============================================================================

    #[test]
    fn test_execute_create_with_null_host() {
        use crate::evm::host::NullHost;
        use crate::state::InMemoryState;

        let mut stack = Stack::new();
        let mut memory = Memory::new();
        let mut state = InMemoryState::new();
        let mut host = NullHost;
        let mut gas = 100_000u64;
        let mut return_data = Vec::new();
        let call_env = CallEnv {
            self_address: Address::from([0xAA; 20]),
            caller: Address::from([0xBB; 20]),
            call_value: U256::ZERO,
        };

        stack.push(U256::from_u64(0)).unwrap(); // size
        stack.push(U256::from_u64(0)).unwrap(); // offset
        stack.push(U256::ZERO).unwrap(); // value
        execute_create::<InMemoryState, _>(
            &mut state,
            &mut host,
            &mut stack,
            &mut memory,
            &call_env,
            &mut gas,
            &mut return_data,
        )
        .unwrap();
        // NullHost returns failure -> 0 on stack
        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_execute_selfdestruct() {
        use crate::state::InMemoryState;

        let mut stack = Stack::new();
        let mut state = InMemoryState::new();
        let self_addr = Address::from([0xAA; 20]);
        let beneficiary = Address::from([0xBB; 20]);

        stack.push(utils::address_to_u256(&beneficiary)).unwrap();
        execute_selfdestruct::<InMemoryState>(&mut state, &mut stack, self_addr).unwrap();
    }

    // =============================================================================
    // Edge Case Tests
    // =============================================================================

    #[test]
    fn test_calldataload_empty_calldata() {
        let mut stack = Stack::new();
        let mut contract_ctx = create_contract_context();
        contract_ctx.call_data = Vec::new();

        stack.push(U256::ZERO).unwrap();
        calldataload(&mut stack, &contract_ctx).unwrap();

        assert_eq!(stack.pop().unwrap(), U256::ZERO);
    }

    #[test]
    fn test_codecopy_empty_code() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();
        let mut contract_ctx = create_contract_context();
        contract_ctx.code = Vec::new();

        stack.push(U256::from_u64(10)).unwrap(); // size
        stack.push(U256::ZERO).unwrap(); // offset
        stack.push(U256::ZERO).unwrap(); // dest_offset
        codecopy(&mut stack, &mut memory, &contract_ctx).unwrap();

        // Should fill with zeros
        let result = memory.mload(0).unwrap();
        assert_eq!(result, U256::ZERO);
    }

    #[test]
    fn test_sha3_large_data() {
        let mut stack = Stack::new();
        let mut memory = Memory::new();

        // Store multiple words
        for i in 0..10 {
            memory.mstore(i * 32, U256::from_u64(i as u64)).unwrap();
        }

        stack.push(U256::from_u64(320)).unwrap(); // 10 words
        stack.push(U256::ZERO).unwrap();
        sha3(&mut stack, &mut memory).unwrap();

        let result = stack.pop().unwrap();
        assert_ne!(result, U256::ZERO);
    }

    #[test]
    fn test_context_cloning() {
        let block_ctx = create_block_context();
        let block_ctx_clone = block_ctx.clone();
        assert_eq!(block_ctx.timestamp, block_ctx_clone.timestamp);

        let tx_ctx = create_tx_context();
        let tx_ctx_clone = tx_ctx.clone();
        assert_eq!(tx_ctx.gas_price, tx_ctx_clone.gas_price);

        let contract_ctx = create_contract_context();
        let contract_ctx_clone = contract_ctx.clone();
        assert_eq!(contract_ctx.call_value, contract_ctx_clone.call_value);
    }
}
