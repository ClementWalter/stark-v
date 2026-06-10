//! Operation recorder: compiles an inner AIR's `evaluate()` into an
//! arithmetic-operation arena (docs/recursion.md, M5).
//!
//! `Recorder` implements `EvalAtRow` with a handle field type, so running any
//! component's `FrameworkEval::evaluate` — the same single-source code the
//! prover and verifier execute — records the composition-polynomial
//! computation as explicit QM31 operations over mask inputs. The arena is the
//! circuit the composition-check component lowers into recursion-AIR rows
//! (mul/inv via the existing components, wiring via relations); an edit to
//! `define_trace_tables!` changes the recorded circuit in the same
//! compilation, with no constraint copy.

use core::cell::RefCell;
use core::ops::{Add, AddAssign, Mul, Neg, Sub};
use std::rc::Rc;

use num_traits::{One, Zero};
use stwo::core::fields::FieldExpOps;
use stwo::core::fields::m31::BaseField;
use stwo::core::fields::qm31::{SECURE_EXTENSION_DEGREE, SecureField};
use stwo_constraint_framework::logup::LogupAtRow;
use stwo_constraint_framework::{EvalAtRow, INTERACTION_TRACE_IDX};

/// One recorded operation over arena nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// A mask input (interaction, column, offset slot).
    Input,
    /// A constant embedded in the constraint system.
    Const,
    Add(usize, usize),
    Sub(usize, usize),
    Mul(usize, usize),
    Neg(usize),
    Inverse(usize),
}

/// A recorded node: the operation and its evaluated value.
#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub op: Op,
    pub value: SecureField,
}

/// The growing list of operations.
#[derive(Default, Debug)]
pub struct Arena {
    pub nodes: Vec<Node>,
}

impl Arena {
    fn push(&mut self, op: Op, value: SecureField) -> usize {
        self.nodes.push(Node { op, value });
        self.nodes.len() - 1
    }
}

type SharedArena = Rc<RefCell<Arena>>;

/// A value handle: either a pure constant or a node in the shared arena.
///
/// Constants fold eagerly; any operation touching a node records a new node.
#[derive(Clone, Debug)]
pub enum Rec {
    Const(SecureField),
    Node {
        id: usize,
        value: SecureField,
        arena: SharedArena,
    },
}

impl Rec {
    pub fn value(&self) -> SecureField {
        match self {
            Rec::Const(value) => *value,
            Rec::Node { value, .. } => *value,
        }
    }

    /// Materialize this handle as an arena node (constants become Const nodes).
    fn node_id(&self, arena: &SharedArena) -> usize {
        match self {
            Rec::Const(value) => arena.borrow_mut().push(Op::Const, *value),
            Rec::Node { id, .. } => *id,
        }
    }

    fn arena(&self) -> Option<&SharedArena> {
        match self {
            Rec::Const(_) => None,
            Rec::Node { arena, .. } => Some(arena),
        }
    }

    fn binary(
        lhs: &Rec,
        rhs: &Rec,
        fold: impl Fn(SecureField, SecureField) -> SecureField,
        op: impl Fn(usize, usize) -> Op,
    ) -> Rec {
        let value = fold(lhs.value(), rhs.value());
        let arena = match lhs.arena().or_else(|| rhs.arena()) {
            // Both constants: fold without recording.
            None => return Rec::Const(value),
            Some(arena) => arena.clone(),
        };
        let lhs_id = lhs.node_id(&arena);
        let rhs_id = rhs.node_id(&arena);
        let id = arena.borrow_mut().push(op(lhs_id, rhs_id), value);
        Rec::Node { id, value, arena }
    }
}

impl PartialEq for Rec {
    fn eq(&self, other: &Self) -> bool {
        self.value() == other.value()
    }
}

impl From<BaseField> for Rec {
    fn from(value: BaseField) -> Self {
        Rec::Const(value.into())
    }
}

impl From<SecureField> for Rec {
    fn from(value: SecureField) -> Self {
        Rec::Const(value)
    }
}

