# Claudeth Implementation Plan

Date: 2026-02-10

## Summary

Claudeth is a minimal-dependency Ethereum STF guest targeting `no_std` on
`riscv32im-unknown-none-elf`. It implements a full EVM interpreter, block
processing with header validation and root checks, partial MPT proofs, and
witness-based state reconstruction (WITNESS v1). Cancun blob transactions
(type `0x03`) and post-Shanghai fields are supported.

## Verified Status (code/README review, 2026-02-10)

### Implemented

- EVM interpreter with full opcode coverage, including Cancun opcodes
  (`BLOBHASH`, `BLOBBASEFEE`, `TLOAD`, `TSTORE`, `PREVRANDAO`) and post-Cancun
  execution semantics (EIP-6780, EIP-3541, EIP-3860, EIP-170).
- Transaction validation and execution for Legacy, EIP-2930, EIP-1559, and
  EIP-4844 blob transactions, including EIP-2718 typed envelopes.
- EIP-4844 blob tx encoding/decoding, signing hash, blob fee validation, and
  blob gas accounting.
- Block processing with parent header validation, receipts/tx/state root checks,
  logs bloom validation, gas used checks, and blob gas used checks.
- EIP-4895 withdrawals application and withdrawals root validation.
- EIP-4788 beacon root system call and EIP-2935 history storage system call.
- Guest input decoding supports optional recent block hashes for BLOCKHASH and
  withdrawals list when `withdrawals_root` is present; witness input detected
  via a versioned 3-item list (WITNESS v1).
- Partial MPT implementation with inclusion/exclusion proof verification.
- Witness-based state reconstruction from WITNESS v1 (account/storage proofs).
- In-tree secp256k1 field/point arithmetic and ECDSA verify/recover, with
  deterministic signer for tests.
- SSTORE gas/refund accounting (EIP-2200 + EIP-2929 + EIP-3529) with original
  storage tracking cleared at tx and system-call boundaries.
- Transient storage (EIP-1153) implementation and clearing.
- Coinbase receives only the priority fee; base fee and blob data fee are burned.

### Known Gaps / Limitations

- EELS blockchain fixtures are external and ignored by default.

## Testing Status

- `cargo test -p claudeth --release` (2026-02-10): pass (1 ignored EELS test).
- `prek run` (2026-02-10): pass (no eligible files).

## Plan

### Now

- No immediate implementation work identified.

### Done

- Use in-tree Keccak-256 for EIP-55 checksum generation.
- Correct EIP-55 checksum example casing in tests.
- WITNESS v1 decoding and proof validation integrated into guest input.
- Blob tx validation, blob fee charging, and blob gas accounting.
- EIP-6780 SELFDESTRUCT, EIP-3541, EIP-3860, EIP-170 enforcement.
- Transaction execution semantics: REVERT non-exceptional; exceptional halts
  consume remaining gas without aborting block processing.
- Receipt encoding/decoding with EIP-2718 typed envelopes.
- SSTORE gas/refund accounting with original storage tracking.
- Transient storage (EIP-1153) implementation and clearing.
- Documentation revalidated against code/README.

### Backlog (Not Scheduled)

- Integrate EELS blockchain fixtures into CI (still optional by default).

## Immediate Next Task

None.
