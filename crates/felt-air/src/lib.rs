//! A felt function system compiled to AIR (docs/felt-air-compiler.md).
//!
//! Each `fn` below becomes an AIR table whose rows are activations: a row
//! consumes its own `(inputs..., outputs...)` tuple through the function's
//! LogUp relation and emits one for every call it makes; the verifier emits
//! the public entry activations. `max_degree` drives materialization:
//! `quint`'s `x2 * x2 * x` chain is automatically unrolled into committed
//! intermediate columns, while `affine`'s additive chain stays one inline
//! expression.

stwo_macros::define_air_fns! {
    max_degree: 3,

    fn cube(x) {
        let x2 = x * x;
        return x2 * x;
    }

    fn quint(x) {
        let x2 = x * x;
        return x2 * x2 * x;
    }

    fn affine(a, b, c) {
        return a + 2 * b + 3 * c + 7;
    }

    fn poly(a, b) {
        let c = cube(a);
        let q = quint(b);
        let s = affine(a, b, c);
        assert (a + b) * (a + b) == a * a + 2 * a * b + b * b;
        return (c + q, s * s);
    }
}