impl Add for Rec {
    type Output = Rec;
    fn add(self, rhs: Rec) -> Rec {
        Rec::binary(&self, &rhs, |a, b| a + b, Op::Add)
    }
}

impl Sub for Rec {
    type Output = Rec;
    fn sub(self, rhs: Rec) -> Rec {
        Rec::binary(&self, &rhs, |a, b| a - b, Op::Sub)
    }
}

impl Mul for Rec {
    type Output = Rec;
    fn mul(self, rhs: Rec) -> Rec {
        Rec::binary(&self, &rhs, |a, b| a * b, Op::Mul)
    }
}

impl AddAssign for Rec {
    fn add_assign(&mut self, rhs: Rec) {
        *self = self.clone() + rhs;
    }
}

impl AddAssign<BaseField> for Rec {
    fn add_assign(&mut self, rhs: BaseField) {
        *self = self.clone() + Rec::from(rhs);
    }
}

impl Mul<BaseField> for Rec {
    type Output = Rec;
    fn mul(self, rhs: BaseField) -> Rec {
        self * Rec::from(rhs)
    }
}

impl Mul<SecureField> for Rec {
    type Output = Rec;
    fn mul(self, rhs: SecureField) -> Rec {
        self * Rec::from(rhs)
    }
}

impl Add<SecureField> for Rec {
    type Output = Rec;
    fn add(self, rhs: SecureField) -> Rec {
        self + Rec::from(rhs)
    }
}

impl Add<BaseField> for Rec {
    type Output = Rec;
    fn add(self, rhs: BaseField) -> Rec {
        self + Rec::from(rhs)
    }
}

impl Sub<SecureField> for Rec {
    type Output = Rec;
    fn sub(self, rhs: SecureField) -> Rec {
        self - Rec::from(rhs)
    }
}

impl Neg for Rec {
    type Output = Rec;
    fn neg(self) -> Rec {
        match self {
            Rec::Const(value) => Rec::Const(-value),
            Rec::Node { id, value, arena } => {
                let id = arena.borrow_mut().push(Op::Neg(id), -value);
                Rec::Node {
                    id,
                    value: -value,
                    arena,
                }
            }
        }
    }
}

impl Zero for Rec {
    fn zero() -> Self {
        Rec::Const(SecureField::zero())
    }
    fn is_zero(&self) -> bool {
        self.value().is_zero()
    }
}

impl One for Rec {
    fn one() -> Self {
        Rec::Const(SecureField::one())
    }
}

impl core::ops::MulAssign for Rec {
    fn mul_assign(&mut self, rhs: Rec) {
        *self = self.clone() * rhs;
    }
}

impl FieldExpOps for Rec {
    fn inverse(&self) -> Self {
        match self {
            Rec::Const(value) => Rec::Const(value.inverse()),
            Rec::Node { id, value, arena } => {
                let inv = value.inverse();
                let new_id = arena.borrow_mut().push(Op::Inverse(*id), inv);
                Rec::Node {
                    id: new_id,
                    value: inv,
                    arena: arena.clone(),
                }
            }
        }
    }
}

/// Records an inner AIR's composition evaluation over given mask values.
pub struct Recorder {
    pub arena: SharedArena,
    /// Mask values per interaction per column (offsets flattened in order).
    pub mask: Vec<Vec<Vec<SecureField>>>,
    col_index: Vec<usize>,
    /// alpha (the constraint-combination coefficient) as a recorded input.
    alpha: Rec,
    /// 1 / V(oods_point) as a recorded input.
    denom_inverse: Rec,
    /// Accumulated combination: acc = acc * alpha + denom_inverse * constraint.
    pub accumulation: Rec,
    pub logup: LogupAtRow<Self>,
}

