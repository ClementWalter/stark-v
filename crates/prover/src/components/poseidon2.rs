//! Poseidon2 hash component for Merkle commitments.

use std::ops::{Add, AddAssign, Mul, Sub};

use num_traits::{One, Zero};
use stwo::core::ColumnVec;
use stwo::core::fields::FieldExpOps;
use stwo::core::fields::m31::{BaseField, M31};
use stwo::core::fields::qm31::QM31;
use stwo::prover::backend::simd::SimdBackend;
use stwo::prover::backend::simd::qm31::PackedQM31;
use stwo::prover::poly::BitReversedOrder;
use stwo::prover::poly::circle::CircleEvaluation;
use stwo_constraint_framework::LogupTraceGenerator;
use stwo_constraint_framework::{EvalAtRow, FrameworkComponent, FrameworkEval};

use crate::add_to_relation;
use crate::relations::Relations;

const T: usize = 16;
const FULL_ROUNDS: usize = 8;
const PARTIAL_ROUNDS: usize = 14;
const FINAL_STATE_START: usize = 1 + T + FULL_ROUNDS * 3 * T + PARTIAL_ROUNDS * 3 - T;

const EXTERNAL_ROUND_CONSTS: [[M31; 16]; 8] = [
    [
        M31::from_u32_unchecked(1988864850),
        M31::from_u32_unchecked(1893772157),
        M31::from_u32_unchecked(1025928330),
        M31::from_u32_unchecked(1839472709),
        M31::from_u32_unchecked(1611656994),
        M31::from_u32_unchecked(1104858731),
        M31::from_u32_unchecked(1694088660),
        M31::from_u32_unchecked(1564660990),
        M31::from_u32_unchecked(1991332205),
        M31::from_u32_unchecked(1875486487),
        M31::from_u32_unchecked(1890340790),
        M31::from_u32_unchecked(1658614),
        M31::from_u32_unchecked(582370530),
        M31::from_u32_unchecked(528029397),
        M31::from_u32_unchecked(1196956642),
        M31::from_u32_unchecked(655401251),
    ],
    [
        M31::from_u32_unchecked(1652877415),
        M31::from_u32_unchecked(26032894),
        M31::from_u32_unchecked(1576640243),
        M31::from_u32_unchecked(1277052539),
        M31::from_u32_unchecked(1450142396),
        M31::from_u32_unchecked(697623591),
        M31::from_u32_unchecked(1401580866),
        M31::from_u32_unchecked(1568404175),
        M31::from_u32_unchecked(2145004971),
        M31::from_u32_unchecked(265835716),
        M31::from_u32_unchecked(1183985610),
        M31::from_u32_unchecked(1031234465),
        M31::from_u32_unchecked(436012490),
        M31::from_u32_unchecked(172735299),
        M31::from_u32_unchecked(352802897),
        M31::from_u32_unchecked(1032863094),
    ],
    [
        M31::from_u32_unchecked(757665783),
        M31::from_u32_unchecked(1082171296),
        M31::from_u32_unchecked(1507509996),
        M31::from_u32_unchecked(309929890),
        M31::from_u32_unchecked(1807683232),
        M31::from_u32_unchecked(43258895),
        M31::from_u32_unchecked(611592566),
        M31::from_u32_unchecked(1854193793),
        M31::from_u32_unchecked(575164234),
        M31::from_u32_unchecked(894217817),
        M31::from_u32_unchecked(72613857),
        M31::from_u32_unchecked(1061659596),
        M31::from_u32_unchecked(8921166),
        M31::from_u32_unchecked(1617355017),
        M31::from_u32_unchecked(998001536),
        M31::from_u32_unchecked(1800758877),
    ],
    [
        M31::from_u32_unchecked(1002748055),
        M31::from_u32_unchecked(1935405944),
        M31::from_u32_unchecked(1351462722),
        M31::from_u32_unchecked(411368491),
        M31::from_u32_unchecked(1913975372),
        M31::from_u32_unchecked(1956167178),
        M31::from_u32_unchecked(442558016),
        M31::from_u32_unchecked(855898408),
        M31::from_u32_unchecked(699687798),
        M31::from_u32_unchecked(1553382248),
        M31::from_u32_unchecked(1708169125),
        M31::from_u32_unchecked(490049183),
        M31::from_u32_unchecked(1251643415),
        M31::from_u32_unchecked(1193594742),
        M31::from_u32_unchecked(880473871),
        M31::from_u32_unchecked(511174042),
    ],
    [
        M31::from_u32_unchecked(1460209171),
        M31::from_u32_unchecked(530850056),
        M31::from_u32_unchecked(398192464),
        M31::from_u32_unchecked(536338716),
        M31::from_u32_unchecked(75179210),
        M31::from_u32_unchecked(1309934197),
        M31::from_u32_unchecked(1335920373),
        M31::from_u32_unchecked(127611036),
        M31::from_u32_unchecked(291093831),
        M31::from_u32_unchecked(1832379621),
        M31::from_u32_unchecked(123571662),
        M31::from_u32_unchecked(303176864),
        M31::from_u32_unchecked(2137685056),
        M31::from_u32_unchecked(1759609530),
        M31::from_u32_unchecked(1418928155),
        M31::from_u32_unchecked(71608334),
    ],
    [
        M31::from_u32_unchecked(6616262),
        M31::from_u32_unchecked(1684515814),
        M31::from_u32_unchecked(1721194338),
        M31::from_u32_unchecked(720801691),
        M31::from_u32_unchecked(878392254),
        M31::from_u32_unchecked(460379263),
        M31::from_u32_unchecked(87930647),
        M31::from_u32_unchecked(940673483),
        M31::from_u32_unchecked(1136203256),
        M31::from_u32_unchecked(551499412),
        M31::from_u32_unchecked(256220454),
        M31::from_u32_unchecked(2007034235),
        M31::from_u32_unchecked(796124985),
        M31::from_u32_unchecked(410436345),
        M31::from_u32_unchecked(1705042586),
        M31::from_u32_unchecked(1286336446),
    ],
    [
        M31::from_u32_unchecked(1522340456),
        M31::from_u32_unchecked(1295296352),
        M31::from_u32_unchecked(309794713),
        M31::from_u32_unchecked(1772145068),
        M31::from_u32_unchecked(956898901),
        M31::from_u32_unchecked(2137070800),
        M31::from_u32_unchecked(988829146),
        M31::from_u32_unchecked(2059451359),
        M31::from_u32_unchecked(1846491684),
        M31::from_u32_unchecked(1105442551),
        M31::from_u32_unchecked(1236497773),
        M31::from_u32_unchecked(1452000568),
        M31::from_u32_unchecked(549485016),
        M31::from_u32_unchecked(385992492),
        M31::from_u32_unchecked(1987107948),
        M31::from_u32_unchecked(1514377269),
    ],
    [
        M31::from_u32_unchecked(2090065934),
        M31::from_u32_unchecked(1444920141),
        M31::from_u32_unchecked(293113979),
        M31::from_u32_unchecked(41120774),
        M31::from_u32_unchecked(855319793),
        M31::from_u32_unchecked(1663284746),
        M31::from_u32_unchecked(1789994008),
        M31::from_u32_unchecked(1120509162),
        M31::from_u32_unchecked(358222743),
        M31::from_u32_unchecked(1406256810),
        M31::from_u32_unchecked(735183687),
        M31::from_u32_unchecked(664485235),
        M31::from_u32_unchecked(1331641456),
        M31::from_u32_unchecked(38121324),
        M31::from_u32_unchecked(595810771),
        M31::from_u32_unchecked(1234594393),
    ],
];

