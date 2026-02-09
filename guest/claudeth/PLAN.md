# Claudeth Implementation Plan (Reality-Based)

Date: 2026-02-09

## Summary

Claudeth is a minimal-dependency Ethereum STF guest that targets `no_std` on
`riscv32im-unknown-none-elf`. The codebase already includes a full EVM
interpreter, block processing with root validations, and a partial MPT.
The largest gaps are EELS compliance, witness-based state reconstruction, and
removing `k256`.

This plan reflects **verified code state** from `src/` and known gaps from
`learnings.md` (not re-run in this session).

## Verified Status (From Code Inspection)

### ✅ Implemented
- EVM interpreter with full opcode coverage (incl. PUSH0, TLOAD/TSTORE, BLOBHASH)
- Gas accounting with EIP-2929/3529/3860 logic and refunds
- Transaction types: Legacy / EIP-2930 / EIP-1559
- Block processing: header validation, tx execution loop, receipts, gas used,
  and validation of receipts root, tx root, logs bloom, state root
- Merkle Patricia Trie + proofs (partial MPT)
- State management with storage tries, transient storage, selfdestruct handling
- `no_std` riscv32 guest entry (`src/main.rs`) and bump allocator
- RLP encoding/decoding for core types

### ⚠️ Known Gaps / Risks
- **EELS compliance**: `tests/eels_blockchain_tests.rs` exists, but last known
  status from learnings was 0/20 passing (not re-run in this session).
- **Witness-based state reconstruction**: guest still accepts full state snapshots;
  proof-based input format is not implemented.
- **Dependency elimination**: `k256` is still used for secp256k1; `serde` is still
  required for types.

## Plan

### P1: Deterministic State Root Computation (DONE)
Goal: ensure state root construction is independent of HashMap iteration order by
sorting account addresses before inserting into the state trie.

### P2: Re-baseline EELS Tests
Goal: re-run EELS tests in `--release` and re-categorize failures.

### P3: Fix One EELS Failure Category
Pick the smallest discrepancy after re-baselining and fix it end-to-end.

### P4: Witness-Based State Reconstruction (Design + Implementation)
Define proof input format and implement proof-based state reconstruction.

### P5: Remove `k256`
Implement in-tree secp256k1 and remove external crypto dependency.

## Immediate Next Task

**P2: Re-baseline EELS Tests**

