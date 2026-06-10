//! Fiat-Shamir sponge replay component: witness generation and AIR evaluation.
//!
//! Each enabled row replays one absorption step of a Poseidon2 sponge: the
//! permutation input is `prev_state` with the absorbed chunk added into the
//! rate (degree-1, in-relation arithmetic), and the permutation itself is
//! bound atomically through the `poseidon2_io` relation to the reused
//! poseidon2 component. States chain through the `sponge_step` relation and
//! the absorbed words through `sponge_data`, both anchored by public claim
//! terms — so a replayed channel balances to its public transcript: initial
//! state in, data chunks in, final digest out.

use prover::relations::Relations;
use runner::poseidon2::{T, poseidon2_permutation, poseidon2_traced_state};
use runner::trace::Poseidon2Table;
use stwo::core::ColumnVec;
use stwo::core::fields::m31::{BaseField, P};
use stwo::core::fields::qm31::QM31;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::qm31::PackedQM31;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::{
    EvalAtRow, FrameworkComponent, FrameworkEval, LogupTraceGenerator, RelationEntry,
};

use crate::ChannelReplayTable;
use crate::prover_columns::ChannelReplayColumns;
use crate::relations::RecursionRelations;

pub type Component = FrameworkComponent<Eval>;

/// Sponge rate in words: chunks are absorbed into the first 8 state words.
pub const RATE: usize = 8;

#[derive(Clone)]
pub struct Eval {
    pub log_size: u32,
    pub relations: Relations,
    pub recursion_relations: RecursionRelations,
}

impl FrameworkEval for Eval {
    fn log_size(&self) -> u32 {
        self.log_size
    }

    fn max_constraint_log_degree_bound(&self) -> u32 {
        self.log_size + 1
    }

    fn evaluate<E: EvalAtRow>(&self, mut eval: E) -> E {
        let cols = ChannelReplayColumns::from_eval(&mut eval);
        for constraint in cols.constraints() {
            eval.add_constraint(constraint);
        }
        let one = E::F::from(BaseField::from_u32_unchecked(1));
        let prev = [
            cols.prev_0.clone(),
            cols.prev_1.clone(),
            cols.prev_2.clone(),
            cols.prev_3.clone(),
            cols.prev_4.clone(),
            cols.prev_5.clone(),
            cols.prev_6.clone(),
            cols.prev_7.clone(),
            cols.prev_8.clone(),
            cols.prev_9.clone(),
            cols.prev_10.clone(),
            cols.prev_11.clone(),
            cols.prev_12.clone(),
            cols.prev_13.clone(),
            cols.prev_14.clone(),
            cols.prev_15.clone(),
        ];
        let chunk = [
            cols.chunk_0.clone(),
            cols.chunk_1.clone(),
            cols.chunk_2.clone(),
            cols.chunk_3.clone(),
            cols.chunk_4.clone(),
            cols.chunk_5.clone(),
            cols.chunk_6.clone(),
            cols.chunk_7.clone(),
        ];
        let out = [
            cols.out_0.clone(),
            cols.out_1.clone(),
            cols.out_2.clone(),
            cols.out_3.clone(),
            cols.out_4.clone(),
            cols.out_5.clone(),
            cols.out_6.clone(),
            cols.out_7.clone(),
            cols.out_8.clone(),
            cols.out_9.clone(),
            cols.out_10.clone(),
            cols.out_11.clone(),
            cols.out_12.clone(),
            cols.out_13.clone(),
            cols.out_14.clone(),
            cols.out_15.clone(),
        ];

        // Atomic permutation binding: input = prev + chunk into the rate.
        let mut io_tuple: Vec<E::F> = Vec::with_capacity(2 * T);
        for (j, prev_j) in prev.iter().enumerate() {
            if j < RATE {
                io_tuple.push(prev_j.clone() + chunk[j].clone());
            } else {
                io_tuple.push(prev_j.clone());
            }
        }
        io_tuple.extend(out.iter().cloned());
        eval.add_to_relation(RelationEntry::new(
            &self.relations.poseidon2_io,
            -E::EF::from(cols.enabler.clone()),
            &io_tuple,
        ));

        // Absorbed data anchored against the public transcript.
        let mut data_tuple: Vec<E::F> = vec![cols.channel_id.clone(), cols.step.clone()];
        data_tuple.extend(chunk.iter().cloned());
        eval.add_to_relation(RelationEntry::new(
            &self.recursion_relations.sponge_data,
            -E::EF::from(cols.enabler.clone()),
            &data_tuple,
        ));

        // State chaining: consume the previous claim, emit the next.
        let mut prev_tuple: Vec<E::F> = vec![cols.channel_id.clone(), cols.step.clone()];
        prev_tuple.extend(prev.iter().cloned());
        eval.add_to_relation(RelationEntry::new(
            &self.recursion_relations.sponge_step,
            -E::EF::from(cols.enabler.clone()),
            &prev_tuple,
        ));
        let mut next_tuple: Vec<E::F> = vec![cols.channel_id.clone(), cols.step.clone() + one];
        next_tuple.extend(out.iter().cloned());
        eval.add_to_relation(RelationEntry::new(
            &self.recursion_relations.sponge_step,
            E::EF::from(cols.enabler.clone()),
            &next_tuple,
        ));

        eval.finalize_logup_in_pairs();
        eval
    }
}

