# Claudeth Completion Plan

Last reviewed: 2026-02-10

## Ground Truth Snapshot

- `README.md` claims full execution-spec compatibility and says tests run on both native and RV32 targets.
- The header-linkage blocker has been removed:
  - Harness parent-hash rewrite is gone from `tests/eels_blockchain_tests.rs`.
  - `BlockHeader` now models Prague `requests_hash` and includes it in RLP/hash ordering.
  - Header RLP nonce encoding now matches execution-specs (`Bytes8`), fixing fixture hash parity.
  - Cancun + Prague fixture-backed header hash tests are in place.
- Remaining deterministic conformance blockers:
  - `tests/eels_blockchain_tests.rs` full execution test is still `#[ignore]`.
  - Fixture conversion still rejects tx type `0x03` blob transactions.
  - Harness still passes empty block-hash history (`&[]`) into `process_block`.
  - Precompile `0x0a` point evaluation remains unimplemented.
  - ALT_BN128 pairing (`0x08`) still lacks full non-trivial pairing checks.
- README "no dependencies" claim remains inaccurate (`serde` in `Cargo.toml`).

## Completed This Turn

### Task 1 (DONE): Fix Prague Header Hash Parity and Remove Parent-Hash Rewrite

Why:
- Parent linkage is consensus-critical; rewriting fixture `parentHash` made conformance results non-authoritative.
- Prague headers include `requestsHash`; omitting it caused deterministic hash divergence.

What:
- Added `requests_hash` support in block-header model/encoding/decoding/hashing.
- Fixed nonce RLP encoding to `Bytes8` parity.
- Removed harness parent-hash mutation.
- Added fixture-backed Cancun/Prague hash regression tests.

How:
- Updated `src/types/block.rs`:
  - new `requests_hash: Option<Hash>` field;
  - RLP encode/decode support after `parent_beacon_block_root`;
  - nonce encoded/decoded as 8-byte string per header schema.
- Updated `tests/eels_blockchain_tests.rs`:
  - parse `requestsHash` from fixtures;
  - removed parent-hash workaround;
  - added `test_fixture_header_hashes_match_for_cancun_and_prague_examples` and
    `test_fixture_parent_hash_linkage_uses_real_header_hashes`.

## Priority Backlog (Why / What / How)

### Task 2 (P0, FIRST): Add Blob Transaction Fixture Conversion (`0x03`)

Why:
- Cancun/Prague fixtures contain blob transactions; rejecting them blocks conformance coverage.

What:
- Convert fixture tx type `0x03` into `Transaction::Blob`.

How:
- Extend `convert_test_transaction` with strict parsing/validation of blob fields.
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
