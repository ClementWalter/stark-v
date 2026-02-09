# Claudeth Implementation Plan

Date: 2026-02-09

## Summary

Claudeth is a minimal-dependency Ethereum STF guest targeting `no_std` on
`riscv32im-unknown-none-elf`. It includes a full EVM interpreter, block
processing with header validations and root checks, a partial MPT, and
EIP-4895 withdrawals application. The block header type includes
Shanghai/Cancun fields, but block processing does not yet apply the Cancun
system calls.

## Verified Status (from code)

### Implemented

- EVM interpreter with full opcode coverage, including `BLOBHASH`,
  `BLOBBASEFEE`, `TLOAD`, `TSTORE`, `PREVRANDAO`
- Transaction types: Legacy / EIP-2930 / EIP-1559
- Block processing: header validation, tx execution, receipts, gas used,
  validation of receipts root, tx root, logs bloom, state root
- EIP-4895 withdrawals application and withdrawals root validation
- Partial MPT with proof support
- Block header type supports Shanghai/Cancun fields
  (`withdrawals_root`, `blob_gas_used`, `excess_blob_gas`,
  `parent_beacon_block_root`, `requests_hash`)
- Guest input decoding supports withdrawals when `withdrawals_root` is present
- `no_std` riscv32 guest entry and bump allocator

### Known Gaps / Limitations

- EIP-4788 beacon root system call not implemented
- EIP-2935 historical block hashes system call not implemented
- Guest input decoding does not include recent block hashes
- Witness-based state reconstruction not implemented
- `k256` dependency still required for secp256k1
- EELS blockchain fixtures are external and ignored by default

## Testing Status

- `cargo test -p claudeth --release` (2026-02-09): pass (ignored EELS test not run)
- `cargo clippy -p claudeth -- -D warnings` (2026-02-09): pass

## Plan

### Completed This Iteration

- Implemented EIP-4895 withdrawals end-to-end:
  - Added `Withdrawal` type with RLP encoding/decoding
  - Applied withdrawals after transaction execution
  - Validated withdrawals root when present
  - Extended guest input decoding to accept withdrawals list
  - Added error codes and tests for withdrawals paths

### P1: Implement EIP-4788 beacon root system call

### P2: Implement EIP-2935 historical block hashes system call

### P3: Add EIP-4844 blob transaction support (type 0x03)

### P4: Witness-Based State Reconstruction

### P5: Remove `k256`

## Immediate Next Task

Implement EIP-4788 beacon root system call.
