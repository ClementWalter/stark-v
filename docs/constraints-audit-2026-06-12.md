# Constraint-System Audit — Post-Refactor Soundness Review

- **Date:** 2026-06-12
- **Branch audited:** `feat/felt-air-pow-syntax` @ `64c40eac` ("a leftover
  test"), plus one uncommitted test in `crates/runner/src/poseidon2.rs`
  (`test_default_hashes_match_permutation_chain`).
- **Question:** did the chain of refactors (hand-written AIRs →
  `define_trace_tables!` DSL → macro-generated component modules → felt-to-AIR
  compiler for Poseidon2) drop, weaken, or alter any constraint or LogUp entry?
- **Verdict:** **no soundness hole introduced by the refactoring was found.**
  All three open soundness issues (#123, #124, #125) are obsolete — fixed or
  answered by `e55578ff`. Remaining action items are pre-existing design gaps
  and test-coverage hardening, listed in the [Plan](#plan) section.

## Method

Five independent checks, cross-confirmed:

1. **Test suites as ground truth.** `cargo test -p prover -p stwo-macros` — all
   green (106 prover lib tests including every component's
   `padding_only_trace_interaction_sum_is_zero` LogUp-balance test, the
   `max_mul`/`max_div` full-proof regressions from `e55578ff`,
   `cargo test -p runner poseidon2` fill-vs-reference, and
   `test_recursion_air_rejects_broken_sponge_chaining`). A full prove/verify e2e
   (`test_prove_verify_fibonacci`) was also executed and passed.
2. **Poseidon2 old↔new equivalence** — old hand-written AIR (`5ebee259~1`) vs.
   the current felt-compiled one, verified on the actual `cargo expand` output,
   not just the source.
3. **Felt-to-AIR compiler internals** — full read of
   `crates/stwo-macros/src/air_fns.rs` (~2,170 lines) for soundness holes
   (unconstrained materializations, degree accounting, power-syntax lowering,
   loop unrolling, inline-fn capture, logup batching, eval-vs-witness
   divergence).
4. **Per-component DSL-migration equivalence** — every instruction component's
   constraints and lookups at `6d4eb601~1` (pre-DSL, hand-written) mapped
   item-by-item to the current `define_trace_tables!` declarations in
   `crates/runner/src/trace.rs`.
5. **Historical-fix regression check** — nine soundness fixes from older commits
   re-located in the current tree; **witness/trace/verifier/segment
   consistency** across the refactor range `d15396f3..HEAD`.

Historical baselines were taken from ancestors of the audited branch via
`git show`: `6d4eb601~1` (pre-DSL components), `5ebee259~1` (pre-felt
Poseidon2), `553684a9~1` (pre-Merkle-defaults precompute), and the individual
bug-fix commits. `main` itself was not compared.

## 1. Poseidon2: felt-compiled AIR ≡ old hand-written AIR

**Verdict: EQUIVALENT, in places strictly more committed. No gaps.**

- All 158 round constants (128 external + 14 internal + 16 internal-matrix)
  positionally identical. `m4`, external column-sum matrix, and internal matrix
  match term-for-term.
- T = 16, 8 full rounds (4+4), 14 partial rounds — identical.
- The `**` power syntax lowers `x**5` by square-and-multiply: both squares are
  **committed columns pinned by equality constraints** (not witness-only),
  exactly the old `square(square(x)) * x` scheme.
- Constraint census matches the old AIR exactly: 426 materialized columns each
  pinned by one equality constraint (= old mask count: 8×16×3 full + 14×1×3
  partial) + enabler booleanity = 427, plus `wide`/`io` booleanity and mutual
  exclusion. Total trace width 445 = old width.
- All four LogUp entries map 1:1 — same relations (`poseidon2` arity 16,
  `poseidon2_io` arity 32), same argument order, same multiplicity signs
  (`-(enabler*(1-io))` input consume, `enabler*(1-wide-io)` narrow emit,
  `enabler*wide` wide emit, `enabler*io` atomic pair). Witness denominators
  slice the same tuple ranges; both sides finalize in pairs.
- The central structural guarantee: the same lowered felt program generates both
  the AIR (`air_expr`) and the witness fill (`concrete_expr`), so
  witness/constraint divergence is precluded by construction.
- Caveat: equivalence was proven against the old hand-written reference (which
  the old AIR also targeted), not re-derived from the Poseidon2 paper.

## 2. Felt-to-AIR compiler (`crates/stwo-macros/src/air_fns.rs`)

**Verdict: no exploitable soundness hole found.** Verified sound:

- Every `materialize` emits both the binding constraint `cell - (expr)` and the
  matching witness `FillStep::Expr` (`air_fns.rs:501-505`); CSE returns the
  already-constrained column.
- LogUp: own activation `-enabler`, calls `+enabler`; witness writes one
  singleton fraction per entry in the same order; multiset closes via
  `total.is_zero()` over claimed sums + public terms; signs match the verifier's
  `+inverse(denom)` for public activations.
- No witness-only inverse trick is possible: the felt language has no field
  inverse/division in expressions, so the classic "witness inverts, constraint
  forgets to check" bug class cannot occur.
- Inline fns splice into a fresh local scope with version-suffixed cell names —
  no variable capture across calls.
- Activations are Fiat-Shamir-bound before relations are drawn.
- `^` is rejected at parse time (only `+ - * **` supported); `**` degree
  accounting verified by hand for odd and even exponents.

Flagged for follow-up (no exploit constructed, but load-bearing or untested):

| #   | Class      | Finding                                                                                                                                                                                                                                                                                                                         |
| --- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C1  | SUSPICIOUS | `materialize` CSE is keyed on the lowered expr's **token-string** (`air_fns.rs:492-507`). Dedup correctness rests on `parse_quote!` token-string stability. No unsound collision found; deserves a targeted test.                                                                                                               |
| C2  | SUSPICIOUS | Standalone mode does **not** materialize return cells (`air_fns.rs:1149-1157`); a degree-2 return flows straight into the LogUp denominator. Sound only because `io_budget = max_degree - 1` — load-bearing and only indirectly tested.                                                                                         |
| C3  | SUSPICIOUS | `assert` lowering discards the returned degree (`air_fns.rs:952-958`); correctness relies on `lower` enforcing its budget internally. Fragile to future changes.                                                                                                                                                                |
| C4  | TEST GAP   | No `max_degree: 2` coverage anywhere — the budget-1 materialization path and the "cannot reduce product below degree 2" error path are untested.                                                                                                                                                                                |
| C5  | TEST GAP   | **Embedded mode entirely untested** (`generate_embedded`/`generate_embedded_fill`, `air_fns.rs:1619-1709`): flag-column ordering, ret materialization, host pairing. The generated code also gates with its own enabler although the doc comment says gating is the host's business — a host that also gates would double-gate. |
| C6  | TEST GAP   | No negative test forging an **internal materialized column** (both cheating tests forge a public output).                                                                                                                                                                                                                       |
| C7  | TEST GAP   | Loop-carried reassignment/`update` aliasing only covered transitively through the Poseidon2 equivalence tests.                                                                                                                                                                                                                  |

## 3. DSL migration of the RISC-V instruction components

**Verdict: all components VERIFIED-EQUIVALENT or strictly stronger.**
Per-component mapping of the hand-written constraints/lookups at `6d4eb601~1` to
today's `trace.rs`:

| Component                                     | Verdict                                                                                     |
| --------------------------------------------- | ------------------------------------------------------------------------------------------- |
| auipc, jal, jalr, lui                         | equivalent                                                                                  |
| lt_reg, lt_imm                                | equivalent                                                                                  |
| branch_eq, branch_lt                          | equivalent                                                                                  |
| base_alu_reg                                  | equivalent + `e55578ff` rd byte checks (pre-migration gap)                                  |
| base_alu_imm                                  | equivalent                                                                                  |
| shifts_reg, shifts_imm                        | equivalent + `e55578ff` carry bounds & `shift_check` (pre-migration weaker)                 |
| mul, mulh                                     | equivalent + `e55578ff` RC_8_11 carries (pre-migration mul was unprovable for max operands) |
| div                                           | equivalent + `e55578ff` schoolbook identity, sign checks, ungated scan, `batch: 1`          |
| load_store                                    | equivalent (address-space selectors, alignment RC_20, M31 base, byte/half/word selection)   |
| program, memory, merkle, mem/reg_clock_update | equivalent                                                                                  |

Macro facts that make the DSL a faithful single source: booleanity of `enabler`
and every opcode flag is auto-emitted; lookup multiplicity signs are parsed
verbatim into `RelationEntry::new`; `batch: 2` → `finalize_logup_in_pairs()`,
`batch: 1` → `finalize_logup()`; relation definitions byte-identical pre/post;
no component dropped from `components!`.

One **latent (dormant) macro inconsistency**: for `batch ≥ 3` the AIR side
computes per-entry batch assignments (`entry/batch`) while the witness side
always pairs fractions when `batch >= 2` (`trace_tables.rs:1056-1129`) — these
disagree for batch ≥ 3. No table uses batch ≥ 3, so it is untriggered today. See
plan item P2.

## 4. Historical soundness fixes — regression check

All nine verified **STILL-FIXED** at HEAD; two are now structurally impossible
to regress because the DSL derives the AIR and witness sides from one source:

| Fix                                                                | Status / where it lives now                                                                                                            |
| ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| `48dcc360` rw memory access relations                              | IO region + word-aligned dedup in `commitment.rs:74-97,286-292`; half-word shift in `ops/{load,store}.rs`; AIR side `trace.rs:958-959` |
| `4c41123b` enabler in range checks                                 | every RC lookup gated by `-enabler` or a boolean derived from it; no bare-constant multipliers remain                                  |
| `4e6eada1` memory mult registration                                | structurally enforced — macro generates `register_multiplicities` for every table (`components.rs:486-494`)                            |
| `5e27c8b4` bitwise_id                                              | `2*xor + or` (`trace.rs:35,113`) matches `preprocessed/bitwise.rs:55-66`                                                               |
| `882400af` multiplicity-negation unification                       | single-source: one signed numerator per `LookupEntry` read by both paths (`trace_tables.rs:962,1192,1221`)                             |
| relation tracker (`5d2e8721`, `6496e027`)                          | generalized into the macro (`components.rs:695-721`), wired under `track-relations` (`prover.rs:173-184`), compiles                    |
| `e55578ff` constraints-audit divergences (5 fixes)                 | all present; `max_mul`/`max_div` full-proof regressions pass                                                                           |
| `0795a0f0` per-fraction batch assignments                          | intact; witness pairing coincides with AIR grouping for the only used batch sizes {1, 2}                                               |
| `90789037`/`d7bfd4dd` Poseidon2 atomic (input,output) pair + flags | survives felt compilation; recursion `channel_replay` depends on it; rejection test passes                                             |

Preprocessed-table `index()` functions all match their DSL tuple shapes and limb
widths (RC_8_8, RC_8_11 incl. the 11-bit mask for the carry-509 case, RC_8_8_4,
RC_M31, bitwise).

## 5. Witness/trace, verifier, and segment plumbing

- **Padding soundness — OK.** Constraints are raw expressions over every row;
  all verified to vanish on all-zero padding rows (flag gating, booleanity on
  zero columns, and the deliberately ungated div comparison scan / load_store
  selector identities each reduce to 0). Fill formulas cannot diverge from
  constraint expressions — both are compiled once from the same DSL expression.
- **Merkle defaults precompute (`553684a9`) — OK, behavior-identical.** The
  constant table reproduces the old dynamic recursion byte-for-byte (proven by
  the new — currently uncommitted —
  `test_default_hashes_match_permutation_chain`); indexing and the empty-subtree
  multiplicity-0 convention unchanged.
- **Verifier — no check disappeared** across the refactor range: public-data
  mixing, pinned preprocessed log-sizes, PoW, relation draws, **LogUp total sum
  == 0 including `public_data.logup_sum`**, preprocessed-root pinning, stwo
  `verify`. `public_data.rs` diffs only add checks.
- **Parallel segment proving (`6886a40d`) — OK**: only the prover iterator
  changed; no cross-segment binding runs in the prover.
- **Segment chaining — adjacent-boundary check intact** in all three
  implementations (`segments.rs:80-99`, `aggregate.rs:48-70`,
  `final_proof.rs:208-219`): `exit_pc==entry_pc`, registers, rw root, program
  root.

**Design-level gaps (pre-existing, not refactor regressions):**

| #   | Finding                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1  | **Segment-end roles are prover-chosen and unverified.** `SegmentRole{is_first,is_last}` decides whether IO is LogUp-anchored, but no role marker is bound into `public_data` or the channel, and `verify_segments`/`verify_final` never assert that proof 0 was proven `is_first=true` and the last `is_last=true`. A chain whose ends were proven as middle segments would silently skip input/output binding. |
| S2  | **No statement binding at the top.** `verify_segments` returns `Result<()>` and binds nothing external; `verify_final`/`aggregate_segments` return the `Boundary` but no in-tree caller compares it (program root, input, output) to an expected statement.                                                                                                                                                     |
| S3  | Empty-memory boundary is vacuous (`None == None` on `Option<u32>` rw roots) — acceptable, noted for completeness.                                                                                                                                                                                                                                                                                               |

## 6. Open soundness issues #123 / #124 / #125

All three reference deleted pre-DSL files and are obsolete:

- **#123** (shifts_imm `bit_shift_carry` never range-checked) — **fixed** by
  `e55578ff`: `range_check_8_8(bit_multiplier - enabler - bit_shift_carry_i, …)`
  at `trace.rs:305-310` (shifts_reg) and `trace.rs:454-459` (shifts_imm).
- **#124** (div carry recurrence computed but never enforced) — **fixed** by
  `e55578ff`: the 8-limb schoolbook carry chain for `rs1 = rs2·q + r` is derived
  (hence equality-constrained) at `trace.rs:1220-1234` and every carry consumed
  by `range_check_8_11(q_i/r_i, carry_k)` at `trace.rs:1334-1341` — exactly the
  RC_8_11 scheme the old TODO described.
- **#125** (RC_20 on `lt_diff` looser than the limb bound) — **sound as
  written.** `lt_diff` is not a free value bounded only by RC_20: it is
  equality-pinned to the marked limb difference by
  `lt_marker_k * (lt_diff - diff_k)` (`trace.rs:1306-1313`), with
  `enabler*(1 - prefix_0)` forcing a marker on every non-special row. Both sides
  of the difference are byte-bounded (q/r limbs via RC_8_11; `r_abs` limbs
  forced into `[0,255]` by the carry-boolean + `r_inv` constraints at
  `trace.rs:1287-1301`), so the true difference lies in `[-255, 255]`. A
  negative difference maps to ≈2³¹ in M31, far outside `[1, 2²⁰+1)` — RC_20's
  only job here is excluding non-positive differences, which it does.

## Plan

No soundness fixes are required for the refactor itself. Prioritized hardening
plan:

### P1 — Segment-end statement binding (the one real design gap; S1/S2)

1. Add the segment role (or equivalently the IO-anchoring mode) to `public_data`
   and mix it into the Fiat-Shamir channel so it is part of the proven
   statement.
2. In `verify_segments` / `verify_final`, assert proof 0 has `is_first=true`,
   the last proof has `is_last=true`, and middles have neither.
3. Add a top-level statement-binding API: `verify(program_root, input, output)`
   that checks the returned `Boundary`/public IO against the caller's expected
   statement, so soundness no longer depends on an out-of-tree caller inspecting
   `public_data` correctly.

### P2 — Macro foot-gun: batch size guard

Reject `batch ∉ {1, 2}` at `define_trace_tables!` parse time (or make the
witness pairing follow the AIR's `entry/batch` assignment). Today the two sides
silently disagree for `batch ≥ 3`; the guard turns a future LogUp-completeness
break into a compile error.

### P3 — Felt-compiler test hardening (C1–C7)

1. Embedded-mode tests: flag-column ordering, ret materialization, host pairing
   — and resolve the enabler double-gating question (either the embedded code
   stops emitting its own enabler gate or the doc comment changes).
2. A `max_degree: 2` test exercising budget-1 materialization and the "cannot
   reduce product below degree 2" error.
3. A negative test that mutates an internal materialized column (e.g. an s-box
   square) and asserts verification fails.
4. An isolated loop-reassignment/array-`update` test (slot read-after-update
   resolves to the new cell; siblings keep old cells).
5. A CSE test: two distinct-but-equal subexpressions must not collapse into one
   column incorrectly, and the materialized constraint always pins the value
   downstream consumers read.

### P4 — Housekeeping

1. Close issues #123 and #124 citing `e55578ff` and the `trace.rs` lines above;
   close #125 with the bound argument above.
2. Commit the currently uncommitted
   `test_default_hashes_match_permutation_chain` — it is the proof that the
   precomputed Merkle defaults match the permutation chain.
