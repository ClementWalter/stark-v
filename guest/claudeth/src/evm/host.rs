//! Host interface for call/create operations and external data access.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

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
