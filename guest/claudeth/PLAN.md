# Claudeth Implementation Plan

This plan is the source of truth for completion. It is based on code reality, not README claims.

## Current Reality Snapshot

- Implemented precompiles: `0x01..0x05` (ECRECOVER, SHA256, RIPEMD160, IDENTITY, MODEXP).
- Missing precompiles: `0x06..0x0a` (BN254 add/mul/pairing, BLAKE2F, point evaluation).
- No fork gating: execution currently applies post-Cancun-style rules broadly.
- EELS integration is partial:
  - skips `InvalidBlocks`;
  - ignores full execution test by default;
  - executes only a tiny subset in ignored mode (`take(10)`);
  - uses a parent-hash workaround in tests.
- RV32im execution tests are not wired through `../../crates/runner`.

## Priority Tasks (Why / What / How)

### Task 1 (P0): Implement Cancun-spec MODEXP precompile (`0x05`) and proper precompile OOG call semantics
Status: Completed.
Why:
- MODEXP is required by core blockchain/state fixtures and currently unimplemented.
- Current precompile call path can escalate insufficient precompile gas into a caller-level exceptional halt instead of a failed call result.
What:
- Add precompile `0x05` with Cancun gas formula (`EIP-2565`) and EELS calldata parsing (`buffer_read` zero-padding semantics).
- Ensure precompile insufficient gas returns `success = false` and consumes forwarded gas, instead of throwing caller OOG.
How:
- Follow `execution-specs/src/ethereum/forks/cancun/vm/precompiled_contracts/modexp.py`.
- Compute gas using `complexity/iterations` exactly per spec.
- Add focused tests for:
  - gas formula boundaries;
  - `base_len=0 && mod_len=0`;
  - modulus zero behavior;
  - OOG call behavior at host/interpreter boundary.

### Task 2 (P0): Implement remaining legacy precompiles (`0x06..0x09`)
Why:
- BN254 and BLAKE2F coverage is required by many execution fixtures.
What:
- Implement `ECADD`, `ECMUL`, `ECPAIRING`, `BLAKE2F` with exact input validation and output conventions.
How:
- Mirror execution-specs implementations and add deterministic vector tests.
- Add call-level OOG and malformed-input behavior tests.

### Task 3 (P0): Add fork-aware rules through STF, EVM, and transaction validation
Why:
- EELS fixtures span multiple forks; one-ruleset execution cannot pass full fixture sets.
What:
- Introduce a fork schedule model and gate:
  - transaction types/fields validity;
  - opcode availability;
  - precompile set and gas tables;
  - header field expectations (withdrawals/blob/beacon root/history behavior).
How:
- Parse fixture `network` into a fork enum.
- Thread fork context from test harness -> `process_block` -> validation/execution layers.

### Task 4 (P0): Make blockchain fixture execution complete and deterministic
Why:
- README-level compatibility requires running and validating real fixtures, including invalid blocks.
What:
- Remove subset/skip behavior and execute full fixture corpus with explicit pass/fail accounting.
- Handle expected exceptions and invalid blocks correctly.
How:
- Replace ad-hoc test flow with a fixture runner that maps execution errors to fixture exception categories.
- Remove parent-hash workaround and fix root-cause mismatches.

### Task 5 (P1): Implement point-evaluation precompile (`0x0a`) with fork gating
Why:
- Cancun fixtures rely on EIP-4844 point evaluation behavior.
What:
- Implement `POINT_EVALUATION` precompile and gate activation by fork.
How:
- Follow execution-specs Cancun implementation and add positive/negative vectors.

### Task 6 (P1): Add RV32im execution path in tests using the stark-v runner
Why:
- Project target is `riscv32im-unknown-none-elf`; native-only testing is insufficient.
What:
- Add `uv run` Python driver to compile and run `-p claudeth` guest binaries via `../../crates/runner`.
How:
- Mirror representative native tests on RV32im and fail CI if mismatched.

### Task 7 (P1): Enforce no_std and benchmark claims continuously
Why:
- Current claims (no_std, cycle efficiency) are not continuously validated.
What:
- Add explicit no_std build checks and reproducible benchmark scripts.
How:
- Use `uv run` scripts for deterministic build/benchmark pipelines and record baseline outputs in `benchmarks/`.
