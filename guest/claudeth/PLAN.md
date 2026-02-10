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
- EIP-6780 SELFDESTRUCT semantics: immediate balance transfer, deletion only for
  contracts created in the same transaction, with created-account tracking.
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
- In-tree secp256k1 field/point arithmetic and ECDSA verify/recover.
- Deterministic in-tree secp256k1 signer for tests.
- EIP-3860 initcode size limits enforced for creation txs and CREATE/CREATE2.
- EIP-170 max code size enforcement and code-deposit gas charging for CREATE/CREATE2.

### Known Gaps / Limitations

- EELS blockchain fixtures are external and ignored by default.

## Testing Status

- `cargo test -p claudeth --release` (2026-02-10): pass.
- `prek run` (2026-02-10): pass.

## Plan

### Done

- Witness RLP decoding alongside `state_entries` input.
- Account/storage proof verification and code hash validation.
- Witness parsing tests for valid/invalid cases.
- In-tree finite-field helpers and curve constants.
- Affine point arithmetic and ECDSA verify/recover.
- Removed k256 dependency; tests now sign with in-tree code.
- Executor validates blob versioned hashes (non-empty, count limit, version byte).
- Enforced EIP-3860 initcode size limits for contract-creation transactions and
  CREATE/CREATE2 (reject > 49,152 bytes) with tests.
- Enforced EIP-170 max code size and code-deposit gas charging for CREATE/CREATE2
  with tests.
- Treat REVERT as non-exceptional: convert REVERT to `success=false` execution
  results and only apply execution state on success.
- Enforce EIP-3541 (reject contract code starting with 0xEF) for tx creation and
  CREATE/CREATE2 paths, consuming all remaining gas on failure.
- Deterministic in-tree signer used by signature-related tests.
- Apply EIP-6780 SELFDESTRUCT rules: transfer balance immediately, delete only
  if created in the same transaction, and clear created-account tracking per tx.

### Backlog (Not Scheduled)

- Integrate EELS blockchain fixtures into CI (still optional by default).

## Immediate Next Task

Integrate EELS blockchain fixtures into CI (still optional by default).
