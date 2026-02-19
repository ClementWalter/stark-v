//! EVM Execution Tracing
//!
//! This module provides execution tracing infrastructure for debugging gas consumption,
//! state changes, and execution flow. Tracing is controlled by the `evm-trace` feature
//! and only compiles in when explicitly enabled.
//!
//! ## Usage
//!
//! Enable tracing with the feature flag:
//! ```bash
//! cargo test --features evm-trace
//! ```

#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(target_arch = "riscv32")]
use alloc::vec::Vec;

/// Gas trace entry recording gas consumption for a single opcode
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GasTraceEntry {
    /// Program counter (bytecode offset)
    pub pc: usize,
    /// Opcode byte value
    pub opcode: u8,
    /// Opcode mnemonic name
    pub name: &'static str,
    /// Gas available before execution
    pub gas_before: u64,
    /// Gas consumed by this opcode
    pub gas_cost: u64,
    /// Gas available after execution
    pub gas_after: u64,
    /// Cumulative gas used from start
    pub cumulative_gas: u64,
}

/// Captured gas trace for a full execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GasTrace {
    /// Initial gas limit for the execution
    pub initial_gas: u64,
    /// Recorded trace entries
    pub entries: Vec<GasTraceEntry>,
}

impl GasTrace {
    /// Format trace as human-readable string
    #[cfg(not(target_arch = "riscv32"))]
    pub fn format(&self) -> String {
        use std::string::String;
        let mut output = String::new();
        output.push_str(&format!(
            "Gas Trace (initial: {}, used: {})\n",
            self.initial_gas,
            self.initial_gas.saturating_sub(
                self.entries
                    .last()
                    .map(|e| e.gas_after)
                    .unwrap_or(self.initial_gas)
            )
        ));
        output.push_str(&format!(
            "{:>6} {:>12} {:>10} {:>10} {:>10} {:>12}\n",
            "PC", "Opcode", "Before", "Cost", "After", "Cumulative"
        ));
        output.push_str(&format!("{}\n", "-".repeat(70)));

        for entry in &self.entries {
            output.push_str(&format!(
                "{:06x} {:12} {:10} {:10} {:10} {:12}\n",
                entry.pc,
                format!("{} (0x{:02x})", entry.name, entry.opcode),
                entry.gas_before,
                entry.gas_cost,
                entry.gas_after,
                entry.cumulative_gas
            ));
        }

        output
    }

    /// Print trace to stderr (for debugging)
    #[cfg(not(target_arch = "riscv32"))]
    pub fn print(&self) {
        eprintln!("{}", self.format());
    }
}

/// Gas tracer that records per-opcode gas consumption
#[derive(Debug, Clone, Default)]
pub struct GasTracer {
    /// Recorded trace entries
    entries: Vec<GasTraceEntry>,
    /// Initial gas limit
    initial_gas: u64,
}

impl GasTracer {
    /// Create a new gas tracer with the given initial gas limit
    pub fn new(initial_gas: u64) -> Self {
        Self {
            entries: Vec::new(),
            initial_gas,
        }
    }

    /// Record a gas trace entry
    pub fn trace(
        &mut self,
        pc: usize,
        opcode: u8,
        name: &'static str,
        gas_before: u64,
        gas_cost: u64,
        gas_after: u64,
    ) {
        let cumulative_gas = self.initial_gas - gas_after;
        self.entries.push(GasTraceEntry {
            pc,
            opcode,
            name,
            gas_before,
            gas_cost,
            gas_after,
            cumulative_gas,
        });
    }

    /// Get all trace entries
    pub fn entries(&self) -> &[GasTraceEntry] {
        &self.entries
    }

    /// Get total gas used
    pub fn total_gas_used(&self) -> u64 {
        self.initial_gas.saturating_sub(
            self.entries
                .last()
                .map(|e| e.gas_after)
                .unwrap_or(self.initial_gas),
        )
    }

