# Claudeth Completion Plan

Last reviewed: 2026-02-10

## Ground Truth Snapshot

- `README.md` still claims full execution-spec compatibility and dual-target enforcement (native + RV32), but this is not currently guaranteed by hard gates.
- `cargo test -p claudeth --release` passes locally, including current unit/integration/doc tests.
- Full blockchain fixture execution (`test_execute_all_blockchain_tests` with `--ignored`) is still not a pass/fail gate and shows broad divergence in practice:
  - multi-chain parent selection mismatches (`InvalidHeader("parent hash does not match provided parent header")`),
  - frequent `GasUsedMismatch` across many valid/invalid suites,
  - `StateRootMismatch` and `WithdrawalsRootMismatch` clusters on fixture families.
- Deterministic harness gap confirmed and fixed this turn:
  - fixture tx type `0x03` (blob tx) existed in EELS fixtures but was unsupported by converter.
- Deterministic EVM gaps still present:
  - precompile `0x0a` (point evaluation) still unimplemented,
  - precompile `0x08` pairing still missing non-trivial full pairing product verification.

## Completed This Turn

### Task 1 (DONE): Prague Header Hash Parity and Parent-Linkage Harness Cleanup

Why:
- Fixture parent linkage is consensus-critical and must use real fixture hashes.

What:
- Added Prague `requests_hash` coverage and removed parent-hash rewrite workaround.

How:
- Updated header model/encoding tests and fixture-backed hash assertions.

### Task 2 (DONE): Blob Transaction Fixture Conversion (`0x03`)

Why:
- Cancun/Prague fixtures include blob transactions, and converter rejection caused deterministic coverage holes.

What:
- Added full fixture conversion support from type `0x03` JSON into `Transaction::Blob`.

How:
- Extended `TestTransaction` fixture schema with blob fields.
- Added strict conversion path in `convert_test_transaction`.
- Added tests:
  - fixture-backed blob conversion success,
  - malformed blob fixture rejection (`max_fee_per_blob_gas` missing).

## Priority Backlog (Why / What / How)

### Task 3 (P0, FIRST): Fix Canonical Parent Selection and BLOCKHASH Inputs in EELS Harness

Why:
- Multi-chain fixtures fail when harness assumes a single linear parent header.
- Passing `&[]` for recent block hashes guarantees wrong `BLOCKHASH` behavior.

What:
- Track executed canonical headers by hash and build per-block parent context correctly.
- Feed bounded recent hash history (oldest -> newest) into `process_block`.

How:
- Replace single `parent_header` variable with canonical-chain mapping logic.
- Select parent by fixture `parent_hash` for each block header.
- Derive recent hash window from canonical ancestry.
- Add regression tests for:
  - multi-chain fixture parent selection,
  - `BLOCKHASH` opcode sensitivity.

### Task 4 (P0): Correct Withdrawal Fixture Semantics and Root Parity

Why:
- Repeated `WithdrawalsRootMismatch` indicates fixture-body interpretation mismatch.

What:
- Reconcile fixture withdrawal parsing + encoding with execution-specs and EELS expectations for Shanghai+ invalid/edge fixtures.

How:
- Audit conversion and RLP encoding against reference execution-spec withdrawal trie construction.
- Add fixture-vector tests covering duplicate indexes, zero-amount, and bounds cases.

### Task 5 (P0): Close Systematic Gas Accounting Divergences

Why:
- `GasUsedMismatch` is widespread and blocks conformance claims.

What:
- Identify and fix deterministic gas deltas (opcode costs, warm/cold behavior, refunds, tx-level accounting).

How:
- Start from smallest reproducible failing fixtures (for example `bcValidBlockTest/timeDiff*` and `bcInvalidHeaderTest/timeDiff0`).
- Compare computed per-tx gas with reference traces/spec formulas.
- Add targeted regression tests for each corrected gas rule.

### Task 6 (P0): Close State Root Divergences in Valid Block Fixtures

Why:
- `StateRootMismatch` in baseline valid fixtures means STF semantics are still off.

What:
- Fix state-transition behavior for CREATE/REVERT/SELFDESTRUCT/other execution paths causing root drift.

How:
- Triage on minimal failing fixtures (`bcExample/*`, exploit/state cases).
- Validate account/storage/code transitions against reference behavior.
- Add deterministic state-root regression tests from fixtures.

### Task 7 (P0): Implement Precompile `0x0a` Point Evaluation

Why:
- EIP-4844 conformance requires point-evaluation precompile behavior.

What:
- Implement calldata validation, gas handling, verification path, and output formatting.

How:
- Mirror execution-spec behavior and add vectors for success, malformed input, invalid proof, and OOG.

### Task 8 (P0): Complete ALT_BN128 Pairing (`0x08`) for Non-Trivial Inputs

Why:
- Current implementation only handles validation + identity-equivalent fast paths.

What:
- Implement full pairing product equation for non-trivial tuples.

How:
- Add Miller loop + final exponentiation with subgroup checks.
- Add fixture-like vectors for valid/invalid multi-pair inputs.

### Task 9 (P0): Turn Full EELS Blockchain Test into a Hard Gate

Why:
- Compatibility cannot be claimed while the comprehensive suite is ignored and non-fatal.

What:
- Make `test_execute_all_blockchain_tests` a strict pass/fail validation step.

How:
- Remove `#[ignore]` only after P0 divergences are resolved.
- Enforce `failed == 0` and `errors == 0`.

### Task 10 (P1): Add Native vs RV32 Parity Automation

Why:
- README claims dual-target testing but parity is not currently enforced automatically.

What:
- Add deterministic native vs RV32 parity checks for curated fixture subsets.

How:
- Add a `uv run` Python runner (PEP 723 metadata) to execute both paths and compare outcomes.
- Integrate in local quality workflow.

### Task 11 (P1): Align README Claims With Verified Reality

Why:
- Public claims should reflect enforced behavior, not aspirational status.

What:
- Update README wording or implementation until claims are accurate.

How:
- Reconcile dependency claim vs `Cargo.toml`.
- Document exact conformance/test-scope guarantees.
