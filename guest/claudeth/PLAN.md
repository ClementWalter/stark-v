# Claudeth Implementation Plan

Date: 2026-02-09

## Summary

Claudeth is a minimal-dependency Ethereum STF guest that targets `no_std` on
`riscv32im-unknown-none-elf`. The codebase includes a full EVM interpreter,
block processing with root validations, and a partial MPT. Cancun-era block
header fields are supported and EIP-4788/EIP-4895 logic is implemented, but
Prague-specific system calls (EIP-2935) are not yet implemented.

## Verified Status (from code)

### Implemented
- EVM interpreter with full opcode coverage, including `BLOBHASH`,
  `BLOBBASEFEE`, `TLOAD`, `TSTORE`
- Transaction types: Legacy / EIP-2930 / EIP-1559
- Block processing: header validation, tx execution, receipts, gas used,
  validation of receipts root, tx root, logs bloom, state root
- Partial MPT with proof support
- EIP-4788 beacon root system call
- EIP-4895 withdrawals
- Block header fields for Cancun (`blob_gas_used`, `excess_blob_gas`)
- `no_std` riscv32 guest entry and bump allocator

### Known Gaps / Limitations
- Prague EIP-2935 Historical Block Hashes system call not implemented
- Header `requests_hash` (EIP-7685) is parsed but unused
- Witness-based state reconstruction not implemented
- `k256` dependency still required for secp256k1
- EELS blockchain fixtures are external and tests are ignored by default

## Testing Status

- `cargo test -p claudeth --release` (2026-02-09): 1172 unit tests, 93 doc tests
  passed; EELS test runner remains ignored by default.
- EELS blockchain tests: fixtures required in `tests/eels/BlockchainTests/`,
  run with `cargo test -p claudeth --release -- --ignored`.

## Plan

### P0: Documentation & Status Hygiene
- Keep README/PLAN/learnings aligned with current code and test provenance

### P1: Prague Support (EIP-2935)
- Implement Historical Block Hashes system call and wire to block processing
- Validate against Prague EELS fixtures once available

### P2: EELS Compliance Debugging
- Run ignored EELS fixtures and fix remaining failures
- Use `evm-trace` for targeted gas/execution diagnostics

### P3: Witness-Based State Reconstruction
- Define witness schema and build parser for `no_std` guest

### P4: Remove `k256`
- Replace secp256k1 dependency with in-tree implementation

## Immediate Next Task

Documentation alignment (README/learnings) and test provenance updates.
