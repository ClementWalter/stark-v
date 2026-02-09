# Claudeth Implementation Plan (Reality-Based)

Date: 2026-02-09 (Session 80)

## Summary

Claudeth is a minimal-dependency Ethereum STF guest that targets `no_std` on
`riscv32im-unknown-none-elf`. The codebase already includes a full EVM
interpreter, block processing with root validations, and a partial MPT.
The largest gaps are EELS compliance, witness-based state reconstruction, and
removing `k256`.

## Verified Status (From Code Inspection + Test Runs)

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
- DELEGATECALL value transfer fix (Session 76)
- Touch tracking toggle for pre-state loading (Session 74)
- Deterministic state root computation with sorted addresses (Session 73)
- Delegatecall storage context regression test (Session 78)
- EELS state trie leaf dump on state root/post-state mismatch (Session 80)

### ⚠️ EELS Test Status (20/20 failures - Session 80)
**Test Results (10 test files, 20 total test cases)**:
- StateRootMismatch: 14 failures
- GasUsedMismatch: 4 failures (2 under, 2 over)
- TransactionExecutionError: 2 failures

**Pattern Analysis**:
1. **Storage persistence issue**: Storage slots showing 0 when non-zero values expected
2. **Storage root mismatch**: Computed storage roots differ from expected (not EMPTY)
3. **Multi-block tests**: nonce expected=4, got=1 (only 1 of 4 blocks executed)
4. **Gas metering**: Expected 82839, got 62939 (19900 gas discrepancy)

**Key Insight**: Storage roots are being computed (not empty), but values differ.
This suggests storage writes ARE happening, but something about the encoding or
key hashing is incorrect.

### ⚠️ Other Known Gaps
- **Witness-based state reconstruction**: guest still accepts full state snapshots
- **Dependency elimination**: `k256` is still used for secp256k1

## Plan

### P1: Deterministic State Root Computation (DONE)
Goal: ensure state root construction is independent of HashMap iteration order by
sorting account addresses before inserting into the state trie.

### P2: Re-baseline EELS Tests (DONE)
✅ Re-ran EELS tests in `--release` mode
✅ Categorized 18 failures: 12 StateRoot, 4 Gas, 2 ExecutionError
✅ Identified storage persistence as primary issue

### P3: Debug Storage Persistence Issue (CURRENT)
**Observation**: Storage slots read as 0 when they should be non-zero, but storage
roots are non-empty (not EMPTY_TRIE_ROOT). This suggests:
- Storage writes ARE being persisted to tries
- But retrieval or encoding is incorrect

**Investigation needed**:
1. Verify delegatecall/CALLCODE storage context with real fixture data (optionsTest)
2. Confirm pre-state storage is loaded correctly for all addresses in fixture
3. Trace storage writes per address during block 0 and compare to expected post-state
4. Verify RLP encoding/decoding of storage values (only if values differ at same keys)

**New tooling added**:
- Dump state trie leaves (address, hashed key, account RLP, storage root) on
  StateRootMismatch and post-state mismatches in EELS runner to pinpoint the
  exact account/encoding divergence.

**Hypothesis**: Storage writes are applied to the wrong address/context (delegatecall/callcode),
or the expected fixture post-state is being compared against a different execution path.

### P4: Fix Gas Metering Discrepancies
After fixing storage, address the 4 GasUsedMismatch failures (19900 gas delta).

### P5: Fix Transaction Execution Failures
Address 2 TransactionExecutionError failures (ShanghaiLove test).

### P6: Witness-Based State Reconstruction (Design + Implementation)
Define proof input format and implement proof-based state reconstruction.

### P7: Remove `k256`
Implement in-tree secp256k1 and remove external crypto dependency.

## Immediate Next Task

**P3: Debug Storage Write Execution (CURRENT - Session 82)**

Analysis of tloadDoesNotPersistAcrossBlocks_Prague test reveals:
- Address `0x0000f90827f1c53a10cb7a02335b175320002935` should have storage after block 0
- Expected: storage[0x00] = `0x0332...` , storage_root = `0xb99438...`
- Actual: storage[0x00] = `0x0000...`, storage_root = `0x56e81...` (EMPTY_TRIE_ROOT)
- Transaction calls `0xa00000000000000000000000000000000000000a` with data `0x0accf739`
- Pre-state has `0x000f3df6d732807ef1319fb7b8bb8522d0beac02` with storage[0x03b6] = 0x03b6

**Key Questions**:
1. Are storage writes happening at all during execution?
2. Are writes going to the correct address (CALL vs DELEGATECALL context)?
3. Is transaction execution even reaching SSTORE opcodes?

**Next Steps**:
1. Add execution tracing to see SSTORE opcodes and target addresses
2. Trace the call chain: tx → 0xa000...000a → ??? → SSTORE to 0x0000f9...
3. Verify CALL/DELEGATECALL context is correct (storage writes to caller vs callee)
