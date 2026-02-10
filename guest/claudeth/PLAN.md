# Claudeth Implementation Plan

Date: 2026-02-10

## Summary

Claudeth is a minimal-dependency Ethereum STF guest targeting `no_std` on
`riscv32im-unknown-none-elf`. It implements a full EVM interpreter, block
processing with header validation and root checks, partial MPT proofs, and
witness-based state reconstruction (WITNESS v1). Cancun blob transactions
(type 0x03) and post-Shanghai fields are supported.

## Verified Status (from code)

### Implemented

- EVM interpreter with full opcode coverage, including Cancun opcodes
  (`BLOBHASH`, `BLOBBASEFEE`, `TLOAD`, `TSTORE`, `PREVRANDAO`).
- Transaction validation and execution for Legacy, EIP-2930, EIP-1559, and
  EIP-4844 blob transactions.
- EIP-4844 blob tx encoding/decoding, signing hash, blob fee validation, and
  blob gas accounting.
- Block processing with parent header validation, receipts/tx/state root checks,
  logs bloom validation, gas used checks, and blob gas used checks.
- EIP-4895 withdrawals application and withdrawals root validation.
- EIP-4788 beacon root system call and EIP-2935 history storage system call.
- Guest input decoding supports optional recent block hashes for BLOCKHASH and
  withdrawals list when `withdrawals_root` is present.
- Partial MPT implementation with inclusion/exclusion proof verification.
- Witness-based state reconstruction from WITNESS v1 (account/storage proofs).
- In-tree secp256k1 field/point arithmetic and ECDSA verify/recover; tests use
  fixed signature vectors.

### Known Gaps / Limitations

- No in-tree signer yet; tests rely on fixed signature vectors.
- EELS blockchain fixtures are external and ignored by default.

## Testing Status

- `cargo test -p claudeth --release` (2026-02-10): pass.
- `prek run` (2026-02-10): pass (no applicable files).

## Plan

### Done

- Witness RLP decoding alongside `state_entries` input.
- Account/storage proof verification and code hash validation.
- Witness parsing tests for valid/invalid cases.
- In-tree finite-field helpers and curve constants.
- Affine point arithmetic and ECDSA verify/recover.
- Removed k256 dependency; tests use fixed signature vectors.
- Executor validates blob versioned hashes (non-empty, count limit, version byte).
- Enforced EIP-3860 initcode size limits for contract-creation transactions and
  CREATE/CREATE2 (reject > 49,152 bytes) with tests.

### Backlog (Not Scheduled)

- Add an in-tree signer for generating ECDSA signatures in tests.
- Integrate EELS blockchain fixtures into CI (still optional by default).

## Immediate Next Task

No immediate task queued; next work should come from new gaps or failing tests.