const INTERNAL_ROUND_CONSTS: [M31; 14] = [
    M31::from_u32_unchecked(2139014335),
    M31::from_u32_unchecked(69309039),
    M31::from_u32_unchecked(1368974953),
    M31::from_u32_unchecked(886780232),
    M31::from_u32_unchecked(1130937085),
    M31::from_u32_unchecked(1718115455),
    M31::from_u32_unchecked(2027103386),
    M31::from_u32_unchecked(1612216449),
    M31::from_u32_unchecked(1994053242),
    M31::from_u32_unchecked(110146615),
    M31::from_u32_unchecked(514413329),
    M31::from_u32_unchecked(1088763546),
    M31::from_u32_unchecked(955319292),
    M31::from_u32_unchecked(488794657),
];

const INTERNAL_MATRIX: [M31; 16] = [
    M31::from_u32_unchecked(129501892),
    M31::from_u32_unchecked(1809435443),
    M31::from_u32_unchecked(1223573407),
    M31::from_u32_unchecked(1331944729),
    M31::from_u32_unchecked(415581875),
    M31::from_u32_unchecked(1526242955),
    M31::from_u32_unchecked(1341275624),
    M31::from_u32_unchecked(1333308150),
    M31::from_u32_unchecked(1404946132),
    M31::from_u32_unchecked(1549369918),
    M31::from_u32_unchecked(709303410),
    M31::from_u32_unchecked(1284988537),
    M31::from_u32_unchecked(1490838740),
    M31::from_u32_unchecked(115945821),
    M31::from_u32_unchecked(754131590),
    M31::from_u32_unchecked(800486749),
];

