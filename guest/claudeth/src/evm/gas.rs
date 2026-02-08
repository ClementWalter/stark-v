//! EVM Gas Metering
//!
//! This module implements gas cost calculations for Ethereum opcodes and operations,
//! following the Fusaka fork specification. Gas metering ensures proper resource
//! accounting for EVM execution.
//!
//! ## Gas Cost Categories
//!
//! - **Opcode costs**: Fixed gas costs for basic operations
//! - **Memory expansion**: Quadratic cost for memory growth
//! - **Storage operations**: Complex costs with warm/cold access tracking
//! - **Call operations**: Gas forwarding and account creation costs

// =============================================================================
// Gas Cost Constants
// =============================================================================

// Basic step costs (from go-ethereum core/vm/gas.go)
/// Gas cost for operations like PC, MSIZE, GAS, ADDRESS, ORIGIN, CALLER, etc.
pub const GAS_QUICK_STEP: u64 = 2;

/// Gas cost for fastest operations like ADD, SUB, NOT, EQ, ISZERO, etc.
pub const GAS_FASTEST_STEP: u64 = 3;

/// Gas cost for fast-ish operations
pub const GAS_FASTISH_STEP: u64 = 4;

/// Gas cost for operations like MUL, DIV, MOD, etc.
pub const GAS_FAST_STEP: u64 = 5;

/// Gas cost for operations like ADDMOD, MULMOD, JUMP
pub const GAS_MID_STEP: u64 = 8;

/// Gas cost for operations like JUMPI
pub const GAS_SLOW_STEP: u64 = 10;

/// Gas cost for operations like BLOCKHASH
pub const GAS_EXT_STEP: u64 = 20;

// Arithmetic operations
/// Gas cost for STOP opcode
pub const GAS_STOP: u64 = 0;

/// Gas cost for ADD opcode
pub const GAS_ADD: u64 = GAS_FASTEST_STEP;

/// Gas cost for MUL opcode
pub const GAS_MUL: u64 = GAS_FAST_STEP;

/// Gas cost for SUB opcode
pub const GAS_SUB: u64 = GAS_FASTEST_STEP;

/// Gas cost for DIV opcode
pub const GAS_DIV: u64 = GAS_FAST_STEP;

/// Gas cost for SDIV (signed division) opcode
pub const GAS_SDIV: u64 = GAS_FAST_STEP;

/// Gas cost for MOD opcode
pub const GAS_MOD: u64 = GAS_FAST_STEP;

/// Gas cost for SMOD (signed modulo) opcode
pub const GAS_SMOD: u64 = GAS_FAST_STEP;

/// Gas cost for ADDMOD opcode
pub const GAS_ADDMOD: u64 = GAS_MID_STEP;

/// Gas cost for MULMOD opcode
pub const GAS_MULMOD: u64 = GAS_MID_STEP;

/// Base gas cost for EXP opcode
pub const GAS_EXP: u64 = 10;

/// Gas cost per byte of exponent for EXP opcode
pub const GAS_EXP_BYTE: u64 = 50;

/// Gas cost for SIGNEXTEND opcode
pub const GAS_SIGNEXTEND: u64 = GAS_FAST_STEP;

// Comparison operations
/// Gas cost for LT (less than) opcode
pub const GAS_LT: u64 = GAS_FASTEST_STEP;

/// Gas cost for GT (greater than) opcode
pub const GAS_GT: u64 = GAS_FASTEST_STEP;

/// Gas cost for SLT (signed less than) opcode
pub const GAS_SLT: u64 = GAS_FASTEST_STEP;

/// Gas cost for SGT (signed greater than) opcode
pub const GAS_SGT: u64 = GAS_FASTEST_STEP;

/// Gas cost for EQ (equal) opcode
pub const GAS_EQ: u64 = GAS_FASTEST_STEP;

/// Gas cost for ISZERO opcode
pub const GAS_ISZERO: u64 = GAS_FASTEST_STEP;

// Bitwise operations
/// Gas cost for AND opcode
pub const GAS_AND: u64 = GAS_FASTEST_STEP;

/// Gas cost for OR opcode
pub const GAS_OR: u64 = GAS_FASTEST_STEP;

/// Gas cost for XOR opcode
pub const GAS_XOR: u64 = GAS_FASTEST_STEP;

/// Gas cost for NOT opcode
pub const GAS_NOT: u64 = GAS_FASTEST_STEP;

/// Gas cost for BYTE opcode
pub const GAS_BYTE: u64 = GAS_FASTEST_STEP;

/// Gas cost for SHL (shift left) opcode
pub const GAS_SHL: u64 = GAS_FASTEST_STEP;

/// Gas cost for SHR (logical shift right) opcode
pub const GAS_SHR: u64 = GAS_FASTEST_STEP;

/// Gas cost for SAR (arithmetic shift right) opcode
pub const GAS_SAR: u64 = GAS_FASTEST_STEP;

// Keccak256 hashing
/// Base gas cost for KECCAK256 opcode
pub const GAS_KECCAK256: u64 = 30;

/// Gas cost per word (32 bytes) for KECCAK256 opcode
pub const GAS_KECCAK256_WORD: u64 = 6;

// Memory operations
/// Gas cost for MLOAD opcode
pub const GAS_MLOAD: u64 = GAS_FASTEST_STEP;

/// Gas cost for MSTORE opcode
pub const GAS_MSTORE: u64 = GAS_FASTEST_STEP;

/// Gas cost for MSTORE8 opcode
pub const GAS_MSTORE8: u64 = GAS_FASTEST_STEP;

/// Gas cost per word for memory copy operations
pub const GAS_COPY: u64 = 3;

/// Memory expansion coefficient
pub const MEMORY_GAS: u64 = 3;

/// Quadratic coefficient divisor for memory expansion
pub const QUAD_COEFF_DIV: u64 = 512;

// Storage operations (EIP-2929 + EIP-2200)
/// Gas cost for SLOAD with cold access (EIP-2929)
pub const GAS_SLOAD_COLD: u64 = 2100;

/// Gas cost for SLOAD with warm access (EIP-2929)
pub const GAS_SLOAD_WARM: u64 = 100;

/// Gas cost for SSTORE when setting a new non-zero value (from zero)
pub const GAS_SSTORE_SET: u64 = 20000;

/// Gas cost for SSTORE when modifying an existing value
pub const GAS_SSTORE_RESET: u64 = 5000;

/// Gas cost for SSTORE when clearing a value (to zero)
pub const GAS_SSTORE_CLEAR: u64 = 5000;

/// Sentry gas reserved for SSTORE (EIP-2200)
pub const GAS_SSTORE_SENTRY: u64 = 2300;

/// Gas cost for SSTORE no-op (warm access, no change)
pub const GAS_SSTORE_NOOP: u64 = 100;

// Transient storage (EIP-1153)
/// Gas cost for TLOAD opcode
pub const GAS_TLOAD: u64 = GAS_SLOAD_WARM;

/// Gas cost for TSTORE opcode
pub const GAS_TSTORE: u64 = GAS_SLOAD_WARM;

