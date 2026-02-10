# Claudeth Completion Plan

This plan is based on direct inspection of `README.md`, `src/`, current tests, and execution-spec references as of 2026-02-10.

## Reality Check vs README

- `cargo test --release -p claudeth` passes locally (unit, integration, doctests).
- Precompiles now implemented in active dispatcher: `0x01..0x07` (`ECADD` + `ECMUL` included).
- Precompiles still missing in active dispatcher: `0x08` (`ECPAIRING`), `0x09` (`BLAKE2F`), `0x0a` (`POINT_EVALUATION`).
- EELS blockchain harness is still intentionally partial (`InvalidBlocks` skipped, only subset executed, parent-hash workaround), so full compatibility claim is not yet proven.
- Fork/network behavior is still mostly hard-coded modern rules, not robustly gated per fixture network.
- RV32im automated parity path through `../../crates/runner` is not yet wired into repeatable checks.

## Completed in This Turn

### Done: ALT_BN128 ECMUL precompile (`0x07`)

Why:
- `0x07` was missing from dispatch, causing incorrect empty-code behavior for valid precompile calls.

What:
- Added `0x07` dispatch and implemented scalar multiplication over validated BN254 G1 points.
- Matched EELS semantics: zero-padded input decoding, fixed gas charge (6000), malformed point -> failed sub-call.
- Added focused tests for scalar `0/1/2`, short-input padding, malformed points, and OOG.

How:
- Mirrored execution-specs `alt_bn128_mul` behavior (`buffer_read` + strict point validation + fixed gas).
- Reused local BN254 parsing/add/double helpers and encoded infinity as 64 zero bytes.

## Remaining Priority Tasks (Why / What / How)

### Task 1 (P0): Implement ALT_BN128 pairing precompile (`0x08`)

Why:
- Pairing is required by many execution-spec fixtures and deployed contracts; without it, README-level compatibility cannot be achieved.

What:
- Add dispatcher route for `0x08`.
- Decode `192 * n` input tuples, validate G1/G2 points and subgroup rules, and return 32-byte boolean result.

How:
- Mirror execution-spec `alt_bn128_pairing_check` behavior exactly:
  - reject non-multiple-of-192 input as failed precompile call;
  - map malformed points/subgroup failures to failed sub-call semantics;
  - charge gas as `GAS_BN256_PAIRING_BASE + n * GAS_BN256_PAIRING_POINT`.
- Add canonical vector tests and failure-path tests (bad length, invalid field element, invalid subgroup, OOG).

### Task 2 (P0): Implement BLAKE2F precompile (`0x09`)

Why:
- EIP-152 fixtures and real call paths require exact BLAKE2F semantics.

What:
- Implement strict 213-byte input format, final flag validation, and per-round gas.

How:
- Port execution-spec behavior with byte-for-byte layout compatibility.
- Add tests for valid vectors, malformed length/flag, and out-of-gas paths.

### Task 3 (P0): Implement POINT_EVALUATION precompile (`0x0a`)

Why:
- Cancun/Prague-era fixtures require EIP-4844 point-evaluation behavior.

What:
- Add `0x0a` dispatch and exact input/output/error handling.

How:
- Follow execution-spec implementation and constants.
- Add deterministic happy-path and failure-path/OOG tests.

### Task 4 (P0): Remove EELS blockchain harness shortcuts and execute deterministically

Why:
- Current harness does not substantiate the README claim of full execution-spec compatibility.

What:
- Run full blockchain fixture coverage (including invalid blocks) with deterministic outcomes.

How:
- Remove `InvalidBlocks` skip, `.take(10)`, and parent-hash override workaround.
- Implement robust exception mapping (`expectException`) to concrete execution/validation errors.
- Re-enable ignored full-run test only when deterministic.

### Task 5 (P0): Add explicit fork/network gating across STF + EVM rules

Why:
- Mixed-fork fixtures cannot pass with a single always-on ruleset.

What:
- Introduce explicit fork schedule context and gate tx/header/opcode/precompile behavior by fork.

How:
- Parse fixture network/fork metadata and thread it through STF/EVM execution.
- Replace unconditional Cancun/Prague assumptions with fork-aware checks.

### Task 6 (P1): Add automated RV32im parity verification through `../../crates/runner`

Why:
- Native-only success does not prove target-architecture correctness.

What:
- Add reproducible automation that runs representative flows on RV32im and checks parity with native outputs.

How:
- Add a `uv run` Python harness for build/run/compare over `-p claudeth` artifacts.
- Include at least precompile-focused and STF-focused parity cases.

### Task 7 (P1): Make README claims continuously verifiable

Why:
- Claims should be enforced by reproducible checks, not manual interpretation.

What:
- Add enforceable checks for compatibility scope, architecture parity, and benchmark reproducibility.

How:
- Add deterministic commands/scripts and artifacts so CI/local runs can validate each claim.
