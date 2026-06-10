use stwo::core::fields::m31::M31;

// Default Poseidon2 Merkle hashes for leaf depth 21 (index = depth, depth 21 is 0).
pub const POSEIDON2_DEFAULT_HASHES_DEPTH_21: [u32; 22] = [
    1217577584, 378597429, 1556938811, 474559429, 423443822, 662201576, 1930942541, 2117464092,
    770448190, 1902191074, 1556109289, 776362864, 1750512713, 1171333637, 1423473161, 372035035,
    1457616685, 1303178213, 1563153690, 1383248003, 1183174448, 0,
];

pub const T: usize = 16;
const FULL_ROUNDS: usize = 8;
const PARTIAL_ROUNDS: usize = 14;

// enabler + initial state + per-round intermediate states + the `wide` flag
// selecting the 8-word digest emission (proof trees) over the 1-word one
// (memory trees).
pub const POSEIDON2_TRACE_COLUMNS: usize = 1 + T * (1 + FULL_ROUNDS * 3) + PARTIAL_ROUNDS * 3 + 1;
/// Index of the first final-state word within a trace row.
pub const POSEIDON2_FINAL_STATE_START: usize = POSEIDON2_TRACE_COLUMNS - T - 1;

const EXTERNAL_ROUND_CONSTS: [[u32; 16]; 8] = [
    [
        1988864850, 1893772157, 1025928330, 1839472709, 1611656994, 1104858731, 1694088660,
        1564660990, 1991332205, 1875486487, 1890340790, 1658614, 582370530, 528029397, 1196956642,
        655401251,
    ],
    [
        1652877415, 26032894, 1576640243, 1277052539, 1450142396, 697623591, 1401580866,
        1568404175, 2145004971, 265835716, 1183985610, 1031234465, 436012490, 172735299, 352802897,
        1032863094,
    ],
    [
        757665783, 1082171296, 1507509996, 309929890, 1807683232, 43258895, 611592566, 1854193793,
        575164234, 894217817, 72613857, 1061659596, 8921166, 1617355017, 998001536, 1800758877,
    ],
    [
        1002748055, 1935405944, 1351462722, 411368491, 1913975372, 1956167178, 442558016,
        855898408, 699687798, 1553382248, 1708169125, 490049183, 1251643415, 1193594742, 880473871,
        511174042,
    ],
    [
        1460209171, 530850056, 398192464, 536338716, 75179210, 1309934197, 1335920373, 127611036,
        291093831, 1832379621, 123571662, 303176864, 2137685056, 1759609530, 1418928155, 71608334,
    ],
    [
        6616262, 1684515814, 1721194338, 720801691, 878392254, 460379263, 87930647, 940673483,
        1136203256, 551499412, 256220454, 2007034235, 796124985, 410436345, 1705042586, 1286336446,
    ],
    [
        1522340456, 1295296352, 309794713, 1772145068, 956898901, 2137070800, 988829146,
        2059451359, 1846491684, 1105442551, 1236497773, 1452000568, 549485016, 385992492,
        1987107948, 1514377269,
    ],
    [
        2090065934, 1444920141, 293113979, 41120774, 855319793, 1663284746, 1789994008, 1120509162,
        358222743, 1406256810, 735183687, 664485235, 1331641456, 38121324, 595810771, 1234594393,
    ],
];

const INTERNAL_ROUND_CONSTS: [u32; 14] = [
    2139014335, 69309039, 1368974953, 886780232, 1130937085, 1718115455, 2027103386, 1612216449,
    1994053242, 110146615, 514413329, 1088763546, 955319292, 488794657,
];

const INTERNAL_MATRIX: [u32; 16] = [
    129501892, 1809435443, 1223573407, 1331944729, 415581875, 1526242955, 1341275624, 1333308150,
    1404946132, 1549369918, 709303410, 1284988537, 1490838740, 115945821, 754131590, 800486749,
];

#[inline]
fn add_m31(a: u32, b: u32) -> u32 {
    (M31::from(a) + M31::from(b)).0
}

