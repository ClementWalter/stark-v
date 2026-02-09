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
//! let (result, _state) = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
//! assert!(result.success);
//! ```

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::{vec, vec::Vec};

#[cfg(not(target_arch = "riscv32"))]
use std::collections::BTreeSet;

#[cfg(target_arch = "riscv32")]
use alloc::collections::BTreeSet;

#[cfg(test)]
use crate::evm::CallKind;
use crate::evm::host::{Host, NullHost};
use crate::evm::memory::Memory;
use crate::evm::opcode_gas_cost;
use crate::evm::opcodes::arithmetic::EvmError as OpcodeError;
use crate::evm::opcodes::exec::{self, ExecContext, StepOutcome};
use crate::evm::stack::Stack;
#[cfg(feature = "evm-trace")]
use crate::evm::trace::{GasTraceEvent, GasTracer, StorageWrite, opcode_name};
use crate::state::State;
use crate::types::{Address, Hash, U256};

/// Re-export EvmError for public API
pub use crate::evm::error::EvmError;

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
    /// Gas refund accumulated during execution (from SSTORE clearing storage)
    pub gas_refund: u64,
    /// Return data (from RETURN or REVERT)
    pub return_data: Vec<u8>,
    /// Logs emitted during execution
    pub logs: Vec<LogEntry>,
    /// Final stack state (for debugging)
    pub stack: Stack,
    /// Final memory state (for debugging)
    pub memory: Memory,
    /// Optional gas trace (available when tracing is enabled)
    pub gas_trace: Option<crate::evm::trace::GasTrace>,
}

/// Log entry emitted by LOG0-LOG4 opcodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub address: Address,
    pub topics: Vec<Hash>,
    pub data: Vec<u8>,
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
    gas_refund: u64, // Accumulated gas refund (from SSTORE clearing storage)
    pc: usize,
    code: Vec<u8>,
    stopped: bool,
    return_data: Vec<u8>,
    block_ctx: BlockContext,
    tx_ctx: TxContext,
    call_ctx: CallContext,
    jumpdests: Vec<bool>, // Valid JUMPDEST positions
    logs: Vec<LogEntry>,
    state: S, // State interface for account/storage access
    host: H,  // Host interface for calls/creates
    // EIP-2929: Warm/cold access tracking
    accessed_addresses: BTreeSet<Address>,
    accessed_storage: BTreeSet<(Address, U256)>,
    // Gas tracing (enabled with evm-trace feature)
    #[cfg(feature = "evm-trace")]
    tracer: Option<GasTracer>,
    #[cfg(feature = "evm-trace")]
    pending_storage_write: Option<StorageWrite>,
}

impl<S: State, H: Host<S>> Evm<S, H> {
    /// Create a new EVM instance
    pub fn new(code: Vec<u8>, gas_limit: u64, state: S, host: H) -> Self {
        let jumpdests = Self::analyze_jumpdests(&code);
        Evm {
            stack: Stack::new(),
            memory: Memory::new(),
            gas_remaining: gas_limit,
            gas_refund: 0,
            pc: 0,
            code,
            stopped: false,
            return_data: Vec::new(),
            block_ctx: BlockContext::default(),
            tx_ctx: TxContext::default(),
            call_ctx: CallContext::default(),
            jumpdests,
            logs: Vec::new(),
            state,
            host,
            accessed_addresses: BTreeSet::new(),
            accessed_storage: BTreeSet::new(),
            #[cfg(feature = "evm-trace")]
            tracer: None,
            #[cfg(feature = "evm-trace")]
            pending_storage_write: None,
        }
    }

    /// Set the block context (for BLOCKHASH, TIMESTAMP, etc.)
    pub fn with_block_context(mut self, block_ctx: BlockContext) -> Self {
        self.block_ctx = block_ctx;
        self
    }

    /// Set the transaction context (for ORIGIN, GASPRICE)
    pub fn with_tx_context(mut self, tx_ctx: TxContext) -> Self {
        self.tx_ctx = tx_ctx;
        self
    }

    /// Set the call context (for ADDRESS, CALLER, CALLVALUE, CALLDATALOAD, etc.)
    pub fn with_call_context(mut self, call_ctx: CallContext) -> Self {
        self.call_ctx = call_ctx;
        self
    }

    /// Consume the EVM and return the final state
    pub fn into_state(self) -> S {
        self.state
    }

    /// Pre-warm addresses (for transaction sender, recipient, precompiles)
    pub fn warm_addresses(mut self, addresses: &[Address]) -> Self {
        for addr in addresses {
            self.accessed_addresses.insert(*addr);
        }
        self
    }