// Control flow
/// Gas cost for JUMP opcode
pub const GAS_JUMP: u64 = GAS_MID_STEP;

/// Gas cost for JUMPI (conditional jump) opcode
pub const GAS_JUMPI: u64 = GAS_SLOW_STEP;

/// Gas cost for JUMPDEST opcode
pub const GAS_JUMPDEST: u64 = 1;

/// Gas cost for PC (program counter) opcode
pub const GAS_PC: u64 = GAS_QUICK_STEP;

/// Gas cost for MSIZE opcode
pub const GAS_MSIZE: u64 = GAS_QUICK_STEP;

/// Gas cost for GAS opcode
pub const GAS_GAS: u64 = GAS_QUICK_STEP;

// Stack operations
/// Gas cost for POP opcode
pub const GAS_POP: u64 = GAS_QUICK_STEP;

/// Gas cost for PUSH0 opcode
pub const GAS_PUSH0: u64 = GAS_QUICK_STEP;

/// Gas cost for PUSH1-PUSH32 opcodes
pub const GAS_PUSH: u64 = GAS_FASTEST_STEP;

/// Gas cost for DUP1-DUP16 opcodes
pub const GAS_DUP: u64 = GAS_FASTEST_STEP;

/// Gas cost for SWAP1-SWAP16 opcodes
pub const GAS_SWAP: u64 = GAS_FASTEST_STEP;

// Block information
/// Gas cost for BLOCKHASH opcode
pub const GAS_BLOCKHASH: u64 = GAS_EXT_STEP;

/// Gas cost for COINBASE opcode
pub const GAS_COINBASE: u64 = GAS_QUICK_STEP;

/// Gas cost for TIMESTAMP opcode
pub const GAS_TIMESTAMP: u64 = GAS_QUICK_STEP;

/// Gas cost for NUMBER opcode
pub const GAS_NUMBER: u64 = GAS_QUICK_STEP;

/// Gas cost for DIFFICULTY/PREVRANDAO opcode
pub const GAS_DIFFICULTY: u64 = GAS_QUICK_STEP;

/// Gas cost for GASLIMIT opcode
pub const GAS_GASLIMIT: u64 = GAS_QUICK_STEP;

/// Gas cost for CHAINID opcode
pub const GAS_CHAINID: u64 = GAS_QUICK_STEP;

/// Gas cost for SELFBALANCE opcode
pub const GAS_SELFBALANCE: u64 = GAS_FAST_STEP;

/// Gas cost for BASEFEE opcode
pub const GAS_BASEFEE: u64 = GAS_QUICK_STEP;

/// Gas cost for BLOBHASH opcode
pub const GAS_BLOBHASH: u64 = GAS_FASTEST_STEP;

/// Gas cost for BLOBBASEFEE opcode
pub const GAS_BLOBBASEFEE: u64 = GAS_QUICK_STEP;

// Environment information
/// Gas cost for ADDRESS opcode
pub const GAS_ADDRESS: u64 = GAS_QUICK_STEP;

/// Gas cost for BALANCE opcode with cold access
pub const GAS_BALANCE_COLD: u64 = 2600;

/// Gas cost for BALANCE opcode with warm access
pub const GAS_BALANCE_WARM: u64 = 100;

/// Gas cost for ORIGIN opcode
pub const GAS_ORIGIN: u64 = GAS_QUICK_STEP;

/// Gas cost for CALLER opcode
pub const GAS_CALLER: u64 = GAS_QUICK_STEP;

/// Gas cost for CALLVALUE opcode
pub const GAS_CALLVALUE: u64 = GAS_QUICK_STEP;

/// Gas cost for CALLDATALOAD opcode
pub const GAS_CALLDATALOAD: u64 = GAS_FASTEST_STEP;

/// Gas cost for CALLDATASIZE opcode
pub const GAS_CALLDATASIZE: u64 = GAS_QUICK_STEP;

/// Gas cost for CALLDATACOPY base cost
pub const GAS_CALLDATACOPY: u64 = GAS_FASTEST_STEP;

/// Gas cost for CODESIZE opcode
pub const GAS_CODESIZE: u64 = GAS_QUICK_STEP;

/// Gas cost for CODECOPY base cost
pub const GAS_CODECOPY: u64 = GAS_FASTEST_STEP;

/// Gas cost for GASPRICE opcode
pub const GAS_GASPRICE: u64 = GAS_QUICK_STEP;

/// Gas cost for EXTCODESIZE with cold access
pub const GAS_EXTCODESIZE_COLD: u64 = 2600;

/// Gas cost for EXTCODESIZE with warm access
pub const GAS_EXTCODESIZE_WARM: u64 = 100;

/// Gas cost for EXTCODECOPY with cold access (base)
pub const GAS_EXTCODECOPY_COLD: u64 = 2600;

/// Gas cost for EXTCODECOPY with warm access (base)
pub const GAS_EXTCODECOPY_WARM: u64 = 100;

/// Gas cost for RETURNDATASIZE opcode
pub const GAS_RETURNDATASIZE: u64 = GAS_QUICK_STEP;

/// Gas cost for RETURNDATACOPY base cost
pub const GAS_RETURNDATACOPY: u64 = GAS_FASTEST_STEP;

/// Gas cost for EXTCODEHASH with cold access
pub const GAS_EXTCODEHASH_COLD: u64 = 2600;

/// Gas cost for EXTCODEHASH with warm access
pub const GAS_EXTCODEHASH_WARM: u64 = 100;

/// Gas cost for MCOPY base cost
pub const GAS_MCOPY: u64 = GAS_FASTEST_STEP;

// Call operations
/// Base gas cost for CALL opcode (warm access)
pub const GAS_CALL_WARM: u64 = 100;

/// Base gas cost for CALL opcode (cold access)
pub const GAS_CALL_COLD: u64 = 2600;

/// Additional gas for value transfer in CALL
pub const GAS_CALL_VALUE_TRANSFER: u64 = 9000;

/// Additional gas for creating new account in CALL
pub const GAS_CALL_NEW_ACCOUNT: u64 = 25000;

/// Stipend provided for value transfer
pub const GAS_CALL_STIPEND: u64 = 2300;

/// Base gas cost for CALLCODE opcode (warm)
pub const GAS_CALLCODE_WARM: u64 = 100;

/// Base gas cost for CALLCODE opcode (cold)
pub const GAS_CALLCODE_COLD: u64 = 2600;

/// Base gas cost for DELEGATECALL opcode (warm)
pub const GAS_DELEGATECALL_WARM: u64 = 100;

/// Base gas cost for DELEGATECALL opcode (cold)
pub const GAS_DELEGATECALL_COLD: u64 = 2600;

/// Base gas cost for STATICCALL opcode (warm)
pub const GAS_STATICCALL_WARM: u64 = 100;

/// Base gas cost for STATICCALL opcode (cold)
pub const GAS_STATICCALL_COLD: u64 = 2600;

// Contract creation
/// Gas cost for CREATE opcode
pub const GAS_CREATE: u64 = 32000;

