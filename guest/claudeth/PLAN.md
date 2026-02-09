# Claudeth Implementation Plan (Reality-Based)

Date: 2026-02-09 (Session 85)

## Summary

Claudeth is a minimal-dependency Ethereum STF guest that targets `no_std` on
`riscv32im-unknown-none-elf`. The codebase includes a full EVM interpreter, block
processing with root validations, and a partial MPT. The largest remaining gaps
are witness-based state reconstruction, removing `k256`, and production
validation against real mainnet blocks.

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

### ✅ EELS Test Status (LAST REPORTED PASSING - Session 84)
**Test Results**: All EELS blockchain tests were reported passing in Session 84.
This session did not re-run EELS; treat status as "last known good."

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

**P4.0: Witness Format v0 (DOC/SCHEMA)**

Document the minimal witness schema in PLAN.md and keep it consistent with
the existing `partial_mpt::proof` API. This enables the next step: adding a
parser and proof-verification scaffolding without locking into a heavyweight
serialization format.
