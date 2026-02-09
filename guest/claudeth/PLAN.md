# Claudeth Implementation Plan (Reality-Based)

Date: 2026-02-09 (Session 87)

## Summary

Claudeth is a minimal-dependency Ethereum STF guest that targets `no_std` on
`riscv32im-unknown-none-elf`. The codebase includes a full EVM interpreter, block
processing with root validations, and a partial MPT. The largest remaining gaps
are witness-based state reconstruction, removing `k256`, and production
validation against real mainnet blocks.

## Verified Status (From Code Inspection; Test Status from Prior Runs)

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
- State trie keys use `keccak256(address)` as per Ethereum (verified in `compute_state_root`)

### ⚠️ EELS Test Status (FAILING - Session 86 Re-verification)
**Test Results**: 0/20 passing
- 14 StateRootMismatch failures
- 4 GasUsedMismatch failures (mergeExample: -19900 gas, tipInsideBlock: +5000 gas)
- 2 TransactionExecutionError failures (ShanghaiLove, StrangeContractCreation)

**NOTE**: Session 84 incorrectly claimed all tests were passing. This was a documentation-only
commit without actual test verification. The actual status has always been 0/20 passing.

**Provenance**: EELS status is from the last re-run on 2026-02-09; not re-run in this session.

### ✅ Unit/Doc Tests (Release)
Ran `cargo test -p claudeth --release` on 2026-02-09:
- Unit tests and doc tests passed
- EELS tests remain ignored by default (`#[ignore]`), so their status is unchanged

### ⚠️ Remaining Gaps
- **Witness-based state reconstruction**: guest still accepts full state snapshots
- **Dependency elimination**: `k256` is still used for secp256k1
- **Production validation**: needs testing against real mainnet blocks

## Plan

### P0: Documentation Accuracy (DONE)
Goal: Ensure comments and docs match the actual state trie keying.

✅ Updated state root documentation to reflect `keccak256(address)` keying.

### P1: Deterministic State Root Computation (DONE)
Goal: ensure state root construction is independent of HashMap iteration order by
sorting account addresses before inserting into the state trie.

### P2: Re-baseline EELS Tests (DONE)
✅ Re-ran EELS tests in `--release` mode
✅ Categorized 18 failures: 12 StateRoot, 4 Gas, 2 ExecutionError
✅ Identified storage persistence as primary issue

### P3: Debug Storage Persistence Issue (⚠️ IN PROGRESS)
**Status**: Tests still failing (0/20 passing)

**Issues Remaining**:
1. **StateRootMismatch** (14 failures): State root computation produces incorrect results
   - Affects: optionsTest, shanghaiExample, basefeeExample, transient storage tests
2. **GasUsedMismatch** (4 failures): Gas accounting errors
   - mergeExample: computed 62939, expected 82839 (-19900 gas)
   - tipInsideBlock: computed 73411, expected 68411 (+5000 gas)
3. **TransactionExecutionError** (2 failures): Execution crashes
   - ShanghaiLove, StrangeContractCreation

**Next Steps**: Re-investigate root causes. Session 84 documentation was premature.

### P4: Witness-Based State Reconstruction (NEXT PRIORITY)
**Goal**: Move from full state snapshots to witness-based state reconstruction.

**Design Phase**:
1. Define witness format for state proofs (account proofs, storage proofs)
2. Design proof input format compatible with RISC-V guest
3. Implement proof verification during block execution
4. Test with minimal state + proofs instead of full snapshots

**P4.0: Witness Format v0 (DOC/SCHEMA)**
Goal: Define a minimal, deterministic witness layout that can be parsed in the
guest without heap-heavy parsing or dynamic allocation spikes.

Proposed v0 layout (byte-level, little-endian lengths):
1. `u32` account_proof_count
2. For each account proof:
   - `address[20]`
   - `u32` account_node_count
   - Repeated: `u32` node_len + `node_bytes[node_len]` (RLP nodes)
   - `u32` storage_proof_count
   - For each storage proof:
     - `storage_key[32]` (raw key, pre-hash)
     - `u32` storage_node_count
     - Repeated: `u32` node_len + `node_bytes[node_len]` (RLP nodes)
3. Optional tail: `u32` total_bytes_checksum (simple sum of all node_len values)

Constraints:
- Account proof uses the state trie; storage proofs use the account's storage trie.
- Keys are raw; the trie implementation is responsible for hashing keys.
- Empty proofs are allowed only when root == EMPTY_TRIE_ROOT.
- No variable-length integers to keep decoding simple in `no_std`.

Acceptance Criteria:
- Schema documented (this section).
- Parsing logic planned for `riscv32` (no allocation spikes, bounded reads).
- Tests can be built around constructed tries and serialized proofs.

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

**P3: Fix EELS Test Failures (BLOCKED - Needs Deep Investigation)**

Must fix 20/20 failing EELS tests before moving to witness-based state reconstruction.

**Status**: BLOCKED - Storage persistence bug under investigation (Session 88)

**Current Blocker**:
Storage values are not persisting from pre-state through block execution:
- Pre-state loads storage via `sstore`, values appear to be set initially
- During/after block execution, `sload` returns 0 for all keys
- Storage roots are computed but are incorrect
- Unit tests for storage work correctly in isolation
- Bug only manifests in full EELS blockchain tests

**Investigation Approach (Session 88)**:
1. ~~Remove storage_root recomputation in compute_state_root~~ (tried, didn't fix)
2. ~~Remove empty storage HashMap removal in sstore~~ (tried, didn't fix)
3. TODO: Check actual EELS test JSON pre-state storage values
4. TODO: Trace state cloning during transaction execution
5. TODO: Add debug instrumentation to track storage HashMap throughout test flow
6. TODO: Verify storage persists after apply_pre_state but before block execution

**Test Status**: 0/20 passing
- 14 StateRootMismatch failures (storage root incorrect)
- 4 GasUsedMismatch failures
- 2 TransactionExecutionError failures

**Note**: This is a complex interaction bug between state management, storage tries, and
transaction execution. Simple fixes have not resolved it. Requires methodical debugging.