/// Gas cost for CREATE2 opcode (base)
pub const GAS_CREATE2: u64 = 32000;

/// Gas cost per word for CREATE2 init code hashing
pub const GAS_CREATE2_WORD: u64 = 6;

/// Gas cost per byte for code deposit
pub const GAS_CODE_DEPOSIT: u64 = 200;

/// Maximum code size (24KB)
pub const MAX_CODE_SIZE: usize = 24576;

/// Maximum init code size (48KB)
pub const MAX_INIT_CODE_SIZE: usize = 49152;

/// Gas cost per word for init code (EIP-3860)
pub const GAS_INIT_CODE_WORD: u64 = 2;

// Logging operations
/// Base gas cost for LOG operations
pub const GAS_LOG: u64 = 375;

/// Gas cost per LOG topic
pub const GAS_LOG_TOPIC: u64 = 375;

/// Gas cost per byte of LOG data
pub const GAS_LOG_DATA: u64 = 8;

// Return and Revert
/// Gas cost for RETURN opcode (base)
pub const GAS_RETURN: u64 = 0;

/// Gas cost for REVERT opcode (base)
pub const GAS_REVERT: u64 = 0;

// Selfdestruct
/// Gas cost for SELFDESTRUCT opcode
pub const GAS_SELFDESTRUCT: u64 = 5000;

/// Additional gas for SELFDESTRUCT to new account
pub const GAS_SELFDESTRUCT_NEW_ACCOUNT: u64 = 25000;

/// Cold access cost for SELFDESTRUCT
pub const GAS_SELFDESTRUCT_COLD: u64 = 2600;

// Transaction costs
/// Base gas cost for transaction
pub const GAS_TRANSACTION: u64 = 21000;

/// Gas cost for contract creation transaction
pub const GAS_TRANSACTION_CREATE: u64 = 53000;

/// Gas cost per zero byte in transaction data
pub const GAS_TX_DATA_ZERO: u64 = 4;

/// Gas cost per non-zero byte in transaction data (post EIP-2028)
pub const GAS_TX_DATA_NONZERO: u64 = 16;

/// Gas cost per access list address
pub const GAS_ACCESS_LIST_ADDRESS: u64 = 2400;

/// Gas cost per access list storage key
pub const GAS_ACCESS_LIST_STORAGE_KEY: u64 = 1900;

// Precompile gas costs
/// Gas cost for ECRECOVER precompile
pub const GAS_ECRECOVER: u64 = 3000;

/// Base gas cost for SHA256 precompile
pub const GAS_SHA256_BASE: u64 = 60;

/// Gas cost per word for SHA256 precompile
pub const GAS_SHA256_WORD: u64 = 12;

/// Base gas cost for RIPEMD160 precompile
pub const GAS_RIPEMD160_BASE: u64 = 600;

/// Gas cost per word for RIPEMD160 precompile
pub const GAS_RIPEMD160_WORD: u64 = 120;

/// Base gas cost for IDENTITY precompile
pub const GAS_IDENTITY_BASE: u64 = 15;

/// Gas cost per word for IDENTITY precompile
pub const GAS_IDENTITY_WORD: u64 = 3;

/// Gas cost for MODEXP precompile (complex calculation)
pub const GAS_MODEXP_BASE: u64 = 200;

/// Gas cost for BN256ADD precompile (post-Istanbul)
pub const GAS_BN256_ADD: u64 = 150;

/// Gas cost for BN256MUL precompile (post-Istanbul)
pub const GAS_BN256_MUL: u64 = 6000;

/// Base gas cost for BN256PAIRING precompile (post-Istanbul)
pub const GAS_BN256_PAIRING_BASE: u64 = 45000;

/// Gas cost per point for BN256PAIRING precompile (post-Istanbul)
pub const GAS_BN256_PAIRING_POINT: u64 = 34000;

/// Gas cost for BLAKE2F precompile per round
pub const GAS_BLAKE2F_ROUND: u64 = 1;

/// Gas cost for BLS12-381 G1 addition
pub const GAS_BLS12_G1_ADD: u64 = 375;

/// Gas cost for BLS12-381 G1 multiplication
pub const GAS_BLS12_G1_MUL: u64 = 12000;

/// Gas cost for BLS12-381 G2 addition
pub const GAS_BLS12_G2_ADD: u64 = 600;

/// Gas cost for BLS12-381 G2 multiplication
pub const GAS_BLS12_G2_MUL: u64 = 22500;

/// Base gas cost for BLS12-381 pairing
pub const GAS_BLS12_PAIRING_BASE: u64 = 37700;

/// Gas cost per pair for BLS12-381 pairing
pub const GAS_BLS12_PAIRING_PER_PAIR: u64 = 32600;

/// Gas cost for BLS12-381 G1 map
pub const GAS_BLS12_MAP_G1: u64 = 5500;

/// Gas cost for BLS12-381 G2 map
pub const GAS_BLS12_MAP_G2: u64 = 23800;

/// Gas cost for P256VERIFY precompile
pub const GAS_P256_VERIFY: u64 = 6900;

/// Gas cost for point evaluation precompile (EIP-4844)
pub const GAS_POINT_EVALUATION: u64 = 50000;

// Limits
/// Maximum stack depth
pub const STACK_LIMIT: usize = 1024;

/// Maximum call depth
pub const CALL_DEPTH_LIMIT: usize = 1024;

// =============================================================================
// Gas Calculation Functions
// =============================================================================

