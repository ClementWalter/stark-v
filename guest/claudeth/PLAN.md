# Claudeth Completion Plan

Last reviewed: 2026-02-12

## Ground Truth Snapshot

- `README.md` claims full EELS compatibility and test validation on native and RV32im.
- Release validation executed:
  - `cargo test -p claudeth --release` passed (`57` EELS focused tests passed, `1` full-sweep test ignored at that moment).
  - `cargo test -p claudeth --release test_execute_all_blockchain_tests -- --ignored --nocapture` passed end-to-end with:
    - `Total: 1142`
    - `Passed: 1142`
    - `Failed: 0`
    - `Errors: 0`
    - Runtime: `3312.94s` (~55m13s)
- Full-suite runner behavior now matches reference expectations:
  - deterministic traversal (sorted file paths + sorted case names),
  - per-case start markers for long silent fixtures,
  - hard assertions on `failed == 0` and `errors == 0`.
- Remaining mismatch with README claim:
  - no deterministic native-vs-RV32 parity gate yet.

## Completion Objective

Make repository truthfully satisfy `README.md` claims by ensuring:

- full EELS blockchain sweep is default, mandatory, and deterministic in release mode;
- native and RV32im executions are both exercised with explicit parity checks;
- long-running full-sweep behavior is operationally explicit and reproducible.

## Priority Backlog (Why / What / How)

### Task 1 (P0, DONE): Promote Full EELS Sweep to Default Gate

Why:
- README-level compatibility must be enforced by default, not hidden behind `--ignored`.
- We now have a completed zero-failure baseline (`1142/1142`) proving the suite is runnable.

What:
- Removed `#[ignore]` from `test_execute_all_blockchain_tests` so default release tests enforce full blockchain conformance.

How:
- Kept deterministic ordering, per-case markers, and hard failure/error assertions intact.
- Preserved 128 MiB stack runner for deep fixtures.

### Task 2 (P1, FIRST): Add Deterministic Native vs RV32 Parity Gate

Reference implementation notes used:
- `execution-specs/src/ethereum/forks/prague/fork.py` (`state_transition` block-level validity checks must hard-fail on mismatches).
- `execution-spec-tests/src/ethereum_test_specs/blockchain.py` (fixture execution enforces parent/environment consistency and explicit exception handling).
- `execution-spec-tests/src/ethereum_test_specs/helpers.py` (strict mismatch surfacing for unexpected success/failure).

Why:
- README claims both native and RV32 paths are validated, but no parity assertion currently proves identical outcomes.

What:
- Add a release-mode parity test harness for a curated high-signal fixture set.

How:
- Execute same fixture inputs on native and RV32 runner.
- Compare state root, receipts root, logs bloom, gas used, and status.
- Fail on any divergence with concise diagnostics.

### Task 3 (P1): Operationalize Full-Sweep Runtime Expectations

Why:
- Full default sweep now takes ~55 minutes; silent phases can look stalled and create false triage.

What:
- Make runtime expectations explicit and CI-friendly without weakening coverage.

How:
- Document expected duration and silent fixture families.
- Keep per-case markers as progress heartbeat.
- Ensure CI timeout/memory settings are compatible with observed runtime envelope.

### Task 4 (P1): Frontier-Driven Semantic Fix Loop (Conditional)

Why:
- Once full sweep is default, any future deterministic failure blocks release readiness.

What:
- Use failing fixture as regression frontier and patch minimal semantic deltas.

How:
- Capture first failing case from deterministic traversal.
- Add focused regression test.
- Patch implementation and rerun release sweep until `failed=0` and `errors=0`.
