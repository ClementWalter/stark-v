//! Host interface for call/create operations and external data access.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

use crate::evm::gas::blob_gas_price;
use crate::state::State;
use crate::types::{Address, Hash, U256};

/// The kind of call being performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallKind {
    Call,
    CallCode,
    DelegateCall,
    StaticCall,
}

/// Call message passed to the host for execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallMessage {
    pub kind: CallKind,
    /// Gas forwarded to the callee.
    pub gas: u64,
    /// The address whose storage/context is used for the call.
    pub address: Address,
    /// The caller (msg.sender) as seen by the callee.
    pub caller: Address,
    /// The value (msg.value) as seen by the callee.
    pub value: U256,
    /// The address whose code is executed.
    pub code_address: Address,
    /// Call input data.
    pub input: Vec<u8>,
    /// Whether the call is static.
    pub is_static: bool,
}

/// Result of a call execution.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CallResult {
    pub success: bool,
    pub return_data: Vec<u8>,
    /// Gas used by the callee.
    pub gas_used: u64,
}

/// Create message for CREATE/CREATE2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateMessage {
    /// Gas forwarded to init code execution.
    pub gas: u64,
    pub caller: Address,
    pub value: U256,
    pub init_code: Vec<u8>,
    /// Salt for CREATE2 (None for CREATE).
    pub salt: Option<U256>,
}

/// Result of a CREATE/CREATE2 operation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreateResult {
    pub success: bool,
    pub address: Option<Address>,
    pub return_data: Vec<u8>,
    /// Gas used by init code execution (including code deposit if applicable).
    pub gas_used: u64,
}


/// Host interface for external calls and block/tx data access.
pub trait Host<S: State> {
    fn call(&mut self, state: &mut S, msg: CallMessage) -> CallResult;
    fn create(&mut self, state: &mut S, msg: CreateMessage) -> CreateResult;
    fn blockhash(&self, number: &U256) -> Option<Hash>;
    fn blobhash(&self, index: &U256) -> Option<Hash>;
    fn blobbasefee(&self) -> U256;
}

/// Default host that performs no external execution.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullHost;

impl<S: State> Host<S> for NullHost {
    fn call(&mut self, _state: &mut S, _msg: CallMessage) -> CallResult {
        CallResult {
            success: false,
            return_data: Vec::new(),
            gas_used: 0,
        }
    }

    fn create(&mut self, _state: &mut S, _msg: CreateMessage) -> CreateResult {
        CreateResult {
            success: false,
            address: None,
            return_data: Vec::new(),
            gas_used: 0,
        }
    }

    fn blockhash(&self, _number: &U256) -> Option<Hash> {
        None
    }

    fn blobhash(&self, _index: &U256) -> Option<Hash> {
        None
    }

    fn blobbasefee(&self) -> U256 {
        U256::ZERO
    }
}

/// Recursive host that executes calls/creates by spawning new EVM instances.
///
/// This host enables full contract-to-contract call support.
/// CALL/DELEGATECALL/STATICCALL/CALLCODE and CREATE/CREATE2 opcodes
/// will recursively execute the target contract code.
#[derive(Debug, Clone)]
pub struct RecursiveHost {
    /// Maximum call depth (default: 1024 per EVM spec)
    pub max_depth: usize,
    /// Current call depth
    pub depth: usize,
    /// Block number for BLOCKHASH lookups
    pub block_number: u64,
    /// Parent block hash for BLOCKHASH lookups
    pub parent_hash: Option<Hash>,
    /// Recent block hashes in increasing block number order (oldest -> newest).
    pub recent_block_hashes: Vec<Hash>,
    /// Block context for environment opcodes
    pub block_ctx: crate::evm::interpreter::BlockContext,
    /// Transaction context for environment opcodes
    pub tx_ctx: crate::evm::interpreter::TxContext,
}

impl Default for RecursiveHost {
    fn default() -> Self {
        Self {
            max_depth: 1024,
            depth: 0,
            block_number: 0,
            parent_hash: None,
            recent_block_hashes: Vec::new(),
            block_ctx: crate::evm::interpreter::BlockContext::default(),
            tx_ctx: crate::evm::interpreter::TxContext::default(),
        }
    }
}

impl RecursiveHost {
    /// Create a new recursive host with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the block context for child executions.
    pub fn with_block_context(mut self, block_ctx: crate::evm::interpreter::BlockContext) -> Self {
        self.block_number = block_ctx.number.as_u64();
        self.block_ctx = block_ctx;
        self
    }

    /// Configure the parent block hash for BLOCKHASH lookups.
    pub fn with_parent_hash(mut self, parent_hash: Hash) -> Self {
        self.parent_hash = Some(parent_hash);
        self
    }