#[inline]
fn mul_m31(a: u32, b: u32) -> u32 {
    (M31::from(a) * M31::from(b)).0
}

#[inline]
fn square_m31(x: u32) -> u32 {
    (M31::from(x) * M31::from(x)).0
}

#[inline]
fn apply_m4(x: [u32; 4]) -> [u32; 4] {
    let t0 = add_m31(x[0], x[1]);
    let t02 = add_m31(t0, t0);
    let t1 = add_m31(x[2], x[3]);
    let t12 = add_m31(t1, t1);
    let t2 = add_m31(add_m31(x[1], x[1]), t1);
    let t3 = add_m31(add_m31(x[3], x[3]), t0);
    let t4 = add_m31(add_m31(t12, t12), t3);
    let t5 = add_m31(add_m31(t02, t02), t2);
    let t6 = add_m31(t3, t5);
    let t7 = add_m31(t2, t4);
    [t6, t5, t7, t4]
}

#[inline]
fn apply_external_round_matrix(state: &mut [u32; 16]) {
    for i in 0..4 {
        let base = 4 * i;
        let out = apply_m4([
            state[base],
            state[base + 1],
            state[base + 2],
            state[base + 3],
        ]);
        state[base] = out[0];
        state[base + 1] = out[1];
        state[base + 2] = out[2];
        state[base + 3] = out[3];
    }

    for j in 0..4 {
        let mut sum = 0u32;
        sum = add_m31(sum, state[j]);
        sum = add_m31(sum, state[j + 4]);
        sum = add_m31(sum, state[j + 8]);
        sum = add_m31(sum, state[j + 12]);
        for i in 0..4 {
            let idx = 4 * i + j;
            state[idx] = add_m31(state[idx], sum);
        }
    }
}

#[inline]
fn apply_internal_round_matrix(state: &mut [u32; 16]) {
    let mut sum = 0u32;
    for val in state.iter() {
        sum = add_m31(sum, *val);
    }

    for (state_i, matrix_i) in state.iter_mut().zip(INTERNAL_MATRIX.iter()) {
        *state_i = add_m31(mul_m31(*state_i, *matrix_i), sum);
    }
}

/// The plain Poseidon2 permutation over `[M31; 16]`, without trace recording.
///
/// Same rounds, constants, and matrices as [`poseidon2_traced`]; used by the
/// proof-system channel and Merkle hasher where no AIR trace is needed.
pub fn poseidon2_permutation(state: &mut [u32; T]) {
    apply_external_round_matrix(state);

    for round_consts in EXTERNAL_ROUND_CONSTS.iter().take(FULL_ROUNDS / 2) {
        for (state_i, round_const) in state.iter_mut().zip(round_consts.iter()) {
            *state_i = add_m31(*state_i, *round_const);
        }
        let initial_state = *state;
        for state_i in state.iter_mut() {
            *state_i = square_m31(square_m31(*state_i));
        }
        for (state_i, init_i) in state.iter_mut().zip(initial_state.iter()) {
            *state_i = mul_m31(*state_i, *init_i);
        }
        apply_external_round_matrix(state);
    }

    for round_const in INTERNAL_ROUND_CONSTS.iter() {
        state[0] = add_m31(state[0], *round_const);
        let initial_state = state[0];
        state[0] = mul_m31(square_m31(square_m31(state[0])), initial_state);
        apply_internal_round_matrix(state);
    }

    for round_consts in EXTERNAL_ROUND_CONSTS.iter().skip(FULL_ROUNDS / 2) {
        for (state_i, round_const) in state.iter_mut().zip(round_consts.iter()) {
            *state_i = add_m31(*state_i, *round_const);
        }
        let initial_state = *state;
        for state_i in state.iter_mut() {
            *state_i = square_m31(square_m31(*state_i));
        }
        for (state_i, init_i) in state.iter_mut().zip(initial_state.iter()) {
            *state_i = mul_m31(*state_i, *init_i);
        }
        apply_external_round_matrix(state);
    }
}