/// Returns the base gas cost for a given opcode.
///
/// This function returns the static base cost for opcodes. Some opcodes have
/// additional dynamic costs that must be calculated separately.
///
/// # Arguments
///
/// * `opcode` - The EVM opcode byte (0x00-0xFF)
///
/// # Returns
///
/// The base gas cost for the opcode, or 0 if the opcode is invalid.
///
/// # Examples
///
/// ```
/// use claudeth::evm::gas::opcode_gas_cost;
///
/// assert_eq!(opcode_gas_cost(0x01), 3); // ADD
/// assert_eq!(opcode_gas_cost(0x02), 5); // MUL
/// assert_eq!(opcode_gas_cost(0x55), 2100); // SLOAD (cold)
/// ```
pub fn opcode_gas_cost(opcode: u8) -> u64 {
    match opcode {
        // 0x00-0x0F: Arithmetic operations
        0x00 => GAS_STOP,           // STOP
        0x01 => GAS_ADD,            // ADD
        0x02 => GAS_MUL,            // MUL
        0x03 => GAS_SUB,            // SUB
        0x04 => GAS_DIV,            // DIV
        0x05 => GAS_SDIV,           // SDIV
        0x06 => GAS_MOD,            // MOD
        0x07 => GAS_SMOD,           // SMOD
        0x08 => GAS_ADDMOD,         // ADDMOD
        0x09 => GAS_MULMOD,         // MULMOD
        0x0A => GAS_EXP,            // EXP (base, dynamic cost per byte)
        0x0B => GAS_SIGNEXTEND,     // SIGNEXTEND

        // 0x10-0x1F: Comparison & bitwise operations
        0x10 => GAS_LT,             // LT
        0x11 => GAS_GT,             // GT
        0x12 => GAS_SLT,            // SLT
        0x13 => GAS_SGT,            // SGT
        0x14 => GAS_EQ,             // EQ
        0x15 => GAS_ISZERO,         // ISZERO
        0x16 => GAS_AND,            // AND
        0x17 => GAS_OR,             // OR
        0x18 => GAS_XOR,            // XOR
        0x19 => GAS_NOT,            // NOT
        0x1A => GAS_BYTE,           // BYTE
        0x1B => GAS_SHL,            // SHL
        0x1C => GAS_SHR,            // SHR
        0x1D => GAS_SAR,            // SAR

        // 0x20: Hashing
        0x20 => GAS_KECCAK256,      // KECCAK256 (base, dynamic per word)

        // 0x30-0x3F: Environment information
        0x30 => GAS_ADDRESS,        // ADDRESS
        0x31 => GAS_BALANCE_COLD,   // BALANCE (cold, can be warm)
        0x32 => GAS_ORIGIN,         // ORIGIN
        0x33 => GAS_CALLER,         // CALLER
        0x34 => GAS_CALLVALUE,      // CALLVALUE
        0x35 => GAS_CALLDATALOAD,   // CALLDATALOAD
        0x36 => GAS_CALLDATASIZE,   // CALLDATASIZE
        0x37 => GAS_CALLDATACOPY,   // CALLDATACOPY (base, dynamic)
        0x38 => GAS_CODESIZE,       // CODESIZE
        0x39 => GAS_CODECOPY,       // CODECOPY (base, dynamic)
        0x3A => GAS_GASPRICE,       // GASPRICE
        0x3B => GAS_EXTCODESIZE_COLD, // EXTCODESIZE (cold)
        0x3C => GAS_EXTCODECOPY_COLD, // EXTCODECOPY (cold, base)
        0x3D => GAS_RETURNDATASIZE, // RETURNDATASIZE
        0x3E => GAS_RETURNDATACOPY, // RETURNDATACOPY (base, dynamic)
        0x3F => GAS_EXTCODEHASH_COLD, // EXTCODEHASH (cold)

        // 0x40-0x4F: Block information
        0x40 => GAS_BLOCKHASH,      // BLOCKHASH
        0x41 => GAS_COINBASE,       // COINBASE
        0x42 => GAS_TIMESTAMP,      // TIMESTAMP
        0x43 => GAS_NUMBER,         // NUMBER
        0x44 => GAS_DIFFICULTY,     // DIFFICULTY/PREVRANDAO
        0x45 => GAS_GASLIMIT,       // GASLIMIT
        0x46 => GAS_CHAINID,        // CHAINID
        0x47 => GAS_SELFBALANCE,    // SELFBALANCE
        0x48 => GAS_BASEFEE,        // BASEFEE
        0x49 => GAS_BLOBHASH,       // BLOBHASH
        0x4A => GAS_BLOBBASEFEE,    // BLOBBASEFEE

        // 0x50-0x5F: Stack, Memory, Storage, and Flow operations
        0x50 => GAS_POP,            // POP
        0x51 => GAS_MLOAD,          // MLOAD (plus memory expansion)
        0x52 => GAS_MSTORE,         // MSTORE (plus memory expansion)
        0x53 => GAS_MSTORE8,        // MSTORE8 (plus memory expansion)
        0x54 => GAS_SLOAD_COLD,     // SLOAD (cold, can be warm)
        0x55 => 0,                  // SSTORE (dynamic only)
        0x56 => GAS_JUMP,           // JUMP
        0x57 => GAS_JUMPI,          // JUMPI
        0x58 => GAS_PC,             // PC
        0x59 => GAS_MSIZE,          // MSIZE
        0x5A => GAS_GAS,            // GAS
        0x5B => GAS_JUMPDEST,       // JUMPDEST
        0x5C => GAS_TLOAD,          // TLOAD (EIP-1153)
        0x5D => GAS_TSTORE,         // TSTORE (EIP-1153)
        0x5E => GAS_MCOPY,          // MCOPY (EIP-5656)
        0x5F => GAS_PUSH0,          // PUSH0 (EIP-3855)

        // 0x60-0x7F: PUSH operations
        0x60..=0x7F => GAS_PUSH,    // PUSH1-PUSH32

        // 0x80-0x8F: DUP operations
        0x80..=0x8F => GAS_DUP,     // DUP1-DUP16

        // 0x90-0x9F: SWAP operations
        0x90..=0x9F => GAS_SWAP,    // SWAP1-SWAP16

        // 0xA0-0xA4: LOG operations
        0xA0 => GAS_LOG,            // LOG0 (base, dynamic)
        0xA1 => GAS_LOG,            // LOG1 (base, dynamic)
        0xA2 => GAS_LOG,            // LOG2 (base, dynamic)
        0xA3 => GAS_LOG,            // LOG3 (base, dynamic)
        0xA4 => GAS_LOG,            // LOG4 (base, dynamic)

        // 0xF0-0xFF: System operations
        0xF0 => GAS_CREATE,         // CREATE
        0xF1 => GAS_CALL_COLD,      // CALL (cold, can be warm)
        0xF2 => GAS_CALLCODE_COLD,  // CALLCODE (cold)
        0xF3 => GAS_RETURN,         // RETURN (base, dynamic)
        0xF4 => GAS_DELEGATECALL_COLD, // DELEGATECALL (cold)
        0xF5 => GAS_CREATE2,        // CREATE2 (base, dynamic)
        0xFA => GAS_STATICCALL_COLD, // STATICCALL (cold)
        0xFD => GAS_REVERT,         // REVERT (base, dynamic)
        0xFE => 0,                  // INVALID (consumes all gas)
        0xFF => GAS_SELFDESTRUCT,   // SELFDESTRUCT (base, dynamic)

        // Unknown opcodes
        _ => 0,
    }
}

/// Calculates the gas cost for memory expansion.
///
/// Memory expansion follows a quadratic formula to discourage excessive memory use:
/// `cost = (new_mem_size_words^2 / 512) + (3 * new_mem_size_words)`
///
/// This function returns the **additional** cost for expanding from `old_size` to `new_size`.
///
/// # Arguments
///
/// * `old_size` - Current memory size in bytes
/// * `new_size` - New memory size in bytes (must be >= old_size)
///
/// # Returns
///
/// The additional gas cost for memory expansion (0 if no expansion needed).
///
/// # Examples
///
/// ```
/// use claudeth::evm::gas::memory_expansion_cost;
///
/// // No expansion
/// assert_eq!(memory_expansion_cost(0, 0), 0);
///
/// // First expansion to 32 bytes (1 word)
/// assert_eq!(memory_expansion_cost(0, 32), 3);
///
/// // Expansion from 32 to 64 bytes (1 to 2 words)
/// assert_eq!(memory_expansion_cost(32, 64), 3);
///
/// // Large expansion has quadratic cost
/// let cost_to_1024 = memory_expansion_cost(0, 1024);
/// assert!(cost_to_1024 > 96); // More than linear 3 * 32 words
/// ```
pub fn memory_expansion_cost(old_size: usize, new_size: usize) -> u64 {
    if new_size <= old_size {
        return 0;
    }

    // Calculate cost for new size
    let new_words = new_size.div_ceil(32); // Round up to nearest word
    let new_cost = (new_words * new_words) / QUAD_COEFF_DIV as usize
                 + (MEMORY_GAS as usize * new_words);

    // Calculate cost for old size
    let old_words = old_size.div_ceil(32);
    let old_cost = (old_words * old_words) / QUAD_COEFF_DIV as usize
                 + (MEMORY_GAS as usize * old_words);

    (new_cost - old_cost) as u64
}

