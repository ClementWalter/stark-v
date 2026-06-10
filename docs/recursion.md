# 2-to-1 Recursion Design (native stwo verifier AIR)

Goal: prove unlimited program lengths by splitting execution into fixed-size
segments (e.g. 100k or 1M steps, configurable), proving each segment, then
aggregating proofs pairwise (2-to-1) up a binary tree until a single root proof
remains.

The aggregator is **not** a RISC-V guest re-executing a Rust verifier. It is a
**native stwo AIR**: a set of stwo components whose constraints assert "these
two stark-v proofs verify, and their boundaries chain". The recursion prover
takes two stwo proofs as witness input and produces a stwo proof of their
verification.

Working in our favor: the inner and outer proofs share the same field
(M31/QM31), so no field emulation is needed anywhere — channel, FRI, and
composition arithmetic are all native.

## Single source of definition

The hard requirement: **no copy of any constraint, ever**. Editing
`define_trace_tables!` must flow through to the recursive verifier.

The mechanism is `EvalAtRow` genericity, and it is already load-bearing in stwo:

- The prover instantiates each component's
  `FrameworkEval::evaluate<E: EvalAtRow>` with a SIMD trace evaluator.
- The verifier instantiates the same function with `PointEvaluator`
  (`constraint-framework/src/point.rs`) to evaluate the composition polynomial
  at the OODS point from sampled mask values.
- `ExprEvaluator` (`constraint-framework/src/expr/`) instantiates it with
  `F = BaseExpr, EF = ExtExpr`, turning the constraint set into expression
  _data_ — including formal LogUp parameters — without any transcription.

The verifier AIR uses the same seam: its composition-check sub-circuit is driven
by the inner components' `evaluate()` — either executed at witness generation
time with `PointEvaluator`-style QM31 values, or instantiated with an evaluator
whose field type is the verifier AIR's own cell/variable type. Either way, a
macro edit changes the inner AIR and the recursive verifier in the same
compilation, with zero copies.

## What the verifier AIR must assert

Mirroring `prover::verifier::verify_rv32im` + `stwo::core::verifier::verify`,
for each of the two child proofs:

1. **Fiat-Shamir channel replay** (Blake2s): mix public data, commitments,
   claims; draw `Relations`, OODS point, FRI alphas, query positions. Requires a
   Blake2s hash component (stwo has a Blake AIR example as reference).
2. **Proof of work** checks (interaction PoW + FRI PoW).
3. **LogUp sum check**: total claimed sum + public data logup sum = 0. The
   public-data logup terms reuse the same `Relations::combine` code.
4. **Composition check at OODS**: recompute the composition polynomial value
   from the sampled mask values via the inner components' `evaluate()` — the
   single-source seam described above.
5. **Merkle decommitments** (Blake2s) of the queried positions against the
   commitments for every trace tree.
6. **FRI verification**: fold query evaluations through all FRI layers and check
   consistency with the last-layer polynomial.
7. **Boundary chaining** (aggregation logic): `child_0.exit == child_1.entry` on
   `(pc, clock, memory/register state)` public data, and exposure of
   `(child_0.entry, child_1.exit)` as the aggregate's public data.

The output proof must be verifiable by the same verifier AIR (fixed-point proof
shape), so aggregation composes up the tree.

## Milestones

- **M1 — constraints as data (seam validation)**: expose the full stark-v
  constraint system programmatically through `ExprEvaluator`/`PointEvaluator`
  from the existing components; test that composition replay from a real proof's
  sampled values matches the verifier. This validates the no-copy seam the
  verifier AIR builds on.
- **M2 — segmentation**: runner support for stopping at N steps with entry/exit
  boundary public data; prove/verify a 2-segment run on the host (no recursion
  yet). Boundary soundness comes from the existing `registers_state` /
  `memory_access` relations plus Merkle-root chaining:
  `final_rw_root(k) == initial_rw_root(k + 1)`. Chaining works because the
  partial Merkle trees use zero-valued default leaves, so an address first
  written in segment `k + 1` (present in its initial tree as 0) hashes
  identically to its absence from segment `k`'s final tree. IO special-casing is
  gated by `runner::SegmentRole`: inputs are LogUp-anchored in the first segment
  only, public outputs consumed in the last only; middle segments treat the IO
  regions as ordinary RW memory. Constraint: a guest taking input must access
  every input word within the first segment (unconsumed input emissions make
  segment 1's LogUp sum non-zero — verification fails safe, but the run is
  unprovable).
- **M3 — QM31 arithmetic components** (started): verifier-AIR building blocks
  for QM31 mul/inverse, point operations, and FRI folding steps. Lives in
  `crates/recursion`, built on `define_component_tables!` (the trace-table DSL
  without the zkVM `Tracer`), so recursion constraints share the single-source
  pipeline. First component: `qm31_mul` (c = a·b over the extension tower, 4
  degree-2 limb constraints, tested against stwo's field arithmetic).
- **M4 — channel + Merkle components**: hash sub-AIR and decommitment paths;
  channel state replay as a trace. Direction: a Poseidon2-M31 `MerkleChannel` so
  inner proofs commit with the hash the existing `poseidon2`/`merkle` components
  already prove — in-AIR Merkle verification becomes component reuse with zero
  new hash constraints. Crucially this needs **no fork changes**: `Channel`,
  `MerkleChannel`, `MerkleHasherLifted`, `MerkleOps`, and
  `BackendForChannel<MC>` are public stwo traits, and the orphan rules permit
  implementing them all for stark-v-local types
  (`impl BackendForChannel<LocalChannel> for SimdBackend` is a legal
  local-type-parameter impl). The permutation already exists in
  `runner::poseidon2`.
- **M5 — composition-check component**: wire the inner `evaluate()` into the
  verifier AIR (witness side via `PointEvaluator` values; constraint side via
  the generic seam).
- **M6 — full verifier AIR + 2-to-1 aggregation**: assemble 1–7, fixed-point
  proof shape, SDK wiring for the aggregation tree.

## Notes

- stwo's `examples/` contain Blake and Poseidon AIRs to draw on for M4; a
  Poseidon-based channel variant may be cheaper inside the AIR than Blake2s and
  is worth evaluating early (it changes the channel of the _inner_ proofs, a
  config choice, not a constraint copy).
- The remaining single-source gap inside stark-v itself: LogUp relation entries
  are written in both `air.rs` (`add_to_relation!`) and `witness.rs`
  (`combine!`/`write_pair!`). Extending `define_trace_tables!` with a
  `relations:` block closes it; the verifier AIR is unaffected either way since
  it consumes `evaluate()`.
