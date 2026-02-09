# Claudeth Implementation Plan

Date: 2026-02-09 (Session 92)

## Summary

Claudeth is a minimal-dependency Ethereum STF guest that targets `no_std` on
`riscv32im-unknown-none-elf`. The codebase includes a full EVM interpreter, block
processing with root validations, and a partial MPT. The largest remaining gaps
are Prague support (EIP-2935 etc.), gas accounting fixes, EVM execution errors,
witness-based state reconstruction, and removing `k256`.

## Verified Status

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
- EIP-4788 Beacon Block Root system call (Cancun)
- EIP-4895 Withdrawal Processing (Shanghai)
- EIP-3651 Warm COINBASE
- Deterministic state root computation with sorted addresses

### ✅ EELS Test Status (Session 92)
**Test Results**: 6/20 passing (was 0/20)

**Passing (6 - all Cancun variants):**
- optionsTest_Cancun ✓
- shanghaiExample_Cancun ✓
- basefeeExample_Cancun ✓
- tipInsideBlock_Cancun ✓
- tloadDoesNotPersistAcrossBlocks_Cancun ✓
- tloadDoesNotPersistCrossTxn_Cancun ✓

**Failing (14):**
- 8 Prague StateRootMismatch (missing EIP-2935 + Prague system calls)
- 2 GasUsedMismatch (mergeExample_Cancun + mergeExample_Prague: 62939 vs 82839)
- 4 TransactionExecutionError (ShanghaiLove×2, StrangeContractCreation×2, transStorageBlockchain_Cancun)

### ✅ Unit/Doc Tests (Release)
- Unit tests and doc tests: 1172+ passing
- State root regression test: passing (validates MPT/RLP against known Ethereum state root)

## Plan

### P0-P2: DONE (see git history)

### P3: Fix EELS Test Failures (✅ PARTIALLY RESOLVED)

**Root Cause Found (Session 92)**: `EMPTY_TRIE_ROOT` constant had a typo at byte 17
(`0x96` instead of `0x48`). This caused every account with empty storage to produce
wrong RLP, poisoning ALL state root computations.

**Fix**: Single byte change in `src/state/partial_mpt/trie.rs:24`.

**Result**: 0/20 → 6/20 passing. All Cancun tests that don't have other issues now pass.

**Remaining Failures by Category:**

#### P3.1: Prague Support (8 failures)
All Prague variants fail with StateRootMismatch because we're missing Prague-specific
system calls:
- **EIP-2935**: Historical Block Hashes contract (`0x0000f908...2935`)
  - System call stores parent block hash in ring buffer at block start
  - Contract exists in Prague pre-state, needs system call implementation
- **EIP-7002**: Execution Layer Exits (may not be needed for test set)
- **EIP-7251**: Consolidation Requests (may not be needed for test set)

#### P3.2: Gas Accounting (2 failures)
mergeExample: computed 62939, expected 82839 (-19900 gas deficit)
- Same failure on both Cancun and Prague
- Need to investigate contract creation gas accounting

#### P3.3: Execution Errors (4 failures)
- ShanghaiLove_Cancun/Prague: TransactionExecutionError
- StrangeContractCreation_Cancun/Prague: TransactionExecutionError
- transStorageBlockchain_Cancun: TransactionExecutionError on block 2

### P3.1: Implement EIP-2935 (NEXT PRIORITY)
**Goal**: Implement Historical Block Hashes system call for Prague support.
This should fix all 8 Prague StateRootMismatch failures.

**Implementation**:
1. Add EIP-2935 contract address constant
2. Implement system call that stores `parent_hash` at `block.number - 1 % HISTORY_SERVE_WINDOW`
3. Call it in `process_block` when requests_hash is present (Prague indicator)

### P4: Witness-Based State Reconstruction (DEFERRED)
### P5: Remove `k256` Dependency (DEFERRED)
### P6: Production Validation (DEFERRED)

## Immediate Next Task

**P3.1: Implement EIP-2935 Historical Block Hashes** for Prague support.