/// Calculates the gas cost for call operations.
///
/// Call operations (CALL, CALLCODE, DELEGATECALL, STATICCALL) have complex gas costs
/// that depend on:
/// - Whether the call transfers value (adds 9000 gas + 2300 stipend)
/// - Whether the recipient account exists (adds 25000 gas for new account)
/// - Whether the address is warm or cold (base cost varies)
///
/// # Arguments
///
/// * `is_value_transfer` - True if the call transfers ETH value
/// * `is_new_account` - True if the recipient account doesn't exist
/// * `is_cold_access` - True if the address hasn't been accessed before
///
/// # Returns
///
/// The total gas cost for the call operation (excluding gas forwarded to callee).
///
/// # Examples
///
/// ```
/// use claudeth::evm::gas::call_gas_cost;
///
/// // Simple call to warm account, no value transfer
/// assert_eq!(call_gas_cost(false, false, false), 100);
///
/// // Call to cold account (first access)
/// assert_eq!(call_gas_cost(false, false, true), 2600);
///
/// // Call with value transfer to existing warm account
/// assert_eq!(call_gas_cost(true, false, false), 9100);
///
/// // Call with value to new account (most expensive)
/// assert_eq!(call_gas_cost(true, true, false), 34100);
/// ```
pub fn call_gas_cost(is_value_transfer: bool, is_new_account: bool, is_cold_access: bool) -> u64 {
    let mut cost = if is_cold_access {
        GAS_CALL_COLD
    } else {
        GAS_CALL_WARM
    };

    if is_value_transfer {
        cost += GAS_CALL_VALUE_TRANSFER;
        if is_new_account {
            cost += GAS_CALL_NEW_ACCOUNT;
        }
    }

    cost
}

/// Calculates the dynamic gas cost for LOG operations.
///
/// LOG operations have a base cost plus costs for topics and data:
/// `cost = 375 + (375 * num_topics) + (8 * data_size)`
///
/// # Arguments
///
/// * `num_topics` - Number of topics (0-4 for LOG0-LOG4)
/// * `data_size` - Size of log data in bytes
///
/// # Returns
///
/// The total gas cost for the LOG operation.
///
/// # Examples
///
/// ```
/// use claudeth::evm::gas::log_gas_cost;
///
/// // LOG0 with 32 bytes of data
/// assert_eq!(log_gas_cost(0, 32), 375 + 256);
///
/// // LOG1 with 1 topic and 64 bytes
/// assert_eq!(log_gas_cost(1, 64), 375 + 375 + 512);
///
/// // LOG4 with 4 topics and 256 bytes
/// assert_eq!(log_gas_cost(4, 256), 375 + 1500 + 2048);
/// ```
pub fn log_gas_cost(num_topics: u8, data_size: usize) -> u64 {
    GAS_LOG + (GAS_LOG_TOPIC * num_topics as u64) + (GAS_LOG_DATA * data_size as u64)
}

/// Calculates the dynamic gas cost for copy operations.
///
/// Operations like CALLDATACOPY, CODECOPY, RETURNDATACOPY, EXTCODECOPY, and MCOPY
/// have a base cost plus 3 gas per 32-byte word copied.
///
/// # Arguments
///
/// * `size` - Number of bytes to copy
///
/// # Returns
///
/// The gas cost for copying (3 gas per word).
///
/// # Examples
///
/// ```
/// use claudeth::evm::gas::copy_gas_cost;
///
/// // Copy 32 bytes (1 word)
/// assert_eq!(copy_gas_cost(32), 3);
///
/// // Copy 64 bytes (2 words)
/// assert_eq!(copy_gas_cost(64), 6);
///
/// // Copy 33 bytes (2 words, rounded up)
/// assert_eq!(copy_gas_cost(33), 6);
/// ```
pub fn copy_gas_cost(size: usize) -> u64 {
    let words = size.div_ceil(32); // Round up to nearest word
    GAS_COPY * words as u64
}

/// Calculates the dynamic gas cost for KECCAK256.
///
/// KECCAK256 has a base cost of 30 gas plus 6 gas per word (32 bytes) of data hashed.
///
/// # Arguments
///
/// * `size` - Number of bytes to hash
///
/// # Returns
///
/// The total gas cost for KECCAK256.
///
/// # Examples
///
/// ```
/// use claudeth::evm::gas::keccak256_gas_cost;
///
/// // Hash 0 bytes
/// assert_eq!(keccak256_gas_cost(0), 30);
///
/// // Hash 32 bytes (1 word)
/// assert_eq!(keccak256_gas_cost(32), 36);
///
/// // Hash 64 bytes (2 words)
/// assert_eq!(keccak256_gas_cost(64), 42);
/// ```
pub fn keccak256_gas_cost(size: usize) -> u64 {
    let words = size.div_ceil(32);
    GAS_KECCAK256 + (GAS_KECCAK256_WORD * words as u64)
}

/// Calculates the dynamic gas cost for EXP (exponentiation).
///
/// EXP has a base cost of 10 gas plus 50 gas per byte in the exponent.
///
/// # Arguments
///
/// * `exponent_bytes` - Number of non-zero bytes in the exponent
///
/// # Returns
///
/// The total gas cost for EXP.
///
/// # Examples
///
/// ```
/// use claudeth::evm::gas::exp_gas_cost;
///
/// // 2^0 (exponent is 0, which has 0 bytes)
/// assert_eq!(exp_gas_cost(0), 10);
///
/// // 2^255 (exponent has 1 byte: 0xFF)
/// assert_eq!(exp_gas_cost(1), 60);
///
/// // 2^65536 (exponent has 2 bytes: 0x01_00_00)
/// assert_eq!(exp_gas_cost(2), 110);
/// ```
pub fn exp_gas_cost(exponent_bytes: usize) -> u64 {
    GAS_EXP + (GAS_EXP_BYTE * exponent_bytes as u64)
}

