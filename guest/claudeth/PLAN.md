# Claudeth Completion Plan

Last reviewed: 2026-02-10

## Ground Truth Snapshot

- `README.md` promises:
  - full compatibility with Ethereum execution-spec tests;
  - tests run on native and RV32 targets;
  - no external dependencies.
- The EELS blockchain execution harness remains `#[ignore]` and still contains the parent-hash rewrite workaround.
- `Cargo.toml` still includes `serde`, so the "no dependencies" claim is currently inaccurate.
- Precompile status in `src/evm/precompiles.rs`:
  - implemented: `0x01..0x07`, `0x09`;
  - partial: `0x08` (non-trivial pairing arithmetic still missing);
  - missing: `0x0a` point evaluation.
- Post-fix ignored-run sampling confirms a protocol constant mismatch:
  - expected empty trie root from fixtures: `0x56e81f17...5b48e01b...`;
  - computed in claudeth: `0x56e81f17...5b96e01b...`.
  This affects withdrawals and receipts root checks even for empty lists.

## Completed This Turn

### Task 1 (DONE): Wire Fixture Withdrawals Into Blockchain Execution

Why:
- The harness previously passed `withdrawals = vec![]` for every block, making Shanghai/Cancun withdrawals validation structurally wrong.

What:
- Added strict fixture withdrawal conversion and passed real fixture withdrawals to `process_block`.

How:
- Replaced `TestBlock.withdrawals` from `Vec<Value>` to `Vec<TestWithdrawal>`.
- Added `convert_test_withdrawal` (strict hex/decimal numeric parsing via existing parsers).
- Wired converted withdrawals into execution with block/withdrawal-index error context.
- Added conversion regression tests for success, invalid address, and out-of-range amount.

## Priority Backlog (Why / What / How)

### Task 2 (P0, FIRST): Fix Empty Trie Root Constant and Add Spec Vectors

Why:
- A wrong empty trie root constant causes deterministic root mismatches (`withdrawals_root`, `receipts_root`) on empty tries.

What:
- Correct the empty trie root constant and lock it with spec-vector tests.

How:
- Align `EMPTY_TRIE_ROOT` with execution-spec / Ethereum canonical value.
- Add tests for empty trie root and known withdrawals-root vectors from `execution-specs`.
- Re-run ignored blockchain execution sample to verify failure class shifts.

### Task 3 (P0): Remove Parent-Hash Rewrite Workaround and Execute Canonical Header Validation

Why:
- Rewriting parent hash bypasses core header validation and blocks conformance claims.

What:
- Stop mutating fixture headers and validate against true parent linkage.

How:
- Remove `block_header.parent_hash = parent_header.compute_hash()` workaround.
- Keep expected-invalid fixture handling (`expectException`) intact.
- Add regression checks for parent-hash mismatch behavior.

### Task 4 (P0): Enforce EELS Blockchain Assertions (Unignore + Hard Fail)

Why:
- Compatibility cannot be claimed while the main blockchain suite is ignored and soft-failing.

What:
- Promote blockchain execution to a strict correctness gate.

How:
- Unignore when P0 functional blockers are resolved.
- Restore strict final assertions on `failed` and `errors`.
- Keep detailed diagnostics for triage.

### Task 5 (P0): Complete ALT_BN128 Pairing (`0x08`) for Non-Trivial Inputs

Why:
- EIP-197 requires full pairing-product evaluation; current behavior rejects non-trivial valid inputs.

What:
- Implement full non-trivial pairing arithmetic with spec-conformant outputs.

How:
- Port execution-spec semantics for Miller loop + final exponentiation.
- Keep strict decoding/subgroup checks.
- Add vectors for valid tuples and malformed/subgroup-invalid cases.

### Task 6 (P0): Implement Point Evaluation Precompile (`0x0a`)

Why:
- Cancun compatibility requires EIP-4844 point-evaluation behavior.

What:
- Implement calldata validation, fixed gas behavior, KZG verification, and canonical output.

How:
- Mirror execution-spec Cancun point-evaluation logic.
- Add vectors for success, invalid proofs, and malformed input.

### Task 7 (P0): Introduce Fork-Aware Rule Gating Across STF/EVM Paths

Why:
- Fixtures span multiple eras; always-on modern rules create deterministic false failures.

What:
- Thread explicit fork capability context into header/tx validation and execution behavior.

How:
- Define fork flags used by validation and precompile availability checks.
- Feed fixture network metadata into execution context.
- Add cross-fork regression tests around Shanghai/Cancun boundaries.

### Task 8 (P1): Add Native vs RV32 Automated Parity Gate

Why:
- README says tests are run on both targets, but no automated parity gate enforces this.

What:
- Add reproducible native-vs-runner parity checks over curated fixtures.

How:
- Add a `uv run` Python driver (PEP 723 metadata) to execute both targets and diff outputs.
- Integrate into local and CI quality workflow.

### Task 9 (P1): Align README Claims With Verifiable Reality

Why:
- Public guarantees must match test-enforced behavior.

What:
- Update README and/or implementation so every claim is true and measurable.

How:
- Resolve dependency claim mismatch (`serde` vs "no dependencies").
- Document exact EELS and cross-target coverage actually enforced by tests.
