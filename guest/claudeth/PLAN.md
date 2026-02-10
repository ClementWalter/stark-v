# Claudeth Completion Plan

Last reviewed: 2026-02-10

## Ground Truth Snapshot

- `README.md` claims full execution-spec compatibility and says tests run on both native and RV32 targets.
- The blockchain conformance harness still has non-authoritative behavior:
  - `tests/eels_blockchain_tests.rs` keeps the full execution test `#[ignore]`.
  - The harness rewrites each fixture `parentHash` (`block_header.parent_hash = parent_header.compute_hash()`), masking real header-linkage failures.
  - The harness passes empty block-hash history (`&[]`) to `process_block`, so `BLOCKHASH` behavior cannot match fixtures that depend on history.
- Root cause identified for parent-hash rewrite: Prague fixtures include `requestsHash` (EIP-7685), but `BlockHeader` currently omits this field from model/RLP/hash. That guarantees hash divergence for Prague+ headers.
- Fixture conversion in `convert_test_transaction` still rejects type `0x03` blob transactions.
- Precompile `0x0a` point evaluation is still unimplemented.
- ALT_BN128 pairing (`0x08`) still rejects non-trivial valid tuples.
- README "no dependencies" claim is not accurate (`serde` dependency is present in `Cargo.toml`).

## Completed Before This Turn

- Empty trie root corrected to canonical value and protected by regression tests.
- Withdrawals root computation anchored to a real Shanghai fixture vector.

## Priority Backlog (Why / What / How)

### Task 1 (P0, FIRST): Fix Prague Header Hash Parity and Remove Parent-Hash Rewrite

Why:
- Parent linkage is consensus-critical. Rewriting fixture `parentHash` hides header validation defects.
- Prague headers add `requestsHash`; omitting it in header hashing causes deterministic parent-hash mismatches.

What:
- Add `requests_hash` support to `BlockHeader` and fixture conversion.
- Remove harness parent-hash mutation and validate real fixture linkage.

How:
- Extend `src/types/block.rs` with optional `requests_hash` field and include it in RLP encode/decode/hash order after `parent_beacon_block_root` (per `execution-specs/src/ethereum/forks/prague/blocks.py`).
- Parse `requestsHash` in `tests/eels_blockchain_tests.rs` conversion.
- Remove workaround assignment in harness.
- Add fixture-backed tests for Cancun and Prague genesis/block header hash parity to lock behavior.

### Task 2 (P0): Add Blob Transaction Fixture Conversion (`0x03`)

Why:
- Cancun/Prague fixtures contain blob transactions; rejecting them blocks conformance coverage.

What:
- Convert fixture tx type `0x03` into `Transaction::Blob`.

How:
- Extend `convert_test_transaction` with strict parsing of blob fields.
- Add conversion tests for valid and malformed blob fixtures.

### Task 3 (P0): Provide Real Block Hash History to `process_block`

Why:
- Passing `&[]` breaks `BLOCKHASH` semantics and causes deterministic divergences.

What:
- Feed canonical recent block hashes while executing fixture chains.

How:
- Track executed canonical headers in harness and pass bounded history (oldest -> newest).
- Add targeted regression tests for BLOCKHASH-sensitive fixtures.

### Task 4 (P0): Implement Precompile `0x0a` Point Evaluation

Why:
- EIP-4844 conformance requires point-evaluation behavior.

What:
- Implement calldata validation, gas charging, proof verification path, and return formatting.

How:
- Mirror execution-spec behavior and add vectors for success, invalid proof, malformed input, and OOG.

### Task 5 (P0): Complete Full ALT_BN128 Pairing (`0x08`)

Why:
- Current implementation handles only validation/identity fast paths.

What:
- Implement full pairing product equation for non-trivial tuples.

How:
- Add Miller loop + final exponentiation with strict subgroup checks.
- Add vectors for valid and invalid multi-pair inputs.

### Task 6 (P0): Make Blockchain EELS Execution a Hard Gate

Why:
- Full compatibility cannot be claimed while the suite is ignored and non-fatal.

What:
- Turn `test_execute_all_blockchain_tests` into a strict pass/fail gate.

How:
- Remove `#[ignore]` after P0 blockers are addressed.
- Assert `failed == 0` and `errors == 0`.

### Task 7 (P1): Add Automated Native vs RV32 Parity Gate

Why:
- README claims both targets are tested, but no enforced parity workflow exists.

What:
- Add deterministic native/RV32 parity checks for curated fixture subsets.

How:
- Add `uv run` Python driver (PEP 723 metadata) that runs both paths and diffs results.
- Integrate into local quality checks.

### Task 8 (P1): Align README With Verified Reality

Why:
- Public claims must match what is actually implemented and enforced.

What:
- Update README text or implementation to eliminate inaccurate claims.

How:
- Reconcile dependency claim (`serde`).
- Document exact conformance scope and target-testing guarantees.
