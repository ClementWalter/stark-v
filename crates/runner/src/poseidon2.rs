use crate::ops::utils::M31_P;

pub const T: usize = 16;
const FULL_ROUNDS: usize = 8;
const PARTIAL_ROUNDS: usize = 14;

pub const POSEIDON2_TRACE_COLUMNS: usize = 1 + T * (1 + FULL_ROUNDS * 3) + PARTIAL_ROUNDS * 3;

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
    let sum = a as u64 + b as u64;
    (sum % M31_P as u64) as u32
}

#[inline]
fn mul_m31(a: u32, b: u32) -> u32 {
    ((a as u64 * b as u64) % M31_P as u64) as u32
}

#[inline]
fn square_m31(x: u32) -> u32 {
    mul_m31(x, x)
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

    for i in 0..T {
        state[i] = add_m31(mul_m31(state[i], INTERNAL_MATRIX[i]), sum);
    }
}

pub fn poseidon2_hash(left: u32, right: u32) -> u32 {
    let mut state = [0u32; T];
    state[0] = left % M31_P;
    state[1] = right % M31_P;

    apply_external_round_matrix(&mut state);

    for round in 0..(FULL_ROUNDS / 2) {
        for i in 0..T {
            state[i] = add_m31(state[i], EXTERNAL_ROUND_CONSTS[round][i]);
        }
        let initial_state = state;
        for i in 0..T {
            state[i] = square_m31(state[i]);
        }
        for i in 0..T {
            state[i] = square_m31(state[i]);
        }
        for i in 0..T {
            state[i] = mul_m31(state[i], initial_state[i]);
        }
        apply_external_round_matrix(&mut state);
    }

    for round in 0..PARTIAL_ROUNDS {
        state[0] = add_m31(state[0], INTERNAL_ROUND_CONSTS[round]);
        let initial_state = state[0];
        state[0] = square_m31(state[0]);
        state[0] = square_m31(state[0]);
        state[0] = mul_m31(state[0], initial_state);
        apply_internal_round_matrix(&mut state);
    }

    for round in 0..(FULL_ROUNDS / 2) {
        let rc_round = round + FULL_ROUNDS / 2;
        for i in 0..T {
            state[i] = add_m31(state[i], EXTERNAL_ROUND_CONSTS[rc_round][i]);
        }
        let initial_state = state;
        for i in 0..T {
            state[i] = square_m31(state[i]);
        }
        for i in 0..T {
            state[i] = square_m31(state[i]);
        }
        for i in 0..T {
            state[i] = mul_m31(state[i], initial_state[i]);
        }
        apply_external_round_matrix(&mut state);
    }

    state[0]
}

pub fn poseidon2_traced(left: u32, right: u32) -> [u32; POSEIDON2_TRACE_COLUMNS] {
    let mut row = [0u32; POSEIDON2_TRACE_COLUMNS];
    let mut idx = 0usize;

    row[idx] = 1;
    idx += 1;

    let mut state = [0u32; T];
    state[0] = left % M31_P;
    state[1] = right % M31_P;

    for i in 0..T {
        row[idx] = state[i];
        idx += 1;
    }

    apply_external_round_matrix(&mut state);

    for round in 0..(FULL_ROUNDS / 2) {
        for i in 0..T {
            state[i] = add_m31(state[i], EXTERNAL_ROUND_CONSTS[round][i]);
        }
        let initial_state = state;
        for i in 0..T {
            state[i] = square_m31(state[i]);
            row[idx] = state[i];
            idx += 1;
        }
        for i in 0..T {
            state[i] = square_m31(state[i]);
            row[idx] = state[i];
            idx += 1;
        }
        for i in 0..T {
            state[i] = mul_m31(state[i], initial_state[i]);
        }
        apply_external_round_matrix(&mut state);
        for i in 0..T {
            row[idx] = state[i];
            idx += 1;
        }
    }

    for round in 0..PARTIAL_ROUNDS {
        state[0] = add_m31(state[0], INTERNAL_ROUND_CONSTS[round]);
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

    for round in 0..(FULL_ROUNDS / 2) {
        let rc_round = round + FULL_ROUNDS / 2;
        for i in 0..T {
            state[i] = add_m31(state[i], EXTERNAL_ROUND_CONSTS[rc_round][i]);
        }
        let initial_state = state;
        for i in 0..T {
            state[i] = square_m31(state[i]);
            row[idx] = state[i];
            idx += 1;
        }
        for i in 0..T {
            state[i] = square_m31(state[i]);
            row[idx] = state[i];
            idx += 1;
        }
        for i in 0..T {
            state[i] = mul_m31(state[i], initial_state[i]);
        }
        apply_external_round_matrix(&mut state);
        for i in 0..T {
            row[idx] = state[i];
            idx += 1;
        }
    }

    debug_assert_eq!(idx, POSEIDON2_TRACE_COLUMNS);
    row
}