    /// Configure recent block hashes for BLOCKHASH lookups.
    pub fn with_recent_block_hashes(mut self, block_hashes: &[Hash]) -> Self {
        let mut hashes = block_hashes.to_vec();
        if hashes.len() > 256 {
            hashes.drain(0..hashes.len().saturating_sub(256));
        }
        self.recent_block_hashes = hashes;
        self
    }

    /// Configure the transaction context for child executions.
    pub fn with_tx_context(mut self, tx_ctx: crate::evm::interpreter::TxContext) -> Self {
        self.tx_ctx = tx_ctx;
        self
    }

    /// Create a child host with incremented depth.
    fn child(&self) -> Option<Self> {
        if self.depth >= self.max_depth {
            return None;
        }
        Some(Self {
            max_depth: self.max_depth,
            depth: self.depth + 1,
            block_number: self.block_number,
            parent_hash: self.parent_hash,
            recent_block_hashes: self.recent_block_hashes.clone(),
            block_ctx: self.block_ctx.clone(),
            tx_ctx: self.tx_ctx.clone(),
        })
    }
}

impl<S: State + Clone> Host<S> for RecursiveHost {
    fn call(&mut self, state: &mut S, msg: CallMessage) -> CallResult {
        // Check call depth
        let Some(child_host) = self.child() else {
            return CallResult {
                success: false,
                return_data: Vec::new(),
                gas_used: msg.gas,
            };
        };

        // Get contract code from code_address
        let code = state.get_code(&msg.code_address).to_vec();

        // If no code, handle value transfer and return
        if code.is_empty() {
            // For CALL and CALLCODE, transfer value from caller to address
            // For DELEGATECALL and STATICCALL, no value transfer (already U256::ZERO)
            if !msg.value.is_zero() {
                // Deduct value from caller
                let caller_balance = state.get_balance(&msg.caller);
                if caller_balance < msg.value {
                    // Insufficient balance
                    return CallResult {
                        success: false,
                        return_data: Vec::new(),
                        gas_used: 0,
                    };
                }
                state.set_balance(&msg.caller, caller_balance.saturating_sub(msg.value));

                // Add value to recipient
                let recipient_balance = state.get_balance(&msg.address);
                state.set_balance(&msg.address, recipient_balance.saturating_add(msg.value));
            }

            return CallResult {
                success: true,
                return_data: Vec::new(),
                gas_used: 0,
            };
        }

        // Clone state to avoid borrowing issues
        let mut call_state = state.clone();

        // Handle value transfer in the cloned state
        if !msg.value.is_zero() {
            // Check sufficient balance
            let caller_balance = call_state.get_balance(&msg.caller);
            if caller_balance < msg.value {
                // Insufficient balance
                return CallResult {
                    success: false,
                    return_data: Vec::new(),
                    gas_used: 0,
                };
            }

            // Deduct from caller
            call_state.set_balance(&msg.caller, caller_balance.saturating_sub(msg.value));

            // Add to recipient
            let recipient_balance = call_state.get_balance(&msg.address);
            call_state.set_balance(&msg.address, recipient_balance.saturating_add(msg.value));
        }

        // Build call context for the called contract
        use crate::evm::interpreter::{CallContext, Evm, EvmError};

        let call_ctx = CallContext {
            address: msg.address,
            caller: msg.caller,
            call_value: msg.value,
            call_data: msg.input.clone(),
        };

        // Create EVM with proper context
        let mut evm = Evm::new(code, msg.gas, call_state, child_host)
            .with_block_context(self.block_ctx.clone())
            .with_tx_context(self.tx_ctx.clone())
            .with_call_context(call_ctx);

        // Execute
        let result = evm.run();

        match result {
            Ok(exec_result) => {
                // Merge state changes back if call succeeded
                if exec_result.success {
                    *state = evm.into_state();
                } else {
                    // Discard failed call state (don't consume evm)
                    // But we need to consume it anyway for type system
                    let _ = evm.into_state();
                }
                CallResult {
                    success: exec_result.success,
                    return_data: exec_result.return_data,
                    gas_used: exec_result.gas_used,
                }
            }
            Err(EvmError::OutOfGas) => CallResult {
                success: false,
                return_data: Vec::new(),
                gas_used: msg.gas,
            },
            Err(_) => CallResult {
                success: false,
                return_data: Vec::new(),
                gas_used: msg.gas,
            },
        }
    }

