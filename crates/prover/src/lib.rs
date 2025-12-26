#![allow(non_camel_case_types)]
#![feature(
    allocator_api,
    portable_simd,
    array_chunks,
    iter_array_chunks,
    macro_metavar_expr_concat
)]

#[macro_use]
pub mod macros;
pub mod components;
pub mod relations;
