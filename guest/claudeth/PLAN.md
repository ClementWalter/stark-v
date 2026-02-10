# Claudeth Implementation Plan

Date: 2026-02-10

## Summary

Claudeth is a minimal-dependency Ethereum STF guest targeting `no_std` on
`riscv32im-unknown-none-elf`. It includes a full EVM interpreter, block
processing with header validations and root checks, a partial MPT (with
inclusion/exclusion proof verification), and EIP-4895 withdrawals application.
Block processing applies EIP-4788 (beacon root) and EIP-2935 (history storage)
system calls before transaction execution. The block header type includes
Shanghai/Cancun fields.

## Verified Status (from code)

### Implemented

- EVM interpreter with full opcode coverage, including `BLOBHASH`,
  `BLOBBASEFEE`, `TLOAD`, `TSTORE`, `PREVRANDAO`.
- `BLOBBASEFEE` uses the execution-specs Taylor expansion formula when
  `excess_blob_gas` is present.
- Transaction types: Legacy / EIP-2930 / EIP-1559 / EIP-4844 blob (type 0x03).
- Block processing: parent header validation, tx execution, receipts, gas used,
  validation of receipts root, tx root, logs bloom, state root.
- Header validation includes base fee per gas and excess blob gas against
  parent.
- EIP-4895 withdrawals application and withdrawals root validation.
- EIP-4788 beacon root system call during block processing.
- EIP-2935 historical block hashes system call during block processing.
- Partial MPT with inclusion/exclusion proof verification (RLP node proofs).
- Block header type supports Shanghai/Cancun fields
  (`withdrawals_root`, `blob_gas_used`, `excess_blob_gas`,
  `parent_beacon_block_root`).
- Guest input decoding supports optional recent block hashes for BLOCKHASH.
- Guest input decoding accepts withdrawals list when `withdrawals_root` is
  present (empty list allowed).
- Guest input validates recent block hashes length ≤ min(block number, 256)
  and requires the last hash to match the parent hash when provided.
- Block processing tests cover empty withdrawals list with withdrawals root
  set.
- Witness RLP decoding with account/storage proof validation builds the initial
  `State` from `WITNESS.md`.
- `TxContext` carries blob versioned hashes; `RecursiveHost::blobhash` reads
  from `TxContext`.
- Blob transactions populate `TxContext.blob_versioned_hashes`.
- Blob data fee charged from sender and block blob gas used tracked/validated.
- Base fee validation enforced for legacy/EIP-2930 (`gas_price >= base_fee`)
  and EIP-1559/EIP-4844 (`max_fee_per_gas >= base_fee`).
- Logs bloom bit ordering matches execution-specs (reversed 11-bit index,
  MSB-first within bytes).
- EIP-2 signature bounds enforced (`r/s` range, low-`s`, and `v/y_parity`).
- `no_std` riscv32 guest entry and bump allocator.
- Deterministic state root computation by sorting account addresses before trie
  insertion.
- Witness format v1 draft defined in `WITNESS.md`.

### Known Gaps / Limitations

- `k256` dependency still required for secp256k1.
- EELS blockchain fixtures are external and ignored by default.

## Testing Status

- `cargo test -p claudeth --release` (2026-02-10): pass.
- `prek run` (2026-02-10): pass.

## Plan

### P1: Witness-Based State Reconstruction

- Implemented: witness RLP decoding alongside existing `state_entries` input.
- Implemented: account/storage proof verification and code hash validation.
- Implemented: witness parsing tests for valid/invalid cases.

### P2: Remove `k256`

- Replace `k256` with in-tree secp256k1 implementation.

## Immediate Next Task

Replace `k256` with in-tree secp256k1 implementation.