impl Recorder {
    pub fn new(
        mask: Vec<Vec<Vec<SecureField>>>,
        alpha: SecureField,
        denom_inverse: SecureField,
        log_size: u32,
        claimed_sum: SecureField,
    ) -> Self {
        let arena: SharedArena = Rc::new(RefCell::new(Arena::default()));
        let input = |value: SecureField, arena: &SharedArena| {
            let id = arena.borrow_mut().push(Op::Input, value);
            Rec::Node {
                id,
                value,
                arena: arena.clone(),
            }
        };
        let alpha = input(alpha, &arena);
        let denom_inverse = input(denom_inverse, &arena);
        let col_index = vec![0; mask.len()];
        Self {
            arena,
            mask,
            col_index,
            alpha,
            denom_inverse,
            accumulation: Rec::Const(SecureField::zero()),
            logup: LogupAtRow::new(INTERACTION_TRACE_IDX, claimed_sum, log_size),
        }
    }

    fn input(&self, value: SecureField) -> Rec {
        let id = self.arena.borrow_mut().push(Op::Input, value);
        Rec::Node {
            id,
            value,
            arena: self.arena.clone(),
        }
    }
}

impl EvalAtRow for Recorder {
    type F = Rec;
    type EF = Rec;

    fn next_interaction_mask<const N: usize>(
        &mut self,
        interaction: usize,
        _offsets: [isize; N],
    ) -> [Self::F; N] {
        let col_index = self.col_index[interaction];
        self.col_index[interaction] += 1;
        let values = &self.mask[interaction][col_index];
        assert_eq!(values.len(), N);
        std::array::from_fn(|i| self.input(values[i]))
    }

    fn add_constraint<G>(&mut self, constraint: G)
    where
        Self::EF: Mul<G, Output = Self::EF>,
    {
        let weighted = self.denom_inverse.clone() * constraint;
        let scaled = Rec::binary(&self.accumulation, &self.alpha, |a, b| a * b, Op::Mul);
        self.accumulation = scaled + weighted;
    }

    fn combine_ef(values: [Self::F; SECURE_EXTENSION_DEGREE]) -> Self::EF {
        // (v0 + i v1) + (v2 + i v3) u over the secure field basis constants.
        let [v0, v1, v2, v3] = values;
        let u_0 = SecureField::from_m31_array([0.into(), 1.into(), 0.into(), 0.into()]);
        let u_1 = SecureField::from_m31_array([0.into(), 0.into(), 1.into(), 0.into()]);
        let u_2 = SecureField::from_m31_array([0.into(), 0.into(), 0.into(), 1.into()]);
        v0 + v1 * u_0 + v2 * u_1 + v3 * u_2
    }

    fn write_logup_frac(&mut self, fraction: stwo::core::Fraction<Self::EF, Self::EF>) {
        if self.logup.fracs.is_empty() {
            self.logup.is_finalized = false;
        }
        self.logup.fracs.push(fraction);
    }

    /// Same batching semantics as the framework's `logup_proxy!` (which is
    /// crate-private): consecutive groups of `batch_size`, cumulative-sum
    /// columns from the interaction masks, the shifted check on the last.
    fn finalize_logup_batched(&mut self, batch_size: usize) {
        assert!(!self.logup.is_finalized, "LogupAtRow was already finalized");
        assert!(batch_size > 0, "Batch size must be positive");

        let fracs = core::mem::take(&mut self.logup.fracs);
        let n_batches = fracs.len().div_ceil(batch_size);
        assert!(n_batches > 0, "No fractions to finalize");

        let mut prev_col_cumsum = <Self::EF as Zero>::zero();
        for (batch_idx, chunk) in fracs.chunks(batch_size).enumerate() {
            let cur_frac: stwo::core::Fraction<Self::EF, Self::EF> = chunk.iter().cloned().sum();
            if batch_idx + 1 < n_batches {
                let [cur_cumsum] =
                    self.next_extension_interaction_mask(self.logup.interaction, [0]);
                let diff = cur_cumsum.clone() - prev_col_cumsum.clone();
                prev_col_cumsum = cur_cumsum;
                self.add_constraint(diff * cur_frac.denominator - cur_frac.numerator);
            } else {
                let [prev_row_cumsum, cur_cumsum] =
                    self.next_extension_interaction_mask(self.logup.interaction, [-1, 0]);
                let diff = cur_cumsum - prev_row_cumsum - prev_col_cumsum.clone();
                let shifted_diff = diff + self.logup.cumsum_shift;
                self.add_constraint(shifted_diff * cur_frac.denominator - cur_frac.numerator);
            }
        }
        self.logup.is_finalized = true;
    }

