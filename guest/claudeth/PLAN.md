# Claudeth Implementation Plan

Date: 2026-02-09

## Summary

Claudeth is a minimal-dependency Ethereum STF guest targeting `no_std` on
`riscv32im-unknown-none-elf`. It includes a full EVM interpreter, block
processing with header validations and root checks, and a partial MPT.
Cancun-era header fields are supported and EIP-4788/EIP-4895/EIP-2935 logic
is implemented.

## Verified Status (from code)

### Implemented
- EVM interpreter with full opcode coverage, including `BLOBHASH`,
  `BLOBBASEFEE`, `TLOAD`, `TSTORE`, `PREVRANDAO`
- Transaction types: Legacy / EIP-2930 / EIP-1559
- Block processing: header validation, tx execution, receipts, gas used,
  validation of receipts root, tx root, logs bloom, state root
- PREVRANDAO (opcode 0x44) returns `mix_hash` post-merge, `difficulty` pre-merge
- Partial MPT with proof support
- EIP-4788 beacon root system call
- EIP-4895 withdrawals
- EIP-2935 historical block hashes system call (Prague)
- Block header fields for Cancun (`blob_gas_used`, `excess_blob_gas`)
- `no_std` riscv32 guest entry and bump allocator

### Known Gaps / Limitations
- Header `requests_hash` (EIP-7685) is parsed but only used for Prague fork detection
- Witness-based state reconstruction not implemented
- `k256` dependency still required for secp256k1
- EELS blockchain fixtures are external and ignored by default
- Guest input does not decode withdrawals

## Testing Status

- `cargo test -p claudeth --release` (2026-02-09): all unit, integration, and
  doc tests passed. EELS runner remains ignored by default.

## Plan

### P0: EELS Compliance — Fix TransactionExecutionError failures
- Re-run EELS ignored fixtures to get current failing cases and error details
- Improve error context from EVM execution failures to unblock diagnosis (done)
- Fix failure patterns one at a time, starting with the simplest (ShanghaiLove
  or transStorageBlockchain)

### P1: Expand EELS Test Coverage
- Remove the `take(10)` limit in test_execute_all_blockchain_tests to run all
  available test files
- Fix newly discovered failures

### P2: Witness-Based State Reconstruction
- Define witness schema and build parser for `no_std` guest

### P3: Remove `k256`
- Replace secp256k1 dependency with in-tree implementation

## Immediate Next Task

Re-run EELS ignored fixtures and capture the specific `EvmError` variants for
the remaining failures.