/// Calculates the gas cost for CREATE2 init code hashing.
///
/// CREATE2 requires hashing the init code, which costs 6 gas per word.
///
/// # Arguments
///
/// * `init_code_size` - Size of init code in bytes
///
/// # Returns
///
/// The gas cost for hashing init code (6 gas per word).
///
/// # Examples
///
/// ```
/// use claudeth::evm::gas::create2_hash_cost;
///
/// // 32 bytes (1 word)
/// assert_eq!(create2_hash_cost(32), 6);
///
/// // 64 bytes (2 words)
/// assert_eq!(create2_hash_cost(64), 12);
/// ```
pub fn create2_hash_cost(init_code_size: usize) -> u64 {
    let words = init_code_size.div_ceil(32);
    GAS_CREATE2_WORD * words as u64
}

/// Calculates the gas cost for init code in CREATE/CREATE2 (EIP-3860).
///
/// Init code costs 2 gas per word, applied to both CREATE and CREATE2.
///
/// # Arguments
///
/// * `init_code_size` - Size of init code in bytes
///
/// # Returns
///
/// The gas cost for init code (2 gas per word).
///
/// # Examples
///
/// ```
/// use claudeth::evm::gas::init_code_gas_cost;
///
/// // 32 bytes (1 word)
/// assert_eq!(init_code_gas_cost(32), 2);
///
/// // 1024 bytes (32 words)
/// assert_eq!(init_code_gas_cost(1024), 64);
/// ```
pub fn init_code_gas_cost(init_code_size: usize) -> u64 {
    let words = init_code_size.div_ceil(32);
    GAS_INIT_CODE_WORD * words as u64
}

