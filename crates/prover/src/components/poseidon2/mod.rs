//! Poseidon2 component: the permutation constraints come from the felt
//! definition in `runner::poseidon2` (its straight-line `evaluation()`
//! yields the round constraints and the `(input, output)` activation
//! tuple); this module only wires the emission modes into the zkVM
//! relations:
//! - `wide = io = 0`: memory commitment trees — consume the 16-word input,
//!   emit the 1-word digest,
//! - `wide`: proof commitment trees — emit the 8-word digest instead,
//! - `io`: sponge chaining — emit the atomic (input, output) pair through
//!   `poseidon2_io` (the input is bound there, not consumed separately).

pub mod air {
    use num_traits::One;
    use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval, RelationEntry};

    use crate::relations::Relations;
    use runner::trace::prover_columns::Poseidon2Columns;

    pub type Component = FrameworkComponent<Eval>;

    #[derive(Clone)]
    pub struct Eval {
        pub log_size: u32,
        pub relations: Relations,
    }

    impl FrameworkEval for Eval {
        fn log_size(&self) -> u32 {
            self.log_size
        }

        fn max_constraint_log_degree_bound(&self) -> u32 {
            self.log_size + 1
        }

        fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
            let cols = Poseidon2Columns::from_eval(&mut eval);
            let (constraints, entries) = cols.evaluation();
            for constraint in constraints {
                eval.add_constraint(constraint);
            }

            // Emission shape flags: booleans, mutually exclusive.
            let one = E::F::one();
            let enabler = cols.enabler.clone();
            let wide = cols.wide.clone();
            let io = cols.io.clone();
            eval.add_constraint(wide.clone() * (one.clone() - wide.clone()));
            eval.add_constraint(io.clone() * (one.clone() - io.clone()));
            eval.add_constraint(wide.clone() * io.clone());

            let (_, tuple) = entries
                .into_iter()
                .next()
                .expect("the felt function has one activation tuple");
            let (input, output) = tuple.split_at(16);

            // io rows bind their input through the atomic pair below.
            eval.add_to_relation(RelationEntry::new(
                &self.relations.poseidon2,
                (-(enabler.clone() * (one.clone() - io.clone()))).into(),
                input,
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.poseidon2,
                (enabler.clone() * (one - wide.clone() - io.clone())).into(),
                &output[..1],
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.poseidon2,
                (enabler.clone() * wide).into(),
                &output[..8],
            ));
            eval.add_to_relation(RelationEntry::new(
                &self.relations.poseidon2_io,
                (enabler * io).into(),
                &tuple,
            ));
            eval.finalize_logup_in_pairs();
            eval
        }
    }
}

pub mod witness {
    use num_traits::{One, Zero};
    use stwo::core::ColumnVec;
    use stwo::core::fields::m31::BaseField;
    use stwo::core::fields::qm31::QM31;
    use stwo::prover::backend::simd::SimdBackend;
    use stwo::prover::backend::simd::m31::PackedM31;
    use stwo::prover::backend::simd::qm31::PackedQM31;
    use stwo::prover::poly::BitReversedOrder;
    use stwo::prover::poly::circle::CircleEvaluation;
    use stwo_constraint_framework::{LogupTraceGenerator, Relation};

    use crate::relations::Relations;
    use runner::trace::prover_columns::Poseidon2Columns;

    /// The same four entries as the AIR, paired in the same order.
    pub fn gen_interaction_trace(
        trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
        relations: &Relations,
    ) -> (
        ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        QM31,
    ) {
        if trace.is_empty() {
            return (vec![], QM31::zero());
        }
        let cols = Poseidon2Columns::from_iter(trace.iter().map(|eval| &eval.values.data));
        let simd_size = cols.enabler.len();
        let log_size = trace[0].domain.log_size();
        let mut logup_gen = LogupTraceGenerator::new(log_size);

        let one = PackedM31::one();
        let mut consume_input = Vec::with_capacity(simd_size);
        let mut input_denoms: Vec<PackedQM31> = Vec::with_capacity(simd_size);
        let mut emit_narrow = Vec::with_capacity(simd_size);
        let mut narrow_denoms: Vec<PackedQM31> = Vec::with_capacity(simd_size);
        let mut emit_wide = Vec::with_capacity(simd_size);
        let mut wide_denoms: Vec<PackedQM31> = Vec::with_capacity(simd_size);
        let mut emit_io = Vec::with_capacity(simd_size);
        let mut io_denoms: Vec<PackedQM31> = Vec::with_capacity(simd_size);
        for i in 0..simd_size {
            let (_, entries) = cols.at(i).evaluation();
            let (_, tuple) = entries.into_iter().next().expect("one activation tuple");
            let enabler = cols.enabler[i];
            let wide = cols.wide[i];
            let io = cols.io[i];
            consume_input.push(-PackedQM31::from(enabler * (one - io)));
            input_denoms.push(relations.poseidon2.combine(&tuple[..16]));
            emit_narrow.push(PackedQM31::from(enabler * (one - wide - io)));
            narrow_denoms.push(relations.poseidon2.combine(&tuple[16..17]));
            emit_wide.push(PackedQM31::from(enabler * wide));
            wide_denoms.push(relations.poseidon2.combine(&tuple[16..24]));
            emit_io.push(PackedQM31::from(enabler * io));
            io_denoms.push(relations.poseidon2_io.combine(&tuple));
        }

        stwo_macros::write_pair!(
            &consume_input,
            &input_denoms,
            &emit_narrow,
            &narrow_denoms,
            logup_gen
        );
        stwo_macros::write_pair!(&emit_wide, &wide_denoms, &emit_io, &io_denoms, logup_gen);
        logup_gen.finalize_last()
    }

    /// Poseidon2 rows request no preprocessed lookup multiplicities.
    pub fn register_multiplicities(
        _trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
        _counters: &mut crate::relations::Counters,
    ) {
    }
}
