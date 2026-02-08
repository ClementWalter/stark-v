# Claudeth Implementation Plan (Reality-Based)

Date: 2026-02-08

## Executive Summary

Claudeth is intended to be a **dependency-free** Ethereum State Transition Function (STF) guest program that compiles `no_std` for `riscv32`, embeds a partial MPT, and can process Ethereum mainnet blocks starting at Fusaka. The repository already contains substantial core functionality, but several README-critical gaps remain.

This plan reflects **verified code presence** (from `src/`) and enumerates the **missing requirements** that must be implemented to match the README.

---

## Current Code Status (Verified by File Inspection)

### ✅ Present in Code
- Core types: `U256/U512`, `Address`, `Hash`, `Bytes`, `BlockHeader`
- Crypto primitives: `keccak256` (in-tree), secp256k1 wrappers (via `k256`)
- RLP encoding/decoding
- Partial MPT: node types, trie ops, proofs, account/storage
- EVM core: stack, memory, gas metering
- EVM opcodes: arithmetic/control/environment
- EVM interpreter with bytecode execution
- Transaction types: Legacy/EIP-2930/EIP-1559
- Transaction validation + receipts
- Execution state trait + `InMemoryState`
- STF transaction executor (pre/post execution pipeline)
- Block header validation helpers (gas used <= limit, extra data size, post-merge fields)

### ⚠️ Known Gaps vs README Requirements
1. **Dependency-free**: `k256` and `rand` are still used (`Cargo.toml`).
2. **Guest program entry point**: no `main.rs` or guest entry for `riscv32`.
3. **Block processing**: no block-level execution loop (header validation vs parent, cumulative gas, receipts root, state root).
4. **EELS compliance**: no EELS test vector integration or runner.

### ⚠️ STF Execution Limitations (Observed in Code)
- **Log capture missing**: LOG0–LOG4 consume gas but do not record logs; executor returns empty logs.
- **Gas refunds not tracked**: SSTORE/SELFDESTRUCT refund accounting is TODO in `executor.rs`.
- **Host wiring**: executor uses `NullHost` (TODO in `executor.rs`).

---

## Implementation Plan

### Phase A: STF Execution Correctness Polishing (Immediate)
Goal: finalize per-transaction correctness before block processing.

1. **Per-tx cleanup** (DONE)
   - `clear_transient_storage` + `clear_selfdestructs` in `State`.
   - Called in `execute_transaction` on completion or failure.

2. **Execution API refactor** (DONE)
   - `execute_bytecode_with_host` returns `(ExecutionResult, S)`.
   - CREATE deployment now writes code to state.

3. **Log capture + gas refunds** (NEXT)
   - Record LOG0–LOG4 data during execution.
   - Plumb logs into receipts.
   - Track gas refunds from SSTORE/SELFDESTRUCT (EIP-3529 cap).

### Phase B: Block Processing
- Validate block header vs parent (timestamp, gas limit bounds, gas used, extra data, PoS fields).
- Execute all transactions in order.
- Track cumulative gas and build receipts.
- Compute receipts root and state root via MPT.

### Phase C: Guest Entry Point
- Add `src/main.rs` with guest entry for `riscv32`.
- Define I/O format (block + witness inputs, result outputs).

### Phase D: EELS Compliance
- Integrate official EELS test vectors.
- Build test runner and fix spec mismatches to 100% pass.

### Phase E: Dependency-Free Crypto
- Replace `k256` with in-tree secp256k1 (verify + recover).
- Remove `rand` usage from tests via deterministic vectors.

---

## Current Status Summary (2026-02-08)

### ✅ Completed (Phase A Partial)
- **Per-transaction cleanup** ✅
- **Execution API refactor** ✅
- **CREATE deployment** ✅

### Phase A Status: Partially Complete
- Remaining: log capture, gas refund tracking, host plumbing

---

## Immediate Next Task (Execute Now)

**Phase A: Log Capture** - Complete the execution pipeline output surface

Task A3: LOG0–LOG4 capture and receipt wiring
- Record log address/topics/data during execution.
- Use correct LOG gas cost (base + topics + data).
- Propagate logs into `TransactionExecutionResult` / receipts.
- Add focused tests for log capture and topic ordering.