    /// Format trace as human-readable string
    #[cfg(not(target_arch = "riscv32"))]
    pub fn format(&self) -> String {
        use std::string::String;
        let mut output = String::new();
        output.push_str(&format!(
            "Gas Trace (initial: {}, used: {})\n",
            self.initial_gas,
            self.total_gas_used()
        ));
        output.push_str(&format!(
            "{:>6} {:>12} {:>10} {:>10} {:>10} {:>12}\n",
            "PC", "Opcode", "Before", "Cost", "After", "Cumulative"
        ));
        output.push_str(&format!("{}\n", "-".repeat(70)));

        for entry in &self.entries {
            output.push_str(&format!(
                "{:06x} {:12} {:10} {:10} {:10} {:12}\n",
                entry.pc,
                format!("{} (0x{:02x})", entry.name, entry.opcode),
                entry.gas_before,
                entry.gas_cost,
                entry.gas_after,
                entry.cumulative_gas
            ));
        }

        output
    }

    /// Print trace to stderr (for debugging)
    #[cfg(not(target_arch = "riscv32"))]
    pub fn print(&self) {
        eprintln!("{}", self.format());
    }

    /// Snapshot the current trace entries for later inspection
    pub fn snapshot(&self) -> GasTrace {
        GasTrace {
            initial_gas: self.initial_gas,
            entries: self.entries.clone(),
        }
    }
}

