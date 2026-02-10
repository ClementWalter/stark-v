# Claudeth Completion Plan

This plan is based on direct inspection of `README.md`, `src/`, `tests/`, `execution-specs/`, and `learnings.md` on 2026-02-10.

## Current Status vs README

- `README.md` currently over-claims:
  - claims "no dependencies" while `Cargo.toml` depends on `serde`;
  - claims full `execution-spec-tests` compatibility, but `tests/eels_blockchain_tests.rs` still limits coverage (`InvalidBlocks` skipped, `.take(10)`, full run ignored, parent-hash workaround).
- Precompile dispatch status in `src/evm/precompiles.rs`:
  - implemented: `0x01..0x07`, `0x09`;
  - partially implemented: `0x08` now matches gas/framing envelope semantics and empty-input success, but non-empty pairing tuples are not implemented yet;
  - unimplemented: `0x0a` (`POINT_EVALUATION`) remains reserved-fail.

## Completed Work (Verified)

- Precompile addresses `0x08` and `0x0a` are now recognized as precompile addresses and no longer fall through to empty-code call path.
- Host-level call semantics preserve sub-call failure behavior for precompile errors (failed call, consumed forwarded gas, no value transfer).
- ALT_BN128 pairing (`0x08`) now enforces spec envelope semantics: gas formula, 192-byte framing, and empty-input `U256(1)` output.

## Remaining Tasks (Why / What / How)

### Task 1 (P0, First): Implement Full ALT_BN128 Pairing Arithmetic (`0x08` Non-Empty Inputs)

Why:
- EIP-197 correctness requires full tuple verification, subgroup checks, and pairing-product comparison.

What:
- Implement G2 decoding, subgroup checks, and pairing product check for all tuples.

How:
- Follow `execution-specs/.../alt_bn128.py` behavior exactly:
  - decode `G1(64)` + `G2(128)` per tuple;
  - field bounds + curve membership validation;
  - subgroup checks (`[curve_order]P == infinity`);
  - output `U256(1)` or `U256(0)`.

### Task 2 (P0): Implement POINT_EVALUATION Precompile (`0x0a`)

Why:
- Cancun correctness requires EIP-4844 proof verification semantics.

What:
- Implement fixed-gas, strict 192-byte parsing, versioned-hash check, proof verification, and canonical output constants.

How:
- Follow `execution-specs/src/ethereum/forks/cancun/vm/precompiled_contracts/point_evaluation.py`.
- Implement `kzg_commitment_to_versioned_hash` and `verify_kzg_proof` support or equivalent integration.

### Task 3 (P0): Make EELS Blockchain Harness Representative and Assertive

Why:
- Current harness setup cannot justify README-level compatibility claims.

What:
- Remove coverage shortcuts and convert informational runs into enforceable checks.

How:
- Remove `InvalidBlocks` skip and `.take(10)` truncation.
- Remove parent-hash overwrite workaround and validate canonical parent linkage.
- Handle `expectException` correctly for invalid fixtures.
- Enable CI-grade assertions in the execution test.

### Task 4 (P0): Introduce Fork-Aware Behavior Gating

Why:
- Multiple EELS fixtures depend on fork-specific rules; Cancun-first assumptions cause regressions on historical fixtures.

What:
- Thread fixture fork/network context into execution and gate rules by fork.

How:
- Parse fixture network/fork metadata.
- Gate precompiles/opcode behavior/header validation where rules diverge by fork.

### Task 5 (P1): RV32 vs Native Parity Automation

Why:
- Native-only passing does not prove RV32 correctness for the target runtime.

What:
- Add automated parity checks through the runner for representative scenarios.

How:
- Add a `uv run` Python harness comparing execution outputs and state roots across native and RV32 paths.

### Task 6 (P1): Align README Claims with Verifiable Reality

Why:
- Project claims should be continuously provable.

What:
- Update README claims and commands so they match current guaranteed behavior.

How:
- Keep only claims backed by passing automated checks.
- Document exact commands that validate each claim.
