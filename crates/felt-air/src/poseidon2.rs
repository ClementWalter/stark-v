//! Poseidon2 as a felt function (docs/felt-air-compiler.md).
//!
//! The permutation is written as code — round loops, an `inline fn` for the
//! M4 block, `map`/`sum`/`update` over the 16-lane state — and the compiler
//! derives the flattened table, the constraints, and the witness fill. The
//! degree budget materializes the s-box chain automatically: two cells per
//! lane in the first round (the input lanes are already columns), three
//! afterwards — the layout the zkVM's hand-flattened poseidon2 table
//! reaches by hand. `test_permute_matches_runner` pins the program to
//! `runner::poseidon2::poseidon2_permutation` bit for bit.

// Generated tables carry hundreds of cells; clippy heuristics over the
// expansion (argument counts, comma layout) don't apply.
#![allow(clippy::too_many_arguments, clippy::possible_missing_comma)]

/// Round constants and the internal matrix (same values as
/// `runner::poseidon2`, which keeps them private).
pub const EXTERNAL_ROUND_CONSTS: [[u32; 16]; 8] = [
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

pub const INTERNAL_ROUND_CONSTS: [u32; 14] = [
    2139014335, 69309039, 1368974953, 886780232, 1130937085, 1718115455, 2027103386, 1612216449,
    1994053242, 110146615, 514413329, 1088763546, 955319292, 488794657,
];

pub const INTERNAL_MATRIX: [u32; 16] = [
    129501892, 1809435443, 1223573407, 1331944729, 415581875, 1526242955, 1341275624, 1333308150,
    1404946132, 1549369918, 709303410, 1284988537, 1490838740, 115945821, 754131590, 800486749,
];

stwo_macros::define_air_fns! {
    max_degree: 3,

    // The 4x4 MDS block of the external round matrix, as its addition chain.
    inline fn m4(x0, x1, x2, x3) {
        let t0 = x0 + x1;
        let t1 = x2 + x3;
        let t2 = 2 * x1 + t1;
        let t3 = 2 * x3 + t0;
        let t4 = 4 * t1 + t3;
        let t5 = 4 * t0 + t2;
        let t6 = t3 + t5;
        let t7 = t2 + t4;
        return (t6, t5, t7, t4);
    }

    // External round matrix: M4 per 4-lane block, then each lane adds its
    // column-wise sum across the blocks. Purely additive: stays inline.
    inline fn external_matrix(state: [felt; 16]) {
        let (b0, b1, b2, b3) = m4(state[0], state[1], state[2], state[3]);
        let (b4, b5, b6, b7) = m4(state[4], state[5], state[6], state[7]);
        let (b8, b9, b10, b11) = m4(state[8], state[9], state[10], state[11]);
        let (b12, b13, b14, b15) = m4(state[12], state[13], state[14], state[15]);
        let mixed = [b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15];
        let sums = map(j, 0..4, mixed[j] + mixed[j + 4] + mixed[j + 8] + mixed[j + 12]);
        let out = map(k, 0..16, mixed[k] + sums[k % 4]);
        return out;
    }

    // The Poseidon2 permutation over [felt; 16]: 4 + 4 full rounds around
    // 14 partial rounds. The x^5 s-box chains materialize automatically
    // under the degree budget.
    fn permute(state: [felt; 16]) {
        let state = external_matrix(state);
        for r in 0..4 {
            let state = map(j, 0..16, (state[j] + constant(crate::poseidon2::EXTERNAL_ROUND_CONSTS[r][j])) ** 5);
            let state = external_matrix(state);
        }
        for r in 0..14 {
            let state = update(state, 0, (state[0] + constant(crate::poseidon2::INTERNAL_ROUND_CONSTS[r])) ** 5);
            let total = sum(j, 0..16, state[j]);
            let state = map(j, 0..16, state[j] * constant(crate::poseidon2::INTERNAL_MATRIX[j]) + total);
        }
        for r in 4..8 {
            let state = map(j, 0..16, (state[j] + constant(crate::poseidon2::EXTERNAL_ROUND_CONSTS[r][j])) ** 5);
            let state = external_matrix(state);
        }
        return state;
    }
}
