# Claudeth Implementation Plan (Reality-Based)

## Executive Summary

Claudeth is intended to be a **dependency-free** Ethereum State Transition Function (STF) guest program that compiles to `no_std` for `riscv32` and proves Ethereum mainnet blocks starting from Fusaka.

**Reality check (2026-02-08 - Session 9):**
- ✅ Core types (U256/U512, Address, Hash, Bytes, BlockHeader) - COMPLETE
- ✅ Crypto primitives (dependency-free keccak256, secp256k1 with k256) - COMPLETE
- ✅ RLP encoding/decoding - COMPLETE
- ✅ Partial MPT (node types, trie ops, proofs, account/storage) - COMPLETE
- ✅ EVM stack/memory/gas metering - COMPLETE
- ✅ EVM interpreter with 119 opcodes - COMPLETE
- ✅ Transaction types (Legacy/EIP-2930/EIP-1559) - COMPLETE
- ✅ Transaction validation - COMPLETE
- ✅ Receipt types + bloom filters - COMPLETE
- ⚠️ Many interpreter opcodes are **stubbed** (return 0 or no-op)
- ❌ No transaction execution engine (`stf/executor.rs` missing)
- ❌ No block processing implementation
- ❌ No EELS test integration
- ❌ Not fully dependency-free (uses `k256` for secp256k1, `rand` for tests)
- ❌ No guest `main` entry point (library only)

**Test Status**: 1028 tests passing (all in --release mode) - Updated Session 11

This plan reflects actual code status and defines the next concrete steps.

---

## Current Code Status (Verified 2026-02-08)

### ✅ Phase 0-3 Complete (969 tests)
- **Core types** (374 tests): `U256/U512`, `Address`, `Hash`, `Bytes`, `BlockHeader` + RLP
- **Crypto** (31 tests): dependency-free `keccak256` + secp256k1 (uses `k256`)
- **Partial MPT** (173 tests): node types, trie ops, proofs, account/storage integration
- **EVM core** (111 tests): stack, memory, gas metering
- **EVM opcodes** (158 tests): 119 opcodes across arithmetic/control/environment
- **EVM interpreter** (41 tests): bytecode execution loop with all opcodes wired
- **Transaction types** (42 tests): Legacy/EIP-2930/EIP-1559 + signing hashes
- **Transaction validation** (46 tests): signature/chain_id/nonce/gas/balance checks
- **Receipt types** (35 tests): bloom filters + receipt root calculation

### ⚠️ Stubbed Opcodes (need state/host interface)
**In interpreter.rs** (all return dummy values):
- `0x31 BALANCE` - returns 0
- `0x3B EXTCODESIZE` - returns 0
- `0x3C EXTCODECOPY` - no-op
- `0x3F EXTCODEHASH` - returns 0
- `0x40 BLOCKHASH` - returns 0
- `0x47 SELFBALANCE` - returns 0
- `0x49 BLOBHASH` - returns 0
- `0x4A BLOBBASEFEE` - returns 0
- `0x54 SLOAD` - returns 0
- `0x55 SSTORE` - no-op
- `0x5C TLOAD` - returns 0
- `0x5D TSTORE` - no-op
- `0xF0 CREATE` - returns 0
- `0xF1 CALL` - returns 0
- `0xF2 CALLCODE` - returns 0
- `0xF4 DELEGATECALL` - returns 0
- `0xF5 CREATE2` - returns 0
- `0xFA STATICCALL` - returns 0
- `0xFF SELFDESTRUCT` - just sets stopped=true

### ❌ Missing Components
- **State interface**: no `State` trait for balance/code/storage/transient access
- **Host interface**: no way for interpreter to call CREATE/CALL/etc
- **Transaction executor**: no `stf/executor.rs` to wire validation → execution → receipts
- **Block processor**: no block header validation + tx loop + state root update
- **Guest entry point**: no `main.rs` with guest_main! macro
- **EELS tests**: no test vector integration
- **Dependency-free goal**: still uses `k256` + `rand`

---

## Gaps vs README Requirements

1. **Dependency-free**: must remove `k256` and `rand` and provide internal secp256k1
2. **Guest program**: a proper guest entry point is required (not just library)
3. **Full STF**: transaction execution + block processing are missing
4. **EELS compliance**: no vector tests or runner

