//! secp256k1 affine point arithmetic.
//!
//! Implements on-curve checks, affine addition/doubling, and scalar
//! multiplication for secp256k1 using the in-tree field helpers.

use crate::crypto::secp256k1_math::{mod_add, mod_inv, mod_mul, mod_sub, secp256k1_p, SECP256K1_B};
use crate::types::U256;

const SECP256K1_GX_BYTES: [u8; 32] = [
    0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac,
    0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b, 0x07,
    0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9,
    0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17, 0x98,
];

const SECP256K1_GY_BYTES: [u8; 32] = [
    0x48, 0x3a, 0xda, 0x77, 0x26, 0xa3, 0xc4, 0x65,
    0x5d, 0xa4, 0xfb, 0xfc, 0x0e, 0x11, 0x08, 0xa8,
    0xfd, 0x17, 0xb4, 0x48, 0xa6, 0x85, 0x54, 0x19,
    0x9c, 0x47, 0xd0, 0x8f, 0xfb, 0x10, 0xd4, 0xb8,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AffinePoint {
    Infinity,
    Point { x: U256, y: U256 },
}

impl AffinePoint {
    pub fn generator() -> Self {
        AffinePoint::Point {
            x: U256::from_be_bytes(SECP256K1_GX_BYTES),
            y: U256::from_be_bytes(SECP256K1_GY_BYTES),
        }
    }

    pub fn is_infinity(&self) -> bool {
        matches!(self, AffinePoint::Infinity)
    }

    pub fn is_on_curve(&self) -> bool {
        match *self {
            AffinePoint::Infinity => true,
            AffinePoint::Point { x, y } => is_on_curve_xy(x, y),
        }
    }

    pub fn negate(self) -> Self {
        match self {
            AffinePoint::Infinity => AffinePoint::Infinity,
            AffinePoint::Point { x, y } => {
                let p = secp256k1_p();
                let neg_y = mod_sub(U256::ZERO, y, p);
                AffinePoint::Point { x, y: neg_y }
            }
        }
    }
}

fn is_on_curve_xy(x: U256, y: U256) -> bool {
    let p = secp256k1_p();
    let y_sq = mod_mul(y, y, p);
    let x_sq = mod_mul(x, x, p);
    let x_cubed = mod_mul(x_sq, x, p);
    let rhs = mod_add(x_cubed, SECP256K1_B, p);
    y_sq == rhs
}

pub fn point_double(point: AffinePoint) -> AffinePoint {
    match point {
        AffinePoint::Infinity => AffinePoint::Infinity,
        AffinePoint::Point { x, y } => {
            if y.is_zero() {
                return AffinePoint::Infinity;
            }
            let p = secp256k1_p();
            let three = U256::from_u64(3);
            let two = U256::from_u64(2);
            let x_sq = mod_mul(x, x, p);
            let numerator = mod_mul(three, x_sq, p);
            let denominator = mod_mul(two, y, p);
            let inv = match mod_inv(denominator, p) {
                Some(value) => value,
                None => return AffinePoint::Infinity,
            };
            let lambda = mod_mul(numerator, inv, p);
            let lambda_sq = mod_mul(lambda, lambda, p);
            let x3 = mod_sub(mod_sub(lambda_sq, x, p), x, p);
            let y3 = mod_sub(mod_mul(lambda, mod_sub(x, x3, p), p), y, p);
            AffinePoint::Point { x: x3, y: y3 }
        }
    }
}

pub fn point_add(left: AffinePoint, right: AffinePoint) -> AffinePoint {
    match (left, right) {
        (AffinePoint::Infinity, point) | (point, AffinePoint::Infinity) => point,
        (AffinePoint::Point { x: x1, y: y1 }, AffinePoint::Point { x: x2, y: y2 }) => {
            let p = secp256k1_p();
            if x1 == x2 {
                if mod_add(y1, y2, p).is_zero() {
                    return AffinePoint::Infinity;
                }
                return point_double(AffinePoint::Point { x: x1, y: y1 });
            }

            let numerator = mod_sub(y2, y1, p);
            let denominator = mod_sub(x2, x1, p);
            let inv = match mod_inv(denominator, p) {
                Some(value) => value,
                None => return AffinePoint::Infinity,
            };
            let lambda = mod_mul(numerator, inv, p);
            let lambda_sq = mod_mul(lambda, lambda, p);
            let x3 = mod_sub(mod_sub(lambda_sq, x1, p), x2, p);
            let y3 = mod_sub(mod_mul(lambda, mod_sub(x1, x3, p), p), y1, p);
            AffinePoint::Point { x: x3, y: y3 }
        }
    }
}

pub fn scalar_mul(mut scalar: U256, point: AffinePoint) -> AffinePoint {
    if scalar.is_zero() || point.is_infinity() {
        return AffinePoint::Infinity;
    }

    let mut acc = AffinePoint::Infinity;
    let mut base = point;

    while !scalar.is_zero() {
        if (scalar & U256::ONE) == U256::ONE {
            acc = point_add(acc, base);
        }
        base = point_double(base);
        scalar >>= 1;
    }

    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::secp256k1_math::secp256k1_n;

    const SECP256K1_2GX_BYTES: [u8; 32] = [
        0xc6, 0x04, 0x7f, 0x94, 0x41, 0xed, 0x7d, 0x6d,
        0x30, 0x45, 0x40, 0x6e, 0x95, 0xc0, 0x7c, 0xd8,
        0x5c, 0x77, 0x8e, 0x4b, 0x8c, 0xef, 0x3c, 0xa7,
        0xab, 0xac, 0x09, 0xb9, 0x5c, 0x70, 0x9e, 0xe5,
    ];

    const SECP256K1_2GY_BYTES: [u8; 32] = [
        0x1a, 0xe1, 0x68, 0xfe, 0xa6, 0x3d, 0xc3, 0x39,
        0xa3, 0xc5, 0x84, 0x19, 0x46, 0x6c, 0xea, 0xee,
        0xf7, 0xf6, 0x32, 0x65, 0x32, 0x66, 0xd0, 0xe1,
        0x23, 0x64, 0x31, 0xa9, 0x50, 0xcf, 0xe5, 0x2a,
    ];

    fn point_from_bytes(x: [u8; 32], y: [u8; 32]) -> AffinePoint {
        AffinePoint::Point {
            x: U256::from_be_bytes(x),
            y: U256::from_be_bytes(y),
        }
    }

    #[test]
    fn test_generator_on_curve() {
        let g = AffinePoint::generator();
        assert!(g.is_on_curve());
    }

    #[test]
    fn test_point_add_identity() {
        let g = AffinePoint::generator();
        assert_eq!(point_add(g, AffinePoint::Infinity), g);
        assert_eq!(point_add(AffinePoint::Infinity, g), g);
    }

    #[test]
    fn test_point_add_inverse() {
        let g = AffinePoint::generator();
        let neg = g.negate();
        assert_eq!(point_add(g, neg), AffinePoint::Infinity);
    }

    #[test]
    fn test_point_double_matches_known() {
        let g = AffinePoint::generator();
        let expected = point_from_bytes(SECP256K1_2GX_BYTES, SECP256K1_2GY_BYTES);
        assert_eq!(point_double(g), expected);
    }

    #[test]
    fn test_scalar_mul_two() {
        let g = AffinePoint::generator();
        let expected = point_from_bytes(SECP256K1_2GX_BYTES, SECP256K1_2GY_BYTES);
        let result = scalar_mul(U256::from_u64(2), g);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_scalar_mul_zero() {
        let g = AffinePoint::generator();
        let result = scalar_mul(U256::ZERO, g);
        assert_eq!(result, AffinePoint::Infinity);
    }

    #[test]
    fn test_scalar_mul_order_is_infinity() {
        let g = AffinePoint::generator();
        let result = scalar_mul(secp256k1_n(), g);
        assert_eq!(result, AffinePoint::Infinity);
    }
}
