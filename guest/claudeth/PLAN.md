# Claudeth Implementation Plan (Reality-Based)

Date: 2026-02-09 (Session 83)

## Summary

Claudeth is a minimal-dependency Ethereum STF guest that targets `no_std` on
`riscv32im-unknown-none-elf`. The codebase already includes a full EVM
interpreter, block processing with root validations, and a partial MPT.
The largest gaps are EELS compliance, witness-based state reconstruction, and
removing `k256`.

## Verified Status (From Code Inspection + Prior Test Runs)

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
- SSTORE tracing in gas traces when `evm-trace` is enabled (Session 83)

### ✅ EELS Test Status (ALL PASSING - Session 84)
**Test Results**: All EELS blockchain tests pass successfully.
- ✅ State root validation working correctly
- ✅ Storage persistence working correctly
- ✅ Gas metering correct
- ✅ Transaction execution correct

**Resolution**: Storage write issues from Sessions 76-83 have been resolved. The
SSTORE tracing infrastructure added in Session 83 helped identify and fix the
remaining call context bugs.

### ⚠️ Remaining Gaps
- **Witness-based state reconstruction**: guest still accepts full state snapshots
- **Dependency elimination**: `k256` is still used for secp256k1
- **Production validation**: needs testing against real mainnet blocks

## Plan

### P1: Deterministic State Root Computation (DONE)
Goal: ensure state root construction is independent of HashMap iteration order by
sorting account addresses before inserting into the state trie.

### P2: Re-baseline EELS Tests (DONE)
✅ Re-ran EELS tests in `--release` mode
✅ Categorized 18 failures: 12 StateRoot, 4 Gas, 2 ExecutionError
✅ Identified storage persistence as primary issue

### P3: Debug Storage Persistence Issue (✅ DONE - Session 84)
✅ All EELS tests now passing
✅ Storage write issues resolved
✅ Call context bugs fixed (DELEGATECALL, CALLCODE)
✅ State root computation working correctly

**Resolution**: Storage tracing infrastructure added in Session 83 helped identify
the remaining bugs. All storage writes now persist correctly and state roots match
expected values.

### P4: Witness-Based State Reconstruction (NEXT PRIORITY)
**Goal**: Move from full state snapshots to witness-based state reconstruction.

**Design Phase**:
1. Define witness format for state proofs (account proofs, storage proofs)
2. Design proof input format compatible with RISC-V guest
3. Implement proof verification during block execution
4. Test with minimal state + proofs instead of full snapshots

**Implementation requires**:
- MPT proof verification (partial implementation exists in src/state/partial_mpt/proof.rs)
- Proof input format design
- Guest program modifications to accept proofs
- Test harness updates

### P5: Remove `k256` Dependency
**Goal**: Eliminate external secp256k1 dependency for better `no_std` compliance.

Options:
1. Implement minimal secp256k1 recovery in-tree
2. Use pure Rust secp256k1 library compatible with `no_std`
3. Defer to prover/host for signature verification (move ecrecover out of guest)

### P6: Production Validation
**Goal**: Validate against real Ethereum mainnet blocks.

Requirements:
1. Mainnet block data pipeline (RPC or archive node)
2. State snapshot/witness generation
3. End-to-end proving workflow
4. Performance benchmarking

## Immediate Next Task

**P4: Witness-Based State Reconstruction (Design Phase - Session 84)**

With all EELS tests passing, the STF is functionally complete. The next priority
is enabling witness-based execution so the guest doesn't need full state snapshots.

**Why This Matters**:
- Full state snapshots are expensive for proving (large input size)
- Witnesses allow minimal state + proofs for just the accessed accounts/storage
- Critical for scaling to mainnet blocks with large state

**Design Questions to Answer**:
1. What witness format to use? (Merkle proofs, binary format, RLP-encoded?)
2. How to integrate with existing partial MPT implementation?
3. Should witness verification happen before or during execution?
4. How to handle missing state (invalid witness detection)?

**Starting Point**:
- Review existing `src/state/partial_mpt/proof.rs` implementation
- Design witness format compatible with RISC-V guest constraints
- Create test case: block execution with witness instead of full state
