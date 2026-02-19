# Claudeth Completion Plan

Last reviewed: 2026-02-12

## Ground Truth Snapshot

- `README.md` claims:
  - full Ethereum execution compatibility,
  - tests validated on both native and `riscv32im-unknown-none-elf`,
  - no external crate dependencies.
- Current implementation status:
  - native EELS blockchain conformance gate exists in `tests/eels_blockchain_tests.rs` and runs by default (`test_execute_all_blockchain_tests` is non-ignored).
  - deterministic full-sweep traversal and hard failure/error assertions are already implemented.
  - deterministic native-vs-RV32 parity gate now exists for curated single-block Cancun/Prague fixtures (`basefeeExample`, `mergeExample`) and compares full guest output payloads.
  - RV32 guest execution is now wired for runner I/O symbols and entrypoint startup (`build.rs`, `linker.ld`, `src/main.rs`) with a stable RV32 allocator path and stack reservation.
  - `Cargo.toml` currently depends on external crates (`serde`, and test-time crates such as `serde_json`, `hex`, `walkdir`), which does not match the strict README dependency claim.

## Completion Objective

Make repository behavior and validation match README claims in a verifiable way:

- keep deterministic native full-suite conformance as a mandatory release gate,
- add deterministic native/RV32 parity checks for the same fixture inputs,
- close remaining README/code mismatches in dependency policy and validation scope.

## Priority Backlog (Why / What / How)

### Task 1 (P0, DONE): Add Deterministic Native vs RV32 Guest Parity Gate

Why:
- README claims both native and RV32 are validated, but current tests only enforce native behavior.
- Cross-architecture drift can silently appear from `no_std`, allocator, or VM I/O path differences even when native tests pass.

What:
- Add a release-mode integration parity test that executes curated EELS fixtures through:
  - native guest entrypoint binary path,
  - RV32 guest binary through `runner::run_with_input`.
- Assert deterministic equality of guest outputs (status/gas/root/error payload) per fixture.

How:
- Reuse existing fixture parsing/conversion logic in `tests/eels_blockchain_tests.rs`.
- Build canonical guest input payloads in the exact RLP format expected by `src/main.rs`.
- Compile RV32 guest once per test process, reuse ELF bytes, and compare native-vs-RV32 results fixture-by-fixture with actionable diagnostics.
- Use execution-spec references as guardrails:
  - `execution-specs/src/ethereum/forks/prague/fork.py` (`state_transition` strict header/body consistency checks),
  - `execution-spec-tests/src/ethereum_test_specs/blockchain.py` (parent/environment consistency),
  - `execution-spec-tests/src/ethereum_test_specs/helpers.py` (strict mismatch surfacing).

### Task 2 (P1, FIRST): Extend Parity Gate Beyond Single-Block Cases

Why:
- Single-block fixtures catch entrypoint and transaction semantics but miss chain-history-sensitive behavior.

What:
- Extend parity harness coverage to multi-block/branching fixtures.

How:
- Add deterministic state snapshot encoding from intermediate in-memory state into guest input state entries or witness format.
- Re-run parity assertions per block in chain order while preserving expected-invalid block handling.
- Keep curated RV32 runtime bounded by selecting representative cases and maintaining explicit `max_cycles` budgets.

### Task 3 (P1): Align Validation Scope with README “execution-spec-tests” Claim

Why:
- Current conformance harness is focused on `ethereum/tests` blockchain fixtures; README references execution-spec-tests-level compatibility.

What:
- Add an explicit mapping and gate that demonstrates covered execution-spec-tests corpus/suites and unsupported subsets.

How:
- Introduce deterministic suite selection + reporting of totals by fork/category.
- Fail CI if covered suites regress.

### Task 4 (P2): Close README Dependency Policy Mismatch

Why:
- README says no external dependencies; current manifest includes external crates.

What:
- Remove or isolate external dependencies to match policy, or introduce a strict no-deps build profile proven in CI.

How:
- Prioritize runtime path first (`src/` and guest binary), then test-only dependencies.
- Replace derive/serde-dependent code paths with in-tree encoders/parsers where needed.
- Add a CI check that fails when external crates appear in the runtime dependency graph.

### Task 5 (P2): Operational Hardening of Long Full-Sweep Runs

Why:
- Full deterministic sweep is long-running; reliability depends on explicit runtime expectations.

What:
- Preserve debuggability and reproducibility for long release-mode runs.

How:
- Keep per-case start markers and deterministic ordering.
- Document expected runtime envelope and required CI timeout/memory settings.
