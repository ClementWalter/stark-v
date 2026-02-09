# Claudeth Implementation Plan

Date: 2026-02-09

## Summary

Claudeth is a minimal-dependency Ethereum STF guest targeting `no_std` on
`riscv32im-unknown-none-elf`. It includes a full EVM interpreter, block
processing with header validations and root checks, and a partial MPT. The
block header type includes Shanghai/Cancun fields, but block processing does
not yet apply those fork-specific system calls.

## Verified Status (from code)

### Implemented

- EVM interpreter with full opcode coverage, including `BLOBHASH`,
  `BLOBBASEFEE`, `TLOAD`, `TSTORE`, `PREVRANDAO`
- Transaction types: Legacy / EIP-2930 / EIP-1559
- Block processing: header validation, tx execution, receipts, gas used,
  validation of receipts root, tx root, logs bloom, state root
- Partial MPT with proof support
- Block header type supports Shanghai/Cancun fields
  (`withdrawals_root`, `blob_gas_used`, `excess_blob_gas`,
  `parent_beacon_block_root`, `requests_hash`)
- `no_std` riscv32 guest entry and bump allocator

### Known Gaps / Limitations

- EIP-4788 beacon root system call not implemented
- EIP-4895 withdrawals not applied in block processing
- EIP-2935 historical block hashes system call not implemented
- Guest input decoding does not include withdrawals or recent block hashes
- Witness-based state reconstruction not implemented
- `k256` dependency still required for secp256k1
- EELS blockchain fixtures are external and ignored by default

## Testing Status

- `cargo test -p claudeth --release` (2026-02-09): 93 passed, 0 failed
- `cargo clippy -p claudeth -- -D warnings` (2026-02-09): pass

## Plan

### Completed This Iteration

- Fixed 55 compilation errors blocking the build:
  - Removed nonexistent `StorageWrite` re-export from `evm/mod.rs`
  - Made `compute_create_address` and `compute_create2_address` `pub` in `evm/host.rs`
  - Added `From<arithmetic::EvmError>` impl for `evm::error::EvmError`

### P0: Implement EIP-4895 withdrawals

- Add withdrawals application after transactions in `process_block`.
- Validate withdrawals root against the header.
- Extend guest input decoding to accept withdrawals list when
  `withdrawals_root` is present.

### P1: Implement EIP-4788 beacon root system call

### P2: Implement EIP-2935 historical block hashes system call

### P3: Add EIP-4844 blob transaction support (type 0x03)

### P4: Witness-Based State Reconstruction

### P5: Remove `k256`

## Immediate Next Task

Implement EIP-4895 withdrawals in block processing and guest input decoding.
