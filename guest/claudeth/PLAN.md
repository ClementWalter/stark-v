# Claudeth Completion Plan

This plan is grounded in actual code/test behavior as of 2026-02-10 and is the
source of truth for remaining work.

## Current Reality Snapshot

- Unit/integration tests pass locally with `cargo test --release -p claudeth`,
  but this does not prove full execution-spec compatibility.
- Precompile dispatcher currently supports `0x01..0x06` only.
- `ECADD` now uses BN254 field-modulus parsing/arithmetic and is covered by
  canonical EIP-196 vectors in unit tests.
- Missing precompiles in active path: `0x07` (`ECMUL`), `0x08` (`ECPAIRING`),
  `0x09` (`BLAKE2F`), `0x0a` (`POINT_EVALUATION`).
- EELS blockchain harness is intentionally partial (`InvalidBlocks` skipped,
  execution test ignored, `.take(10)`, parent-hash override workaround).
- Validation/execution is mostly always-on modern rules, without robust
  fixture-network fork gating.
- Automated RV32im test execution through `../../crates/runner` is not wired.
- README claims are not fully proven by code/tests yet (`no dependencies`,
  full EELS compatibility, RV32im+native parity).

## Priority Tasks (Why / What / How)

### Task 1 (P0): Fix ALT_BN128 ECADD correctness (`0x06`) to match execution-spec semantics
Status: Completed (2026-02-10).

Why:
- Current `ECADD` arithmetic is over the wrong modulus, so it can return values
  that are self-consistent in tests but invalid against EELS vectors.
- Keeping a wrong BN254 base makes all later BN254 work (`ECMUL`, `ECPAIRING`)
  structurally unsound.

What:
- Use BN254 field modulus for G1 parsing and curve arithmetic.
- Keep existing call-failure behavior for malformed points (failed sub-call).
- Rebase tests on canonical vectors from execution-specs EIP-196 fixtures.

How:
- Mirror `bytes_to_g1` and `alt_bn128_add` behavior from
  `execution-specs/src/ethereum/forks/byzantium/vm/precompiled_contracts/alt_bn128.py`.
- Update constants and expected outputs in `src/evm/precompiles.rs` tests to
  match known vectors (`G1`, `[2]G1`, `P1 + Q1 = R1`).
- Verify invalid field elements / off-curve points still fail and OOG behavior
  is unchanged.

### Task 2 (P0): Implement ALT_BN128 ECMUL precompile (`0x07`)

Why:
- Many fixtures and contracts require both `ECADD` and `ECMUL`; missing `0x07`
  blocks broad compatibility.

What:
- Add dispatcher route for `0x07`.
- Implement scalar multiplication with zero-padded input parsing and fixed gas
  charge (`6000` in current Istanbul+ schedule constants).

How:
- Reuse Task 1 G1 decode/validation and point ops.
- Parse scalar from `buffer_read(data, 64, 32)` semantics.
- Add vectors for scalar `0`, `1`, `2`, short-input zero-padding, malformed
  points, and OOG.

### Task 3 (P0): Implement ALT_BN128 pairing precompile (`0x08`)

Why:
- Pairing is heavily exercised in EELS fixtures and is required by deployed
  contracts.

What:
- Decode 192-byte tuples, validate points/subgroup, and return 32-byte boolean.

How:
- Follow execution-spec behavior for invalid length/invalid point failure.
- Use gas formula constants already present in `src/evm/gas.rs`.
- Add focused positive/negative fixture-derived tests.

### Task 4 (P0): Implement BLAKE2F precompile (`0x09`)

Why:
- EIP-152 fixture coverage is currently impossible without `0x09`.

What:
- Implement strict 213-byte input decoding, flag validation, and round-based
  gas metering.

How:
- Port execution-spec behavior and validate against known vectors.
- Add success, malformed-input, and OOG tests.

### Task 5 (P0): Implement POINT_EVALUATION precompile (`0x0a`)

Why:
- Cancun/Prague-era tests require EIP-4844 point-evaluation semantics.

What:
- Add dispatcher + implementation with exact input/output/error behavior.

How:
- Mirror execution-spec implementation and add deterministic vectors/OOG tests.

### Task 6 (P0): Replace partial EELS blockchain harness with full deterministic runner

Why:
- Current harness cannot support README-level compatibility claims.

What:
- Execute full fixture set (including invalid blocks) with exception matching.

How:
- Remove `InvalidBlocks` skip, `.take(10)`, and parent-hash override.
- Map `expectException` to concrete block/tx processing errors.
- Un-ignore end-to-end execution test once deterministic.

### Task 7 (P0): Add explicit fork/network gating across STF + EVM validation

Why:
- Mixed-fork fixtures cannot be satisfied by a single always-on ruleset.

What:
- Introduce fork schedule and gate tx/header/opcode/precompile behavior.

How:
- Parse fixture network/fork and thread context through execution.
- Replace unconditional assumptions (e.g., precompile warm set, blob rules).

### Task 8 (P1): Add automated RV32im verification path through `../../crates/runner`

Why:
- Native-only testing does not validate the declared target architecture.

What:
- Build and run representative test flows on RV32im in automation.

How:
- Add reproducible `uv run` Python harness that executes runner flows for
  `-p claudeth` artifacts and checks parity with native outputs.

### Task 9 (P1): Close README claim gaps with enforceable checks

Why:
- Claims should be continuously verifiable, not implicit.

What:
- Add checks for no_std discipline, dependency constraints, and benchmark
  reproducibility.

How:
- Add deterministic validation commands/scripts and baseline artifacts.
