# 2-to-1 Recursion Design

Goal: prove unlimited program lengths by splitting execution into fixed-size
segments (e.g. 100k or 1M steps), proving each segment independently, then
aggregating proofs pairwise (2-to-1) up a binary tree until a single root proof
remains.

## Single source of definition

The hard requirement: **no copy of any constraint, ever**. Editing
`define_trace_tables!` must flow to the recursive verifier automatically.

This already holds structurally, through two mechanisms:

1. **The macro DSL** (`stwo_macros::define_trace_tables!`): columns, derived
   columns, and constraints are declared once per table. The macro generates
   generic methods on `*Columns<T>` used by trace generation (`T = PackedM31`),
   AIR evaluation (`T = E::F`), and any future evaluator.

2. **Stwo's generic `EvalAtRow`**: each component's
   `FrameworkEval::evaluate<E: EvalAtRow>` is the _single_ constraint
   implementation. The prover instantiates it with a SIMD trace evaluator;
   `stwo::core::verifier::verify` instantiates the same function with a point
   evaluator at the OODS point. A verifier embedded in a guest program therefore
   links the exact same `prover` crate AIR modules — no transcription, no
   re-derivation.

So the recursion verifier is not a re-implementation of the AIR: it is
`prover::verifier::verify_rv32im` (or its no_std core) compiled to the guest
target, fed two child proofs.

## Architecture

```text
run(2N steps) ──► segment_0 (N steps) ──► proof_0 ─┐
                                                   ├─► aggregate(proof_0, proof_1) ──► proof_root
              └─► segment_1 (N steps) ──► proof_1 ─┘
```

- **Segmentation**: the runner stops every N steps and emits a segment boundary:
  `(pc, clock, registers commitment, memory commitment)`. Each segment proof
  exposes its entry and exit boundary as public data. The existing
  `registers_state` and `memory_access` LogUp relations already make the trace
  sound relative to those boundaries.
- **Aggregation guest**: a RISC-V guest program that
  1. reads two child proofs (+ their public data) from stdin,
  2. runs the stwo verifier on each (same `Components`, same `Relations`, same
     `evaluate` code as the host verifier),
  3. checks the boundary chain: `segment_0.exit == segment_1.entry`,
  4. commits the combined public data `(segment_0.entry, segment_1.exit)`.
- **Recursion step**: stark-v proves the aggregation guest's execution. The
  output proof has the same shape as a segment proof, so aggregation composes up
  the tree to arbitrary depth.

## Feasibility findings (verified 2026-06-10)

- Guest target is `riscv32im-unknown-none-elf`: no_std, no atomic instructions.
  Guests already have `alloc` via a bump allocator
  (`guest/guest-bin/src/heap.rs`), which the verifier needs (it allocates
  freely).
- The stwo verifier is no_std by design: prover code is behind the `prover`
  feature, and the `external/stwo/ensure-verifier-no_std/` CI gate compiles the
  verifier + constraint-framework with `default-features = false` (currently
  exercised for `wasm32-unknown-unknown`).
- `cargo check -p stwo --no-default-features --target riscv32im-unknown-none-elf`
  fails today. Blockers, in dependency order:
  1. `dashmap` is an unconditional stwo dependency but is used only in
     prover-side code (`prover/mempool.rs`, `prover/pcs/`,
     `prover/air/component_prover.rs`). It must become `optional = true`,
     enabled by the `prover` feature (fork change).
  2. `tracing-subscriber` is unconditional but used only in `src/tracing/`
     (gated by the `tracing` feature) — same treatment.
  3. `once_cell` (via `tracing-core` and `dashmap`) does not build for targets
     without atomics; with the deps above gated and `tracing` built with
     `default-features = false`, it drops out or needs the
     `critical-section`/`portable-atomic` route.
  4. The stark-v root workspace owns feature resolution for `external/stwo`
     crates (they are path members), so workspace-level `tracing = "0.1"` (std
     default) must not leak into the guest build — the aggregation guest builds
     in its own workspace (like `guest/guest-bin`, which is `exclude`d),
     avoiding this.

## Work plan

1. **Verifier-core extraction** — split the `prover` crate so the AIR side
   (component `Eval`s, `Claim`, `Relations`, `Components`, `verify_rv32im`)
   compiles without the witness side (`witness.rs`, SimdBackend, rayon). The
   per-component `air.rs` / `witness.rs` split already matches this boundary; it
   becomes a cargo feature (`witness`, on by default).
2. **no_std-ready codegen** — `define_trace_tables!` output for `prover_columns`
   uses `core::fmt`/`alloc::vec` instead of `std`, so the generated columns
   compile in the guest. Single source preserved: only the macro changes.
3. **Fork gating** — make `dashmap`/`tracing-subscriber` optional in the stwo
   fork (build-level change only; verifier logic untouched). Gate:
   `cargo check --no-default-features --target riscv32im-unknown-none-elf` green
   for `stwo` + `stwo-constraint-framework` + the AIR side of `prover`.
4. **Segmentation** — runner support for stopping at N steps with boundary
   export, and public-data plumbing of entry/exit boundaries; prove/verify a
   2-segment run on the host first (no recursion yet).
5. **Aggregation guest** — guest program wrapping the no_std verifier; prove it;
   wire the 2-to-1 tree in the SDK.

Remaining single-source gap (orthogonal to recursion, since the verifier reuses
`evaluate()`): LogUp relation entries are still written twice — once in `air.rs`
(`add_to_relation!`) and once in `witness.rs`
(`combine!`/`write_pair!`/`register_multiplicities`). Extending the macro with a
per-table `relations:` block would close it and shrink each component to its
`mod.rs`.
