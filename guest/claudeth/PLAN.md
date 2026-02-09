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
- Transaction types: Legacy / EIP-2930 / EIP-1559
- EIP-4844 blob transaction type `0x03` decoding/encoding and signing hash
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
- Guest input decoding supports withdrawals when `withdrawals_root` is present
- Guest input decoding supports optional recent block hashes for BLOCKHASH
- `TxContext` carries blob versioned hashes; `RecursiveHost::blobhash` reads
  from `TxContext`
- Blob transactions populate `TxContext.blob_versioned_hashes`
- `no_std` riscv32 guest entry and bump allocator

### Known Gaps / Limitations

- EIP-4844 blob transaction validation is incomplete (blob hash version,
  non-empty blob list, max fee per blob gas)
- Blob gas accounting (`blob_gas_used`, per-tx blob data fee) is not implemented
- Witness-based state reconstruction is not implemented
- `k256` dependency still required for secp256k1
- EELS blockchain fixtures are external and ignored by default

## Testing Status

- `cargo test -p claudeth --release` (2026-02-09): pass
- `prek run` (2026-02-09): pass (no files to check)

## Plan

### P1: Add EIP-4844 Blob Transaction Support (Type `0x03`)

1. **Introduce `BlobTransaction` type** (done)
   - Add type `0x03` decoding/encoding and signing hash (EIP-4844).
   - Include `max_fee_per_blob_gas` and `blob_versioned_hashes`.
2. **Validation and fee checks**
   - Enforce `to` is non-null for blob txs.
   - Validate blob versioned hash version byte and non-empty list.
   - Enforce `max_fee_per_blob_gas >= blob_gas_price`.
3. **Execution plumbing**
   - Populate `TxContext.blob_versioned_hashes` from blob txs.
4. **Block-level blob gas accounting**
   - Track `blob_gas_used` per tx and validate against header.
   - Charge blob data fee and account for balance effects.

### P2: Witness-Based State Reconstruction

### P3: Remove `k256`

## Immediate Next Task

Introduce `BlobTransaction` (type `0x03`) decoding/encoding and signing hash.
