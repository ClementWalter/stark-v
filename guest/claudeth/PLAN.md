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
- EVM execution failures (OOG, InvalidJump, etc.) handled as failed
  transactions (not block-level errors) per Ethereum spec
- Memory expansion gas: uncapped quadratic formula, overflow-safe

### Known Gaps / Limitations

- Header `requests_hash` (EIP-7685) is parsed but only used for Prague fork
  detection
- Witness-based state reconstruction not implemented
- `k256` dependency still required for secp256k1
- EELS blockchain fixtures are external and ignored by default
- Guest input does not decode withdrawals
- EIP-4844 blob transactions (type 0x03) not supported

## Testing Status

- `cargo test -p claudeth --release` (2026-02-09): all 93 unit, integration,
  and doc tests pass.
- EELS blockchain tests: **236/882 passing** (all 216 test files, no take limit)
  - 638 GasUsedMismatch (gas calculation issues)
  - 4 GasLimitExceeded
  - 2 StateRootMismatch
  - 2 unsupported tx type 0x03

## Plan

### P0: Fix Gas Calculation — GasUsedMismatch (638 failures)

The majority of remaining EELS failures are gas mismatches. Two patterns:

1. **computed == gas_limit** (most common): EVM execution is failing
   (returning OOG/error) on transactions that should succeed. This
   consumes all gas via the exceptional-halt handler.
   - Root cause likely in: opcode gas costs, CALL/CREATE gas forwarding
     (EIP-150 63/64 rule), access list warm/cold accounting, or
     SSTORE gas (EIP-2200/3529).

2. **computed < expected**: EVM execution succeeds but uses less gas
   than expected.
   - Root cause likely in: missing gas charges for specific opcodes,
     incorrect warm/cold access tracking, or missing EIP-2935 gas.

**Next steps:**
- Pick one specific failing test with simple bytecode (e.g., from
  bcBlockGasLimitTest or bcExploitTest)
- Trace per-opcode gas with `--features evm-trace`
- Compare against reference EELS execution
- Fix the specific gas miscalculation

### P1: Fix GasLimitExceeded (4 failures)

Block gas limit exceeded check may be too strict or cumulative gas
tracking may have a bug.

### P2: Fix StateRootMismatch (2 failures)

State trie root computation differs from expected after execution.

### P3: Add EIP-4844 blob transaction support

Type 0x03 transactions need parsing and execution support.

### P4: Witness-Based State Reconstruction

Define witness schema and build parser for `no_std` guest.

### P5: Remove `k256`

Replace secp256k1 dependency with in-tree implementation.

## Immediate Next Task

Pick one GasUsedMismatch test with simple bytecode, enable evm-trace,
and debug the specific gas calculation error.
