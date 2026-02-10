# Claudeth Completion Plan

Last reviewed: 2026-02-10

## Ground Truth Snapshot

- `cargo test -p claudeth --release` passes locally.
- `tests/eels_blockchain_tests.rs` still does not prove end-to-end compatibility:
  - `test_execute_all_blockchain_tests` is `#[ignore]`;
  - the parent hash is still overwritten with a local workaround;
  - final failure assertions are still commented out.
- Precompile status in `src/evm/precompiles.rs`:
  - implemented: `0x01..0x07`, `0x09`;
  - partially implemented: `0x08` (strict tuple decoding + G2 validation + infinity identity success; non-trivial pairing arithmetic still missing);
  - not implemented: `0x0a` (`POINT_EVALUATION`) is still reserved-fail.
- README contract is not yet satisfied by enforceable checks:
  - it claims full `execution-spec-tests` compatibility, but the execution harness is not enforcing zero failures;
  - it claims all tests are run on native and RV32im, but CI-level parity checks are not present in this crate.

## Completed This Turn

- Implemented Task 1 in `src/evm/precompiles.rs`:
  - added strict G2 tuple decoding and on-curve validation aligned with execution-spec byte ordering;
  - added non-empty identity fast path for pairing tuples involving infinity points;
  - kept non-trivial pairing arithmetic explicitly unsupported (deterministic failure).
- Added/updated pairing regression tests:
  - all-zero tuple now returns `1`;
  - infinity-G1 + valid-G2 tuple returns `1`;
  - malformed G2 field element fails;
  - non-trivial tuple remains pending and fails.
- Re-ran quality gates:
  - `cargo test -p claudeth --release`;
  - `prek run --all-files`.

## Priority Backlog (Why / What / How)

### Task 1 (P0, DONE): Pairing Input Validation + Infinity Identity Fast Path

Why:
- Valid non-empty pairing calldata where every pair is identity-equivalent (infinity cases) should succeed per EIP-197 semantics.
- Previously, all non-empty inputs failed, which created guaranteed false negatives even before full pairing arithmetic was added.

What:
- Implement strict tuple decoding for `0x08` (G1 + G2) with canonical field/layout checks.
- Implement the pairing identity fast path: if all decoded pairs are trivial (contain infinity), return success `U256(1)`.
- Keep non-trivial pairings explicitly unimplemented for now.

How:
- Mirror `bytes_to_g2` framing from `execution-specs/src/ethereum/forks/cancun/vm/precompiled_contracts/alt_bn128.py`:
  - 128-byte G2 decoding, field modulus checks, coefficient order parity (`x = (x1, x0)`, `y = (y1, y0)`), and on-curve validation.
- Parse all tuples after gas/malformed-length checks.
- For each tuple, detect infinity-only contributions and short-circuit to deterministic success when every tuple is trivial.
- Add Rust tests covering:
  - one all-zero tuple succeeds with `1`;
  - malformed G2 coordinates fail;
  - non-trivial valid tuple remains explicitly unsupported.

### Task 2 (P0, FIRST): Complete ALT_BN128 Non-Trivial Pairing Arithmetic

Why:
- EIP-197 correctness requires full pairing product evaluation for non-infinity tuples.

What:
- Implement non-trivial pairing result computation and canonical success/failure output (`1`/`0`).

How:
- Follow execution-spec semantics for subgroup checks and pairing product evaluation.
- Add finite field extension arithmetic required by Miller loop/final exponentiation.
- Preserve failure mode semantics for malformed/subgroup-invalid tuples.

### Task 3 (P0): Implement POINT_EVALUATION Precompile (`0x0a`)

Why:
- Cancun/4844 fixtures require this precompile; reserved-fail cannot pass conformance.

What:
- Implement strict input validation, fixed gas, proof verification, and canonical output words.

How:
- Follow `execution-specs/src/ethereum/forks/cancun/vm/precompiled_contracts/point_evaluation.py`.
- Implement versioned hash derivation and proof verification semantics equivalent to the reference.
- Add success/failure vectors from execution-spec tests.

### Task 4 (P0): Remove Blockchain Harness Workarounds and Enforce Assertions

Why:
- Compatibility claims are not meaningful while the harness is ignored and mutates header linkage.

What:
- Convert EELS blockchain test execution into an enforceable correctness gate.

How:
- Remove parent-hash overwrite workaround.
- Keep full diagnostic output, but make summary checks hard-fail (`failed == 0`, `errors == 0`) once precompile and fork gaps are closed.
- Keep parser coverage broad across valid + invalid block suites.

### Task 5 (P0): Add Fork-Aware Rule Gating

Why:
- Fixtures span multiple hard forks; always-on Cancun-era behavior causes deterministic mismatches.

What:
- Thread fork/network metadata into execution paths and gate rules accordingly.

How:
- Parse and propagate fixture network/fork context.
- Gate precompile availability (notably `0x0a`) and fork-specific transaction/block validation.
- Add focused regression tests across at least two fork boundaries.

### Task 6 (P1): Add Native vs RV32im Parity Checks

Why:
- README claims cross-target testing; native-only success is insufficient.

What:
- Add reproducible parity checks between native and runner execution.

How:
- Add a `uv run` Python script (PEP 723 metadata) that executes a curated fixture set on both targets and compares deterministic outputs.
- Integrate parity checks into the project test workflow.

### Task 7 (P1): Align Implementation with README Contract

Why:
- Remaining README claims should be enforceable through code and tests, not aspirational.

What:
- Close the last contract gaps once functional parity tasks above are complete.

How:
- Reconcile any remaining mismatch between documented guarantees and verified behavior.
- If strict zero-dependency is required, remove residual non-core dependencies as a dedicated follow-up.
