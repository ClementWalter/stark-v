//! Unified AIR schema proc-macro: relations, preprocessed lookups, and trace tables.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Token, braced, parse_macro_input};

use crate::relations::{RelationsInput, generate_relations, parse_relation_defs};
use crate::trace_tables::{
    TraceTablesDef, generate_trace_op_macro, generate_trace_tables, parse_opcode_defs,
};

/// Single source of truth for zkVM AIR metadata.
struct AirInput {
    relations: RelationsInput,
    opcodes: Vec<crate::trace_tables::OpcodeDef>,
}

impl Parse for AirInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut relations = None;
        let mut preprocessed = None;
        let mut opcodes = None;

        while !input.is_empty() {
            let label: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let content;
            braced!(content in input);
            match label.to_string().as_str() {
                "relations" => {
                    if relations.is_some() {
                        return Err(syn::Error::new(
                            label.span(),
                            "duplicate `relations:` block",
                        ));
                    }
                    relations = Some(parse_relation_defs(&content)?);
                }
                "preprocessed" => {
                    if preprocessed.is_some() {
                        return Err(syn::Error::new(
                            label.span(),
                            "duplicate `preprocessed:` block",
                        ));
                    }
                    preprocessed = Some(parse_relation_defs(&content)?);
                }
                "trace" => {
                    if opcodes.is_some() {
                        return Err(syn::Error::new(label.span(), "duplicate `trace:` block"));
                    }
                    opcodes = Some(parse_opcode_defs(&content)?);
                }
                other => {
                    return Err(syn::Error::new(
                        label.span(),
                        format!(
                            "unknown section `{other}`, expected `relations:`, `preprocessed:`, or `trace:`"
                        ),
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let relations = relations.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "missing `relations: { ... }` section",
            )
        })?;
        let preprocessed = preprocessed.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "missing `preprocessed: { ... }` section",
            )
        })?;
        let opcodes = opcodes.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "missing `trace: { ... }` section",
            )
        })?;

        Ok(AirInput {
            relations: RelationsInput {
                relations,
                preprocessed,
            },
            opcodes,
        })
    }
}

pub fn define_air(input: TokenStream) -> TokenStream {
    let AirInput { relations, opcodes } = parse_macro_input!(input as AirInput);

    let relations_tokens = generate_relations(&relations);
    let trace_def = TraceTablesDef::from_trace(opcodes, &relations.preprocessed);
    let traced: Vec<_> = trace_def.opcodes.iter().filter(|op| !op.air_only).collect();
    let trace_op_macro = generate_trace_op_macro(&traced);
    let trace_tokens = generate_trace_tables(&trace_def, quote!(crate::trace::AccessTable));

    quote! {
        #trace_op_macro

        pub mod relations {
            //! LogUp relation registry for zkVM trace tables and prover components.

            #![allow(non_camel_case_types)]

            #[cfg(debug_assertions)]
            pub const INTERACTION_POW_BITS: u32 = 1;
            #[cfg(not(debug_assertions))]
            pub const INTERACTION_POW_BITS: u32 = 10;

            #relations_tokens
        }

        pub mod trace {
            #![allow(clippy::too_many_arguments)]
            //! Trace capture for zkVM execution.

            use simd::AlignedVec;
            use crate::trace::{Access, DEFAULT_MAX_CLOCK_DIFF};

            pub use crate::poseidon2::Poseidon2Table;

            #trace_tokens
        }
    }
    .into()
}
