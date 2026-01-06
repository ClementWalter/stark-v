//! LogUp protocol proc-macros for witness generation and AIR constraints.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, Token, bracketed};

// =============================================================================
// combine! macro
// =============================================================================

/// Input for combine: `$relations:expr, [$($col:expr),+]`
struct CombineInput {
    relations: Expr,
    cols: Vec<Expr>,
}

impl Parse for CombineInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let relations: Expr = input.parse()?;
        input.parse::<Token![,]>()?;

        let content;
        bracketed!(content in input);
        let cols: Punctuated<Expr, Token![,]> = content.parse_terminated(Expr::parse, Token![,])?;

        // Optional trailing comma after bracket
        let _ = input.parse::<Token![,]>();

        Ok(CombineInput {
            relations,
            cols: cols.into_iter().collect(),
        })
    }
}

pub fn combine(input: TokenStream) -> TokenStream {
    let CombineInput { relations, cols } = syn::parse_macro_input!(input as CombineInput);

    quote! {{
        use stwo_constraint_framework::Relation;

        let cols: Vec<&[stwo::prover::backend::simd::m31::PackedM31]> = vec![
            #(#cols.as_slice()),*
        ];
        let simd_size = cols[0].len();

        let mut combined: Vec<stwo::prover::backend::simd::qm31::PackedQM31> =
            Vec::with_capacity(simd_size);

        for row in 0..simd_size {
            let packed_m31_values: Vec<stwo::prover::backend::simd::m31::PackedM31> =
                cols.iter().map(|c| c[row]).collect();
            combined.push(#relations.combine(&packed_m31_values));
        }
        combined
    }}
    .into()
}

// =============================================================================
// emit_col! macro
// =============================================================================

/// Input for emit_col: `$denom:expr, $interaction_trace:expr`
struct TwoExprInput {
    first: Expr,
    second: Expr,
}

impl Parse for TwoExprInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let first: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let second: Expr = input.parse()?;
        // Optional trailing comma
        let _ = input.parse::<Token![,]>();
        Ok(TwoExprInput { first, second })
    }
}

pub fn emit_col(input: TokenStream) -> TokenStream {
    let TwoExprInput {
        first: denom,
        second: interaction_trace,
    } = syn::parse_macro_input!(input as TwoExprInput);

    quote! {{
        use num_traits::One;
        let mut col = #interaction_trace.new_col();
        let one = stwo::prover::backend::simd::qm31::PackedQM31::one();
        for (vec_row, &d) in (#denom).iter().enumerate() {
            col.write_frac(vec_row, one, d);
        }
        col.finalize_col();
    }}
    .into()
}

// =============================================================================
// consume_col! macro
// =============================================================================

pub fn consume_col(input: TokenStream) -> TokenStream {
    let TwoExprInput {
        first: denom,
        second: interaction_trace,
    } = syn::parse_macro_input!(input as TwoExprInput);

    quote! {{
        use num_traits::One;
        let mut col = #interaction_trace.new_col();
        let minus_one = -stwo::prover::backend::simd::qm31::PackedQM31::one();
        for (vec_row, &d) in (#denom).iter().enumerate() {
            col.write_frac(vec_row, minus_one, d);
        }
        col.finalize_col();
    }}
    .into()
}

// =============================================================================
// write_col! macro
// =============================================================================

/// Input for write_col: `$numerator:expr, $denom:expr, $interaction_trace:expr`
struct ThreeExprInput {
    first: Expr,
    second: Expr,
    third: Expr,
}

impl Parse for ThreeExprInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let first: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let second: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let third: Expr = input.parse()?;
        // Optional trailing comma
        let _ = input.parse::<Token![,]>();
        Ok(ThreeExprInput {
            first,
            second,
            third,
        })
    }
}

pub fn write_col(input: TokenStream) -> TokenStream {
    let ThreeExprInput {
        first: numerator,
        second: denom,
        third: interaction_trace,
    } = syn::parse_macro_input!(input as ThreeExprInput);

    quote! {{
        let mut col = #interaction_trace.new_col();
        for (vec_row, (n, d)) in itertools::izip!((#numerator).iter(), (#denom).iter()).enumerate() {
            col.write_frac(vec_row, *n, *d);
        }
        col.finalize_col();
    }}
    .into()
}

// =============================================================================
// write_pair! macro
// =============================================================================

/// Input for write_pair: `$n0:expr, $d0:expr, $n1:expr, $d1:expr, $trace:expr`
struct FiveExprInput {
    n0: Expr,
    d0: Expr,
    n1: Expr,
    d1: Expr,
    trace: Expr,
}

impl Parse for FiveExprInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let n0: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let d0: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let n1: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let d1: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let trace: Expr = input.parse()?;
        // Optional trailing comma
        let _ = input.parse::<Token![,]>();
        Ok(FiveExprInput { n0, d0, n1, d1, trace })
    }
}