    /// Pre-warm a storage slot (EIP-2929; for access list)
    pub fn warm_storage(&mut self, address: &Address, key: &U256) {
        self.accessed_storage.insert((*address, *key));
    }

    /// Enable gas tracing (only available with evm-trace feature)
    #[cfg(feature = "evm-trace")]
    pub fn with_tracing(mut self) -> Self {
        self.tracer = Some(GasTracer::new(self.gas_remaining));
        self
    }

    /// Get the gas tracer (only available with evm-trace feature)
    #[cfg(feature = "evm-trace")]
    pub fn tracer(&self) -> Option<&GasTracer> {
        self.tracer.as_ref()
    }

    /// Analyze code to find valid JUMPDEST positions (uses opcodes::control).
    fn analyze_jumpdests(code: &[u8]) -> Vec<bool> {
        crate::evm::opcodes::control::analyze_jumpdests_bitmap(code)
    }

    /// Consume gas
    fn consume_gas(&mut self, amount: u64) -> Result<(), EvmError> {
        if self.gas_remaining < amount {
            return Err(EvmError::OutOfGas);
        }
        self.gas_remaining -= amount;
        Ok(())
    }

    /// Get current opcode
    fn current_opcode(&self) -> Result<u8, EvmError> {
        if self.pc >= self.code.len() {
            return Err(EvmError::PcOutOfBounds);
        }
        Ok(self.code[self.pc])
    }

