//! AIR evaluation for the Poseidon2 component.

use super::*;

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

    #[allow(clippy::needless_range_loop)]
    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let enabler = eval.next_trace_mask();
        let one = E::F::one();
        let mut state: [_; T] = std::array::from_fn(|_| eval.next_trace_mask());
        let initial_state = state.clone();

        eval.add_constraint(enabler.clone() * (one - enabler.clone()));
        apply_external_round_matrix(&mut state);

        for round in 0..(FULL_ROUNDS / 2) {
            for i in 0..T {
                state[i] += EXTERNAL_ROUND_CONSTS[round][i];
            }
            let initial_state = state.clone();

            state = std::array::from_fn(|i| square(state[i].clone()));
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });

            state = std::array::from_fn(|i| square(state[i].clone()));
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });

            state = std::array::from_fn(|i| state[i].clone() * initial_state[i].clone());
            apply_external_round_matrix(&mut state);
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });
        }

        for round in 0..PARTIAL_ROUNDS {
            state[0] += INTERNAL_ROUND_CONSTS[round];
            let initial_state = state[0].clone();

            let m = eval.next_trace_mask();
            eval.add_constraint(enabler.clone() * (square(state[0].clone()) - m.clone()));
            state[0] = m;

            let m = eval.next_trace_mask();
            eval.add_constraint(enabler.clone() * (square(state[0].clone()) - m.clone()));
            state[0] = m;

            let m = eval.next_trace_mask();
            eval.add_constraint(enabler.clone() * (initial_state * state[0].clone() - m.clone()));
            state[0] = m;

            apply_internal_round_matrix(&mut state);
        }

        for round in 0..(FULL_ROUNDS / 2) {
            for i in 0..T {
                state[i] += EXTERNAL_ROUND_CONSTS[FULL_ROUNDS / 2 + round][i];
            }
            let initial_state = state.clone();

            state = std::array::from_fn(|i| square(state[i].clone()));
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });

            state = std::array::from_fn(|i| square(state[i].clone()));
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });

            state = std::array::from_fn(|i| state[i].clone() * initial_state[i].clone());
            apply_external_round_matrix(&mut state);
            state.iter_mut().for_each(|s| {
                let m = eval.next_trace_mask();
                eval.add_constraint(enabler.clone() * (s.clone() - m.clone()));
                *s = m;
            });
        }

        // Emission shape flags: `wide` selects the 8-word digest (proof
        // trees) over the 1-word one (memory trees); `io` selects the atomic
        // (input, output) pair for sponge chaining. Mutually exclusive.
        let wide = eval.next_trace_mask();
        let io = eval.next_trace_mask();
        eval.add_constraint(wide.clone() * (E::F::one() - wide.clone()));
        eval.add_constraint(io.clone() * (E::F::one() - io.clone()));
        eval.add_constraint(wide.clone() * io.clone());

        // io rows bind their input through the atomic pair below instead.
        add_to_relation!(
            eval,
            self.relations.poseidon2,
            -(enabler.clone() * (E::F::one() - io.clone())),
            initial_state[0].clone(),
            initial_state[1].clone(),
            initial_state[2].clone(),
            initial_state[3].clone(),
            initial_state[4].clone(),
            initial_state[5].clone(),
            initial_state[6].clone(),
            initial_state[7].clone(),
            initial_state[8].clone(),
            initial_state[9].clone(),
            initial_state[10].clone(),
            initial_state[11].clone(),
            initial_state[12].clone(),
            initial_state[13].clone(),
            initial_state[14].clone(),
            initial_state[15].clone()
        );
        add_to_relation!(
            eval,
            self.relations.poseidon2,
            enabler.clone() * (E::F::one() - wide.clone() - io.clone()),
            state[0].clone()
        );
        add_to_relation!(
            eval,
            self.relations.poseidon2,
            enabler.clone() * wide.clone(),
            state[0].clone(),
            state[1].clone(),
            state[2].clone(),
            state[3].clone(),
            state[4].clone(),
            state[5].clone(),
            state[6].clone(),
            state[7].clone()
        );
        add_to_relation!(
            eval,
            self.relations.poseidon2_io,
            enabler * io,
            initial_state[0].clone(),
            initial_state[1].clone(),
            initial_state[2].clone(),
            initial_state[3].clone(),
            initial_state[4].clone(),
            initial_state[5].clone(),
            initial_state[6].clone(),
            initial_state[7].clone(),
            initial_state[8].clone(),
            initial_state[9].clone(),
            initial_state[10].clone(),
            initial_state[11].clone(),
            initial_state[12].clone(),
            initial_state[13].clone(),
            initial_state[14].clone(),
            initial_state[15].clone(),
            state[0].clone(),
            state[1].clone(),
            state[2].clone(),
            state[3].clone(),
            state[4].clone(),
            state[5].clone(),
            state[6].clone(),
            state[7].clone(),
            state[8].clone(),
            state[9].clone(),
            state[10].clone(),
            state[11].clone(),
            state[12].clone(),
            state[13].clone(),
            state[14].clone(),
            state[15].clone()
        );
        eval.finalize_logup_in_pairs();
        eval
    }
}
