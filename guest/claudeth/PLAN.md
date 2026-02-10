# Claudeth Completion Plan

This plan is grounded in code and test reality as of 2026-02-10, not README claims.

## Current Reality Snapshot

- `cargo test --release -p claudeth` passes locally for unit/integration tests, but this is not equivalent to full EELS compatibility.
- Implemented precompiles in code path: `0x01..0x06` (`ECADD` added).
- Missing precompiles in active dispatcher: `0x07..0x0a` (`ECMUL`, `ECPAIRING`, `BLAKE2F`, `POINT_EVALUATION`).
- Blockchain fixture harness is intentionally partial:
  - `InvalidBlocks` are skipped at discovery.
  - Full execution test is `#[ignore]`.
  - Even ignored execution path only runs `.take(10)`.
  - Parent hash is overwritten in test execution as a workaround.
- Transaction/block validation applies a mostly Cancun-style ruleset without fixture-network-driven fork gating.
- RV32im execution validation via `../../crates/runner` is not wired in the automated test path.

## Priority Tasks (Why / What / How)

### Task 1 (P0): Implement ALT_BN128 ECADD precompile (`0x06`) end-to-end
Status: Completed (2026-02-10).

Why:
- `ECADD` is a mandatory legacy precompile and currently unimplemented, so any fixture touching `0x06` cannot pass.
- `ECADD` is the smallest missing cryptographic precompile and unblocks subsequent BN254 work (`0x07`, `0x08`) with shared parsing/curve primitives.

What:
- Add precompile dispatcher support for address `0x06`.
- Implement exact EELS-compatible `ECADD` behavior:
  - input is two G1 points from zero-padded 64-byte slices;
  - reject coordinates `>= field_modulus`;
  - accept `(0,0)` as infinity;
  - reject non-curve points;
  - on invalid input, fail the sub-call via precompile error semantics.
- Charge fixed gas `150` (Istanbul+ schedule used by current codebase gas constants).

How:
- Mirror semantics from `execution-specs/src/ethereum/forks/prague/vm/precompiled_contracts/alt_bn128.py` (`bytes_to_g1`, `alt_bn128_add`).
- Reuse existing big-integer/modular arithmetic infrastructure (`U256`/`U512`) and implement BN254 affine add/double with explicit infinity handling.
- Add deterministic unit tests for:
  - addition of two valid points;
  - infinity identity behavior;
  - invalid field element and invalid-curve input -> precompile failure;
  - fixed gas charge and OOG behavior.

### Task 2 (P0): Implement ALT_BN128 ECMUL precompile (`0x07`)
Status: Next priority.

Why:
- Fixtures commonly pair `ECADD` and `ECMUL`; partial BN254 support still blocks broad compatibility.

What:
- Implement scalar multiplication over BN254 G1 with spec parsing and gas (`6000`).

How:
- Reuse Task 1 parsing and curve primitives.
- Match execution-specs behavior for malformed points and short input via zero-padding reads.
- Add vectors and OOG/invalid-input tests.

### Task 3 (P0): Implement ALT_BN128 pairing precompile (`0x08`)

Why:
- Pairing is required by core consensus-era fixture coverage and many real contracts.

What:
- Implement pair list decoding (192-byte chunks), subgroup checks, and boolean 32-byte output semantics.

How:
- Follow execution-specs `alt_bn128_pairing_check` behavior exactly, including invalid length handling and gas formula `45000 + 34000 * n`.
- Add focused tests for empty input, valid pairing, invalid length, invalid points.

### Task 4 (P0): Implement BLAKE2F precompile (`0x09`)

Why:
- EIP-152 behavior is fixture-visible and currently missing.

What:
- Implement round function precompile with strict 213-byte input format and final-block flag validation.

How:
- Port execution-specs logic and add test vectors from EIP/execution-spec-tests.

### Task 5 (P0): Implement POINT_EVALUATION precompile (`0x0a`) with correct activation assumptions

Why:
- Cancun/Prague coverage needs EIP-4844 point evaluation semantics.

What:
- Add precompile logic and wire it in dispatcher.

How:
- Follow execution-specs point evaluation implementation and expected error/output behavior.
- Add positive/negative vectors and OOG tests.

### Task 6 (P0): Introduce explicit fork/network gating throughout validation and execution

Why:
- A single always-on ruleset cannot satisfy mixed-fork fixture expectations.

What:
- Parse fixture network/fork and gate tx types, opcodes, precompiles, and header field constraints.

How:
- Add a fork enum/schedule and thread it through STF and interpreter contexts.
- Replace unconditional assumptions (e.g., always-warm `0x01..0x0a`, blob context expectations) with fork-aware logic.

### Task 7 (P0): Replace partial EELS harness with full deterministic fixture runner

Why:
- Current harness is explicitly scoped to debugging and does not validate README-level compatibility.

What:
- Execute full fixture set (including invalid blocks) with expected exception matching and deterministic reporting.

How:
- Remove `InvalidBlocks` skip, `.take(10)`, and parent-hash override.
- Map block/tx execution errors to fixture `expectException` outcomes.
- Make the execution test non-ignored once deterministic.

### Task 8 (P1): Add RV32im execution-path verification with `../../crates/runner`

Why:
- Project target is RV32im; native-only correctness is insufficient.

What:
- Add an automated `uv run` Python driver that compiles and executes representative tests through the runner.

How:
- Create a reproducible runner harness for `-p claudeth` binaries and assert parity against native expectations.

### Task 9 (P1): Continuously verify no_std and benchmark claims

Why:
- README claims require ongoing proof, not ad-hoc local checks.

What:
- Add deterministic checks for `no_std`-compatible build path and benchmark reproducibility.

How:
- Add explicit CI/local commands (via `uv run` Python scripts) and capture baseline outputs in `benchmarks/`.