/// Replay one absorption step host-side and record the witness rows: the
/// binding row here and the io permutation row in the poseidon2 table.
/// Returns the next sponge state.
pub fn push_sponge_step(
    table: &mut ChannelReplayTable,
    poseidon2: &mut Poseidon2Table,
    channel_id: u32,
    step: u32,
    prev_state: [u32; T],
    chunk: [u32; RATE],
) -> [u32; T] {
    let mut input = prev_state;
    for (slot, &word) in input.iter_mut().zip(chunk.iter()) {
        debug_assert!(word < P);
        *slot = ((*slot as u64 + word as u64) % P as u64) as u32;
    }
    let row = poseidon2_traced_state(input, false, true);
    poseidon2.push_row(&row);
    let mut out = input;
    poseidon2_permutation(&mut out);

    let mut values = vec![channel_id, step];
    values.extend_from_slice(&prev_state);
    values.extend_from_slice(&chunk);
    values.extend_from_slice(&out);
    table.push_row_values(&values);
    out
}

impl ChannelReplayTable {
    /// Push a row from flat values (channel_id, step, prev, chunk, out)
    /// without the 42-argument `push` call.
    fn push_row_values(&mut self, values: &[u32]) {
        // push_row expects the enabler prefix the generated tables carry.
        let mut row = Vec::with_capacity(values.len() + 1);
        row.push(1);
        row.extend_from_slice(values);
        self.push_row(&row);
    }
}

/// Replay a full transcript host-side: fold `chunks` from the all-zero state.
pub fn replay_digest(chunks: &[[u32; RATE]]) -> [u32; T] {
    let mut state = [0u32; T];
    for chunk in chunks {
        for (slot, &word) in state.iter_mut().zip(chunk.iter()) {
            *slot = ((*slot as u64 + word as u64) % P as u64) as u32;
        }
        poseidon2_permutation(&mut state);
    }
    state
}

/// Generate the interaction trace and the claimed sum of the four relation
/// entries.
pub fn gen_interaction_trace(
    trace: &[CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>],
    relations: &Relations,
    recursion_relations: &RecursionRelations,
) -> (
    ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
    QM31,
) {
    let cols = ChannelReplayColumns::from_iter(trace.iter().map(|eval| &eval.values.data));
    let simd_size = cols.enabler.len();
    let log_size = trace[0].domain.log_size();
    let mut logup_gen = LogupTraceGenerator::new(log_size);

    let pos_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| PackedQM31::from(cols.enabler[i]))
        .collect();
    let neg_enabler: Vec<PackedQM31> = (0..simd_size)
        .map(|i| -PackedQM31::from(cols.enabler[i]))
        .collect();

    let one = stwo::prover::backend::simd::m31::PackedM31::broadcast(BaseField::from(1));
    let in_rate: Vec<Vec<_>> = (0..RATE)
        .map(|j| {
            let prev = [
                cols.prev_0,
                cols.prev_1,
                cols.prev_2,
                cols.prev_3,
                cols.prev_4,
                cols.prev_5,
                cols.prev_6,
                cols.prev_7,
            ][j];
            let chunk = [
                cols.chunk_0,
                cols.chunk_1,
                cols.chunk_2,
                cols.chunk_3,
                cols.chunk_4,
                cols.chunk_5,
                cols.chunk_6,
                cols.chunk_7,
            ][j];
            (0..simd_size).map(|i| prev[i] + chunk[i]).collect()
        })
        .collect();
    let step_plus_1: Vec<_> = (0..simd_size).map(|i| cols.step[i] + one).collect();

    let io_denom = combine!(
        relations.poseidon2_io,
        [
            &in_rate[0],
            &in_rate[1],
            &in_rate[2],
            &in_rate[3],
            &in_rate[4],
            &in_rate[5],
            &in_rate[6],
            &in_rate[7],
            cols.prev_8,
            cols.prev_9,
            cols.prev_10,
            cols.prev_11,
            cols.prev_12,
            cols.prev_13,
            cols.prev_14,
            cols.prev_15,
            cols.out_0,
            cols.out_1,
            cols.out_2,
            cols.out_3,
            cols.out_4,
            cols.out_5,
            cols.out_6,
            cols.out_7,
            cols.out_8,
            cols.out_9,
            cols.out_10,
            cols.out_11,
            cols.out_12,
            cols.out_13,
            cols.out_14,
            cols.out_15
        ]
    );
    let data_denom = combine!(
        recursion_relations.sponge_data,
        [
            cols.channel_id,
            cols.step,
            cols.chunk_0,
            cols.chunk_1,
            cols.chunk_2,
            cols.chunk_3,
            cols.chunk_4,
            cols.chunk_5,
            cols.chunk_6,
            cols.chunk_7
        ]
    );
    let prev_denom = combine!(
        recursion_relations.sponge_step,
        [
            cols.channel_id,
            cols.step,
            cols.prev_0,
            cols.prev_1,
            cols.prev_2,
            cols.prev_3,
            cols.prev_4,
            cols.prev_5,
            cols.prev_6,
            cols.prev_7,
            cols.prev_8,
            cols.prev_9,
            cols.prev_10,
            cols.prev_11,
            cols.prev_12,
            cols.prev_13,
            cols.prev_14,
            cols.prev_15
        ]
    );
    let next_denom = combine!(
        recursion_relations.sponge_step,
        [
            cols.channel_id,
            &step_plus_1,
            cols.out_0,
            cols.out_1,
            cols.out_2,
            cols.out_3,
            cols.out_4,
            cols.out_5,
            cols.out_6,
            cols.out_7,
            cols.out_8,
            cols.out_9,
            cols.out_10,
            cols.out_11,
            cols.out_12,
            cols.out_13,
            cols.out_14,
            cols.out_15
        ]
    );

    write_pair!(
        &neg_enabler,
        &io_denom,
        &neg_enabler,
        &data_denom,
        logup_gen
    );
    write_pair!(
        &neg_enabler,
        &prev_denom,
        &pos_enabler,
        &next_denom,
        logup_gen
    );
    logup_gen.finalize_last()
}
