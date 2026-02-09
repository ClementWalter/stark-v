# Claudeth Implementation Plan

Date: 2026-02-09

## Summary

Claudeth is a minimal-dependency Ethereum STF guest that targets `no_std` on
`riscv32im-unknown-none-elf`. The codebase includes a full EVM interpreter,
block processing with root validations, and a partial MPT. Cancun-era block
header fields are supported and EIP-4788/EIP-4895/EIP-2935 logic is
implemented.

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
- Header `requests_hash` (EIP-7685) is parsed but unused beyond Prague fork detection
- Witness-based state reconstruction not implemented
- `k256` dependency still required for secp256k1
- EELS blockchain fixtures are external and tests are ignored by default
- 6 EELS tests still failing (3 distinct issues, see below)

## Testing Status

- `cargo test -p claudeth --release` (2026-02-09): 1172 unit tests, 93 doc tests
  passed; EELS test runner remains ignored by default.
- EELS blockchain tests: 14/20 passing (fixtures in `tests/eels/BlockchainTests/`,
  run with `cargo test -p claudeth --release -- --ignored`).
- Failing EELS tests (all `TransactionExecutionError(ExecutionFailed)`):
  - `transStorageBlockchain` (Block 2): multi-block transient storage test with
    CREATE calls — likely an issue with nested CALL/CREATE execution
  - `ShanghaiLove` (Block 0): empty-data transaction to a contract — likely
    CALL execution issue
  - `StrangeContractCreation` (Block 0): large CREATE transaction with
    constructor bytecode — likely CREATE/code-deploy issue

## Plan

### P0: EELS Compliance — Fix TransactionExecutionError failures
- Investigate the 3 remaining failure patterns (transStorageBlockchain,
  ShanghaiLove, StrangeContractCreation)
- Root cause is likely in CALL/CREATE execution paths in executor.rs
- The `ExecutionFailed` error is too generic — add context to help diagnose
- Fix one at a time, starting with the simplest (ShanghaiLove or
  transStorageBlockchain)

### P1: Expand EELS Test Coverage
- Remove the `take(10)` limit in test_execute_all_blockchain_tests to run all
  available test files
- Fix newly discovered failures

### P2: Witness-Based State Reconstruction
- Define witness schema and build parser for `no_std` guest

### P3: Remove `k256`
- Replace secp256k1 dependency with in-tree implementation

## Immediate Next Task

Investigate and fix `TransactionExecutionError(ExecutionFailed)` in remaining
EELS tests. Start with `ShanghaiLove` or `transStorageBlockchain` as they
appear to involve CALL/CREATE execution bugs.
