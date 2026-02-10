# Claudeth Completion Plan

This plan is based on direct inspection of `README.md`, `src/`, `tests/`,
`execution-specs/`, and `learnings.md` on 2026-02-10.

## Reality Check vs README

- `README.md` claims full execution-spec compatibility, but the current EELS
  harness is still partial:
  - skips `InvalidBlocks`,
  - executes only a small subset (`.take(10)`),
  - is disabled for full execution (`#[ignore]`),
  - uses a parent-hash workaround.
- Precompile coverage is incomplete in `src/evm/precompiles.rs`:
  - implemented: `0x01..0x07`, `0x09`;
  - missing: `0x08` (`ALT_BN128_PAIRING`), `0x0a` (`POINT_EVALUATION`).
- RV32 parity is not enforced by repeatable automated checks through
  `../../crates/runner/`.

## Completed Baseline

- Native `cargo test -p claudeth --release` passes locally.
- Existing precompile behavior (`0x01..0x07`, `0x09`) includes gas-aware failure
  semantics at call boundaries.
- ECADD/ECMUL implementations already follow EELS-style zero-padding and strict
  malformed-point rejection semantics.
- BLAKE2F (`0x09`) now matches EIP-152 input layout/endian rules, per-round gas
  accounting, and execution-spec vectors (`rounds=0` and `rounds=12`).

## Remaining Tasks (Ordered by Priority)

### Task 1 (P0): Implement ALT_BN128 pairing precompile (`0x08`)

Why:
- Execution-spec tests and production contracts rely on EIP-197 pairing.
- Missing `0x08` causes semantic mismatches for valid precompile calls.

What:
- Add `0x08` dispatch and pairing-check implementation over BN254 G1/G2.
- Return 32-byte boolean result, with strict malformed input handling.

How:
- Follow `execution-specs` `alt_bn128_pairing_check` rules:
  - input length multiple of 192;
  - strict field and curve validation for G1/G2;
  - subgroup checks (`[curve_order]P == inf`);
  - gas `GAS_BN256_PAIRING_BASE + n * GAS_BN256_PAIRING_POINT`.
- Add positive/negative vectors and OOG tests.

### Task 2 (P0): Implement POINT_EVALUATION precompile (`0x0a`)

Why:
- Cancun-era correctness depends on EIP-4844 point-evaluation behavior.

What:
- Add `0x0a` dispatch and strict 192-byte input handling.
- Verify versioned hash and KZG proof semantics.

How:
- Match `execution-specs` `point_evaluation` behavior:
  - fixed gas `GAS_POINT_EVALUATION`,
  - versioned-hash check,
  - proof verification and expected 64-byte output constants.
- Add valid and invalid proof/path tests, including gas behavior.

### Task 3 (P0): Remove EELS blockchain harness shortcuts

Why:
- Current harness cannot substantiate README-level compatibility.

What:
- Make the blockchain test integration deterministic and representative.

How:
- Remove `InvalidBlocks` skip and `.take(10)` truncation.
- Remove parent-hash overwrite workaround.
- Implement robust `expectException` matching for invalid fixtures.
- Unignore full execution test once deterministic.

### Task 4 (P0): Add explicit fork-aware rule gating

Why:
- Hard-coded modern assumptions can break historical fixtures.

What:
- Thread fork/network context through STF/EVM rule checks.

How:
- Use fixture network metadata to gate tx/header/precompile/opcode behavior.
- Replace unconditional post-Cancun assumptions with fork-conditioned logic.

### Task 5 (P1): Add automated RV32 parity checks via runner

Why:
- Native passing tests do not prove RV32 correctness.

What:
- Add reproducible checks that compare native and RV32 outputs.

How:
- Add a small `uv run` Python parity harness for representative scenarios
  (precompiles + STF transitions).
- Wire the harness into local validation workflow.

### Task 6 (P1): Make README claims continuously verifiable

Why:
- Claims should be backed by explicit, repeatable evidence.

What:
- Align documentation claims with enforceable checks and current scope.

How:
- Either reduce unsupported claims or add the missing automation/tests.
- Document exact commands that prove each claim.