#[inline(always)]
fn apply_m4<F>(x: [F; 4]) -> [F; 4]
where
    F: Clone + AddAssign<F> + Add<F, Output = F> + Sub<F, Output = F> + Mul<M31, Output = F>,
{
    let t0 = x[0].clone() + x[1].clone();
    let t02 = t0.clone() + t0.clone();
    let t1 = x[2].clone() + x[3].clone();
    let t12 = t1.clone() + t1.clone();
    let t2 = x[1].clone() + x[1].clone() + t1;
    let t3 = x[3].clone() + x[3].clone() + t0;
    let t4 = t12.clone() + t12 + t3.clone();
    let t5 = t02.clone() + t02 + t2.clone();
    let t6 = t3 + t5.clone();
    let t7 = t2 + t4.clone();
    [t6, t5, t7, t4]
}

#[inline(always)]
fn apply_external_round_matrix<F>(state: &mut [F; 16])
where
    F: Clone + AddAssign<F> + Add<F, Output = F> + Sub<F, Output = F> + Mul<M31, Output = F>,
{
    for i in 0..4 {
        [
            state[4 * i],
            state[4 * i + 1],
            state[4 * i + 2],
            state[4 * i + 3],
        ] = apply_m4([
            state[4 * i].clone(),
            state[4 * i + 1].clone(),
            state[4 * i + 2].clone(),
            state[4 * i + 3].clone(),
        ]);
    }
    for j in 0..4 {
        let s =
            state[j].clone() + state[j + 4].clone() + state[j + 8].clone() + state[j + 12].clone();
        for i in 0..4 {
            state[4 * i + j] += s.clone();
        }
    }
}

#[inline(always)]
fn apply_internal_round_matrix<F>(state: &mut [F; 16])
where
    F: Clone + AddAssign<F> + Add<F, Output = F> + Sub<F, Output = F> + Mul<M31, Output = F>,
{
    let sum = state[1..]
        .iter()
        .cloned()
        .fold(state[0].clone(), |acc, s| acc + s);
    state.iter_mut().enumerate().for_each(|(i, s)| {
        *s = s.clone() * INTERNAL_MATRIX[i] + sum.clone();
    });
}

#[inline(always)]
fn square<F: FieldExpOps>(x: F) -> F {
    x.clone() * x
}

pub mod air {
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
                eval.add_constraint(
                    enabler.clone() * (initial_state * state[0].clone() - m.clone()),
                );
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

            add_to_relation!(
                eval,
                self.relations.poseidon2,
                -enabler.clone(),
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
                enabler,
                state[0].clone()
            );
            eval.finalize_logup_in_pairs();
            eval
        }
    }
}

pub mod witness {
    use super::*;
    use crate::{combine, write_pair};

    pub fn gen_interaction_trace(
        trace: &ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        relations: &Relations,
    ) -> (
        ColumnVec<CircleEvaluation<SimdBackend, BaseField, BitReversedOrder>>,
        QM31,
    ) {
        if trace.is_empty() {
            return (vec![], QM31::zero());
        }

        let enabler = &trace[0].data;
        let simd_size = enabler.len();
        let log_size = trace[0].domain.log_size();
        let mut interaction_trace = LogupTraceGenerator::new(log_size);

        let neg_enabler: Vec<PackedQM31> = (0..simd_size)
            .map(|i| -PackedQM31::from(enabler[i]))
            .collect();
        let pos_enabler: Vec<PackedQM31> = (0..simd_size)
            .map(|i| PackedQM31::from(enabler[i]))
            .collect();

        let init_state_denom = combine!(
            relations.poseidon2,
            [
                &trace[1].data,
                &trace[2].data,
                &trace[3].data,
                &trace[4].data,
                &trace[5].data,
                &trace[6].data,
                &trace[7].data,
                &trace[8].data,
                &trace[9].data,
                &trace[10].data,
                &trace[11].data,
                &trace[12].data,
                &trace[13].data,
                &trace[14].data,
                &trace[15].data,
                &trace[16].data
            ]
        );
        let output_denom = combine!(relations.poseidon2, [&trace[FINAL_STATE_START].data]);

        write_pair!(
            &neg_enabler,
            &init_state_denom,
            &pos_enabler,
            &output_denom,
            interaction_trace
        );

        interaction_trace.finalize_last()
    }
}
