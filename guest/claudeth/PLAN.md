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
  `excess_blob_gas` is present
- Transaction types: Legacy / EIP-2930 / EIP-1559 / EIP-4844 blob (type 0x03)
- Block processing: parent header validation, tx execution, receipts, gas used,
  validation of receipts root, tx root, logs bloom, state root
- Header validation includes base fee per gas and excess blob gas against parent
- EIP-4895 withdrawals application and withdrawals root validation
- EIP-4788 beacon root system call during block processing
- EIP-2935 historical block hashes system call during block processing
- Partial MPT with proof support
- Block header type supports Shanghai/Cancun fields
  (`withdrawals_root`, `blob_gas_used`, `excess_blob_gas`,
  `parent_beacon_block_root`)
- Guest input decoding supports optional recent block hashes for BLOCKHASH
- Guest input decoding accepts withdrawals list when `withdrawals_root` is
  present (empty list allowed)
- Block processing tests cover empty withdrawals list with withdrawals root set
- `TxContext` carries blob versioned hashes; `RecursiveHost::blobhash` reads
  from `TxContext`
- Blob transactions populate `TxContext.blob_versioned_hashes`
- Blob data fee charged from sender and block blob gas used tracked/validated
- `no_std` riscv32 guest entry and bump allocator
- Deterministic state root computation by sorting account addresses before trie
  insertion

### Known Gaps / Limitations

- Witness-based state reconstruction is not implemented
- `k256` dependency still required for secp256k1
- EELS blockchain fixtures are external and ignored by default

## Testing Status

- `cargo test -p claudeth --release` (2026-02-09): pass
- `prek run` (2026-02-09): pass

## Plan

### P1: Witness-Based State Reconstruction

- Specify witness format (accounts, storage proofs, code) and validation rules.
- Implement proof verification and minimal state reconstruction in `State`.

### P2: Remove `k256`

- Replace `k256` with in-tree secp256k1 implementation.

## Immediate Next Task

Define the witness format and validation rules for P1.
