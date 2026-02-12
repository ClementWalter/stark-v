# Claudeth Completion Plan

Last reviewed: 2026-02-11

## Ground Truth Snapshot

- `README.md` claims full EELS compatibility and native/RV32 validation.
- Current harness state:
  - `test_execute_all_blockchain_tests` is still `#[ignore]`.
  - Full-suite execution now has deterministic ordering (sorted fixture files and sorted case names).
  - Full-suite execution prints per-case start markers to make long fixtures diagnosable.
  - Full-suite execution asserts `failed == 0` and `errors == 0` when run.
- Release validation executed in this turn:
  - `cargo test -p claudeth --release --tests --no-run`
  - `cargo test -p claudeth --release test_can_parse_blockchain_tests`
  - `cargo test -p claudeth --release test_random_statetest99bc_`
  - `cargo test -p claudeth --release test_blockhash_tests_`
- Full-suite probes (ignored and non-ignored exploratory run) progressed through invalid suites, random blockhash suites, opcode matrices, and into wallet fixtures without observed failures before manual interruption due very long runtime (~47 minutes in one run).
- Native/RV32 parity is still not enforced as a gate.

## Completion Objective

Make implementation truthfully match `README.md` by ensuring:

- full EELS blockchain coverage completes to zero failures/errors in release mode;
- full-suite execution is stable enough to run as a default gate;
- native and RV32 paths are both validated with deterministic parity checks.

## Priority Backlog (Why / What / How)

### Task 1 (P0, DONE): Make Full EELS Sweep Deterministic and Instrumented

Why:
- Nondeterministic ordering and silent long phases made frontier capture unreliable.
- Long fixture phases were hard to distinguish from dead runs.

What:
- Make full-sweep traversal reproducible and observable.

How:
- Sort discovered fixture file paths.
- Sort per-file case names after JSON decode.
- Add per-case start markers.
- Keep hard `failed/errors` assertions in the full-sweep runner.

Reference implementation notes used:
- `execution-spec-tests/src/ethereum_test_specs/blockchain.py`
- `execution-spec-tests/src/ethereum_test_specs/helpers.py`

### Task 2 (P0, FIRST): Complete One Uninterrupted Full Ignored Sweep and Capture Final Totals

Why:
- We still need one complete end-to-end result to prove zero deterministic failures across the full supported fixture surface.

What:
- Run the full blockchain sweep to completion and record totals (`Total/Passed/Failed/Errors`).

How:
- Run: `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture`
- If runtime is excessive, capture slowest fixture families and profile/optimize runner paths before retrying.

### Task 3 (P0): Promote Full Sweep to Default Non-Ignored Gate After Stable Zero-Failure Completion

Why:
- README compatibility claims require default enforcement, not optional/manual checks.

What:
- Remove `#[ignore]` once Task 2 demonstrates stable completion and acceptable runtime for normal release validation loops.

How:
- Un-ignore `test_execute_all_blockchain_tests`.
- Keep deterministic ordering and hard assertions.
- Re-run release hooks and ensure operational viability.

### Task 4 (P1): Frontier-Driven Fix Loop if Any Fixture Fails

Why:
- Any deterministic mismatch invalidates compatibility claims.

What:
- Capture first failing family, add focused regression, patch minimal semantic delta, rerun.

How:
- Loop until full sweep totals are zero failures and zero errors.

### Task 5 (P1): Enforce Native vs RV32 Deterministic Parity

Why:
- README states both targets are validated, but no parity gate currently enforces this.

What:
- Add a release-mode parity suite over high-signal fixtures.

How:
- Run identical fixture subset on native and runner.
- Compare post-state root, receipts root, logs bloom, and gas used.
- Fail on any divergence.
