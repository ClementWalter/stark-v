# Claudeth Implementation Plan

Date: 2026-02-09

## Summary

Claudeth is a minimal-dependency Ethereum STF guest targeting `no_std` on
`riscv32im-unknown-none-elf`. It includes a full EVM interpreter, block
processing with header validations and root checks, a partial MPT, and
EIP-4895 withdrawals application. The block header type includes
Shanghai/Cancun fields. Block processing applies the EIP-4788 beacon root and
EIP-2935 historical block hashes system calls.

## Verified Status (from code)

### Implemented

- EVM interpreter with full opcode coverage, including `BLOBHASH`,
  `BLOBBASEFEE`, `TLOAD`, `TSTORE`, `PREVRANDAO`
- `BLOBBASEFEE` uses the execution-specs Taylor expansion formula when
  `excess_blob_gas` is present in the block header
- Transaction types: Legacy / EIP-2930 / EIP-1559
- Block processing: parent header validation, tx execution, receipts, gas used,
  validation of receipts root, tx root, logs bloom, state root
- Block header validation includes gas-used bounds, extra data length, and
  post-merge invariants (including empty ommers hash)
- EIP-4895 withdrawals application and withdrawals root validation
- EIP-4788 beacon root system call during block processing
- EIP-2935 historical block hashes system call during block processing
- Partial MPT with proof support
- Block header type supports Shanghai/Cancun fields
  (`withdrawals_root`, `blob_gas_used`, `excess_blob_gas`,
  `parent_beacon_block_root`)
- Guest input decoding supports withdrawals when `withdrawals_root` is present
- Guest input decoding supports optional recent block hashes for BLOCKHASH
- `no_std` riscv32 guest entry and bump allocator

### Known Gaps / Limitations

- EIP-4844 blob transactions (type 0x03) not implemented
- Base fee and excess blob gas header validation not implemented
- Witness-based state reconstruction not implemented
- `k256` dependency still required for secp256k1
- EELS blockchain fixtures are external and ignored by default

## Testing Status

- `cargo test -p claudeth --release` (2026-02-09): pass
- `cargo clippy -p claudeth -- -D warnings` (2026-02-09): pass

## Plan

### Completed This Iteration

- Enforced `BlockHeader::validate()` at block processing entry (gas fields,
  extra data, post-merge checks)
- Added empty ommers hash validation for post-merge headers

### P1: Add EIP-4844 blob transaction support (type 0x03)

### P2: Header Validation for Base Fee / Excess Blob Gas

- Validate base fee per gas and excess blob gas against parent (execution-specs)

### P3: Witness-Based State Reconstruction

### P4: Remove `k256`

## Immediate Next Task

Implement EIP-4844 blob transaction support (type 0x03).