/// Calculates the gas cost for code deposit during contract creation.
///
/// When a contract is created, storing its code costs 200 gas per byte.
///
/// # Arguments
///
/// * `code_size` - Size of deployed code in bytes
///
/// # Returns
///
/// The gas cost for code deposit (200 gas per byte).
///
/// # Examples
///
/// ```
/// use claudeth::evm::gas::code_deposit_cost;
///
/// // 1 byte
/// assert_eq!(code_deposit_cost(1), 200);
///
/// // 1000 bytes
/// assert_eq!(code_deposit_cost(1000), 200000);
/// ```
pub fn code_deposit_cost(code_size: usize) -> u64 {
    GAS_CODE_DEPOSIT * code_size as u64
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Arithmetic Opcode Gas Cost Tests
    // =========================================================================

    #[test]
    fn test_arithmetic_opcodes() {
        assert_eq!(opcode_gas_cost(0x00), 0);   // STOP
        assert_eq!(opcode_gas_cost(0x01), 3);   // ADD
        assert_eq!(opcode_gas_cost(0x02), 5);   // MUL
        assert_eq!(opcode_gas_cost(0x03), 3);   // SUB
        assert_eq!(opcode_gas_cost(0x04), 5);   // DIV
        assert_eq!(opcode_gas_cost(0x05), 5);   // SDIV
        assert_eq!(opcode_gas_cost(0x06), 5);   // MOD
        assert_eq!(opcode_gas_cost(0x07), 5);   // SMOD
        assert_eq!(opcode_gas_cost(0x08), 8);   // ADDMOD
        assert_eq!(opcode_gas_cost(0x09), 8);   // MULMOD
        assert_eq!(opcode_gas_cost(0x0A), 10);  // EXP (base)
        assert_eq!(opcode_gas_cost(0x0B), 5);   // SIGNEXTEND
    }

    #[test]
    fn test_comparison_opcodes() {
        assert_eq!(opcode_gas_cost(0x10), 3);   // LT
        assert_eq!(opcode_gas_cost(0x11), 3);   // GT
        assert_eq!(opcode_gas_cost(0x12), 3);   // SLT
        assert_eq!(opcode_gas_cost(0x13), 3);   // SGT
        assert_eq!(opcode_gas_cost(0x14), 3);   // EQ
        assert_eq!(opcode_gas_cost(0x15), 3);   // ISZERO
    }

    #[test]
    fn test_bitwise_opcodes() {
        assert_eq!(opcode_gas_cost(0x16), 3);   // AND
        assert_eq!(opcode_gas_cost(0x17), 3);   // OR
        assert_eq!(opcode_gas_cost(0x18), 3);   // XOR
        assert_eq!(opcode_gas_cost(0x19), 3);   // NOT
        assert_eq!(opcode_gas_cost(0x1A), 3);   // BYTE
        assert_eq!(opcode_gas_cost(0x1B), 3);   // SHL
        assert_eq!(opcode_gas_cost(0x1C), 3);   // SHR
        assert_eq!(opcode_gas_cost(0x1D), 3);   // SAR
    }

    #[test]
    fn test_hash_opcode() {
        assert_eq!(opcode_gas_cost(0x20), 30);  // KECCAK256 (base)
    }

    #[test]
    fn test_environment_opcodes() {
        assert_eq!(opcode_gas_cost(0x30), 2);     // ADDRESS
        assert_eq!(opcode_gas_cost(0x31), 2600);  // BALANCE (cold)
        assert_eq!(opcode_gas_cost(0x32), 2);     // ORIGIN
        assert_eq!(opcode_gas_cost(0x33), 2);     // CALLER
        assert_eq!(opcode_gas_cost(0x34), 2);     // CALLVALUE
        assert_eq!(opcode_gas_cost(0x35), 3);     // CALLDATALOAD
        assert_eq!(opcode_gas_cost(0x36), 2);     // CALLDATASIZE
        assert_eq!(opcode_gas_cost(0x37), 3);     // CALLDATACOPY (base)
        assert_eq!(opcode_gas_cost(0x38), 2);     // CODESIZE
        assert_eq!(opcode_gas_cost(0x39), 3);     // CODECOPY (base)
        assert_eq!(opcode_gas_cost(0x3A), 2);     // GASPRICE
        assert_eq!(opcode_gas_cost(0x3B), 2600);  // EXTCODESIZE (cold)
        assert_eq!(opcode_gas_cost(0x3C), 2600);  // EXTCODECOPY (cold)
        assert_eq!(opcode_gas_cost(0x3D), 2);     // RETURNDATASIZE
        assert_eq!(opcode_gas_cost(0x3E), 3);     // RETURNDATACOPY (base)
        assert_eq!(opcode_gas_cost(0x3F), 2600);  // EXTCODEHASH (cold)
    }

    #[test]
    fn test_block_opcodes() {
        assert_eq!(opcode_gas_cost(0x40), 20);  // BLOCKHASH
        assert_eq!(opcode_gas_cost(0x41), 2);   // COINBASE
        assert_eq!(opcode_gas_cost(0x42), 2);   // TIMESTAMP
        assert_eq!(opcode_gas_cost(0x43), 2);   // NUMBER
        assert_eq!(opcode_gas_cost(0x44), 2);   // DIFFICULTY
        assert_eq!(opcode_gas_cost(0x45), 2);   // GASLIMIT
        assert_eq!(opcode_gas_cost(0x46), 2);   // CHAINID
        assert_eq!(opcode_gas_cost(0x47), 5);   // SELFBALANCE
        assert_eq!(opcode_gas_cost(0x48), 2);   // BASEFEE
        assert_eq!(opcode_gas_cost(0x49), 3);   // BLOBHASH
        assert_eq!(opcode_gas_cost(0x4A), 2);   // BLOBBASEFEE
    }

    #[test]
    fn test_stack_memory_storage_opcodes() {
        assert_eq!(opcode_gas_cost(0x50), 2);     // POP
        assert_eq!(opcode_gas_cost(0x51), 3);     // MLOAD (base)
        assert_eq!(opcode_gas_cost(0x52), 3);     // MSTORE (base)
        assert_eq!(opcode_gas_cost(0x53), 3);     // MSTORE8 (base)
        assert_eq!(opcode_gas_cost(0x54), 2100);  // SLOAD (cold)
        assert_eq!(opcode_gas_cost(0x55), 0);     // SSTORE (dynamic only)
        assert_eq!(opcode_gas_cost(0x56), 8);     // JUMP
        assert_eq!(opcode_gas_cost(0x57), 10);    // JUMPI
        assert_eq!(opcode_gas_cost(0x58), 2);     // PC
        assert_eq!(opcode_gas_cost(0x59), 2);     // MSIZE
        assert_eq!(opcode_gas_cost(0x5A), 2);     // GAS
        assert_eq!(opcode_gas_cost(0x5B), 1);     // JUMPDEST
        assert_eq!(opcode_gas_cost(0x5C), 100);   // TLOAD
        assert_eq!(opcode_gas_cost(0x5D), 100);   // TSTORE
        assert_eq!(opcode_gas_cost(0x5E), 3);     // MCOPY
        assert_eq!(opcode_gas_cost(0x5F), 2);     // PUSH0
    }

    #[test]
    fn test_push_opcodes() {
        // PUSH1-PUSH32
        for i in 0x60..=0x7F {
            assert_eq!(opcode_gas_cost(i), 3);
        }
    }

    #[test]
    fn test_dup_opcodes() {
        // DUP1-DUP16
        for i in 0x80..=0x8F {
            assert_eq!(opcode_gas_cost(i), 3);
        }
    }

    #[test]
    fn test_swap_opcodes() {
        // SWAP1-SWAP16
        for i in 0x90..=0x9F {
            assert_eq!(opcode_gas_cost(i), 3);
        }
    }

    #[test]
    fn test_log_opcodes() {
        // LOG0-LOG4
        for i in 0xA0..=0xA4 {
            assert_eq!(opcode_gas_cost(i), 375);
        }
    }

    #[test]
    fn test_system_opcodes() {
        assert_eq!(opcode_gas_cost(0xF0), 32000); // CREATE
        assert_eq!(opcode_gas_cost(0xF1), 2600);  // CALL (cold)
        assert_eq!(opcode_gas_cost(0xF2), 2600);  // CALLCODE (cold)
        assert_eq!(opcode_gas_cost(0xF3), 0);     // RETURN (base)
        assert_eq!(opcode_gas_cost(0xF4), 2600);  // DELEGATECALL (cold)
        assert_eq!(opcode_gas_cost(0xF5), 32000); // CREATE2 (base)
        assert_eq!(opcode_gas_cost(0xFA), 2600);  // STATICCALL (cold)
        assert_eq!(opcode_gas_cost(0xFD), 0);     // REVERT (base)
        assert_eq!(opcode_gas_cost(0xFE), 0);     // INVALID
        assert_eq!(opcode_gas_cost(0xFF), 5000);  // SELFDESTRUCT (base)
    }

    #[test]
    fn test_invalid_opcodes() {
        // Undefined opcodes should return 0
        assert_eq!(opcode_gas_cost(0x0C), 0);
        assert_eq!(opcode_gas_cost(0x21), 0);
        assert_eq!(opcode_gas_cost(0xA5), 0);
    }

    // =========================================================================
    // Memory Expansion Tests
    // =========================================================================

    #[test]
    fn test_memory_expansion_no_expansion() {
        assert_eq!(memory_expansion_cost(0, 0), 0);
        assert_eq!(memory_expansion_cost(100, 100), 0);
        assert_eq!(memory_expansion_cost(1000, 500), 0); // new < old
    }

    #[test]
    fn test_memory_expansion_first_word() {
        // Expanding from 0 to 32 bytes (1 word)
        // Cost = (1*1)/512 + 3*1 = 0 + 3 = 3
        assert_eq!(memory_expansion_cost(0, 32), 3);
    }

    #[test]
    fn test_memory_expansion_second_word() {
        // Expanding from 32 to 64 bytes (1 to 2 words)
        // Cost for 2 words = (2*2)/512 + 3*2 = 0 + 6 = 6
        // Cost for 1 word = (1*1)/512 + 3*1 = 0 + 3 = 3
        // Additional cost = 6 - 3 = 3
        assert_eq!(memory_expansion_cost(32, 64), 3);
    }

    #[test]
    fn test_memory_expansion_partial_word() {
        // Expanding from 0 to 33 bytes (rounds up to 2 words)
        // Cost for 2 words = (2*2)/512 + 3*2 = 0 + 6 = 6
        assert_eq!(memory_expansion_cost(0, 33), 6);

        // Expanding from 0 to 63 bytes (rounds up to 2 words)
        assert_eq!(memory_expansion_cost(0, 63), 6);

        // Expanding from 0 to 64 bytes (exactly 2 words)
        assert_eq!(memory_expansion_cost(0, 64), 6);
    }

    #[test]
    fn test_memory_expansion_large() {
        // Expanding to 1024 bytes (32 words)
        // Cost = (32*32)/512 + 3*32 = 2 + 96 = 98
        assert_eq!(memory_expansion_cost(0, 1024), 98);

        // Expanding from 1024 to 2048 bytes (32 to 64 words)
        // Cost for 64 words = (64*64)/512 + 3*64 = 8 + 192 = 200
        // Cost for 32 words = 98
        // Additional cost = 200 - 98 = 102
        assert_eq!(memory_expansion_cost(1024, 2048), 102);
    }

    #[test]
    fn test_memory_expansion_quadratic_growth() {
        // Verify quadratic nature: larger expansions cost more per word
        let cost_0_to_1024 = memory_expansion_cost(0, 1024);
        let cost_1024_to_2048 = memory_expansion_cost(1024, 2048);

        // Second expansion should cost more (quadratic)
        assert!(cost_1024_to_2048 > cost_0_to_1024);
    }

    // =========================================================================
    // Call Gas Cost Tests
    // =========================================================================

    #[test]
    fn test_call_gas_simple_warm() {
        // Simple call to warm account, no value transfer
        assert_eq!(call_gas_cost(false, false, false), 100);
    }

    #[test]
    fn test_call_gas_cold_access() {
        // First access to account (cold)
        assert_eq!(call_gas_cost(false, false, true), 2600);
    }

    #[test]
    fn test_call_gas_value_transfer_warm() {
        // Value transfer to existing warm account
        assert_eq!(call_gas_cost(true, false, false), 9100);
    }

    #[test]
    fn test_call_gas_value_transfer_cold() {
        // Value transfer to cold account
        assert_eq!(call_gas_cost(true, false, true), 11600);
    }

    #[test]
    fn test_call_gas_new_account_warm() {
        // Call with value to new account (warm)
        assert_eq!(call_gas_cost(true, true, false), 34100);
    }

    #[test]
    fn test_call_gas_new_account_cold() {
        // Call with value to new account (cold) - most expensive
        assert_eq!(call_gas_cost(true, true, true), 36600);
    }

    #[test]
    fn test_call_gas_new_account_no_value() {
        // New account flag is irrelevant without value transfer
        assert_eq!(call_gas_cost(false, true, false), 100);
        assert_eq!(call_gas_cost(false, true, true), 2600);
    }

    // =========================================================================
    // LOG Gas Cost Tests
    // =========================================================================

    #[test]
    fn test_log0_no_data() {
        // LOG0 with no data: 375 base
        assert_eq!(log_gas_cost(0, 0), 375);
    }

    #[test]
    fn test_log0_with_data() {
        // LOG0 with 32 bytes: 375 + 8*32 = 631
        assert_eq!(log_gas_cost(0, 32), 631);
    }

    #[test]
    fn test_log1() {
        // LOG1 with 1 topic, 32 bytes: 375 + 375 + 8*32 = 1006
        assert_eq!(log_gas_cost(1, 32), 1006);
    }

    #[test]
    fn test_log4_large_data() {
        // LOG4 with 4 topics, 256 bytes: 375 + 4*375 + 8*256 = 3923
        assert_eq!(log_gas_cost(4, 256), 3923);
    }

    #[test]
    fn test_log_formula() {
        for topics in 0..=4 {
            for size in [0, 32, 64, 128, 256] {
                let expected = 375 + (375 * topics as u64) + (8 * size as u64);
                assert_eq!(log_gas_cost(topics, size), expected);
            }
        }
    }

    // =========================================================================
    // Copy Gas Cost Tests
    // =========================================================================

    #[test]
    fn test_copy_zero_bytes() {
        assert_eq!(copy_gas_cost(0), 0);
    }

    #[test]
    fn test_copy_one_word() {
        assert_eq!(copy_gas_cost(32), 3);
    }

    #[test]
    fn test_copy_two_words() {
        assert_eq!(copy_gas_cost(64), 6);
    }

    #[test]
    fn test_copy_partial_word() {
        // 33 bytes = 2 words (rounded up)
        assert_eq!(copy_gas_cost(33), 6);

        // 1 byte = 1 word
        assert_eq!(copy_gas_cost(1), 3);

        // 63 bytes = 2 words
        assert_eq!(copy_gas_cost(63), 6);
    }

    // =========================================================================
    // KECCAK256 Gas Cost Tests
    // =========================================================================

    #[test]
    fn test_keccak256_empty() {
        // Empty data: 30 base
        assert_eq!(keccak256_gas_cost(0), 30);
    }

    #[test]
    fn test_keccak256_one_word() {
        // 32 bytes: 30 + 6*1 = 36
        assert_eq!(keccak256_gas_cost(32), 36);
    }

    #[test]
    fn test_keccak256_two_words() {
        // 64 bytes: 30 + 6*2 = 42
        assert_eq!(keccak256_gas_cost(64), 42);
    }

    #[test]
    fn test_keccak256_partial_word() {
        // 33 bytes = 2 words: 30 + 6*2 = 42
        assert_eq!(keccak256_gas_cost(33), 42);
    }

    // =========================================================================
    // EXP Gas Cost Tests
    // =========================================================================

    #[test]
    fn test_exp_zero_exponent() {
        // x^0: 10 base
        assert_eq!(exp_gas_cost(0), 10);
    }

    #[test]
    fn test_exp_one_byte() {
        // Exponent fits in 1 byte: 10 + 50*1 = 60
        assert_eq!(exp_gas_cost(1), 60);
    }

    #[test]
    fn test_exp_multi_byte() {
        // Exponent needs 8 bytes: 10 + 50*8 = 410
        assert_eq!(exp_gas_cost(8), 410);
    }

    // =========================================================================
    // CREATE2 Hash Cost Tests
    // =========================================================================

    #[test]
    fn test_create2_hash_one_word() {
        assert_eq!(create2_hash_cost(32), 6);
    }

    #[test]
    fn test_create2_hash_large_code() {
        // 1024 bytes = 32 words: 6*32 = 192
        assert_eq!(create2_hash_cost(1024), 192);
    }

    // =========================================================================
    // Init Code Gas Cost Tests
    // =========================================================================

    #[test]
    fn test_init_code_one_word() {
        assert_eq!(init_code_gas_cost(32), 2);
    }

    #[test]
    fn test_init_code_large() {
        // 1024 bytes = 32 words: 2*32 = 64
        assert_eq!(init_code_gas_cost(1024), 64);
    }

    // =========================================================================
    // Code Deposit Cost Tests
    // =========================================================================

    #[test]
    fn test_code_deposit_one_byte() {
        assert_eq!(code_deposit_cost(1), 200);
    }

    #[test]
    fn test_code_deposit_max_size() {
        // Max code size is 24576 bytes
        assert_eq!(code_deposit_cost(MAX_CODE_SIZE), 200 * 24576);
    }

    #[test]
    fn test_code_deposit_typical() {
        // 1000 bytes of code
        assert_eq!(code_deposit_cost(1000), 200000);
    }

    // =========================================================================
    // Gas Constants Verification
    // =========================================================================

    #[test]
    fn test_gas_constants() {
        // Verify basic step costs
        assert_eq!(GAS_QUICK_STEP, 2);
        assert_eq!(GAS_FASTEST_STEP, 3);
        assert_eq!(GAS_FAST_STEP, 5);
        assert_eq!(GAS_MID_STEP, 8);
        assert_eq!(GAS_SLOW_STEP, 10);
        assert_eq!(GAS_EXT_STEP, 20);

        // Verify storage costs
        assert_eq!(GAS_SLOAD_COLD, 2100);
        assert_eq!(GAS_SLOAD_WARM, 100);
        assert_eq!(GAS_SSTORE_SET, 20000);

        // Verify call costs
        assert_eq!(GAS_CALL_COLD, 2600);
        assert_eq!(GAS_CALL_WARM, 100);
        assert_eq!(GAS_CALL_VALUE_TRANSFER, 9000);
        assert_eq!(GAS_CALL_NEW_ACCOUNT, 25000);

        // Verify creation costs
        assert_eq!(GAS_CREATE, 32000);
        assert_eq!(GAS_CREATE2, 32000);
        assert_eq!(GAS_CODE_DEPOSIT, 200);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_memory_expansion_edge_cases() {
        // Expansion by 1 byte
        let cost = memory_expansion_cost(0, 1);
        assert_eq!(cost, 3); // Rounds up to 1 word

        // Very large expansion
        let cost = memory_expansion_cost(0, 100000);
        assert!(cost > 0);
    }

    #[test]
    fn test_log_max_topics() {
        // LOG4 is the maximum (4 topics)
        let cost = log_gas_cost(4, 0);
        assert_eq!(cost, 375 + 375 * 4);
    }
}
