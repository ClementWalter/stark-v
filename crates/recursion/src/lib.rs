//! Recursive verifier AIR components (docs/recursion.md, M3+).
//!
//! Building blocks for the native stwo AIR that verifies stark-v proofs.
//! Tables, derived columns, and constraints are declared once through
//! `define_component_tables!` — the same DSL as the opcode tables — keeping a
//! single source of definition for the whole recursion stack.
#![allow(clippy::too_many_arguments)] // generated table push takes one arg per column

pub mod circle_double;
pub mod fri_fold;
pub mod prover;
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

    // FRI line fold: folded = (f(x) + f(-x)) + alpha * (f(x) - f(-x)) * x^-1,
    // i.e. stwo's `ibutterfly` followed by the alpha combination. The odd
    // part t = (f(x) - f(-x)) * x^-1 is a witness column so every constraint
    // stays degree 2; x^-1 is bound to x by x * x_inv = enabler. f, alpha,
    // t, and folded are QM31 values as 4 M31 limbs; x is a base-field domain
    // coordinate.
    fri_fold_line: {
        x, x_inv,
        f_x_0, f_x_1, f_x_2, f_x_3,
        f_neg_x_0, f_neg_x_1, f_neg_x_2, f_neg_x_3,
        t_0, t_1, t_2, t_3,
        alpha_0, alpha_1, alpha_2, alpha_3,
        folded_0, folded_1, folded_2, folded_3,
        constraints: {
            // x_inv is the inverse of x on enabled rows
            |enabler, x, x_inv| x * x_inv - enabler,
            // t = (f(x) - f(-x)) * x_inv, limb-wise (x_inv is a base scalar)
            |x_inv, f_x_0, f_neg_x_0, t_0| (f_x_0 - f_neg_x_0) * x_inv - t_0,
            |x_inv, f_x_1, f_neg_x_1, t_1| (f_x_1 - f_neg_x_1) * x_inv - t_1,
            |x_inv, f_x_2, f_neg_x_2, t_2| (f_x_2 - f_neg_x_2) * x_inv - t_2,
            |x_inv, f_x_3, f_neg_x_3, t_3| (f_x_3 - f_neg_x_3) * x_inv - t_3,
            // folded = (f(x) + f(-x)) + alpha * t, with alpha * t expanded
            // over the extension tower exactly as in qm31_mul
            |f_x_0, f_neg_x_0, alpha_0, alpha_1, alpha_2, alpha_3, t_0, t_1, t_2, t_3, folded_0|
                f_x_0 + f_neg_x_0
                + alpha_0 * t_0 - alpha_1 * t_1
                + 2 * (alpha_2 * t_2 - alpha_3 * t_3) - (alpha_2 * t_3 + alpha_3 * t_2)
                - folded_0,
            |f_x_1, f_neg_x_1, alpha_0, alpha_1, alpha_2, alpha_3, t_0, t_1, t_2, t_3, folded_1|
                f_x_1 + f_neg_x_1
                + alpha_0 * t_1 + alpha_1 * t_0
                + (alpha_2 * t_2 - alpha_3 * t_3) + 2 * (alpha_2 * t_3 + alpha_3 * t_2)
                - folded_1,
            |f_x_2, f_neg_x_2, alpha_0, alpha_1, alpha_2, alpha_3, t_0, t_1, t_2, t_3, folded_2|
                f_x_2 + f_neg_x_2
                + alpha_0 * t_2 - alpha_1 * t_3 + alpha_2 * t_0 - alpha_3 * t_1
                - folded_2,
            |f_x_3, f_neg_x_3, alpha_0, alpha_1, alpha_2, alpha_3, t_0, t_1, t_2, t_3, folded_3|
                f_x_3 + f_neg_x_3
                + alpha_0 * t_3 + alpha_1 * t_2 + alpha_2 * t_1 + alpha_3 * t_0
                - folded_3,
        },
    },

    // Circle point doubling over QM31: r = 2p on the unit circle
    // x^2 + y^2 = 1, i.e. r_x = 2 p_x^2 - 1 and r_y = 2 p_x p_y. The squares
    // and products expand over the extension tower exactly as in qm31_mul;
    // the `- 1` lands on limb 0 as `- enabler` so padding rows hold.
    circle_double: {
        p_x_0, p_x_1, p_x_2, p_x_3,
        p_y_0, p_y_1, p_y_2, p_y_3,
        r_x_0, r_x_1, r_x_2, r_x_3,
        r_y_0, r_y_1, r_y_2, r_y_3,
        constraints: {
            // r_x = 2 * p_x^2 - 1
            |enabler, p_x_0, p_x_1, p_x_2, p_x_3, r_x_0|
                2 * (p_x_0 * p_x_0 - p_x_1 * p_x_1
                    + 2 * (p_x_2 * p_x_2 - p_x_3 * p_x_3) - 2 * (p_x_2 * p_x_3))
                - enabler - r_x_0,
            |p_x_0, p_x_1, p_x_2, p_x_3, r_x_1|
                2 * (2 * (p_x_0 * p_x_1)
                    + (p_x_2 * p_x_2 - p_x_3 * p_x_3) + 4 * (p_x_2 * p_x_3))
                - r_x_1,
            |p_x_0, p_x_1, p_x_2, p_x_3, r_x_2|
                2 * (2 * (p_x_0 * p_x_2) - 2 * (p_x_1 * p_x_3)) - r_x_2,
            |p_x_0, p_x_1, p_x_2, p_x_3, r_x_3|
                2 * (2 * (p_x_0 * p_x_3) + 2 * (p_x_1 * p_x_2)) - r_x_3,
            // r_y = 2 * p_x * p_y
            |p_x_0, p_x_1, p_x_2, p_x_3, p_y_0, p_y_1, p_y_2, p_y_3, r_y_0|
                2 * (p_x_0 * p_y_0 - p_x_1 * p_y_1
                    + 2 * (p_x_2 * p_y_2 - p_x_3 * p_y_3) - (p_x_2 * p_y_3 + p_x_3 * p_y_2))
                - r_y_0,
            |p_x_0, p_x_1, p_x_2, p_x_3, p_y_0, p_y_1, p_y_2, p_y_3, r_y_1|
                2 * (p_x_0 * p_y_1 + p_x_1 * p_y_0
                    + (p_x_2 * p_y_2 - p_x_3 * p_y_3) + 2 * (p_x_2 * p_y_3 + p_x_3 * p_y_2))
                - r_y_1,
            |p_x_0, p_x_1, p_x_2, p_x_3, p_y_0, p_y_1, p_y_2, p_y_3, r_y_2|
                2 * (p_x_0 * p_y_2 - p_x_1 * p_y_3 + p_x_2 * p_y_0 - p_x_3 * p_y_1) - r_y_2,
            |p_x_0, p_x_1, p_x_2, p_x_3, p_y_0, p_y_1, p_y_2, p_y_3, r_y_3|
                2 * (p_x_0 * p_y_3 + p_x_1 * p_y_2 + p_x_2 * p_y_1 + p_x_3 * p_y_0) - r_y_3,
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