    fn create(&mut self, state: &mut S, msg: CreateMessage) -> CreateResult {
        // Check call depth
        let Some(child_host) = self.child() else {
            return CreateResult {
                success: false,
                address: None,
                return_data: Vec::new(),
                gas_used: msg.gas,
            };
        };

        // Compute contract address
        let nonce = state.get_nonce(&msg.caller);
        let contract_address = if let Some(salt) = msg.salt {
            // CREATE2
            compute_create2_address(&msg.caller, &salt, &msg.init_code)
        } else {
            // CREATE
            compute_create_address(&msg.caller, nonce.saturating_sub(U256::ONE))
        };

        // Clone state
        let mut create_state = state.clone();

        // Handle value transfer in the cloned state
        if !msg.value.is_zero() {
            // Check sufficient balance
            let caller_balance = create_state.get_balance(&msg.caller);
            if caller_balance < msg.value {
                // Insufficient balance
                return CreateResult {
                    success: false,
                    address: None,
                    return_data: Vec::new(),
                    gas_used: 0,
                };
            }

            // Deduct from caller
            create_state.set_balance(&msg.caller, caller_balance.saturating_sub(msg.value));

            // Add to new contract address
            let contract_balance = create_state.get_balance(&contract_address);
            create_state.set_balance(&contract_address, contract_balance.saturating_add(msg.value));
        }

        // Build call context for the constructor
        use crate::evm::interpreter::{CallContext, Evm, EvmError};

        let call_ctx = CallContext {
            address: contract_address,
            caller: msg.caller,
            call_value: msg.value,
            call_data: Vec::new(), // Init code has no call data
        };

        // Create EVM with proper context
        let mut evm = Evm::new(msg.init_code.clone(), msg.gas, create_state, child_host)
            .with_block_context(self.block_ctx.clone())
            .with_tx_context(self.tx_ctx.clone())
            .with_call_context(call_ctx);

        // Execute
        let result = evm.run();

        match result {
            Ok(exec_result) => {
                if exec_result.success && !exec_result.return_data.is_empty() {
                    if exec_result.return_data[0] == 0xEF {
                        // EIP-3541: reject code starting with 0xEF and consume all gas.
                        let _ = evm.into_state();
                        return CreateResult {
                            success: false,
                            address: None,
                            return_data: Vec::new(),
                            gas_used: msg.gas,
                        };
                    }
                    // Deploy the contract code
                    let mut final_state = evm.into_state();
                    final_state.set_code(&contract_address, exec_result.return_data.clone());
                    *state = final_state;
                    CreateResult {
                        success: true,
                        address: Some(contract_address),
                        return_data: exec_result.return_data,
                        gas_used: exec_result.gas_used,
                    }
                } else {
                    // Discard failed create state
                    let _ = evm.into_state();
                    CreateResult {
                        success: false,
                        address: None,
                        return_data: exec_result.return_data,
                        gas_used: exec_result.gas_used,
                    }
                }
            }
            Err(EvmError::OutOfGas) => CreateResult {
                success: false,
                address: None,
                return_data: Vec::new(),
                gas_used: msg.gas,
            },
            Err(_) => CreateResult {
                success: false,
                address: None,
                return_data: Vec::new(),
                gas_used: msg.gas,
            },
        }
    }

    fn blockhash(&self, _number: &U256) -> Option<Hash> {
        let requested = _number.as_u64();
        if requested >= self.block_number {
            return None;
        }

        let distance = self.block_number - requested;
        if distance > 256 {
            return None;
        }

        if !self.recent_block_hashes.is_empty() {
            let index = self
                .recent_block_hashes
                .len()
                .checked_sub(distance as usize)?;
            return self.recent_block_hashes.get(index).copied();
        }

        if distance == 1 {
            return self.parent_hash;
        }

        None
    }

    fn blobhash(&self, _index: &U256) -> Option<Hash> {
        let max_index = U256::from_u64(usize::MAX as u64);
        if *_index > max_index {
            return None;
        }
        let index = _index.as_u64() as usize;
        self.tx_ctx.blob_versioned_hashes.get(index).copied()
    }

    fn blobbasefee(&self) -> U256 {
        match self.block_ctx.excess_blob_gas {
            Some(excess) => blob_gas_price(excess),
            None => U256::ZERO,
        }
    }
}

/// Compute CREATE address: keccak256(rlp([sender, nonce]))[12:]
pub fn compute_create_address(sender: &Address, nonce: U256) -> Address {
    use crate::crypto::{keccak256, rlp};

    let sender_bytes = rlp::encode_address(sender);
    let nonce_bytes = rlp::encode_u256(&nonce);
    let encoded = rlp::encode_list(&[sender_bytes, nonce_bytes]);
    let hash = keccak256(&encoded);

    let mut address = Address::ZERO;
    address.as_bytes_mut()[..].copy_from_slice(&hash.as_bytes()[12..]);
    address
}

