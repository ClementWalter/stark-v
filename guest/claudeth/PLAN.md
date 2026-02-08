# Claudeth Implementation Plan (Reality-Based)

## Executive Summary

Claudeth is intended to be a **dependency-free** Ethereum State Transition Function (STF) guest program that compiles to `no_std` for `riscv32` and proves Ethereum mainnet blocks starting from Fusaka.

**Reality check (2026-02-08):**
- Core types, crypto wrappers, MPT, EVM stack/memory/gas, opcode modules, and an interpreter exist.
- The interpreter contains **many stubs** for environment/state opcodes.
- There is **no transaction execution engine** (`stf/executor.rs` missing).
- There is **no block processing** implementation.
- There is **no EELS test integration**.
- The crate is **not dependency-free** (uses `k256`, `rand`).
- There is **no guest `main`** entry point (library only).

This plan reflects actual code status and defines the next concrete steps.

---

## Current Code Status (Verified)

### ✅ Implemented (with tests)
- **Core types**: `U256/U512`, `Address`, `Hash`, `Bytes`, `BlockHeader`.
- **RLP**: encoder/decoder.
- **Crypto**: dependency-free `keccak256` implemented; secp256k1 still uses `k256`.
- **Partial MPT**: node types, trie ops, proofs, account/storage integration.
- **EVM core**: stack, memory, gas metering.
- **Opcode modules**: arithmetic/control/environment modules exist with tests.
- **Interpreter**: bytecode execution loop + opcode dispatch.
- **Transactions**: Legacy/EIP-2930/EIP-1559 types + signing hashes.
- **STF**: transaction validation + receipts.

### ⚠️ Implemented but Stubbed / Incomplete
- **Interpreter environment/state opcodes**: `BALANCE`, `EXTCODE*`, `BLOCKHASH`, `SELFBALANCE`, `BLOB*`, `SLOAD/SSTORE`, `TLOAD/TSTORE`, `CREATE/CREATE2`, `CALL/CALLCODE/DELEGATECALL/STATICCALL`, `SELFDESTRUCT`.
- **Environment opcode module**: contains several stubbed implementations.
- **No state interface**: no `State` trait for balance/code/storage access.
- **No transaction execution**: missing STF executor for state transitions.

### ❌ Missing
- **Dependency-free crypto**: k256/rand violate README.
- **Guest entry point**: no `main.rs`/guest program wiring.
- **Block processing**: header validation, tx execution loop, receipts root, state root.
- **EELS integration**: test vectors and runner.

---

## Gaps vs README Requirements

1. **Dependency-free**: must remove `k256` and `rand` and provide internal secp256k1.
2. **Guest program**: a proper guest entry point is required (not just library).
3. **Full STF**: transaction execution + block processing are missing.
4. **EELS compliance**: no vector tests or runner.

---

## Phase 4: Transaction Execution & State Integration (NEXT)

### Goals
- Replace interpreter stubs with real semantics using a proper execution context and state interface.
- Implement state transitions for transactions (CREATE/CALL/SELFDESTRUCT). 
- Produce correct receipts and state roots.

### Work Items
1. **Execution Context + State Interface** (P0)
   - Define `ExecutionContext` (address, caller, callvalue, calldata, code, return_data, block/tx context).
   - Define `State` trait (balance, code, code_hash, storage load/store, account access, selfdestruct).
   - Provide in-memory test state implementation.

2. **Interpreter Stub Replacement** (P0)
   - `CALLDATA*`, `RETURNDATA*`, `ADDRESS`, `CALLER`, `CALLVALUE` are now implemented with real context.
   - Wire `SLOAD/SSTORE/TLOAD/TSTORE` to state storage.
   - Implement `BALANCE`, `EXTCODESIZE`, `EXTCODECOPY`, `EXTCODEHASH`, `SELFBALANCE`.
   - Implement `CREATE/CREATE2`, `CALL/CALLCODE/DELEGATECALL/STATICCALL`, `SELFDESTRUCT` (host-driven).

3. **STF Executor** (P0)
   - Create `src/stf/executor.rs`.
   - Pre-execution: charge gas, increment nonce.
   - Execute transaction via interpreter with context/state.
   - Post-execution: apply refunds, transfer value, update receipts.

### Exit Criteria
- Interpreter has **no stubs** for environment/state opcodes.
- Transactions execute end-to-end with receipts and state updates.
- 100% test coverage for new/modified modules.
- All tests pass in `--release` mode.

---

## Phase 5: Block Processing

- Validate block headers (Fusaka rules).
- Execute all transactions in order with cumulative gas.
- Compute receipts root + state root.
- Tests with small synthetic blocks.

---

## Phase 6: EELS Compliance

- Integrate official EELS test vectors.
- Build test runner.
- Fix spec mismatches until 100% pass.

---

## Phase 7: Dependency-Free Crypto (README REQUIREMENT)

- Keccak-256 is now dependency-free.
- Replace `k256` with in-tree secp256k1 (verify + recover).
- Remove `rand` by using deterministic test vectors.
- Maintain full test coverage.

---

## Parallel Work Available NOW (Concrete)

1. **State Interface + In-Memory State**
   - Define a `State` trait with balance/code/storage access.
   - Implement a simple in-memory state for tests.

2. **Replace Remaining Interpreter Stubs**
   - Wire `BALANCE`, `EXTCODE*`, `SLOAD/SSTORE`, `TLOAD/TSTORE`, `SELFBALANCE` to state.

(These two tasks are independent if the state interface is defined first in a small shared module.)