pub fn poseidon2_traced(left: u32, right: u32) -> [u32; POSEIDON2_TRACE_COLUMNS] {
    let mut state = [0u32; T];
    state[0] = M31::from(left).0;
    state[1] = M31::from(right).0;
    poseidon2_traced_state(state, false)
}

/// Trace one permutation of an arbitrary initial state.
///
/// `wide` selects the 8-word digest emission in the Poseidon2 component
/// (proof commitment trees) instead of the 1-word one (memory trees).
pub fn poseidon2_traced_state(
    initial_state: [u32; T],
    wide: bool,
) -> [u32; POSEIDON2_TRACE_COLUMNS] {
    let mut row = [0u32; POSEIDON2_TRACE_COLUMNS];
    let mut idx = 0usize;

    row[idx] = 1;
    idx += 1;

    let mut state = initial_state;

    for value in state.iter() {
        row[idx] = *value;
        idx += 1;
    }

    apply_external_round_matrix(&mut state);

    for round_consts in EXTERNAL_ROUND_CONSTS.iter().take(FULL_ROUNDS / 2) {
        for (state_i, round_const) in state.iter_mut().zip(round_consts.iter()) {
            *state_i = add_m31(*state_i, *round_const);
        }
        let initial_state = state;
        for state_i in state.iter_mut() {
            *state_i = square_m31(*state_i);
            row[idx] = *state_i;
            idx += 1;
        }
        for state_i in state.iter_mut() {
            *state_i = square_m31(*state_i);
            row[idx] = *state_i;
            idx += 1;
        }
        for (state_i, init_i) in state.iter_mut().zip(initial_state.iter()) {
            *state_i = mul_m31(*state_i, *init_i);
        }
        apply_external_round_matrix(&mut state);
        for value in state.iter() {
            row[idx] = *value;
            idx += 1;
        }
    }

    for round_const in INTERNAL_ROUND_CONSTS.iter() {
        state[0] = add_m31(state[0], *round_const);
        let initial_state = state[0];
        state[0] = square_m31(state[0]);
        row[idx] = state[0];
        idx += 1;
        state[0] = square_m31(state[0]);
        row[idx] = state[0];
        idx += 1;
        state[0] = mul_m31(state[0], initial_state);
        row[idx] = state[0];
        idx += 1;
        apply_internal_round_matrix(&mut state);
    }

    for round_consts in EXTERNAL_ROUND_CONSTS.iter().skip(FULL_ROUNDS / 2) {
        for (state_i, round_const) in state.iter_mut().zip(round_consts.iter()) {
            *state_i = add_m31(*state_i, *round_const);
        }
        let initial_state = state;
        for state_i in state.iter_mut() {
            *state_i = square_m31(*state_i);
            row[idx] = *state_i;
            idx += 1;
        }
        for state_i in state.iter_mut() {
            *state_i = square_m31(*state_i);
            row[idx] = *state_i;
            idx += 1;
        }
        for (state_i, init_i) in state.iter_mut().zip(initial_state.iter()) {
            *state_i = mul_m31(*state_i, *init_i);
        }
        apply_external_round_matrix(&mut state);
        for value in state.iter() {
            row[idx] = *value;
            idx += 1;
        }
    }

    row[idx] = wide as u32;
    idx += 1;

    debug_assert_eq!(idx, POSEIDON2_TRACE_COLUMNS);
    row
}

#[cfg(test)]
mod permutation_tests {
    use super::*;

    #[test]
    fn test_permutation_matches_traced_oracle() {
        // poseidon2_traced records the permutation of (left, right, 0, ..);
        // its final 16 cells are the output state.
        let row = poseidon2_traced(123456789, 987654321);
        let mut state = [0u32; T];
        state[0] = 123456789;
        state[1] = 987654321;
        poseidon2_permutation(&mut state);
        assert_eq!(
            state,
            row[POSEIDON2_FINAL_STATE_START..POSEIDON2_FINAL_STATE_START + T]
        );
    }
}
