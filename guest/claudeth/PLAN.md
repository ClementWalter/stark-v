# Claudeth Completion Plan

Last reviewed: 2026-02-10

## Ground Truth Snapshot

- `README.md` still claims full execution-spec compatibility and that tests run on both native and RV32 targets.
- Deterministic conformance blockers still present in code:
  - `tests/eels_blockchain_tests.rs` keeps the full blockchain suite `#[ignore]` and does not hard-fail on failures/errors.
  - The harness still rewrites `block_header.parent_hash = parent_header.compute_hash()` as a workaround.
  - The harness converts only tx types `0x00/0x01/0x02`; blob tx (`0x03`) fixtures are rejected.
  - `process_block` is called with empty block-hash history (`&[]`), so `BLOCKHASH` fixture behavior cannot match.
  - Precompile `0x0a` (point evaluation) is still unimplemented in `src/evm/precompiles.rs`.
- `Cargo.toml` still includes `serde`, so README "no dependencies" is currently inaccurate.

## Completed This Turn

### Task 1 (DONE): Fix Canonical Empty Trie Root and Lock Spec Vectors

Why:
- Empty trie root is a consensus constant used by transactions/receipts/withdrawals roots.
- A wrong constant creates deterministic root mismatches even when the rest of the transition logic is correct.

What:
- Corrected `EMPTY_TRIE_ROOT` to canonical Ethereum value.
- Added regression vectors for the canonical empty root and a non-empty Shanghai withdrawals root.

How:
- Updated `src/state/partial_mpt/trie.rs` constant from `...5b96...` to `...5b48...`.
- Added `test_empty_trie_root_matches_execution_specs_constant` in trie tests.
- Added `test_calculate_withdrawals_root_matches_shanghai_fixture_vector` in block tests using
  `tests/eels/BlockchainTests/ValidBlocks/bcExample/shanghaiExample.json`
  (`0x27f166f1d7c789251299535cb176ba34116e44894476a7886fe5d73d9be5c973`).

## Priority Backlog (Why / What / How)

### Task 2 (P0, FIRST): Remove Parent-Hash Rewrite Workaround

Why:
- Rewriting parent hash bypasses core header validity and makes blockchain conformance results non-authoritative.

What:
- Execute fixtures with true header parent hash values.

How:
- Remove workaround mutation in `tests/eels_blockchain_tests.rs`.
- Fix header hash/encoding parity path (`src/types/block.rs` and conversion assumptions) until parent linkage validates natively.
- Keep `expectException` handling for invalid blocks intact.

### Task 3 (P0): Support Blob Transactions in EELS Fixture Conversion

Why:
- Cancun/Prague fixtures include type-`0x03` transactions; rejecting them blocks large conformance surface area.

What:
- Extend fixture conversion to parse and build `Transaction::Blob`.

How:
- Add `0x03` branch in `convert_test_transaction` with strict field validation.
- Add conversion tests for valid blob txs and malformed cases.

### Task 4 (P0): Wire Real Block Hash History for `BLOCKHASH` Semantics

Why:
- Passing `&[]` for block hashes causes deterministic divergence for any fixture that depends on historical hashes.

What:
- Feed canonical parent-history hashes into `process_block` in fixture execution order.

How:
- Track prior canonical headers in harness and pass their hashes (up to required window) into `process_block`.
- Add regression tests around known BLOCKHASH fixtures.

### Task 5 (P0): Implement Precompile `0x0a` Point Evaluation

Why:
- EIP-4844 Cancun conformance requires point-evaluation behavior; current implementation always fails.

What:
- Implement calldata validation, gas accounting, proof verification, and output formatting for `0x0a`.

How:
- Mirror execution-spec behavior for point-evaluation precompile.
- Add vectors for valid proof, invalid proof, malformed input, and OOG behavior.

### Task 6 (P0): Complete Non-Trivial ALT_BN128 Pairing (`0x08`)

Why:
- Current pairing implementation only handles validation/identity fast paths and rejects non-trivial valid tuples.

What:
- Implement full pairing product check.

How:
- Add Miller loop + final exponentiation path with strict subgroup/field validation.
- Add conformance vectors for valid and invalid tuples.

### Task 7 (P0): Turn Blockchain EELS Execution Into a Hard Gate

Why:
- Compatibility cannot be claimed while the canonical blockchain suite is ignored and non-fatal.

What:
- Promote `test_execute_all_blockchain_tests` to a strict correctness gate.

How:
- Remove `#[ignore]` once preceding P0 blockers are resolved.
- Restore hard assertions on `failed == 0` and `errors == 0`.
- Keep diagnostics for triage only, not pass criteria.

### Task 8 (P1): Add Native vs RV32 Automated Parity Gate

Why:
- README claims both native and RV32 test execution; this is not currently enforced as an automated parity gate.

What:
- Add deterministic parity checks for curated fixture sets across both targets.

How:
- Add `uv run` Python parity driver (PEP 723 metadata) that runs native and runner workflows and compares results.
- Integrate parity command into local quality workflow.

### Task 9 (P1): Align README Claims With Verified Reality

Why:
- Public claims must match measured, enforced behavior.

What:
- Update README (or implementation) to remove unverifiable/inaccurate claims.

How:
- Resolve dependency claim mismatch (`serde` vs "no dependencies").
- Document exact scope/status of EELS coverage and native/RV32 parity checks.
