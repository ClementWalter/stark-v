# A felt language that compiles to AIR (design)

## The observation

Write-once (single-assignment) memory with Cairo-style call frames is already a
circuit description. Each frame only references values in the window
`[fp - arg_size, fp + frame_size)`; every cell is written exactly once as a felt
expression of earlier cells. That is precisely a row of an AIR table: the
frame's cells are the columns, the write expressions are the wires, and a
"function" is a reusable sub-circuit. The Cairo VM's memory model is not an
execution detail — it is the reason Cairo programs _define_ AIRs instead of
merely running on one.

So instead of writing components as flattened column lists plus hand-named
intermediates, we should write them as straight-line felt code and compile to
the AIR, with the **maximum constraint degree as a compiler parameter**.

## What the compiler does

The program is felt-valued single-assignment code over the table's input
columns. The compiler builds the expression DAG and decides, per node, whether
it stays **inline** (a derived expression, free) or is **materialized** (a
committed trace column plus one equality constraint):

- Multiplication compounds degree: `x * y * z` at `max_degree = 2` splits into
  `t = x * y` (materialized, constraint `t - x * y`) and the inline `t * z`.
- Addition does not: `a + b + c + d` stays one inline expression no matter its
  length — it never increases the degree.
- Reuse counts: a degree-2 subexpression used by ten constraints may be cheaper
  materialized once than inlined ten times into the composition evaluation; the
  compiler weighs (columns added) against (constraint degree and evaluation
  cost).

The output is exactly what `define_trace_tables!` consumes today: a column list
(inputs + materialized intermediates), `derived:` (the inline nodes),
`constraints:` (the materialization equalities plus the program's asserted
zeros), and `lookups:` (relation calls in the code become LogUp entries).

The same program is the witness generator: run it with concrete `PackedM31`
values and every materialized node _is_ the column fill, in the same order. This
is the existing `T`-generic trick (one expression, evaluated with `E::F` for the
AIR and `PackedM31` via `at(i)` for the witness) taken to its conclusion — the
macro already proves the architecture works; the compiler adds control flow and
automatic materialization on top.

## Poseidon2, the motivating case

Today the poseidon2 table is ~700 hand-flattened columns
(`full0_sq1_0..15, full0_sq2_0..15, full0_mix_0..15, full1_...`) and the one
remaining hand-written component module, because the expression DSL cannot loop.
As felt code it is:

```text
fn poseidon2(state: [felt; 16]) -> [felt; 16] {
    for round in 0..8 {                       // static bound: unrolled
        state = add_round_constants(state, EXTERNAL[round]);
        for i in 0..16 { state[i] = sbox(state[i]); }   // x^5
        state = external_mix(state);          // additive: stays inline
    }
    // ... partial rounds ...
    state
}

fn sbox(x: felt) -> felt {
    let x2 = x * x;      // materialized at max_degree = 2 (or 3)
    let x4 = x2 * x2;    // materialized
    x4 * x              // inline
}
```

At `max_degree = 3` the compiler materializes two cells per s-box instead of
today's hand-chosen three (`sq1`, `sq2`, `mix`), derives the column count, the
constraints, and the witness fill — and changing the degree budget re-derives
all three. The flattened table becomes generated output, not source.

## Control flow: the calling convention is a LogUp relation

The Cairo frame layout completes the model. A call frame receives its inputs at
`[fp - n, fp - 3)` and leaves its outputs at the final `[ap - m, ap - 1)`; the
two remaining slots — `fp - 2` (saved fp) and `fp - 1` (return pc) — are pure
control-flow plumbing. In the AIR view those two slots disappear entirely: LogUp
replaces sequencing. What remains per activation is exactly one natural tuple,
`(inputs..., outputs...)`, and that tuple **is** the function's relation.

- Each function is an AIR table; each activation (call) is one row.
- A row starts by **consuming** its own activation tuple
  (`-enabler * fn_io(args..., rets...)`) and its constraints enforce
  `rets = body(args)`.
- A caller **emits** the tuple for every call it makes
  (`+enabler * callee_io(call_args..., call_rets...)`) — the returned values are
  witness columns in the caller's frame, received through the relation, and the
  callee's constraints are what make them right.
- A recursive call is the same emission against the function's own relation:
  rows of one AIR consuming and emitting each other, telescoping exactly like
  the recursion crate's `merkle_node` paths and `sponge_step` chains.
- The program's public interface is the entry activations: the verifier emits
  `+fn_io(inputs, outputs)` as public claim terms (the `RootClaim` pattern), and
  the whole multiset must cancel.

Purity makes the unkeyed tuple sound: a function is a relation in the
mathematical sense, so two activations with the same inputs have the same
outputs and collapse into multiplicity — no call-site nonce needed.

The codebase already runs on this pattern without naming it: the opcode tables
are "functions" consuming `program_access` and `memory_access` tuples;
`poseidon2_io(in16, out16)` is precisely an activation tuple; the recursion
circuit's `op_def`/`wire` relations are call frames for QM31 arithmetic. The
language makes the pattern first-class: `let c = cube(a)` in source compiles to
a column `c`, an emission into `cube`'s relation, and a row in `cube`'s table —
wiring, table layout, and witness fill all from one line.

## Relation to the current DSL (incremental path)

The expression DSL already has: single-assignment named intermediates
(`derived:`), expansion-time constant folding (`pow2`, `inv`, integer subtrees),
spec-shaped lookups, and the dual AIR/witness evaluation. What it lacks is
exactly what the compiler adds:

1. **Degree-budget materialization** (smallest step, immediately useful): today
   a derived column that would breach the bound must be manually split into a
   real trace column (the div carry chain stayed at degree 2 only by careful
   hand-shaping, and two pre-existing degree-4 groups in div shipped unnoticed
   until the first real-row proof — see commit e55578ff). Let the macro compute
   each expression's degree and either reject with "materialize this" or
   auto-materialize. Auto-materialization changes the table layout, so the macro
   must also emit the fill — which it can, since the fill is the same expression
   evaluated concretely.
2. **Static control flow**: `for` with constant bounds (unroll), `if` on
   compile-time flags (select). This is enough to absorb poseidon2 and delete
   the last hand-written component.
3. **Functions/frames**: reusable sub-circuits with the Cairo frame rule — a
   callee reads only its arguments, writes only its frame. At this point the
   language is a real (if minimal) felt language, and per the opening
   observation it could in principle be _executed_ on a write-once-memory VM as
   well as compiled to the AIR.

Step 1 hardens soundness today; step 2 removes the poseidon2 exception; step 3
is the full language. Each step keeps the single-source invariant: AIR, witness,
and (through the recursion recorder) the final proof all derive from the same
definition.

## Open questions

- **Materialization vs masks**: stwo also allows referencing neighboring rows
  (masks). A frame that reads its caller's cells maps naturally to a mask
  offset; deciding when a value crosses rows vs stays in-row is a layout
  question the compiler eventually owns.
- **Lookup placement**: relation calls inside loops/functions multiply entries;
  the batching parameter (`batch:`) should become a per-entry degree decision
  the compiler makes (quadratic denominators → singleton), not a table-level
  annotation.
- **Cost model**: columns are committed (Merkle + FRI cost per column); inline
  expressions cost composition-evaluation work. The right objective is prover
  time, with max_degree as the hard constraint.