    /// Execute a single step
    pub fn step(&mut self) -> Result<(), EvmError> {
        if self.stopped || self.pc >= self.code.len() {
            self.stopped = true;
            return Ok(());
        }

        let opcode = self.current_opcode()?;

        #[cfg(feature = "evm-trace")]
        let pc = self.pc;
        #[cfg(feature = "evm-trace")]
        let gas_before = self.gas_remaining;

        // Charge base gas
        let base_gas = opcode_gas_cost(opcode);
        self.consume_gas(base_gas)?;

        // Delegate to opcode dispatcher (exec)
        let mut exec_ctx = ExecContext {
            stack: &mut self.stack,
            memory: &mut self.memory,
            state: &mut self.state,
            host: &mut self.host,
            code: self.code.as_slice(),
            pc: self.pc,
            gas_remaining: &mut self.gas_remaining,
            gas_refund: &mut self.gas_refund,
            return_data: &mut self.return_data,
            jumpdests: self.jumpdests.as_slice(),
            accessed_addresses: &mut self.accessed_addresses,
            accessed_storage: &mut self.accessed_storage,
            block_number: self.block_ctx.number,
            block_timestamp: self.block_ctx.timestamp,
            block_coinbase: self.block_ctx.coinbase,
            block_difficulty: self.block_ctx.difficulty,
            block_gas_limit: self.block_ctx.gas_limit,
            block_chain_id: self.block_ctx.chain_id,
            block_base_fee: self.block_ctx.base_fee,
            tx_origin: self.tx_ctx.origin,
            tx_gas_price: self.tx_ctx.gas_price,
            call_address: self.call_ctx.address,
            call_caller: self.call_ctx.caller,
            call_value: self.call_ctx.call_value,
            call_data: self.call_ctx.call_data.as_slice(),
        };
        let (outcome, _sstore_trace) = exec::execute_opcode(opcode, &mut exec_ctx)?;
        // Record (address, key) for EIP-2929 so the next access to the same slot is warm.
        if let Some((addr, k, _)) = _sstore_trace {
            self.accessed_storage.insert((addr, k));
        }
        #[cfg(feature = "evm-trace")]
        {
            self.pending_storage_write = _sstore_trace.map(|(address, key, value)| StorageWrite {
                address,
                key,
                value,
            });
        }
        match outcome {
            StepOutcome::Continue(adv) => self.pc += adv as usize,
            StepOutcome::Stop => {
                self.stopped = true;
                self.pc += 1;
            }
            StepOutcome::Return(data) => {
                self.return_data = data;
                self.stopped = true;
                self.pc += 1;
            }
            StepOutcome::Revert(data) => return Err(EvmError::Revert(data)),
            StepOutcome::Jump(new_pc) => self.pc = new_pc,
            StepOutcome::Log(address, topics, data) => {
                self.logs.push(LogEntry {
                    address,
                    topics,
                    data,
                });
                self.pc += 1;
            }
        }

        // Record gas trace (if tracing enabled)
        #[cfg(feature = "evm-trace")]
        {
            let gas_after = self.gas_remaining;
            let gas_cost = gas_before.saturating_sub(gas_after);
            if let Some(ref mut tracer) = self.tracer {
                tracer.trace(GasTraceEvent {
                    pc,
                    opcode,
                    name: opcode_name(opcode),
                    gas_before,
                    gas_cost,
                    gas_after,
                    storage_write: self.pending_storage_write.take(),
                });
            }
        }

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
            gas_refund: self.gas_refund,
            return_data: self.return_data.clone(),
            logs: self.logs.clone(),
            stack: self.stack.clone(),
            memory: self.memory.clone(),
            gas_trace: {
                #[cfg(feature = "evm-trace")]
                {
                    self.tracer.as_ref().map(|tracer| tracer.snapshot())
                }
                #[cfg(not(feature = "evm-trace"))]
                {
                    None
                }
            },
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
/// let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
/// assert!(result.success);
/// ```
pub fn execute_bytecode<S: State>(
    code: &[u8],
    gas_limit: u64,
    state: S,
) -> Result<(ExecutionResult, S), EvmError> {
    execute_bytecode_with_host(code, gas_limit, state, NullHost)
}

/// Execute bytecode with a custom host implementation.
pub fn execute_bytecode_with_host<S: State, H: Host<S>>(
    code: &[u8],
    gas_limit: u64,
    state: S,
    host: H,
) -> Result<(ExecutionResult, S), EvmError> {
    execute_bytecode_with_host_and_contexts(
        code,
        gas_limit,
        state,
        host,
        BlockContext::default(),
        TxContext::default(),
        CallContext::default(),
    )
}

/// Execute bytecode with a custom host and explicit contexts.
///
/// This is a convenience wrapper around `execute_bytecode_with_host_contexts_and_access_list`
/// with an empty access list.
pub fn execute_bytecode_with_host_and_contexts<S: State, H: Host<S>>(
    code: &[u8],
    gas_limit: u64,
    state: S,
    host: H,
    block_ctx: BlockContext,
    tx_ctx: TxContext,
    call_ctx: CallContext,
) -> Result<(ExecutionResult, S), EvmError> {
    execute_bytecode_with_host_contexts_and_access_list(
        code,
        gas_limit,
        state,
        host,
        block_ctx,
        tx_ctx,
        call_ctx,
        &[],
    )
}

/// Execute bytecode with a custom host, explicit contexts, and access list (EIP-2930).
#[allow(clippy::too_many_arguments)]
pub fn execute_bytecode_with_host_contexts_and_access_list<S: State, H: Host<S>>(
    code: &[u8],
    gas_limit: u64,
    state: S,
    host: H,
    block_ctx: BlockContext,
    tx_ctx: TxContext,
    call_ctx: CallContext,
    access_list: &[(Address, Vec<U256>)],
) -> Result<(ExecutionResult, S), EvmError> {
    // EIP-2929: Pre-warm addresses
    // Warm the sender (origin), recipient (to), and precompiles (0x01-0x0a)
    #[cfg(not(target_arch = "riscv32"))]
    let mut warm_addresses = Vec::new();
    #[cfg(target_arch = "riscv32")]
    let mut warm_addresses = alloc::vec::Vec::new();

    warm_addresses.push(tx_ctx.origin); // Sender
    warm_addresses.push(call_ctx.address); // Recipient/contract being called
    warm_addresses.push(block_ctx.coinbase); // EIP-3651: Warm COINBASE
    // Precompile addresses (0x01-0x0a for Prague)
    for i in 1..=10 {
        let mut addr_bytes = [0u8; 20];
        addr_bytes[19] = i;
        warm_addresses.push(Address::from_slice(&addr_bytes).unwrap());
    }

    // Add access list addresses
    for (addr, _) in access_list {
        warm_addresses.push(*addr);
    }

    #[cfg(not(feature = "evm-trace"))]
    let mut evm = Evm::new(code.to_vec(), gas_limit, state, host)
        .with_block_context(block_ctx)
        .with_tx_context(tx_ctx)
        .with_call_context(call_ctx)
        .warm_addresses(&warm_addresses);

    // Enable gas tracing if feature is enabled
    #[cfg(feature = "evm-trace")]
    let mut evm = Evm::new(code.to_vec(), gas_limit, state, host)
        .with_block_context(block_ctx)
        .with_tx_context(tx_ctx)
        .with_call_context(call_ctx)
        .warm_addresses(&warm_addresses)
        .with_tracing();

    // Warm access list storage slots
    for (addr, keys) in access_list {
        for key in keys {
            evm.warm_storage(addr, key);
        }
    }

    let result = evm.run()?;
    Ok((result, evm.state))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evm::RecursiveHost;
    use crate::evm::host::{CallMessage, CallResult, CreateMessage, CreateResult};
    use crate::evm::memory::MemoryError;
    use crate::evm::stack::StackError;
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
            inner.blockhash.filter(|_| *number != U256::ZERO)
        }

        fn blobhash(&self, index: &U256) -> Option<Hash> {
            let inner = self.borrow();
            inner.blobhash.filter(|_| *index != U256::ZERO)
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
        let (result, _state) = execute_bytecode(&[], 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.gas_used, 0);
    }

    #[test]
    fn test_stop() {
        let code = vec![0x00]; // STOP
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
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
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(5));
    }

