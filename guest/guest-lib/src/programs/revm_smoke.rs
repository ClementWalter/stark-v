//! Minimal revm smoke test for guest execution.

use serde::{Deserialize, Serialize};

use revm::{
    EVM,
    db::EmptyDB,
    primitives::{
        Address, B256, Bytes, CreateScheme, Env, ExecutionResult, Output, SpecId, TransactTo, U256,
    },
};

const INIT_CODE: &[u8] = &[
    0x60, 0x2a, // PUSH1 0x2a
    0x60, 0x00, // PUSH1 0x00
    0x52, // MSTORE
    0x60, 0x20, // PUSH1 0x20
    0x60, 0x00, // PUSH1 0x00
    0xf3, // RETURN
];

/// Result of the revm smoke test execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevmSmokeResult {
    pub status: u8,
    pub gas_used: u64,
    pub return_len: u32,
    pub return_last: u8,
}

fn output_to_bytes(output: Output) -> Bytes {
    match output {
        Output::Call(data) => data,
        Output::Create(data, _) => data,
    }
}

/// Execute a minimal EVM create transaction and return basic output metrics.
pub fn revm_smoke() -> RevmSmokeResult {
    let caller = Address::from_word(B256::with_last_byte(1));

    let mut env = Env::default();
    env.cfg.spec_id = SpecId::LATEST;
    env.tx.caller = caller;
    env.tx.gas_limit = 1_000_000;
    env.tx.transact_to = TransactTo::Create(CreateScheme::Create);
    env.tx.data = Bytes::from_static(INIT_CODE);
    env.tx.value = U256::ZERO;

    let mut evm = EVM::new();
    evm.database(EmptyDB::default());
    evm.env = env;

    let result = match evm.transact() {
        Ok(result) => result,
        Err(_) => {
            return RevmSmokeResult {
                status: 3,
                gas_used: 0,
                return_len: 0,
                return_last: 0,
            };
        }
    };

    let (status, gas_used, output) = match result.result {
        ExecutionResult::Success {
            gas_used, output, ..
        } => (0u8, gas_used, output_to_bytes(output)),
        ExecutionResult::Revert { gas_used, output } => (1u8, gas_used, output),
        ExecutionResult::Halt { gas_used, .. } => (2u8, gas_used, Bytes::new()),
    };

    let return_len = output.len() as u32;
    let return_last = output.last().copied().unwrap_or(0);

    RevmSmokeResult {
        status,
        gas_used,
        return_len,
        return_last,
    }
}

/// Standard test entry point for e2e testing.
pub fn test_call() -> RevmSmokeResult {
    revm_smoke()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_revm_smoke() {
        let result = revm_smoke();
        assert_eq!(result.status, 0);
        assert_eq!(result.return_len, 32);
        assert_eq!(result.return_last, 0x2a);
    }
}
