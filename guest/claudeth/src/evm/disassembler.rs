//! EVM bytecode disassembler utilities.
//!
//! This is intended for debugging and test output, not for execution.

#[cfg(target_arch = "riscv32")]
extern crate alloc;

#[cfg(not(target_arch = "riscv32"))]
use std::string::String;
#[cfg(not(target_arch = "riscv32"))]
use std::vec::Vec;

#[cfg(target_arch = "riscv32")]
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

/// A single disassembled instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// Program counter.
    pub pc: usize,
    /// Opcode byte.
    pub opcode: u8,
    /// Human-readable opcode name.
    pub name: String,
    /// Immediate data (for PUSH opcodes).
    pub immediate: Vec<u8>,
}

/// Disassemble EVM bytecode into a vector of [`Instruction`]s.
pub fn disassemble(bytecode: &[u8]) -> Vec<Instruction> {
    let mut pc = 0usize;
    let mut output = Vec::new();

    while pc < bytecode.len() {
        let opcode = bytecode[pc];
        let name = opcode_name(opcode);
        let mut immediate = Vec::new();
        let start_pc = pc;

        if opcode == 0x5F {
            // PUSH0
            pc += 1;
        } else if (0x60..=0x7F).contains(&opcode) {
            let len = (opcode - 0x5F) as usize;
            let data_start = pc + 1;
            let data_end = (pc + 1 + len).min(bytecode.len());
            immediate.extend_from_slice(&bytecode[data_start..data_end]);
            pc = data_end;
        } else {
            pc += 1;
        }

        output.push(Instruction {
            pc: start_pc,
            opcode,
            name,
            immediate,
        });
    }

    output
}

/// Format disassembly into printable lines.
pub fn format_disassembly(bytecode: &[u8]) -> Vec<String> {
    disassemble(bytecode)
        .into_iter()
        .map(|inst| format_instruction(&inst))
        .collect()
}

fn format_instruction(inst: &Instruction) -> String {
    if inst.immediate.is_empty() {
        format!("{:04x}: {} (0x{:02x})", inst.pc, inst.name, inst.opcode)
    } else {
        let data = to_hex(&inst.immediate);
        format!(
            "{:04x}: {} (0x{:02x}) 0x{}",
            inst.pc, inst.name, inst.opcode, data
        )
    }
}

fn opcode_name(opcode: u8) -> String {
    match opcode {
        0x00 => "STOP".to_string(),
        0x01 => "ADD".to_string(),
        0x02 => "MUL".to_string(),
        0x03 => "SUB".to_string(),
        0x04 => "DIV".to_string(),
        0x05 => "SDIV".to_string(),
        0x06 => "MOD".to_string(),
        0x07 => "SMOD".to_string(),
        0x08 => "ADDMOD".to_string(),
        0x09 => "MULMOD".to_string(),
        0x0A => "EXP".to_string(),
        0x0B => "SIGNEXTEND".to_string(),
        0x10 => "LT".to_string(),
        0x11 => "GT".to_string(),
        0x12 => "SLT".to_string(),
        0x13 => "SGT".to_string(),
        0x14 => "EQ".to_string(),
        0x15 => "ISZERO".to_string(),
        0x16 => "AND".to_string(),
        0x17 => "OR".to_string(),
        0x18 => "XOR".to_string(),
        0x19 => "NOT".to_string(),
        0x1A => "BYTE".to_string(),
        0x1B => "SHL".to_string(),
        0x1C => "SHR".to_string(),
        0x1D => "SAR".to_string(),
        0x20 => "KECCAK256".to_string(),
        0x30 => "ADDRESS".to_string(),
        0x31 => "BALANCE".to_string(),
        0x32 => "ORIGIN".to_string(),
        0x33 => "CALLER".to_string(),
        0x34 => "CALLVALUE".to_string(),
        0x35 => "CALLDATALOAD".to_string(),
        0x36 => "CALLDATASIZE".to_string(),
        0x37 => "CALLDATACOPY".to_string(),
        0x38 => "CODESIZE".to_string(),
        0x39 => "CODECOPY".to_string(),
        0x3A => "GASPRICE".to_string(),
        0x3B => "EXTCODESIZE".to_string(),
        0x3C => "EXTCODECOPY".to_string(),
        0x3D => "RETURNDATASIZE".to_string(),
        0x3E => "RETURNDATACOPY".to_string(),
        0x3F => "EXTCODEHASH".to_string(),
        0x40 => "BLOCKHASH".to_string(),
        0x41 => "COINBASE".to_string(),
        0x42 => "TIMESTAMP".to_string(),
        0x43 => "NUMBER".to_string(),
        0x44 => "DIFFICULTY".to_string(),
        0x45 => "GASLIMIT".to_string(),
        0x46 => "CHAINID".to_string(),
        0x47 => "SELFBALANCE".to_string(),
        0x48 => "BASEFEE".to_string(),
        0x49 => "BLOBHASH".to_string(),
        0x4A => "BLOBBASEFEE".to_string(),
        0x50 => "POP".to_string(),
        0x51 => "MLOAD".to_string(),
        0x52 => "MSTORE".to_string(),
        0x53 => "MSTORE8".to_string(),
        0x54 => "SLOAD".to_string(),
        0x55 => "SSTORE".to_string(),
        0x56 => "JUMP".to_string(),
        0x57 => "JUMPI".to_string(),
        0x58 => "PC".to_string(),
        0x59 => "MSIZE".to_string(),
        0x5A => "GAS".to_string(),
        0x5B => "JUMPDEST".to_string(),
        0x5C => "TLOAD".to_string(),
        0x5D => "TSTORE".to_string(),
        0x5E => "MCOPY".to_string(),
        0x5F => "PUSH0".to_string(),
        0x80..=0x8F => format!("DUP{}", opcode - 0x7F),
        0x90..=0x9F => format!("SWAP{}", opcode - 0x8F),
        0xA0..=0xA4 => format!("LOG{}", opcode - 0xA0),
        0xF0 => "CREATE".to_string(),
        0xF1 => "CALL".to_string(),
        0xF2 => "CALLCODE".to_string(),
        0xF3 => "RETURN".to_string(),
        0xF4 => "DELEGATECALL".to_string(),
        0xF5 => "CREATE2".to_string(),
        0xFA => "STATICCALL".to_string(),
        0xFD => "REVERT".to_string(),
        0xFE => "INVALID".to_string(),
        0xFF => "SELFDESTRUCT".to_string(),
        0x60..=0x7F => format!("PUSH{}", opcode - 0x5F),
        _ => format!("UNKNOWN_0x{opcode:02x}"),
    }
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}