    #[test]
    fn test_mul() {
        // PUSH1 0x03 PUSH1 0x04 MUL STOP
        let code = vec![0x60, 0x03, 0x60, 0x04, 0x02, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(12));
    }

    #[test]
    fn test_sub() {
        // PUSH1 0x05 PUSH1 0x03 SUB STOP (5 - 3 = 2)
        let code = vec![0x60, 0x05, 0x60, 0x03, 0x03, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(2));
    }

    #[test]
    fn test_div() {
        // PUSH1 0x08 PUSH1 0x02 DIV STOP (8 / 2 = 4)
        let code = vec![0x60, 0x08, 0x60, 0x02, 0x04, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
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
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.len(), 0);
    }

    #[test]
    fn test_dup1() {
        // PUSH1 0x42 DUP1 STOP
        let code = vec![0x60, 0x42, 0x80, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.len(), 2);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x42));
        assert_eq!(result.stack.peek(1).unwrap(), &U256::from_u64(0x42));
    }

    #[test]
    fn test_swap1() {
        // PUSH1 0x01 PUSH1 0x02 SWAP1 STOP
        let code = vec![0x60, 0x01, 0x60, 0x02, 0x90, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
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
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x42));
    }

    #[test]
    fn test_mstore8() {
        // PUSH1 0xFF PUSH1 0x00 MSTORE8 PUSH1 0x00 MLOAD STOP
        let code = vec![0x60, 0xFF, 0x60, 0x00, 0x53, 0x60, 0x00, 0x51, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
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
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x42));
    }

    #[test]
    fn test_jumpi_taken() {
        // PUSH1 0x01 PUSH1 0x06 JUMPI INVALID JUMPDEST PUSH1 0x42 STOP
        let code = vec![0x60, 0x01, 0x60, 0x06, 0x57, 0xFE, 0x5B, 0x60, 0x42, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x42));
    }

    #[test]
    fn test_jumpi_not_taken() {
        // PUSH1 0x00 PUSH1 0x06 JUMPI PUSH1 0x99 STOP
        let code = vec![0x60, 0x00, 0x60, 0x06, 0x57, 0x60, 0x99, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
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
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.gas_used, 3);
    }

    #[test]
    fn test_balance_warm_refund() {
        let target = Address::from([0x11; 20]);
        let origin = Address::from([0x22; 20]);
        let call_address = Address::from([0x33; 20]);

        let mut code = Vec::new();
        code.push(0x73); // PUSH20
        code.extend_from_slice(&target.to_bytes());
        code.push(0x31); // BALANCE (cold)
        code.push(0x73); // PUSH20
        code.extend_from_slice(&target.to_bytes());
        code.push(0x31); // BALANCE (warm)
        code.push(0x00); // STOP

        let mut state = InMemoryState::new();
        state.set_balance(&target, U256::from_u64(1));

        let block_ctx = BlockContext::default();
        let tx_ctx = TxContext {
            origin,
            gas_price: U256::ONE,
        };
        let call_ctx = CallContext {
            address: call_address,
            caller: origin,
            call_value: U256::ZERO,
            call_data: Vec::new(),
        };

        let (result, _state) = execute_bytecode_with_host_and_contexts(
            &code, 10_000, state, NullHost, block_ctx, tx_ctx, call_ctx,
        )
        .unwrap();

        assert!(result.success);
        // Gas: PUSH20 (3) + BALANCE cold (2600) + PUSH20 (3) + BALANCE warm (100)
        assert_eq!(result.gas_used, 2706);
    }

    #[test]
    fn test_sstore_cold_warm_gas() {
        // Test SSTORE cold then warm access gas costs (EIP-2929)
        // PUSH1 1 PUSH1 0 SSTORE  (cold)
        // PUSH1 2 PUSH1 0 SSTORE  (warm - same key)
        // STOP
        let code = vec![
            0x60, 0x01, // PUSH1 1 (value)
            0x60, 0x00, // PUSH1 0 (key)
            0x55, // SSTORE (cold: 20000 + 2100 = 22100)
            0x60, 0x02, // PUSH1 2 (value)
            0x60, 0x00, // PUSH1 0 (key)
            0x55, // SSTORE (warm: 20000 + 100 = 20100)
            0x00, // STOP
        ];

        let (result, _state) = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
        // Gas: PUSH1(3) + PUSH1(3) + SSTORE cold SET(20000+2100=22100)
        //      PUSH1(3) + PUSH1(3) + SSTORE warm RESET(2900+100=3000) + STOP(0)
        // Total: 3 + 3 + 22100 + 3 + 3 + 3000 + 0 = 25112
        assert_eq!(result.gas_used, 25112);
    }

    // =============================================================================
    // Return/Revert Tests
    // =============================================================================

    #[test]
    fn test_return_empty() {
        // PUSH1 0x00 PUSH1 0x00 RETURN
        let code = vec![0x60, 0x00, 0x60, 0x00, 0xF3];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
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
            0x01, // ADD (result: 2)
            0x80, // DUP1 (stack: 2, 2)
            0x01, // ADD (result: 4)
            0x80, // DUP1 (stack: 4, 4)
            0x01, // ADD (result: 8)
            0x00, // STOP
        ];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(8));
    }

    #[test]
    fn test_memory_expansion_cost() {
        // Store at high memory offset
        // PUSH1 0x42 PUSH1 0xFF MSTORE STOP
        let code = vec![0x60, 0x42, 0x60, 0xFF, 0x52, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert!(result.gas_used > 6); // Base gas + memory expansion
    }

    #[test]
    fn test_codecopy() {
        // Copy 3 bytes of code to memory at offset 0
        // PUSH1 0x03 PUSH1 0x00 PUSH1 0x00 CODECOPY PUSH1 0x00 MLOAD STOP
        let code = vec![
            0x60, 0x03, 0x60, 0x00, 0x60, 0x00, 0x39, 0x60, 0x00, 0x51, 0x00,
        ];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
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
            &crate::evm::opcodes::utils::address_to_u256(&address)
        );
        assert_eq!(
            result.stack.peek(1).unwrap(),
            &crate::evm::opcodes::utils::address_to_u256(&caller)
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
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_gas_opcode() {
        // GAS STOP
        let code = vec![0x5A, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        // Gas should be less than initial (2 gas consumed)
        let remaining = result.stack.peek(0).unwrap();
        assert_eq!(*remaining, U256::from_u64(998));
    }

    #[test]
    fn test_msize() {
        // PUSH1 0x42 PUSH1 0x40 MSTORE MSIZE STOP
        let code = vec![0x60, 0x42, 0x60, 0x40, 0x52, 0x59, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
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
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ONE);
    }

    #[test]
    fn test_bitwise_operations() {
        // PUSH1 0x0F PUSH1 0xF0 AND STOP (0xF0 & 0x0F = 0)
        let code = vec![0x60, 0x0F, 0x60, 0xF0, 0x16, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_push0() {
        // PUSH0 STOP
        let code = vec![0x5F, 0x00];
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_push32() {
        // PUSH32 0xFF..FF STOP
        let mut code = vec![0x7F];
        code.extend_from_slice(&[0xFF; 32]);
        code.push(0x00);
        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
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

        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
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

        let (result, _state) = execute_bytecode(&code, 1000, InMemoryState::new()).unwrap();
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
        assert!(matches!(
            result,
            Err(EvmError::StackError(StackError::Overflow))
        ));
    }

    #[test]
    fn test_stack_underflow() {
        // Try to POP from empty stack
        let code = vec![0x50]; // POP
        let result = execute_bytecode(&code, 1000, InMemoryState::new());
        assert!(matches!(
            result,
            Err(EvmError::StackError(StackError::Underflow))
        ));
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
            0x60, 0x42, // PUSH1 0x42
            0x60, 0x00, // PUSH1 0x00
            0x52, // MSTORE
            0x60, 0x20, // PUSH1 0x20 (size)
            0x60, 0x00, // PUSH1 0x00 (offset)
            0xF3, // RETURN
        ];
        let (result, _state) = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.return_data.len(), 32);
        assert_eq!(result.return_data[31], 0x42);
    }

    #[test]
    fn test_conditional_logic() {
        // If-else using JUMPI: if x > 5 then push 1 else push 0
        // PUSH1 0x06 PUSH1 0x05 GT PUSH1 0x0D JUMPI PUSH1 0x00 PUSH1 0x11 JUMP JUMPDEST PUSH1 0x01 JUMPDEST STOP
        let code = vec![
            0x60, 0x06, // PUSH1 6
            0x60, 0x05, // PUSH1 5
            0x11, // GT (6 > 5 = true)
            0x60, 0x0D, // PUSH1 13 (jump target)
            0x57, // JUMPI
            0x60, 0x00, // PUSH1 0 (else branch)
            0x60, 0x11, // PUSH1 17 (skip to end)
            0x56, // JUMP
            0x5B, // JUMPDEST (offset 13)
            0x60, 0x01, // PUSH1 1 (then branch)
            0x5B, // JUMPDEST (offset 17)
            0x00, // STOP
        ];
        let (result, _state) = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ONE);
    }

    #[test]
    fn test_loop_simulation() {
        // Simple counter loop: push 3 values and add them
        // PUSH1 1 PUSH1 2 PUSH1 3 ADD ADD STOP
        let code = vec![
            0x60, 0x01, // PUSH1 1
            0x60, 0x02, // PUSH1 2
            0x60, 0x03, // PUSH1 3
            0x01, // ADD (2+3=5)
            0x01, // ADD (1+5=6)
            0x00, // STOP
        ];
        let (result, _state) = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(6));
    }

    #[test]
    fn test_mcopy_operation() {
        // MCOPY: Copy memory region
        // Store 0x42 at offset 0, then copy to offset 32
        // PUSH1 0x42 PUSH1 0x00 MSTORE PUSH1 0x20 PUSH1 0x00 PUSH1 0x20 MCOPY PUSH1 0x20 MLOAD STOP
        let code = vec![
            0x60, 0x42, // PUSH1 0x42
            0x60, 0x00, // PUSH1 0x00
            0x52, // MSTORE
            0x60, 0x20, // PUSH1 32 (size)
            0x60, 0x00, // PUSH1 0 (src)
            0x60, 0x20, // PUSH1 32 (dest)
            0x5E, // MCOPY
            0x60, 0x20, // PUSH1 32
            0x51, // MLOAD
            0x00, // STOP
        ];
        let (result, _state) = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x42));
    }

    #[test]
    fn test_all_push_sizes() {
        // Test PUSH1, PUSH2, PUSH4, etc.
        // PUSH2 0x1234 PUSH1 0x56 ADD STOP
        let code = vec![
            0x61, 0x12, 0x34, // PUSH2 0x1234
            0x60, 0x56, // PUSH1 0x56
            0x01, // ADD
            0x00, // STOP
        ];
        let (result, _state) = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
        assert!(result.success);
        assert_eq!(
            result.stack.peek(0).unwrap(),
            &U256::from_u64(0x1234 + 0x56)
        );
    }

    #[test]
    fn test_complete_opcode_coverage() {
        // Test coverage of major opcode categories
        let mut code = Vec::new();

        // Arithmetic
        code.extend_from_slice(&[0x60, 0x0A]); // PUSH1 10
        code.extend_from_slice(&[0x60, 0x05]); // PUSH1 5
        code.push(0x01); // ADD
        code.push(0x60);
        code.push(0x02); // PUSH1 2
        code.push(0x02); // MUL
        code.push(0x60);
        code.push(0x03); // PUSH1 3
        code.push(0x04); // DIV

        // Bitwise
        code.push(0x60);
        code.push(0xFF); // PUSH1 0xFF
        code.push(0x16); // AND

        // Comparison
        code.push(0x60);
        code.push(0x00); // PUSH1 0
        code.push(0x14); // EQ

        // Stack manipulation
        code.push(0x80); // DUP1
        code.push(0x50); // POP

        code.push(0x00); // STOP

        let (result, _state) = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();
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

        let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
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

        let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
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

        let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
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
            0x73, // PUSH20
        ];
        code.extend_from_slice(&test_addr.to_bytes());
        code.extend_from_slice(&[
            0x3C, // EXTCODECOPY
            0x60, 0x00, // PUSH1 0
            0x51, // MLOAD
            0x00, // STOP
        ]);

        let mut state = InMemoryState::new();
        state.set_code(&test_addr, vec![0xDE, 0xAD, 0xBE, 0xEF, 0x99]);

        let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
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

        let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
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
            0x55, // SSTORE
            0x60, 0x42, // PUSH1 0x42 (key)
            0x54, // SLOAD
            0x00, // STOP
        ];

        let state = InMemoryState::new();
        let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x99));
    }

    #[test]
    fn test_sload_uninitialized() {
        // PUSH1 0xFF SLOAD STOP
        let code = vec![0x60, 0xFF, 0x54, 0x00]; // PUSH1 0xFF SLOAD STOP

        let state = InMemoryState::new();
        let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::ZERO);
    }

    #[test]
    fn test_tload_tstore_opcodes() {
        // PUSH1 0x77 PUSH1 0x33 TSTORE PUSH1 0x33 TLOAD STOP
        let code = vec![
            0x60, 0x77, // PUSH1 0x77 (value)
            0x60, 0x33, // PUSH1 0x33 (key)
            0x5D, // TSTORE
            0x60, 0x33, // PUSH1 0x33 (key)
            0x5C, // TLOAD
            0x00, // STOP
        ];

        let state = InMemoryState::new();
        let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(0x77));
    }

    #[test]
    fn test_tload_uninitialized() {
        // PUSH1 0xAA TLOAD STOP
        let code = vec![0x60, 0xAA, 0x5C, 0x00]; // PUSH1 0xAA TLOAD STOP

        let state = InMemoryState::new();
        let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
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
            0x55, // SSTORE
            0x60, 0x22, // PUSH1 0x22
            0x60, 0x02, // PUSH1 0x02 (key)
            0x55, // SSTORE
            0x60, 0x01, // PUSH1 0x01 (key)
            0x54, // SLOAD
            0x60, 0x02, // PUSH1 0x02 (key)
            0x54, // SLOAD
            0x00, // STOP
        ];

        let state = InMemoryState::new();
        let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
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
            0x5D, // TSTORE (transient)
            0x60, 0xBB, // PUSH1 0xBB
            0x60, 0x10, // PUSH1 0x10 (key)
            0x55, // SSTORE (permanent)
            0x60, 0x10, // PUSH1 0x10 (key)
            0x5C, // TLOAD (transient)
            0x60, 0x10, // PUSH1 0x10 (key)
            0x54, // SLOAD (permanent)
            0x00, // STOP
        ];

        let state = InMemoryState::new();
        let (result, _state) = execute_bytecode(&code, 100000, state).unwrap();
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
            0x40, // BLOCKHASH
            0x60, 0x02, // PUSH1 0x02
            0x49, // BLOBHASH
            0x4A, // BLOBBASEFEE
            0x00, // STOP
        ];

        let (result, _state) =
            execute_bytecode_with_host(&code, 100000, InMemoryState::new(), host).unwrap();
        assert!(result.success);
        assert_eq!(result.stack.peek(0).unwrap(), &U256::from_u64(7));
        assert_eq!(
            result.stack.peek(1).unwrap(),
            &crate::evm::opcodes::utils::hash_to_u256(&blob_hash)
        );
        assert_eq!(
            result.stack.peek(2).unwrap(),
            &crate::evm::opcodes::utils::hash_to_u256(&block_hash)
        );
    }

    #[test]
    fn test_call_opcode_invokes_host_and_writes_output() {
        let host_state = TestHost {
            call_result: CallResult {
                success: true,
                return_data: vec![0xAA, 0xBB],
                gas_used: 5,
            },
            ..Default::default()
        };
        let host = Rc::new(RefCell::new(host_state));

        let mut to_bytes = [0u8; 20];
        to_bytes[19] = 0x01;
        let to = Address::from_slice(&to_bytes).unwrap();

        let mut code = vec![
            0x60, 0x01, // PUSH1 0x01
            0x60, 0x00, // PUSH1 0x00
            0x53, // MSTORE8 (memory[0] = 0x01)
            0x60, 0x03, // PUSH1 0x03 (out_size)
            0x60, 0x20, // PUSH1 0x20 (out_offset)
            0x60, 0x01, // PUSH1 0x01 (in_size)
            0x60, 0x00, // PUSH1 0x00 (in_offset)
            0x60, 0x00, // PUSH1 0x00 (value)
            0x73, // PUSH20
        ];
        code.extend_from_slice(&to.to_bytes());
        code.extend_from_slice(&[
            0x60, 0x10, // PUSH1 0x10 (gas)
            0xF1, // CALL
            0x00, // STOP
        ]);

        let (result, _state) =
            execute_bytecode_with_host(&code, 100000, InMemoryState::new(), host.clone()).unwrap();
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
            0x53, // MSTORE8 (init code byte)
            0x61, 0x12, 0x34, // PUSH2 0x1234 (salt)
            0x60, 0x01, // PUSH1 0x01 (size)
            0x60, 0x00, // PUSH1 0x00 (offset)
            0x60, 0x00, // PUSH1 0x00 (value)
            0xF5, // CREATE2
            0x00, // STOP
        ];

        let (result, _state) =
            execute_bytecode_with_host(&code, 100000, InMemoryState::new(), host.clone()).unwrap();
        assert!(result.success);
        assert_eq!(
            result.stack.peek(0).unwrap(),
            &crate::evm::opcodes::utils::address_to_u256(&created_addr)
        );

        let recorded = host.borrow().last_create.clone().unwrap();
        assert_eq!(recorded.salt, Some(U256::from_u64(0x1234)));
        assert_eq!(recorded.init_code, vec![0xAA]);
    }

    #[test]
    fn test_delegatecall_uses_caller_and_value_from_context() {
        let host_state = TestHost {
            call_result: CallResult {
                success: true,
                return_data: Vec::new(),
                gas_used: 1,
            },
            ..Default::default()
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
            0x73, // PUSH20
        ];
        code.extend_from_slice(&to.to_bytes());
        code.extend_from_slice(&[
            0x60, 0x20, // PUSH1 0x20 (gas)
            0xF4, // DELEGATECALL
            0x00, // STOP
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

    #[test]
    fn test_delegatecall_writes_to_caller_storage() {
        let caller = Address::new([0x11; 20]);
        let callee = Address::new([0x22; 20]);
        let sender = Address::new([0x33; 20]);

        let callee_code = vec![
            0x60, 0xAA, // PUSH1 0xAA (value)
            0x60, 0x01, // PUSH1 0x01 (key)
            0x55, // SSTORE
            0x00, // STOP
        ];

        let mut caller_code = vec![
            0x60, 0x00, // PUSH1 0x00 (out_size)
            0x60, 0x00, // PUSH1 0x00 (out_offset)
            0x60, 0x00, // PUSH1 0x00 (in_size)
            0x60, 0x00, // PUSH1 0x00 (in_offset)
            0x73, // PUSH20
        ];
        caller_code.extend_from_slice(&callee.to_bytes());
        caller_code.extend_from_slice(&[
            0x61, 0xC3, 0x50, // PUSH2 0xC350 (gas = 50000)
            0xF4, // DELEGATECALL
            0x00, // STOP
        ]);

        let mut state = InMemoryState::new();
        state.set_code(&callee, callee_code);

        let call_ctx = CallContext {
            address: caller,
            caller: sender,
            call_value: U256::ZERO,
            call_data: Vec::new(),
        };

        let host = RecursiveHost::new();
        let (result, final_state) = execute_bytecode_with_host_and_contexts(
            &caller_code,
            100_000,
            state,
            host,
            BlockContext::default(),
            TxContext::default(),
            call_ctx,
        )
        .unwrap();

        assert!(result.success);
        assert_eq!(
            final_state.sload(&caller, &U256::from(1u64)),
            U256::from(0xAAu64)
        );
        assert_eq!(final_state.sload(&callee, &U256::from(1u64)), U256::ZERO);
    }

    #[test]
    fn test_log1_captures_data_and_topic() {
        let code = vec![
            0x60, 0xAA, // PUSH1 0xAA
            0x60, 0x00, // PUSH1 0x00
            0x53, // MSTORE8
            0x60, 0xBB, // PUSH1 0xBB
            0x60, 0x01, // PUSH1 0x01
            0x53, // MSTORE8
            0x60, 0x01, // PUSH1 topic
            0x60, 0x02, // PUSH1 size
            0x60, 0x00, // PUSH1 offset
            0xA1, // LOG1
            0x00, // STOP
        ];

        let (result, _state) = execute_bytecode(&code, 100000, InMemoryState::new()).unwrap();

        assert_eq!(result.logs.len(), 1);
        let log = &result.logs[0];
        assert_eq!(log.address, Address::ZERO);
        assert_eq!(log.data, vec![0xAA, 0xBB]);

        let mut topic_bytes = [0u8; 32];
        topic_bytes[31] = 1;
        assert_eq!(log.topics, vec![Hash::from(topic_bytes)]);
    }
}