/// Opcode name lookup table
pub fn opcode_name(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "STOP",
        0x01 => "ADD",
        0x02 => "MUL",
        0x03 => "SUB",
        0x04 => "DIV",
        0x05 => "SDIV",
        0x06 => "MOD",
        0x07 => "SMOD",
        0x08 => "ADDMOD",
        0x09 => "MULMOD",
        0x0a => "EXP",
        0x0b => "SIGNEXTEND",
        0x10 => "LT",
        0x11 => "GT",
        0x12 => "SLT",
        0x13 => "SGT",
        0x14 => "EQ",
        0x15 => "ISZERO",
        0x16 => "AND",
        0x17 => "OR",
        0x18 => "XOR",
        0x19 => "NOT",
        0x1a => "BYTE",
        0x1b => "SHL",
        0x1c => "SHR",
        0x1d => "SAR",
        0x20 => "KECCAK256",
        0x30 => "ADDRESS",
        0x31 => "BALANCE",
        0x32 => "ORIGIN",
        0x33 => "CALLER",
        0x34 => "CALLVALUE",
        0x35 => "CALLDATALOAD",
        0x36 => "CALLDATASIZE",
        0x37 => "CALLDATACOPY",
        0x38 => "CODESIZE",
        0x39 => "CODECOPY",
        0x3a => "GASPRICE",
        0x3b => "EXTCODESIZE",
        0x3c => "EXTCODECOPY",
        0x3d => "RETURNDATASIZE",
        0x3e => "RETURNDATACOPY",
        0x3f => "EXTCODEHASH",
        0x40 => "BLOCKHASH",
        0x41 => "COINBASE",
        0x42 => "TIMESTAMP",
        0x43 => "NUMBER",
        0x44 => "PREVRANDAO",
        0x45 => "GASLIMIT",
        0x46 => "CHAINID",
        0x47 => "SELFBALANCE",
        0x48 => "BASEFEE",
        0x49 => "BLOBHASH",
        0x4a => "BLOBBASEFEE",
        0x50 => "POP",
        0x51 => "MLOAD",
        0x52 => "MSTORE",
        0x53 => "MSTORE8",
        0x54 => "SLOAD",
        0x55 => "SSTORE",
        0x56 => "JUMP",
        0x57 => "JUMPI",
        0x58 => "PC",
        0x59 => "MSIZE",
        0x5a => "GAS",
        0x5b => "JUMPDEST",
        0x5c => "TLOAD",
        0x5d => "TSTORE",
        0x5e => "MCOPY",
        0x5f => "PUSH0",
        0x60 => "PUSH1",
        0x61 => "PUSH2",
        0x62 => "PUSH3",
        0x63 => "PUSH4",
        0x64 => "PUSH5",
        0x65 => "PUSH6",
        0x66 => "PUSH7",
        0x67 => "PUSH8",
        0x68 => "PUSH9",
        0x69 => "PUSH10",
        0x6a => "PUSH11",
        0x6b => "PUSH12",
        0x6c => "PUSH13",
        0x6d => "PUSH14",
        0x6e => "PUSH15",
        0x6f => "PUSH16",
        0x70 => "PUSH17",
        0x71 => "PUSH18",
        0x72 => "PUSH19",
        0x73 => "PUSH20",
        0x74 => "PUSH21",
        0x75 => "PUSH22",
        0x76 => "PUSH23",
        0x77 => "PUSH24",
        0x78 => "PUSH25",
        0x79 => "PUSH26",
        0x7a => "PUSH27",
        0x7b => "PUSH28",
        0x7c => "PUSH29",
        0x7d => "PUSH30",
        0x7e => "PUSH31",
        0x7f => "PUSH32",
        0x80 => "DUP1",
        0x81 => "DUP2",
        0x82 => "DUP3",
        0x83 => "DUP4",
        0x84 => "DUP5",
        0x85 => "DUP6",
        0x86 => "DUP7",
        0x87 => "DUP8",
        0x88 => "DUP9",
        0x89 => "DUP10",
        0x8a => "DUP11",
        0x8b => "DUP12",
        0x8c => "DUP13",
        0x8d => "DUP14",
        0x8e => "DUP15",
        0x8f => "DUP16",
        0x90 => "SWAP1",
        0x91 => "SWAP2",
        0x92 => "SWAP3",
        0x93 => "SWAP4",
        0x94 => "SWAP5",
        0x95 => "SWAP6",
        0x96 => "SWAP7",
        0x97 => "SWAP8",
        0x98 => "SWAP9",
        0x99 => "SWAP10",
        0x9a => "SWAP11",
        0x9b => "SWAP12",
        0x9c => "SWAP13",
        0x9d => "SWAP14",
        0x9e => "SWAP15",
        0x9f => "SWAP16",
        0xa0 => "LOG0",
        0xa1 => "LOG1",
        0xa2 => "LOG2",
        0xa3 => "LOG3",
        0xa4 => "LOG4",
        0xf0 => "CREATE",
        0xf1 => "CALL",
        0xf2 => "CALLCODE",
        0xf3 => "RETURN",
        0xf4 => "DELEGATECALL",
        0xf5 => "CREATE2",
        0xfa => "STATICCALL",
        0xfd => "REVERT",
        0xfe => "INVALID",
        0xff => "SELFDESTRUCT",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_tracer_basic() {
        let mut tracer = GasTracer::new(1000);

        // Simulate a few opcode executions
        tracer.trace(0, 0x60, "PUSH1", 1000, 3, 997);
        tracer.trace(2, 0x60, "PUSH1", 997, 3, 994);
        tracer.trace(4, 0x01, "ADD", 994, 3, 991);
        tracer.trace(5, 0x00, "STOP", 991, 0, 991);

        assert_eq!(tracer.entries().len(), 4);
        assert_eq!(tracer.total_gas_used(), 9);

        let first = &tracer.entries()[0];
        assert_eq!(first.pc, 0);
        assert_eq!(first.opcode, 0x60);
        assert_eq!(first.name, "PUSH1");
        assert_eq!(first.gas_before, 1000);
        assert_eq!(first.gas_cost, 3);
        assert_eq!(first.gas_after, 997);
        assert_eq!(first.cumulative_gas, 3);
    }

    #[test]
    fn test_opcode_names() {
        assert_eq!(opcode_name(0x00), "STOP");
        assert_eq!(opcode_name(0x01), "ADD");
        assert_eq!(opcode_name(0x60), "PUSH1");
        assert_eq!(opcode_name(0x7f), "PUSH32");
        assert_eq!(opcode_name(0xf1), "CALL");
        assert_eq!(opcode_name(0xff), "SELFDESTRUCT");
        assert_eq!(opcode_name(0x99), "SWAP10");
        assert_eq!(opcode_name(0xef), "UNKNOWN"); // 0xef is not a valid opcode
    }

    #[cfg(not(target_arch = "riscv32"))]
    #[test]
    fn test_gas_tracer_format() {
        let mut tracer = GasTracer::new(100);
        tracer.trace(0, 0x60, "PUSH1", 100, 3, 97);
        tracer.trace(2, 0x01, "ADD", 97, 3, 94);

        let output = tracer.format();
        assert!(output.contains("Gas Trace"));
        assert!(output.contains("PUSH1"));
        assert!(output.contains("ADD"));
        assert!(output.contains("100")); // initial gas
        assert!(output.contains("6")); // used gas
    }
}
