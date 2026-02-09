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
- Header validation includes base fee per gas and excess blob gas against parent
  (execution-specs)
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
- `no_std` riscv32 guest entry and bump allocator

### Known Gaps / Limitations

- EIP-4844 blob transactions (type 0x03) not implemented
- Blob gas accounting (block `blob_gas_used` and per-tx blob data fee) is not
  implemented
- Witness-based state reconstruction not implemented
- `k256` dependency still required for secp256k1
- EELS blockchain fixtures are external and ignored by default

## Testing Status

- `cargo test -p claudeth --release` (2026-02-09): pass
- `prek run` (2026-02-09): pass (no files to check)

## Plan

### P1: Add EIP-4844 Blob Transaction Support (Type 0x03)

1. **Plumb blob versioned hashes into execution** (done)
   - Extend `TxContext` to carry `blob_versioned_hashes`.
   - Update `RecursiveHost::blobhash` to read from `TxContext`.
   - Ensure all tx contexts are populated (empty for non-blob txs).
2. **Introduce BlobTransaction type**
   - Add type `0x03` decoding/encoding and signing hash (EIP-4844).
   - Include `max_fee_per_blob_gas` and `blob_versioned_hashes`.
3. **Validation and fee checks**
   - Enforce `to` is non-null for blob txs.
   - Validate blob versioned hash version byte and non-empty list.
   - Enforce `max_fee_per_blob_gas >= blob_gas_price`.
4. **Block-level blob gas accounting**
   - Track `blob_gas_used` per tx and validate against header.
   - Charge blob data fee and account for balance effects.

### P3: Witness-Based State Reconstruction

### P4: Remove `k256`

## Immediate Next Task

Introduce `BlobTransaction` (type 0x03) decoding/encoding and signing hash.