    fn finalize_logup(&mut self) {
        self.finalize_logup_batched(1)
    }

    fn finalize_logup_in_pairs(&mut self) {
        self.finalize_logup_batched(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::{Rng, SeedableRng};
    use stwo::core::air::accumulation::PointEvaluationAccumulator;
    use stwo::core::pcs::TreeVec;
    use stwo_constraint_framework::{FrameworkEval, InfoEvaluator, PointEvaluator};

    fn random_secure(rng: &mut SmallRng) -> SecureField {
        SecureField::from_m31_array(std::array::from_fn(|_| {
            BaseField::from_u32_unchecked(rng.gen_range(0..(1 << 30)))
        }))
    }

    /// Record an inner component and check the accumulated value matches
    /// stwo's own PointEvaluator over the same masks — i.e. the recorded
    /// circuit faithfully computes the inner composition.
    #[test]
    fn test_recorder_matches_point_evaluator_on_lui() {
        let mut rng = SmallRng::seed_from_u64(0);
        let eval = prover::components::opcodes::lui::air::Eval {
            log_size: 6,
            relations: prover::relations::Relations::dummy(),
        };

        // Mask shape from the single-source InfoEvaluator.
        let info = eval.evaluate(InfoEvaluator::empty());
        let mask_values: Vec<Vec<Vec<SecureField>>> = info
            .mask_offsets
            .iter()
            .map(|interaction| {
                interaction
                    .iter()
                    .map(|offsets| {
                        (0..offsets.len())
                            .map(|_| random_secure(&mut rng))
                            .collect()
                    })
                    .collect()
            })
            .collect();

        let alpha = random_secure(&mut rng);
        let denom_inverse = random_secure(&mut rng);
        let claimed_sum = random_secure(&mut rng);
        let log_size = 6;

        // Ground truth: stwo's point evaluation.
        let mut accumulator = PointEvaluationAccumulator::new(alpha);
        let mask_refs: TreeVec<Vec<&Vec<SecureField>>> =
            TreeVec::new(mask_values.iter().map(|t| t.iter().collect()).collect());
        let point_eval = PointEvaluator::new(
            mask_refs,
            &mut accumulator,
            denom_inverse,
            log_size,
            claimed_sum,
        );
        eval.evaluate(point_eval);
        let expected = accumulator.finalize();

        // Recorded circuit.
        let recorder = Recorder::new(mask_values, alpha, denom_inverse, log_size, claimed_sum);
        let recorder = eval.evaluate(recorder);
        assert_eq!(recorder.accumulation.value(), expected);
    }

    #[test]
    fn test_recorder_arena_contains_operations() {
        let eval = prover::components::opcodes::lui::air::Eval {
            log_size: 4,
            relations: prover::relations::Relations::dummy(),
        };
        let info = eval.evaluate(InfoEvaluator::empty());
        let mut rng = SmallRng::seed_from_u64(1);
        let mask_values: Vec<Vec<Vec<SecureField>>> = info
            .mask_offsets
            .iter()
            .map(|interaction| {
                interaction
                    .iter()
                    .map(|offsets| {
                        (0..offsets.len())
                            .map(|_| random_secure(&mut rng))
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let recorder = Recorder::new(
            mask_values,
            random_secure(&mut rng),
            random_secure(&mut rng),
            4,
            random_secure(&mut rng),
        );
        let recorder = eval.evaluate(recorder);
        let arena = recorder.arena.borrow();
        // The circuit has inputs and arithmetic: a faithful compilation of
        // the inner constraints, produced by running the same evaluate().
        assert!(arena.nodes.iter().any(|n| matches!(n.op, Op::Mul(_, _))));
    }
}