pub fn write_pair(input: TokenStream) -> TokenStream {
    let FiveExprInput {
        n0: numerator_0,
        d0: denom_0,
        n1: numerator_1,
        d1: denom_1,
        trace: interaction_trace,
    } = syn::parse_macro_input!(input as FiveExprInput);

    quote! {{
        let mut col = #interaction_trace.new_col();
        for (vec_row, (n_0, d_0, n_1, d_1)) in itertools::izip!(
            (#numerator_0).iter(),
            (#denom_0).iter(),
            (#numerator_1).iter(),
            (#denom_1).iter()
        )
        .enumerate()
        {
            let numerator = *n_0 * *d_1 + *n_1 * *d_0;
            let denom = *d_0 * *d_1;
            col.write_frac(vec_row, numerator, denom);
        }
        col.finalize_col();
    }}
    .into()
}

// =============================================================================
// emit_pair! macro
// =============================================================================

pub fn emit_pair(input: TokenStream) -> TokenStream {
    let ThreeExprInput {
        first: denom_0,
        second: denom_1,
        third: interaction_trace,
    } = syn::parse_macro_input!(input as ThreeExprInput);

    quote! {{
        let mut col = #interaction_trace.new_col();
        for (vec_row, (d_0, d_1)) in itertools::izip!((#denom_0).iter(), (#denom_1).iter()).enumerate()
        {
            let numerator = *d_0 + *d_1;
            let denom = *d_0 * *d_1;
            col.write_frac(vec_row, numerator, denom);
        }
        col.finalize_col();
    }}
    .into()
}

// =============================================================================
// consume_pair! macro
// =============================================================================

/// Input for consume_pair - handles two variants:
/// 1. `$interaction_trace:expr; $($col:expr),+` - consume columns in pairs
/// 2. `$denom_0:expr, $denom_1:expr, $interaction_trace:expr` - consume two specific columns
enum ConsumePairInput {
    /// Variant 1: trace ; col1, col2, ...
    ListVariant { trace: Expr, cols: Vec<Expr> },
    /// Variant 2: denom0, denom1, trace
    PairVariant {
        denom_0: Expr,
        denom_1: Expr,
        trace: Expr,
    },
}

impl Parse for ConsumePairInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let first: Expr = input.parse()?;

        // Check if next token is semicolon (variant 1) or comma (variant 2)
        if input.peek(Token![;]) {
            input.parse::<Token![;]>()?;
            let cols: Punctuated<Expr, Token![,]> =
                Punctuated::parse_separated_nonempty(input)?;
            Ok(ConsumePairInput::ListVariant {
                trace: first,
                cols: cols.into_iter().collect(),
            })
        } else {
            input.parse::<Token![,]>()?;
            let second: Expr = input.parse()?;
            input.parse::<Token![,]>()?;
            let third: Expr = input.parse()?;
            // Optional trailing comma
            let _ = input.parse::<Token![,]>();
            Ok(ConsumePairInput::PairVariant {
                denom_0: first,
                denom_1: second,
                trace: third,
            })
        }
    }
}

pub fn consume_pair(input: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(input as ConsumePairInput);

    match parsed {
        ConsumePairInput::ListVariant { trace, cols } => {
            quote! {{
                let secure_columns = vec![#(#cols),*];
                for [pair0, pair1] in secure_columns.into_iter().array_chunks::<2>() {
                    let mut col = #trace.new_col();
                    for (vec_row, (d_0, d_1)) in itertools::izip!((pair0).iter(), (pair1).iter()).enumerate() {
                        let numerator = *d_0 + *d_1;
                        let denom = *d_0 * *d_1;
                        col.write_frac(vec_row, -numerator, denom);
                    }
                    col.finalize_col();
                }
            }}
            .into()
        }
        ConsumePairInput::PairVariant {
            denom_0,
            denom_1,
            trace,
        } => {
            quote! {{
                let mut col = #trace.new_col();
                for (vec_row, (d_0, d_1)) in itertools::izip!((#denom_0).iter(), (#denom_1).iter()).enumerate() {
                    let numerator = *d_0 + *d_1;
                    let denom = *d_0 * *d_1;
                    col.write_frac(vec_row, -numerator, denom);
                }
                col.finalize_col();
            }}
            .into()
        }
    }
}

// =============================================================================
// add_to_relation! macro
// =============================================================================

/// Input for add_to_relation: `$eval:expr, $relation:expr, $numerator:expr, $($col:expr),+`
struct AddToRelationInput {
    eval: Expr,
    relation: Expr,
    numerator: Expr,
    cols: Vec<Expr>,
}

impl Parse for AddToRelationInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let eval: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let relation: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let numerator: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let cols: Punctuated<Expr, Token![,]> = Punctuated::parse_separated_nonempty(input)?;
        Ok(AddToRelationInput {
            eval,
            relation,
            numerator,
            cols: cols.into_iter().collect(),
        })
    }
}

pub fn add_to_relation(input: TokenStream) -> TokenStream {
    let AddToRelationInput {
        eval,
        relation,
        numerator,
        cols,
    } = syn::parse_macro_input!(input as AddToRelationInput);

    quote! {{
        #[allow(clippy::cloned_ref_to_slice_refs)]
        #eval.add_to_relation(stwo_constraint_framework::RelationEntry::new(
            &#relation,
            (#numerator).into(),
            &[#(#cols.clone()),*],
        ))
    }}
    .into()
}