/// Compute CREATE2 address: keccak256(0xff ++ sender ++ salt ++ keccak256(init_code))[12:]
pub fn compute_create2_address(sender: &Address, salt: &U256, init_code: &[u8]) -> Address {
    use crate::crypto::keccak256;

    let code_hash = keccak256(init_code);
    let salt_bytes = salt.to_be_bytes();

    let mut data = Vec::with_capacity(1 + 20 + 32 + 32);
    data.push(0xff);
    data.extend_from_slice(sender.as_bytes());
    data.extend_from_slice(&salt_bytes);
    data.extend_from_slice(code_hash.as_bytes());

    let hash = keccak256(&data);

    let mut address = Address::ZERO;
    address.as_bytes_mut()[..].copy_from_slice(&hash.as_bytes()[12..]);
    address
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evm::interpreter::{BlockContext, TxContext};
    use crate::state::InMemoryState;
    use crate::types::{Address, U256};

    #[test]
    fn test_recursive_host_blockhash_parent_only() {
        let parent_hash = Hash::from([0x11; 32]);
        let block_ctx = BlockContext {
            number: U256::from_u64(100),
            ..BlockContext::default()
        };

        let host = RecursiveHost::new()
            .with_block_context(block_ctx)
            .with_parent_hash(parent_hash);

        assert_eq!(
            Host::<InMemoryState>::blockhash(&host, &U256::from_u64(99)),
            Some(parent_hash)
        );
        assert_eq!(
            Host::<InMemoryState>::blockhash(&host, &U256::from_u64(100)),
            None
        );
        assert_eq!(
            Host::<InMemoryState>::blockhash(&host, &U256::from_u64(98)),
            None
        );
    }

    #[test]
    fn test_recursive_host_blockhash_recent_hashes() {
        let block_ctx = BlockContext {
            number: U256::from_u64(100),
            ..BlockContext::default()
        };
        let hash_98 = Hash::from([0x22; 32]);
        let hash_99 = Hash::from([0x33; 32]);
        let recent_hashes = vec![hash_98, hash_99];

        let host = RecursiveHost::new()
            .with_block_context(block_ctx)
            .with_recent_block_hashes(&recent_hashes);

        assert_eq!(
            Host::<InMemoryState>::blockhash(&host, &U256::from_u64(99)),
            Some(hash_99)
        );
        assert_eq!(
            Host::<InMemoryState>::blockhash(&host, &U256::from_u64(98)),
            Some(hash_98)
        );
        assert_eq!(
            Host::<InMemoryState>::blockhash(&host, &U256::from_u64(97)),
            None
        );
    }

    #[test]
    fn test_recursive_host_blobhash_from_tx_context() {
        let blob_hash_0 = Hash::from([0x10; 32]);
        let blob_hash_1 = Hash::from([0x20; 32]);
        let tx_ctx = TxContext {
            origin: Address::from([0x01; 20]),
            gas_price: U256::from_u64(1),
            blob_versioned_hashes: vec![blob_hash_0, blob_hash_1],
        };

        let host = RecursiveHost::new().with_tx_context(tx_ctx);

        assert_eq!(
            Host::<InMemoryState>::blobhash(&host, &U256::from_u64(0)),
            Some(blob_hash_0)
        );
        assert_eq!(
            Host::<InMemoryState>::blobhash(&host, &U256::from_u64(1)),
            Some(blob_hash_1)
        );
        assert_eq!(
            Host::<InMemoryState>::blobhash(&host, &U256::from_u64(2)),
            None
        );
    }

    #[test]
    fn test_recursive_host_create_rejects_0xef_prefix() {
        let mut state = InMemoryState::new();
        let caller = Address::from([0x01; 20]);
        state.set_balance(&caller, U256::from_u64(1_000_000));
        state.increment_nonce(&caller);

        let block_ctx = BlockContext::default();
        let tx_ctx = TxContext::default();
        let mut host = RecursiveHost::new()
            .with_block_context(block_ctx)
            .with_tx_context(tx_ctx);

        let init_code = vec![
            0x60, 0xEF, // PUSH1 0xEF
            0x60, 0x00, // PUSH1 0x00
            0x53,       // MSTORE8
            0x60, 0x01, // PUSH1 0x01
            0x60, 0x00, // PUSH1 0x00
            0xF3,       // RETURN
        ];

        let gas = 100000;
        let msg = CreateMessage {
            gas,
            caller,
            value: U256::ZERO,
            init_code,
            salt: None,
        };

        let result = host.create(&mut state, msg);

        assert!(!result.success);
        assert_eq!(result.gas_used, gas);
        assert!(result.return_data.is_empty());
        assert!(result.address.is_none());

        let expected_address = compute_create_address(&caller, U256::ZERO);
        assert!(state.get_code(&expected_address).is_empty());
    }
}