---

## Phase 4: Transaction Execution & State Integration (CURRENT)

**Status**: Phase 4 Wave 1 complete (validation + receipts), Wave 2 next (executor + state)

### Wave 1: Validation + Receipts ✅ COMPLETE
- ✅ Transaction validation (46 tests) - `stf/transaction.rs`
- ✅ Receipt types + bloom filters (35 tests) - `stf/receipt.rs`

### Wave 2: Execution Engine (IN PROGRESS)

**Task 1: State Interface** ✅ COMPLETE (Session 10)
- ✅ Define `State` trait (balance, nonce, code, storage, transient, selfdestruct tracking)
- ✅ Implement `InMemoryState` for tests (using existing Account/Storage types)
- ✅ File: src/state/execution.rs (892 lines)
- ✅ Tests: 46 tests (exceeded 25 target by 84%)
- **Depends on**: nothing

**Task 2: Interpreter State Integration** ✅ COMPLETE (Session 11)
- ✅ Replaced stubbed opcodes with real state access:
  - ✅ `BALANCE`, `EXTCODESIZE`, `EXTCODECOPY`, `EXTCODEHASH`, `SELFBALANCE`
  - ✅ `SLOAD`, `SSTORE` (permanent storage)
  - ✅ `TLOAD`, `TSTORE` (transient storage EIP-1153)
  - ✅ `SELFDESTRUCT` (now marks accounts for deletion)
  - ⚠️ `BLOCKHASH` (still stubbed - requires block hash history)
- ✅ Wired State trait into interpreter via generic parameter
- ✅ Updated all tests to use InMemoryState
- ✅ Tests: 13 new tests (total 1028 tests)
- **Depends on**: Task 1 ✅

**Task 3: Host Interface + Call Opcodes** (blocked by Tasks 1, 2)
- Define `Host` trait (create, call, selfdestruct handling)
- Implement call opcodes: `CALL`, `CALLCODE`, `DELEGATECALL`, `STATICCALL`
- Implement create opcodes: `CREATE`, `CREATE2`
- Implement `SELFDESTRUCT`
- **Depends on**: Tasks 1, 2
- **Tests**: 40+ tests

**Task 4: Transaction Executor** (blocked by all above)
- Create `src/stf/executor.rs`
- Pre-execution: validate, charge intrinsic gas, increment nonce
- Execution: run interpreter with state + host
- Post-execution: apply gas refunds, transfer value, generate receipts
- **Depends on**: Tasks 1, 2, 3
- **Tests**: 35+ tests

### Exit Criteria (Phase 4 Complete)
- ✅ Validation + receipts (81 tests) - Session 8
- ✅ State interface + in-memory implementation (46 tests) - Session 10
- ✅ Interpreter with real state access (13 tests) - Session 11
- ⏸️ Host interface + call/create opcodes (40+ tests) - NEXT
- ⏸️ Transaction executor (35+ tests)
- **Total target**: 215+ new tests (140 done, 75 remaining)
- All tests pass in `--release` mode
- Zero clippy warnings

---

## Phase 5: Block Processing (FUTURE)

- Validate block headers (Fusaka rules)
- Execute all transactions in order with cumulative gas
- Compute receipts root + state root
- Tests with small synthetic blocks

---

## Phase 6: EELS Compliance (FUTURE)

- Integrate official EELS test vectors
- Build test runner
- Fix spec mismatches until 100% pass

---

## Phase 7: Dependency-Free Crypto (README REQUIREMENT)

- Keccak-256 is now dependency-free ✅
- Replace `k256` with in-tree secp256k1 (verify + recover)
- Remove `rand` by using deterministic test vectors
- Maintain full test coverage

---

## Parallel Work Available NOW

**Wave 2 can start with 2 parallel streams:**

**Stream A (immediate start)**:
- Task 1: State Interface + In-Memory State
  - No blockers
  - Creates foundation for all other tasks
  - Est. 25+ tests

**Stream B (starts after Task 1)**:
- Task 2: Interpreter State Integration
- Task 3: Host Interface + Call Opcodes
  - Both depend on Task 1
  - Can run in parallel with each other
  - Combined: 70+ tests

**Stream C (starts after all above)**:
- Task 4: Transaction Executor
  - Integrates everything
  - Est. 35+ tests
