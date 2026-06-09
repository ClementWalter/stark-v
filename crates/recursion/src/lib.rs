//! Recursive verifier AIR components (docs/recursion.md, M3+).
//!
//! Building blocks for the native stwo AIR that verifies stark-v proofs.
//! Tables, derived columns, and constraints are declared once through
//! `define_component_tables!` — the same DSL as the opcode tables — keeping a
//! single source of definition for the whole recursion stack.
#![allow(clippy::too_many_arguments)] // generated table push takes one arg per column

pub mod qm31_inv;
pub mod qm31_mul;

use stwo_macros::define_component_tables;

define_component_tables! {
    // QM31 multiplication: c = a * b over the degree-4 extension of M31.
    //
    // QM31 = CM31[u] / (u^2 - (2 + i)) with CM31 = M31[i] / (i^2 + 1).
    // Writing a = (a_0 + a_1 i) + (a_2 + a_3 i) u (likewise b, c) and
    // expanding (A + B u)(C + D u) = (AC + (2 + i) BD) + (AD + BC) u gives
    // the four limb constraints below. Every constraint is degree 2 and
    // vanishes on all-zero padding rows.
    qm31_mul: {
        a_0, a_1, a_2, a_3,
        b_0, b_1, b_2, b_3,
        c_0, c_1, c_2, c_3,
        constraints: {
            // Re(first): Re(AC) + Re((2 + i) BD)
            |a_0, a_1, a_2, a_3, b_0, b_1, b_2, b_3, c_0|
                a_0 * b_0 - a_1 * b_1
                + 2 * (a_2 * b_2 - a_3 * b_3) - (a_2 * b_3 + a_3 * b_2)
                - c_0,
            // Im(first): Im(AC) + Im((2 + i) BD)
            |a_0, a_1, a_2, a_3, b_0, b_1, b_2, b_3, c_1|
                a_0 * b_1 + a_1 * b_0
                + (a_2 * b_2 - a_3 * b_3) + 2 * (a_2 * b_3 + a_3 * b_2)
                - c_1,
            // Re(second): Re(AD) + Re(BC)
            |a_0, a_1, a_2, a_3, b_0, b_1, b_2, b_3, c_2|
                a_0 * b_2 - a_1 * b_3 + a_2 * b_0 - a_3 * b_1 - c_2,
            // Im(second): Im(AD) + Im(BC)
            |a_0, a_1, a_2, a_3, b_0, b_1, b_2, b_3, c_3|
                a_0 * b_3 + a_1 * b_2 + a_2 * b_1 + a_3 * b_0 - c_3,
        },
    },

    // QM31 inverse: inv = a^-1, asserted as a * inv = 1 with the same limb
    // expansion as qm31_mul. The right-hand side is `enabler` for limb 0 so
    // all-zero padding rows satisfy the constraints, and enabled rows force
    // `a` to be invertible.
    qm31_inv: {
        a_0, a_1, a_2, a_3,
        inv_0, inv_1, inv_2, inv_3,
        constraints: {
            |enabler, a_0, a_1, a_2, a_3, inv_0, inv_1, inv_2, inv_3|
                a_0 * inv_0 - a_1 * inv_1
                + 2 * (a_2 * inv_2 - a_3 * inv_3) - (a_2 * inv_3 + a_3 * inv_2)
                - enabler,
            |a_0, a_1, a_2, a_3, inv_0, inv_1, inv_2, inv_3|
                a_0 * inv_1 + a_1 * inv_0
                + (a_2 * inv_2 - a_3 * inv_3) + 2 * (a_2 * inv_3 + a_3 * inv_2),
            |a_0, a_1, a_2, a_3, inv_0, inv_1, inv_2, inv_3|
                a_0 * inv_2 - a_1 * inv_3 + a_2 * inv_0 - a_3 * inv_1,
            |a_0, a_1, a_2, a_3, inv_0, inv_1, inv_2, inv_3|
                a_0 * inv_3 + a_1 * inv_2 + a_2 * inv_1 + a_3 * inv_0,
        },
    },
}
